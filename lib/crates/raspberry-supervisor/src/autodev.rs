use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use fabro_config::run::{load_run_config, resolve_graph_path};
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::controller_lease::{
    acquire_autodev_lease, autodev_controller_active, ControllerLeaseError,
};
use crate::dispatch::{execute_selected_lanes, DispatchError, DispatchOutcome, DispatchSettings};
use crate::evaluate::{evaluate_program, parse_usage_summary, EvaluateError, LaneExecutionStatus};
use crate::failure::{
    classify_failure, default_recovery_action, FailureKind, FailureRecoveryAction,
};
use crate::maintenance::{load_active_maintenance, MaintenanceError};
use crate::manifest::{ManifestError, ProgramManifest};
use crate::program_state::{mark_lane_regenerate_noop, ProgramRuntimeState, ProgramStateError};

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
    pub provenance: Option<AutodevProvenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<AutodevCurrentSnapshot>,
    pub cycles: Vec<AutodevCycleReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutodevProvenance {
    pub controller: BinaryProvenance,
    pub fabro_bin: BinaryProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryProvenance {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutodevCurrentSnapshot {
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_parallel: Option<usize>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub critical_blockers: Vec<CriticalBlocker>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_routes: Vec<LaneProviderRoute>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub usage_by_provider: Vec<ProviderUsageSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriticalBlocker {
    pub lane_key: String,
    pub blocked_downstream: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneProviderRoute {
    pub lane_key: String,
    pub status: LaneExecutionStatus,
    pub model: String,
    pub provider: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderUsageSnapshot {
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_microusd: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutodevCycleReport {
    pub cycle: usize,
    pub evolved: bool,
    pub evolve_target: Option<String>,
    pub ready_lanes: Vec<String>,
    #[serde(default)]
    pub replayed_lanes: Vec<String>,
    #[serde(default)]
    pub regenerate_noop_lanes: Vec<String>,
    pub dispatched: Vec<DispatchOutcome>,
    pub running_after: usize,
    pub complete_after: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<AutodevCycleTiming>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telemetry: Option<AutodevCycleTelemetry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutodevCycleTiming {
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub total_duration_ms: u64,
    pub refresh_evaluate_ms: u64,
    pub evolve_ms: u64,
    pub dispatch_ms: u64,
    pub post_dispatch_evaluate_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutodevCycleTelemetry {
    pub available_slots: usize,
    pub fresh_capacity_reserve: usize,
    pub replay_budget: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oldest_running_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub running_without_completion_cycles: usize,
    #[serde(default, skip_serializing_if = "is_false")]
    pub ready_while_saturated: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ready_but_undispatched: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub direct_ready_but_undispatched: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_program_ready: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub idle_with_ready_work: bool,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub failed_bootstrap_count: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub runtime_path_errors: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub stale_running_reclaimed: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub provider_access_limited: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub stall_watchdog: usize,
}

const DOCTRINE_STATE_SCHEMA_VERSION: &str = "raspberry.doctrine.v1";
const BACKOFF_RETRY_MIN_SECS: i64 = 300;
const TRANSIENT_LAUNCH_RETRY_MIN_SECS: i64 = 15;
const ENVIRONMENT_COLLISION_RETRY_MIN_SECS: i64 = 15;
const REFRESH_FROM_TRUNK_MIN_SECS: i64 = 30;
const SURFACE_BLOCKED_RETRY_MIN_SECS: i64 = 900;
const CODEX_UNBLOCK_RETRY_MIN_SECS: i64 = 30;
const MAX_SURFACE_BLOCKED_RETRIES: u32 = 10;
const REGENERATE_SPARE_CAPACITY_RETRY_SECS: u64 = 15;
const PAPERCLIP_REFRESH_MIN_SECS: u64 = 15;
const SYNTH_EVOLVE_TIMEOUT_SECS: u64 = 120;
const TARGET_REPO_SYNC_COOLDOWN_SECS: i64 = 30;
const DEFAULT_DOCTRINE_ROOT_FILES: &[&str] = &[
    "README.md",
    "SPEC.md",
    "SPECS.md",
    "PLANS.md",
    "DESIGN.md",
    "AGENTS.md",
    "CLAUDE.md",
];

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}

fn elapsed_millis(start: Instant) -> u64 {
    let millis = start.elapsed().as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TargetRepoFreshness {
    NoOrigin,
    InvalidPushRemote { reason: String },
    Current,
    FastForwarded,
    LocalAhead,
    WrongBranch { current: String, expected: String },
    BehindWithLocalChanges { behind: usize },
    Diverged { ahead: usize, behind: usize },
    FetchFailed,
    MergeFailed,
}

fn autodev_debug_steps_enabled() -> bool {
    std::env::var_os("FABRO_AUTODEV_DEBUG_STEPS").is_some()
}

fn autodev_debug_step(program: &str, cycle: usize, message: &str) {
    if autodev_debug_steps_enabled() {
        eprintln!("[autodev-step] program={program} cycle={cycle} {message}");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutodevStopReason {
    InProgress,
    Settled,
    CycleLimit,
    Maintenance,
}

#[derive(Debug, Error)]
pub enum AutodevError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error(transparent)]
    Evaluate(#[from] EvaluateError),
    #[error(transparent)]
    Dispatch(#[from] DispatchError),
    #[error("failed to start workspace admission check in {target_repo}: {source}")]
    AdmissionCheckSpawn {
        target_repo: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("autodev admission gate rejected {target_repo}: {message}")]
    AdmissionCheckFailed {
        target_repo: PathBuf,
        message: String,
    },
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
    #[error(transparent)]
    ControllerLease(#[from] ControllerLeaseError),
    #[error(transparent)]
    Maintenance(#[from] MaintenanceError),
    #[error(transparent)]
    ProgramState(#[from] ProgramStateError),
}

fn is_synth_evolve_timeout(error: &AutodevError) -> bool {
    match error {
        AutodevError::FabroFailed {
            step, exit_status, ..
        } => step == "synth create" && matches!(*exit_status, 124 | 137 | 143),
        _ => false,
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct LaneRenderFingerprint {
    run_config_sha256: Option<String>,
    graph_sha256: Option<String>,
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
    let requested_max_parallel = settings
        .max_parallel_override
        .unwrap_or(initial_manifest.max_parallel)
        .max(1);
    let provenance = Some(capture_autodev_provenance(settings));
    if let Some(_maintenance) = load_active_maintenance(&manifest_path, &initial_manifest)? {
        let current = evaluate_program(&manifest_path)
            .ok()
            .map(|program| current_snapshot(&program, None));
        let report = AutodevReport {
            program: initial_manifest.program.clone(),
            stop_reason: AutodevStopReason::Maintenance,
            updated_at: Utc::now(),
            provenance,
            current,
            cycles: Vec::new(),
        };
        save_autodev_report(&manifest_path, &initial_manifest, &report)?;
        drop(guard);
        return Ok(report);
    }
    enforce_workspace_admission_gate(&manifest_path, &initial_manifest, requested_max_parallel)?;
    let _lease = acquire_autodev_lease(&manifest_path, &initial_manifest)?;
    let max_cycles = cycle_limit(settings.max_cycles);
    let poll_interval = Duration::from_millis(settings.poll_interval_ms.max(1));
    let evolve_every = Duration::from_secs(settings.evolve_every_seconds);
    let mut last_evolve_at = None::<Instant>;
    let mut last_evolve_frontier = None::<FrontierSignature>;
    let mut last_paperclip_refresh_at = None::<Instant>;
    let mut last_repo_sync_at = None::<DateTime<Utc>>;
    let mut report = AutodevReport {
        program: initial_manifest.program.clone(),
        stop_reason: AutodevStopReason::InProgress,
        updated_at: Utc::now(),
        provenance,
        current: None,
        cycles: Vec::new(),
    };

    let mut cycle_number = 0usize;
    let mut last_complete_count: Option<usize> = None;
    loop {
        if let Some(limit) = max_cycles {
            if cycle_number >= limit {
                break;
            }
        }

        cycle_number += 1;
        let cycle_started_at = Utc::now();
        let cycle_started = Instant::now();
        autodev_debug_step(&initial_manifest.program, cycle_number, "cycle-start");
        let manifest = ProgramManifest::load(&manifest_path)?;
        autodev_debug_step(&manifest.program, cycle_number, "manifest-loaded");
        // Only sync the target repo when the complete count may have changed
        // (first cycle, or after a dispatch/evolve that could trigger integration).
        // This avoids running git fetch+reset every 5s poll cycle.
        if last_complete_count.is_none() {
            maybe_sync_target_repo_to_origin(&manifest, &manifest_path, &mut last_repo_sync_at);
        }
        let program_before = evaluate_program(&manifest_path)?;
        autodev_debug_step(&manifest.program, cycle_number, "program-before-evaluated");
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
        let regenerate_fingerprints_before =
            lane_render_fingerprints(&program_before, &regenerable_failures);
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
        autodev_debug_step(
            &manifest.program,
            cycle_number,
            &format!(
                "frontier ready={ready_before} running={running_before} replayable={} regenerable={} doctrine_changed={doctrine_changed}",
                replayable_failures_before.len(),
                regenerable_failures.len()
            ),
        );
        let refresh_evaluate_ms = elapsed_millis(cycle_started);

        let mut evolved = false;
        let mut evolve_target = None;
        let evolve_started = Instant::now();
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
            autodev_debug_step(&manifest.program, cycle_number, "running-synth-evolve");
            match run_synth_evolve(&manifest_path, &manifest, settings) {
                Ok(()) => {
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
                Err(error) if is_synth_evolve_timeout(&error) => {
                    autodev_debug_step(
                        &manifest.program,
                        cycle_number,
                        "synth-evolve-timeout-skipping-cycle-evolve",
                    );
                    eprintln!(
                        "[autodev] synth evolve timed out for program `{}`; skipping evolve this cycle",
                        manifest.program
                    );
                    last_evolve_at = Some(Instant::now());
                    last_evolve_frontier = Some(frontier_before);
                }
                Err(error) => {
                    eprintln!(
                        "[autodev] synth evolve failed for program `{}`: {error}; continuing",
                        manifest.program
                    );
                    last_evolve_at = Some(Instant::now());
                    last_evolve_frontier = Some(frontier_before);
                }
            }
        }
        let evolve_ms = elapsed_millis(evolve_started);
        autodev_debug_step(
            &manifest.program,
            cycle_number,
            if evolved {
                "evolve-complete"
            } else {
                "evolve-skipped"
            },
        );

        let manifest = if evolved {
            ProgramManifest::load(&manifest_path)?
        } else {
            manifest
        };
        let mut program = if evolved {
            evaluate_program(&manifest_path)?
        } else {
            program_before
        };
        autodev_debug_step(
            &manifest.program,
            cycle_number,
            "program-after-evolve-evaluated",
        );
        let regenerate_noop_lanes = if evolved {
            let noop_lanes = detect_regenerate_noop_lanes(
                &regenerable_failures,
                &regenerate_fingerprints_before,
                &program,
            );
            if !noop_lanes.is_empty() {
                mark_regenerate_noop_lanes(
                    &manifest_path,
                    &manifest,
                    &program,
                    &noop_lanes,
                    &regenerate_fingerprints_before,
                )?;
                program = evaluate_program(&manifest_path)?;
            }
            noop_lanes
        } else {
            Vec::new()
        };
        let replayable_failures = prioritized_failure_lane_keys(
            &program,
            dispatchable_failed_lanes(&manifest, &program, evolved),
        );
        let ready_lanes = prioritized_lane_keys(
            program
                .lanes
                .iter()
                .filter(|lane| lane.status == LaneExecutionStatus::Ready)
                .collect::<Vec<_>>(),
        );
        let dispatch_started = Instant::now();
        let current_running = program
            .lanes
            .iter()
            .filter(|lane| lane.status == LaneExecutionStatus::Running)
            .count();
        let available_slots = max_parallel.saturating_sub(current_running);
        let fresh_capacity_reserve =
            reserve_fresh_ready_capacity(&program, &replayable_failures, available_slots);
        let replay_budget = available_slots.saturating_sub(fresh_capacity_reserve);
        let replayed_lanes =
            select_replayed_lanes_for_dispatch(&program, &replayable_failures, replay_budget);
        let remaining_slots = available_slots.saturating_sub(replayed_lanes.len());
        let selected_ready_lanes =
            select_ready_lanes_for_dispatch(&program, remaining_slots, &replayed_lanes);
        let mut lanes_to_dispatch = replayed_lanes.clone();
        lanes_to_dispatch.extend(selected_ready_lanes);
        autodev_debug_step(
            &manifest.program,
            cycle_number,
            &format!(
                "dispatch-plan available_slots={available_slots} fresh_reserve={fresh_capacity_reserve} replay_budget={replay_budget} replayed={} ready={} dispatching={}",
                replayed_lanes.len(),
                ready_lanes.len(),
                lanes_to_dispatch.len()
            ),
        );

        let dispatched = if lanes_to_dispatch.is_empty() {
            Vec::new()
        } else {
            match execute_selected_lanes(
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
            ) {
                Ok(outcomes) => outcomes,
                Err(DispatchError::TargetRepoStale { message }) => {
                    eprintln!(
                        "[autodev] dispatch skipped for program `{}`: target repo is stale: {}",
                        manifest.program, message
                    );
                    Vec::new()
                }
                Err(error) => return Err(error.into()),
            }
        };
        autodev_debug_step(
            &manifest.program,
            cycle_number,
            &format!("dispatch-complete outcomes={}", dispatched.len()),
        );
        let dispatch_ms = elapsed_millis(dispatch_started);

        // After dispatch, sync the target repo if integrations may have landed.
        // This ensures evaluate sees freshly-pushed artifacts for milestone checks.
        if !dispatched.is_empty() || evolved {
            maybe_sync_target_repo_to_origin(&manifest, &manifest_path, &mut last_repo_sync_at);
        }

        // Skip redundant evaluation when nothing changed (no dispatch, no evolve).
        // Between program_before and here only ms elapsed; re-evaluating 221 lanes
        // with check probes and progress file reads is wasted work.
        let post_dispatch_evaluate_started = Instant::now();
        let program_after = if dispatched.is_empty() && !evolved {
            autodev_debug_step(
                &manifest.program,
                cycle_number,
                "program-after-dispatch-skipped-no-changes",
            );
            program
        } else {
            let p = evaluate_program(&manifest_path)?;
            autodev_debug_step(
                &manifest.program,
                cycle_number,
                "program-after-dispatch-evaluated",
            );
            p
        };
        let post_dispatch_evaluate_ms = elapsed_millis(post_dispatch_evaluate_started);
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
        let running_without_completion_cycles = next_running_without_completion_cycles(
            report
                .cycles
                .last()
                .and_then(|cycle| cycle.telemetry.as_ref())
                .map(|telemetry| telemetry.running_without_completion_cycles)
                .unwrap_or(0),
            last_complete_count,
            running_after,
            complete_after,
        );
        let cycle_telemetry = build_cycle_telemetry(
            &program_after,
            available_slots,
            fresh_capacity_reserve,
            replay_budget,
            &ready_lanes,
            &lanes_to_dispatch,
            running_without_completion_cycles,
            oldest_running_seconds(&program_after, Utc::now()),
            running_after >= max_parallel
                && program_after
                    .lanes
                    .iter()
                    .any(|lane| lane.status == LaneExecutionStatus::Ready),
        );

        // Track complete count so we know when to sync on the next cycle
        last_complete_count = Some(complete_after);

        report.cycles.push(AutodevCycleReport {
            cycle: cycle_number,
            evolved,
            evolve_target,
            ready_lanes: ready_lanes.clone(),
            replayed_lanes,
            regenerate_noop_lanes,
            dispatched,
            running_after,
            complete_after,
            timing: Some(AutodevCycleTiming {
                started_at: cycle_started_at,
                finished_at: Utc::now(),
                total_duration_ms: elapsed_millis(cycle_started),
                refresh_evaluate_ms,
                evolve_ms,
                dispatch_ms,
                post_dispatch_evaluate_ms,
            }),
            telemetry: Some(cycle_telemetry),
        });
        report.current = Some(current_snapshot(&program_after, Some(max_parallel)));
        report.updated_at = Utc::now();
        save_autodev_report(&manifest_path, &manifest, &report)?;
        autodev_debug_step(&manifest.program, cycle_number, "report-saved");
        maybe_refresh_paperclip_dashboard(
            &manifest_path,
            &manifest,
            settings,
            &mut last_paperclip_refresh_at,
        );

        let has_ready = program_after
            .lanes
            .iter()
            .any(|lane| lane.status == LaneExecutionStatus::Ready);
        let has_running = program_after
            .lanes
            .iter()
            .any(|lane| lane.status == LaneExecutionStatus::Running);
        let spare_child_slots = max_parallel.saturating_sub(running_after);
        if spare_child_slots > 0
            && advance_child_programs(
                &manifest_path,
                &manifest,
                settings,
                &program_after,
                spare_child_slots,
            )?
            && has_more_cycles(max_cycles, cycle_number)
        {
            autodev_debug_step(&manifest.program, cycle_number, "advanced-child-programs");
            thread::sleep(poll_interval);
            continue;
        }
        if !has_ready && !has_running {
            report.stop_reason = AutodevStopReason::Settled;
            report.current = Some(current_snapshot(&program_after, Some(max_parallel)));
            report.updated_at = Utc::now();
            save_autodev_report(&manifest_path, &manifest, &report)?;
            maybe_refresh_paperclip_dashboard(
                &manifest_path,
                &manifest,
                settings,
                &mut last_paperclip_refresh_at,
            );
            return Ok(report);
        }

        if has_more_cycles(max_cycles, cycle_number) {
            thread::sleep(poll_interval);
        }
    }

    report.stop_reason = AutodevStopReason::CycleLimit;
    report.updated_at = Utc::now();
    let final_manifest = ProgramManifest::load(&manifest_path)?;
    let final_program = evaluate_program(&manifest_path)?;
    let final_max_parallel = settings
        .max_parallel_override
        .unwrap_or(final_manifest.max_parallel)
        .max(1);
    report.current = Some(current_snapshot(&final_program, Some(final_max_parallel)));
    save_autodev_report(&manifest_path, &final_manifest, &report)?;
    maybe_refresh_paperclip_dashboard(
        &manifest_path,
        &final_manifest,
        settings,
        &mut last_paperclip_refresh_at,
    );
    drop(guard);
    Ok(report)
}

fn cycle_limit(max_cycles: usize) -> Option<usize> {
    if max_cycles == 0 {
        None
    } else {
        Some(max_cycles)
    }
}

fn has_more_cycles(max_cycles: Option<usize>, cycle_number: usize) -> bool {
    match max_cycles {
        Some(limit) => cycle_number < limit,
        None => true,
    }
}

fn build_cycle_telemetry(
    program: &crate::evaluate::EvaluatedProgram,
    available_slots: usize,
    fresh_capacity_reserve: usize,
    replay_budget: usize,
    ready_lanes: &[String],
    lanes_to_dispatch: &[String],
    running_without_completion_cycles: usize,
    oldest_running_seconds: Option<u64>,
    ready_while_saturated: bool,
) -> AutodevCycleTelemetry {
    let lanes_to_dispatch: BTreeSet<&str> = lanes_to_dispatch.iter().map(String::as_str).collect();
    let ready_lane_kind = program
        .lanes
        .iter()
        .map(|lane| (lane.lane_key.as_str(), lane.lane_kind))
        .collect::<BTreeMap<_, _>>();
    let ready_but_undispatched = ready_lanes
        .iter()
        .filter(|lane| !lanes_to_dispatch.contains(lane.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let direct_ready_but_undispatched = ready_but_undispatched
        .iter()
        .filter(|lane| {
            ready_lane_kind
                .get(lane.as_str())
                .is_some_and(|kind| *kind != crate::manifest::LaneKind::Orchestration)
        })
        .cloned()
        .collect::<Vec<_>>();
    let child_program_ready = ready_but_undispatched
        .iter()
        .filter(|lane| {
            ready_lane_kind
                .get(lane.as_str())
                .is_some_and(|kind| *kind == crate::manifest::LaneKind::Orchestration)
        })
        .cloned()
        .collect::<Vec<_>>();

    let mut failed_bootstrap_count = 0usize;
    let mut runtime_path_errors = 0usize;
    let mut stale_running_reclaimed = 0usize;
    let mut provider_access_limited = 0usize;
    let mut stall_watchdog = 0usize;

    for lane in &program.lanes {
        let Some(kind) = lane.failure_kind else {
            if stale_run_reclaimed(lane) {
                stale_running_reclaimed += 1;
            }
            continue;
        };
        match kind {
            FailureKind::TransientLaunchFailure => failed_bootstrap_count += 1,
            FailureKind::ProviderAccessLimited => provider_access_limited += 1,
            FailureKind::StallWatchdog => stall_watchdog += 1,
            _ => {}
        }
        if runtime_path_error(lane, kind) {
            runtime_path_errors += 1;
        }
        if stale_run_reclaimed(lane) {
            stale_running_reclaimed += 1;
        }
    }

    AutodevCycleTelemetry {
        available_slots,
        fresh_capacity_reserve,
        replay_budget,
        oldest_running_seconds,
        running_without_completion_cycles,
        ready_while_saturated,
        idle_with_ready_work: available_slots > 0
            && !direct_ready_but_undispatched.is_empty()
            && lanes_to_dispatch.is_empty(),
        ready_but_undispatched,
        direct_ready_but_undispatched,
        child_program_ready,
        failed_bootstrap_count,
        runtime_path_errors,
        stale_running_reclaimed,
        provider_access_limited,
        stall_watchdog,
    }
}

fn next_running_without_completion_cycles(
    previous_cycles: usize,
    previous_complete_after: Option<usize>,
    running_after: usize,
    complete_after: usize,
) -> usize {
    if running_after == 0 {
        return 0;
    }
    match previous_complete_after {
        Some(previous_complete_after) if previous_complete_after == complete_after => {
            previous_cycles + 1
        }
        _ => 0,
    }
}

fn oldest_running_seconds(
    program: &crate::evaluate::EvaluatedProgram,
    observed_at: DateTime<Utc>,
) -> Option<u64> {
    program
        .lanes
        .iter()
        .filter(|lane| lane.status == LaneExecutionStatus::Running)
        .filter_map(|lane| lane.last_started_at)
        .map(|started_at| {
            let elapsed = (observed_at - started_at).num_seconds().max(0);
            u64::try_from(elapsed).unwrap_or(u64::MAX)
        })
        .max()
}

fn runtime_path_error(lane: &crate::evaluate::EvaluatedLane, kind: FailureKind) -> bool {
    let mut combined = String::new();
    for text in [
        Some(lane.detail.as_str()),
        lane.last_error.as_deref(),
        lane.last_stdout_snippet.as_deref(),
        lane.last_stderr_snippet.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if text.trim().is_empty() {
            continue;
        }
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&text.to_lowercase());
    }

    if combined.is_empty() {
        return false;
    }

    matches!(
        kind,
        FailureKind::TransientLaunchFailure | FailureKind::ProviderPolicyMismatch
    ) && (combined.contains("no such file or directory")
        || combined.contains("failed to spawn fabro")
        || combined.contains("failed to spawn git")
        || combined.contains("prompt")
        || combined.contains("@../../prompts/")
        || combined.contains("synth")
        || combined.contains("manifest"))
}

fn stale_run_reclaimed(lane: &crate::evaluate::EvaluatedLane) -> bool {
    lane.last_error
        .as_deref()
        .is_some_and(|error| error.contains("tracked run was removed after controller stopped"))
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
        match orchestrate_program(
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
        ) {
            Ok(_) => advanced = true,
            Err(AutodevError::ControllerLease(ControllerLeaseError::AlreadyRunning { .. })) => {
                advanced = true;
            }
            Err(error) => return Err(error),
        }
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
        let actionable = match lane.status {
            LaneExecutionStatus::Ready => true,
            LaneExecutionStatus::Failed => lane.recovery_action.is_some(),
            LaneExecutionStatus::Blocked
            | LaneExecutionStatus::Running
            | LaneExecutionStatus::Complete => false,
        };
        if !actionable {
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

fn should_fast_track_regenerate_evolve(
    last_evolve_at: Option<Instant>,
    _frontier: &FrontierSignature,
    _max_parallel: usize,
    frontier_progressed: bool,
    recovery_needs_evolve: bool,
) -> bool {
    // Evolve re-synthesises run configs without consuming a parallel slot, so
    // it must not be gated on spare dispatch capacity.  Otherwise regenerable
    // lanes deadlock when all slots are occupied by running/replaying work.
    if !recovery_needs_evolve {
        return false;
    }
    let Some(last_evolve_at) = last_evolve_at else {
        return true;
    };
    frontier_progressed
        || last_evolve_at.elapsed() >= Duration::from_secs(REGENERATE_SPARE_CAPACITY_RETRY_SECS)
}

#[allow(clippy::too_many_arguments)]
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
    let frontier_progressed = last_evolve_frontier != Some(frontier);
    if should_fast_track_regenerate_evolve(
        last_evolve_at,
        frontier,
        max_parallel,
        frontier_progressed,
        recovery_needs_evolve,
    ) {
        return true;
    }
    if frontier.total_work() >= frontier_budget {
        return false;
    }
    let spare_capacity = frontier.running < max_parallel;
    let no_active_work = frontier.running == 0;
    let spare_capacity_trigger =
        spare_capacity && no_active_work && frontier.ready == 0 && frontier_progressed;
    if doctrine_changed && (locally_settled || spare_capacity_trigger) {
        return true;
    }
    let recovery_trigger = recovery_needs_evolve
        && frontier_progressed
        && (locally_settled || no_active_work || spare_capacity);
    if recovery_trigger {
        return true;
    }
    if !should_evolve(last_evolve_at, evolve_every) {
        return false;
    }
    if !frontier_progressed {
        return false;
    }
    if locally_settled {
        return true;
    }
    spare_capacity_trigger
}

fn prioritized_failure_lane_keys(
    program: &crate::evaluate::EvaluatedProgram,
    lane_keys: Vec<String>,
) -> Vec<String> {
    let lane_map = program
        .lanes
        .iter()
        .map(|lane| (lane.lane_key.clone(), lane))
        .collect::<BTreeMap<_, _>>();
    let mut ordered = lane_keys;
    ordered.sort_by(|left, right| {
        let left_lane = lane_map.get(left);
        let right_lane = lane_map.get(right);
        lane_dispatch_priority_tuple(right_lane)
            .cmp(&lane_dispatch_priority_tuple(left_lane))
            .then_with(|| left.cmp(right))
    });
    ordered
}

fn prioritized_lane_keys(lanes: Vec<&crate::evaluate::EvaluatedLane>) -> Vec<String> {
    let mut lanes = lanes;
    lanes.sort_by(|left, right| {
        lane_dispatch_priority_tuple(Some(right))
            .cmp(&lane_dispatch_priority_tuple(Some(left)))
            .then_with(|| left.lane_key.cmp(&right.lane_key))
    });
    let high_priority_present = lanes
        .iter()
        .any(|lane| lane_dispatch_priority_score(lane) >= 60);
    if high_priority_present {
        lanes.retain(|lane| lane_dispatch_priority_score(lane) >= 30);
    }
    lanes
        .into_iter()
        .map(|lane| lane.lane_key.clone())
        .collect()
}

fn select_ready_lanes_for_dispatch(
    program: &crate::evaluate::EvaluatedProgram,
    available_slots: usize,
    already_selected: &[String],
) -> Vec<String> {
    if available_slots == 0 {
        return Vec::new();
    }

    let ready_lanes = program
        .lanes
        .iter()
        .filter(|lane| lane.status == LaneExecutionStatus::Ready)
        .collect::<Vec<_>>();
    let ordered = prioritized_lane_keys(ready_lanes);
    if ordered.len() <= 1 {
        return ordered.into_iter().take(available_slots).collect();
    }

    let lane_map = program
        .lanes
        .iter()
        .map(|lane| (lane.lane_key.clone(), lane))
        .collect::<BTreeMap<_, _>>();
    let unit_ids = program
        .lanes
        .iter()
        .map(|lane| lane.unit_id.clone())
        .collect::<BTreeSet<_>>();
    let distinct_families = ordered
        .iter()
        .filter_map(|lane_key| {
            lane_map
                .get(lane_key)
                .map(|lane| lane_root_plan_family(lane, &unit_ids))
        })
        .collect::<BTreeSet<_>>();

    if distinct_families.len() <= 1 {
        return ordered.into_iter().take(available_slots).collect();
    }

    let mut selected = Vec::new();
    let mut seen_families = already_selected
        .iter()
        .filter_map(|lane_key| {
            lane_map
                .get(lane_key)
                .map(|lane| lane_root_plan_family(lane, &unit_ids))
        })
        .collect::<BTreeSet<_>>();
    for lane_key in ordered {
        if selected.len() >= available_slots {
            break;
        }
        let Some(lane) = lane_map.get(&lane_key) else {
            continue;
        };
        let family = lane_root_plan_family(lane, &unit_ids);
        if seen_families.insert(family) {
            selected.push(lane_key);
        }
    }
    selected
}

fn select_replayed_lanes_for_dispatch(
    program: &crate::evaluate::EvaluatedProgram,
    replayable_failures: &[String],
    available_slots: usize,
) -> Vec<String> {
    if available_slots == 0 || replayable_failures.is_empty() {
        return Vec::new();
    }

    let lane_map = program
        .lanes
        .iter()
        .map(|lane| (lane.lane_key.clone(), lane))
        .collect::<BTreeMap<_, _>>();
    let unit_ids = program
        .lanes
        .iter()
        .map(|lane| lane.unit_id.clone())
        .collect::<BTreeSet<_>>();
    let distinct_families = replayable_failures
        .iter()
        .filter_map(|lane_key| {
            lane_map
                .get(lane_key)
                .map(|lane| lane_root_plan_family(lane, &unit_ids))
        })
        .collect::<BTreeSet<_>>();

    if distinct_families.len() <= 1 {
        return replayable_failures
            .iter()
            .take(available_slots)
            .cloned()
            .collect();
    }

    let mut selected = Vec::new();
    let mut seen_families = BTreeSet::new();
    for lane_key in replayable_failures {
        if selected.len() >= available_slots {
            break;
        }
        let Some(lane) = lane_map.get(lane_key) else {
            continue;
        };
        let family = lane_root_plan_family(lane, &unit_ids);
        if seen_families.insert(family) {
            selected.push(lane_key.clone());
        }
    }
    selected
}

fn reserve_fresh_ready_capacity(
    program: &crate::evaluate::EvaluatedProgram,
    replayable_failures: &[String],
    available_slots: usize,
) -> usize {
    if available_slots < 2 || replayable_failures.is_empty() {
        return 0;
    }
    let direct_ready_count = program
        .lanes
        .iter()
        .filter(|lane| {
            lane.status == LaneExecutionStatus::Ready
                && lane.lane_kind != crate::manifest::LaneKind::Orchestration
        })
        .count();
    if direct_ready_count == 0 {
        0
    } else {
        let replay_includes_codex_unblock = replayable_failures.iter().any(|lane_key| {
            program
                .lanes
                .iter()
                .find(|lane| lane.lane_key == *lane_key)
                .is_some_and(|lane| lane.lane_id.ends_with("-codex-unblock"))
        });
        let reserve_cap = if replay_includes_codex_unblock { 2 } else { 1 };
        direct_ready_count
            .min(reserve_cap)
            .min(available_slots.saturating_sub(1))
    }
}

fn lane_dispatch_priority_tuple(
    lane: Option<&&crate::evaluate::EvaluatedLane>,
) -> (i32, i32, std::cmp::Reverse<String>) {
    let Some(lane) = lane else {
        return (0, 0, std::cmp::Reverse(String::new()));
    };
    (
        lane_dispatch_priority_score(lane),
        lane_kind_priority(&lane.lane_kind),
        std::cmp::Reverse(lane.lane_key.clone()),
    )
}

fn lane_dispatch_priority_score(lane: &crate::evaluate::EvaluatedLane) -> i32 {
    let key = lane.lane_key.as_str();
    let unit = lane.unit_id.as_str();
    let mut score = 50;
    let failure_kind = failure_kind_for_lane(lane);

    if unit == "master" {
        score -= 40;
    }
    if unit.starts_with("phase-") && unit.ends_with("-gate") {
        score -= 30;
    }
    if unit.contains("-parent-") {
        score -= 25;
    }
    if unit.contains("document") || unit.contains("release") || unit.ends_with("-retro") {
        score -= 15;
    }
    if unit.contains("benchmark") {
        score -= 10;
    }
    if lane.lane_id.ends_with("-codex-unblock") {
        score -= 15;
    }

    if unit.contains("autodev-efficiency")
        || unit.contains("greenfield-bootstrap")
        || unit.contains("provider-policy")
        || unit.contains("test-coverage")
    {
        score += 40;
    } else if unit.contains("error-handling") || unit.contains("workspace-integration") {
        score += 30;
    } else if unit.contains("sprint-contracts") || unit.contains("genesis-onboarding") {
        score += 20;
    }

    if key.contains("live-validation") || key.contains("fresh-install-test") {
        score += 10;
    }
    if key.contains("regression") || key.contains("edge-case") {
        score += 20;
    }
    if key.contains("baseline-tests") {
        score += 10;
    }
    if key.contains("autodev-integration-test") || key.contains("ci-preservation") {
        score -= 10;
    }
    if matches!(
        failure_kind,
        Some(FailureKind::IntegrationTargetUnavailable)
    ) {
        score += 60;
        if !lane.lane_id.ends_with("-codex-unblock") {
            score += 40;
        }
    }
    if lane.lane_id.ends_with("-codex-unblock")
        && matches!(failure_kind, Some(FailureKind::ProofScriptFailure))
    {
        score -= 10;
    }

    score
}

fn lane_root_plan_family(
    lane: &crate::evaluate::EvaluatedLane,
    unit_ids: &BTreeSet<String>,
) -> String {
    let segments = lane.unit_id.split('-').collect::<Vec<_>>();
    for count in (1..=segments.len()).rev() {
        let prefix = segments[..count].join("-");
        let matches = unit_ids
            .iter()
            .filter(|unit_id| *unit_id == &prefix || unit_id.starts_with(&format!("{prefix}-")))
            .count();
        if matches >= 2 {
            return prefix;
        }
    }
    lane.unit_id.clone()
}

fn lane_kind_priority(kind: &crate::manifest::LaneKind) -> i32 {
    match kind {
        crate::manifest::LaneKind::Service => 6,
        crate::manifest::LaneKind::Interface => 5,
        crate::manifest::LaneKind::Integration => 4,
        crate::manifest::LaneKind::Platform => 3,
        crate::manifest::LaneKind::Artifact => 2,
        crate::manifest::LaneKind::Orchestration => 1,
        crate::manifest::LaneKind::Recurring => 0,
    }
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

fn capture_autodev_provenance(settings: &AutodevSettings) -> AutodevProvenance {
    let controller_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("raspberry"));
    AutodevProvenance {
        controller: capture_binary_provenance(&controller_path),
        fabro_bin: capture_binary_provenance(&settings.fabro_bin),
    }
}

fn capture_binary_provenance(path: &Path) -> BinaryProvenance {
    let version = Command::new(path)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if !output.status.success() {
                return None;
            }
            let stdout = String::from_utf8(output.stdout).ok()?;
            stdout
                .lines()
                .find(|line| !line.trim().is_empty())
                .map(|line| line.trim().to_string())
        });
    BinaryProvenance {
        path: path.display().to_string(),
        version,
    }
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
    write_atomic(path, &json).map_err(|source| AutodevError::WriteDoctrineState {
        path: path.to_path_buf(),
        source,
    })
}

fn write_atomic(path: &Path, contents: &str) -> Result<(), std::io::Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("state"),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    fs::write(&temp, contents)?;
    if let Err(first_error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(path);
        if let Err(second_error) = fs::rename(&temp, path) {
            let _ = fs::remove_file(&temp);
            return Err(std::io::Error::new(
                second_error.kind(),
                format!(
                    "atomic rename failed for {}: {first_error}; retry failed: {second_error}",
                    path.display()
                ),
            ));
        }
    }
    Ok(())
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

/// Fast-forward the target repo's local checkout to match origin/main.
///
/// Integration lanes push directly to origin via SSH, but the autodev
/// evaluate function checks the local filesystem for milestone artifacts.
/// Without this sync, artifacts exist on origin but not locally, so
/// milestones are never satisfied and lanes re-dispatch indefinitely.
fn current_git_branch(target_repo: &Path) -> Option<String> {
    Command::new("git")
        .current_dir(target_repo)
        .args(["symbolic-ref", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn current_origin_head_branch(target_repo: &Path) -> Option<String> {
    Command::new("git")
        .current_dir(target_repo)
        .args([
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .and_then(|branch| branch.strip_prefix("origin/").map(str::to_string))
}

fn branch_allowed_for_dispatch(current: &str, expected: &str) -> bool {
    current == expected
        || current == format!("autodev/{expected}")
        || current == format!("autodev-{expected}")
}

fn worktree_has_tracked_changes(target_repo: &Path) -> bool {
    Command::new("git")
        .current_dir(target_repo)
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| !output.stdout.is_empty())
}

fn ahead_behind_counts(target_repo: &Path, remote_ref: &str) -> Option<(usize, usize)> {
    let output = Command::new("git")
        .current_dir(target_repo)
        .args([
            "rev-list",
            "--left-right",
            "--count",
            &format!("HEAD...{remote_ref}"),
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let mut parts = raw.split_whitespace();
    let ahead = parts.next()?.parse().ok()?;
    let behind = parts.next()?.parse().ok()?;
    Some((ahead, behind))
}

pub(crate) fn ensure_target_repo_fresh_for_dispatch(
    manifest: &ProgramManifest,
    manifest_path: &Path,
) -> TargetRepoFreshness {
    let target_repo = manifest.resolved_target_repo(manifest_path);
    let has_origin = Command::new("git")
        .current_dir(&target_repo)
        .args(["remote", "get-url", "origin"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()
        .is_some_and(|status| status.success());
    if !has_origin {
        return TargetRepoFreshness::NoOrigin;
    }
    if let Err(error) = fabro_workflows::git::resolve_ssh_push_url(&target_repo, "origin") {
        return TargetRepoFreshness::InvalidPushRemote {
            reason: error.to_string(),
        };
    }
    let fetch = Command::new("git")
        .current_dir(&target_repo)
        .args(["fetch", "origin", "--quiet"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match &fetch {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!(
                "[autodev] git fetch origin failed (exit {}), target repo may be stale",
                s.code().unwrap_or(-1)
            );
            return TargetRepoFreshness::FetchFailed;
        }
        Err(e) => {
            eprintln!("[autodev] git fetch origin failed to spawn: {e}");
            return TargetRepoFreshness::FetchFailed;
        }
    }
    let Some(head_branch) = current_git_branch(&target_repo) else {
        return TargetRepoFreshness::Current;
    };
    let Some(origin_head_branch) = current_origin_head_branch(&target_repo) else {
        return TargetRepoFreshness::Current;
    };
    if !branch_allowed_for_dispatch(&head_branch, &origin_head_branch) {
        return TargetRepoFreshness::WrongBranch {
            current: head_branch,
            expected: origin_head_branch,
        };
    }
    let remote_ref = format!("origin/{origin_head_branch}");
    let Some((ahead, behind)) = ahead_behind_counts(&target_repo, &remote_ref) else {
        return TargetRepoFreshness::Current;
    };
    if behind == 0 {
        return if ahead > 0 {
            TargetRepoFreshness::LocalAhead
        } else {
            TargetRepoFreshness::Current
        };
    }
    if ahead > 0 {
        return TargetRepoFreshness::Diverged { ahead, behind };
    }
    if worktree_has_tracked_changes(&target_repo) {
        return TargetRepoFreshness::BehindWithLocalChanges { behind };
    }
    let merge = Command::new("git")
        .current_dir(&target_repo)
        .args(["merge", "--ff-only", &remote_ref])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match merge {
        Ok(status) if status.success() => TargetRepoFreshness::FastForwarded,
        Ok(status) => {
            eprintln!(
                "[autodev] git merge --ff-only {} failed (exit {})",
                remote_ref,
                status.code().unwrap_or(-1)
            );
            TargetRepoFreshness::MergeFailed
        }
        Err(e) => {
            eprintln!("[autodev] git merge --ff-only {} failed: {e}", remote_ref);
            TargetRepoFreshness::MergeFailed
        }
    }
}

fn dirty_worktree_paths(target_repo: &Path) -> Vec<String> {
    let output = Command::new("git")
        .current_dir(target_repo)
        .args(["status", "--porcelain", "--untracked-files=all"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let line = line.trim_end();
            if line.len() < 4 {
                return None;
            }
            let path = &line[3..];
            Some(
                path.rsplit_once(" -> ")
                    .map(|(_, dest)| dest)
                    .unwrap_or(path)
                    .to_string(),
            )
        })
        .collect()
}

fn is_generated_package_dirty_path(path: &str) -> bool {
    [
        ".raspberry/",
        "malinka/programs/",
        "malinka/workflows/",
        "malinka/run-configs/",
        "malinka/prompts/",
        "outputs/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

fn dirty_worktree_is_generated_package_only(target_repo: &Path) -> bool {
    let paths = dirty_worktree_paths(target_repo);
    !paths.is_empty()
        && paths
            .iter()
            .all(|path| is_generated_package_dirty_path(path.as_str()))
}

pub(crate) fn autoheal_generated_target_repo_for_dispatch(
    manifest: &ProgramManifest,
    manifest_path: &Path,
    fabro_bin: &Path,
    doctrine_files: &[PathBuf],
    evidence_paths: &[PathBuf],
    preview_evolve_root: Option<&Path>,
) -> bool {
    let target_repo = manifest.resolved_target_repo(manifest_path);
    if !dirty_worktree_is_generated_package_only(&target_repo) {
        return false;
    }

    let stash_message = format!(
        "fabro-generated-autosync-{}-{}",
        manifest.program,
        Utc::now().timestamp()
    );
    let stash = Command::new("git")
        .current_dir(&target_repo)
        .args(["stash", "push", "-u", "-m", &stash_message])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match stash {
        Ok(status) if status.success() => {}
        _ => return false,
    }

    let Some(origin_head_branch) = current_origin_head_branch(&target_repo) else {
        return false;
    };
    let remote_ref = format!("origin/{origin_head_branch}");
    let merge = Command::new("git")
        .current_dir(&target_repo)
        .args(["merge", "--ff-only", &remote_ref])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match merge {
        Ok(status) if status.success() => {}
        _ => return false,
    }

    rerender_program_package(
        manifest_path,
        manifest,
        fabro_bin,
        doctrine_files,
        evidence_paths,
        preview_evolve_root,
    )
    .is_ok()
}

fn sync_target_repo_to_origin(manifest: &ProgramManifest, manifest_path: &Path) {
    let _ = ensure_target_repo_fresh_for_dispatch(manifest, manifest_path);
}

fn maybe_sync_target_repo_to_origin(
    manifest: &ProgramManifest,
    manifest_path: &Path,
    last_repo_sync_at: &mut Option<DateTime<Utc>>,
) {
    let now = Utc::now();
    if last_repo_sync_at.as_ref().is_some_and(|last| {
        now.signed_duration_since(*last).num_seconds() < TARGET_REPO_SYNC_COOLDOWN_SECS
    }) {
        return;
    }
    sync_target_repo_to_origin(manifest, manifest_path);
    *last_repo_sync_at = Some(now);
}

pub(crate) fn autodev_cargo_target_dir(target_repo: &Path) -> PathBuf {
    target_repo
        .join(".raspberry")
        .join("cargo-target")
        .components()
        .collect()
}

const WIDE_AUTODEV_ADMISSION_THRESHOLD: usize = 10;

fn enforce_workspace_admission_gate(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    requested_max_parallel: usize,
) -> Result<(), AutodevError> {
    if requested_max_parallel < WIDE_AUTODEV_ADMISSION_THRESHOLD {
        return Ok(());
    }

    let target_repo = manifest.resolved_target_repo(manifest_path);
    let output = Command::new("cargo")
        .current_dir(&target_repo)
        .env("CARGO_TARGET_DIR", autodev_cargo_target_dir(&target_repo))
        .args(["check", "--workspace", "--message-format", "short"])
        .output()
        .map_err(|source| AutodevError::AdmissionCheckSpawn {
            target_repo: target_repo.clone(),
            source,
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(AutodevError::AdmissionCheckFailed {
            target_repo,
            message: summarize_admission_output(
                "cargo check --workspace failed; repair the workspace baseline before launching a wide autodev run",
                &stdout,
                &stderr,
            ),
        });
    }

    if cargo_output_has_warning(&stdout) || cargo_output_has_warning(&stderr) {
        return Err(AutodevError::AdmissionCheckFailed {
            target_repo,
            message: summarize_admission_output(
                "workspace emits warnings; zero-warning policy is required before launching a wide autodev run",
                &stdout,
                &stderr,
            ),
        });
    }

    Ok(())
}

fn cargo_output_has_warning(output: &str) -> bool {
    output.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("warning:") || trimmed.contains(": warning:")
    })
}

fn summarize_admission_output(summary: &str, stdout: &str, stderr: &str) -> String {
    let mut details = stdout
        .lines()
        .chain(stderr.lines())
        .filter(|line| !line.trim().is_empty())
        .take(8)
        .map(str::trim)
        .collect::<Vec<_>>();
    if details.is_empty() {
        summary.to_string()
    } else {
        details.insert(0, summary);
        details.join(" | ")
    }
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
    let report = deserialize_autodev_report(&path, manifest, &raw)?;
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
    let mut report = deserialize_autodev_report(&path, manifest, &raw)?;
    let preserved_max_parallel = report
        .current
        .as_ref()
        .and_then(|current| current.max_parallel);
    let next_snapshot = current_snapshot(program, preserved_max_parallel);
    let controller_active = autodev_controller_active(manifest_path, manifest)?;
    let next_stop_reason =
        synced_stop_reason(report.stop_reason, &next_snapshot, controller_active);
    if report.current.as_ref() == Some(&next_snapshot) && report.stop_reason == next_stop_reason {
        return Ok(());
    }
    report.current = Some(next_snapshot);
    report.stop_reason = next_stop_reason;
    report.updated_at = Utc::now();
    save_autodev_report(manifest_path, manifest, &report)?;
    Ok(())
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
    write_atomic(&path, &json).map_err(|source| AutodevError::WriteReport {
        path: path.clone(),
        source,
    })
}

fn deserialize_autodev_report(
    path: &Path,
    _manifest: &ProgramManifest,
    raw: &str,
) -> Result<AutodevReport, AutodevError> {
    serde_json::from_str(raw).map_err(|source| AutodevError::ParseReport {
        path: path.to_path_buf(),
        source,
    })
}

fn synced_stop_reason(
    previous: AutodevStopReason,
    snapshot: &AutodevCurrentSnapshot,
    controller_active: bool,
) -> AutodevStopReason {
    if controller_active {
        return AutodevStopReason::InProgress;
    }
    if snapshot.ready == 0 && snapshot.running == 0 {
        return AutodevStopReason::Settled;
    }
    previous
}

pub fn current_snapshot_for_program(
    program: &crate::evaluate::EvaluatedProgram,
    max_parallel: Option<usize>,
) -> AutodevCurrentSnapshot {
    current_snapshot(program, max_parallel)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct LaneProviderHint {
    provider: Option<String>,
    model: Option<String>,
}

fn current_snapshot(
    program: &crate::evaluate::EvaluatedProgram,
    max_parallel: Option<usize>,
) -> AutodevCurrentSnapshot {
    let mut ready = 0usize;
    let mut running = 0usize;
    let mut blocked = 0usize;
    let mut failed = 0usize;
    let mut complete = 0usize;
    let mut ready_lanes = Vec::new();
    let mut running_lanes = Vec::new();
    let mut failed_lanes = Vec::new();
    let mut provider_routes = Vec::new();
    let mut usage_by_provider: BTreeMap<String, ProviderUsageSnapshot> = BTreeMap::new();
    let recovering_failed_lanes = recovering_failed_lane_keys(program);

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
                if recovering_failed_lanes.contains(&lane.lane_key) {
                    blocked += 1;
                } else {
                    failed += 1;
                    failed_lanes.push(lane.lane_key.clone());
                }
            }
            LaneExecutionStatus::Complete => complete += 1,
        }

        if lane.status != LaneExecutionStatus::Running {
            let (nested_running, nested_running_lanes) =
                nested_child_running_from_detail(&lane.detail);
            running += nested_running;
            running_lanes.extend(nested_running_lanes);
        }

        let usage = lane
            .last_usage_summary
            .as_deref()
            .and_then(parse_usage_summary);
        let hint = lane_provider_hint(lane);
        let resolved_model = usage
            .as_ref()
            .map(|parsed| parsed.model.clone())
            .or_else(|| hint.as_ref().and_then(|value| value.model.clone()));
        let resolved_provider = usage
            .as_ref()
            .map(|parsed| parsed.provider.clone())
            .filter(|provider| provider != "unknown")
            .or_else(|| hint.as_ref().and_then(|value| value.provider.clone()))
            .or_else(|| usage.as_ref().map(|parsed| parsed.provider.clone()));

        if let Some(provider) = resolved_provider.clone() {
            provider_routes.push(LaneProviderRoute {
                lane_key: lane.lane_key.clone(),
                status: lane.status,
                model: resolved_model
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                provider,
            });
        }

        let Some(parsed) = usage else {
            continue;
        };
        let provider_key = resolved_provider.unwrap_or_else(|| parsed.provider.clone());
        let entry =
            usage_by_provider
                .entry(provider_key.clone())
                .or_insert(ProviderUsageSnapshot {
                    provider: provider_key,
                    input_tokens: 0,
                    output_tokens: 0,
                    cost_microusd: Some(0),
                });
        entry.input_tokens += parsed.input_tokens;
        entry.output_tokens += parsed.output_tokens;
        match (entry.cost_microusd, parsed.cost_estimate_usd) {
            (Some(current), Some(cost)) => {
                let microusd = (cost * 1_000_000.0).round();
                if microusd.is_finite() && microusd >= 0.0 {
                    entry.cost_microusd = Some(current.saturating_add(microusd as u64));
                } else {
                    entry.cost_microusd = None;
                }
            }
            (Some(_), None) => entry.cost_microusd = None,
            (None, _) => {}
        }
    }

    // Compute critical blockers: failed lanes ranked by how many downstream
    // lanes they transitively block.  The detail field for blocked lanes contains
    // the unsatisfied dependency, letting us attribute blocked lanes to their
    // blocking root.
    let failed_set: BTreeSet<&str> = failed_lanes.iter().map(|s| s.as_str()).collect();
    let mut blocker_counts: BTreeMap<String, usize> = BTreeMap::new();
    for lane in &program.lanes {
        if lane.status != LaneExecutionStatus::Blocked {
            continue;
        }
        // The detail field lists unsatisfied dependencies.  Check which failed
        // lanes appear in the dependency chain.
        for failed_key in &failed_set {
            if lane.detail.contains(failed_key) {
                *blocker_counts.entry(failed_key.to_string()).or_default() += 1;
            }
        }
    }
    let mut critical_blockers: Vec<CriticalBlocker> = blocker_counts
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(lane_key, blocked_downstream)| CriticalBlocker {
            lane_key,
            blocked_downstream,
        })
        .collect();
    critical_blockers.sort_by(|a, b| b.blocked_downstream.cmp(&a.blocked_downstream));
    critical_blockers.truncate(10);
    provider_routes.sort_by(|left, right| left.lane_key.cmp(&right.lane_key));

    AutodevCurrentSnapshot {
        updated_at: Utc::now(),
        max_parallel,
        ready,
        running,
        blocked,
        failed,
        complete,
        ready_lanes,
        running_lanes,
        failed_lanes,
        critical_blockers,
        provider_routes,
        usage_by_provider: usage_by_provider.into_values().collect(),
    }
}

fn lane_provider_hint(lane: &crate::evaluate::EvaluatedLane) -> Option<LaneProviderHint> {
    lane.current_run_dir
        .as_deref()
        .and_then(provider_hint_from_run_dir)
        .or_else(|| {
            lane.last_run_dir
                .as_deref()
                .and_then(provider_hint_from_run_dir)
        })
}

fn provider_hint_from_run_dir(run_dir: &Path) -> Option<LaneProviderHint> {
    let progress_path = run_dir.join("progress.jsonl");
    let raw = fs::read_to_string(progress_path).ok()?;
    let mut hint = LaneProviderHint::default();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let event = value
            .get("event")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        match event {
            "CliEnsureStarted" | "CliEnsureCompleted" => {
                if let Some(provider) = value.get("provider").and_then(|value| value.as_str()) {
                    hint.provider = Some(provider.to_string());
                }
                if let Some(model) = value.get("model").and_then(|value| value.as_str()) {
                    hint.model = Some(model.to_string());
                }
            }
            "StageCompleted" | "WorkflowRunCompleted" => {
                let Some(usage) = value.get("usage") else {
                    continue;
                };
                if let Some(provider) = usage.get("provider").and_then(|value| value.as_str()) {
                    hint.provider = Some(provider.to_string());
                }
                if let Some(model) = usage.get("model").and_then(|value| value.as_str()) {
                    hint.model = Some(model.to_string());
                }
            }
            _ => {}
        }
    }
    (hint.provider.is_some() || hint.model.is_some()).then_some(hint)
}

fn recovering_failed_lane_keys(program: &crate::evaluate::EvaluatedProgram) -> BTreeSet<String> {
    program
        .lanes
        .iter()
        .filter(|lane| lane.status == LaneExecutionStatus::Running)
        .filter_map(|lane| source_lane_key_for_codex_unblock(program, lane))
        .collect()
}

fn source_lane_key_for_codex_unblock(
    program: &crate::evaluate::EvaluatedProgram,
    running_lane: &crate::evaluate::EvaluatedLane,
) -> Option<String> {
    if !running_lane.lane_key.ends_with("-codex-unblock") {
        return None;
    }
    program
        .lanes
        .iter()
        .find(|candidate| {
            candidate.lane_key != running_lane.lane_key
                && codex_unblock_lane_key(&candidate.unit_id, &candidate.lane_id)
                    == running_lane.lane_key
        })
        .map(|candidate| candidate.lane_key.clone())
}

fn nested_child_running_from_detail(detail: &str) -> (usize, Vec<String>) {
    if !detail.starts_with("child program `") {
        return (0, Vec::new());
    }
    let Some(running_index) = detail.find(" running=") else {
        return (0, Vec::new());
    };
    let running_text = &detail[running_index + " running=".len()..];
    let digits = running_text
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    let nested_running = digits.parse::<usize>().unwrap_or(0);
    if nested_running == 0 {
        return (0, Vec::new());
    }
    let nested_lanes = detail
        .split(" | running_lanes=")
        .nth(1)
        .map(|value| {
            value
                .split(" | ")
                .next()
                .unwrap_or("")
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    (nested_running, nested_lanes)
}

fn lane_render_fingerprints(
    program: &crate::evaluate::EvaluatedProgram,
    lane_keys: &[String],
) -> Vec<(String, LaneRenderFingerprint)> {
    lane_keys
        .iter()
        .filter_map(|lane_key| {
            let lane = program
                .lanes
                .iter()
                .find(|lane| lane.lane_key == *lane_key)?;
            Some((lane_key.clone(), lane_render_fingerprint(&lane.run_config)))
        })
        .collect()
}

fn detect_regenerate_noop_lanes(
    regenerable_lanes: &[String],
    before: &[(String, LaneRenderFingerprint)],
    program: &crate::evaluate::EvaluatedProgram,
) -> Vec<String> {
    let mut noop_lanes = Vec::new();
    for lane_key in regenerable_lanes {
        let Some((_, before_fingerprint)) = before
            .iter()
            .find(|(candidate_lane, _)| candidate_lane == lane_key)
        else {
            continue;
        };
        let Some(lane) = program.lanes.iter().find(|lane| lane.lane_key == *lane_key) else {
            continue;
        };
        let after_fingerprint = lane_render_fingerprint(&lane.run_config);
        if *before_fingerprint == after_fingerprint {
            noop_lanes.push(lane_key.clone());
        }
    }
    noop_lanes
}

fn mark_regenerate_noop_lanes(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    program: &crate::evaluate::EvaluatedProgram,
    lane_keys: &[String],
    before: &[(String, LaneRenderFingerprint)],
) -> Result<(), AutodevError> {
    if lane_keys.is_empty() {
        return Ok(());
    }
    let state_path = manifest.resolved_state_path(manifest_path);
    let mut state = ProgramRuntimeState::load_optional(&state_path)?
        .unwrap_or_else(|| ProgramRuntimeState::new(&manifest.program));
    for lane_key in lane_keys {
        let Some(lane) = program.lanes.iter().find(|lane| lane.lane_key == *lane_key) else {
            continue;
        };
        let before_fingerprint = before
            .iter()
            .find(|(candidate_lane, _)| candidate_lane == lane_key)
            .map(|(_, fingerprint)| fingerprint.clone())
            .unwrap_or_else(|| lane_render_fingerprint(&lane.run_config));
        let detail = format!(
            "synth evolve did not materially change run config or graph (run_config_sha256={}, graph_sha256={})",
            before_fingerprint
                .run_config_sha256
                .as_deref()
                .unwrap_or("missing"),
            before_fingerprint.graph_sha256.as_deref().unwrap_or("missing"),
        );
        mark_lane_regenerate_noop(&mut state, lane_key, &lane.run_config, &detail);
    }
    state.save(&state_path)?;
    Ok(())
}

fn lane_render_fingerprint(run_config_path: &Path) -> LaneRenderFingerprint {
    let run_config_sha256 = file_sha256(run_config_path);
    let graph_sha256 = load_run_config(run_config_path)
        .ok()
        .map(|config| resolve_graph_path(run_config_path, &config.graph))
        .and_then(|graph_path| file_sha256(&graph_path));
    LaneRenderFingerprint {
        run_config_sha256,
        graph_sha256,
    }
}

fn file_sha256(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let digest = Sha256::digest(bytes);
    Some(format!("{digest:x}"))
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
        .filter(|lane| {
            matches!(
                lane.status,
                LaneExecutionStatus::Blocked | LaneExecutionStatus::Failed
            )
        })
        .filter_map(|lane| replay_target_lane(manifest, program, lane, now, allow_regenerate))
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
    recovery_decision_for_lane(lane).map(|(kind, _)| kind)
}

fn recovery_decision_for_lane(
    lane: &crate::evaluate::EvaluatedLane,
) -> Option<(FailureKind, FailureRecoveryAction)> {
    let classified = classify_failure(
        lane.last_error.as_deref(),
        lane.last_stderr_snippet.as_deref(),
        lane.last_stdout_snippet.as_deref(),
    );
    match (lane.failure_kind, classified) {
        (Some(FailureKind::Unknown), Some(kind)) if kind != FailureKind::Unknown => {
            Some((kind, default_recovery_action(kind)))
        }
        (Some(kind), _) => Some((
            kind,
            lane.recovery_action
                .unwrap_or_else(|| default_recovery_action(kind)),
        )),
        (None, Some(kind)) => Some((kind, default_recovery_action(kind))),
        (None, None) => None,
    }
}

fn replay_target_lane(
    manifest: &ProgramManifest,
    program: &crate::evaluate::EvaluatedProgram,
    lane: &crate::evaluate::EvaluatedLane,
    now: DateTime<Utc>,
    allow_regenerate: bool,
) -> Option<String> {
    let (kind, action) = recovery_decision_for_lane(lane)?;
    if is_verify_gate_miss_without_retry(lane) {
        return allow_regenerate.then(|| lane.lane_key.clone());
    }
    if let Some(target) = codex_unblock_recovery_target(program, lane, kind, now) {
        return Some(target);
    }
    match action {
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
            let cooldown = if kind == FailureKind::TransientLaunchFailure {
                TRANSIENT_LAUNCH_RETRY_MIN_SECS
            } else if kind == FailureKind::EnvironmentCollision {
                ENVIRONMENT_COLLISION_RETRY_MIN_SECS
            } else {
                BACKOFF_RETRY_MIN_SECS
            };
            retry_after_cooldown(lane, now, cooldown, lane.lane_key.clone())
        }
        FailureRecoveryAction::RegenerateLane => {
            if allow_regenerate {
                Some(lane.lane_key.clone())
            } else {
                None
            }
        }
        FailureRecoveryAction::SurfaceBlocked => {
            if lane.consecutive_failures >= MAX_SURFACE_BLOCKED_RETRIES {
                return None;
            }
            retry_after_cooldown(
                lane,
                now,
                SURFACE_BLOCKED_RETRY_MIN_SECS,
                lane.lane_key.clone(),
            )
        }
    }
}

fn codex_unblock_recovery_target(
    program: &crate::evaluate::EvaluatedProgram,
    lane: &crate::evaluate::EvaluatedLane,
    kind: FailureKind,
    now: DateTime<Utc>,
) -> Option<String> {
    if lane.lane_id.ends_with("-codex-unblock") || !should_use_codex_unblock(kind) {
        return None;
    }
    let unblock_lane_key = codex_unblock_lane_key(&lane.unit_id, &lane.lane_id);
    let unblock = program
        .lanes
        .iter()
        .find(|candidate| candidate.lane_key == unblock_lane_key)?;
    match unblock.status {
        LaneExecutionStatus::Running => None,
        LaneExecutionStatus::Complete => {
            if unblock_finished_after_source_failure(unblock, lane) {
                Some(lane.lane_key.clone())
            } else {
                retry_after_cooldown(
                    lane,
                    now,
                    CODEX_UNBLOCK_RETRY_MIN_SECS,
                    unblock.lane_key.clone(),
                )
            }
        }
        LaneExecutionStatus::Blocked | LaneExecutionStatus::Ready => retry_after_cooldown(
            lane,
            now,
            CODEX_UNBLOCK_RETRY_MIN_SECS,
            unblock.lane_key.clone(),
        ),
        LaneExecutionStatus::Failed => None,
    }
}

fn should_use_codex_unblock(kind: FailureKind) -> bool {
    matches!(
        kind,
        FailureKind::ProviderAccessLimited
            | FailureKind::TransientLaunchFailure
            | FailureKind::ProviderPolicyMismatch
    )
}

fn codex_unblock_lane_key(unit_id: &str, lane_id: &str) -> String {
    let base = if unit_id == lane_id {
        format!("{unit_id}-codex-unblock")
    } else {
        format!("{unit_id}-{lane_id}-codex-unblock")
    };
    format!("{base}:{base}")
}

fn unblock_finished_after_source_failure(
    unblock: &crate::evaluate::EvaluatedLane,
    source: &crate::evaluate::EvaluatedLane,
) -> bool {
    match (
        unblock.last_finished_at.or(unblock.last_started_at),
        source.last_finished_at.or(source.last_started_at),
    ) {
        (Some(unblock_time), Some(source_time)) => unblock_time > source_time,
        (Some(_), None) => true,
        _ => false,
    }
}

fn is_verify_gate_miss_without_retry(lane: &crate::evaluate::EvaluatedLane) -> bool {
    let error = lane
        .last_error
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    error.contains("goal gate unsatisfied for node verify") && error.contains("no retry target")
}

fn regenerable_failed_lanes(program: &crate::evaluate::EvaluatedProgram) -> Vec<String> {
    let mut lanes = program
        .lanes
        .iter()
        .filter(|lane| {
            matches!(
                lane.status,
                LaneExecutionStatus::Blocked | LaneExecutionStatus::Failed
            )
        })
        .filter_map(|lane| {
            if is_verify_gate_miss_without_retry(lane) {
                return Some(lane.lane_key.clone());
            }
            let (_, action) = recovery_decision_for_lane(lane)?;
            (action == FailureRecoveryAction::RegenerateLane).then(|| lane.lane_key.clone())
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
            let (kind, action) = recovery_decision_for_lane(lane)?;
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
    let lane_name = dependency.lane.as_deref()?;
    Some(format!("{}:{}", dependency.unit, lane_name))
}

fn run_synth_evolve(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    settings: &AutodevSettings,
) -> Result<(), AutodevError> {
    rerender_program_package(
        manifest_path,
        manifest,
        &settings.fabro_bin,
        &settings.doctrine_files,
        &settings.evidence_paths,
        settings.preview_evolve_root.as_deref(),
    )
}

fn rerender_program_package(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    fabro_bin: &Path,
    doctrine_files: &[PathBuf],
    evidence_paths: &[PathBuf],
    preview_evolve_root: Option<&Path>,
) -> Result<(), AutodevError> {
    let target_repo = manifest.resolved_target_repo(manifest_path);
    let temp_dir = autodev_temp_dir(&manifest.program)?;
    let source_blueprint = manifest_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!("blueprints/{}.yaml", manifest.program));
    if source_blueprint.exists() {
        let copied = temp_dir.join(format!("{}-autodev.yaml", manifest.program));
        fs::copy(&source_blueprint, &copied).map_err(|source| AutodevError::Spawn {
            step: "copy source blueprint".to_string(),
            program: manifest.program.clone(),
            source,
        })?;
        inject_inputs_into_blueprint(&copied, doctrine_files, evidence_paths)?;
    }

    // Deterministic autodev steering should use the same CLI path operators use by
    // hand: `synth evolve --no-review`. That keeps the runtime loop aligned with the
    // direct end-to-end workflow instead of using a hidden import+create rerender path.
    let mut evolve = Command::new("timeout");
    evolve
        .arg("--signal=TERM")
        .arg(SYNTH_EVOLVE_TIMEOUT_SECS.to_string())
        .arg(fabro_bin)
        .current_dir(&target_repo)
        .env("CARGO_TARGET_DIR", autodev_cargo_target_dir(&target_repo))
        .arg("--no-upgrade-check")
        .arg("synth")
        .arg("evolve")
        .arg("--no-review")
        .arg("--target-repo")
        .arg(&target_repo)
        .arg("--program")
        .arg(&manifest.program);
    if let Some(preview_root) = preview_evolve_root {
        evolve.arg("--preview-root").arg(preview_root);
    }
    let output = evolve.output().map_err(|source| AutodevError::Spawn {
        step: "synth evolve".to_string(),
        program: manifest.program.clone(),
        source,
    })?;
    ensure_success("synth evolve", &manifest.program, output)?;

    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
}

fn maybe_refresh_paperclip_dashboard(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    settings: &AutodevSettings,
    last_refresh_at: &mut Option<Instant>,
) {
    let Some(bundle_root) = paperclip_bundle_root(manifest_path, manifest) else {
        return;
    };
    if !bundle_root.join("bootstrap-state.json").exists() {
        return;
    }
    if last_refresh_at
        .as_ref()
        .is_some_and(|instant| instant.elapsed() < Duration::from_secs(PAPERCLIP_REFRESH_MIN_SECS))
    {
        return;
    }

    let target_repo = manifest.resolved_target_repo(manifest_path);
    let _ = Command::new(&settings.fabro_bin)
        .current_dir(&target_repo)
        .env("CARGO_TARGET_DIR", autodev_cargo_target_dir(&target_repo))
        .arg("--no-upgrade-check")
        .arg("paperclip")
        .arg("refresh")
        .arg("--target-repo")
        .arg(&target_repo)
        .arg("--program")
        .arg(&manifest.program)
        .output();
    *last_refresh_at = Some(Instant::now());
}

fn paperclip_bundle_root(manifest_path: &Path, manifest: &ProgramManifest) -> Option<PathBuf> {
    let target_repo = manifest.resolved_target_repo(manifest_path);
    let root = target_repo
        .join("malinka")
        .join("paperclip")
        .join(&manifest.program);
    root.exists().then_some(root)
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
    use std::process::Command;
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
            run_config: PathBuf::from("malinka/run-configs/bootstrap/demo.toml"),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: None,
            current_run_dir: None,
            last_run_dir: None,
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
            consecutive_failures: 0,
        }
    }

    fn failed_lane_with_kind(
        lane_key: &str,
        error: &str,
        failure_kind: FailureKind,
    ) -> EvaluatedLane {
        let mut lane = failed_lane(lane_key, error);
        lane.failure_kind = Some(failure_kind);
        lane.detail = format!("failure_kind={failure_kind}");
        lane
    }

    fn ready_lane(
        lane_key: &str,
        unit_id: &str,
        lane_id: &str,
        lane_kind: LaneKind,
    ) -> EvaluatedLane {
        EvaluatedLane {
            lane_key: lane_key.to_string(),
            unit_id: unit_id.to_string(),
            unit_title: unit_id.to_string(),
            lane_id: lane_id.to_string(),
            lane_title: lane_id.to_string(),
            lane_kind,
            status: LaneExecutionStatus::Ready,
            operational_state: None,
            precondition_state: None,
            proof_state: None,
            orchestration_state: None,
            detail: String::new(),
            managed_milestone: "reviewed".to_string(),
            proof_profile: None,
            run_config: PathBuf::from("malinka/run-configs/bootstrap/demo.toml"),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: None,
            current_run_dir: None,
            last_run_dir: None,
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
            consecutive_failures: 0,
        }
    }

    #[test]
    fn current_snapshot_captures_provider_routes_and_usage() {
        let mut running = ready_lane("runtime:page", "runtime", "page", LaneKind::Artifact);
        running.status = LaneExecutionStatus::Running;
        running.last_usage_summary = Some("gpt-5.4: 1200 in / 800 out".to_string());

        let mut failed = failed_lane("consensus:chapter", "provider rejected request");
        failed.last_usage_summary = Some("claude-opus-4-6: 300 in / 40 out".to_string());

        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 2,
            runtime_max_parallel: Some(2),
            lanes: vec![running, failed],
        };

        let snapshot = current_snapshot_for_program(&program, Some(2));

        assert_eq!(snapshot.provider_routes.len(), 2);
        assert!(snapshot
            .provider_routes
            .iter()
            .any(|route| route.lane_key == "runtime:page"
                && route.model == "gpt-5.4"
                && route.provider == "openai"));
        assert!(snapshot
            .usage_by_provider
            .iter()
            .any(|usage| usage.provider == "openai"
                && usage.input_tokens == 1200
                && usage.output_tokens == 800));
        assert!(snapshot
            .usage_by_provider
            .iter()
            .any(|usage| usage.provider == "anthropic"
                && usage.input_tokens == 300
                && usage.output_tokens == 40));
    }

    #[test]
    fn current_snapshot_recovers_provider_routes_from_progress_log_hints() {
        let temp = tempfile::tempdir().expect("tempdir");
        let run_dir = temp.path().join("run");
        fs::create_dir_all(&run_dir).expect("create run dir");
        fs::write(
            run_dir.join("progress.jsonl"),
            concat!(
                "{\"event\":\"CliEnsureCompleted\",\"provider\":\"openai\",\"model\":\"k2p5\"}\n",
                "{\"event\":\"StageCompleted\",\"usage\":{\"model\":\"k2p5\",\"input_tokens\":10,\"output_tokens\":3}}\n",
            ),
        )
        .expect("write progress");

        let mut running = ready_lane("runtime:page", "runtime", "page", LaneKind::Artifact);
        running.status = LaneExecutionStatus::Running;
        running.current_run_dir = Some(run_dir.clone());
        running.last_usage_summary = Some("k2p5: 10 in / 3 out".to_string());

        let mut ready = ready_lane("runtime:queued", "runtime", "queued", LaneKind::Artifact);
        ready.current_run_dir = Some(run_dir);

        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 2,
            runtime_max_parallel: Some(2),
            lanes: vec![running, ready],
        };

        let snapshot = current_snapshot_for_program(&program, Some(2));

        assert!(snapshot.provider_routes.iter().any(|route| {
            route.lane_key == "runtime:page" && route.model == "k2p5" && route.provider == "openai"
        }));
        assert!(snapshot.provider_routes.iter().any(|route| {
            route.lane_key == "runtime:queued"
                && route.model == "k2p5"
                && route.provider == "openai"
        }));
        assert!(snapshot.usage_by_provider.iter().any(|usage| {
            usage.provider == "openai" && usage.input_tokens == 10 && usage.output_tokens == 3
        }));
    }

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(dir)
            .args(args)
            .status()
            .expect("git command spawns");
        assert!(
            status.success(),
            "git {:?} failed with status {:?}",
            args,
            status.code()
        );
    }

    fn demo_manifest(target_repo: &Path) -> ProgramManifest {
        ProgramManifest {
            program: "demo".to_string(),
            target_repo: target_repo.to_path_buf(),
            state_path: target_repo.join(".raspberry/demo-state.json"),
            max_parallel: 1,
            run_dir: None,
            units: BTreeMap::new(),
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
            runtime_max_parallel: None,
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
    fn replayable_failed_lanes_include_proof_failures() {
        let manifest = ProgramManifest::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml"),
        )
        .expect("manifest loads");
        let mut lane = failed_lane("proof:lane", "Script failed with exit code: 101");
        lane.last_started_at = Some(Utc::now() - chrono::Duration::minutes(5));
        lane.last_finished_at = Some(Utc::now() - chrono::Duration::minutes(1));
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            runtime_max_parallel: None,
            lanes: vec![lane],
        };

        let lanes = replayable_failed_lanes(&manifest, &program);

        assert_eq!(lanes, vec!["proof:lane".to_string()]);
    }

    #[test]
    fn regenerable_failed_lanes_include_blocked_supervisor_only_lanes() {
        let mut lane = failed_lane(
            "blocked:lane",
            "repo-level orchestration lanes are executed directly by raspberry supervisor",
        );
        lane.status = LaneExecutionStatus::Blocked;
        lane.failure_kind = Some(FailureKind::SupervisorOnlyLane);
        lane.recovery_action = Some(FailureRecoveryAction::RegenerateLane);
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            runtime_max_parallel: None,
            lanes: vec![lane],
        };

        let lanes = regenerable_failed_lanes(&program);

        assert_eq!(lanes, vec!["blocked:lane".to_string()]);
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
            runtime_max_parallel: None,
            lanes: vec![lane],
        };

        let lanes = dispatchable_failed_lanes(&manifest, &program, true);

        assert_eq!(lanes, vec!["broken:lane".to_string()]);
    }

    #[test]
    fn child_program_manifests_to_advance_uses_spare_slots_for_failed_children() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_path = temp.path().join("program.yaml");
        std::fs::create_dir_all(temp.path().join("malinka/programs")).expect("program dir");
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
        run_config: malinka/programs/alpha.yaml
        managed_milestone: coordinated
        program_manifest: malinka/programs/alpha.yaml
  - id: beta
    title: Beta
    output_root: out/beta
    artifacts: []
    milestones: []
    lanes:
      - id: program
        kind: orchestration
        title: Beta Program
        run_config: malinka/programs/beta.yaml
        managed_milestone: coordinated
        program_manifest: malinka/programs/beta.yaml
  - id: gamma
    title: Gamma
    output_root: out/gamma
    artifacts: []
    milestones: []
    lanes:
      - id: program
        kind: orchestration
        title: Gamma Program
        run_config: malinka/programs/gamma.yaml
        managed_milestone: coordinated
        program_manifest: malinka/programs/gamma.yaml
"#,
        )
        .expect("manifest written");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 5,
            runtime_max_parallel: None,
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
                    run_config: PathBuf::from("malinka/programs/alpha.yaml"),
                    run_id: None,
                    current_run_id: None,
                    current_fabro_run_id: None,
                    current_stage: None,
                    last_run_id: None,
                    current_run_dir: None,
                    last_run_dir: None,
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
                    consecutive_failures: 0,
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
                    run_config: PathBuf::from("malinka/programs/beta.yaml"),
                    run_id: None,
                    current_run_id: None,
                    current_fabro_run_id: None,
                    current_stage: None,
                    last_run_id: None,
                    current_run_dir: None,
                    last_run_dir: None,
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
                    consecutive_failures: 0,
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
                    run_config: PathBuf::from("malinka/programs/gamma.yaml"),
                    run_id: None,
                    current_run_id: Some("01RUNNING".to_string()),
                    current_fabro_run_id: Some("01RUNNING".to_string()),
                    current_stage: Some("Implement".to_string()),
                    last_run_id: Some("01RUNNING".to_string()),
                    current_run_dir: None,
                    last_run_dir: None,
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
                    consecutive_failures: 0,
                },
            ],
        };

        let manifests = child_program_manifests_to_advance(&manifest_path, &manifest, &program, 1);

        assert_eq!(manifests.len(), 1);
        assert!(manifests[0].ends_with("malinka/programs/alpha.yaml"));
    }

    #[test]
    fn child_program_manifests_to_advance_includes_ready_children() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_path = temp.path().join("program.yaml");
        std::fs::create_dir_all(temp.path().join("malinka/programs")).expect("program dir");
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
        run_config: malinka/programs/alpha.yaml
        managed_milestone: coordinated
        program_manifest: malinka/programs/alpha.yaml
  - id: beta
    title: Beta
    output_root: out/beta
    artifacts: []
    milestones: []
    lanes:
      - id: program
        kind: orchestration
        title: Beta Program
        run_config: malinka/programs/beta.yaml
        managed_milestone: coordinated
        program_manifest: malinka/programs/beta.yaml
"#,
        )
        .expect("manifest written");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 5,
            runtime_max_parallel: None,
            lanes: vec![
                EvaluatedLane {
                    lane_key: "alpha:program".to_string(),
                    unit_id: "alpha".to_string(),
                    unit_title: "Alpha".to_string(),
                    lane_id: "program".to_string(),
                    lane_title: "Alpha Program".to_string(),
                    lane_kind: LaneKind::Orchestration,
                    status: LaneExecutionStatus::Ready,
                    operational_state: None,
                    precondition_state: None,
                    proof_state: None,
                    orchestration_state: None,
                    detail: String::new(),
                    managed_milestone: "coordinated".to_string(),
                    proof_profile: None,
                    run_config: PathBuf::from("malinka/programs/alpha.yaml"),
                    run_id: None,
                    current_run_id: None,
                    current_fabro_run_id: None,
                    current_stage: None,
                    last_run_id: None,
                    current_run_dir: None,
                    last_run_dir: None,
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
                    consecutive_failures: 0,
                },
                EvaluatedLane {
                    lane_key: "beta:program".to_string(),
                    unit_id: "beta".to_string(),
                    unit_title: "Beta".to_string(),
                    lane_id: "program".to_string(),
                    lane_title: "Beta Program".to_string(),
                    lane_kind: LaneKind::Orchestration,
                    status: LaneExecutionStatus::Blocked,
                    operational_state: None,
                    precondition_state: None,
                    proof_state: None,
                    orchestration_state: None,
                    detail: String::new(),
                    managed_milestone: "coordinated".to_string(),
                    proof_profile: None,
                    run_config: PathBuf::from("malinka/programs/beta.yaml"),
                    run_id: None,
                    current_run_id: None,
                    current_fabro_run_id: None,
                    current_stage: None,
                    last_run_id: None,
                    current_run_dir: None,
                    last_run_dir: None,
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
                    consecutive_failures: 0,
                },
            ],
        };

        let manifests = child_program_manifests_to_advance(&manifest_path, &manifest, &program, 1);

        assert_eq!(manifests.len(), 1);
        assert!(manifests[0].ends_with("malinka/programs/alpha.yaml"));
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
            run_config: PathBuf::from("malinka/run-configs/integrate/demo.toml"),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: None,
            current_run_dir: None,
            last_run_dir: None,
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
            consecutive_failures: 0,
        };

        let target = replay_target_lane(
            &manifest,
            &crate::evaluate::EvaluatedProgram {
                program: manifest.program.clone(),
                max_parallel: manifest.max_parallel,
                runtime_max_parallel: None,
                lanes: vec![lane.clone()],
            },
            &lane,
            Utc::now(),
            false,
        );

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
    fn replayable_failed_lanes_reclassify_unknown_failures_from_last_error() {
        let manifest = ProgramManifest::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml"),
        )
        .expect("manifest loads");
        let mut lane = failed_lane(
            "broken:lane",
            "thread 'main' panicked: failed printing to stdout: Quota exceeded (os error 122)",
        );
        lane.failure_kind = Some(FailureKind::Unknown);
        lane.recovery_action = Some(FailureRecoveryAction::SurfaceBlocked);
        lane.last_started_at = Some(Utc::now() - chrono::Duration::minutes(30));
        lane.last_finished_at = Some(Utc::now() - chrono::Duration::minutes(20));
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            runtime_max_parallel: None,
            lanes: vec![lane],
        };

        let lanes = replayable_failed_lanes(&manifest, &program);

        assert_eq!(lanes, vec!["broken:lane".to_string()]);
    }

    #[test]
    fn replayable_failed_lanes_upgrade_unknown_recovery_action_from_last_error() {
        let manifest = ProgramManifest::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml"),
        )
        .expect("manifest loads");
        let mut lane = failed_lane(
            "house-agent-ws-accept-loop:house-agent-ws-accept-loop",
            "failed to resolve SSH push URL: Engine error: remote `origin` must use SSH or be convertible from GitHub HTTPS, got `/tmp/controller`",
        );
        lane.unit_id = "house-agent-ws-accept-loop".to_string();
        lane.lane_id = "house-agent-ws-accept-loop".to_string();
        lane.lane_kind = LaneKind::Service;
        lane.failure_kind = Some(FailureKind::Unknown);
        lane.recovery_action = Some(FailureRecoveryAction::SurfaceBlocked);
        lane.last_started_at = Some(Utc::now() - chrono::Duration::minutes(2));
        lane.last_finished_at = Some(Utc::now() - chrono::Duration::minutes(1));
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            runtime_max_parallel: None,
            lanes: vec![lane],
        };

        let lanes = replayable_failed_lanes(&manifest, &program);

        assert_eq!(
            lanes,
            vec!["house-agent-ws-accept-loop:house-agent-ws-accept-loop".to_string()]
        );
    }

    #[test]
    fn prioritized_failure_lane_keys_prefer_source_replay_after_remote_fix() {
        let manifest = ProgramManifest::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml"),
        )
        .expect("manifest loads");
        let remote_error = "failed to resolve SSH push URL: Engine error: remote `origin` must use SSH or be convertible from GitHub HTTPS, got `/tmp/controller`";

        let mut house = failed_lane(
            "house-agent-ws-accept-loop:house-agent-ws-accept-loop",
            remote_error,
        );
        house.unit_id = "house-agent-ws-accept-loop".to_string();
        house.lane_id = "house-agent-ws-accept-loop".to_string();
        house.lane_kind = LaneKind::Service;
        house.failure_kind = Some(FailureKind::Unknown);
        house.recovery_action = Some(FailureRecoveryAction::SurfaceBlocked);
        house.last_started_at = Some(Utc::now() - chrono::Duration::minutes(30));
        house.last_finished_at = Some(Utc::now() - chrono::Duration::minutes(20));

        let mut unblock = failed_lane(
            "test-coverage-wallet-rpc-tests-codex-unblock:test-coverage-wallet-rpc-tests-codex-unblock",
            remote_error,
        );
        unblock.unit_id = "test-coverage-wallet-rpc-tests-codex-unblock".to_string();
        unblock.lane_id = "test-coverage-wallet-rpc-tests-codex-unblock".to_string();
        unblock.lane_kind = LaneKind::Platform;
        unblock.failure_kind = Some(FailureKind::Unknown);
        unblock.recovery_action = Some(FailureRecoveryAction::SurfaceBlocked);
        unblock.last_started_at = Some(Utc::now() - chrono::Duration::minutes(30));
        unblock.last_finished_at = Some(Utc::now() - chrono::Duration::minutes(20));

        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 2,
            runtime_max_parallel: None,
            lanes: vec![house, unblock],
        };

        let replayable = dispatchable_failed_lanes(&manifest, &program, false);
        let prioritized = prioritized_failure_lane_keys(&program, replayable);

        assert_eq!(
            prioritized,
            vec![
                "house-agent-ws-accept-loop:house-agent-ws-accept-loop".to_string(),
                "test-coverage-wallet-rpc-tests-codex-unblock:test-coverage-wallet-rpc-tests-codex-unblock"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn replayable_failed_lanes_dispatch_codex_unblock_for_provider_access_limits() {
        let manifest = ProgramManifest::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml"),
        )
        .expect("manifest loads");
        let mut lane = failed_lane(
            "blocked:blocked",
            "You've hit your usage limit. Visit https://chatgpt.com/codex/settings/usage to purchase more credits or try again at Mar 25th, 2026 1:07 PM.",
        );
        lane.unit_id = "blocked".to_string();
        lane.lane_id = "blocked".to_string();
        lane.failure_kind = Some(FailureKind::ProviderAccessLimited);
        lane.recovery_action = Some(FailureRecoveryAction::SurfaceBlocked);
        lane.last_started_at = Some(Utc::now() - chrono::Duration::minutes(20));
        lane.last_finished_at = Some(Utc::now() - chrono::Duration::minutes(10));
        let unblock = EvaluatedLane {
            lane_key: "blocked-codex-unblock:blocked-codex-unblock".to_string(),
            unit_id: "blocked-codex-unblock".to_string(),
            unit_title: "Blocked Codex Unblock".to_string(),
            lane_id: "blocked-codex-unblock".to_string(),
            lane_title: "Blocked Codex Unblock".to_string(),
            lane_kind: LaneKind::Platform,
            status: LaneExecutionStatus::Blocked,
            operational_state: None,
            precondition_state: None,
            proof_state: None,
            orchestration_state: None,
            detail: String::new(),
            managed_milestone: "blocked-codex-unblock-done".to_string(),
            proof_profile: Some("unblock".to_string()),
            run_config: PathBuf::from("run-configs/codex-unblock/blocked-codex-unblock.toml"),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: None,
            current_run_dir: None,
            last_run_dir: None,
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
            consecutive_failures: 0,
        };
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            runtime_max_parallel: None,
            lanes: vec![lane, unblock],
        };

        let lanes = replayable_failed_lanes(&manifest, &program);

        assert_eq!(
            lanes,
            vec!["blocked-codex-unblock:blocked-codex-unblock".to_string()]
        );
    }

    #[test]
    fn replayable_failed_lanes_retry_surface_blocked_after_cooldown() {
        let manifest = ProgramManifest::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml"),
        )
        .expect("manifest loads");
        let mut lane = failed_lane(
            "blocked:lane",
            "You've hit your usage limit. Visit https://chatgpt.com/codex/settings/usage to purchase more credits or try again later.",
        );
        lane.status = LaneExecutionStatus::Blocked;
        lane.failure_kind = Some(FailureKind::ProviderAccessLimited);
        lane.recovery_action = Some(FailureRecoveryAction::SurfaceBlocked);
        lane.last_started_at = Some(Utc::now() - chrono::Duration::minutes(30));
        lane.last_finished_at = Some(Utc::now() - chrono::Duration::minutes(20));
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            runtime_max_parallel: None,
            lanes: vec![lane],
        };

        let lanes = replayable_failed_lanes(&manifest, &program);

        assert_eq!(lanes, vec!["blocked:lane".to_string()]);
    }

    #[test]
    fn replayable_failed_lanes_retry_source_lane_for_regenerate_noop() {
        let manifest = ProgramManifest::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml"),
        )
        .expect("manifest loads");
        let mut source = failed_lane(
            "poker-tui-screen:poker-tui-screen",
            "synth evolve did not materially change run config or graph",
        );
        source.unit_id = "poker-tui-screen".to_string();
        source.lane_id = "poker-tui-screen".to_string();
        source.status = LaneExecutionStatus::Failed;
        source.failure_kind = Some(FailureKind::RegenerateNoop);
        source.recovery_action = Some(FailureRecoveryAction::SurfaceBlocked);
        source.last_started_at = Some(Utc::now() - chrono::Duration::minutes(20));
        source.last_finished_at = Some(Utc::now() - chrono::Duration::minutes(16));

        let unblock = EvaluatedLane {
            lane_key: "poker-tui-screen-codex-unblock:poker-tui-screen-codex-unblock".to_string(),
            unit_id: "poker-tui-screen-codex-unblock".to_string(),
            unit_title: "Poker TUI Screen Codex Unblock".to_string(),
            lane_id: "poker-tui-screen-codex-unblock".to_string(),
            lane_title: "Poker TUI Screen Codex Unblock".to_string(),
            lane_kind: LaneKind::Platform,
            status: LaneExecutionStatus::Blocked,
            operational_state: None,
            precondition_state: None,
            proof_state: None,
            orchestration_state: None,
            detail: String::new(),
            managed_milestone: "poker-tui-screen-codex-unblock-done".to_string(),
            proof_profile: Some("unblock".to_string()),
            run_config: PathBuf::from(
                "run-configs/codex-unblock/poker-tui-screen-codex-unblock.toml",
            ),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: None,
            current_run_dir: None,
            last_run_dir: None,
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
            consecutive_failures: 0,
        };
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            runtime_max_parallel: None,
            lanes: vec![source, unblock],
        };

        let lanes = replayable_failed_lanes(&manifest, &program);

        assert_eq!(lanes, vec!["poker-tui-screen:poker-tui-screen".to_string()]);
    }

    #[test]
    fn replayable_failed_lanes_dispatch_codex_unblock_for_transient_launch_failures() {
        let manifest = ProgramManifest::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml"),
        )
        .expect("manifest loads");
        let mut source = failed_lane(
            "wallet-rpc-tests:wallet-rpc-tests",
            "cli command exited with code 1 and total_cost_usd=0 input_tokens=0 output_tokens=0",
        );
        source.unit_id = "wallet-rpc-tests".to_string();
        source.lane_id = "wallet-rpc-tests".to_string();
        source.status = LaneExecutionStatus::Failed;
        source.failure_kind = Some(FailureKind::TransientLaunchFailure);
        source.recovery_action = Some(FailureRecoveryAction::BackoffRetry);
        source.last_started_at = Some(Utc::now() - chrono::Duration::minutes(5));
        source.last_finished_at = Some(Utc::now() - chrono::Duration::minutes(2));

        let unblock = EvaluatedLane {
            lane_key: "wallet-rpc-tests-codex-unblock:wallet-rpc-tests-codex-unblock".to_string(),
            unit_id: "wallet-rpc-tests-codex-unblock".to_string(),
            unit_title: "Wallet Rpc Tests Codex Unblock".to_string(),
            lane_id: "wallet-rpc-tests-codex-unblock".to_string(),
            lane_title: "Wallet Rpc Tests Codex Unblock".to_string(),
            lane_kind: LaneKind::Platform,
            status: LaneExecutionStatus::Ready,
            operational_state: None,
            precondition_state: None,
            proof_state: None,
            orchestration_state: None,
            detail: String::new(),
            managed_milestone: "wallet-rpc-tests-codex-unblock-done".to_string(),
            proof_profile: Some("unblock".to_string()),
            run_config: PathBuf::from(
                "run-configs/codex-unblock/wallet-rpc-tests-codex-unblock.toml",
            ),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: None,
            current_run_dir: None,
            last_run_dir: None,
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
            consecutive_failures: 0,
        };
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            runtime_max_parallel: None,
            lanes: vec![source, unblock],
        };

        let lanes = replayable_failed_lanes(&manifest, &program);

        assert_eq!(
            lanes,
            vec!["wallet-rpc-tests-codex-unblock:wallet-rpc-tests-codex-unblock".to_string()]
        );
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
            run_config: PathBuf::from("malinka/run-configs/bootstrap/demo.toml"),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: None,
            current_run_dir: None,
            last_run_dir: None,
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
            consecutive_failures: 0,
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
    fn replayed_lanes_do_not_consume_all_fresh_capacity() {
        let program = crate::evaluate::EvaluatedProgram {
            program: "fabro".to_string(),
            max_parallel: 5,
            runtime_max_parallel: None,
            lanes: vec![ready_lane(
                "ready:lane",
                "fresh-unit",
                "fresh-lane",
                LaneKind::Platform,
            )],
        };

        let reserved = reserve_fresh_ready_capacity(&program, &["failed:lane".to_string()], 3);

        assert_eq!(reserved, 1);
    }

    #[test]
    fn fresh_capacity_reservation_skips_when_no_fresh_work_exists() {
        let program = crate::evaluate::EvaluatedProgram {
            program: "fabro".to_string(),
            max_parallel: 5,
            runtime_max_parallel: None,
            lanes: Vec::new(),
        };

        let reserved = reserve_fresh_ready_capacity(&program, &["failed:lane".to_string()], 3);

        assert_eq!(reserved, 0);
    }

    #[test]
    fn codex_unblock_replays_reserve_two_direct_ready_slots_when_possible() {
        let mut unblock = failed_lane_with_kind(
            "wallet-rpc-tests-codex-unblock:wallet-rpc-tests-codex-unblock",
            "provider returned empty response with zero tokens",
            FailureKind::ProviderAccessLimited,
        );
        unblock.unit_id = "wallet-rpc-tests-codex-unblock".to_string();
        unblock.lane_id = "wallet-rpc-tests-codex-unblock".to_string();
        let program = crate::evaluate::EvaluatedProgram {
            program: "fabro".to_string(),
            max_parallel: 6,
            runtime_max_parallel: None,
            lanes: vec![
                ready_lane(
                    "ready:lane-1",
                    "fresh-unit",
                    "fresh-lane-1",
                    LaneKind::Platform,
                ),
                ready_lane(
                    "ready:lane-2",
                    "fresh-unit",
                    "fresh-lane-2",
                    LaneKind::Integration,
                ),
                unblock,
            ],
        };

        let reserved = reserve_fresh_ready_capacity(
            &program,
            &["wallet-rpc-tests-codex-unblock:wallet-rpc-tests-codex-unblock".to_string()],
            4,
        );

        assert_eq!(reserved, 2);
    }

    #[test]
    fn ready_lane_dispatch_diversifies_initial_foundation_wave() {
        let program = crate::evaluate::EvaluatedProgram {
            program: "fabro".to_string(),
            max_parallel: 10,
            runtime_max_parallel: None,
            lanes: vec![
                ready_lane(
                    "autodev-efficiency-and-dispatch:autodev-efficiency-and-dispatch",
                    "autodev-efficiency-and-dispatch",
                    "autodev-efficiency-and-dispatch",
                    LaneKind::Platform,
                ),
                ready_lane(
                    "greenfield-bootstrap-reliability:greenfield-bootstrap-reliability",
                    "greenfield-bootstrap-reliability",
                    "greenfield-bootstrap-reliability",
                    LaneKind::Platform,
                ),
                ready_lane(
                    "provider-policy-stabilization:provider-policy-stabilization",
                    "provider-policy-stabilization",
                    "provider-policy-stabilization",
                    LaneKind::Platform,
                ),
                ready_lane(
                    "test-coverage-critical-paths-autodev-integration-test:test-coverage-critical-paths-autodev-integration-test",
                    "test-coverage-critical-paths-autodev-integration-test",
                    "test-coverage-critical-paths-autodev-integration-test",
                    LaneKind::Integration,
                ),
                ready_lane(
                    "test-coverage-critical-paths-ci-preservation-and-hardening:test-coverage-critical-paths-ci-preservation-and-hardening",
                    "test-coverage-critical-paths-ci-preservation-and-hardening",
                    "test-coverage-critical-paths-ci-preservation-and-hardening",
                    LaneKind::Platform,
                ),
                ready_lane(
                    "test-coverage-critical-paths-synthesis-runtime-regression-tests:test-coverage-critical-paths-synthesis-runtime-regression-tests",
                    "test-coverage-critical-paths-synthesis-runtime-regression-tests",
                    "test-coverage-critical-paths-synthesis-runtime-regression-tests",
                    LaneKind::Platform,
                ),
            ],
        };

        let selected = select_ready_lanes_for_dispatch(&program, 10, &[]);

        assert_eq!(selected.len(), 4);
        assert!(selected.contains(
            &"autodev-efficiency-and-dispatch:autodev-efficiency-and-dispatch".to_string()
        ));
        assert!(selected.contains(
            &"greenfield-bootstrap-reliability:greenfield-bootstrap-reliability".to_string()
        ));
        assert!(selected
            .contains(&"provider-policy-stabilization:provider-policy-stabilization".to_string()));
        assert!(selected.contains(
            &"test-coverage-critical-paths-synthesis-runtime-regression-tests:test-coverage-critical-paths-synthesis-runtime-regression-tests".to_string()
        ));
    }

    #[test]
    fn replayed_lanes_cap_same_family_ready_expansion() {
        let replayed = vec![
            "test-coverage-critical-paths-autodev-integration-test:test-coverage-critical-paths-autodev-integration-test"
                .to_string(),
            "test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock:test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock"
                .to_string(),
        ];
        let program = crate::evaluate::EvaluatedProgram {
            program: "fabro".to_string(),
            max_parallel: 10,
            runtime_max_parallel: None,
            lanes: vec![
                ready_lane(
                    "autodev-efficiency-and-dispatch:autodev-efficiency-and-dispatch",
                    "autodev-efficiency-and-dispatch",
                    "autodev-efficiency-and-dispatch",
                    LaneKind::Platform,
                ),
                ready_lane(
                    "greenfield-bootstrap-reliability:greenfield-bootstrap-reliability",
                    "greenfield-bootstrap-reliability",
                    "greenfield-bootstrap-reliability",
                    LaneKind::Platform,
                ),
                ready_lane(
                    "provider-policy-stabilization:provider-policy-stabilization",
                    "provider-policy-stabilization",
                    "provider-policy-stabilization",
                    LaneKind::Platform,
                ),
                ready_lane(
                    "test-coverage-critical-paths-synthesis-runtime-regression-tests:test-coverage-critical-paths-synthesis-runtime-regression-tests",
                    "test-coverage-critical-paths-synthesis-runtime-regression-tests",
                    "test-coverage-critical-paths-synthesis-runtime-regression-tests",
                    LaneKind::Platform,
                ),
                ready_lane(
                    "test-coverage-critical-paths-fabro-db-baseline-tests:test-coverage-critical-paths-fabro-db-baseline-tests",
                    "test-coverage-critical-paths-fabro-db-baseline-tests",
                    "test-coverage-critical-paths-fabro-db-baseline-tests",
                    LaneKind::Platform,
                ),
                ready_lane(
                    "test-coverage-critical-paths-minimal-coverage-for-fabro-mcp-and-fabro-github:test-coverage-critical-paths-minimal-coverage-for-fabro-mcp-and-fabro-github",
                    "test-coverage-critical-paths-minimal-coverage-for-fabro-mcp-and-fabro-github",
                    "test-coverage-critical-paths-minimal-coverage-for-fabro-mcp-and-fabro-github",
                    LaneKind::Platform,
                ),
                ready_lane(
                    "test-coverage-critical-paths-autodev-integration-test:test-coverage-critical-paths-autodev-integration-test",
                    "test-coverage-critical-paths-autodev-integration-test",
                    "test-coverage-critical-paths-autodev-integration-test",
                    LaneKind::Integration,
                ),
                ready_lane(
                    "test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock:test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock",
                    "test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock",
                    "test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock",
                    LaneKind::Platform,
                ),
            ],
        };

        let selected = select_ready_lanes_for_dispatch(&program, 8, &replayed);

        assert_eq!(
            selected,
            vec![
                "autodev-efficiency-and-dispatch:autodev-efficiency-and-dispatch".to_string(),
                "greenfield-bootstrap-reliability:greenfield-bootstrap-reliability".to_string(),
                "provider-policy-stabilization:provider-policy-stabilization".to_string(),
            ]
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
            run_config: PathBuf::from("malinka/run-configs/integrate/demo.toml"),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: Some("01KMTEST".to_string()),
            current_run_dir: None,
            last_run_dir: None,
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
            consecutive_failures: 0,
        };

        let target = replay_target_lane(
            &manifest,
            &crate::evaluate::EvaluatedProgram {
                program: manifest.program.clone(),
                max_parallel: manifest.max_parallel,
                runtime_max_parallel: None,
                lanes: vec![lane.clone()],
            },
            &lane,
            Utc::now(),
            false,
        );

        assert_eq!(target.as_deref(), Some("demo:integrate"));
    }

    #[test]
    fn environment_collision_retry_waits_for_short_cooldown() {
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
            run_config: PathBuf::from("malinka/run-configs/bootstrap/demo.toml"),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: Some("01KMTEST".to_string()),
            current_run_dir: None,
            last_run_dir: None,
            last_started_at: Some(Utc::now() - chrono::Duration::seconds(20)),
            last_finished_at: Some(Utc::now() - chrono::Duration::seconds(10)),
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
            consecutive_failures: 0,
        };

        let target = replay_target_lane(
            &manifest,
            &crate::evaluate::EvaluatedProgram {
                program: manifest.program.clone(),
                max_parallel: manifest.max_parallel,
                runtime_max_parallel: None,
                lanes: vec![lane.clone()],
            },
            &lane,
            Utc::now(),
            false,
        );

        assert!(target.is_none());

        let ready_lane = EvaluatedLane {
            last_started_at: Some(Utc::now() - chrono::Duration::seconds(40)),
            last_finished_at: Some(Utc::now() - chrono::Duration::seconds(20)),
            ..lane
        };
        let target = replay_target_lane(
            &manifest,
            &crate::evaluate::EvaluatedProgram {
                program: manifest.program.clone(),
                max_parallel: manifest.max_parallel,
                runtime_max_parallel: None,
                lanes: vec![ready_lane.clone()],
            },
            &ready_lane,
            Utc::now(),
            false,
        );

        assert_eq!(target.as_deref(), Some("demo:service"));
    }

    #[test]
    fn ensure_target_repo_fresh_for_dispatch_fast_forwards_clean_default_branch() {
        let temp = tempfile::tempdir().expect("tempdir");
        let remote = temp.path().join("remote.git");
        let source = temp.path().join("source");
        let local = temp.path().join("local");
        std::fs::create_dir_all(&source).expect("source dir");

        git(
            temp.path(),
            &["init", "--bare", remote.to_str().expect("remote path")],
        );
        git(&source, &["init"]);
        git(&source, &["config", "user.name", "Fabro"]);
        git(&source, &["config", "user.email", "fabro@example.com"]);
        std::fs::write(source.join("README.md"), "hello\n").expect("write readme");
        git(&source, &["add", "README.md"]);
        git(&source, &["commit", "-m", "initial"]);
        git(&source, &["branch", "-M", "main"]);
        git(
            &source,
            &[
                "remote",
                "add",
                "origin",
                remote.to_str().expect("remote path"),
            ],
        );
        git(&source, &["push", "-u", "origin", "main"]);
        git(&remote, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        git(
            temp.path(),
            &[
                "clone",
                remote.to_str().expect("remote path"),
                local.to_str().expect("local path"),
            ],
        );

        std::fs::write(source.join("README.md"), "hello\nworld\n").expect("update readme");
        git(&source, &["commit", "-am", "advance remote"]);
        git(&source, &["push", "origin", "main"]);

        let manifest = demo_manifest(&local);
        let manifest_path = temp.path().join("demo.yaml");
        std::fs::write(&manifest_path, "program: demo\nunits: {}\n").expect("manifest");

        let freshness = ensure_target_repo_fresh_for_dispatch(&manifest, &manifest_path);
        assert_eq!(freshness, TargetRepoFreshness::FastForwarded);

        let counts = ahead_behind_counts(&local, "origin/main").expect("ahead behind");
        assert_eq!(counts, (0, 0));
    }

    #[test]
    fn ensure_target_repo_fresh_for_dispatch_blocks_dirty_repo_that_is_behind() {
        let temp = tempfile::tempdir().expect("tempdir");
        let remote = temp.path().join("remote.git");
        let source = temp.path().join("source");
        let local = temp.path().join("local");
        std::fs::create_dir_all(&source).expect("source dir");

        git(
            temp.path(),
            &["init", "--bare", remote.to_str().expect("remote path")],
        );
        git(&source, &["init"]);
        git(&source, &["config", "user.name", "Fabro"]);
        git(&source, &["config", "user.email", "fabro@example.com"]);
        std::fs::write(source.join("README.md"), "hello\n").expect("write readme");
        git(&source, &["add", "README.md"]);
        git(&source, &["commit", "-m", "initial"]);
        git(&source, &["branch", "-M", "main"]);
        git(
            &source,
            &[
                "remote",
                "add",
                "origin",
                remote.to_str().expect("remote path"),
            ],
        );
        git(&source, &["push", "-u", "origin", "main"]);
        git(&remote, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        git(
            temp.path(),
            &[
                "clone",
                remote.to_str().expect("remote path"),
                local.to_str().expect("local path"),
            ],
        );

        std::fs::write(source.join("README.md"), "hello\nworld\n").expect("update readme");
        git(&source, &["commit", "-am", "advance remote"]);
        git(&source, &["push", "origin", "main"]);
        std::fs::write(local.join("README.md"), "hello\nlocal dirty\n").expect("dirty local");

        let manifest = demo_manifest(&local);
        let manifest_path = temp.path().join("demo.yaml");
        std::fs::write(&manifest_path, "program: demo\nunits: {}\n").expect("manifest");

        let freshness = ensure_target_repo_fresh_for_dispatch(&manifest, &manifest_path);
        assert_eq!(
            freshness,
            TargetRepoFreshness::BehindWithLocalChanges { behind: 1 }
        );
    }

    #[test]
    fn ensure_target_repo_fresh_for_dispatch_fast_forwards_with_only_untracked_noise() {
        let temp = tempfile::tempdir().expect("tempdir");
        let remote = temp.path().join("remote.git");
        let source = temp.path().join("source");
        let local = temp.path().join("local");
        std::fs::create_dir_all(&source).expect("source dir");

        git(
            temp.path(),
            &["init", "--bare", remote.to_str().expect("remote path")],
        );
        git(&source, &["init"]);
        git(&source, &["config", "user.name", "Fabro"]);
        git(&source, &["config", "user.email", "fabro@example.com"]);
        std::fs::write(source.join("README.md"), "hello\n").expect("write readme");
        git(&source, &["add", "README.md"]);
        git(&source, &["commit", "-m", "initial"]);
        git(&source, &["branch", "-M", "main"]);
        git(
            &source,
            &[
                "remote",
                "add",
                "origin",
                remote.to_str().expect("remote path"),
            ],
        );
        git(&source, &["push", "-u", "origin", "main"]);
        git(&remote, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        git(
            temp.path(),
            &[
                "clone",
                remote.to_str().expect("remote path"),
                local.to_str().expect("local path"),
            ],
        );

        std::fs::write(source.join("README.md"), "hello\nworld\n").expect("update readme");
        git(&source, &["commit", "-am", "advance remote"]);
        git(&source, &["push", "origin", "main"]);
        std::fs::write(local.join("scratch.ipynb"), "noise\n").expect("noise file");

        let manifest = demo_manifest(&local);
        let manifest_path = temp.path().join("demo.yaml");
        std::fs::write(&manifest_path, "program: demo\nunits: {}\n").expect("manifest");

        let freshness = ensure_target_repo_fresh_for_dispatch(&manifest, &manifest_path);
        assert_eq!(freshness, TargetRepoFreshness::FastForwarded);

        let counts = ahead_behind_counts(&local, "origin/main").expect("ahead behind");
        assert_eq!(counts, (0, 0));
    }

    #[test]
    fn ensure_target_repo_fresh_for_dispatch_allows_autodev_controller_branch() {
        let temp = tempfile::tempdir().expect("tempdir");
        let remote = temp.path().join("remote.git");
        let source = temp.path().join("source");
        let local = temp.path().join("local");
        std::fs::create_dir_all(&source).expect("source dir");

        git(
            temp.path(),
            &["init", "--bare", remote.to_str().expect("remote path")],
        );
        git(&source, &["init"]);
        git(&source, &["config", "user.name", "Fabro"]);
        git(&source, &["config", "user.email", "fabro@example.com"]);
        std::fs::write(source.join("README.md"), "hello\n").expect("write readme");
        git(&source, &["add", "README.md"]);
        git(&source, &["commit", "-m", "initial"]);
        git(&source, &["branch", "-M", "main"]);
        git(
            &source,
            &[
                "remote",
                "add",
                "origin",
                remote.to_str().expect("remote path"),
            ],
        );
        git(&source, &["push", "-u", "origin", "main"]);
        git(&remote, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        git(
            temp.path(),
            &[
                "clone",
                remote.to_str().expect("remote path"),
                local.to_str().expect("local path"),
            ],
        );
        git(&local, &["checkout", "-b", "autodev/main"]);

        std::fs::write(source.join("README.md"), "hello\nworld\n").expect("update readme");
        git(&source, &["commit", "-am", "advance remote"]);
        git(&source, &["push", "origin", "main"]);

        let manifest = demo_manifest(&local);
        let manifest_path = temp.path().join("demo.yaml");
        std::fs::write(&manifest_path, "program: demo\nunits: {}\n").expect("manifest");

        let freshness = ensure_target_repo_fresh_for_dispatch(&manifest, &manifest_path);
        assert_eq!(freshness, TargetRepoFreshness::FastForwarded);
    }

    #[test]
    fn ensure_target_repo_fresh_for_dispatch_rejects_local_path_origin() {
        let temp = tempfile::tempdir().expect("tempdir");
        let remote = temp.path().join("remote.git");
        let source = temp.path().join("source");
        let local = temp.path().join("local");
        std::fs::create_dir_all(&source).expect("source dir");

        git(
            temp.path(),
            &["init", "--bare", remote.to_str().expect("remote path")],
        );
        git(&source, &["init"]);
        git(&source, &["config", "user.name", "Fabro"]);
        git(&source, &["config", "user.email", "fabro@example.com"]);
        std::fs::write(source.join("README.md"), "hello\n").expect("write readme");
        git(&source, &["add", "README.md"]);
        git(&source, &["commit", "-m", "initial"]);
        git(&source, &["branch", "-M", "main"]);
        git(
            &source,
            &[
                "remote",
                "add",
                "origin",
                remote.to_str().expect("remote path"),
            ],
        );
        git(&source, &["push", "-u", "origin", "main"]);
        git(&remote, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        git(
            temp.path(),
            &[
                "clone",
                remote.to_str().expect("remote path"),
                local.to_str().expect("local path"),
            ],
        );

        git(
            &local,
            &[
                "remote",
                "set-url",
                "origin",
                temp.path()
                    .join("controller")
                    .to_str()
                    .expect("controller path"),
            ],
        );

        let manifest = demo_manifest(&local);
        let manifest_path = temp.path().join("demo.yaml");
        std::fs::write(&manifest_path, "program: demo\nunits: {}\n").expect("manifest");

        let freshness = ensure_target_repo_fresh_for_dispatch(&manifest, &manifest_path);
        assert!(
            matches!(freshness, TargetRepoFreshness::InvalidPushRemote { .. }),
            "expected invalid push remote, got {:?}",
            freshness
        );
    }

    #[test]
    fn generated_package_dirty_path_classifier_accepts_generated_roots_only() {
        for path in [
            ".raspberry/rxmragent-state.json",
            "malinka/programs/rxmragent.yaml",
            "malinka/workflows/holistic-review-minimax/demo.fabro",
            "malinka/run-configs/holistic-preflight/demo.toml",
            "malinka/prompts/holistic-review-deep/demo/plan.md",
            "outputs/demo/review.md",
        ] {
            assert!(is_generated_package_dirty_path(path), "{path}");
        }
        for path in [
            "malinka/blueprints/rxmragent.yaml",
            "crates/casino-core/src/lib.rs",
            "README.md",
        ] {
            assert!(!is_generated_package_dirty_path(path), "{path}");
        }
    }

    #[test]
    fn dirty_worktree_is_generated_package_only_rejects_user_code_changes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path();
        git(repo, &["init"]);
        git(repo, &["config", "user.name", "Fabro"]);
        git(repo, &["config", "user.email", "fabro@example.com"]);
        std::fs::create_dir_all(repo.join("malinka/prompts")).expect("prompts dir");
        std::fs::create_dir_all(repo.join("crates/app/src")).expect("src dir");
        std::fs::write(repo.join("malinka/prompts/demo.md"), "generated\n").expect("write");
        std::fs::write(repo.join("crates/app/src/lib.rs"), "user change\n").expect("write");

        assert!(!dirty_worktree_is_generated_package_only(repo));
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
        let manifest_path = temp.path().join("malinka/programs/demo.yaml");
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
            running: 0,
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
                running: 1,
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
            1,
            false,
            false,
            false,
            Some(&FrontierSignature {
                ready: 0,
                running: 1,
                replayable_failed: 1,
                regenerable_failed: 0,
                complete: 2,
                failed_recovery_keys: vec![
                    "failed:lane:environment_collision:backoff_retry".to_string()
                ],
            }),
        ));

        let active_frontier = FrontierSignature {
            running: 2,
            ..frontier.clone()
        };
        assert!(!should_trigger_evolve(
            Some(Instant::now()),
            Duration::ZERO,
            &active_frontier,
            5,
            5,
            false,
            false,
            false,
            Some(&frontier),
        ));
    }

    #[test]
    fn regenerate_lane_evolve_fires_even_at_full_capacity() {
        let frontier = FrontierSignature {
            ready: 14,
            running: 5,
            replayable_failed: 0,
            regenerable_failed: 1,
            complete: 5,
            failed_recovery_keys: vec![
                "failed:lane:supervisor_only_lane:regenerate_lane".to_string()
            ],
        };

        // Evolve re-synthesises configs without consuming a slot, so it must
        // fire even when all parallel slots are occupied.
        assert!(should_trigger_evolve(
            None,
            Duration::from_secs(3600),
            &frontier,
            5,
            1,
            false,
            false,
            true,
            Some(&frontier),
        ));
    }

    #[test]
    fn verify_gate_without_retry_target_is_regenerable() {
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            runtime_max_parallel: None,
            lanes: vec![crate::evaluate::EvaluatedLane {
                lane_key: "demo:lane".to_string(),
                unit_id: "demo".to_string(),
                unit_title: "Demo".to_string(),
                lane_id: "lane".to_string(),
                lane_title: "Lane".to_string(),
                lane_kind: crate::manifest::LaneKind::Service,
                status: LaneExecutionStatus::Failed,
                operational_state: None,
                precondition_state: None,
                proof_state: None,
                orchestration_state: None,
                detail: "failed".to_string(),
                managed_milestone: "reviewed".to_string(),
                proof_profile: None,
                run_config: PathBuf::from("run.toml"),
                run_id: None,
                current_run_id: None,
                current_fabro_run_id: None,
                current_stage: None,
                last_run_id: None,
                current_run_dir: None,
                last_run_dir: None,
                last_started_at: None,
                last_finished_at: Some(Utc::now()),
                last_exit_status: Some(1),
                last_error: Some(
                    "Engine error: goal gate unsatisfied for node verify and no retry target"
                        .to_string(),
                ),
                failure_kind: Some(FailureKind::ProofScriptFailure),
                recovery_action: Some(FailureRecoveryAction::ReplayLane),
                last_completed_stage_label: Some("Start".to_string()),
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
                consecutive_failures: 0,
            }],
        };

        let regenerable = regenerable_failed_lanes(&program);
        assert_eq!(regenerable, vec!["demo:lane".to_string()]);
    }

    #[test]
    fn cycle_limit_treats_zero_as_unbounded() {
        assert_eq!(cycle_limit(0), None);
        assert_eq!(cycle_limit(3), Some(3));
    }

    #[test]
    fn has_more_cycles_respects_bounded_and_unbounded_limits() {
        assert!(has_more_cycles(None, 1));
        assert!(has_more_cycles(Some(3), 2));
        assert!(!has_more_cycles(Some(3), 3));
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
    fn detect_regenerate_noop_lanes_flags_unchanged_render() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workflow = temp.path().join("workflow.fabro");
        let run_config = temp.path().join("run.toml");
        std::fs::write(&workflow, "digraph demo { start -> exit }\n").expect("workflow");
        std::fs::write(
            &run_config,
            "version = 1\ngraph = \"workflow.fabro\"\ngoal = \"demo\"\ndirectory = \".\"\n",
        )
        .expect("run config");

        let before = vec![(
            "demo:lane".to_string(),
            lane_render_fingerprint(&run_config),
        )];
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            runtime_max_parallel: None,
            lanes: vec![crate::evaluate::EvaluatedLane {
                lane_key: "demo:lane".to_string(),
                unit_id: "demo".to_string(),
                unit_title: "Demo".to_string(),
                lane_id: "lane".to_string(),
                lane_title: "Lane".to_string(),
                lane_kind: crate::manifest::LaneKind::Artifact,
                status: LaneExecutionStatus::Failed,
                operational_state: None,
                precondition_state: None,
                proof_state: None,
                orchestration_state: None,
                detail: "failed".to_string(),
                managed_milestone: "reviewed".to_string(),
                proof_profile: None,
                run_config: run_config.clone(),
                run_id: None,
                current_run_id: None,
                current_fabro_run_id: None,
                current_stage: None,
                last_run_id: None,
                current_run_dir: None,
                last_run_dir: None,
                last_started_at: None,
                last_finished_at: None,
                last_exit_status: Some(1),
                last_error: Some("failed".to_string()),
                failure_kind: Some(FailureKind::SupervisorOnlyLane),
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
                consecutive_failures: 0,
            }],
        };

        let noop = detect_regenerate_noop_lanes(&["demo:lane".to_string()], &before, &program);

        assert_eq!(noop, vec!["demo:lane".to_string()]);
    }

    #[test]
    fn current_snapshot_counts_nested_child_running_work() {
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            runtime_max_parallel: None,
            lanes: vec![crate::evaluate::EvaluatedLane {
                lane_key: "demo:program".to_string(),
                unit_id: "demo".to_string(),
                unit_title: "Demo".to_string(),
                lane_id: "program".to_string(),
                lane_title: "Program".to_string(),
                lane_kind: crate::manifest::LaneKind::Orchestration,
                status: LaneExecutionStatus::Blocked,
                operational_state: None,
                precondition_state: None,
                proof_state: None,
                orchestration_state: None,
                detail: "child program `child`: complete=0 ready=0 running=1 blocked=0 failed=0 | running_lanes=child:lane@Review".to_string(),
                managed_milestone: "coordinated".to_string(),
                proof_profile: None,
                run_config: PathBuf::from("child.toml"),
                run_id: None,
                current_run_id: None,
                current_fabro_run_id: None,
                current_stage: None,
                last_run_id: None,
                current_run_dir: None,
                last_run_dir: None,
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
                consecutive_failures: 0,
            }],
        };

        let snapshot = current_snapshot(&program, Some(5));

        assert_eq!(snapshot.running, 1);
        assert_eq!(
            snapshot.running_lanes,
            vec!["child:lane@Review".to_string()]
        );
        assert_eq!(snapshot.blocked, 1);
    }

    #[test]
    fn current_snapshot_hides_failed_source_lane_while_unblock_runs() {
        let source = EvaluatedLane {
            lane_key:
                "test-coverage-critical-paths-ci-preservation-and-hardening:test-coverage-critical-paths-ci-preservation-and-hardening"
                    .to_string(),
            unit_id: "test-coverage-critical-paths-ci-preservation-and-hardening".to_string(),
            unit_title: "CI Preservation".to_string(),
            lane_id: "test-coverage-critical-paths-ci-preservation-and-hardening".to_string(),
            lane_title: "CI Preservation".to_string(),
            lane_kind: LaneKind::Platform,
            status: LaneExecutionStatus::Failed,
            operational_state: None,
            precondition_state: None,
            proof_state: None,
            orchestration_state: None,
            detail: "failed".to_string(),
            managed_milestone: "integrated".to_string(),
            proof_profile: None,
            run_config: PathBuf::from("ci.toml"),
            run_id: None,
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage: None,
            last_run_id: None,
            current_run_dir: None,
            last_run_dir: None,
            last_started_at: None,
            last_finished_at: None,
            last_exit_status: Some(1),
            last_error: Some("failed".to_string()),
            failure_kind: Some(FailureKind::RegenerateNoop),
            recovery_action: Some(FailureRecoveryAction::SurfaceBlocked),
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
            consecutive_failures: 0,
        };
        let unblock = EvaluatedLane {
            lane_key:
                "test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock:test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock"
                    .to_string(),
            unit_id:
                "test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock"
                    .to_string(),
            unit_title: "CI Preservation Codex Unblock".to_string(),
            lane_id:
                "test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock"
                    .to_string(),
            lane_title: "CI Preservation Codex Unblock".to_string(),
            lane_kind: LaneKind::Platform,
            status: LaneExecutionStatus::Running,
            operational_state: None,
            precondition_state: None,
            proof_state: None,
            orchestration_state: None,
            detail: String::new(),
            managed_milestone: "done".to_string(),
            proof_profile: Some("unblock".to_string()),
            run_config: PathBuf::from("ci-unblock.toml"),
            run_id: None,
            current_run_id: Some("run-123".to_string()),
            current_fabro_run_id: Some("run-123".to_string()),
            current_stage: None,
            last_run_id: Some("run-123".to_string()),
            current_run_dir: None,
            last_run_dir: None,
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
            consecutive_failures: 0,
        };
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 10,
            runtime_max_parallel: None,
            lanes: vec![source, unblock],
        };

        let snapshot = current_snapshot(&program, Some(10));

        assert_eq!(snapshot.running, 1);
        assert_eq!(snapshot.failed, 0);
        assert!(snapshot.failed_lanes.is_empty());
        assert_eq!(snapshot.blocked, 1);
    }

    #[test]
    fn orchestrate_program_reports_recursive_child_program_cycles() {
        let temp = tempfile::tempdir().expect("tempdir");
        let programs_dir = temp.path().join("malinka/programs");
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
        run_config: ../../malinka/programs/child.yaml
        managed_milestone: coordinated
        program_manifest: ../../malinka/programs/child.yaml
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
        run_config: ../../malinka/programs/parent.yaml
        managed_milestone: coordinated
        program_manifest: ../../malinka/programs/parent.yaml
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

    #[test]
    fn orchestrate_program_returns_maintenance_stop_reason_when_locked() {
        let temp = tempfile::tempdir().expect("tempdir");
        let programs_dir = temp.path().join("malinka/programs");
        let raspberry_dir = temp.path().join(".raspberry");
        std::fs::create_dir_all(&programs_dir).expect("program dir");
        std::fs::create_dir_all(&raspberry_dir).expect("raspberry dir");
        let manifest_path = programs_dir.join("demo.yaml");
        std::fs::write(
            &manifest_path,
            r#"
version: 1
program: demo
target_repo: ../..
state_path: ../../.raspberry/demo-state.json
max_parallel: 1
units:
  - id: docs
    title: Docs
    output_root: ../../outputs/docs
    artifacts:
      - id: plan
        path: plan.md
    milestones:
      - id: reviewed
        requires: [plan]
    lanes:
      - id: lane
        title: Docs Lane
        kind: artifact
        run_config: ../run-configs/bootstrap/docs.toml
        managed_milestone: reviewed
        produces: [plan]
"#,
        )
        .expect("manifest");
        std::fs::write(
            raspberry_dir.join("maintenance.json"),
            r#"{"enabled":true,"reason":"core redesign in progress","set_by":"codex"}"#,
        )
        .expect("maintenance");

        let report = orchestrate_program(
            &manifest_path,
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
        .expect("maintenance should return report");

        assert_eq!(report.stop_reason, AutodevStopReason::Maintenance);
        assert!(report.cycles.is_empty());
    }

    #[test]
    fn orchestrate_program_dispatches_from_fresh_autodev_controller_fixture() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path();
        std::fs::create_dir_all(repo.join("malinka/blueprints")).expect("blueprints dir");
        std::fs::create_dir_all(repo.join("malinka/programs")).expect("programs dir");
        std::fs::create_dir_all(repo.join("malinka/run-configs/bootstrap"))
            .expect("run-configs dir");
        std::fs::create_dir_all(repo.join("malinka/workflows/bootstrap")).expect("workflows dir");
        std::fs::create_dir_all(repo.join(".raspberry")).expect("state dir");
        std::fs::write(repo.join("README.md"), "demo\n").expect("readme");
        std::fs::write(repo.join("malinka/blueprints/demo.yaml"), "program: demo\n")
            .expect("blueprint");
        std::fs::write(
            repo.join("malinka/programs/demo.yaml"),
            r#"
version: 1
program: demo
target_repo: ../..
state_path: ../../.raspberry/demo-state.json
max_parallel: 1
units:
  - id: docs
    title: Docs
    output_root: ../../outputs/docs
    artifacts:
      - id: plan
        path: plan.md
    milestones:
      - id: reviewed
        requires: [plan]
    lanes:
      - id: lane
        title: Docs Lane
        kind: artifact
        run_config: ../run-configs/bootstrap/docs.toml
        managed_milestone: reviewed
        produces: [plan]
"#,
        )
        .expect("manifest");
        std::fs::write(
            repo.join("malinka/run-configs/bootstrap/docs.toml"),
            "version = 1\ngraph = \"../../workflows/bootstrap/docs.fabro\"\ngoal = \"Demo\"\n",
        )
        .expect("run config");
        std::fs::write(
            repo.join("malinka/workflows/bootstrap/docs.fabro"),
            "digraph docs {\n  start [shape=Mdiamond, label=\"Start\"]\n  exit [shape=Msquare, label=\"Exit\"]\n  work [label=\"Work\", prompt=\"Do the work\"]\n  start -> work -> exit\n}\n",
        )
        .expect("workflow");
        git(repo, &["init"]);
        git(repo, &["config", "user.name", "Fabro"]);
        git(repo, &["config", "user.email", "fabro@example.com"]);
        git(repo, &["add", "."]);
        git(repo, &["commit", "-m", "initial"]);
        git(repo, &["checkout", "-b", "autodev/main"]);

        let fabro_bin = repo.join("fake-fabro.sh");
        std::fs::write(
            &fabro_bin,
            format!(
                "#!/bin/sh\n\
set -eu\n\
runs_base={runs_base}\n\
case \" $* \" in\n\
  *\" synth create \"*)\n\
  exit 0\n\
  ;;\n\
  *\" run \"*)\n\
  run_id=01K00000000000000000000000\n\
  run_dir=\"$runs_base/20260327-$run_id\"\n\
  mkdir -p \"$run_dir\"\n\
  cat > \"$run_dir/manifest.json\" <<'EOF'\n\
{{\"run_id\":\"01K00000000000000000000000\",\"workflow_name\":\"docs\",\"goal\":\"Demo\",\"start_time\":\"2026-03-27T00:00:00Z\",\"node_count\":1,\"edge_count\":0}}\n\
EOF\n\
  cat > \"$run_dir/state.json\" <<'EOF'\n\
{{\"run_id\":\"01K00000000000000000000000\",\"updated_at\":\"2026-03-27T00:00:01Z\",\"status\":\"running\",\"current_stage_label\":\"Implement\",\"completed_stage_count\":0}}\n\
EOF\n\
  echo \"$run_id\"\n\
  exit 0\n\
  ;;\n\
esac\n\
exit 1\n"
                ,
                runs_base = fabro_workflows::run_lookup::default_runs_base().display()
            ),
        )
        .expect("fake fabro");
        let mut perms = std::fs::metadata(&fabro_bin)
            .expect("metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fabro_bin, perms).expect("chmod");

        let manifest_path = repo.join("malinka/programs/demo.yaml");

        let report = orchestrate_program(
            &manifest_path,
            &AutodevSettings {
                fabro_bin: fabro_bin.clone(),
                max_parallel_override: Some(1),
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
        .expect("autodev should dispatch");

        assert!(!matches!(
            report.stop_reason,
            AutodevStopReason::Maintenance
        ));
        assert!(
            report
                .cycles
                .first()
                .is_some_and(|cycle| !cycle.dispatched.is_empty()),
            "expected at least one dispatch outcome"
        );
    }

    #[test]
    fn sync_autodev_report_with_program_marks_live_controller_in_progress() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_path = temp.path().join("malinka/programs/demo.yaml");
        std::fs::create_dir_all(manifest_path.parent().expect("parent")).expect("program dir");
        let manifest = demo_manifest(temp.path());
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            runtime_max_parallel: None,
            lanes: vec![ready_lane("demo:lane", "demo", "lane", LaneKind::Artifact)],
        };
        let previous_snapshot = current_snapshot(&program, Some(1));
        let previous_updated_at = Utc::now() - chrono::Duration::minutes(5);
        let report = AutodevReport {
            program: "demo".to_string(),
            stop_reason: AutodevStopReason::CycleLimit,
            updated_at: previous_updated_at,
            provenance: None,
            current: Some(previous_snapshot),
            cycles: Vec::new(),
        };
        save_autodev_report(&manifest_path, &manifest, &report).expect("report saved");
        let _lease = crate::controller_lease::acquire_autodev_lease(&manifest_path, &manifest)
            .expect("lease acquired");

        sync_autodev_report_with_program(&manifest_path, &manifest, &program)
            .expect("sync succeeds");

        let synced = load_optional_autodev_report(&manifest_path, &manifest)
            .expect("report loads")
            .expect("report present");
        assert_eq!(synced.stop_reason, AutodevStopReason::InProgress);
        assert!(synced.updated_at > previous_updated_at);
        assert_eq!(
            synced.current.as_ref().map(|snapshot| snapshot.running),
            Some(0)
        );
        assert_eq!(
            synced.current.as_ref().map(|snapshot| snapshot.ready),
            Some(1)
        );
    }

    #[test]
    fn cycle_telemetry_surfaces_zero_throughput_and_provider_debt() {
        let mut reclaimed = failed_lane(
            "demo:stale",
            "tracked run was removed after controller stopped",
        );
        reclaimed.detail = "stale running record reconciled".to_string();
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 10,
            runtime_max_parallel: None,
            lanes: vec![
                ready_lane("demo:ready-1", "demo", "ready-1", LaneKind::Artifact),
                ready_lane("demo:ready-2", "demo", "ready-2", LaneKind::Artifact),
                failed_lane_with_kind(
                    "demo:bootstrap",
                    "failed to spawn fabro synth import: No such file or directory",
                    FailureKind::TransientLaunchFailure,
                ),
                failed_lane_with_kind(
                    "demo:provider",
                    "provider returned empty response with zero tokens",
                    FailureKind::ProviderAccessLimited,
                ),
                failed_lane_with_kind(
                    "demo:watchdog",
                    "CLI command timed out after 1800s",
                    FailureKind::StallWatchdog,
                ),
                reclaimed,
            ],
        };

        let telemetry = build_cycle_telemetry(
            &program,
            10,
            1,
            9,
            &["demo:ready-1".to_string(), "demo:ready-2".to_string()],
            &[],
            3,
            Some(120),
            true,
        );

        assert_eq!(telemetry.available_slots, 10);
        assert_eq!(telemetry.fresh_capacity_reserve, 1);
        assert_eq!(telemetry.replay_budget, 9);
        assert_eq!(telemetry.oldest_running_seconds, Some(120));
        assert_eq!(telemetry.running_without_completion_cycles, 3);
        assert!(telemetry.ready_while_saturated);
        assert!(telemetry.idle_with_ready_work);
        assert_eq!(telemetry.ready_but_undispatched.len(), 2);
        assert_eq!(telemetry.failed_bootstrap_count, 1);
        assert_eq!(telemetry.runtime_path_errors, 1);
        assert_eq!(telemetry.stale_running_reclaimed, 1);
        assert_eq!(telemetry.provider_access_limited, 1);
        assert_eq!(telemetry.stall_watchdog, 1);
    }

    #[test]
    fn cycle_telemetry_does_not_mark_full_capacity_as_idle() {
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 10,
            runtime_max_parallel: None,
            lanes: vec![ready_lane(
                "demo:ready-1",
                "demo",
                "ready-1",
                LaneKind::Artifact,
            )],
        };

        let telemetry = build_cycle_telemetry(
            &program,
            0,
            0,
            0,
            &["demo:ready-1".to_string()],
            &[],
            0,
            None,
            false,
        );

        assert!(!telemetry.idle_with_ready_work);
        assert_eq!(
            telemetry.ready_but_undispatched,
            vec!["demo:ready-1".to_string()]
        );
    }

    #[test]
    fn cycle_telemetry_treats_orchestration_ready_as_child_program_capacity() {
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 10,
            runtime_max_parallel: None,
            lanes: vec![EvaluatedLane {
                lane_key: "demo:program".to_string(),
                unit_id: "demo".to_string(),
                unit_title: "Demo".to_string(),
                lane_id: "program".to_string(),
                lane_title: "Demo Program".to_string(),
                lane_kind: LaneKind::Orchestration,
                status: LaneExecutionStatus::Ready,
                operational_state: None,
                precondition_state: None,
                proof_state: None,
                orchestration_state: None,
                detail: String::new(),
                managed_milestone: "coordinated".to_string(),
                proof_profile: None,
                run_config: PathBuf::from("malinka/programs/demo.yaml"),
                run_id: None,
                current_run_id: None,
                current_fabro_run_id: None,
                current_stage: None,
                last_run_id: None,
                current_run_dir: None,
                last_run_dir: None,
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
                consecutive_failures: 0,
            }],
        };

        let telemetry = build_cycle_telemetry(
            &program,
            1,
            0,
            1,
            &["demo:program".to_string()],
            &[],
            0,
            None,
            false,
        );

        assert!(!telemetry.idle_with_ready_work);
        assert!(telemetry.direct_ready_but_undispatched.is_empty());
        assert_eq!(
            telemetry.child_program_ready,
            vec!["demo:program".to_string()]
        );
    }

    #[test]
    fn running_without_completion_cycles_tracks_saturated_stalls() {
        assert_eq!(
            next_running_without_completion_cycles(0, Some(118), 10, 118),
            1
        );
        assert_eq!(
            next_running_without_completion_cycles(1, Some(118), 10, 118),
            2
        );
        assert_eq!(
            next_running_without_completion_cycles(2, Some(118), 10, 119),
            0
        );
        assert_eq!(
            next_running_without_completion_cycles(2, Some(118), 0, 118),
            0
        );
    }

    #[test]
    fn oldest_running_seconds_uses_oldest_live_lane() {
        let mut older = ready_lane("demo:older", "demo", "older", LaneKind::Artifact);
        older.status = LaneExecutionStatus::Running;
        older.last_started_at = Some(Utc::now() - chrono::Duration::seconds(45));
        let mut newer = ready_lane("demo:newer", "demo", "newer", LaneKind::Artifact);
        newer.status = LaneExecutionStatus::Running;
        newer.last_started_at = Some(Utc::now() - chrono::Duration::seconds(15));
        let program = crate::evaluate::EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 10,
            runtime_max_parallel: None,
            lanes: vec![older, newer],
        };

        let oldest = oldest_running_seconds(&program, Utc::now()).expect("oldest running age");
        assert!(oldest >= 45, "{oldest}");
    }

    #[test]
    fn load_optional_autodev_report_surfaces_parse_errors() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_path = temp.path().join("malinka/programs/demo.yaml");
        std::fs::create_dir_all(manifest_path.parent().expect("parent")).expect("program dir");
        let manifest = demo_manifest(temp.path());
        let report_path = autodev_report_path(&manifest_path, &manifest);
        std::fs::create_dir_all(report_path.parent().expect("parent")).expect("report dir");
        std::fs::write(&report_path, "{not-json").expect("invalid report written");

        let error = load_optional_autodev_report(&manifest_path, &manifest)
            .expect_err("invalid report should fail");

        assert!(matches!(error, AutodevError::ParseReport { .. }));
    }

    #[test]
    fn paperclip_bundle_root_detects_existing_bundle() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_path = temp.path().join("malinka/programs/demo.yaml");
        std::fs::create_dir_all(manifest_path.parent().expect("parent")).expect("program dir");
        std::fs::create_dir_all(temp.path().join("malinka/paperclip/raspberry-demo"))
            .expect("paperclip dir");
        let manifest = ProgramManifest {
            program: "raspberry-demo".to_string(),
            target_repo: PathBuf::from("../.."),
            state_path: PathBuf::from("../../.raspberry/demo-state.json"),
            max_parallel: 1,
            run_dir: None,
            units: std::collections::BTreeMap::new(),
        };

        let root = paperclip_bundle_root(&manifest_path, &manifest).expect("bundle root");
        assert_eq!(root, temp.path().join("malinka/paperclip/raspberry-demo"));
    }

    #[test]
    fn cargo_output_has_warning_detects_short_message_warnings() {
        assert!(cargo_output_has_warning(
            "src/lib.rs:10:5: warning: unused import"
        ));
        assert!(cargo_output_has_warning("warning: dead code"));
        assert!(!cargo_output_has_warning(
            "Finished `dev` profile target(s) in 0.25s"
        ));
    }

    #[test]
    fn summarize_admission_output_prefers_actionable_context_lines() {
        let summary = summarize_admission_output(
            "workspace emits warnings",
            "src/lib.rs:10:5: warning: unused import\n\nFinished target(s) in 1s",
            "",
        );
        assert!(summary.contains("workspace emits warnings"));
        assert!(summary.contains("warning: unused import"));
    }
}
