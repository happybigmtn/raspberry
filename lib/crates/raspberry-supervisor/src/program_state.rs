use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use fabro_workflows::conclusion::Conclusion;
use fabro_workflows::live_state::RunLiveState;
use fabro_workflows::run_inspect::{finished_at, inspect_run, summarize_usage, RunInspection};
use fabro_workflows::run_status::{RunStatus, RunStatusRecord};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::evaluate::LaneExecutionStatus;
use crate::manifest::ProgramManifest;

const PROGRAM_STATE_SCHEMA_VERSION: &str = "raspberry.program.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramRuntimeState {
    pub schema_version: String,
    pub program: String,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub lanes: BTreeMap<String, LaneRuntimeRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneRuntimeRecord {
    pub lane_key: String,
    pub status: LaneExecutionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_config: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_fabro_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_stage_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_started_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_finished_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_exit_status: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_completed_stage_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_stage_duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_usage_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_files_read: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_files_written: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_stdout_snippet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_stderr_snippet: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LiveLaneProgress {
    pub fabro_run_id: Option<String>,
    pub workflow_status: Option<RunStatus>,
    pub current_stage_label: Option<String>,
    pub last_completed_stage_label: Option<String>,
    pub last_stage_duration_ms: Option<u64>,
    pub last_usage_summary: Option<String>,
    pub last_files_read: Vec<String>,
    pub last_files_written: Vec<String>,
    pub latest_event: Option<String>,
    pub last_failure: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Error)]
