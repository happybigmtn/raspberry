use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use fabro_workflows::conclusion::Conclusion;
use fabro_workflows::live_state::RunLiveState;
use fabro_workflows::run_inspect::{finished_at, inspect_run, summarize_usage, RunInspection};
use fabro_workflows::run_status::{RunStatus, RunStatusRecord};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::evaluate::EvaluatedProgram;
use crate::evaluate::{evaluate_program_internal, LaneExecutionStatus};
use crate::failure::{
    classify_failure, default_recovery_action, FailureKind, FailureRecoveryAction,
};
use crate::manifest::ProgramManifest;
use crate::resource_lease;

const PROGRAM_STATE_SCHEMA_VERSION: &str = "raspberry.program.v2";
const STALE_RUNNING_GRACE_SECS: i64 = 30;

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
    pub failure_kind: Option<FailureKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_action: Option<FailureRecoveryAction>,
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
    pub worker_alive: Option<bool>,
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
    let normalized_run_config = normalize_path_lexically(run_config);
    let record = state
        .lanes
        .entry(lane_key.to_string())
        .or_insert_with(|| LaneRuntimeRecord {
            lane_key: lane_key.to_string(),
            status: LaneExecutionStatus::Blocked,
            run_config: Some(normalized_run_config.clone()),
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage_label: None,
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
        });
    if record.run_config.as_ref() != Some(&normalized_run_config) {
        record.run_config = Some(normalized_run_config);
    }
    record
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
    record.failure_kind = None;
    record.recovery_action = None;
    record.last_completed_stage_label = None;
    record.last_stage_duration_ms = None;
    record.last_usage_summary = None;
    record.last_files_read.clear();
    record.last_files_written.clear();
    record.last_stdout_snippet = None;
    record.last_stderr_snippet = None;
    state.updated_at = now;
}

pub fn mark_lane_started(state: &mut ProgramRuntimeState, lane_key: &str, run_config: &Path) {
    let now = Utc::now();
    let record = ensure_lane_record(state, lane_key, run_config);
    record.status = LaneExecutionStatus::Running;
    record.current_run_id = None;
    record.current_fabro_run_id = None;
    record.current_stage_label = None;
    record.last_finished_at = None;
    record.last_exit_status = None;
    record.last_error = None;
    record.failure_kind = None;
    record.recovery_action = None;
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
    record.failure_kind = classify_failure(
        record.last_error.as_deref(),
        Some(&outcome.stderr),
        Some(&outcome.stdout),
    );
    record.recovery_action = record.failure_kind.map(default_recovery_action);
    record.last_usage_summary = Some(operator_summary(outcome));
    record.last_files_read = Vec::new();
    record.last_files_written = Vec::new();
    record.last_stdout_snippet = non_empty_snippet(&outcome.stdout);
    record.last_stderr_snippet = non_empty_snippet(&outcome.stderr);
    state.updated_at = now;
}

