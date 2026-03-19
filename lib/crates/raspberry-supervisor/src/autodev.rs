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
use crate::manifest::{ManifestError, ProgramManifest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutodevSettings {
    pub fabro_bin: PathBuf,
    pub max_parallel_override: Option<usize>,
    pub max_cycles: usize,
    pub poll_interval_ms: u64,
    pub evolve_every_seconds: u64,
    pub doctrine_files: Vec<PathBuf>,
    pub evidence_paths: Vec<PathBuf>,
    pub preview_evolve_root: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutodevReport {
    pub program: String,
    pub stop_reason: AutodevStopReason,
    pub updated_at: DateTime<Utc>,
    pub cycles: Vec<AutodevCycleReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutodevCycleReport {
    pub cycle: usize,
    pub evolved: bool,
    pub evolve_target: Option<String>,
    pub ready_lanes: Vec<String>,
    pub dispatched: Vec<DispatchOutcome>,
    pub running_after: usize,
    pub complete_after: usize,
}

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
}

pub fn orchestrate_program(
    manifest_path: &Path,
    settings: &AutodevSettings,
) -> Result<AutodevReport, AutodevError> {
    let initial_manifest = ProgramManifest::load(manifest_path)?;
    let max_cycles = settings.max_cycles.max(1);
    let poll_interval = Duration::from_millis(settings.poll_interval_ms.max(1));
    let evolve_every = Duration::from_secs(settings.evolve_every_seconds);
    let mut last_evolve_at = None::<Instant>;
    let mut report = AutodevReport {
        program: initial_manifest.program.clone(),
        stop_reason: AutodevStopReason::CycleLimit,
        updated_at: Utc::now(),
        cycles: Vec::new(),
    };

    for cycle_index in 0..max_cycles {
        let manifest = ProgramManifest::load(manifest_path)?;
        let cycle_number = cycle_index + 1;
        let _program_before = evaluate_program(manifest_path)?;

        let mut evolved = false;
        let mut evolve_target = None;
        if should_evolve(last_evolve_at, evolve_every) {
            run_synth_evolve(manifest_path, &manifest, settings)?;
            last_evolve_at = Some(Instant::now());
            evolved = true;
            evolve_target = Some(
                settings
                    .preview_evolve_root
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| {
                        manifest
                            .resolved_target_repo(manifest_path)
                            .display()
                            .to_string()
                    }),
            );
        }

        let program = evaluate_program(manifest_path)?;
        let ready_lanes = program
            .lanes
            .iter()
            .filter(|lane| lane.status == LaneExecutionStatus::Ready)
            .map(|lane| lane.lane_key.clone())
            .collect::<Vec<_>>();

        let dispatched = if ready_lanes.is_empty() {
            Vec::new()
        } else {
            execute_selected_lanes(
                manifest_path,
                &ready_lanes,
                &DispatchSettings {
                    fabro_bin: settings.fabro_bin.clone(),
                    max_parallel_override: settings.max_parallel_override,
                    doctrine_files: settings.doctrine_files.clone(),
                    evidence_paths: settings.evidence_paths.clone(),
                    preview_evolve_root: settings.preview_evolve_root.clone(),
                },
            )?
        };

        let program_after = evaluate_program(manifest_path)?;
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
            dispatched,
            running_after,
            complete_after,
        });
        report.updated_at = Utc::now();
        save_autodev_report(manifest_path, &manifest, &report)?;

        let has_ready = program_after
            .lanes
            .iter()
            .any(|lane| lane.status == LaneExecutionStatus::Ready);
        let has_running = program_after
            .lanes
            .iter()
            .any(|lane| lane.status == LaneExecutionStatus::Running);
        if !has_ready && !has_running {
            if advance_child_programs(manifest_path, &manifest, settings)? {
                if cycle_number < max_cycles {
                    thread::sleep(poll_interval);
                    continue;
                }
            }
            report.stop_reason = AutodevStopReason::Settled;
            report.updated_at = Utc::now();
            save_autodev_report(manifest_path, &manifest, &report)?;
            return Ok(report);
        }

        if cycle_number < max_cycles {
            thread::sleep(poll_interval);
        }
    }

    report.stop_reason = AutodevStopReason::CycleLimit;
    report.updated_at = Utc::now();
    let final_manifest = ProgramManifest::load(manifest_path)?;
    save_autodev_report(manifest_path, &final_manifest, &report)?;
    Ok(report)
}

fn advance_child_programs(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    settings: &AutodevSettings,
) -> Result<bool, AutodevError> {
    let mut advanced = false;
    for (unit_id, unit) in &manifest.units {
        for lane_id in unit.lanes.keys() {
            let Some(child_manifest) =
                manifest.resolve_lane_program_manifest(manifest_path, unit_id, lane_id)
            else {
                continue;
            };
            let child_program = evaluate_program(&child_manifest)?;
            let child_has_ready = child_program
                .lanes
                .iter()
                .any(|lane| lane.status == LaneExecutionStatus::Ready);
            let child_has_running = child_program
                .lanes
                .iter()
                .any(|lane| lane.status == LaneExecutionStatus::Running);
            if child_has_ready || child_has_running {
                continue;
            }

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
                    max_cycles: 1,
                    poll_interval_ms: 1,
                    evolve_every_seconds: 0,
                    doctrine_files: settings.doctrine_files.clone(),
                    evidence_paths: settings.evidence_paths.clone(),
                    preview_evolve_root,
                },
            )?;
            advanced = true;
        }
    }
    Ok(advanced)
}

fn should_evolve(last_evolve_at: Option<Instant>, evolve_every: Duration) -> bool {
    let Some(last_evolve_at) = last_evolve_at else {
        return true;
    };
    evolve_every.is_zero() || last_evolve_at.elapsed() >= evolve_every
}

pub fn autodev_report_path(manifest_path: &Path, manifest: &ProgramManifest) -> PathBuf {
    manifest
        .resolved_target_repo(manifest_path)
        .join(".raspberry")
        .join(format!("{}-autodev.json", manifest.program))
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
    let report = serde_json::from_str(&raw).map_err(|source| AutodevError::ParseReport {
        path: path.clone(),
        source,
    })?;
    Ok(Some(report))
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

fn run_synth_evolve(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    settings: &AutodevSettings,
) -> Result<(), AutodevError> {
    let target_repo = manifest.resolved_target_repo(manifest_path);
    let temp_dir = autodev_temp_dir(&manifest.program)?;
    let blueprint_path = temp_dir.join(format!("{}-autodev.yaml", manifest.program));

    let import = Command::new(&settings.fabro_bin)
        .current_dir(&target_repo)
        .arg("--no-upgrade-check")
        .arg("synth")
        .arg("import")
        .arg("--target-repo")
        .arg(&target_repo)
        .arg("--program")
        .arg(&manifest.program)
        .arg("--output")
        .arg(&blueprint_path)
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

    let mut evolve = Command::new(&settings.fabro_bin);
    evolve
        .current_dir(&target_repo)
        .arg("--no-upgrade-check")
        .arg("synth")
        .arg("evolve")
        .arg("--blueprint")
        .arg(&blueprint_path)
        .arg("--target-repo")
        .arg(&target_repo);
    if let Some(preview_root) = &settings.preview_evolve_root {
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