pub enum ProgramStateError {
    #[error("failed to read program state {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse program state {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to write program state {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize program state for {path}: {source}")]
    Serialize {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

impl ProgramRuntimeState {
    pub fn load(path: &Path) -> Result<Self, ProgramStateError> {
        let raw = std::fs::read_to_string(path).map_err(|source| ProgramStateError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_str(&raw).map_err(|source| ProgramStateError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn load_optional(path: &Path) -> Result<Option<Self>, ProgramStateError> {
        if !path.exists() {
            return Ok(None);
        }
        Self::load(path).map(Some)
    }

    pub fn new(program: impl Into<String>) -> Self {
        Self {
            schema_version: PROGRAM_STATE_SCHEMA_VERSION.to_string(),
            program: program.into(),
            updated_at: Utc::now(),
            lanes: BTreeMap::new(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), ProgramStateError> {
        let json =
            serde_json::to_string_pretty(self).map_err(|source| ProgramStateError::Serialize {
                path: path.to_path_buf(),
                source,
            })?;
        write_atomic(path, &json).map_err(|source| ProgramStateError::Write {
            path: path.to_path_buf(),
            source,
        })
    }
}

pub fn ensure_lane_record<'a>(
    state: &'a mut ProgramRuntimeState,
    lane_key: &str,
    run_config: &Path,
) -> &'a mut LaneRuntimeRecord {
    state
        .lanes
        .entry(lane_key.to_string())
        .or_insert_with(|| LaneRuntimeRecord {
            lane_key: lane_key.to_string(),
            status: LaneExecutionStatus::Blocked,
            run_config: Some(run_config.to_path_buf()),
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage_label: None,
            last_run_id: None,
            last_started_at: None,
            last_finished_at: None,
            last_exit_status: None,
            last_error: None,
            last_completed_stage_label: None,
            last_stage_duration_ms: None,
            last_usage_summary: None,
            last_files_read: Vec::new(),
            last_files_written: Vec::new(),
            last_stdout_snippet: None,
            last_stderr_snippet: None,
        })
}

pub fn mark_lane_submitted(
    state: &mut ProgramRuntimeState,
    lane_key: &str,
    run_config: &Path,
    fabro_run_id: &str,
) {
    let now = Utc::now();
    let record = ensure_lane_record(state, lane_key, run_config);
    record.status = LaneExecutionStatus::Running;
    record.current_run_id = Some(fabro_run_id.to_string());
    record.current_fabro_run_id = Some(fabro_run_id.to_string());
    record.current_stage_label = None;
    record.last_run_id = Some(fabro_run_id.to_string());
    record.last_started_at = Some(now);
    record.last_finished_at = None;
    record.last_exit_status = None;
    record.last_error = None;
    record.last_completed_stage_label = None;
    record.last_stage_duration_ms = None;
    record.last_usage_summary = None;
    record.last_files_read.clear();
    record.last_files_written.clear();
    record.last_stdout_snippet = None;
    record.last_stderr_snippet = None;
    state.updated_at = now;
}

pub fn mark_lane_started(
    state: &mut ProgramRuntimeState,
    lane_key: &str,
    run_config: &Path,
) {
    let now = Utc::now();
    let record = ensure_lane_record(state, lane_key, run_config);
    record.status = LaneExecutionStatus::Running;
    record.current_run_id = None;
    record.current_fabro_run_id = None;
    record.current_stage_label = None;
    record.last_finished_at = None;
    record.last_exit_status = None;
    record.last_error = None;
    record.last_completed_stage_label = None;
    record.last_stage_duration_ms = None;
    record.last_usage_summary = None;
    record.last_files_read.clear();
    record.last_files_written.clear();
    record.last_stdout_snippet = None;
    record.last_stderr_snippet = None;
    if record.last_started_at.is_none() {
        record.last_started_at = Some(now);
    }
    state.updated_at = now;
}

pub fn mark_lane_dispatch_failed(
    state: &mut ProgramRuntimeState,
    lane_key: &str,
    run_config: &Path,
    outcome: &crate::dispatch::DispatchOutcome,
) {
    let now = Utc::now();
    let record = ensure_lane_record(state, lane_key, run_config);
    record.status = LaneExecutionStatus::Failed;
    record.current_run_id = None;
    record.current_fabro_run_id = outcome.fabro_run_id.clone();
    record.current_stage_label = None;
    record.last_run_id = outcome.fabro_run_id.clone();
    record.last_finished_at = Some(now);
    record.last_exit_status = Some(outcome.exit_status);
    record.last_error = Some(summarize_failure(outcome));
    record.last_usage_summary = Some(operator_summary(outcome));
    record.last_files_read = Vec::new();
    record.last_files_written = Vec::new();
    record.last_stdout_snippet = non_empty_snippet(&outcome.stdout);
    record.last_stderr_snippet = non_empty_snippet(&outcome.stderr);
    state.updated_at = now;
}

pub fn refresh_program_state(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    state: &mut ProgramRuntimeState,
) -> Result<bool, ProgramStateError> {
    let mut changed = false;
    for (unit_id, unit) in &manifest.units {
        for (lane_id, lane) in &unit.lanes {
            let lane_key = format!("{unit_id}:{lane_id}");
            let run_config = manifest
                .resolve_lane_run_config(manifest_path, unit_id, lane_id)
                .unwrap_or_else(|| lane.run_config.clone());
            let record = ensure_lane_record(state, &lane_key, &run_config);
            let progress = if let Some(run_id) = tracked_run_id(record) {
                read_live_lane_progress_for_run_id(run_id)?
            } else if let Some(run_dir) =
                manifest.resolve_lane_run_dir(manifest_path, unit_id, lane_id)
            {
                read_live_lane_progress(&run_dir)?
            } else {
                None
            };
            let Some(progress) = progress else {
                continue;
            };
            let mut lane_changed = false;

            lane_changed |= assign_opt(
                &mut record.current_fabro_run_id,
                progress.fabro_run_id.clone(),
            );
            lane_changed |= assign_opt(&mut record.current_run_id, progress.fabro_run_id.clone());
            lane_changed |= assign_opt(&mut record.last_run_id, progress.fabro_run_id.clone());
            lane_changed |= assign_opt(
                &mut record.current_stage_label,
                progress.current_stage_label.clone(),
            );
            lane_changed |= assign_opt(
                &mut record.last_completed_stage_label,
                progress.last_completed_stage_label.clone(),
            );
            lane_changed |= assign_opt(
                &mut record.last_stage_duration_ms,
                progress.last_stage_duration_ms,
            );
            lane_changed |= assign_opt(
                &mut record.last_usage_summary,
                progress.last_usage_summary.clone(),
            );
            lane_changed |= assign_opt(&mut record.last_error, progress.last_failure.clone());
            lane_changed |= assign_opt(&mut record.last_finished_at, progress.finished_at);
            if record.last_files_read != progress.last_files_read {
                record.last_files_read = progress.last_files_read.clone();
                lane_changed = true;
            }
            if record.last_files_written != progress.last_files_written {
                record.last_files_written = progress.last_files_written.clone();
                lane_changed = true;
            }
            match progress.workflow_status {
                Some(status) if status.is_active() => {
                    if record.status != LaneExecutionStatus::Running {
                        record.status = LaneExecutionStatus::Running;
                        lane_changed = true;
                    }
                    if record.last_finished_at.take().is_some() {
                        lane_changed = true;
                    }
                }
                Some(RunStatus::Succeeded) => {
                    if record.status == LaneExecutionStatus::Running {
                        record.status = LaneExecutionStatus::Ready;
                        lane_changed = true;
                    }
                    if record.last_exit_status != Some(0) {
                        record.last_exit_status = Some(0);
                        lane_changed = true;
                    }
                }
                Some(RunStatus::Failed | RunStatus::Dead) => {
                    if record.status != LaneExecutionStatus::Failed {
                        record.status = LaneExecutionStatus::Failed;
                        lane_changed = true;
                    }
                }
                _ => {}
            }
            if let Some(updated_at) = progress.updated_at {
                if record.last_started_at.is_none() {
                    record.last_started_at = Some(updated_at);
                    lane_changed = true;
                }
            }

            if lane_changed {
                changed = true;
            }
        }
    }
    if changed {
        state.updated_at = Utc::now();
    }
    Ok(changed)
}

fn assign_opt<T: PartialEq>(slot: &mut Option<T>, next: Option<T>) -> bool {
    if *slot != next {
        *slot = next;
        true
    } else {
        false
    }
}

fn tracked_run_id(record: &LaneRuntimeRecord) -> Option<&str> {
    record
        .current_fabro_run_id
        .as_deref()
        .or(record.last_run_id.as_deref())
        .or(record.current_run_id.as_deref())
}

fn read_live_lane_progress_for_run_id(
    run_id: &str,
) -> Result<Option<LiveLaneProgress>, ProgramStateError> {
    let base = fabro_workflows::run_lookup::default_runs_base();
    let inspection = match inspect_run(&base, run_id) {
        Ok(inspection) => inspection,
        Err(_) => return Ok(None),
    };
    Ok(Some(live_lane_progress_from_inspection(&inspection)))
}

fn read_live_lane_progress(run_root: &Path) -> Result<Option<LiveLaneProgress>, ProgramStateError> {
    let state_path = run_root.join("state.json");
    let run_state = RunLiveState::load(&state_path).ok();
    let run_status = RunStatusRecord::load(&run_root.join("status.json")).ok();
    let conclusion = Conclusion::load(&run_root.join("conclusion.json")).ok();
    let progress_path = run_root.join("progress.jsonl");
    let contents = match std::fs::read_to_string(&progress_path) {
        Ok(contents) => contents,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(source) => {
            return Err(ProgramStateError::Read {
                path: progress_path,
                source,
            });
        }
    };

    let mut progress = LiveLaneProgress {
        fabro_run_id: run_state.as_ref().map(|state| state.run_id.clone()),
        workflow_status: run_state
            .as_ref()
            .map(|state| state.status)
            .or_else(|| run_status.as_ref().map(|status| status.status)),
        current_stage_label: run_state
            .as_ref()
            .and_then(|state| state.current_stage_label.clone()),
        last_completed_stage_label: run_state
            .as_ref()
            .and_then(|state| state.last_completed_stage_label.clone()),
        last_failure: run_state
            .as_ref()
            .and_then(|state| state.last_failure.clone()),
        updated_at: run_state.as_ref().map(|state| state.updated_at),
        finished_at: conclusion.as_ref().map(|conclusion| conclusion.timestamp),
        ..LiveLaneProgress::default()
    };

    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let event = value
            .get("event")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        progress.latest_event = Some(event.clone());
        match event.as_str() {
            "StageStarted" => {
                progress.current_stage_label = value
                    .get("node_label")
                    .or_else(|| value.get("name"))
                    .and_then(|value| value.as_str())
                    .map(ToOwned::to_owned);
            }
            "StageCompleted" => {
                progress.last_completed_stage_label = value
                    .get("node_label")
                    .or_else(|| value.get("name"))
                    .and_then(|value| value.as_str())
                    .map(ToOwned::to_owned);
                progress.last_stage_duration_ms =
                    value.get("duration_ms").and_then(|value| value.as_u64());
                progress.last_files_read = parse_string_array(&value, "files_read");
                progress.last_files_written = parse_string_array(&value, "files_written");
                progress.last_usage_summary = summarize_event_usage(&value);
                progress.current_stage_label = None;
            }
            "WorkflowRunCompleted" => {
                progress.workflow_status = Some(RunStatus::Succeeded);
                progress.last_usage_summary = summarize_event_usage(&value);
                progress.current_stage_label = None;
            }
            "WorkflowRunFailed" => {
                progress.workflow_status = Some(RunStatus::Failed);
                progress.last_failure = value
                    .get("error")
                    .and_then(|value| value.as_str())
                    .map(ToOwned::to_owned)
                    .or_else(|| {
                        value
                            .get("failure_reason")
                            .and_then(|value| value.as_str())
                            .map(ToOwned::to_owned)
                    });
                progress.current_stage_label = None;
            }
            _ => {}
        }
    }

    if progress.fabro_run_id.is_none() {
        progress.fabro_run_id = run_root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
    }
    if progress.fabro_run_id.is_none()
        && progress.latest_event.is_none()
        && progress.workflow_status.is_none()
        && progress.current_stage_label.is_none()
        && progress.last_completed_stage_label.is_none()
    {
        return Ok(None);
    }
    Ok(Some(progress))
}

fn live_lane_progress_from_inspection(inspection: &RunInspection) -> LiveLaneProgress {
    let state = inspection.state.as_ref();
    LiveLaneProgress {
        fabro_run_id: Some(inspection.run_id.clone()),
        workflow_status: Some(inspection.status),
        current_stage_label: inspection
            .progress
            .current_stage_label
            .clone()
            .or_else(|| state.and_then(|state| state.current_stage_label.clone())),
        last_completed_stage_label: inspection
            .progress
            .last_completed_stage_label
            .clone()
            .or_else(|| state.and_then(|state| state.last_completed_stage_label.clone())),
        last_stage_duration_ms: inspection.progress.last_stage_duration_ms,
        last_usage_summary: summarize_usage(inspection.progress.last_usage.as_ref()),
        last_files_read: if inspection.progress.last_files_read.is_empty() {
            state
                .map(|state| state.last_files_read.clone())
                .unwrap_or_default()
        } else {
            inspection.progress.last_files_read.clone()
        },
        last_files_written: if inspection.progress.last_files_written.is_empty() {
            state
                .map(|state| state.last_files_written.clone())
                .unwrap_or_default()
        } else {
            inspection.progress.last_files_written.clone()
        },
        latest_event: inspection
            .progress
            .latest_event
            .clone()
            .or_else(|| state.and_then(|state| state.last_event.clone())),
        last_failure: state
            .and_then(|state| state.last_failure.clone())
            .or_else(|| {
                inspection
                    .conclusion
                    .as_ref()
                    .and_then(|conclusion| conclusion.failure_reason.clone())
            }),
        updated_at: state.map(|state| state.updated_at),
        finished_at: finished_at(inspection),
    }
}

fn parse_string_array(value: &serde_json::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
        .collect::<Vec<_>>()
}

fn summarize_event_usage(value: &serde_json::Value) -> Option<String> {
    let usage = value.get("usage")?;
    let model = usage
        .get("model")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown");
    let input = usage
        .get("input_tokens")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let output = usage
        .get("output_tokens")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    Some(format!("{model}: {input} in / {output} out"))
}

fn summarize_failure(outcome: &crate::dispatch::DispatchOutcome) -> String {
    let combined = format!("{}\n{}", outcome.stdout, outcome.stderr);
    combined
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("fabro run failed")
        .trim()
        .to_string()
}

fn operator_summary(outcome: &crate::dispatch::DispatchOutcome) -> String {
    let phase = if outcome.exit_status == 0 {
        "submitted"
    } else {
        "failed"
    };
    format!("{phase} with exit_status={}", outcome.exit_status)
}

fn non_empty_snippet(output: &str) -> Option<String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.chars().take(200).collect())
    }
}

fn write_atomic(path: &Path, contents: &str) -> Result<(), std::io::Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("state"),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    std::fs::write(&temp, contents)?;
    if let Err(first_error) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(path);
        if let Err(second_error) = std::fs::rename(&temp, path) {
            let _ = std::fs::remove_file(&temp);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_optional_returns_none_for_missing_state() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("missing.json");
        let state = ProgramRuntimeState::load_optional(&path).expect("load should succeed");
        assert!(state.is_none());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("state.json");
        let mut state = ProgramRuntimeState::new("demo");
        state.lanes.insert(
            "runtime:page".to_string(),
            LaneRuntimeRecord {
                lane_key: "runtime:page".to_string(),
                status: LaneExecutionStatus::Ready,
                run_config: Some(PathBuf::from("runtime-page.toml")),
                current_run_id: None,
                current_fabro_run_id: None,
                current_stage_label: None,
                last_run_id: None,
                last_started_at: None,
                last_finished_at: None,
                last_exit_status: None,
                last_error: None,
                last_completed_stage_label: None,
                last_stage_duration_ms: None,
                last_usage_summary: None,
                last_files_read: Vec::new(),
                last_files_written: Vec::new(),
                last_stdout_snippet: None,
                last_stderr_snippet: None,
            },
        );
        state.save(&path).expect("save should succeed");

        let loaded = ProgramRuntimeState::load(&path).expect("load should succeed");
        assert_eq!(loaded.program, "demo");
        assert!(loaded.lanes.contains_key("runtime:page"));
    }
}