pub fn mark_lane_finished(
    state: &mut ProgramRuntimeState,
    lane_key: &str,
    run_config: &Path,
    outcome: &crate::dispatch::DispatchOutcome,
) {
    let now = Utc::now();
    let record = ensure_lane_record(state, lane_key, run_config);
    record.status = if outcome.exit_status == 0 {
        LaneExecutionStatus::Complete
    } else {
        LaneExecutionStatus::Failed
    };
    record.current_run_id = None;
    record.current_fabro_run_id = outcome.fabro_run_id.clone();
    record.current_stage_label = None;
    if outcome.fabro_run_id.is_some() {
        record.last_run_id = outcome.fabro_run_id.clone();
    }
    if record.last_started_at.is_none() {
        record.last_started_at = Some(now);
    }
    record.last_finished_at = Some(now);
    record.last_exit_status = Some(outcome.exit_status);
    record.last_completed_stage_label = None;
    record.last_stage_duration_ms = None;
    record.last_error = if outcome.exit_status == 0 {
        None
    } else {
        Some(summarize_failure(outcome))
    };
    record.failure_kind = if outcome.exit_status == 0 {
        None
    } else {
        classify_failure(
            record.last_error.as_deref(),
            Some(&outcome.stderr),
            Some(&outcome.stdout),
        )
    };
    record.recovery_action = record.failure_kind.map(default_recovery_action);
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
    let mut expected_lane_keys = std::collections::BTreeSet::new();
    for (unit_id, unit) in &manifest.units {
        for (lane_id, lane) in &unit.lanes {
            let lane_key = format!("{unit_id}:{lane_id}");
            expected_lane_keys.insert(lane_key.clone());
            let run_config = manifest
                .resolve_lane_run_config(manifest_path, unit_id, lane_id)
                .unwrap_or_else(|| lane.run_config.clone());
            let record = ensure_lane_record(state, &lane_key, &run_config);
            let mut lane_changed = assign_opt(
                &mut record.run_config,
                Some(normalize_path_lexically(&run_config)),
            );
            if let Some(child_manifest) =
                manifest.resolve_lane_program_manifest(manifest_path, unit_id, lane_id)
            {
                if sync_child_program_runtime_record(record, &child_manifest) {
                    changed = true;
                }
                if lane_changed {
                    changed = true;
                }
                continue;
            }
            let resolved_run_dir = manifest.resolve_lane_run_dir(manifest_path, unit_id, lane_id);
            let progress = if let Some(run_id) = tracked_run_id(record) {
                read_live_lane_progress_for_run_id(run_id)?.or_else(|| {
                    resolved_run_dir
                        .as_ref()
                        .and_then(|run_dir| read_live_lane_progress(run_dir).ok().flatten())
                })
            } else if let Some(run_dir) = resolved_run_dir.as_ref() {
                read_live_lane_progress(run_dir)?
            } else {
                None
            };
            let Some(mut progress) = progress else {
                continue;
            };
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
            lane_changed |= assign_opt(
                &mut record.failure_kind,
                classify_failure(record.last_error.as_deref(), None, None),
            );
            lane_changed |= assign_opt(
                &mut record.recovery_action,
                record.failure_kind.map(default_recovery_action),
            );
            lane_changed |= assign_opt(&mut record.last_finished_at, progress.finished_at);
            if stale_active_progress(&progress, record.last_started_at) {
                if record.status != LaneExecutionStatus::Failed {
                    record.status = LaneExecutionStatus::Failed;
                    lane_changed = true;
                }
                if record.last_error.is_none() {
                    record.last_error = Some(
                        "tracked run remained active after its worker process disappeared"
                            .to_string(),
                    );
                    lane_changed = true;
                }
                if record.failure_kind != Some(FailureKind::TransientLaunchFailure) {
                    record.failure_kind = Some(FailureKind::TransientLaunchFailure);
                    lane_changed = true;
                }
                if record.recovery_action != Some(FailureRecoveryAction::BackoffRetry) {
                    record.recovery_action = Some(FailureRecoveryAction::BackoffRetry);
                    lane_changed = true;
                }
                let finished_at = progress.updated_at.unwrap_or_else(Utc::now);
                if record.last_finished_at != Some(finished_at) {
                    record.last_finished_at = Some(finished_at);
                    lane_changed = true;
                }
                if record.last_exit_status != Some(1) {
                    record.last_exit_status = Some(1);
                    lane_changed = true;
                }
                if record.current_stage_label.take().is_some() {
                    lane_changed = true;
                }
                if record.current_run_id.take().is_some() {
                    lane_changed = true;
                }
                if record.current_fabro_run_id.take().is_some() {
                    lane_changed = true;
                }
                progress.workflow_status = Some(RunStatus::Failed);
            }
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
                    if record.failure_kind.take().is_some() {
                        lane_changed = true;
                    }
                    if record.recovery_action.take().is_some() {
                        lane_changed = true;
                    }
                    if record.last_error.take().is_some() {
                        lane_changed = true;
                    }
                    if record.last_stdout_snippet.take().is_some() {
                        lane_changed = true;
                    }
                    if record.last_stderr_snippet.take().is_some() {
                        lane_changed = true;
                    }
                }
                Some(RunStatus::Succeeded) => {
                    if record.status == LaneExecutionStatus::Running {
                        record.status = LaneExecutionStatus::Ready;
                        lane_changed = true;
                    }
                    if record.current_stage_label.take().is_some() {
                        lane_changed = true;
                    }
                    if record.current_run_id.take().is_some() {
                        lane_changed = true;
                    }
                    if record.current_fabro_run_id.take().is_some() {
                        lane_changed = true;
                    }
                    if record.last_exit_status != Some(0) {
                        record.last_exit_status = Some(0);
                        lane_changed = true;
                    }
                    if record.failure_kind.take().is_some() {
                        lane_changed = true;
                    }
                    if record.recovery_action.take().is_some() {
                        lane_changed = true;
                    }
                    if record.last_error.take().is_some() {
                        lane_changed = true;
                    }
                    if record.last_stdout_snippet.take().is_some() {
                        lane_changed = true;
                    }
                    if record.last_stderr_snippet.take().is_some() {
                        lane_changed = true;
                    }
                }
                Some(RunStatus::Failed | RunStatus::Dead) => {
                    if record.status != LaneExecutionStatus::Failed {
                        record.status = LaneExecutionStatus::Failed;
                        lane_changed = true;
                    }
                    if record.current_stage_label.take().is_some() {
                        lane_changed = true;
                    }
                    if record.current_run_id.take().is_some() {
                        lane_changed = true;
                    }
                    if record.current_fabro_run_id.take().is_some() {
                        lane_changed = true;
                    }
                    let next_failure_kind =
                        classify_failure(record.last_error.as_deref(), None, None);
                    if record.failure_kind != next_failure_kind {
                        record.failure_kind = next_failure_kind;
                        lane_changed = true;
                    }
                    let next_recovery_action = record.failure_kind.map(default_recovery_action);
                    if record.recovery_action != next_recovery_action {
                        record.recovery_action = next_recovery_action;
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
    let original_len = state.lanes.len();
    state
        .lanes
        .retain(|lane_key, _| expected_lane_keys.contains(lane_key));
    if state.lanes.len() != original_len {
        changed = true;
    }
    let running_lane_keys = state
        .lanes
        .iter()
        .filter(|(_, record)| record.status == LaneExecutionStatus::Running)
        .map(|(lane_key, _)| lane_key.clone())
        .collect::<std::collections::BTreeSet<_>>();
    if resource_lease::cleanup_leases(
        &manifest.resolved_target_repo(manifest_path),
        &running_lane_keys,
    )
    .map_err(|error| ProgramStateError::Read {
        path: manifest.resolved_target_repo(manifest_path),
        source: std::io::Error::other(error.to_string()),
    })? {
        changed = true;
    }
    if changed {
        state.updated_at = Utc::now();
    }
    Ok(changed)
}

pub fn sync_program_state_with_evaluated(
    state: &mut ProgramRuntimeState,
    program: &EvaluatedProgram,
) -> bool {
    let mut changed = false;
    for lane in &program.lanes {
        let record = ensure_lane_record(state, &lane.lane_key, &lane.run_config);
        changed |= assign_opt(
            &mut record.run_config,
            Some(normalize_path_lexically(&lane.run_config)),
        );
        changed |= assign_value(&mut record.status, lane.status);
        changed |= assign_opt(&mut record.current_run_id, lane.current_run_id.clone());
        changed |= assign_opt(
            &mut record.current_fabro_run_id,
            lane.current_fabro_run_id.clone(),
        );
        changed |= assign_opt(&mut record.current_stage_label, lane.current_stage.clone());
        changed |= assign_opt(&mut record.last_run_id, lane.last_run_id.clone());
        changed |= assign_opt(&mut record.last_started_at, lane.last_started_at);
        changed |= assign_opt(&mut record.last_finished_at, lane.last_finished_at);
        changed |= assign_opt(&mut record.last_exit_status, lane.last_exit_status);
        changed |= assign_opt(&mut record.last_error, lane.last_error.clone());
        changed |= assign_opt(&mut record.failure_kind, lane.failure_kind);
        changed |= assign_opt(&mut record.recovery_action, lane.recovery_action);
        changed |= assign_opt(
            &mut record.last_completed_stage_label,
            lane.last_completed_stage_label.clone(),
        );
        changed |= assign_opt(
            &mut record.last_stage_duration_ms,
            lane.last_stage_duration_ms,
        );
        changed |= assign_opt(
            &mut record.last_usage_summary,
            lane.last_usage_summary.clone(),
        );
        if record.last_files_read != lane.last_files_read {
            record.last_files_read = lane.last_files_read.clone();
            changed = true;
        }
        if record.last_files_written != lane.last_files_written {
            record.last_files_written = lane.last_files_written.clone();
            changed = true;
        }
        changed |= assign_opt(
            &mut record.last_stdout_snippet,
            lane.last_stdout_snippet.clone(),
        );
        changed |= assign_opt(
            &mut record.last_stderr_snippet,
            lane.last_stderr_snippet.clone(),
        );
        if lane.status != LaneExecutionStatus::Failed {
            if record.last_error.take().is_some() {
                changed = true;
            }
            if record.failure_kind.take().is_some() {
                changed = true;
            }
            if record.recovery_action.take().is_some() {
                changed = true;
            }
            if record.last_stdout_snippet.take().is_some() {
                changed = true;
            }
            if record.last_stderr_snippet.take().is_some() {
                changed = true;
            }
        }
    }
    if changed {
        state.updated_at = Utc::now();
    }
    changed
}

fn sync_child_program_runtime_record(
    record: &mut LaneRuntimeRecord,
    child_manifest: &Path,
) -> bool {
    if crate::evaluate::evaluation_stack_contains(child_manifest) {
        return false;
    }
    let mut child_has_ready = false;
    let mut child_has_running = false;
    let mut child_has_failed = false;
    let mut child_total = 0usize;
    let mut child_ready = 0usize;
    let mut child_running = 0usize;
    let mut child_failed = 0usize;
    let mut child_complete = 0usize;
    let mut child_running_record: Option<LaneRuntimeRecord> = None;
    let mut child_failed_record: Option<LaneRuntimeRecord> = None;
    let mut latest_started_at: Option<DateTime<Utc>> = None;
    let mut latest_finished_at: Option<DateTime<Utc>> = None;
    if let Ok(child_manifest_spec) = ProgramManifest::load(child_manifest) {
        let state_path = child_manifest_spec.resolved_state_path(child_manifest);
        if let Ok(child_state) = ProgramRuntimeState::load_optional(&state_path) {
            let mut child_state = child_state
                .unwrap_or_else(|| ProgramRuntimeState::new(child_manifest_spec.program.clone()));
            if let Ok(changed) =
                refresh_program_state(child_manifest, &child_manifest_spec, &mut child_state)
            {
                if changed {
                    let _ = child_state.save(&state_path);
                }
            }
            child_total = child_state.lanes.len();
            child_has_running = child_state.lanes.values().any(|lane| {
                lane.status == LaneExecutionStatus::Running && lane.last_finished_at.is_none()
            });
            child_has_ready = child_state
                .lanes
                .values()
                .any(|lane| lane.status == LaneExecutionStatus::Ready);
            child_has_failed = child_state
                .lanes
                .values()
                .any(|lane| lane.status == LaneExecutionStatus::Failed);
            for lane in child_state.lanes.values() {
                match lane.status {
                    LaneExecutionStatus::Ready => child_ready += 1,
                    LaneExecutionStatus::Running => child_running += 1,
                    LaneExecutionStatus::Failed => child_failed += 1,
                    LaneExecutionStatus::Complete => child_complete += 1,
                    LaneExecutionStatus::Blocked => {}
                }
            }
            child_running_record = child_state
                .lanes
                .values()
                .filter(|lane| {
                    lane.status == LaneExecutionStatus::Running && lane.last_finished_at.is_none()
                })
                .max_by_key(|lane| lane.last_started_at)
                .cloned();
            child_failed_record = child_state
                .lanes
                .values()
                .filter(|lane| lane.status == LaneExecutionStatus::Failed)
                .max_by_key(|lane| lane.last_finished_at.or(lane.last_started_at))
                .cloned();
            latest_started_at = child_state
                .lanes
                .values()
                .filter_map(|lane| lane.last_started_at)
                .max();
            latest_finished_at = child_state
                .lanes
                .values()
                .filter_map(|lane| lane.last_finished_at)
                .max();
        }
    }
    if child_total == 0 {
        let Ok(program) = evaluate_program_internal(child_manifest, false) else {
            return false;
        };
        child_total = program.lanes.len();
        for lane in &program.lanes {
            match lane.status {
                LaneExecutionStatus::Ready => child_ready += 1,
                LaneExecutionStatus::Running => child_running += 1,
                LaneExecutionStatus::Failed => child_failed += 1,
                LaneExecutionStatus::Complete => child_complete += 1,
                LaneExecutionStatus::Blocked => {}
            }
        }
        child_has_ready = child_ready > 0;
        child_has_running = child_running > 0;
        child_has_failed = child_failed > 0;
    }

    let next_status = if child_has_running || child_running > 0 {
        LaneExecutionStatus::Running
    } else if child_has_failed || child_failed > 0 {
        LaneExecutionStatus::Failed
    } else if child_has_ready {
        LaneExecutionStatus::Ready
    } else if child_complete == child_total && child_total > 0 {
        LaneExecutionStatus::Complete
    } else if child_ready > 0 {
        LaneExecutionStatus::Ready
    } else {
        LaneExecutionStatus::Blocked
    };

    let was_running = record.status == LaneExecutionStatus::Running;
    let mut changed = false;
    if record.status != next_status {
        record.status = next_status;
        changed = true;
    }
    if let Some(child) = child_running_record.as_ref() {
        changed |= assign_opt(&mut record.current_run_id, child.current_run_id.clone());
        changed |= assign_opt(
            &mut record.current_fabro_run_id,
            child.current_fabro_run_id.clone(),
        );
        changed |= assign_opt(
            &mut record.current_stage_label,
            child.current_stage_label.clone(),
        );
        changed |= assign_opt(&mut record.last_started_at, child.last_started_at);
        changed |= assign_opt(&mut record.last_run_id, child.last_run_id.clone());
        changed |= assign_opt(
            &mut record.last_completed_stage_label,
            child.last_completed_stage_label.clone(),
        );
        changed |= assign_opt(
            &mut record.last_stage_duration_ms,
            child.last_stage_duration_ms,
        );
        changed |= assign_opt(
            &mut record.last_usage_summary,
            child.last_usage_summary.clone(),
        );
        if record.last_error.take().is_some() {
            changed = true;
        }
        if record.last_stdout_snippet.take().is_some() {
            changed = true;
        }
        if record.last_stderr_snippet.take().is_some() {
            changed = true;
        }
        if record.failure_kind.take().is_some() {
            changed = true;
        }
        if record.recovery_action.take().is_some() {
            changed = true;
        }
        if record.last_exit_status.take().is_some() {
            changed = true;
        }
    } else {
        if record.current_run_id.take().is_some() {
            changed = true;
        }
        if record.current_fabro_run_id.take().is_some() {
            changed = true;
        }
        if next_status != LaneExecutionStatus::Running
            && record.current_stage_label.take().is_some()
        {
            changed = true;
        }
        changed |= assign_opt(&mut record.last_started_at, latest_started_at);
    }
    if let Some(child) = child_failed_record.as_ref() {
        changed |= assign_opt(&mut record.last_error, child.last_error.clone());
        changed |= assign_opt(&mut record.failure_kind, child.failure_kind);
        changed |= assign_opt(&mut record.recovery_action, child.recovery_action);
        changed |= assign_opt(&mut record.last_finished_at, child.last_finished_at);
        changed |= assign_opt(&mut record.last_exit_status, child.last_exit_status);
        changed |= assign_opt(
            &mut record.last_completed_stage_label,
            child.last_completed_stage_label.clone(),
        );
        changed |= assign_opt(
            &mut record.last_stage_duration_ms,
            child.last_stage_duration_ms,
        );
        changed |= assign_opt(
            &mut record.last_usage_summary,
            child.last_usage_summary.clone(),
        );
    } else {
        if next_status != LaneExecutionStatus::Failed && record.last_error.take().is_some() {
            changed = true;
        }
        if next_status != LaneExecutionStatus::Failed && record.failure_kind.take().is_some() {
            changed = true;
        }
        if next_status != LaneExecutionStatus::Failed && record.recovery_action.take().is_some() {
            changed = true;
        }
        if next_status == LaneExecutionStatus::Complete {
            changed |= assign_opt(&mut record.last_exit_status, Some(0));
        }
        if next_status != LaneExecutionStatus::Running {
            changed |= assign_opt(&mut record.last_finished_at, latest_finished_at);
        }
    }
    if was_running
        && next_status != LaneExecutionStatus::Running
        && record.last_finished_at.is_none()
    {
        record.last_finished_at = Some(Utc::now());
        changed = true;
    }
    if next_status == LaneExecutionStatus::Running && record.last_finished_at.take().is_some() {
        changed = true;
    }
    changed
}

fn assign_opt<T: PartialEq>(slot: &mut Option<T>, next: Option<T>) -> bool {
    if *slot != next {
        *slot = next;
        true
    } else {
        false
    }
}

fn assign_value<T: PartialEq>(slot: &mut T, next: T) -> bool {
    if *slot != next {
        *slot = next;
        true
    } else {
        false
    }
}

fn normalize_path_lexically(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut normalized = PathBuf::new();
    let mut saw_root = false;

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => {
                normalized.push(component.as_os_str());
                saw_root = true;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if let Some(last) = normalized.components().next_back() {
                    match last {
                        Component::Normal(_) => {
                            normalized.pop();
                        }
                        Component::ParentDir => normalized.push(component.as_os_str()),
                        Component::RootDir | Component::Prefix(_) => {}
                        Component::CurDir => {}
                    }
                } else if !saw_root {
                    normalized.push(component.as_os_str());
                }
            }
            Component::Normal(_) => normalized.push(component.as_os_str()),
        }
    }

    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
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
    if let Ok(inspection) = inspect_run(&base, run_id) {
        if let Ok(Some(progress)) = read_live_lane_progress(&inspection.run_dir) {
            return Ok(Some(progress));
        }
        return Ok(Some(live_lane_progress_from_inspection(&inspection)));
    }
    let Ok(run_dir) = fabro_workflows::run_lookup::find_run_by_prefix(&base, run_id) else {
        return Ok(None);
    };
    read_live_lane_progress(&run_dir)
}

fn read_live_lane_progress(run_root: &Path) -> Result<Option<LiveLaneProgress>, ProgramStateError> {
    let state_path = run_root.join("state.json");
    let run_state = RunLiveState::load(&state_path).ok();
    let run_status = RunStatusRecord::load(&run_root.join("status.json")).ok();
    let conclusion = Conclusion::load(&run_root.join("conclusion.json")).ok();
    let authoritative_status =
        authoritative_failure_status(run_status.as_ref(), conclusion.as_ref());
    let cli_activity_at = latest_cli_activity_at(run_root);
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
        worker_alive: worker_process_alive(run_root),
        current_stage_label: run_state
            .as_ref()
            .and_then(|state| state.current_stage_label.clone()),
        last_completed_stage_label: run_state
            .as_ref()
            .and_then(|state| state.last_completed_stage_label.clone()),
        last_failure: merge_failure_detail(
            run_state
                .as_ref()
                .and_then(|state| state.last_failure.clone()),
            conclusion
                .as_ref()
                .and_then(|conclusion| conclusion.failure_reason.clone()),
        ),
        updated_at: max_datetime(
            run_state.as_ref().map(|state| state.updated_at),
            cli_activity_at,
        ),
        finished_at: conclusion.as_ref().map(|conclusion| conclusion.timestamp),
        ..LiveLaneProgress::default()
    };
    if authoritative_status.is_some() {
        progress.workflow_status = authoritative_status;
    }

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
            "StageFailed" => {
                progress.last_failure = merge_failure_detail(
                    progress.last_failure.take(),
                    extract_failure_text(&value),
                );
            }
            "WorkflowRunFailed" => {
                progress.workflow_status = Some(RunStatus::Failed);
                progress.last_failure = merge_failure_detail(
                    progress.last_failure.take(),
                    extract_failure_text(&value),
                );
                progress.current_stage_label = None;
            }
            _ => {}
        }
    }
    if authoritative_status.is_some() {
        progress.workflow_status = authoritative_status;
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

fn authoritative_failure_status(
    run_status: Option<&RunStatusRecord>,
    conclusion: Option<&Conclusion>,
) -> Option<RunStatus> {
    if matches!(
        run_status.map(|status| status.status),
        Some(RunStatus::Failed | RunStatus::Dead)
    ) {
        return run_status.map(|status| status.status);
    }
    if matches!(
        conclusion.map(|conclusion| conclusion.status.clone()),
        Some(fabro_workflows::outcome::StageStatus::Fail)
    ) {
        return Some(RunStatus::Failed);
    }
    None
}

fn worker_process_alive(run_root: &Path) -> Option<bool> {
    let pid = match std::fs::read_to_string(run_root.join("run.pid")) {
        Ok(pid) => pid,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Some(false),
        Err(_) => return None,
    };
    let pid = pid.trim();
    if pid.is_empty() {
        return Some(false);
    }
    #[cfg(target_os = "linux")]
    {
        return Some(Path::new("/proc").join(pid).exists());
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        Some(false)
    }
}

fn latest_cli_activity_at(run_root: &Path) -> Option<DateTime<Utc>> {
    let nodes_dir = run_root.join("nodes");
    let entries = std::fs::read_dir(nodes_dir).ok()?;
    let mut latest = None;
    for entry in entries.filter_map(Result::ok) {
        let node_dir = entry.path();
        if !node_dir.is_dir() {
            continue;
        }
        for file_name in ["cli_stdout.log", "cli_stderr.log"] {
            let path = node_dir.join(file_name);
            let Ok(metadata) = std::fs::metadata(path) else {
                continue;
            };
            let Ok(modified) = metadata.modified() else {
                continue;
            };
            let modified = DateTime::<Utc>::from(modified);
            latest = max_datetime(latest, Some(modified));
        }
    }
    latest
}

fn max_datetime(
    left: Option<DateTime<Utc>>,
    right: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    match (left, right) {
        (Some(left), Some(right)) => Some(std::cmp::max(left, right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn stale_active_progress(
    progress: &LiveLaneProgress,
    last_started_at: Option<DateTime<Utc>>,
) -> bool {
    let Some(status) = progress.workflow_status else {
        return false;
    };
    if !status.is_active() {
        return false;
    }
    if progress.worker_alive != Some(false) {
        return false;
    }
    let Some(started_at) = last_started_at.or(progress.updated_at) else {
        return false;
    };
    (Utc::now() - started_at).num_seconds() >= STALE_RUNNING_GRACE_SECS
}

fn live_lane_progress_from_inspection(inspection: &RunInspection) -> LiveLaneProgress {
    let state = inspection.state.as_ref();
    LiveLaneProgress {
        fabro_run_id: Some(inspection.run_id.clone()),
        workflow_status: Some(inspection.status),
        worker_alive: worker_process_alive(&inspection.run_dir),
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
        last_failure: merge_failure_detail(
            state.and_then(|state| state.last_failure.clone()),
            inspection
                .conclusion
                .as_ref()
                .and_then(|conclusion| conclusion.failure_reason.clone()),
        ),
        updated_at: state.map(|state| state.updated_at),
        finished_at: finished_at(inspection),
    }
}

fn extract_failure_text(value: &serde_json::Value) -> Option<String> {
    value
        .get("error")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .or_else(|| {
            value
                .get("failure_reason")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
        })
}

fn merge_failure_detail(current: Option<String>, candidate: Option<String>) -> Option<String> {
    match (current, candidate) {
        (Some(current), Some(candidate)) => {
            if (is_generic_failure(&current) || is_cycle_collapse_failure(&current))
                && !is_generic_failure(&candidate)
            {
                Some(candidate)
            } else {
                Some(current)
            }
        }
        (Some(current), None) => Some(current),
        (None, Some(candidate)) => Some(candidate),
        (None, None) => None,
    }
}

fn is_generic_failure(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed == "Script failed with exit code: 1"
        || (trimmed.starts_with("Script failed with exit code:") && !trimmed.contains('\n'))
        || trimmed == "fabro run failed"
}

fn is_cycle_collapse_failure(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("run is stuck in a cycle")
        || lower.contains("visited 3 times")
        || lower.contains("node limit 3")
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

    fn copy_dir(source: &Path, target: &Path) -> Result<(), std::io::Error> {
        for entry in walk(source)? {
            let relative = entry.strip_prefix(source).expect("prefix");
            let destination = target.join(relative);
            if entry.is_dir() {
                std::fs::create_dir_all(&destination)?;
                continue;
            }
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&entry, &destination)?;
        }
        Ok(())
    }

    fn walk(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut paths = Vec::new();
        visit(root, &mut paths)?;
        Ok(paths)
    }

    fn visit(root: &Path, paths: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
        paths.push(root.to_path_buf());
        if !root.is_dir() {
            return Ok(());
        }
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            visit(&entry.path(), paths)?;
        }
        Ok(())
    }
    use crate::manifest::ProgramManifest;

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
                failure_kind: None,
                recovery_action: None,
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

    #[test]
    fn refresh_program_state_syncs_child_program_lane_statuses() {
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../test/fixtures/raspberry-supervisor/portfolio-program.yaml");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let mut state = ProgramRuntimeState::new("portfolio-demo");
        state.lanes.insert(
            "ready:program".to_string(),
            LaneRuntimeRecord {
                lane_key: "ready:program".to_string(),
                status: LaneExecutionStatus::Running,
                run_config: Some(PathBuf::from("ready-program.yaml")),
                current_run_id: None,
                current_fabro_run_id: None,
                current_stage_label: Some("Implement".to_string()),
                last_run_id: None,
                last_started_at: Some(Utc::now()),
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
            },
        );
        state.lanes.insert(
            "complete:program".to_string(),
            LaneRuntimeRecord {
                lane_key: "complete:program".to_string(),
                status: LaneExecutionStatus::Running,
                run_config: Some(PathBuf::from("complete-program.yaml")),
                current_run_id: None,
                current_fabro_run_id: None,
                current_stage_label: Some("Review".to_string()),
                last_run_id: None,
                last_started_at: Some(Utc::now()),
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
            },
        );

        let changed =
            refresh_program_state(&manifest_path, &manifest, &mut state).expect("refresh works");

        assert!(changed);
        assert_eq!(
            state.lanes.get("ready:program").map(|record| record.status),
            Some(LaneExecutionStatus::Ready)
        );
        assert_eq!(
            state
                .lanes
                .get("complete:program")
                .map(|record| record.status),
            Some(LaneExecutionStatus::Complete)
        );
        assert!(state
            .lanes
            .get("ready:program")
            .and_then(|record| record.current_stage_label.as_ref())
            .is_none());
        assert!(state
            .lanes
            .get("complete:program")
            .and_then(|record| record.last_finished_at)
            .is_some());
    }

    #[test]
    fn refresh_program_state_prefers_child_runtime_state_over_artifact_completion() {
        let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../test/fixtures/raspberry-supervisor");
        let temp = tempfile::tempdir().expect("tempdir");
        copy_dir(&fixture_root, temp.path()).expect("copy fixture tree");

        let manifest_path = temp.path().join("portfolio-program.yaml");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        std::fs::write(
            temp.path().join(".raspberry/complete-program-state.json"),
            serde_json::json!({
                "schema_version": "raspberry.program.v2",
                "program": "complete-program",
                "updated_at": Utc::now(),
                "lanes": {
                    "docs:default": {
                        "lane_key": "docs:default",
                        "status": "ready",
                        "run_config": "run-configs/runtime-page.toml"
                    }
                }
            })
            .to_string(),
        )
        .expect("write child state");

        let mut state = ProgramRuntimeState::new("portfolio-demo");
        state.lanes.insert(
            "complete:program".to_string(),
            LaneRuntimeRecord {
                lane_key: "complete:program".to_string(),
                status: LaneExecutionStatus::Complete,
                run_config: Some(PathBuf::from("complete-program.yaml")),
                current_run_id: None,
                current_fabro_run_id: None,
                current_stage_label: None,
                last_run_id: None,
                last_started_at: Some(Utc::now()),
                last_finished_at: Some(Utc::now()),
                last_exit_status: Some(0),
                last_error: Some("merge failed".to_string()),
                failure_kind: Some(crate::failure::FailureKind::IntegrationConflict),
                recovery_action: Some(crate::failure::FailureRecoveryAction::RefreshFromTrunk),
                last_completed_stage_label: Some("Exit".to_string()),
                last_stage_duration_ms: Some(0),
                last_usage_summary: None,
                last_files_read: Vec::new(),
                last_files_written: Vec::new(),
                last_stdout_snippet: None,
                last_stderr_snippet: None,
            },
        );

        let changed =
            refresh_program_state(&manifest_path, &manifest, &mut state).expect("refresh works");

        assert!(changed);
        assert_eq!(
            state
                .lanes
                .get("complete:program")
                .map(|record| record.status),
            Some(LaneExecutionStatus::Ready)
        );
    }

    #[test]
    fn refresh_program_state_clears_current_fields_for_failed_run() {
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../test/fixtures/raspberry-supervisor/program.yaml");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let mut state = ProgramRuntimeState::new("raspberry-demo");
        state.lanes.insert(
            "consensus:chapter".to_string(),
            LaneRuntimeRecord {
                lane_key: "consensus:chapter".to_string(),
                status: LaneExecutionStatus::Running,
                run_config: Some(PathBuf::from("run-configs/consensus-chapter.toml")),
                current_run_id: Some("01CONSENSUSFAIL000000000000000".to_string()),
                current_fabro_run_id: Some("01CONSENSUSFAIL000000000000000".to_string()),
                current_stage_label: Some("Review".to_string()),
                last_run_id: Some("01CONSENSUSFAIL000000000000000".to_string()),
                last_started_at: Some(Utc::now()),
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
            },
        );

        let changed =
            refresh_program_state(&manifest_path, &manifest, &mut state).expect("refresh works");

        assert!(changed);
        let record = state
            .lanes
            .get("consensus:chapter")
            .expect("consensus record exists");
        assert_eq!(record.status, LaneExecutionStatus::Failed);
        assert!(record.current_run_id.is_none());
        assert!(record.current_fabro_run_id.is_none());
        assert!(record.current_stage_label.is_none());
    }

    #[test]
    fn refresh_program_state_propagates_child_running_runtime_details() {
        let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../test/fixtures/raspberry-supervisor");
        let temp = tempfile::tempdir().expect("tempdir");
        copy_dir(&fixture_root, temp.path()).expect("copy fixture tree");

        let manifest_path = temp.path().join("portfolio-program.yaml");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        std::fs::write(
            temp.path().join(".raspberry/ready-program-state.json"),
            serde_json::json!({
                "schema_version": "raspberry.program.v2",
                "program": "ready-program",
                "updated_at": Utc::now(),
                "lanes": {
                    "docs:default": {
                        "lane_key": "docs:default",
                        "status": "running",
                        "run_config": "run-configs/runtime-page.toml",
                        "current_run_id": "01KMCHILDRUN000000000000000",
                        "current_fabro_run_id": "01KMCHILDRUN000000000000000",
                        "current_stage_label": "Implement",
                        "last_run_id": "01KMCHILDRUN000000000000000",
                        "last_started_at": Utc::now(),
                        "last_completed_stage_label": "Preflight",
                        "last_stage_duration_ms": 1234,
                        "last_usage_summary": "anthropic: 10 in / 20 out"
                    }
                }
            })
            .to_string(),
        )
        .expect("write child state");

        let mut state = ProgramRuntimeState::new("portfolio-demo");
        state.lanes.insert(
            "ready:program".to_string(),
            LaneRuntimeRecord {
                lane_key: "ready:program".to_string(),
                status: LaneExecutionStatus::Failed,
                run_config: Some(PathBuf::from("ready-program.yaml")),
                current_run_id: Some("01KMSTALEFAIL0000000000000".to_string()),
                current_fabro_run_id: Some("01KMSTALEFAIL0000000000000".to_string()),
                current_stage_label: Some("Review".to_string()),
                last_run_id: Some("01KMSTALEFAIL0000000000000".to_string()),
                last_started_at: Some(Utc::now()),
                last_finished_at: Some(Utc::now()),
                last_exit_status: Some(1),
                last_error: Some("stale child failure".to_string()),
                failure_kind: None,
                recovery_action: None,
                last_completed_stage_label: None,
                last_stage_duration_ms: None,
                last_usage_summary: None,
                last_files_read: Vec::new(),
                last_files_written: Vec::new(),
                last_stdout_snippet: None,
                last_stderr_snippet: None,
            },
        );
        let changed =
            refresh_program_state(&manifest_path, &manifest, &mut state).expect("refresh works");

        assert!(changed);
        let record = state
            .lanes
            .get("ready:program")
            .expect("ready program record exists");
        assert_eq!(record.status, LaneExecutionStatus::Running);
        assert_eq!(
            record.current_run_id.as_deref(),
            Some("01KMCHILDRUN000000000000000")
        );
        assert_eq!(
            record.current_fabro_run_id.as_deref(),
            Some("01KMCHILDRUN000000000000000")
        );
        assert_eq!(record.current_stage_label.as_deref(), Some("Implement"));
        assert_eq!(
            record.last_completed_stage_label.as_deref(),
            Some("Preflight")
        );
        assert_eq!(record.last_stage_duration_ms, Some(1234));
        assert_eq!(
            record.last_usage_summary.as_deref(),
            Some("anthropic: 10 in / 20 out")
        );
        assert!(record.last_error.is_none());
        assert!(record.last_exit_status.is_none());
        assert!(record.last_stdout_snippet.is_none());
        assert!(record.last_stderr_snippet.is_none());
    }

    #[test]
    fn refresh_program_state_clears_stale_failure_residue_for_succeeded_run_progress() {
        let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../test/fixtures/raspberry-supervisor");
        let temp = tempfile::tempdir().expect("tempdir");
        copy_dir(&fixture_root, temp.path()).expect("copy fixture tree");

        let manifest_path = temp.path().join("program.yaml");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        std::fs::write(
            temp.path().join("runs/consensus-chapter/progress.jsonl"),
            concat!(
                "{\"ts\":\"2026-03-18T21:59:00.000Z\",\"run_id\":\"01CONSENSUSFAIL000000000000000\",",
                "\"event\":\"StageCompleted\",\"node_id\":\"brief\",\"node_label\":\"Brief\",",
                "\"duration_ms\":1800,\"usage\":{\"model\":\"gpt-5.4\",\"input_tokens\":700,",
                "\"output_tokens\":300},\"files_read\":[\"docs/source.md\"],",
                "\"files_written\":[\"brief.md\"]}\n",
                "{\"ts\":\"2026-03-18T22:00:00.000Z\",\"run_id\":\"01CONSENSUSFAIL000000000000000\",",
                "\"event\":\"WorkflowRunCompleted\",\"usage\":{\"model\":\"gpt-5.4\",",
                "\"input_tokens\":700,\"output_tokens\":300}}\n"
            ),
        )
        .expect("write progress");

        let mut state = ProgramRuntimeState::new("raspberry-demo");
        state.lanes.insert(
            "consensus:chapter".to_string(),
            LaneRuntimeRecord {
                lane_key: "consensus:chapter".to_string(),
                status: LaneExecutionStatus::Running,
                run_config: Some(PathBuf::from("run-configs/consensus-chapter.toml")),
                current_run_id: Some("01CONSENSUSFAIL000000000000000".to_string()),
                current_fabro_run_id: Some("01CONSENSUSFAIL000000000000000".to_string()),
                current_stage_label: Some("Review".to_string()),
                last_run_id: Some("01CONSENSUSFAIL000000000000000".to_string()),
                last_started_at: Some(Utc::now()),
                last_finished_at: None,
                last_exit_status: None,
                last_error: Some("LLM error: builder error".to_string()),
                failure_kind: Some(crate::failure::FailureKind::Unknown),
                recovery_action: Some(crate::failure::FailureRecoveryAction::SurfaceBlocked),
                last_completed_stage_label: Some("Brief".to_string()),
                last_stage_duration_ms: Some(1800),
                last_usage_summary: None,
                last_files_read: Vec::new(),
                last_files_written: Vec::new(),
                last_stdout_snippet: Some("stdout".to_string()),
                last_stderr_snippet: Some("stderr".to_string()),
            },
        );

        let changed =
            refresh_program_state(&manifest_path, &manifest, &mut state).expect("refresh works");

        assert!(changed);
        let record = state
            .lanes
            .get("consensus:chapter")
            .expect("consensus record exists");
        assert_eq!(record.status, LaneExecutionStatus::Ready);
        assert_eq!(record.last_exit_status, Some(0));
        assert!(record.last_error.is_none());
        assert!(record.failure_kind.is_none());
        assert!(record.recovery_action.is_none());
        assert!(record.last_stdout_snippet.is_none());
        assert!(record.last_stderr_snippet.is_none());
    }

    #[test]
    fn refresh_program_state_prefers_failed_status_record_over_succeeded_live_state() {
        let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../test/fixtures/raspberry-supervisor");
        let temp = tempfile::tempdir().expect("tempdir");
        copy_dir(&fixture_root, temp.path()).expect("copy fixture tree");

        let manifest_path = temp.path().join("program.yaml");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        std::fs::write(
            temp.path().join("runs/consensus-chapter/state.json"),
            serde_json::json!({
                "run_id": "01CONSENSUSFAIL000000000000000",
                "updated_at": "2026-03-18T22:00:00Z",
                "status": "succeeded",
                "reason": "completed",
                "last_completed_stage_id": "brief",
                "last_completed_stage_label": "Brief",
                "last_completed_stage_status": "success",
                "last_event": "WorkflowRunCompleted",
                "last_event_seq": 9,
                "completed_stage_count": 1
            })
            .to_string(),
        )
        .expect("write live state");
        std::fs::write(
            temp.path().join("runs/consensus-chapter/status.json"),
            serde_json::json!({
                "status": "failed",
                "reason": "workflow_error",
                "updated_at": "2026-03-18T22:00:00Z"
            })
            .to_string(),
        )
        .expect("write status");
        std::fs::write(
            temp.path().join("runs/consensus-chapter/conclusion.json"),
            serde_json::json!({
                "timestamp": "2026-03-18T22:00:00Z",
                "status": "fail",
                "duration_ms": 1234,
                "failure_reason": "git push failed: fatal: 'origin' does not appear to be a git repository\nfatal: Could not read from remote repository.",
                "stages": [],
                "total_retries": 0
            })
            .to_string(),
        )
        .expect("write conclusion");
        std::fs::write(
            temp.path().join("runs/consensus-chapter/progress.jsonl"),
            concat!(
                "{\"event\":\"WorkflowRunCompleted\",\"event_seq\":1}\n",
                "{\"event\":\"StageCompleted\",\"event_seq\":2,\"node_label\":\"Brief\",\"duration_ms\":15}\n"
            ),
        )
        .expect("write progress");

        let mut state = ProgramRuntimeState::new("raspberry-demo");
        state.lanes.insert(
            "consensus:chapter".to_string(),
            LaneRuntimeRecord {
                lane_key: "consensus:chapter".to_string(),
                status: LaneExecutionStatus::Running,
                run_config: Some(PathBuf::from("run-configs/consensus-chapter.toml")),
                current_run_id: Some("01CONSENSUSFAIL000000000000000".to_string()),
                current_fabro_run_id: Some("01CONSENSUSFAIL000000000000000".to_string()),
                current_stage_label: Some("Review".to_string()),
                last_run_id: Some("01CONSENSUSFAIL000000000000000".to_string()),
                last_started_at: Some(Utc::now()),
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
            },
        );

        let changed =
            refresh_program_state(&manifest_path, &manifest, &mut state).expect("refresh works");

        assert!(changed);
        let record = state
            .lanes
            .get("consensus:chapter")
            .expect("consensus record exists");
        assert_eq!(record.status, LaneExecutionStatus::Failed);
        assert_eq!(
            record.last_error.as_deref(),
            Some(
                "git push failed: fatal: 'origin' does not appear to be a git repository\nfatal: Could not read from remote repository."
            )
        );
        assert_eq!(
            record.failure_kind,
            Some(crate::failure::FailureKind::IntegrationTargetUnavailable)
        );
        assert_eq!(
            record.recovery_action,
            Some(crate::failure::FailureRecoveryAction::ReplayLane)
        );
    }

    #[test]
    fn sync_program_state_with_evaluated_clears_stale_failure_residue_for_ready_lane() {
        let mut state = ProgramRuntimeState::new("demo");
        state.lanes.insert(
            "foundations:foundations".to_string(),
            LaneRuntimeRecord {
                lane_key: "foundations:foundations".to_string(),
                status: LaneExecutionStatus::Failed,
                run_config: Some(PathBuf::from("run-configs/bootstrap/foundations.toml")),
                current_run_id: None,
                current_fabro_run_id: None,
                current_stage_label: None,
                last_run_id: Some("01FAILED".to_string()),
                last_started_at: Some(Utc::now()),
                last_finished_at: Some(Utc::now()),
                last_exit_status: Some(1),
                last_error: Some("merge failed".to_string()),
                failure_kind: Some(crate::failure::FailureKind::IntegrationConflict),
                recovery_action: Some(crate::failure::FailureRecoveryAction::RefreshFromTrunk),
                last_completed_stage_label: Some("Exit".to_string()),
                last_stage_duration_ms: Some(0),
                last_usage_summary: None,
                last_files_read: Vec::new(),
                last_files_written: Vec::new(),
                last_stdout_snippet: Some("stdout".to_string()),
                last_stderr_snippet: Some("stderr".to_string()),
            },
        );

        let program = EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            lanes: vec![crate::evaluate::EvaluatedLane {
                lane_key: "foundations:foundations".to_string(),
                unit_id: "foundations".to_string(),
                unit_title: "Foundations".to_string(),
                lane_id: "foundations".to_string(),
                lane_title: "Foundations Lane".to_string(),
                lane_kind: crate::manifest::LaneKind::Platform,
                status: LaneExecutionStatus::Ready,
                operational_state: None,
                precondition_state: None,
                proof_state: None,
                orchestration_state: None,
                detail: "dependencies satisfied".to_string(),
                managed_milestone: "reviewed".to_string(),
                proof_profile: None,
                run_config: PathBuf::from("run-configs/bootstrap/foundations.toml"),
                run_id: None,
                current_run_id: None,
                current_fabro_run_id: None,
                current_stage: None,
                last_run_id: Some("01READY".to_string()),
                last_started_at: Some(Utc::now()),
                last_finished_at: Some(Utc::now()),
                last_exit_status: Some(0),
                last_error: None,
                failure_kind: None,
                recovery_action: None,
                last_completed_stage_label: Some("Exit".to_string()),
                last_stage_duration_ms: Some(0),
                last_usage_summary: None,
                last_files_read: Vec::new(),
                last_files_written: Vec::new(),
                last_stdout_snippet: Some("stdout".to_string()),
                last_stderr_snippet: Some("stderr".to_string()),
                ready_checks_passing: Vec::new(),
                ready_checks_failing: Vec::new(),
                running_checks_passing: Vec::new(),
                running_checks_failing: Vec::new(),
            }],
        };

        let changed = sync_program_state_with_evaluated(&mut state, &program);

        assert!(changed);
        let record = state
            .lanes
            .get("foundations:foundations")
            .expect("lane record");
        assert_eq!(record.status, LaneExecutionStatus::Ready);
        assert!(record.last_error.is_none());
        assert!(record.failure_kind.is_none());
        assert!(record.recovery_action.is_none());
        assert!(record.last_stdout_snippet.is_none());
        assert!(record.last_stderr_snippet.is_none());
    }

    #[test]
    fn sync_program_state_with_evaluated_clears_stale_failure_residue_for_complete_lane() {
        let mut state = ProgramRuntimeState::new("demo");
        state.lanes.insert(
            "interface:lane".to_string(),
            LaneRuntimeRecord {
                lane_key: "interface:lane".to_string(),
                status: LaneExecutionStatus::Failed,
                run_config: Some(PathBuf::from("run-configs/interface.toml")),
                current_run_id: None,
                current_fabro_run_id: None,
                current_stage_label: None,
                last_run_id: Some("01FAILED".to_string()),
                last_started_at: Some(Utc::now()),
                last_finished_at: Some(Utc::now()),
                last_exit_status: Some(1),
                last_error: Some("provider rejected request".to_string()),
                failure_kind: Some(crate::failure::FailureKind::ProviderPolicyMismatch),
                recovery_action: Some(crate::failure::FailureRecoveryAction::RegenerateLane),
                last_completed_stage_label: Some("Exit".to_string()),
                last_stage_duration_ms: Some(0),
                last_usage_summary: None,
                last_files_read: Vec::new(),
                last_files_written: Vec::new(),
                last_stdout_snippet: Some("stdout".to_string()),
                last_stderr_snippet: Some("stderr".to_string()),
            },
        );

        let program = EvaluatedProgram {
            program: "demo".to_string(),
            max_parallel: 1,
            lanes: vec![crate::evaluate::EvaluatedLane {
                lane_key: "interface:lane".to_string(),
                unit_id: "interface".to_string(),
                unit_title: "Interface".to_string(),
                lane_id: "lane".to_string(),
                lane_title: "Interface Lane".to_string(),
                lane_kind: crate::manifest::LaneKind::Interface,
                status: LaneExecutionStatus::Complete,
                operational_state: None,
                precondition_state: None,
                proof_state: None,
                orchestration_state: None,
                detail: "managed milestone `reviewed` satisfied".to_string(),
                managed_milestone: "reviewed".to_string(),
                proof_profile: None,
                run_config: PathBuf::from("run-configs/interface.toml"),
                run_id: None,
                current_run_id: None,
                current_fabro_run_id: None,
                current_stage: None,
                last_run_id: Some("01COMPLETE".to_string()),
                last_started_at: Some(Utc::now()),
                last_finished_at: Some(Utc::now()),
                last_exit_status: Some(0),
                last_error: Some("provider rejected request".to_string()),
                failure_kind: Some(crate::failure::FailureKind::ProviderPolicyMismatch),
                recovery_action: Some(crate::failure::FailureRecoveryAction::RegenerateLane),
                last_completed_stage_label: Some("Exit".to_string()),
                last_stage_duration_ms: Some(0),
                last_usage_summary: None,
                last_files_read: Vec::new(),
                last_files_written: Vec::new(),
                last_stdout_snippet: Some("stdout".to_string()),
                last_stderr_snippet: Some("stderr".to_string()),
                ready_checks_passing: Vec::new(),
                ready_checks_failing: Vec::new(),
                running_checks_passing: Vec::new(),
                running_checks_failing: Vec::new(),
            }],
        };

        let changed = sync_program_state_with_evaluated(&mut state, &program);

        assert!(changed);
        let record = state.lanes.get("interface:lane").expect("lane record");
        assert_eq!(record.status, LaneExecutionStatus::Complete);
        assert!(record.last_error.is_none());
        assert!(record.failure_kind.is_none());
        assert!(record.recovery_action.is_none());
        assert!(record.last_stdout_snippet.is_none());
        assert!(record.last_stderr_snippet.is_none());
    }

    #[test]
    fn refresh_program_state_prunes_removed_lanes() {
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../test/fixtures/raspberry-supervisor/portfolio-program.yaml");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let mut state = ProgramRuntimeState::new("portfolio-demo");
        state.lanes.insert(
            "removed:lane".to_string(),
            LaneRuntimeRecord {
                lane_key: "removed:lane".to_string(),
                status: LaneExecutionStatus::Running,
                run_config: Some(PathBuf::from("removed.toml")),
                current_run_id: Some("01KMREMOVED0000000000000000".to_string()),
                current_fabro_run_id: Some("01KMREMOVED0000000000000000".to_string()),
                current_stage_label: Some("Specify".to_string()),
                last_run_id: Some("01KMREMOVED0000000000000000".to_string()),
                last_started_at: Some(Utc::now()),
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
            },
        );

        let changed =
            refresh_program_state(&manifest_path, &manifest, &mut state).expect("refresh works");

        assert!(changed);
        assert!(!state.lanes.contains_key("removed:lane"));
    }

    #[test]
    fn refresh_program_state_uses_child_state_without_running_child_checks() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("malinka/programs")).expect("program dir");
        std::fs::create_dir_all(temp.path().join("malinka/.raspberry")).expect("state dir");
        let marker_path = temp.path().join("child-check-ran.txt");
        let parent_manifest_path = temp.path().join("malinka/programs/parent.yaml");
        let child_manifest_path = temp.path().join("malinka/programs/child.yaml");

        std::fs::write(
            &parent_manifest_path,
            r#"
version: 1
program: parent
target_repo: ../..
state_path: ../../.raspberry/parent-state.json
units:
  - id: child
    title: Child
    output_root: outputs/child
    lanes:
      - id: program
        kind: orchestration
        title: Child Program
        run_config: ../run-configs/orchestrate/child.toml
        program_manifest: child.yaml
        managed_milestone: coordinated
"#,
        )
        .expect("parent manifest");
        std::fs::write(
            &child_manifest_path,
            format!(
                r#"
version: 1
program: child
target_repo: ../..
state_path: ../../.raspberry/child-state.json
units:
  - id: work
    title: Work
    output_root: outputs/work
    lanes:
      - id: lane
        kind: platform
        title: Work Lane
        run_config: ../run-configs/bootstrap/work.toml
        managed_milestone: reviewed
        checks:
          - label: should_not_run
            kind: precondition
            scope: ready
            type: command_succeeds
            command: \"python -c \\\"from pathlib import Path; Path(r'{}').write_text('ran')\\\"\"
"#,
                marker_path.display()
            ),
        )
        .expect("child manifest");
        std::fs::write(
            temp.path().join("malinka/.raspberry/child-state.json"),
            serde_json::json!({
                "schema_version": "raspberry.program.v2",
                "program": "child",
                "updated_at": Utc::now(),
                "lanes": {
                    "work:lane": {
                        "lane_key": "work:lane",
                        "status": "ready",
                        "run_config": "run-configs/bootstrap/work.toml"
                    }
                }
            })
            .to_string(),
        )
        .expect("child state");

        let manifest = ProgramManifest::load(&parent_manifest_path).expect("manifest loads");
        let mut state = ProgramRuntimeState::new("parent");

        refresh_program_state(&parent_manifest_path, &manifest, &mut state).expect("refresh");

        assert!(state.lanes.contains_key("child:program"));
        assert!(
            !marker_path.exists(),
            "child checks should not run during parent refresh"
        );
    }

    #[test]
    fn stale_active_progress_marks_run_as_stale() {
        let progress = LiveLaneProgress {
            workflow_status: Some(RunStatus::Submitted),
            worker_alive: Some(false),
            updated_at: Some(Utc::now()),
            ..LiveLaneProgress::default()
        };

        assert!(stale_active_progress(
            &progress,
            Some(Utc::now() - chrono::Duration::seconds(STALE_RUNNING_GRACE_SECS + 1)),
        ));
    }

    #[test]
    fn latest_cli_activity_at_prefers_recent_stage_logs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let run_root = temp.path();
        std::fs::create_dir_all(run_root.join("nodes/specify")).expect("nodes dir");
        std::fs::write(run_root.join("nodes/specify/cli_stdout.log"), "hello").expect("write log");

        let activity = latest_cli_activity_at(run_root).expect("activity timestamp");
        assert!(activity <= Utc::now());
    }

    #[test]
    fn ensure_lane_record_normalizes_bloated_run_config_paths() {
        let mut state = ProgramRuntimeState::new("demo");

        let record = ensure_lane_record(
            &mut state,
            "demo:lane",
            Path::new(
                "/home/r/coding/zend/malinka/programs/../../malinka/programs/../../malinka/run-configs/implement/demo.toml",
            ),
        );

        assert_eq!(
            record.run_config.as_deref(),
            Some(Path::new(
                "/home/r/coding/zend/malinka/run-configs/implement/demo.toml"
            ))
        );
    }

    #[test]
    fn merge_failure_detail_prefers_richer_stage_failure() {
        let merged = merge_failure_detail(
            Some("Script failed with exit code: 1".to_string()),
            Some("OSError: [Errno 98] Address already in use".to_string()),
        );

        assert_eq!(
            merged.as_deref(),
            Some("OSError: [Errno 98] Address already in use")
        );
    }

    #[test]
    fn merge_failure_detail_keeps_existing_specific_failure() {
        let merged = merge_failure_detail(
            Some("Engine error: deterministic failure cycle detected".to_string()),
            Some("Script failed with exit code: 1".to_string()),
        );

        assert_eq!(
            merged.as_deref(),
            Some("Engine error: deterministic failure cycle detected")
        );
    }

    #[test]
    fn merge_failure_detail_prefers_underlying_stage_failure_over_cycle_wrapper() {
        let merged = merge_failure_detail(
            Some("Engine error: node \"fixup\" visited 3 times (node limit 3); run is stuck in a cycle".to_string()),
            Some("OSError: [Errno 98] Address already in use".to_string()),
        );

        assert_eq!(
            merged.as_deref(),
            Some("OSError: [Errno 98] Address already in use")
        );
    }
}
