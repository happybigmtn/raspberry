use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::thread;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::autodev::{autodev_cargo_target_dir, orchestrate_program, AutodevSettings};
use crate::evaluate::{evaluate_program, refresh_parent_programs, LaneExecutionStatus};
use crate::integration::{integrate_lane, IntegrationRequest};
use crate::manifest::{LaneKind, ProgramManifest};
use crate::program_state::{
    mark_lane_dispatch_failed, mark_lane_finished, mark_lane_started, mark_lane_submitted,
    ProgramRuntimeState, ProgramStateError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchOutcome {
    pub lane_key: String,
    pub exit_status: i32,
    pub fabro_run_id: Option<String>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchSettings {
    pub fabro_bin: PathBuf,
    pub max_parallel_override: Option<usize>,
    pub doctrine_files: Vec<PathBuf>,
    pub evidence_paths: Vec<PathBuf>,
    pub preview_evolve_root: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum DispatchError {
    #[error(transparent)]
    Manifest(#[from] crate::manifest::ManifestError),
    #[error(transparent)]
    Evaluate(#[from] crate::evaluate::EvaluateError),
    #[error(transparent)]
    ProgramState(#[from] ProgramStateError),
    #[error("selected lane `{lane}` does not exist")]
    MissingLane { lane: String },
    #[error("lane `{lane}` is not ready to execute")]
    LaneNotReady { lane: String },
    #[error("run config for lane `{lane}` does not exist at {path}")]
    MissingRunConfig { lane: String, path: PathBuf },
    #[error("program manifest for lane `{lane}` does not exist at {path}")]
    MissingProgramManifest { lane: String, path: PathBuf },
    #[error("integration lane `{lane}` is invalid: {message}")]
    InvalidIntegrationLane { lane: String, message: String },
    #[error("integration lane `{lane}` is missing an `integration` artifact path")]
    MissingIntegrationArtifact { lane: String },
    #[error("integration lane `{lane}` has no source run id for `{source_lane}`")]
    MissingIntegrationSourceRunId { lane: String, source_lane: String },
    #[error("failed to spawn fabro for lane `{lane}` at {path}: {source}")]
    Spawn {
        lane: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("worker thread panicked while dispatching lane `{lane}`")]
    WorkerPanicked { lane: String },
    #[error("fabro detach for lane `{lane}` exited successfully but returned no run id")]
    MissingRunId { lane: String },
}

pub fn execute_selected_lanes(
    manifest_path: &Path,
    selected_lanes: &[String],
    settings: &DispatchSettings,
) -> Result<Vec<DispatchOutcome>, DispatchError> {
    let manifest = ProgramManifest::load(manifest_path)?;
    let evaluated = evaluate_program(manifest_path)?;
    let mut state =
        ProgramRuntimeState::load_optional(&manifest.resolved_state_path(manifest_path))?
            .unwrap_or_else(|| ProgramRuntimeState::new(manifest.program.clone()));
    let max_parallel = settings
        .max_parallel_override
        .unwrap_or(manifest.max_parallel)
        .max(1);

    let explicitly_selected = !selected_lanes.is_empty();
    let ready_lanes = if selected_lanes.is_empty() {
        evaluated
            .lanes
            .iter()
            .filter(|lane| lane.status == LaneExecutionStatus::Ready)
            .map(|lane| lane.lane_key.clone())
            .collect::<Vec<_>>()
    } else {
        selected_lanes.to_vec()
    };

    let target_repo = manifest.resolved_target_repo(manifest_path);
    let settings = Arc::new(settings.clone());
    let mut outcomes = Vec::new();
    for chunk in ready_lanes.chunks(max_parallel) {
        let mut chunk_lanes = Vec::new();
        for lane_key in chunk {
            let lane = evaluated
                .lanes
                .iter()
                .find(|lane| lane.lane_key == *lane_key)
                .ok_or_else(|| DispatchError::MissingLane {
                    lane: lane_key.clone(),
                })?;
            let allowed = lane.status == LaneExecutionStatus::Ready
                || (explicitly_selected
                    && matches!(
                        lane.status,
                        LaneExecutionStatus::Failed | LaneExecutionStatus::Complete
                    ));
            if !allowed {
                return Err(DispatchError::LaneNotReady {
                    lane: lane.lane_key.clone(),
                });
            }
            chunk_lanes.push(lane.clone());
        }

        let mut handles = Vec::new();
        for lane in chunk_lanes {
            let settings = Arc::clone(&settings);
            let target_repo = target_repo.clone();
            let child_manifest =
                manifest.resolve_lane_program_manifest(manifest_path, &lane.unit_id, &lane.lane_id);
            let is_program_lane = child_manifest.is_some();
            let is_integration_lane = lane.lane_kind == LaneKind::Integration;
            let integration_request = if is_integration_lane {
                Some(build_integration_request(
                    manifest_path,
                    &manifest,
                    &state,
                    &lane,
                )?)
            } else {
                None
            };
            let lane_key = lane.lane_key.clone();
            handles.push((
                lane_key,
                thread::spawn(move || {
                    let outcome = if let Some(request) = integration_request {
                        match integrate_lane(&request) {
                            Ok(outcome) => Ok(outcome),
                            Err(error) => Ok(DispatchOutcome {
                                lane_key: request.lane_key,
                                exit_status: 1,
                                fabro_run_id: None,
                                stdout: String::new(),
                                stderr: error.to_string(),
                            }),
                        }
                    } else if let Some(child_manifest) = child_manifest {
                        run_program_lane(&lane.lane_key, &child_manifest, &settings)
                    } else {
                        run_fabro(
                            &settings.fabro_bin,
                            &target_repo,
                            &lane.run_config,
                            &lane.lane_key,
                        )
                    };
                    (lane, is_program_lane, is_integration_lane, outcome)
                }),
            ));
        }

        for (lane_key, handle) in handles {
            let (lane, is_program_lane, is_integration_lane, output) = handle
                .join()
                .map_err(|_| DispatchError::WorkerPanicked { lane: lane_key })?;
            let output = output?;
            if is_integration_lane {
                mark_lane_finished(&mut state, &lane.lane_key, &lane.run_config, &output);
            } else if output.exit_status == 0 {
                if is_program_lane && output.fabro_run_id.is_none() {
                    mark_lane_started(&mut state, &lane.lane_key, &lane.run_config);
                } else {
                    let Some(run_id) = output.fabro_run_id.as_deref() else {
                        return Err(DispatchError::MissingRunId {
                            lane: lane.lane_key.clone(),
                        });
                    };
                    mark_lane_submitted(&mut state, &lane.lane_key, &lane.run_config, run_id);
                }
            } else {
                mark_lane_dispatch_failed(&mut state, &lane.lane_key, &lane.run_config, &output);
            }
            outcomes.push(output);
        }
        state.save(&manifest.resolved_state_path(manifest_path))?;
        refresh_parent_programs(manifest_path, &manifest)?;
    }

    Ok(outcomes)
}

fn build_integration_request(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    state: &ProgramRuntimeState,
    lane: &crate::evaluate::EvaluatedLane,
) -> Result<IntegrationRequest, DispatchError> {
    let lane_manifest = manifest
        .units
        .get(&lane.unit_id)
        .and_then(|unit| unit.lanes.get(&lane.lane_id))
        .ok_or_else(|| DispatchError::InvalidIntegrationLane {
            lane: lane.lane_key.clone(),
            message: "lane manifest is missing".to_string(),
        })?;
    let source_dependency = lane_manifest
        .dependencies
        .iter()
        .find(|dependency| dependency.lane.is_some())
        .ok_or_else(|| DispatchError::InvalidIntegrationLane {
            lane: lane.lane_key.clone(),
            message: "expected a lane dependency on the settled implementation lane".to_string(),
        })?;
    let source_lane_id = source_dependency
        .lane
        .as_ref()
        .expect("lane dependency exists");
    let source_lane_key = format!("{}:{}", source_dependency.unit, source_lane_id);
    let source_record = state.lanes.get(&source_lane_key).ok_or_else(|| {
        DispatchError::MissingIntegrationSourceRunId {
            lane: lane.lane_key.clone(),
            source_lane: source_lane_key.clone(),
        }
    })?;
    let source_run_id = source_record
        .last_run_id
        .as_deref()
        .or(source_record.current_fabro_run_id.as_deref())
        .or(source_record.current_run_id.as_deref())
        .ok_or_else(|| DispatchError::MissingIntegrationSourceRunId {
            lane: lane.lane_key.clone(),
            source_lane: source_lane_key.clone(),
        })?;
    let artifact_path = manifest
        .resolve_lane_artifacts(manifest_path, &lane.unit_id, &lane.lane_id)
        .into_iter()
        .find(|artifact| artifact.id == "integration")
        .map(|artifact| artifact.path)
        .ok_or_else(|| DispatchError::MissingIntegrationArtifact {
            lane: lane.lane_key.clone(),
        })?;

    Ok(IntegrationRequest {
        lane_key: lane.lane_key.clone(),
        source_lane_key,
        source_run_id: source_run_id.to_string(),
        target_repo: manifest.resolved_target_repo(manifest_path),
        artifact_path,
    })
}

fn run_program_lane(
    lane_key: &str,
    program_manifest: &Path,
    settings: &DispatchSettings,
) -> Result<DispatchOutcome, DispatchError> {
    if !program_manifest.exists() {
        return Err(DispatchError::MissingProgramManifest {
            lane: lane_key.to_string(),
            path: program_manifest.to_path_buf(),
        });
    }

    let child_manifest = ProgramManifest::load(program_manifest)?;
    let preview_evolve_root = settings
        .preview_evolve_root
        .as_ref()
        .map(|root| root.join(&child_manifest.program));
    let result = orchestrate_program(
        program_manifest,
        &AutodevSettings {
            fabro_bin: settings.fabro_bin.clone(),
            max_parallel_override: None,
            frontier_budget: None,
            max_cycles: 1,
            poll_interval_ms: 1,
            evolve_every_seconds: 0,
            doctrine_files: settings.doctrine_files.clone(),
            evidence_paths: settings.evidence_paths.clone(),
            preview_evolve_root,
        },
    );

    match result {
        Ok(report) => Ok(DispatchOutcome {
            lane_key: lane_key.to_string(),
            exit_status: 0,
            fabro_run_id: None,
            stdout: format!(
                "child_program={} stop_reason={:?} cycles={}",
                report.program,
                report.stop_reason,
                report.cycles.len()
            ),
            stderr: String::new(),
        }),
        Err(error) => Ok(DispatchOutcome {
            lane_key: lane_key.to_string(),
            exit_status: 1,
            fabro_run_id: None,
            stdout: String::new(),
            stderr: error.to_string(),
        }),
    }
}

fn run_fabro(
    fabro_bin: &Path,
    target_repo: &Path,
    run_config: &Path,
    lane_key: &str,
) -> Result<DispatchOutcome, DispatchError> {
    if !run_config.exists() {
        return Err(DispatchError::MissingRunConfig {
            lane: lane_key.to_string(),
            path: run_config.to_path_buf(),
        });
    }

    let command = format!(
        "export CARGO_TARGET_DIR={}; exec {} --no-upgrade-check run --detach {}",
        shell_escape(&autodev_cargo_target_dir(target_repo).display().to_string()),
        shell_escape(&fabro_bin.display().to_string()),
        shell_escape(&run_config.display().to_string()),
    );

    let output = Command::new("bash")
        .current_dir(target_repo)
        .arg("-ic")
        .arg(command)
        .output()
        .map_err(|source| DispatchError::Spawn {
            lane: lane_key.to_string(),
            path: run_config.to_path_buf(),
            source,
        })?;

    Ok(DispatchOutcome {
        lane_key: lane_key.to_string(),
        exit_status: output.status.code().unwrap_or(-1),
        fabro_run_id: parse_detached_run_id(&output.stdout),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn parse_detached_run_id(stdout: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(stdout);
    text.lines().find_map(|line| {
        let candidate = line.trim();
        if candidate.len() == 26
            && candidate
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
        {
            Some(candidate.to_string())
        } else {
            None
        }
    })
}
