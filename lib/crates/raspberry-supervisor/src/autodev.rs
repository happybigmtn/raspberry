use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use thiserror::Error;

use crate::dispatch::{execute_selected_lanes, DispatchError, DispatchOutcome, DispatchSettings};
use crate::evaluate::{evaluate_program, EvaluateError, LaneExecutionStatus};
use crate::failure::{
    classify_failure, default_recovery_action, FailureKind, FailureRecoveryAction,
};
use crate::manifest::{ManifestError, ProgramManifest};

thread_local! {
    static ORCHESTRATION_STACK: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutodevSettings {
    pub fabro_bin: PathBuf,
    pub max_parallel_override: Option<usize>,
    pub frontier_budget: Option<usize>,
    pub max_cycles: usize,
    pub poll_interval_ms: u64,
    pub evolve_every_seconds: u64,
    pub doctrine_files: Vec<PathBuf>,
    pub evidence_paths: Vec<PathBuf>,
    pub preview_evolve_root: Option<PathBuf>,
    pub manifest_stack: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutodevReport {
    pub program: String,
    pub stop_reason: AutodevStopReason,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<AutodevCurrentSnapshot>,
    pub cycles: Vec<AutodevCycleReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutodevCurrentSnapshot {
    pub updated_at: DateTime<Utc>,
    pub ready: usize,
    pub running: usize,
    pub blocked: usize,
    pub failed: usize,
    pub complete: usize,
    #[serde(default)]
    pub ready_lanes: Vec<String>,
    #[serde(default)]
    pub running_lanes: Vec<String>,
    #[serde(default)]
    pub failed_lanes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutodevCycleReport {
    pub cycle: usize,
    pub evolved: bool,
    pub evolve_target: Option<String>,
    pub ready_lanes: Vec<String>,
    #[serde(default)]
    pub replayed_lanes: Vec<String>,
    pub dispatched: Vec<DispatchOutcome>,
    pub running_after: usize,
    pub complete_after: usize,
}

const DOCTRINE_STATE_SCHEMA_VERSION: &str = "raspberry.doctrine.v1";
const BACKOFF_RETRY_MIN_SECS: i64 = 300;
const REFRESH_FROM_TRUNK_MIN_SECS: i64 = 300;
const DEFAULT_DOCTRINE_ROOT_FILES: &[&str] = &[
    "README.md",
    "SPEC.md",
    "SPECS.md",
    "PLANS.md",
    "DESIGN.md",
    "AGENTS.md",
    "CLAUDE.md",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutodevStopReason {
    Settled,
    CycleLimit,
}

#[derive(Debug, Error)]
pub enum AutodevError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error(transparent)]
    Evaluate(#[from] EvaluateError),
    #[error(transparent)]
    Dispatch(#[from] DispatchError),
    #[error("failed to read blueprint {path}: {source}")]
    ReadBlueprint {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse blueprint {path}: {source}")]
    ParseBlueprint {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("blueprint {path} is invalid: {message}")]
    InvalidBlueprint { path: PathBuf, message: String },
    #[error("failed to write blueprint {path}: {source}")]
    WriteBlueprint {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read autodev report {path}: {source}")]
    ReadReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse autodev report {path}: {source}")]
    ParseReport {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize autodev report {path}: {source}")]
    SerializeReport {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to write autodev report {path}: {source}")]
    WriteReport {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read doctrine state {path}: {source}")]
    ReadDoctrineState {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse doctrine state {path}: {source}")]
    ParseDoctrineState {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize doctrine state {path}: {source}")]
    SerializeDoctrineState {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to write doctrine state {path}: {source}")]
    WriteDoctrineState {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create autodev temp dir {path}: {source}")]
    CreateTempDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to spawn fabro {step} for program `{program}`: {source}")]
    Spawn {
        step: String,
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("fabro {step} for program `{program}` failed with exit_status={exit_status}")]
    FabroFailed {
        step: String,
        program: String,
        exit_status: i32,
        stdout: String,
        stderr: String,
    },
    #[error("recursive child program cycle detected: {cycle}")]
    RecursiveProgramCycle { cycle: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrontierSignature {
    ready: usize,
    running: usize,
    replayable_failed: usize,
    regenerable_failed: usize,
    complete: usize,
    failed_recovery_keys: Vec<String>,
}

impl FrontierSignature {
    fn total_work(&self) -> usize {
        self.ready + self.running + self.replayable_failed + self.regenerable_failed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DoctrineState {
    schema_version: String,
    program: String,
    manifest: PathBuf,
    updated_at: DateTime<Utc>,
    files: Vec<DoctrineFileFingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct DoctrineFileFingerprint {
    path: PathBuf,
    len: u64,
    modified_unix_ms: i64,
}

pub fn orchestrate_program(
    manifest_path: &Path,
    settings: &AutodevSettings,
) -> Result<AutodevReport, AutodevError> {
    let manifest_path = normalize_path(manifest_path);
    if settings
        .manifest_stack
        .iter()
        .any(|path| path == &manifest_path)
    {
        let mut cycle = settings
            .manifest_stack
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>();
        cycle.push(manifest_path.display().to_string());
        return Err(AutodevError::RecursiveProgramCycle {
            cycle: cycle.join(" -> "),
        });
    }
    let guard = enter_orchestration_scope(&manifest_path)?;
    let initial_manifest = ProgramManifest::load(&manifest_path)?;
    let max_cycles = settings.max_cycles.max(1);
    let poll_interval = Duration::from_millis(settings.poll_interval_ms.max(1));
    let evolve_every = Duration::from_secs(settings.evolve_every_seconds);
    let mut last_evolve_at = None::<Instant>;
    let mut last_evolve_frontier = None::<FrontierSignature>;
    let mut report = AutodevReport {
        program: initial_manifest.program.clone(),
        stop_reason: AutodevStopReason::CycleLimit,
        updated_at: Utc::now(),
        current: None,
        cycles: Vec::new(),
    };

    for cycle_index in 0..max_cycles {
        let manifest = ProgramManifest::load(&manifest_path)?;
        let cycle_number = cycle_index + 1;
        let program_before = evaluate_program(&manifest_path)?;
        let ready_before = count_lanes_with_status(&program_before, LaneExecutionStatus::Ready);
        let running_before = count_lanes_with_status(&program_before, LaneExecutionStatus::Running);
        let complete_before =
            count_lanes_with_status(&program_before, LaneExecutionStatus::Complete);
        let has_ready_before = ready_before > 0;
        let has_running_before = running_before > 0;
        let locally_settled = !has_ready_before && !has_running_before;
        let replayable_failures_before =
            dispatchable_failed_lanes(&manifest, &program_before, false);
        let regenerable_failures = regenerable_failed_lanes(&program_before);
        let max_parallel = settings
            .max_parallel_override
            .unwrap_or(manifest.max_parallel)
            .max(1);
        let frontier_budget = resolve_frontier_budget(settings, max_parallel);
        let frontier_before = FrontierSignature {
            ready: ready_before,
            running: running_before,
            replayable_failed: replayable_failures_before.len(),
            regenerable_failed: regenerable_failures.len(),
            complete: complete_before,
            failed_recovery_keys: failed_recovery_keys(&program_before),
        };
        let doctrine_changed = doctrine_inputs_changed(&manifest_path, &manifest, settings)?;

        let mut evolved = false;
        let mut evolve_target = None;
        if should_trigger_evolve(
            last_evolve_at,
            evolve_every,
            &frontier_before,
            max_parallel,
            frontier_budget,
            locally_settled,
            doctrine_changed,
            !regenerable_failures.is_empty(),
            last_evolve_frontier.as_ref(),
        ) {
            run_synth_evolve(&manifest_path, &manifest, settings)?;
            last_evolve_at = Some(Instant::now());
            last_evolve_frontier = Some(frontier_before);
            evolved = true;
            evolve_target = Some(
                settings
                    .preview_evolve_root
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| {
                        manifest
                            .resolved_target_repo(&manifest_path)
                            .display()
                            .to_string()
                    }),
            );
        }

        let manifest = if evolved {
            ProgramManifest::load(&manifest_path)?
        } else {
            manifest
        };
        let program = if evolved {
            evaluate_program(&manifest_path)?
        } else {
            program_before
        };
        let replayable_failures = dispatchable_failed_lanes(&manifest, &program, evolved);
        let ready_lanes = program
            .lanes
            .iter()
            .filter(|lane| lane.status == LaneExecutionStatus::Ready)
            .map(|lane| lane.lane_key.clone())
            .collect::<Vec<_>>();
        let current_running = program
            .lanes
            .iter()
            .filter(|lane| lane.status == LaneExecutionStatus::Running)
            .count();
        let available_slots = max_parallel.saturating_sub(current_running);
        let replayed_lanes = replayable_failures
            .iter()
            .take(available_slots)
            .cloned()
            .collect::<Vec<_>>();
        let remaining_slots = available_slots.saturating_sub(replayed_lanes.len());
        let mut lanes_to_dispatch = replayed_lanes.clone();
        lanes_to_dispatch.extend(ready_lanes.iter().take(remaining_slots).cloned());

        let dispatched = if lanes_to_dispatch.is_empty() {
            Vec::new()
        } else {
            execute_selected_lanes(
                &manifest_path,
                &lanes_to_dispatch,
                &DispatchSettings {
                    fabro_bin: settings.fabro_bin.clone(),
                    max_parallel_override: settings.max_parallel_override,
                    doctrine_files: settings.doctrine_files.clone(),
                    evidence_paths: settings.evidence_paths.clone(),
                    preview_evolve_root: settings.preview_evolve_root.clone(),
                    manifest_stack: settings
                        .manifest_stack
                        .iter()
                        .cloned()
                        .chain(std::iter::once(manifest_path.clone()))
                        .collect(),
                },
            )?
        };

        let program_after = evaluate_program(&manifest_path)?;
        let running_after = program_after
            .lanes
            .iter()
            .filter(|lane| lane.status == LaneExecutionStatus::Running)
            .count();
        let complete_after = program_after
            .lanes
            .iter()
            .filter(|lane| lane.status == LaneExecutionStatus::Complete)
            .count();

        report.cycles.push(AutodevCycleReport {
            cycle: cycle_number,
            evolved,
            evolve_target,
            ready_lanes: ready_lanes.clone(),
            replayed_lanes,
            dispatched,
            running_after,
            complete_after,
        });
        report.current = Some(current_snapshot(&program_after));
        report.updated_at = Utc::now();
        save_autodev_report(&manifest_path, &manifest, &report)?;

        let has_ready = program_after
            .lanes
            .iter()
            .any(|lane| lane.status == LaneExecutionStatus::Ready);
        let has_running = program_after
            .lanes
            .iter()
            .any(|lane| lane.status == LaneExecutionStatus::Running);
        let spare_child_slots = max_parallel.saturating_sub(running_after);
        if spare_child_slots > 0 {
            if advance_child_programs(
                &manifest_path,
                &manifest,
                settings,
                &program_after,
                spare_child_slots,
            )? {
                if cycle_number < max_cycles {
                    thread::sleep(poll_interval);
                    continue;
                }
            }
        }
        if !has_ready && !has_running {
            report.stop_reason = AutodevStopReason::Settled;
            report.current = Some(current_snapshot(&program_after));
            report.updated_at = Utc::now();
            save_autodev_report(&manifest_path, &manifest, &report)?;
            return Ok(report);
        }

        if cycle_number < max_cycles {
            thread::sleep(poll_interval);
        }
    }

    report.stop_reason = AutodevStopReason::CycleLimit;
    report.updated_at = Utc::now();
    let final_manifest = ProgramManifest::load(&manifest_path)?;
    let final_program = evaluate_program(&manifest_path)?;
    report.current = Some(current_snapshot(&final_program));
    save_autodev_report(&manifest_path, &final_manifest, &report)?;
    drop(guard);
    Ok(report)
}

fn enter_orchestration_scope(
    manifest_path: &Path,
) -> Result<OrchestrationScopeGuard, AutodevError> {
    ORCHESTRATION_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        if let Some(index) = stack.iter().position(|path| path == manifest_path) {
            let mut cycle = stack[index..]
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>();
            cycle.push(manifest_path.display().to_string());
            return Err(AutodevError::RecursiveProgramCycle {
                cycle: cycle.join(" -> "),
            });
        }
        stack.push(manifest_path.to_path_buf());
        Ok(OrchestrationScopeGuard)
    })
}

struct OrchestrationScopeGuard;

impl Drop for OrchestrationScopeGuard {
    fn drop(&mut self) {
        ORCHESTRATION_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(part) => normalized.push(part),
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(std::path::MAIN_SEPARATOR.to_string()),
        }
    }
    normalized
}

fn advance_child_programs(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    settings: &AutodevSettings,
    program: &crate::evaluate::EvaluatedProgram,
    max_advances: usize,
) -> Result<bool, AutodevError> {
    if max_advances == 0 {
        return Ok(false);
    }
    let child_manifests =
        child_program_manifests_to_advance(manifest_path, manifest, program, max_advances);
    let mut advanced = false;
    for child_manifest in child_manifests {
        let child_manifest_spec = ProgramManifest::load(&child_manifest)?;
        let preview_evolve_root = settings
            .preview_evolve_root
            .as_ref()
            .map(|root| root.join(&child_manifest_spec.program));
        let _ = orchestrate_program(
            &child_manifest,
            &AutodevSettings {
                fabro_bin: settings.fabro_bin.clone(),
                max_parallel_override: None,
                frontier_budget: settings.frontier_budget,
                max_cycles: 1,
                poll_interval_ms: 1,
                evolve_every_seconds: 0,
                doctrine_files: settings.doctrine_files.clone(),
                evidence_paths: settings.evidence_paths.clone(),
                preview_evolve_root,
                manifest_stack: settings
                    .manifest_stack
                    .iter()
                    .cloned()
                    .chain(std::iter::once(manifest_path.to_path_buf()))
                    .collect(),
            },
        )?;
        advanced = true;
    }
    Ok(advanced)
}

fn child_program_manifests_to_advance(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    program: &crate::evaluate::EvaluatedProgram,
    max_advances: usize,
) -> Vec<PathBuf> {
    let mut manifests = Vec::new();
    for lane in &program.lanes {
        if manifests.len() >= max_advances {
            break;
        }
        if lane.status != LaneExecutionStatus::Failed {
            continue;
        }
        if lane.recovery_action.is_none() {
            continue;
        }
        let Some(child_manifest) =
            manifest.resolve_lane_program_manifest(manifest_path, &lane.unit_id, &lane.lane_id)
        else {
            continue;
        };
        manifests.push(child_manifest);
    }
    manifests
}

fn should_evolve(last_evolve_at: Option<Instant>, evolve_every: Duration) -> bool {
    let Some(last_evolve_at) = last_evolve_at else {
        return true;
    };
    evolve_every.is_zero() || last_evolve_at.elapsed() >= evolve_every
}

fn should_trigger_evolve(
    last_evolve_at: Option<Instant>,
    evolve_every: Duration,
    frontier: &FrontierSignature,
    max_parallel: usize,
    frontier_budget: usize,
    locally_settled: bool,
    doctrine_changed: bool,
    recovery_needs_evolve: bool,
    last_evolve_frontier: Option<&FrontierSignature>,
) -> bool {
    if frontier.total_work() >= frontier_budget {
        return false;
    }
    let spare_capacity = frontier.running < max_parallel;
    let frontier_progressed = last_evolve_frontier != Some(frontier);
    let spare_capacity_trigger = spare_capacity && frontier.ready == 0 && frontier_progressed;
    if doctrine_changed && (locally_settled || spare_capacity_trigger) {
        return true;
    }
    let recovery_trigger =
        recovery_needs_evolve && frontier_progressed && (locally_settled || spare_capacity);
    if recovery_trigger {
        return true;
    }
    if !should_evolve(last_evolve_at, evolve_every) {
        return false;
    }
    if locally_settled {
        return true;
    }
    spare_capacity_trigger
}

fn count_lanes_with_status(
    program: &crate::evaluate::EvaluatedProgram,
    status: LaneExecutionStatus,
) -> usize {
    program
        .lanes
        .iter()
        .filter(|lane| lane.status == status)
        .count()
}

fn resolve_frontier_budget(settings: &AutodevSettings, max_parallel: usize) -> usize {
    settings
        .frontier_budget
        .unwrap_or_else(|| max_parallel.saturating_add(2))
        .max(max_parallel)
}

pub fn autodev_report_path(manifest_path: &Path, manifest: &ProgramManifest) -> PathBuf {
    manifest
        .resolved_target_repo(manifest_path)
        .join(".raspberry")
        .join(format!("{}-autodev.json", manifest.program))
}

fn doctrine_state_path(manifest_path: &Path, manifest: &ProgramManifest) -> PathBuf {
    manifest
        .resolved_target_repo(manifest_path)
        .join(".raspberry")
        .join(format!("{}-doctrine-state.json", manifest.program))
}

fn doctrine_inputs_changed(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    settings: &AutodevSettings,
) -> Result<bool, AutodevError> {
    let path = doctrine_state_path(manifest_path, manifest);
    let target_repo = manifest.resolved_target_repo(manifest_path);
    let manifest_relative = repo_relative_or_absolute(&target_repo, manifest_path);
    let next_files = collect_doctrine_fingerprints(&target_repo, settings)?;
    if next_files.is_empty() {
        return Ok(false);
    }

    let previous = load_optional_doctrine_state(&path)?;
    let changed = previous
        .as_ref()
        .map(|state| state.files != next_files || state.manifest != manifest_relative)
        .unwrap_or(true);
    if changed {
        save_doctrine_state(
            &path,
            &DoctrineState {
                schema_version: DOCTRINE_STATE_SCHEMA_VERSION.to_string(),
                program: manifest.program.clone(),
                manifest: manifest_relative,
                updated_at: Utc::now(),
                files: next_files,
            },
        )?;
    }
    Ok(changed)
}

fn load_optional_doctrine_state(path: &Path) -> Result<Option<DoctrineState>, AutodevError> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).map_err(|source| AutodevError::ReadDoctrineState {
        path: path.to_path_buf(),
        source,
    })?;
    let state = serde_json::from_str(&raw).map_err(|source| AutodevError::ParseDoctrineState {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(Some(state))
}

fn save_doctrine_state(path: &Path, state: &DoctrineState) -> Result<(), AutodevError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| AutodevError::WriteDoctrineState {
            path: path.to_path_buf(),
            source,
        })?;
    }
    let json = serde_json::to_string_pretty(state).map_err(|source| {
        AutodevError::SerializeDoctrineState {
            path: path.to_path_buf(),
            source,
        }
    })?;
    fs::write(path, json).map_err(|source| AutodevError::WriteDoctrineState {
        path: path.to_path_buf(),
        source,
    })
}

fn collect_doctrine_fingerprints(
    target_repo: &Path,
    settings: &AutodevSettings,
) -> Result<Vec<DoctrineFileFingerprint>, AutodevError> {
    let inputs = collect_doctrine_inputs(target_repo, settings)?;
    let mut fingerprints = Vec::new();
    for path in inputs {
        let absolute = if path.is_absolute() {
            path.clone()
        } else {
            target_repo.join(&path)
        };
        let Ok(metadata) = fs::metadata(&absolute) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let modified_unix_ms = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or_default();
        fingerprints.push(DoctrineFileFingerprint {
            path: repo_relative_or_absolute(target_repo, &absolute),
            len: metadata.len(),
            modified_unix_ms,
        });
    }
    fingerprints.sort();
    fingerprints.dedup();
    Ok(fingerprints)
}

fn collect_doctrine_inputs(
    target_repo: &Path,
    settings: &AutodevSettings,
) -> Result<Vec<PathBuf>, AutodevError> {
    let mut inputs = BTreeSet::new();
    for relative in DEFAULT_DOCTRINE_ROOT_FILES {
        let path = PathBuf::from(relative);
        if target_repo.join(&path).is_file() {
            inputs.insert(path);
        }
    }
    collect_doctrine_dir(target_repo, Path::new("plans"), &mut inputs)?;
    collect_doctrine_dir(target_repo, Path::new("specs"), &mut inputs)?;
    for path in &settings.doctrine_files {
        collect_doctrine_path(target_repo, path, &mut inputs)?;
    }
    Ok(inputs.into_iter().collect())
}

fn collect_doctrine_dir(
    target_repo: &Path,
    relative_dir: &Path,
    inputs: &mut BTreeSet<PathBuf>,
) -> Result<(), AutodevError> {
    let root = target_repo.join(relative_dir);
    if !root.is_dir() {
        return Ok(());
    }
    let mut stack = vec![root];
    while let Some(directory) = stack.pop() {
        let entries =
            fs::read_dir(&directory).map_err(|source| AutodevError::ReadDoctrineState {
                path: directory.clone(),
                source,
            })?;
        for entry in entries {
            let entry = entry.map_err(|source| AutodevError::ReadDoctrineState {
                path: directory.clone(),
                source,
            })?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !is_doctrine_file(&path) {
                continue;
            }
            inputs.insert(repo_relative_or_absolute(target_repo, &path));
        }
    }
    Ok(())
}

fn collect_doctrine_path(
    target_repo: &Path,
    raw_path: &Path,
    inputs: &mut BTreeSet<PathBuf>,
) -> Result<(), AutodevError> {
    let absolute = if raw_path.is_absolute() {
        raw_path.to_path_buf()
    } else {
        target_repo.join(raw_path)
    };
    if absolute.is_dir() {
        let relative = repo_relative_or_absolute(target_repo, &absolute);
        return collect_doctrine_dir(target_repo, &relative, inputs);
    }
    if absolute.is_file() && is_doctrine_file(&absolute) {
        inputs.insert(repo_relative_or_absolute(target_repo, &absolute));
    }
    Ok(())
}

fn is_doctrine_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("md" | "mdx")
    )
}

fn repo_relative_or_absolute(target_repo: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(target_repo)
        .map(PathBuf::from)
        .unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn autodev_cargo_target_dir(target_repo: &Path) -> PathBuf {
    target_repo
        .join(".raspberry")
        .join("cargo-target")
        .components()
        .collect()
}

pub fn load_optional_autodev_report(
    manifest_path: &Path,
    manifest: &ProgramManifest,
) -> Result<Option<AutodevReport>, AutodevError> {
    let path = autodev_report_path(manifest_path, manifest);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(|source| AutodevError::ReadReport {
        path: path.clone(),
        source,
    })?;
    let mut report: AutodevReport =
        serde_json::from_str(&raw).map_err(|source| AutodevError::ParseReport {
            path: path.clone(),
            source,
        })?;
    let program = evaluate_program(manifest_path)?;
    let next_snapshot = current_snapshot(&program);
    if report.current.as_ref() != Some(&next_snapshot) {
        report.current = Some(next_snapshot);
        report.updated_at = Utc::now();
        save_autodev_report(manifest_path, manifest, &report)?;
    }
    Ok(Some(report))
}

pub fn sync_autodev_report_with_program(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    program: &crate::evaluate::EvaluatedProgram,
) -> Result<(), AutodevError> {
    let path = autodev_report_path(manifest_path, manifest);
    if !path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&path).map_err(|source| AutodevError::ReadReport {
        path: path.clone(),
        source,
    })?;
    let mut report: AutodevReport =
        serde_json::from_str(&raw).map_err(|source| AutodevError::ParseReport {
            path: path.clone(),
            source,
        })?;
    let next_snapshot = current_snapshot(program);
    if report.current.as_ref() == Some(&next_snapshot) {
        return Ok(());
    }
    report.current = Some(next_snapshot);
    report.updated_at = Utc::now();
    save_autodev_report(manifest_path, manifest, &report)
}

fn save_autodev_report(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    report: &AutodevReport,
) -> Result<(), AutodevError> {
    let path = autodev_report_path(manifest_path, manifest);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| AutodevError::WriteReport {
            path: path.clone(),
            source,
        })?;
    }
    let json =
        serde_json::to_string_pretty(report).map_err(|source| AutodevError::SerializeReport {
            path: path.clone(),
            source,
        })?;
    fs::write(&path, json).map_err(|source| AutodevError::WriteReport {
        path: path.clone(),
        source,
    })
}

fn current_snapshot(program: &crate::evaluate::EvaluatedProgram) -> AutodevCurrentSnapshot {
    let mut ready = 0usize;
    let mut running = 0usize;
    let mut blocked = 0usize;
    let mut failed = 0usize;
    let mut complete = 0usize;
    let mut ready_lanes = Vec::new();
    let mut running_lanes = Vec::new();
    let mut failed_lanes = Vec::new();

    for lane in &program.lanes {
        match lane.status {
            LaneExecutionStatus::Ready => {
                ready += 1;
                ready_lanes.push(lane.lane_key.clone());
            }
            LaneExecutionStatus::Running => {
                running += 1;
                running_lanes.push(lane.lane_key.clone());
            }
            LaneExecutionStatus::Blocked => blocked += 1,
            LaneExecutionStatus::Failed => {
                failed += 1;
                failed_lanes.push(lane.lane_key.clone());
            }
            LaneExecutionStatus::Complete => complete += 1,
        }
    }

    AutodevCurrentSnapshot {
        updated_at: Utc::now(),
        ready,
        running,
        blocked,
        failed,
        complete,
        ready_lanes,
        running_lanes,
        failed_lanes,
    }
}

fn dispatchable_failed_lanes(
    manifest: &ProgramManifest,
    program: &crate::evaluate::EvaluatedProgram,
    allow_regenerate: bool,
) -> Vec<String> {
    let now = Utc::now();
    let mut lanes = program
        .lanes
        .iter()
        .filter(|lane| lane.status == LaneExecutionStatus::Failed)
        .filter_map(|lane| replay_target_lane(manifest, lane, now, allow_regenerate))
        .collect::<Vec<_>>();
    lanes.sort();
    lanes.dedup();
    lanes
}

#[cfg(test)]
fn replayable_failed_lanes(
    manifest: &ProgramManifest,
    program: &crate::evaluate::EvaluatedProgram,
) -> Vec<String> {
    dispatchable_failed_lanes(manifest, program, false)
}

fn failure_kind_for_lane(lane: &crate::evaluate::EvaluatedLane) -> Option<FailureKind> {
    lane.failure_kind.or_else(|| {
        classify_failure(
            lane.last_error.as_deref(),
            lane.last_stderr_snippet.as_deref(),
            lane.last_stdout_snippet.as_deref(),
        )
    })
}

fn replay_target_lane(
    manifest: &ProgramManifest,
    lane: &crate::evaluate::EvaluatedLane,
    now: DateTime<Utc>,
    allow_regenerate: bool,
) -> Option<String> {
    let kind = failure_kind_for_lane(lane)?;
    match default_recovery_action(kind) {
        FailureRecoveryAction::ReplaySourceLane => {
            integration_source_lane_key(manifest, lane).or_else(|| Some(lane.lane_key.clone()))
        }
        FailureRecoveryAction::ReplayLane => Some(lane.lane_key.clone()),
        FailureRecoveryAction::RefreshFromTrunk => retry_after_cooldown(
            lane,
            now,
            REFRESH_FROM_TRUNK_MIN_SECS,
            lane.lane_key.clone(),
        ),
        FailureRecoveryAction::BackoffRetry => {
            retry_after_cooldown(lane, now, BACKOFF_RETRY_MIN_SECS, lane.lane_key.clone())
        }
        FailureRecoveryAction::RegenerateLane => {
            if allow_regenerate {
                Some(lane.lane_key.clone())
            } else {
                None
            }
        }
        FailureRecoveryAction::SurfaceBlocked => None,
    }
}

fn regenerable_failed_lanes(program: &crate::evaluate::EvaluatedProgram) -> Vec<String> {
    let mut lanes = program
        .lanes
        .iter()
        .filter(|lane| lane.status == LaneExecutionStatus::Failed)
        .filter_map(|lane| {
            let kind = failure_kind_for_lane(lane)?;
            (default_recovery_action(kind) == FailureRecoveryAction::RegenerateLane)
                .then(|| lane.lane_key.clone())
        })
        .collect::<Vec<_>>();
    lanes.sort();
    lanes.dedup();
    lanes
}

fn failed_recovery_keys(program: &crate::evaluate::EvaluatedProgram) -> Vec<String> {
    let mut keys = program
        .lanes
        .iter()
        .filter(|lane| lane.status == LaneExecutionStatus::Failed)
        .filter_map(|lane| {
            let kind = failure_kind_for_lane(lane)?;
            let action = default_recovery_action(kind);
            Some(format!("{}:{kind}:{action}", lane.lane_key))
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    keys
}

fn retry_after_cooldown(
    lane: &crate::evaluate::EvaluatedLane,
    now: DateTime<Utc>,
    cooldown_secs: i64,
    target_lane_key: String,
) -> Option<String> {
    let finished_at = lane.last_finished_at.or(lane.last_started_at)?;
    if (now - finished_at).num_seconds() < cooldown_secs {
        return None;
    }
    Some(target_lane_key)
}

fn integration_source_lane_key(
    manifest: &ProgramManifest,
    lane: &crate::evaluate::EvaluatedLane,
) -> Option<String> {
    let unit = manifest.units.get(&lane.unit_id)?;
    let lane = unit.lanes.get(&lane.lane_id)?;
    let dependency = lane
        .dependencies
        .iter()
        .find(|dependency| dependency.lane.is_some())?;
    Some(format!(
        "{}:{}",
        dependency.unit,
        dependency.lane.as_ref().expect("lane dependency exists")
    ))
}

fn run_synth_evolve(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    settings: &AutodevSettings,
) -> Result<(), AutodevError> {
    let target_repo = manifest.resolved_target_repo(manifest_path);
    let temp_dir = autodev_temp_dir(&manifest.program)?;
    let blueprint_path = temp_dir.join(format!("{}-autodev.yaml", manifest.program));

    let import_cmd = format!(
        "export CARGO_TARGET_DIR={}; exec {} --no-upgrade-check synth import --target-repo {} --program {} --output {}",
        shell_escape(&autodev_cargo_target_dir(&target_repo).display().to_string()),
        shell_escape(&settings.fabro_bin.display().to_string()),
        shell_escape(&target_repo.display().to_string()),
        shell_escape(&manifest.program),
        shell_escape(&blueprint_path.display().to_string()),
    );
    let import = Command::new("bash")
        .current_dir(&target_repo)
        .arg("-ic")
        .arg(import_cmd)
        .output()
        .map_err(|source| AutodevError::Spawn {
            step: "synth import".to_string(),
            program: manifest.program.clone(),
            source,
        })?;
    ensure_success("synth import", &manifest.program, import)?;

    inject_inputs_into_blueprint(
        &blueprint_path,
        &settings.doctrine_files,
        &settings.evidence_paths,
    )?;

    let mut evolve_cmd = format!(
        "export CARGO_TARGET_DIR={}; exec {} --no-upgrade-check synth evolve --blueprint {} --target-repo {}",
        shell_escape(&autodev_cargo_target_dir(&target_repo).display().to_string()),
        shell_escape(&settings.fabro_bin.display().to_string()),
        shell_escape(&blueprint_path.display().to_string()),
        shell_escape(&target_repo.display().to_string()),
    );
    if let Some(preview_root) = &settings.preview_evolve_root {
        evolve_cmd.push_str(" --preview-root ");
        evolve_cmd.push_str(&shell_escape(&preview_root.display().to_string()));
    }
    let output = Command::new("bash")
        .current_dir(&target_repo)
        .arg("-ic")
        .arg(evolve_cmd)
        .output()
        .map_err(|source| AutodevError::Spawn {
            step: "synth evolve".to_string(),
            program: manifest.program.clone(),
            source,
        })?;
    ensure_success("synth evolve", &manifest.program, output)?;

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

fn ensure_success(
    step: &str,
    program: &str,
    output: std::process::Output,
) -> Result<(), AutodevError> {
    if output.status.success() {
        return Ok(());
    }

    Err(AutodevError::FabroFailed {
        step: step.to_string(),
        program: program.to_string(),
        exit_status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn autodev_temp_dir(program: &str) -> Result<PathBuf, AutodevError> {
    let path = std::env::temp_dir().join(format!(
        "raspberry-autodev-{}-{}-{}",
        program,
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    fs::create_dir_all(&path).map_err(|source| AutodevError::CreateTempDir {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn inject_inputs_into_blueprint(
    blueprint_path: &Path,
    doctrine_files: &[PathBuf],
    evidence_paths: &[PathBuf],
) -> Result<(), AutodevError> {
    if doctrine_files.is_empty() && evidence_paths.is_empty() {
        return Ok(());
    }

    let raw = fs::read_to_string(blueprint_path).map_err(|source| AutodevError::ReadBlueprint {
        path: blueprint_path.to_path_buf(),
        source,
    })?;
    let mut root: Value =
        serde_yaml::from_str(&raw).map_err(|source| AutodevError::ParseBlueprint {
            path: blueprint_path.to_path_buf(),
            source,
        })?;
    let root_map = root
        .as_mapping_mut()
        .ok_or_else(|| AutodevError::InvalidBlueprint {
            path: blueprint_path.to_path_buf(),
            message: "blueprint root must be a mapping".to_string(),
        })?;

    let inputs_key = Value::String("inputs".to_string());
    let inputs_value = root_map
        .entry(inputs_key)
        .or_insert_with(|| Value::Mapping(Mapping::new()));
    let inputs_map =
        inputs_value
            .as_mapping_mut()
            .ok_or_else(|| AutodevError::InvalidBlueprint {
                path: blueprint_path.to_path_buf(),
                message: "blueprint inputs must be a mapping".to_string(),
            })?;

    if !doctrine_files.is_empty() {
        inputs_map.insert(
            Value::String("doctrine_files".to_string()),
            Value::Sequence(
                doctrine_files
                    .iter()
                    .map(|path| Value::String(path.display().to_string()))
                    .collect(),
            ),
        );
    }
    if !evidence_paths.is_empty() {
        inputs_map.insert(
            Value::String("evidence_paths".to_string()),
            Value::Sequence(
                evidence_paths
                    .iter()
                    .map(|path| Value::String(path.display().to_string()))
                    .collect(),
            ),
        );
    }

    let yaml = serde_yaml::to_string(&root).map_err(|source| AutodevError::ParseBlueprint {
        path: blueprint_path.to_path_buf(),
        source,
    })?;
    let trimmed = yaml.trim_start_matches("---\n");
    fs::write(blueprint_path, trimmed).map_err(|source| AutodevError::WriteBlueprint {
        path: blueprint_path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluate::{EvaluatedLane, LaneExecutionStatus};
    use crate::manifest::LaneKind;
    use std::collections::BTreeMap;
    use std::time::Duration;

    fn failed_lane(lane_key: &str, error: &str) -> EvaluatedLane {
        EvaluatedLane {
            lane_key: lane_key.to_string(),
            unit_id: "unit".to_string(),
            unit_title: "Unit".to_string(),
            lane_id: "lane".to_string(),
            lane_title: "Lane".to_string(),
            lane_kind: LaneKind::Platform,
            status: LaneExecutionStatus::Failed,
            operational_state: None,
            precondition_state: None,
            proof_state: None,
            orchestration_state: None,
            detail: String::new(),
            managed_milestone: "reviewed".to_string(),
            proof_profile: None,
            run_config: PathBuf::from("fabro/run-configs/bootstrap/demo.toml"),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: None,
            last_started_at: None,
            last_finished_at: None,
            last_exit_status: Some(1),
            last_error: Some(error.to_string()),
            failure_kind: None,
            recovery_action: None,
            last_completed_stage_label: None,
            last_stage_duration_ms: None,
            last_usage_summary: None,
            last_files_read: Vec::new(),
            last_files_written: Vec::new(),
            last_stdout_snippet: None,
            last_stderr_snippet: None,
            ready_checks_passing: Vec::new(),
            ready_checks_failing: Vec::new(),
            running_checks_passing: Vec::new(),
            running_checks_failing: Vec::new(),
        }
    }

    #[test]
    fn replayable_failed_lanes_only_selects_recoverable_failures() {
        let manifest = ProgramManifest::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml"),
        )
        .expect("manifest loads");
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            lanes: vec![
                failed_lane(
                    "impl:lane",
                    "direct integration requires a branch-backed run, but worktree setup failed",
                ),
                failed_lane("broken:lane", "real test failure"),
            ],
        };

        let lanes = replayable_failed_lanes(&manifest, &program);

        assert_eq!(lanes, vec!["impl:lane".to_string()]);
    }

    #[test]
    fn dispatchable_failed_lanes_selects_regenerable_failures_after_evolve() {
        let manifest = ProgramManifest::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml"),
        )
        .expect("manifest loads");
        let mut lane = failed_lane(
            "broken:lane",
            "Engine error: deterministic failure cycle detected: signature verify repeated 3 times",
        );
        lane.last_started_at = Some(Utc::now() - chrono::Duration::minutes(20));
        lane.last_finished_at = Some(Utc::now() - chrono::Duration::minutes(10));
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            lanes: vec![lane],
        };

        let lanes = dispatchable_failed_lanes(&manifest, &program, true);

        assert_eq!(lanes, vec!["broken:lane".to_string()]);
    }

    #[test]
    fn child_program_manifests_to_advance_uses_spare_slots_for_failed_children() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_path = temp.path().join("program.yaml");
        std::fs::create_dir_all(temp.path().join("fabro/programs")).expect("program dir");
        std::fs::write(
            &manifest_path,
            r#"
version: 1
program: demo
target_repo: .
state_path: .raspberry/demo-state.json
max_parallel: 5
units:
  - id: alpha
    title: Alpha
    output_root: out/alpha
    artifacts: []
    milestones: []
    lanes:
      - id: program
        kind: orchestration
        title: Alpha Program
        run_config: fabro/programs/alpha.yaml
        managed_milestone: coordinated
        program_manifest: fabro/programs/alpha.yaml
  - id: beta
    title: Beta
    output_root: out/beta
    artifacts: []
    milestones: []
    lanes:
      - id: program
        kind: orchestration
        title: Beta Program
        run_config: fabro/programs/beta.yaml
        managed_milestone: coordinated
        program_manifest: fabro/programs/beta.yaml
  - id: gamma
    title: Gamma
    output_root: out/gamma
    artifacts: []
    milestones: []
    lanes:
      - id: program
        kind: orchestration
        title: Gamma Program
        run_config: fabro/programs/gamma.yaml
        managed_milestone: coordinated
        program_manifest: fabro/programs/gamma.yaml
"#,
        )
        .expect("manifest written");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 5,
            lanes: vec![
                EvaluatedLane {
                    lane_key: "alpha:program".to_string(),
                    unit_id: "alpha".to_string(),
                    unit_title: "Alpha".to_string(),
                    lane_id: "program".to_string(),
                    lane_title: "Alpha Program".to_string(),
                    lane_kind: LaneKind::Orchestration,
                    status: LaneExecutionStatus::Failed,
                    operational_state: None,
                    precondition_state: None,
                    proof_state: None,
                    orchestration_state: None,
                    detail: String::new(),
                    managed_milestone: "coordinated".to_string(),
                    proof_profile: None,
                    run_config: PathBuf::from("fabro/programs/alpha.yaml"),
                    run_id: None,
                    current_run_id: None,
                    current_fabro_run_id: None,
                    current_stage: None,
                    last_run_id: None,
                    last_started_at: None,
                    last_finished_at: None,
                    last_exit_status: Some(1),
                    last_error: Some("deterministic failure cycle detected".to_string()),
                    failure_kind: Some(FailureKind::DeterministicVerifyCycle),
                    recovery_action: Some(FailureRecoveryAction::RegenerateLane),
                    last_completed_stage_label: None,
                    last_stage_duration_ms: None,
                    last_usage_summary: None,
                    last_files_read: Vec::new(),
                    last_files_written: Vec::new(),
                    last_stdout_snippet: None,
                    last_stderr_snippet: None,
                    ready_checks_passing: Vec::new(),
                    ready_checks_failing: Vec::new(),
                    running_checks_passing: Vec::new(),
                    running_checks_failing: Vec::new(),
                },
                EvaluatedLane {
                    lane_key: "beta:program".to_string(),
                    unit_id: "beta".to_string(),
                    unit_title: "Beta".to_string(),
                    lane_id: "program".to_string(),
                    lane_title: "Beta Program".to_string(),
                    lane_kind: LaneKind::Orchestration,
                    status: LaneExecutionStatus::Failed,
                    operational_state: None,
                    precondition_state: None,
                    proof_state: None,
                    orchestration_state: None,
                    detail: String::new(),
                    managed_milestone: "coordinated".to_string(),
                    proof_profile: None,
                    run_config: PathBuf::from("fabro/programs/beta.yaml"),
                    run_id: None,
                    current_run_id: None,
                    current_fabro_run_id: None,
                    current_stage: None,
                    last_run_id: None,
                    last_started_at: None,
                    last_finished_at: None,
                    last_exit_status: Some(1),
                    last_error: Some("deterministic failure cycle detected".to_string()),
                    failure_kind: Some(FailureKind::DeterministicVerifyCycle),
                    recovery_action: Some(FailureRecoveryAction::RegenerateLane),
                    last_completed_stage_label: None,
                    last_stage_duration_ms: None,
                    last_usage_summary: None,
                    last_files_read: Vec::new(),
                    last_files_written: Vec::new(),
                    last_stdout_snippet: None,
                    last_stderr_snippet: None,
                    ready_checks_passing: Vec::new(),
                    ready_checks_failing: Vec::new(),
                    running_checks_passing: Vec::new(),
                    running_checks_failing: Vec::new(),
                },
                EvaluatedLane {
                    lane_key: "gamma:program".to_string(),
                    unit_id: "gamma".to_string(),
                    unit_title: "Gamma".to_string(),
                    lane_id: "program".to_string(),
                    lane_title: "Gamma Program".to_string(),
                    lane_kind: LaneKind::Orchestration,
                    status: LaneExecutionStatus::Running,
                    operational_state: None,
                    precondition_state: None,
                    proof_state: None,
                    orchestration_state: None,
                    detail: String::new(),
                    managed_milestone: "coordinated".to_string(),
                    proof_profile: None,
                    run_config: PathBuf::from("fabro/programs/gamma.yaml"),
                    run_id: None,
                    current_run_id: Some("01RUNNING".to_string()),
                    current_fabro_run_id: Some("01RUNNING".to_string()),
                    current_stage: Some("Implement".to_string()),
                    last_run_id: Some("01RUNNING".to_string()),
                    last_started_at: None,
                    last_finished_at: None,
                    last_exit_status: None,
                    last_error: None,
                    failure_kind: None,
                    recovery_action: None,
                    last_completed_stage_label: None,
                    last_stage_duration_ms: None,
                    last_usage_summary: None,
                    last_files_read: Vec::new(),
                    last_files_written: Vec::new(),
                    last_stdout_snippet: None,
                    last_stderr_snippet: None,
                    ready_checks_passing: Vec::new(),
                    ready_checks_failing: Vec::new(),
                    running_checks_passing: Vec::new(),
                    running_checks_failing: Vec::new(),
                },
            ],
        };

        let manifests = child_program_manifests_to_advance(&manifest_path, &manifest, &program, 1);

        assert_eq!(manifests.len(), 1);
        assert!(manifests[0].ends_with("fabro/programs/alpha.yaml"));
    }

    #[test]
    fn integration_replay_targets_source_lane_when_run_was_branchless() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_path = temp.path().join("program.yaml");
        std::fs::write(
            &manifest_path,
            r#"
version: 1
program: demo
target_repo: .
state_path: .raspberry/demo-state.json
units:
  - id: play
    title: Play
    output_root: outputs/play
    artifacts:
      - id: implementation
        path: implementation.md
      - id: integration
        path: integration.md
    milestones:
      - id: merge_ready
        requires: [implementation]
      - id: integrated
        requires: [integration]
    lanes:
      - id: tui-implement
        kind: interface
        title: Implement
        run_config: run-configs/implement.toml
        managed_milestone: merge_ready
        produces: [implementation]
      - id: tui-integrate
        kind: integration
        title: Integrate
        run_config: run-configs/integrate.toml
        managed_milestone: integrated
        depends_on:
          - unit: play
            lane: tui-implement
            milestone: merge_ready
        produces: [integration]
"#,
        )
        .expect("manifest written");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let lane = EvaluatedLane {
            lane_key: "play:tui-integrate".to_string(),
            unit_id: "play".to_string(),
            unit_title: "Play".to_string(),
            lane_id: "tui-integrate".to_string(),
            lane_title: "Integrate".to_string(),
            lane_kind: LaneKind::Integration,
            status: LaneExecutionStatus::Failed,
            operational_state: None,
            precondition_state: None,
            proof_state: None,
            orchestration_state: None,
            detail: String::new(),
            managed_milestone: "integrated".to_string(),
            proof_profile: None,
            run_config: PathBuf::from("fabro/run-configs/integrate/demo.toml"),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: None,
            last_started_at: None,
            last_finished_at: None,
            last_exit_status: Some(1),
            last_error: Some(
                "run `01KMTEST` is not branch-backed; rerun it in a branch-backed worktree"
                    .to_string(),
            ),
            failure_kind: None,
            recovery_action: None,
            last_completed_stage_label: None,
            last_stage_duration_ms: None,
            last_usage_summary: None,
            last_files_read: Vec::new(),
            last_files_written: Vec::new(),
            last_stdout_snippet: None,
            last_stderr_snippet: None,
            ready_checks_passing: Vec::new(),
            ready_checks_failing: Vec::new(),
            running_checks_passing: Vec::new(),
            running_checks_failing: Vec::new(),
        };

        let target = replay_target_lane(&manifest, &lane, Utc::now(), false);

        assert_eq!(target.as_deref(), Some("play:tui-implement"));
    }

    #[test]
    fn replayable_failed_lanes_replay_source_lane_for_failed_integration_program() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_path = temp.path().join("program.yaml");
        std::fs::create_dir_all(temp.path().join(".raspberry")).expect("state dir");
        std::fs::write(
            &manifest_path,
            r#"
version: 1
program: myosu-play-tui-implementation
target_repo: .
state_path: .raspberry/myosu-play-tui-implementation-state.json
max_parallel: 1
units:
  - id: play
    title: Gameplay TUI Delivery
    output_root: outputs/play/tui
    artifacts:
      - id: implementation
        path: implementation.md
      - id: integration
        path: integration.md
    milestones:
      - id: merge_ready
        requires: [implementation]
      - id: integrated
        requires: [integration]
    lanes:
      - id: tui-implement
        kind: interface
        title: Gameplay TUI Implementation Lane
        run_config: run-configs/implement/play-tui.toml
        managed_milestone: merge_ready
        produces: [implementation]
      - id: tui-integrate
        kind: integration
        title: Gameplay TUI Integration Lane
        run_config: run-configs/integrate/play-tui.toml
        managed_milestone: integrated
        depends_on:
          - unit: play
            lane: tui-implement
            milestone: merge_ready
        produces: [integration]
"#,
        )
        .expect("manifest written");
        std::fs::write(
            temp.path()
                .join(".raspberry/myosu-play-tui-implementation-state.json"),
            serde_json::json!({
                "schema_version": "raspberry.program.v2",
                "program": "myosu-play-tui-implementation",
                "updated_at": chrono::Utc::now(),
                "lanes": {
                    "play:tui-implement": {
                        "lane_key": "play:tui-implement",
                        "status": "complete",
                        "run_config": "run-configs/implement/play-tui.toml",
                        "last_run_id": "01KM48DT7VMSPZAMAQKTKAJGAR",
                        "last_started_at": chrono::Utc::now(),
                        "last_finished_at": chrono::Utc::now(),
                        "last_exit_status": 0
                    },
                    "play:tui-integrate": {
                        "lane_key": "play:tui-integrate",
                        "status": "failed",
                        "run_config": "run-configs/integrate/play-tui.toml",
                        "last_started_at": chrono::Utc::now(),
                        "last_finished_at": chrono::Utc::now(),
                        "last_exit_status": 1,
                        "last_error": "run `01KM48DT7VMSPZAMAQKTKAJGAR` is not branch-backed; rerun it in a branch-backed worktree",
                        "last_stderr_snippet": "run `01KM48DT7VMSPZAMAQKTKAJGAR` is not branch-backed; rerun it in a branch-backed worktree"
                    }
                }
            })
            .to_string(),
        )
        .expect("state written");

        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let program = evaluate_program(&manifest_path).expect("program evaluates");

        let lanes = replayable_failed_lanes(&manifest, &program);

        assert_eq!(lanes, vec!["play:tui-implement".to_string()]);
    }

    #[test]
    fn replayed_lanes_can_fill_available_capacity_before_ready_work() {
        let ready_lane = EvaluatedLane {
            lane_key: "ready:lane".to_string(),
            unit_id: "unit".to_string(),
            unit_title: "Unit".to_string(),
            lane_id: "lane".to_string(),
            lane_title: "Lane".to_string(),
            lane_kind: LaneKind::Platform,
            status: LaneExecutionStatus::Ready,
            operational_state: None,
            precondition_state: None,
            proof_state: None,
            orchestration_state: None,
            detail: String::new(),
            managed_milestone: "reviewed".to_string(),
            proof_profile: None,
            run_config: PathBuf::from("fabro/run-configs/bootstrap/demo.toml"),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: None,
            last_started_at: None,
            last_finished_at: None,
            last_exit_status: None,
            last_error: None,
            failure_kind: None,
            recovery_action: None,
            last_completed_stage_label: None,
            last_stage_duration_ms: None,
            last_usage_summary: None,
            last_files_read: Vec::new(),
            last_files_written: Vec::new(),
            last_stdout_snippet: None,
            last_stderr_snippet: None,
            ready_checks_passing: Vec::new(),
            ready_checks_failing: Vec::new(),
            running_checks_passing: Vec::new(),
            running_checks_failing: Vec::new(),
        };
        let replayed_lanes = vec!["failed:lane".to_string()];
        let ready_lanes = vec![ready_lane.lane_key.clone()];
        let available_slots = 2usize;
        let remaining_slots = available_slots.saturating_sub(replayed_lanes.len());
        let mut lanes_to_dispatch = replayed_lanes.clone();
        lanes_to_dispatch.extend(ready_lanes.iter().take(remaining_slots).cloned());

        assert_eq!(
            lanes_to_dispatch,
            vec!["failed:lane".to_string(), "ready:lane".to_string()]
        );
    }

    #[test]
    fn refresh_from_trunk_replays_same_lane_after_cooldown() {
        let manifest = ProgramManifest::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml"),
        )
        .expect("manifest loads");
        let lane = EvaluatedLane {
            lane_key: "demo:integrate".to_string(),
            unit_id: "demo".to_string(),
            unit_title: "Demo".to_string(),
            lane_id: "integrate".to_string(),
            lane_title: "Integrate".to_string(),
            lane_kind: LaneKind::Integration,
            status: LaneExecutionStatus::Failed,
            operational_state: None,
            precondition_state: None,
            proof_state: None,
            orchestration_state: None,
            detail: String::new(),
            managed_milestone: "integrated".to_string(),
            proof_profile: None,
            run_config: PathBuf::from("fabro/run-configs/integrate/demo.toml"),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: Some("01KMTEST".to_string()),
            last_started_at: Some(Utc::now() - chrono::Duration::minutes(20)),
            last_finished_at: Some(Utc::now() - chrono::Duration::minutes(10)),
            last_exit_status: Some(1),
            last_error: Some(
                "git merge --squash failed in /tmp/worktree: Recorded preimage for 'foo'"
                    .to_string(),
            ),
            failure_kind: Some(FailureKind::IntegrationConflict),
            recovery_action: Some(FailureRecoveryAction::RefreshFromTrunk),
            last_completed_stage_label: Some("Exit".to_string()),
            last_stage_duration_ms: Some(0),
            last_usage_summary: None,
            last_files_read: Vec::new(),
            last_files_written: Vec::new(),
            last_stdout_snippet: None,
            last_stderr_snippet: None,
            ready_checks_passing: Vec::new(),
            ready_checks_failing: Vec::new(),
            running_checks_passing: Vec::new(),
            running_checks_failing: Vec::new(),
        };

        let target = replay_target_lane(&manifest, &lane, Utc::now(), false);

        assert_eq!(target.as_deref(), Some("demo:integrate"));
    }

    #[test]
    fn backoff_retry_waits_for_cooldown() {
        let manifest = ProgramManifest::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml"),
        )
        .expect("manifest loads");
        let lane = EvaluatedLane {
            lane_key: "demo:service".to_string(),
            unit_id: "demo".to_string(),
            unit_title: "Demo".to_string(),
            lane_id: "service".to_string(),
            lane_title: "Service".to_string(),
            lane_kind: LaneKind::Service,
            status: LaneExecutionStatus::Failed,
            operational_state: None,
            precondition_state: None,
            proof_state: None,
            orchestration_state: None,
            detail: String::new(),
            managed_milestone: "reviewed".to_string(),
            proof_profile: None,
            run_config: PathBuf::from("fabro/run-configs/bootstrap/demo.toml"),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: Some("01KMTEST".to_string()),
            last_started_at: Some(Utc::now() - chrono::Duration::minutes(1)),
            last_finished_at: Some(Utc::now() - chrono::Duration::seconds(30)),
            last_exit_status: Some(1),
            last_error: Some("OSError: [Errno 98] Address already in use".to_string()),
            failure_kind: Some(FailureKind::EnvironmentCollision),
            recovery_action: Some(FailureRecoveryAction::BackoffRetry),
            last_completed_stage_label: Some("Verify".to_string()),
            last_stage_duration_ms: Some(0),
            last_usage_summary: None,
            last_files_read: Vec::new(),
            last_files_written: Vec::new(),
            last_stdout_snippet: None,
            last_stderr_snippet: None,
            ready_checks_passing: Vec::new(),
            ready_checks_failing: Vec::new(),
            running_checks_passing: Vec::new(),
            running_checks_failing: Vec::new(),
        };

        let target = replay_target_lane(&manifest, &lane, Utc::now(), false);

        assert!(target.is_none());
    }

    #[test]
    fn doctrine_inputs_changed_tracks_plan_updates_per_program() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        std::fs::write(temp.path().join("README.md"), "# Demo\n").expect("readme");
        std::fs::write(
            temp.path().join("plans/2026-03-20-demo.md"),
            "# Demo Plan\n\n- [ ] Keep moving\n",
        )
        .expect("plan");
        let manifest_path = temp.path().join("fabro/programs/demo.yaml");
        std::fs::create_dir_all(
            manifest_path
                .parent()
                .expect("manifest parent should exist"),
        )
        .expect("program dir");
        std::fs::write(&manifest_path, "program: demo\nunits: {}\n").expect("manifest");
        let manifest = ProgramManifest {
            program: "demo".to_string(),
            target_repo: PathBuf::from("../.."),
            state_path: PathBuf::from("../../.raspberry/demo-state.json"),
            max_parallel: 1,
            run_dir: None,
            units: BTreeMap::new(),
        };
        let settings = AutodevSettings {
            fabro_bin: PathBuf::from("fabro"),
            max_parallel_override: None,
            frontier_budget: None,
            max_cycles: 1,
            poll_interval_ms: 1,
            evolve_every_seconds: 0,
            doctrine_files: Vec::new(),
            evidence_paths: Vec::new(),
            preview_evolve_root: None,
            manifest_stack: Vec::new(),
        };

        assert!(
            doctrine_inputs_changed(&manifest_path, &manifest, &settings)
                .expect("first scan should detect doctrine")
        );
        assert!(
            !doctrine_inputs_changed(&manifest_path, &manifest, &settings)
                .expect("second scan should be stable")
        );

        std::fs::write(
            temp.path().join("plans/2026-03-20-demo.md"),
            "# Demo Plan\n\n- [ ] Keep moving\n- [ ] Add another frontier\n",
        )
        .expect("plan updated");

        assert!(
            doctrine_inputs_changed(&manifest_path, &manifest, &settings)
                .expect("plan update should force doctrine delta")
        );
    }

    #[test]
    fn spare_capacity_evolve_stays_bounded_by_frontier_budget() {
        let frontier = FrontierSignature {
            ready: 0,
            running: 1,
            replayable_failed: 1,
            regenerable_failed: 0,
            complete: 3,
            failed_recovery_keys: vec![
                "failed:lane:environment_collision:backoff_retry".to_string()
            ],
        };

        assert!(should_trigger_evolve(
            Some(Instant::now()),
            Duration::ZERO,
            &frontier,
            5,
            5,
            false,
            false,
            false,
            Some(&FrontierSignature {
                ready: 0,
                running: 2,
                replayable_failed: 1,
                regenerable_failed: 0,
                complete: 2,
                failed_recovery_keys: vec![
                    "failed:lane:environment_collision:backoff_retry".to_string()
                ],
            }),
        ));
        assert!(!should_trigger_evolve(
            Some(Instant::now()),
            Duration::ZERO,
            &frontier,
            5,
            2,
            false,
            false,
            false,
            Some(&FrontierSignature {
                ready: 0,
                running: 2,
                replayable_failed: 1,
                regenerable_failed: 0,
                complete: 2,
                failed_recovery_keys: vec![
                    "failed:lane:environment_collision:backoff_retry".to_string()
                ],
            }),
        ));
    }

    #[test]
    fn regenerable_failures_trigger_evolve_once_frontier_changes() {
        let frontier = FrontierSignature {
            ready: 0,
            running: 0,
            replayable_failed: 1,
            regenerable_failed: 1,
            complete: 5,
            failed_recovery_keys: vec![
                "broken:lane:deterministic_verify_cycle:regenerate_lane".to_string()
            ],
        };

        assert!(should_trigger_evolve(
            Some(Instant::now()),
            Duration::from_secs(300),
            &frontier,
            5,
            7,
            true,
            false,
            true,
            Some(&FrontierSignature {
                ready: 0,
                running: 1,
                replayable_failed: 0,
                regenerable_failed: 0,
                complete: 5,
                failed_recovery_keys: vec![
                    "broken:lane:proof_script_failure:regenerate_lane".to_string()
                ],
            }),
        ));

        assert!(!should_trigger_evolve(
            Some(Instant::now()),
            Duration::from_secs(300),
            &frontier,
            5,
            7,
            true,
            false,
            true,
            Some(&frontier),
        ));
    }

    #[test]
    fn orchestrate_program_reports_recursive_child_program_cycles() {
        let temp = tempfile::tempdir().expect("tempdir");
        let programs_dir = temp.path().join("fabro/programs");
        std::fs::create_dir_all(&programs_dir).expect("program dir");
        let parent_manifest = programs_dir.join("parent.yaml");
        let child_manifest = programs_dir.join("child.yaml");
        std::fs::write(
            &parent_manifest,
            r#"
version: 1
program: parent
target_repo: ../..
state_path: ../../.raspberry/parent-state.json
max_parallel: 1
units:
  - id: child
    title: Child
    output_root: ../../.raspberry/portfolio/child
    artifacts: []
    milestones: []
    lanes:
      - id: program
        kind: orchestration
        title: Child Program
        run_config: ../../fabro/programs/child.yaml
        managed_milestone: coordinated
        program_manifest: ../../fabro/programs/child.yaml
"#,
        )
        .expect("parent manifest");
        std::fs::write(
            &child_manifest,
            r#"
version: 1
program: child
target_repo: ../..
state_path: ../../.raspberry/child-state.json
max_parallel: 1
units:
  - id: parent
    title: Parent
    output_root: ../../.raspberry/portfolio/parent
    artifacts: []
    milestones: []
    lanes:
      - id: program
        kind: orchestration
        title: Parent Program
        run_config: ../../fabro/programs/parent.yaml
        managed_milestone: coordinated
        program_manifest: ../../fabro/programs/parent.yaml
"#,
        )
        .expect("child manifest");

        let error = orchestrate_program(
            &parent_manifest,
            &AutodevSettings {
                fabro_bin: PathBuf::from("/bin/false"),
                max_parallel_override: None,
                frontier_budget: None,
                max_cycles: 1,
                poll_interval_ms: 1,
                evolve_every_seconds: 0,
                doctrine_files: Vec::new(),
                evidence_paths: Vec::new(),
                preview_evolve_root: None,
                manifest_stack: Vec::new(),
            },
        )
        .expect_err("recursive program cycle should fail cleanly");

        assert!(!error.to_string().is_empty());
    }
}
