use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use fabro_workflows::live_state::RunLiveState;
use fabro_workflows::run_status::{RunStatus, RunStatusRecord};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::failure::{FailureKind, FailureRecoveryAction};
use crate::manifest::{
    LaneCheck, LaneCheckKind, LaneCheckProbe, LaneCheckScope, LaneDependency, LaneKind,
    LaneManifest, ProgramManifest,
};
use crate::program_state::{
    refresh_program_state, sync_program_state_with_evaluated, LaneRuntimeRecord,
    ProgramRuntimeState, ProgramStateError,
};

thread_local! {
    static EVALUATION_STACK: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
}

const CHECK_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
struct CommandProbeOutput {
    success: bool,
    stdout: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum LaneExecutionStatus {
    Blocked,
    Ready,
    Running,
    Complete,
    Failed,
}

impl fmt::Display for LaneExecutionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Blocked => "blocked",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Complete => "complete",
            Self::Failed => "failed",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatedProgram {
    pub program: String,
    pub max_parallel: usize,
    pub runtime_max_parallel: Option<usize>,
    pub lanes: Vec<EvaluatedLane>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatedLane {
    pub lane_key: String,
    pub unit_id: String,
    pub unit_title: String,
    pub lane_id: String,
    pub lane_title: String,
    pub lane_kind: LaneKind,
    pub status: LaneExecutionStatus,
    pub operational_state: Option<LaneOperationalState>,
    pub precondition_state: Option<DerivedCheckState>,
    pub proof_state: Option<DerivedCheckState>,
    pub orchestration_state: Option<LaneOrchestrationState>,
    pub detail: String,
    pub managed_milestone: String,
    pub proof_profile: Option<String>,
    pub run_config: PathBuf,
    pub run_id: Option<String>,
    pub current_run_id: Option<String>,
    pub current_fabro_run_id: Option<String>,
    pub current_stage: Option<String>,
    pub last_run_id: Option<String>,
    pub last_started_at: Option<DateTime<Utc>>,
    pub last_finished_at: Option<DateTime<Utc>>,
    pub last_exit_status: Option<i32>,
    pub last_error: Option<String>,
    pub failure_kind: Option<FailureKind>,
    pub recovery_action: Option<FailureRecoveryAction>,
    pub last_completed_stage_label: Option<String>,
    pub last_stage_duration_ms: Option<u64>,
    pub last_usage_summary: Option<String>,
    pub last_files_read: Vec<String>,
    pub last_files_written: Vec<String>,
    pub last_stdout_snippet: Option<String>,
    pub last_stderr_snippet: Option<String>,
    pub ready_checks_passing: Vec<String>,
    pub ready_checks_failing: Vec<String>,
    pub running_checks_passing: Vec<String>,
    pub running_checks_failing: Vec<String>,
    pub consecutive_failures: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneOperationalState {
    Healthy,
    Degraded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivedCheckState {
    Met,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneOrchestrationState {
    Ready,
    Waiting,
    Blocked,
}

impl fmt::Display for LaneOperationalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
        };
        f.write_str(value)
    }
}

impl fmt::Display for DerivedCheckState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Met => "met",
            Self::Failed => "failed",
        };
        f.write_str(value)
    }
}

impl fmt::Display for LaneOrchestrationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Ready => "ready",
            Self::Waiting => "waiting",
            Self::Blocked => "blocked",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Default)]
