use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::thread;

use thiserror::Error;

use crate::evaluate::{LaneExecutionStatus, evaluate_program};
use crate::manifest::ProgramManifest;
use crate::program_state::{
    ProgramRuntimeState, ProgramStateError, mark_lane_dispatch_failed, mark_lane_submitted,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchOutcome {
    pub lane_key: String,
    pub exit_status: i32,
    pub fabro_run_id: Option<String>,
    pub stdout: String,
    pub stderr: String,
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
    fabro_bin: &Path,
    max_parallel_override: Option<usize>,
) -> Result<Vec<DispatchOutcome>, DispatchError> {
    let manifest = ProgramManifest::load(manifest_path)?;
    let evaluated = evaluate_program(manifest_path)?;
    let mut state = ProgramRuntimeState::load_optional(&manifest.resolved_state_path(manifest_path))?
        .unwrap_or_else(|| ProgramRuntimeState::new(manifest.program.clone()));
    let max_parallel = max_parallel_override
        .unwrap_or(manifest.max_parallel)
        .max(1);

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
    let fabro_bin = Arc::new(fabro_bin.to_path_buf());
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
            if lane.status != LaneExecutionStatus::Ready {
                return Err(DispatchError::LaneNotReady {
                    lane: lane.lane_key.clone(),
                });
            }
            chunk_lanes.push(lane.clone());
        }

        let mut handles = Vec::new();
        for lane in chunk_lanes {
            let fabro_bin = Arc::clone(&fabro_bin);
            let target_repo = target_repo.clone();
            let lane_key = lane.lane_key.clone();
            handles.push((lane_key, thread::spawn(move || {
                let outcome = run_fabro(&fabro_bin, &target_repo, &lane.run_config, &lane.lane_key);
                (lane, outcome)
            })));
        }

        for (lane_key, handle) in handles {
            let (lane, output) = handle
                .join()
                .map_err(|_| DispatchError::WorkerPanicked {
                    lane: lane_key,
                })?;
            let output = output?;
            if output.exit_status == 0 {
                let Some(run_id) = output.fabro_run_id.as_deref() else {
                    return Err(DispatchError::MissingRunId {
                        lane: lane.lane_key.clone(),
                    });
                };
                mark_lane_submitted(&mut state, &lane.lane_key, &lane.run_config, run_id);
            } else {
                mark_lane_dispatch_failed(&mut state, &lane.lane_key, &lane.run_config, &output);
            }
            outcomes.push(output);
        }
        state.save(&manifest.resolved_state_path(manifest_path))?;
    }

    Ok(outcomes)
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

    let output = Command::new(fabro_bin)
        .current_dir(target_repo)
        .arg("--no-upgrade-check")
        .arg("run")
        .arg("--detach")
        .arg(run_config)
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