struct RunSnapshot {
    status: Option<RunStatus>,
    run_id: Option<String>,
    current_stage: Option<String>,
    last_failure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnitStatus {
    lifecycle: String,
    present_artifacts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChildProgramSummary {
    program: String,
    complete: usize,
    ready: usize,
    running: usize,
    blocked: usize,
    failed: usize,
    total: usize,
    running_details: Vec<String>,
    failed_details: Vec<String>,
}

#[derive(Debug, Error)]
pub enum EvaluateError {
    #[error(transparent)]
    Manifest(#[from] crate::manifest::ManifestError),
    #[error(transparent)]
    ProgramState(#[from] ProgramStateError),
}

pub fn evaluate_program(manifest_path: &Path) -> Result<EvaluatedProgram, EvaluateError> {
    evaluate_program_internal(manifest_path, true)
}

pub fn evaluate_program_local(manifest_path: &Path) -> Result<EvaluatedProgram, EvaluateError> {
    evaluate_program_internal(manifest_path, false)
}

pub(crate) fn evaluate_program_internal(
    manifest_path: &Path,
    propagate_parents: bool,
) -> Result<EvaluatedProgram, EvaluateError> {
    let manifest_path = normalize_path(manifest_path);
    if evaluation_stack_contains(&manifest_path) {
        let manifest = ProgramManifest::load(&manifest_path)?;
        let state_path = manifest.resolved_state_path(&manifest_path);
        let program_state = ProgramRuntimeState::load_optional(&state_path)?;
        return Ok(evaluate_with_state(
            &manifest_path,
            &manifest,
            program_state.as_ref(),
        ));
    }
    let _guard = enter_evaluation_scope(&manifest_path);
    let manifest = ProgramManifest::load(&manifest_path)?;
    let state_path = manifest.resolved_state_path(&manifest_path);
    let mut program_state = ProgramRuntimeState::load_optional(&state_path)?
        .unwrap_or_else(|| ProgramRuntimeState::new(&manifest.program));
    if refresh_program_state(&manifest_path, &manifest, &mut program_state)? {
        program_state.save(&state_path)?;
    }
    let mut program = evaluate_with_state(&manifest_path, &manifest, Some(&program_state));
    if sync_program_state_with_evaluated(&mut program_state, &program) {
        program_state.save(&state_path)?;
    }
    // Expose the autodev controller's runtime max_parallel for display purposes
    // only (e.g. `raspberry status`).  Do NOT override program.max_parallel — the
    // manifest is the source of truth for scheduling, not a stale report from a
    // previous controller session.
    if let Ok(Some(report)) =
        crate::autodev::load_optional_autodev_report(&manifest_path, &manifest)
    {
        if let Some(runtime_parallel) = report.current.and_then(|current| current.max_parallel) {
            program.runtime_max_parallel = Some(runtime_parallel);
        }
    }
    if propagate_parents {
        refresh_parent_programs(&manifest_path, &manifest)?;
    }
    Ok(program)
}

pub(crate) fn evaluation_stack_contains(manifest_path: &Path) -> bool {
    let manifest_path = normalize_path(manifest_path);
    EVALUATION_STACK.with(|stack| stack.borrow().iter().any(|path| path == &manifest_path))
}

fn enter_evaluation_scope(manifest_path: &Path) -> EvaluationScopeGuard {
    let manifest_path = normalize_path(manifest_path);
    EVALUATION_STACK.with(|stack| {
        stack.borrow_mut().push(manifest_path);
    });
    EvaluationScopeGuard
}

struct EvaluationScopeGuard;

impl Drop for EvaluationScopeGuard {
    fn drop(&mut self) {
        EVALUATION_STACK.with(|stack| {
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

pub(crate) fn refresh_parent_programs(
    manifest_path: &Path,
    manifest: &ProgramManifest,
) -> Result<(), EvaluateError> {
    let programs_dir = manifest
        .resolved_target_repo(manifest_path)
        .join("malinka")
        .join("programs");
    let Ok(entries) = std::fs::read_dir(&programs_dir) else {
        return Ok(());
    };
    let mut manifests = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
        .collect::<Vec<_>>();
    manifests.sort();

    for candidate in manifests {
        if same_manifest_path(&candidate, manifest_path) {
            continue;
        }
        let Ok(parent) = ProgramManifest::load(&candidate) else {
            continue;
        };
        if !references_child_program(&candidate, &parent, manifest_path) {
            continue;
        }
        let _ = evaluate_program_internal(&candidate, false)?;
    }

    Ok(())
}

pub fn evaluate_with_state(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    program_state: Option<&ProgramRuntimeState>,
) -> EvaluatedProgram {
    let runtime_records = runtime_record_map(program_state);
    let unit_statuses = build_unit_statuses(manifest_path, manifest);
    let satisfied = satisfied_milestones(manifest_path, manifest, &runtime_records, &unit_statuses);
    let mut command_probe_cache = HashMap::new();
    let mut lanes = Vec::new();

    for (unit_id, unit) in &manifest.units {
        for lane_id in unit.lanes.keys() {
            lanes.push(evaluate_lane(
                manifest_path,
                manifest,
                unit_id,
                lane_id,
                &satisfied,
                unit_statuses
                    .get(unit_id)
                    .expect("unit status should exist for every unit"),
                runtime_records.get(&lane_key(unit_id, lane_id)).copied(),
                &mut command_probe_cache,
            ));
        }
    }

    EvaluatedProgram {
        program: manifest.program.clone(),
        max_parallel: manifest.max_parallel,
        runtime_max_parallel: None,
        lanes,
    }
}

pub fn render_grouped_summary(program: &EvaluatedProgram) -> String {
    let mut grouped: BTreeMap<LaneExecutionStatus, Vec<&EvaluatedLane>> = BTreeMap::new();
    for lane in &program.lanes {
        grouped.entry(lane.status).or_default().push(lane);
    }

    let order = [
        LaneExecutionStatus::Complete,
        LaneExecutionStatus::Ready,
        LaneExecutionStatus::Running,
        LaneExecutionStatus::Blocked,
        LaneExecutionStatus::Failed,
    ];

    let mut lines = vec![
        format!("Program: {}", program.program),
        format!("Max parallel: {}", program.max_parallel),
    ];
    for status in order {
        let Some(lanes) = grouped.get(&status) else {
            continue;
        };
        lines.push(format!("{}:", title_case(status)));
        for lane in lanes {
            let mut line = format!(
                "  - {} [{}] — {}",
                lane.lane_key, lane.lane_kind, lane.detail
            );
            if let Some(proof_profile) = &lane.proof_profile {
                line.push_str(&format!(" | proof={proof_profile}"));
            }
            if let Some(precondition_state) = lane.precondition_state {
                line.push_str(&format!(" | preconditions={precondition_state}"));
            }
            if let Some(proof_state) = lane.proof_state {
                line.push_str(&format!(" | proof_state={proof_state}"));
            }
            if let Some(orchestration_state) = lane.orchestration_state {
                line.push_str(&format!(" | orchestration={orchestration_state}"));
            }
            if let Some(operational_state) = lane.operational_state {
                line.push_str(&format!(" | operational={operational_state}"));
            }
            if let Some(recovery_action) = lane.recovery_action {
                line.push_str(&format!(" | recovery={recovery_action}"));
            }
            if let Some((landing_state, _)) = trunk_delivery_state_for_lane(lane) {
                line.push_str(&format!(" | landing={landing_state}"));
            }
            lines.push(line);
        }
    }
    lines.join("\n")
}

pub fn render_status_table(program: &EvaluatedProgram) -> String {
    let mut counts = BTreeMap::new();
    for lane in &program.lanes {
        *counts.entry(lane.status).or_insert(0usize) += 1;
    }
    let mut lines = vec![
        format!("Program: {}", program.program),
        format!("Max parallel: {}", program.max_parallel),
        format!(
            "Counts: complete={} ready={} running={} blocked={} failed={}",
            counts
                .get(&LaneExecutionStatus::Complete)
                .copied()
                .unwrap_or(0),
            counts
                .get(&LaneExecutionStatus::Ready)
                .copied()
                .unwrap_or(0),
            counts
                .get(&LaneExecutionStatus::Running)
                .copied()
                .unwrap_or(0),
            counts
                .get(&LaneExecutionStatus::Blocked)
                .copied()
                .unwrap_or(0),
            counts
                .get(&LaneExecutionStatus::Failed)
                .copied()
                .unwrap_or(0),
        ),
    ];
    for lane in &program.lanes {
        let mut line = format!(
            "{} [{}|{}] {}",
            lane.lane_key, lane.status, lane.lane_kind, lane.detail
        );
        if let Some(proof_profile) = &lane.proof_profile {
            line.push_str(&format!(" | proof_profile={proof_profile}"));
        }
        if let Some(precondition_state) = lane.precondition_state {
            line.push_str(&format!(" | preconditions={precondition_state}"));
        }
        if let Some(proof_state) = lane.proof_state {
            line.push_str(&format!(" | proof_state={proof_state}"));
        }
        if let Some(orchestration_state) = lane.orchestration_state {
            line.push_str(&format!(" | orchestration={orchestration_state}"));
        }
        if let Some(operational_state) = lane.operational_state {
            line.push_str(&format!(" | operational={operational_state}"));
        }
        if let Some(recovery_action) = lane.recovery_action {
            line.push_str(&format!(" | recovery={recovery_action}"));
        }
        if let Some((landing_state, _)) = trunk_delivery_state_for_lane(lane) {
            line.push_str(&format!(" | landing={landing_state}"));
        }
        if let Some(stage) = &lane.current_stage {
            line.push_str(&format!(" | stage={stage}"));
        }
        if let Some(run_id) = &lane.current_run_id {
            if lane.current_fabro_run_id.as_deref() != Some(run_id.as_str()) {
                line.push_str(&format!(" | current_run_id={run_id}"));
            }
        }
        if let Some(run_id) = &lane.current_fabro_run_id {
            line.push_str(&format!(" | fabro_run_id={run_id}"));
        }
        if let Some(run_id) = &lane.run_id {
            line.push_str(&format!(" | run_id={run_id}"));
        }
        if let Some(run_id) = &lane.last_run_id {
            line.push_str(&format!(" | last_run_id={run_id}"));
        }
        if let Some(exit_status) = lane.last_exit_status {
            line.push_str(&format!(" | last_exit_status={exit_status}"));
        }
        if let Some(started_at) = lane.last_started_at {
            line.push_str(&format!(" | started_at={}", started_at.to_rfc3339()));
        }
        if let Some(finished_at) = lane.last_finished_at {
            line.push_str(&format!(" | finished_at={}", finished_at.to_rfc3339()));
        }
        if let Some(stage) = &lane.last_completed_stage_label {
            line.push_str(&format!(" | last_completed_stage={stage}"));
        }
        if let Some(duration_ms) = lane.last_stage_duration_ms {
            line.push_str(&format!(" | last_stage_duration_ms={duration_ms}"));
        }
        lines.push(line);
        if let Some(summary) = &lane.last_usage_summary {
            lines.push(format!("  usage: {summary}"));
        }
        if let Some((_, landing_detail)) = trunk_delivery_state_for_lane(lane) {
            lines.push(format!("  trunk_landing: {landing_detail}"));
        }
        if !lane.ready_checks_passing.is_empty() {
            lines.push(format!(
                "  ready_checks_passing: {}",
                lane.ready_checks_passing.join(", ")
            ));
        }
        if !lane.ready_checks_failing.is_empty() {
            lines.push(format!(
                "  ready_checks_failing: {}",
                lane.ready_checks_failing.join(", ")
            ));
        }
        if !lane.running_checks_passing.is_empty() {
            lines.push(format!(
                "  running_checks_passing: {}",
                lane.running_checks_passing.join(", ")
            ));
        }
        if !lane.running_checks_failing.is_empty() {
            lines.push(format!(
                "  running_checks_failing: {}",
                lane.running_checks_failing.join(", ")
            ));
        }
        if !lane.last_files_read.is_empty() {
            lines.push(format!("  files_read: {}", lane.last_files_read.join(", ")));
        }
        if !lane.last_files_written.is_empty() {
            lines.push(format!(
                "  files_written: {}",
                lane.last_files_written.join(", ")
            ));
        }
        if let Some(stdout) = &lane.last_stdout_snippet {
            lines.push(format!("  stdout: {stdout}"));
        }
        if let Some(stderr) = &lane.last_stderr_snippet {
            lines.push(format!("  stderr: {stderr}"));
        }
        if let Some(error) = &lane.last_error {
            lines.push(format!("  error: {error}"));
        }
    }
    lines.join("\n")
}

fn trunk_delivery_state_for_lane(lane: &EvaluatedLane) -> Option<(String, String)> {
    let run_id = lane
        .last_run_id
        .as_deref()
        .or(lane.current_run_id.as_deref())
        .or(lane.current_fabro_run_id.as_deref())?;
    let base = fabro_workflows::run_lookup::default_runs_base();
    let run_dir = fabro_workflows::run_lookup::find_run_by_prefix(&base, run_id).ok()?;
    let run_config = fabro_config::run::load_run_config(&run_dir.join("run.toml")).ok()?;
    if !run_config
        .integration
        .as_ref()
        .is_some_and(|config| config.enabled)
    {
        return None;
    }

    if let Ok(raw) = std::fs::read_to_string(run_dir.join("direct_integration.json")) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
            let target_branch = value
                .get("target_branch")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let pushed = value
                .get("pushed")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            if pushed {
                return Some((
                    "landed".to_string(),
                    format!("integrated to {target_branch}"),
                ));
            }
            return Some((
                "not_landed".to_string(),
                format!("integration recorded locally for {target_branch}"),
            ));
        }
    }

    if let Ok(raw) = std::fs::read_to_string(run_dir.join("progress.jsonl")) {
        let mut failed_pushes = Vec::new();
        for line in raw.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if value.get("event").and_then(|value| value.as_str()) != Some("GitPush") {
                continue;
            }
            if value.get("success").and_then(|value| value.as_bool()) == Some(false) {
                let branch = value
                    .get("branch")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown");
                failed_pushes.push(branch.to_string());
            }
        }
        if !failed_pushes.is_empty() {
            return Some((
                "push_failed".to_string(),
                format!("push failed for {}", failed_pushes.join(", ")),
            ));
        }
    }

    Some((
        "not_landed".to_string(),
        "integration enabled but no landed record was found".to_string(),
    ))
}

fn evaluate_lane(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    unit_id: &str,
    lane_id: &str,
    satisfied: &BTreeSet<String>,
    unit_status: &UnitStatus,
    runtime_record: Option<&LaneRuntimeRecord>,
    command_probe_cache: &mut HashMap<String, Option<CommandProbeOutput>>,
) -> EvaluatedLane {
    let unit = &manifest.units[unit_id];
    let lane = &unit.lanes[lane_id];
    let key = lane_key(unit_id, lane_id);
    let should_load_run_snapshot = runtime_record
        .map(|record| {
            record.current_run_id.is_some()
                || record.current_fabro_run_id.is_some()
                || record.status == LaneExecutionStatus::Running
        })
        .unwrap_or(false);
    let run_snapshot = if should_load_run_snapshot {
        manifest
            .resolve_lane_run_dir(manifest_path, unit_id, lane_id)
            .map(|run_dir| load_run_snapshot(&run_dir))
            .unwrap_or_default()
    } else {
        RunSnapshot::default()
    };
    let check_result = evaluate_lane_checks(manifest_path, manifest, lane, command_probe_cache);
    let orchestration_state = lane_orchestration_state(manifest_path, lane);
    let child_program = summarize_child_program(manifest_path, lane);

    let status = classify_lane(
        &key,
        lane,
        satisfied,
        unit_status,
        runtime_record,
        &run_snapshot,
        &check_result,
        orchestration_state,
        child_program.as_ref(),
    );
    let detail = lane_detail(
        &key,
        lane,
        satisfied,
        unit_status,
        runtime_record,
        &run_snapshot,
        &check_result,
        orchestration_state,
        status,
        child_program.as_ref(),
    );
    let operational_state =
        lane_operational_state(manifest_path, manifest, lane, status, &check_result);
    let precondition_state = derived_check_state(
        &check_result.ready_precondition_passing,
        &check_result.ready_precondition_failing,
    );
    let proof_state = lane_proof_state(manifest_path, manifest, lane, &check_result);

    EvaluatedLane {
        lane_key: key,
        unit_id: unit_id.to_string(),
        unit_title: unit.title.clone(),
        lane_id: lane_id.to_string(),
        lane_title: lane.title.clone(),
        lane_kind: lane.kind,
        status,
        operational_state,
        precondition_state,
        proof_state,
        orchestration_state,
        detail,
        managed_milestone: lane.managed_milestone.clone(),
        proof_profile: lane.proof_profile.clone(),
        run_config: manifest
            .resolve_lane_run_config(manifest_path, unit_id, lane_id)
            .unwrap_or_else(|| lane.run_config.clone()),
        run_id: runtime_record
            .and_then(|record| record.current_fabro_run_id.clone())
            .or(run_snapshot.run_id),
        current_run_id: runtime_record.and_then(|record| record.current_run_id.clone()),
        current_fabro_run_id: runtime_record.and_then(|record| record.current_fabro_run_id.clone()),
        current_stage: runtime_record
            .and_then(|record| record.current_stage_label.clone())
            .or(run_snapshot.current_stage),
        last_run_id: runtime_record.and_then(|record| record.last_run_id.clone()),
        last_started_at: runtime_record.and_then(|record| record.last_started_at),
        last_finished_at: runtime_record.and_then(|record| record.last_finished_at),
        last_exit_status: runtime_record.and_then(|record| record.last_exit_status),
        last_error: runtime_record.and_then(|record| record.last_error.clone()),
        failure_kind: runtime_record.and_then(|record| record.failure_kind),
        recovery_action: runtime_record.and_then(|record| record.recovery_action),
        last_completed_stage_label: runtime_record
            .and_then(|record| record.last_completed_stage_label.clone()),
        last_stage_duration_ms: runtime_record.and_then(|record| record.last_stage_duration_ms),
        last_usage_summary: runtime_record.and_then(|record| record.last_usage_summary.clone()),
        last_files_read: runtime_record
            .map(|record| record.last_files_read.clone())
            .unwrap_or_default(),
        last_files_written: runtime_record
            .map(|record| record.last_files_written.clone())
            .unwrap_or_default(),
        last_stdout_snippet: runtime_record.and_then(|record| record.last_stdout_snippet.clone()),
        last_stderr_snippet: runtime_record.and_then(|record| record.last_stderr_snippet.clone()),
        ready_checks_passing: check_result.ready_passing,
        ready_checks_failing: check_result.ready_failing,
        running_checks_passing: check_result.running_passing,
        running_checks_failing: check_result.running_failing,
        consecutive_failures: runtime_record
            .map(|record| record.consecutive_failures)
            .unwrap_or(0),
    }
}

fn satisfied_milestones(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    runtime_records: &BTreeMap<String, &LaneRuntimeRecord>,
    unit_statuses: &BTreeMap<String, UnitStatus>,
) -> BTreeSet<String> {
    let mut satisfied = BTreeSet::new();
    for (unit_id, unit) in &manifest.units {
        let unit_status = unit_statuses
            .get(unit_id)
            .expect("unit status should exist for every unit");
        for milestone in &unit.milestones {
            if lifecycle_reached(unit, unit_status, &milestone.id)
                || legacy_runtime_integrated(unit_id, unit, &milestone.id, runtime_records)
            {
                satisfied.insert(format!("{unit_id}@{}", milestone.id));
            }
        }
        for (lane_id, lane) in &unit.lanes {
            let lane_key = lane_key(unit_id, lane_id);
            let managed_complete = lane
                .program_manifest
                .as_ref()
                .and_then(|_| summarize_child_program(manifest_path, lane))
                .map(|summary| summary.complete == summary.total && summary.total > 0)
                .unwrap_or(false)
                || unit
                    .milestones
                    .iter()
                    .find(|milestone| milestone.id == lane.managed_milestone)
                    .map(|_| lifecycle_reached(unit, unit_status, &lane.managed_milestone))
                    .unwrap_or(false);
            if managed_complete {
                satisfied.insert(format!("{lane_key}@{}", lane.managed_milestone));
            }
        }
    }
    satisfied
}

fn legacy_runtime_integrated(
    unit_id: &str,
    unit: &crate::manifest::UnitManifest,
    milestone_id: &str,
    runtime_records: &BTreeMap<String, &LaneRuntimeRecord>,
) -> bool {
    if milestone_id != "integrated" {
        return false;
    }

    let integration_lanes = unit
        .lanes
        .iter()
        .filter(|(_, lane)| lane.produces.iter().any(|artifact| artifact == "integration"))
        .collect::<Vec<_>>();
    if integration_lanes.is_empty() {
        return false;
    }

    integration_lanes.into_iter().all(|(lane_id, _)| {
        let key = lane_key(unit_id, lane_id);
        runtime_records
            .get(&key)
            .is_some_and(|record| record.status == LaneExecutionStatus::Complete)
    })
}

fn build_unit_statuses(
    manifest_path: &Path,
    manifest: &ProgramManifest,
) -> BTreeMap<String, UnitStatus> {
    let target_repo = manifest.resolved_target_repo(manifest_path);
    manifest
        .units
        .iter()
        .map(|(unit_id, unit)| {
            (
                unit_id.clone(),
                evaluate_unit_status(manifest_path, &target_repo, unit),
            )
        })
        .collect()
}

fn evaluate_unit_status(
    manifest_path: &Path,
    target_repo: &Path,
    unit: &crate::manifest::UnitManifest,
) -> UnitStatus {
    let present_artifacts = unit
        .artifacts
        .iter()
        .filter_map(|(artifact_id, path)| {
            let absolute = artifact_path(manifest_path, unit, path);
            if is_ignored_controller_artifact(target_repo, &absolute) || !absolute.is_file() {
                return None;
            }
            Some(artifact_id.clone())
        })
        .collect::<Vec<_>>();
    let present_set = present_artifacts.iter().cloned().collect::<BTreeSet<_>>();

    let lifecycle = if unit.milestones.is_empty() {
        if present_artifacts.is_empty() {
            "not_started".to_string()
        } else {
            "complete".to_string()
        }
    } else {
        let mut reached = None;
        for milestone in &unit.milestones {
            if milestone
                .requires
                .iter()
                .all(|artifact| present_set.contains(artifact))
            {
                reached = Some(milestone.id.clone());
            } else {
                break;
            }
        }
        reached.unwrap_or_else(|| {
            if present_artifacts.is_empty() {
                "not_started".to_string()
            } else {
                "partial".to_string()
            }
        })
    };

    UnitStatus {
        lifecycle,
        present_artifacts,
    }
}

fn is_ignored_controller_artifact(target_repo: &Path, artifact: &Path) -> bool {
    let ignored_root = crate::autodev::autodev_cargo_target_dir(target_repo);
    artifact.starts_with(&ignored_root)
}

fn lifecycle_reached(
    unit: &crate::manifest::UnitManifest,
    status: &UnitStatus,
    target: &str,
) -> bool {
    let Some(current_index) = unit
        .milestones
        .iter()
        .position(|milestone| milestone.id == status.lifecycle)
    else {
        return false;
    };
    let Some(target_index) = unit
        .milestones
        .iter()
        .position(|milestone| milestone.id == target)
    else {
        return false;
    };
    current_index >= target_index
}

fn artifact_path(
    manifest_path: &Path,
    unit: &crate::manifest::UnitManifest,
    path: &Path,
) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    if let Some(output_root) = unit.output_root.as_ref() {
        let base = if output_root.is_absolute() {
            output_root.clone()
        } else {
            manifest_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(output_root)
        };
        return base.join(path);
    }
    manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(path)
}

fn runtime_record_map(state: Option<&ProgramRuntimeState>) -> BTreeMap<String, &LaneRuntimeRecord> {
    state
        .map(|state| state.lanes.iter().map(|(k, v)| (k.clone(), v)).collect())
        .unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
fn classify_lane(
    lane_key: &str,
    lane: &crate::manifest::LaneManifest,
    satisfied: &BTreeSet<String>,
    _unit_status: &UnitStatus,
    runtime_record: Option<&LaneRuntimeRecord>,
    run_snapshot: &RunSnapshot,
    check_result: &LaneCheckResult,
    orchestration_state: Option<LaneOrchestrationState>,
    child_program: Option<&ChildProgramSummary>,
) -> LaneExecutionStatus {
    if is_active(run_snapshot, runtime_record) {
        return LaneExecutionStatus::Running;
    }
    if lane_requires_child_program_manifest(lane) && child_program.is_none() {
        return LaneExecutionStatus::Blocked;
    }
    if is_failed(run_snapshot, runtime_record) {
        return LaneExecutionStatus::Failed;
    }
    if let Some(child_program) = child_program {
        return classify_child_program_lane(
            lane,
            satisfied,
            check_result,
            orchestration_state,
            child_program,
        );
    }
    if !check_result.ready_precondition_failing.is_empty()
        || !check_result.ready_proof_failing.is_empty()
    {
        return LaneExecutionStatus::Blocked;
    }
    if let Some(LaneOrchestrationState::Waiting | LaneOrchestrationState::Blocked) =
        orchestration_state
    {
        return LaneExecutionStatus::Blocked;
    }
    if satisfied.contains(&managed_milestone_key(lane_key, &lane.managed_milestone)) {
        return LaneExecutionStatus::Complete;
    }
    if dependencies_satisfied(&lane.dependencies, satisfied) {
        return LaneExecutionStatus::Ready;
    }
    LaneExecutionStatus::Blocked
}

fn classify_child_program_lane(
    lane: &crate::manifest::LaneManifest,
    satisfied: &BTreeSet<String>,
    check_result: &LaneCheckResult,
    orchestration_state: Option<LaneOrchestrationState>,
    child_program: &ChildProgramSummary,
) -> LaneExecutionStatus {
    if child_program.failed > 0 {
        return LaneExecutionStatus::Failed;
    }
    if child_program.running > 0 {
        return LaneExecutionStatus::Running;
    }
    if child_program.complete == child_program.total && child_program.total > 0 {
        return LaneExecutionStatus::Complete;
    }
    if !check_result.ready_precondition_failing.is_empty()
        || !check_result.ready_proof_failing.is_empty()
    {
        return LaneExecutionStatus::Blocked;
    }
    if let Some(LaneOrchestrationState::Waiting | LaneOrchestrationState::Blocked) =
        orchestration_state
    {
        return LaneExecutionStatus::Blocked;
    }
    if child_program.ready > 0 && dependencies_satisfied(&lane.dependencies, satisfied) {
        return LaneExecutionStatus::Ready;
    }
    LaneExecutionStatus::Blocked
}

fn is_active(run_snapshot: &RunSnapshot, runtime_record: Option<&LaneRuntimeRecord>) -> bool {
    run_snapshot
        .status
        .map(RunStatus::is_active)
        .unwrap_or(false)
        || runtime_record
            .map(|record| {
                record.status == LaneExecutionStatus::Running && record.last_finished_at.is_none()
            })
            .unwrap_or(false)
}

fn is_failed(run_snapshot: &RunSnapshot, runtime_record: Option<&LaneRuntimeRecord>) -> bool {
    matches!(
        run_snapshot.status,
        Some(RunStatus::Failed | RunStatus::Dead)
    ) || runtime_record
        .map(|record| record.status == LaneExecutionStatus::Failed)
        .unwrap_or(false)
}

fn dependencies_satisfied(dependencies: &[LaneDependency], satisfied: &BTreeSet<String>) -> bool {
    dependencies.iter().all(|dependency| {
        let key = dependency_key(dependency);
        satisfied.contains(&key)
    })
}

#[allow(clippy::too_many_arguments)]
fn lane_detail(
    lane_key: &str,
    lane: &crate::manifest::LaneManifest,
    satisfied: &BTreeSet<String>,
    unit_status: &UnitStatus,
    runtime_record: Option<&LaneRuntimeRecord>,
    run_snapshot: &RunSnapshot,
    check_result: &LaneCheckResult,
    orchestration_state: Option<LaneOrchestrationState>,
    status: LaneExecutionStatus,
    child_program: Option<&ChildProgramSummary>,
) -> String {
    if let Some(child_program) = child_program {
        return child_program_detail(
            lane_key,
            lane,
            satisfied,
            check_result,
            orchestration_state,
            status,
            child_program,
        );
    }
    match status {
        LaneExecutionStatus::Complete => {
            format!("managed milestone `{}` satisfied", lane.managed_milestone)
        }
        LaneExecutionStatus::Ready => {
            if !check_result.ready_proof_passing.is_empty()
                && !check_result.ready_precondition_passing.is_empty()
            {
                format!(
                    "dependencies, preconditions, and proof checks satisfied ({})",
                    check_result.ready_passing.join(", ")
                )
            } else if !check_result.ready_proof_passing.is_empty() {
                format!(
                    "dependencies and proof checks satisfied ({})",
                    check_result.ready_proof_passing.join(", ")
                )
            } else if !check_result.ready_precondition_passing.is_empty() {
                format!(
                    "dependencies and preconditions satisfied ({})",
                    check_result.ready_precondition_passing.join(", ")
                )
            } else if !check_result.ready_passing.is_empty() {
                format!(
                    "dependencies and checks satisfied ({})",
                    check_result.ready_passing.join(", ")
                )
            } else {
                "dependencies satisfied".to_string()
            }
        }
        LaneExecutionStatus::Running => {
            let active_stage = runtime_record
                .and_then(|record| record.current_stage_label.as_deref())
                .or(run_snapshot.current_stage.as_deref())
                .unwrap_or("unknown");
            if !check_result.running_failing.is_empty() {
                return format!(
                    "run active at stage `{}`; failing checks: {}",
                    active_stage,
                    check_result.running_failing.join(", ")
                );
            }
            if !check_result.running_passing.is_empty() {
                return format!(
                    "run active at stage `{}`; checks passing: {}",
                    active_stage,
                    check_result.running_passing.join(", ")
                );
            }
            if run_snapshot.current_stage.is_none() && produced_present(lane, unit_status) {
                return "produced artifacts present before managed milestone".to_string();
            }
            let stage = active_stage;
            format!("run active at stage `{stage}`")
        }
        LaneExecutionStatus::Failed => run_snapshot
            .last_failure
            .clone()
            .or_else(|| {
                runtime_record
                    .and_then(|record| record.failure_kind)
                    .map(|kind| format!("failure_kind={kind}"))
            })
            .or_else(|| {
                runtime_record
                    .and_then(|record| record.last_stderr_snippet.clone())
                    .filter(|text| !text.trim().is_empty())
            })
            .map(|summary| {
                let recovery = runtime_record
                    .and_then(|record| record.recovery_action)
                    .map(|action| format!(" | next_action={action}"))
                    .unwrap_or_default();
                format!("{summary}{recovery}")
            })
            .unwrap_or_else(|| "most recent run failed".to_string()),
        LaneExecutionStatus::Blocked => {
            blocked_detail(lane_key, lane, satisfied, check_result, orchestration_state)
        }
    }
}

fn child_program_detail(
    lane_key: &str,
    lane: &crate::manifest::LaneManifest,
    satisfied: &BTreeSet<String>,
    check_result: &LaneCheckResult,
    orchestration_state: Option<LaneOrchestrationState>,
    status: LaneExecutionStatus,
    child_program: &ChildProgramSummary,
) -> String {
    let mut summary = format!(
        "child program `{}`: complete={} ready={} running={} blocked={} failed={}",
        child_program.program,
        child_program.complete,
        child_program.ready,
        child_program.running,
        child_program.blocked,
        child_program.failed
    );
    if !child_program.running_details.is_empty() {
        summary.push_str(&format!(
            " | running_lanes={}",
            child_program.running_details.join(", ")
        ));
    }
    if !child_program.failed_details.is_empty() {
        summary.push_str(&format!(
            " | failed_lanes={}",
            child_program.failed_details.join(", ")
        ));
    }
    match status {
        LaneExecutionStatus::Complete => format!("{summary} | all child lanes complete"),
        LaneExecutionStatus::Ready => format!("{summary} | ready child work available"),
        LaneExecutionStatus::Running => format!("{summary} | child work in flight"),
        LaneExecutionStatus::Failed => format!("{summary} | child program has failures"),
        LaneExecutionStatus::Blocked => {
            if child_program.ready > 0 {
                format!(
                    "{summary} | waiting on {}",
                    blocked_detail(lane_key, lane, satisfied, check_result, orchestration_state,)
                )
            } else {
                format!("{summary} | no child lanes are currently ready")
            }
        }
    }
}

fn blocked_detail(
    lane_key: &str,
    lane: &crate::manifest::LaneManifest,
    satisfied: &BTreeSet<String>,
    check_result: &LaneCheckResult,
    orchestration_state: Option<LaneOrchestrationState>,
) -> String {
    if lane_requires_child_program_manifest(lane) && lane.program_manifest.is_none() {
        return "orchestration lane missing child program manifest".to_string();
    }
    if let Some(orchestration_state) = orchestration_state {
        match orchestration_state {
            LaneOrchestrationState::Waiting => {
                return "waiting on orchestration state".to_string();
            }
            LaneOrchestrationState::Blocked => {
                return "blocked by orchestration state".to_string();
            }
            LaneOrchestrationState::Ready => {}
        }
    }
    if !check_result.ready_proof_failing.is_empty() {
        return format!(
            "waiting on proof checks: {}",
            check_result.ready_proof_failing.join(", ")
        );
    }
    if !check_result.ready_precondition_failing.is_empty() {
        return format!(
            "waiting on preconditions: {}",
            check_result.ready_precondition_failing.join(", ")
        );
    }
    if !check_result.ready_failing.is_empty() {
        return format!(
            "waiting on checks: {}",
            check_result.ready_failing.join(", ")
        );
    }
    lane.dependencies
        .iter()
        .find(|dependency| !satisfied.contains(&dependency_key(dependency)))
        .map(dependency_display_key)
        .unwrap_or_else(|| format!("lane `{lane_key}` is blocked"))
}

fn lane_requires_child_program_manifest(lane: &crate::manifest::LaneManifest) -> bool {
    if lane.kind == LaneKind::Orchestration {
        return true;
    }
    lane.program_manifest.is_none()
        && lane
            .run_config
            .components()
            .any(|component| component.as_os_str() == "orchestration")
}

fn summarize_child_program(
    manifest_path: &Path,
    lane: &LaneManifest,
) -> Option<ChildProgramSummary> {
    let program_manifest = lane.program_manifest.as_ref()?;
    let program_manifest = resolve_check_path(manifest_path, program_manifest);
    if program_manifest == manifest_path {
        return None;
    }
    if let Some(summary) = summarize_child_program_from_state(&program_manifest) {
        return Some(summary);
    }
    if evaluation_stack_contains(&program_manifest) {
        return None;
    }
    let program = evaluate_program_internal(&program_manifest, false).ok()?;
    let mut summary = ChildProgramSummary {
        program: program.program,
        complete: 0,
        ready: 0,
        running: 0,
        blocked: 0,
        failed: 0,
        total: program.lanes.len(),
        running_details: Vec::new(),
        failed_details: Vec::new(),
    };
    for lane in program.lanes {
        match lane.status {
            LaneExecutionStatus::Complete => summary.complete += 1,
            LaneExecutionStatus::Ready => summary.ready += 1,
            LaneExecutionStatus::Running => {
                summary.running += 1;
                let detail = lane
                    .current_stage
                    .as_deref()
                    .map(|stage| format!("{}@{}", lane.lane_key, stage))
                    .unwrap_or(lane.lane_key);
                summary.running_details.push(detail);
            }
            LaneExecutionStatus::Blocked => summary.blocked += 1,
            LaneExecutionStatus::Failed => {
                summary.failed += 1;
                summary.failed_details.push(lane.lane_key);
            }
        }
    }
    Some(summary)
}

fn references_child_program(
    parent_manifest_path: &Path,
    parent: &ProgramManifest,
    child_manifest_path: &Path,
) -> bool {
    parent.units.values().any(|unit| {
        unit.lanes.values().any(|lane| {
            lane.program_manifest.as_ref().is_some_and(|path| {
                same_manifest_path(
                    &resolve_check_path(parent_manifest_path, path),
                    child_manifest_path,
                )
            })
        })
    })
}

fn same_manifest_path(left: &Path, right: &Path) -> bool {
    let left = std::fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = std::fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left == right
}

fn summarize_child_program_from_state(program_manifest: &Path) -> Option<ChildProgramSummary> {
    let manifest = ProgramManifest::load(program_manifest).ok()?;
    let state_path = manifest.resolved_state_path(program_manifest);
    let state = ProgramRuntimeState::load_optional(&state_path).ok()??;
    let mut summary = ChildProgramSummary {
        program: manifest.program,
        complete: 0,
        ready: 0,
        running: 0,
        blocked: 0,
        failed: 0,
        total: state.lanes.len(),
        running_details: Vec::new(),
        failed_details: Vec::new(),
    };
    for lane in state.lanes.values() {
        match lane.status {
            LaneExecutionStatus::Complete => summary.complete += 1,
            LaneExecutionStatus::Ready => summary.ready += 1,
            LaneExecutionStatus::Running => {
                summary.running += 1;
                let detail = lane
                    .current_stage_label
                    .as_deref()
                    .map(|stage| format!("{}@{}", lane.lane_key, stage))
                    .unwrap_or_else(|| lane.lane_key.clone());
                summary.running_details.push(detail);
            }
            LaneExecutionStatus::Blocked => summary.blocked += 1,
            LaneExecutionStatus::Failed => {
                summary.failed += 1;
                summary.failed_details.push(lane.lane_key.clone());
            }
        }
    }
    Some(summary)
}

fn load_run_snapshot(run_dir: &Path) -> RunSnapshot {
    let status = RunStatusRecord::load(&run_dir.join("status.json"))
        .ok()
        .map(|record| record.status);
    let live_state = RunLiveState::load(&run_dir.join("state.json")).ok();
    RunSnapshot {
        status,
        run_id: live_state.as_ref().map(|state| state.run_id.clone()),
        current_stage: live_state
            .as_ref()
            .and_then(|state| state.current_stage_label.clone()),
        last_failure: live_state.and_then(|state| state.last_failure),
    }
}

fn lane_key(unit_id: &str, lane_id: &str) -> String {
    format!("{unit_id}:{lane_id}")
}

fn dependency_key(dependency: &LaneDependency) -> String {
    let base = match dependency.lane.as_deref() {
        Some(lane) => lane_key(&dependency.unit, lane),
        None => dependency.unit.clone(),
    };
    match dependency.milestone.as_deref() {
        Some(milestone) => format!("{base}@{milestone}"),
        None => base,
    }
}

fn managed_milestone_key(lane_key: &str, milestone: &str) -> String {
    format!("{lane_key}@{milestone}")
}

fn dependency_display_key(dependency: &LaneDependency) -> String {
    dependency_key(dependency)
}

fn produced_present(lane: &crate::manifest::LaneManifest, unit_status: &UnitStatus) -> bool {
    lane.produces.iter().any(|artifact| {
        unit_status
            .present_artifacts
            .iter()
            .any(|present| present == artifact)
    })
}

#[derive(Debug, Clone, Default)]
struct LaneCheckResult {
    ready_passing: Vec<String>,
    ready_failing: Vec<String>,
    running_passing: Vec<String>,
    running_failing: Vec<String>,
    ready_precondition_passing: Vec<String>,
    ready_precondition_failing: Vec<String>,
    ready_proof_passing: Vec<String>,
    ready_proof_failing: Vec<String>,
    running_health_passing: Vec<String>,
    running_health_failing: Vec<String>,
}

fn evaluate_lane_checks(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    lane: &crate::manifest::LaneManifest,
    command_probe_cache: &mut HashMap<String, Option<CommandProbeOutput>>,
) -> LaneCheckResult {
    let mut result = LaneCheckResult::default();
    for check in &lane.checks {
        let passed = evaluate_check(manifest_path, manifest, check, command_probe_cache);
        match (check.scope, passed) {
            (LaneCheckScope::Ready, true) => {
                result.ready_passing.push(check.label.clone());
                match check.kind {
                    LaneCheckKind::Precondition => {
                        result.ready_precondition_passing.push(check.label.clone())
                    }
                    LaneCheckKind::Proof => result.ready_proof_passing.push(check.label.clone()),
                    LaneCheckKind::Health => {}
                }
            }
            (LaneCheckScope::Ready, false) => {
                result.ready_failing.push(check.label.clone());
                match check.kind {
                    LaneCheckKind::Precondition => {
                        result.ready_precondition_failing.push(check.label.clone())
                    }
                    LaneCheckKind::Proof => result.ready_proof_failing.push(check.label.clone()),
                    LaneCheckKind::Health => {}
                }
            }
            (LaneCheckScope::Running, true) => {
                result.running_passing.push(check.label.clone());
                if check.kind == LaneCheckKind::Health {
                    result.running_health_passing.push(check.label.clone());
                }
            }
            (LaneCheckScope::Running, false) => {
                result.running_failing.push(check.label.clone());
                if check.kind == LaneCheckKind::Health {
                    result.running_health_failing.push(check.label.clone());
                }
            }
        }
    }
    result
}

fn derived_check_state(passing: &[String], failing: &[String]) -> Option<DerivedCheckState> {
    if !failing.is_empty() {
        return Some(DerivedCheckState::Failed);
    }
    if !passing.is_empty() {
        return Some(DerivedCheckState::Met);
    }
    None
}

fn lane_operational_state(
    manifest_path: &Path,
    _manifest: &ProgramManifest,
    lane: &crate::manifest::LaneManifest,
    status: LaneExecutionStatus,
    check_result: &LaneCheckResult,
) -> Option<LaneOperationalState> {
    if lane.kind != LaneKind::Service || status != LaneExecutionStatus::Running {
        return None;
    }
    if let Some(path) = &lane.service_state_path {
        return read_service_state(manifest_path, path);
    }
    if !check_result.running_failing.is_empty() {
        return Some(LaneOperationalState::Degraded);
    }
    if !check_result.running_passing.is_empty() {
        return Some(LaneOperationalState::Healthy);
    }
    None
}

fn lane_proof_state(
    manifest_path: &Path,
    _manifest: &ProgramManifest,
    lane: &crate::manifest::LaneManifest,
    check_result: &LaneCheckResult,
) -> Option<DerivedCheckState> {
    if let Some(path) = &lane.proof_state_path {
        return read_proof_state(manifest_path, path);
    }
    derived_check_state(
        &check_result.ready_proof_passing,
        &check_result.ready_proof_failing,
    )
}

fn lane_orchestration_state(
    manifest_path: &Path,
    lane: &crate::manifest::LaneManifest,
) -> Option<LaneOrchestrationState> {
    if lane.kind != LaneKind::Orchestration {
        return None;
    }
    let Some(path) = &lane.orchestration_state_path else {
        return None;
    };
    let value = read_state_contract(manifest_path, path)?;
    match value.get("status")?.as_str()? {
        "ready" => Some(LaneOrchestrationState::Ready),
        "waiting" => Some(LaneOrchestrationState::Waiting),
        "blocked" => Some(LaneOrchestrationState::Blocked),
        _ => None,
    }
}

fn read_proof_state(manifest_path: &Path, path: &Path) -> Option<DerivedCheckState> {
    let value = read_state_contract(manifest_path, path)?;
    match value.get("status")?.as_str()? {
        "passed" | "met" => Some(DerivedCheckState::Met),
        "failed" => Some(DerivedCheckState::Failed),
        _ => None,
    }
}

fn read_service_state(manifest_path: &Path, path: &Path) -> Option<LaneOperationalState> {
    let value = read_state_contract(manifest_path, path)?;
    match value.get("status")?.as_str()? {
        "healthy" | "ok" => Some(LaneOperationalState::Healthy),
        "degraded" | "failed" => Some(LaneOperationalState::Degraded),
        _ => None,
    }
}

fn read_state_contract(manifest_path: &Path, path: &Path) -> Option<serde_json::Value> {
    let path = resolve_check_path(manifest_path, path);
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn evaluate_check(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    check: &LaneCheck,
    command_probe_cache: &mut HashMap<String, Option<CommandProbeOutput>>,
) -> bool {
    evaluate_probe(manifest_path, manifest, &check.probe, command_probe_cache)
}

fn evaluate_probe(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    probe: &LaneCheckProbe,
    command_probe_cache: &mut HashMap<String, Option<CommandProbeOutput>>,
) -> bool {
    match probe {
        LaneCheckProbe::FileExists { path } => resolve_check_path(manifest_path, path).is_file(),
        LaneCheckProbe::JsonFieldEquals {
            path,
            field,
            equals,
        } => {
            let path = resolve_check_path(manifest_path, path);
            let Ok(raw) = std::fs::read_to_string(path) else {
                return false;
            };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
                return false;
            };
            let Some(found) = json_lookup(&value, field) else {
                return false;
            };
            found == equals
        }
        LaneCheckProbe::CommandSucceeds { command } => {
            run_check_command_cached(manifest_path, manifest, command, command_probe_cache)
                .map(|output| output.success)
                .unwrap_or(false)
        }
        LaneCheckProbe::CommandStdoutContains { command, contains } => {
            run_check_command_cached(manifest_path, manifest, command, command_probe_cache)
                .map(|output| output.success && output.stdout.contains(contains))
                .unwrap_or(false)
        }
    }
}

fn run_check_command_uncached(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    command: &str,
) -> Option<std::process::Output> {
    let cwd = manifest.resolved_target_repo(manifest_path);
    let mut child = Command::new("bash")
        .arg("-lc")
        .arg(command)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + CHECK_COMMAND_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => return child.wait_with_output().ok(),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait_with_output();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}

fn run_check_command_cached(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    command: &str,
    command_probe_cache: &mut HashMap<String, Option<CommandProbeOutput>>,
) -> Option<CommandProbeOutput> {
    if let Some(cached) = command_probe_cache.get(command) {
        return cached.clone();
    }
    let output = run_check_command_uncached(manifest_path, manifest, command).map(|output| {
        CommandProbeOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        }
    });
    command_probe_cache.insert(command.to_string(), output.clone());
    output
}

fn resolve_check_path(manifest_path: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    }
}

fn json_lookup<'a>(value: &'a serde_json::Value, field: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for part in field.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

fn title_case(status: LaneExecutionStatus) -> &'static str {
    match status {
        LaneExecutionStatus::Blocked => "Blocked",
        LaneExecutionStatus::Ready => "Ready",
        LaneExecutionStatus::Running => "Running",
        LaneExecutionStatus::Complete => "Complete",
        LaneExecutionStatus::Failed => "Failed",
    }
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

    fn fixture_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../test/fixtures/raspberry-supervisor/program.yaml")
    }

    fn myosu_fixture_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml")
    }

    fn portfolio_fixture_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../test/fixtures/raspberry-supervisor/portfolio-program.yaml")
    }

    #[test]
    fn evaluate_fixture_classifies_all_lane_states() {
        let program = evaluate_program(&fixture_path()).expect("fixture should evaluate");
        let statuses: BTreeMap<String, LaneExecutionStatus> = program
            .lanes
            .iter()
            .map(|lane| (lane.lane_key.clone(), lane.status))
            .collect();

        assert_eq!(
            statuses.get("runtime:chapter"),
            Some(&LaneExecutionStatus::Complete)
        );
        assert_eq!(
            statuses.get("runtime:page"),
            Some(&LaneExecutionStatus::Ready)
        );
        assert_eq!(
            statuses.get("runtime:proof"),
            Some(&LaneExecutionStatus::Ready)
        );
        assert_eq!(
            statuses.get("consensus:chapter"),
            Some(&LaneExecutionStatus::Failed)
        );
        assert_eq!(
            statuses.get("consensus:page"),
            Some(&LaneExecutionStatus::Blocked)
        );
        assert_eq!(
            statuses.get("p2p:chapter"),
            Some(&LaneExecutionStatus::Failed)
        );
    }

    #[test]
    fn grouped_summary_mentions_ready_and_blocked_lanes() {
        let program = evaluate_program(&fixture_path()).expect("fixture should evaluate");
        let summary = render_grouped_summary(&program);
        assert!(summary.contains("Max parallel: 2"));
        assert!(summary.contains("runtime:page"));
        assert!(summary.contains("runtime:proof"));
        assert!(summary.contains("consensus:page"));
        assert!(summary.contains("Ready:"));
        assert!(summary.contains("Blocked:"));
    }

    #[test]
    fn evaluate_mysou_shaped_fixture_classifies_broad_repo_units() {
        let program =
            evaluate_program(&myosu_fixture_path()).expect("myosu fixture should evaluate");
        let statuses: BTreeMap<String, LaneExecutionStatus> = program
            .lanes
            .iter()
            .map(|lane| (lane.lane_key.clone(), lane.status))
            .collect();

        assert_eq!(
            statuses.get("chain:runtime"),
            Some(&LaneExecutionStatus::Complete)
        );
        assert_eq!(
            statuses.get("validator:oracle"),
            Some(&LaneExecutionStatus::Complete)
        );
        assert_eq!(
            statuses.get("miner:service"),
            Some(&LaneExecutionStatus::Failed)
        );
        assert_eq!(
            statuses.get("operations:scorecard"),
            Some(&LaneExecutionStatus::Blocked)
        );
        assert_eq!(
            statuses.get("launch:devnet"),
            Some(&LaneExecutionStatus::Blocked)
        );
        assert_eq!(statuses.get("play:tui"), Some(&LaneExecutionStatus::Failed));

        let miner = program
            .lanes
            .iter()
            .find(|lane| lane.lane_key == "miner:service")
            .expect("miner lane should exist");
        assert_eq!(miner.operational_state, None);
        let operations = program
            .lanes
            .iter()
            .find(|lane| lane.lane_key == "operations:scorecard")
            .expect("operations lane should exist");
        assert_eq!(operations.proof_state, Some(DerivedCheckState::Failed));
        let launch = program
            .lanes
            .iter()
            .find(|lane| lane.lane_key == "launch:devnet")
            .expect("launch lane should exist");
        assert_eq!(
            launch.orchestration_state,
            Some(LaneOrchestrationState::Waiting)
        );
    }

    #[test]
    fn evaluate_portfolio_fixture_summarizes_child_programs() {
        let program =
            evaluate_program(&portfolio_fixture_path()).expect("portfolio fixture should evaluate");
        let statuses: BTreeMap<String, LaneExecutionStatus> = program
            .lanes
            .iter()
            .map(|lane| (lane.lane_key.clone(), lane.status))
            .collect();

        assert_eq!(
            statuses.get("ready:program"),
            Some(&LaneExecutionStatus::Ready)
        );
        assert_eq!(
            statuses.get("complete:program"),
            Some(&LaneExecutionStatus::Complete)
        );

        let ready = program
            .lanes
            .iter()
            .find(|lane| lane.lane_key == "ready:program")
            .expect("ready program lane should exist");
        assert!(ready.detail.contains("child program `ready-program`"));
        assert!(ready.detail.contains("ready=1"));
        assert!(ready.detail.contains("ready child work available"));
    }

    #[test]
    fn evaluate_portfolio_prefers_child_state_snapshot_when_present() {
        let fixture_root = portfolio_fixture_path()
            .parent()
            .expect("fixture parent")
            .to_path_buf();
        let temp = tempfile::tempdir().expect("tempdir");
        copy_dir(&fixture_root, temp.path()).expect("copy fixture tree");

        let child_state_path = temp.path().join(".raspberry/complete-program-state.json");
        std::fs::write(
            &child_state_path,
            serde_json::json!({
                "schema_version": "raspberry.program.v2",
                "program": "complete-program",
                "updated_at": chrono::Utc::now(),
                "lanes": {
                    "docs:default": {
                        "lane_key": "docs:default",
                        "status": "running",
                        "run_config": "run-configs/runtime-page.toml"
                    }
                }
            })
            .to_string(),
        )
        .expect("write child state");

        let program = evaluate_program(&temp.path().join("portfolio-program.yaml"))
            .expect("portfolio evaluates");
        let statuses: BTreeMap<String, LaneExecutionStatus> = program
            .lanes
            .iter()
            .map(|lane| (lane.lane_key.clone(), lane.status))
            .collect();

        assert_eq!(
            statuses.get("complete:program"),
            Some(&LaneExecutionStatus::Running)
        );
        let complete = program
            .lanes
            .iter()
            .find(|lane| lane.lane_key == "complete:program")
            .expect("complete program lane should exist");
        assert!(complete.detail.contains("running_lanes=docs:default"));
    }

    #[test]
    fn evaluating_child_program_refreshes_parent_state() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path();
        std::fs::create_dir_all(repo.join("malinka/programs")).expect("programs dir");
        std::fs::create_dir_all(repo.join(".raspberry")).expect("state dir");

        let child_manifest_path = repo.join("malinka/programs/child.yaml");
        std::fs::write(
            &child_manifest_path,
            r#"
version: 1
program: child
target_repo: ../..
state_path: ../../.raspberry/child-state.json
units:
  - id: work
    title: Child Work
    output_root: ../../outputs/child
    artifacts:
      - id: spec
        path: spec.md
      - id: review
        path: review.md
    milestones:
      - id: reviewed
        requires: [spec, review]
    lanes:
      - id: task
        title: Child Task
        kind: artifact
        run_config: ../run-configs/bootstrap/task.toml
        managed_milestone: reviewed
        produces: [spec, review]
"#,
        )
        .expect("child manifest");
        std::fs::write(
            repo.join(".raspberry/child-state.json"),
            serde_json::json!({
                "schema_version": "raspberry.program.v2",
                "program": "child",
                "updated_at": chrono::Utc::now(),
                "lanes": {
                    "work:task": {
                        "lane_key": "work:task",
                        "status": "running",
                        "run_config": "malinka/run-configs/bootstrap/task.toml",
                        "current_run_id": "01KMCHILD000000000000000000"
                    }
                }
            })
            .to_string(),
        )
        .expect("child state");

        let parent_manifest_path = repo.join("malinka/programs/parent.yaml");
        std::fs::write(
            &parent_manifest_path,
            r#"
version: 1
program: parent
target_repo: ../..
state_path: ../../.raspberry/parent-state.json
units:
  - id: child
    title: Child Program
    lanes:
      - id: program
        title: Child Program Lane
        kind: orchestration
        run_config: ../run-configs/orchestration/child.toml
        program_manifest: child.yaml
        managed_milestone: coordinated
"#,
        )
        .expect("parent manifest");

        evaluate_program(&child_manifest_path).expect("child evaluates");

        let parent_state =
            ProgramRuntimeState::load_optional(&repo.join(".raspberry/parent-state.json"))
                .expect("parent state loads")
                .expect("parent state exists");
        assert_eq!(
            parent_state
                .lanes
                .get("child:program")
                .map(|record| record.status),
            Some(LaneExecutionStatus::Running)
        );
    }

    #[test]
    fn active_runtime_wins_over_satisfied_milestone() {
        let manifest = ProgramManifest::load(&portfolio_fixture_path()).expect("manifest loads");
        let manifest_path = portfolio_fixture_path();
        let unit = &manifest.units["complete"];
        let lane = &unit.lanes["program"];
        let satisfied = BTreeSet::from([managed_milestone_key(
            "complete:program",
            &lane.managed_milestone,
        )]);
        let runtime_record = LaneRuntimeRecord {
            lane_key: "complete:program".to_string(),
            status: LaneExecutionStatus::Running,
            run_config: Some(PathBuf::from("malinka/programs/complete-program.yaml")),
            current_run_id: Some("01KMTESTRUNNING0000000000000".to_string()),
            current_fabro_run_id: Some("01KMTESTRUNNING0000000000000".to_string()),
            current_stage_label: Some("Promote".to_string()),
            last_run_id: Some("01KMTESTRUNNING0000000000000".to_string()),
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
            consecutive_failures: 0,
        };
        let unit_status = evaluate_unit_status(
            &manifest_path,
            &manifest.resolved_target_repo(&manifest_path),
            unit,
        );
        let mut command_probe_cache = HashMap::new();
        let check_result =
            evaluate_lane_checks(&manifest_path, &manifest, lane, &mut command_probe_cache);
        let status = classify_lane(
            "complete:program",
            lane,
            &satisfied,
            &unit_status,
            Some(&runtime_record),
            &RunSnapshot::default(),
            &check_result,
            lane_orchestration_state(&manifest_path, lane),
            summarize_child_program(&manifest_path, lane).as_ref(),
        );

        assert_eq!(status, LaneExecutionStatus::Running);
    }

    #[test]
    fn orchestration_lane_without_child_manifest_stays_blocked() {
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
  - id: ops
    title: Ops
    milestones:
      - id: coordinated
        requires: []
    lanes:
      - id: program
        title: Ops Program
        kind: orchestration
        run_config: malinka/run-configs/orchestration/ops.toml
        managed_milestone: coordinated
"#,
        )
        .expect("manifest");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let unit = &manifest.units["ops"];
        let lane = &unit.lanes["program"];
        let mut command_probe_cache = HashMap::new();
        let check_result =
            evaluate_lane_checks(&manifest_path, &manifest, lane, &mut command_probe_cache);
        let status = classify_lane(
            "ops:program",
            lane,
            &BTreeSet::new(),
            &UnitStatus {
                lifecycle: "not_started".to_string(),
                present_artifacts: Vec::new(),
            },
            None,
            &RunSnapshot::default(),
            &check_result,
            lane_orchestration_state(&manifest_path, lane),
            None,
        );

        assert_eq!(status, LaneExecutionStatus::Blocked);
        assert_eq!(
            blocked_detail(
                "ops:program",
                lane,
                &BTreeSet::new(),
                &check_result,
                lane_orchestration_state(&manifest_path, lane),
            ),
            "orchestration lane missing child program manifest"
        );
    }

    #[test]
    fn service_lane_with_orchestration_run_config_stays_blocked() {
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
  - id: ops
    title: Ops
    milestones:
      - id: reviewed
        requires: []
    lanes:
      - id: deploy
        title: Deploy
        kind: service
        run_config: malinka/run-configs/orchestration/deploy.toml
        managed_milestone: reviewed
"#,
        )
        .expect("manifest");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let unit = &manifest.units["ops"];
        let lane = &unit.lanes["deploy"];
        let mut command_probe_cache = HashMap::new();
        let check_result =
            evaluate_lane_checks(&manifest_path, &manifest, lane, &mut command_probe_cache);
        let status = classify_lane(
            "ops:deploy",
            lane,
            &BTreeSet::new(),
            &UnitStatus {
                lifecycle: "not_started".to_string(),
                present_artifacts: Vec::new(),
            },
            Some(&LaneRuntimeRecord {
                lane_key: "ops:deploy".to_string(),
                status: LaneExecutionStatus::Failed,
                run_config: Some(PathBuf::from(
                    "malinka/run-configs/orchestration/deploy.toml",
                )),
                current_run_id: None,
                current_fabro_run_id: None,
                current_stage_label: None,
                last_run_id: Some("01KMOLD".to_string()),
                last_started_at: None,
                last_finished_at: Some(Utc::now()),
                last_exit_status: Some(1),
                last_error: Some("old supervisor-only failure".to_string()),
                failure_kind: Some(FailureKind::SupervisorOnlyLane),
                recovery_action: Some(FailureRecoveryAction::RegenerateLane),
                last_completed_stage_label: None,
                last_stage_duration_ms: None,
                last_usage_summary: None,
                last_files_read: Vec::new(),
                last_files_written: Vec::new(),
                last_stdout_snippet: None,
                last_stderr_snippet: None,
                consecutive_failures: 0,
            }),
            &RunSnapshot::default(),
            &check_result,
            lane_orchestration_state(&manifest_path, lane),
            None,
        );

        assert_eq!(status, LaneExecutionStatus::Blocked);
        assert_eq!(
            blocked_detail(
                "ops:deploy",
                lane,
                &BTreeSet::new(),
                &check_result,
                lane_orchestration_state(&manifest_path, lane),
            ),
            "orchestration lane missing child program manifest"
        );
    }

    #[test]
    fn stale_runtime_complete_does_not_satisfy_managed_milestone_without_artifacts() {
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
  - id: docs
    title: Docs
    output_root: outputs/docs
    artifacts:
      - id: plan
        path: plan.md
      - id: review
        path: review.md
    milestones:
      - id: reviewed
        requires: [plan, review]
    lanes:
      - id: lane
        title: Docs Lane
        kind: artifact
        run_config: malinka/run-configs/bootstrap/docs.toml
        managed_milestone: reviewed
        produces: [plan, review]
"#,
        )
        .expect("manifest");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let unit = &manifest.units["docs"];
        let lane = &unit.lanes["lane"];
        let runtime_record = LaneRuntimeRecord {
            lane_key: "docs:lane".to_string(),
            status: LaneExecutionStatus::Complete,
            run_config: Some(PathBuf::from("malinka/run-configs/bootstrap/docs.toml")),
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage_label: None,
            last_run_id: Some("01KMTESTCOMPLETE000000000000".to_string()),
            last_started_at: None,
            last_finished_at: None,
            last_exit_status: Some(0),
            last_error: None,
            failure_kind: None,
            recovery_action: None,
            last_completed_stage_label: Some("Review".to_string()),
            last_stage_duration_ms: None,
            last_usage_summary: None,
            last_files_read: Vec::new(),
            last_files_written: Vec::new(),
            last_stdout_snippet: None,
            last_stderr_snippet: None,
            consecutive_failures: 0,
        };

        let unit_status = evaluate_unit_status(
            &manifest_path,
            &manifest.resolved_target_repo(&manifest_path),
            unit,
        );
        let mut command_probe_cache = HashMap::new();
        let check_result =
            evaluate_lane_checks(&manifest_path, &manifest, lane, &mut command_probe_cache);
        let satisfied = satisfied_milestones(
            &manifest_path,
            &manifest,
            &BTreeMap::from([("docs:lane".to_string(), &runtime_record)]),
            &BTreeMap::from([("docs".to_string(), unit_status.clone())]),
        );
        let status = classify_lane(
            "docs:lane",
            lane,
            &satisfied,
            &unit_status,
            Some(&runtime_record),
            &RunSnapshot::default(),
            &check_result,
            lane_orchestration_state(&manifest_path, lane),
            None,
        );

        assert!(!satisfied.contains("docs:lane@reviewed"));
        assert_eq!(status, LaneExecutionStatus::Ready);
    }

    #[test]
    fn legacy_complete_lane_can_satisfy_unit_integrated_for_parent_dependencies() {
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
  - id: game
    title: Game
    output_root: outputs/game
    artifacts:
      - id: spec
        path: spec.md
      - id: review
        path: review.md
      - id: integration
        path: integration.md
    milestones:
      - id: implemented
        requires: [spec, review]
      - id: integrated
        requires: [integration]
    lanes:
      - id: lane
        title: Game Lane
        kind: artifact
        run_config: malinka/run-configs/bootstrap/game.toml
        managed_milestone: implemented
        produces: [spec, review, integration]
  - id: parent
    title: Parent
    output_root: outputs/parent
    artifacts:
      - id: verification
        path: verification.md
      - id: review
        path: review.md
    milestones:
      - id: parent-verified
        requires: [verification, review]
    lanes:
      - id: holistic-preflight
        title: Parent Holistic Preflight
        kind: platform
        run_config: malinka/run-configs/holistic-preflight/parent.toml
        managed_milestone: parent-verified
        depends_on:
          - unit: game
            milestone: integrated
        produces: [verification, review]
"#,
        )
        .expect("manifest");
        std::fs::create_dir_all(temp.path().join("outputs/game")).expect("outputs dir");
        std::fs::write(temp.path().join("outputs/game/spec.md"), "spec").expect("spec");
        std::fs::write(temp.path().join("outputs/game/review.md"), "review").expect("review");

        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let unit_statuses = build_unit_statuses(&manifest_path, &manifest);
        let runtime_record = LaneRuntimeRecord {
            lane_key: "game:lane".to_string(),
            status: LaneExecutionStatus::Complete,
            run_config: Some(PathBuf::from("malinka/run-configs/bootstrap/game.toml")),
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage_label: None,
            last_run_id: Some("01KMTESTLEGACYINTEGRATED000000".to_string()),
            last_started_at: None,
            last_finished_at: Some(Utc::now()),
            last_exit_status: Some(0),
            last_error: None,
            failure_kind: None,
            recovery_action: None,
            last_completed_stage_label: Some("Exit".to_string()),
            last_stage_duration_ms: None,
            last_usage_summary: None,
            last_files_read: Vec::new(),
            last_files_written: Vec::new(),
            last_stdout_snippet: None,
            last_stderr_snippet: None,
            consecutive_failures: 0,
        };
        let runtime_records = BTreeMap::from([("game:lane".to_string(), &runtime_record)]);
        let satisfied =
            satisfied_milestones(&manifest_path, &manifest, &runtime_records, &unit_statuses);

        assert!(satisfied.contains("game@integrated"));

        let parent_unit = &manifest.units["parent"];
        let parent_lane = &parent_unit.lanes["holistic-preflight"];
        let parent_status = evaluate_unit_status(
            &manifest_path,
            &manifest.resolved_target_repo(&manifest_path),
            parent_unit,
        );
        let mut command_probe_cache = HashMap::new();
        let check_result = evaluate_lane_checks(
            &manifest_path,
            &manifest,
            parent_lane,
            &mut command_probe_cache,
        );
        let status = classify_lane(
            "parent:holistic-preflight",
            parent_lane,
            &satisfied,
            &parent_status,
            None,
            &RunSnapshot::default(),
            &check_result,
            lane_orchestration_state(&manifest_path, parent_lane),
            None,
        );

        assert_eq!(status, LaneExecutionStatus::Ready);
    }

    #[test]
    fn evaluating_parent_with_missing_child_state_uses_local_child_summary() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path();
        std::fs::create_dir_all(repo.join("malinka/programs")).expect("programs dir");
        std::fs::create_dir_all(repo.join("outputs/child")).expect("outputs dir");

        let child_manifest_path = repo.join("malinka/programs/child.yaml");
        std::fs::write(
            &child_manifest_path,
            r#"
version: 1
program: child
target_repo: ../..
state_path: ../../.raspberry/child-state.json
units:
  - id: work
    title: Child Work
    output_root: ../../outputs/child
    artifacts:
      - id: spec
        path: spec.md
      - id: review
        path: review.md
    milestones:
      - id: reviewed
        requires: [spec, review]
    lanes:
      - id: task
        title: Child Task
        kind: artifact
        run_config: ../run-configs/bootstrap/task.toml
        managed_milestone: reviewed
        produces: [spec, review]
"#,
        )
        .expect("child manifest");

        let parent_manifest_path = repo.join("malinka/programs/parent.yaml");
        std::fs::write(
            &parent_manifest_path,
            r#"
version: 1
program: parent
target_repo: ../..
state_path: ../../.raspberry/parent-state.json
units:
  - id: child
    title: Child Program
    lanes:
      - id: program
        title: Child Program Lane
        kind: orchestration
        run_config: ../run-configs/orchestration/child.toml
        program_manifest: child.yaml
        managed_milestone: coordinated
"#,
        )
        .expect("parent manifest");

        let program = evaluate_program_local(&parent_manifest_path).expect("parent evaluates");
        let lane = program
            .lanes
            .iter()
            .find(|lane| lane.lane_key == "child:program")
            .expect("child program lane exists");

        assert_eq!(lane.status, LaneExecutionStatus::Ready);
        assert!(lane.detail.contains("child program `child`"));
        assert!(lane.detail.contains("ready=1"));
    }
}
