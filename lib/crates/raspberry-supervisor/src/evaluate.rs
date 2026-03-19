use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Utc};
use fabro_workflows::live_state::RunLiveState;
use fabro_workflows::run_status::{RunStatus, RunStatusRecord};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::manifest::{
    LaneCheck, LaneCheckKind, LaneCheckProbe, LaneCheckScope, LaneDependency, LaneKind,
    LaneManifest, ProgramManifest,
};
use crate::program_state::{
    LaneRuntimeRecord, ProgramRuntimeState, ProgramStateError, refresh_program_state,
};

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
}

#[derive(Debug, Error)]
pub enum EvaluateError {
    #[error(transparent)]
    Manifest(#[from] crate::manifest::ManifestError),
    #[error(transparent)]
    ProgramState(#[from] ProgramStateError),
}

pub fn evaluate_program(manifest_path: &Path) -> Result<EvaluatedProgram, EvaluateError> {
    let manifest = ProgramManifest::load(manifest_path)?;
    let state_path = manifest.resolved_state_path(manifest_path);
    let mut program_state =
        ProgramRuntimeState::load_optional(&state_path)?.unwrap_or_else(|| ProgramRuntimeState::new(&manifest.program));
    if refresh_program_state(manifest_path, &manifest, &mut program_state)? {
        program_state.save(&state_path)?;
    }
    Ok(evaluate_with_state(manifest_path, &manifest, Some(&program_state)))
}

pub fn evaluate_with_state(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    program_state: Option<&ProgramRuntimeState>,
) -> EvaluatedProgram {
    let runtime_records = runtime_record_map(program_state);
    let unit_statuses = build_unit_statuses(manifest_path, manifest);
    let satisfied = satisfied_milestones(
        manifest_path,
        manifest,
        &runtime_records,
        &unit_statuses,
    );
    let mut lanes = Vec::new();

    for (unit_id, unit) in &manifest.units {
        for (lane_id, _lane) in &unit.lanes {
            lanes.push(evaluate_lane(
                manifest_path,
                manifest,
                unit_id,
                lane_id,
                &satisfied,
                unit_statuses
                    .get(unit_id)
                    .expect("unit status should exist for every unit"),
                runtime_records
                    .get(&lane_key(unit_id, lane_id))
                    .copied(),
            ));
        }
    }

    EvaluatedProgram {
        program: manifest.program.clone(),
        max_parallel: manifest.max_parallel,
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
            let mut line = format!("  - {} [{}] — {}", lane.lane_key, lane.lane_kind, lane.detail);
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
            counts.get(&LaneExecutionStatus::Complete).copied().unwrap_or(0),
            counts.get(&LaneExecutionStatus::Ready).copied().unwrap_or(0),
            counts.get(&LaneExecutionStatus::Running).copied().unwrap_or(0),
            counts.get(&LaneExecutionStatus::Blocked).copied().unwrap_or(0),
            counts.get(&LaneExecutionStatus::Failed).copied().unwrap_or(0),
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

fn evaluate_lane(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    unit_id: &str,
    lane_id: &str,
    satisfied: &BTreeSet<String>,
    unit_status: &UnitStatus,
    runtime_record: Option<&LaneRuntimeRecord>,
) -> EvaluatedLane {
    let unit = &manifest.units[unit_id];
    let lane = &unit.lanes[lane_id];
    let key = lane_key(unit_id, lane_id);
    let run_snapshot = manifest
        .resolve_lane_run_dir(manifest_path, unit_id, lane_id)
        .map(|run_dir| load_run_snapshot(&run_dir))
        .unwrap_or_default();
    let check_result = evaluate_lane_checks(manifest_path, manifest, lane);
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
            .or_else(|| run_snapshot.run_id),
        current_run_id: runtime_record.and_then(|record| record.current_run_id.clone()),
        current_fabro_run_id: runtime_record
            .and_then(|record| record.current_fabro_run_id.clone()),
        current_stage: runtime_record
            .and_then(|record| record.current_stage_label.clone())
            .or(run_snapshot.current_stage),
        last_run_id: runtime_record.and_then(|record| record.last_run_id.clone()),
        last_started_at: runtime_record.and_then(|record| record.last_started_at),
        last_finished_at: runtime_record.and_then(|record| record.last_finished_at),
        last_exit_status: runtime_record.and_then(|record| record.last_exit_status),
        last_error: runtime_record.and_then(|record| record.last_error.clone()),
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
            if lifecycle_reached(unit, unit_status, &milestone.id) {
                satisfied.insert(format!("{unit_id}@{}", milestone.id));
            }
        }
        for (lane_id, lane) in &unit.lanes {
            let lane_key = lane_key(unit_id, lane_id);
            let managed_complete = runtime_records
                .get(&lane_key)
                .map(|record| record.status == LaneExecutionStatus::Complete)
                .unwrap_or(false)
                || lane
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

fn build_unit_statuses(
    manifest_path: &Path,
    manifest: &ProgramManifest,
) -> BTreeMap<String, UnitStatus> {
    manifest
        .units
        .iter()
        .map(|(unit_id, unit)| {
            (
                unit_id.clone(),
                evaluate_unit_status(manifest_path, unit),
            )
        })
        .collect()
}

fn evaluate_unit_status(manifest_path: &Path, unit: &crate::manifest::UnitManifest) -> UnitStatus {
    let present_artifacts = unit
        .artifacts
        .iter()
        .filter(|(_, path)| artifact_path(manifest_path, unit, path).is_file())
        .map(|(artifact_id, _)| artifact_id.clone())
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

fn lifecycle_reached(unit: &crate::manifest::UnitManifest, status: &UnitStatus, target: &str) -> bool {
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

fn artifact_path(manifest_path: &Path, unit: &crate::manifest::UnitManifest, path: &Path) -> PathBuf {
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

fn runtime_record_map(
    state: Option<&ProgramRuntimeState>,
) -> BTreeMap<String, &LaneRuntimeRecord> {
    state
        .map(|state| state.lanes.iter().map(|(k, v)| (k.clone(), v)).collect())
        .unwrap_or_default()
}

fn classify_lane(
    lane_key: &str,
    lane: &crate::manifest::LaneManifest,
    satisfied: &BTreeSet<String>,
    unit_status: &UnitStatus,
    runtime_record: Option<&LaneRuntimeRecord>,
    run_snapshot: &RunSnapshot,
    check_result: &LaneCheckResult,
    orchestration_state: Option<LaneOrchestrationState>,
    child_program: Option<&ChildProgramSummary>,
) -> LaneExecutionStatus {
    if let Some(child_program) = child_program {
        return classify_child_program_lane(
            lane,
            satisfied,
            check_result,
            orchestration_state,
            child_program,
        );
    }
    if runtime_record
        .map(|record| record.status == LaneExecutionStatus::Complete)
        .unwrap_or(false)
        || satisfied.contains(&managed_milestone_key(lane_key, &lane.managed_milestone))
    {
        return LaneExecutionStatus::Complete;
    }
    if is_failed(run_snapshot, runtime_record) {
        return LaneExecutionStatus::Failed;
    }
    if !check_result.ready_precondition_failing.is_empty() || !check_result.ready_proof_failing.is_empty() {
        return LaneExecutionStatus::Blocked;
    }
    if let Some(LaneOrchestrationState::Waiting | LaneOrchestrationState::Blocked) =
        orchestration_state
    {
        return LaneExecutionStatus::Blocked;
    }
    if is_active(run_snapshot, runtime_record) || produced_present(lane, unit_status) {
        return LaneExecutionStatus::Running;
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
    matches!(run_snapshot.status, Some(RunStatus::Failed | RunStatus::Dead))
        || runtime_record
            .map(|record| record.status == LaneExecutionStatus::Failed)
            .unwrap_or(false)
}

fn dependencies_satisfied(
    dependencies: &[LaneDependency],
    satisfied: &BTreeSet<String>,
) -> bool {
    dependencies.iter().all(|dependency| {
        let key = dependency_key(dependency);
        satisfied.contains(&key)
    })
}

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
            if !check_result.ready_proof_passing.is_empty() && !check_result.ready_precondition_passing.is_empty() {
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
                    .and_then(|record| record.last_stderr_snippet.clone())
                    .filter(|text| !text.trim().is_empty())
            })
            .unwrap_or_else(|| "most recent run failed".to_string()),
        LaneExecutionStatus::Blocked => blocked_detail(
            lane_key,
            lane,
            satisfied,
            check_result,
            orchestration_state,
        ),
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
    let summary = format!(
        "child program `{}`: complete={} ready={} running={} blocked={} failed={}",
        child_program.program,
        child_program.complete,
        child_program.ready,
        child_program.running,
        child_program.blocked,
        child_program.failed
    );
    match status {
        LaneExecutionStatus::Complete => format!("{summary} | all child lanes complete"),
        LaneExecutionStatus::Ready => format!("{summary} | ready child work available"),
        LaneExecutionStatus::Running => format!("{summary} | child work in flight"),
        LaneExecutionStatus::Failed => format!("{summary} | child program has failures"),
        LaneExecutionStatus::Blocked => {
            if child_program.ready > 0 {
                format!(
                    "{summary} | waiting on {}",
                    blocked_detail(
                        lane_key,
                        lane,
                        satisfied,
                        check_result,
                        orchestration_state,
                    )
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
        return format!("waiting on checks: {}", check_result.ready_failing.join(", "));
    }
    lane.dependencies
        .iter()
        .find(|dependency| !satisfied.contains(&dependency_key(dependency)))
        .map(|dependency| {
            dependency_display_key(dependency)
        })
        .unwrap_or_else(|| format!("lane `{lane_key}` is blocked"))
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
    let program = evaluate_program(&program_manifest).ok()?;
    let mut summary = ChildProgramSummary {
        program: program.program,
        complete: 0,
        ready: 0,
        running: 0,
        blocked: 0,
        failed: 0,
        total: program.lanes.len(),
    };
    for lane in program.lanes {
        match lane.status {
            LaneExecutionStatus::Complete => summary.complete += 1,
            LaneExecutionStatus::Ready => summary.ready += 1,
            LaneExecutionStatus::Running => summary.running += 1,
            LaneExecutionStatus::Blocked => summary.blocked += 1,
            LaneExecutionStatus::Failed => summary.failed += 1,
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
    lane.produces
        .iter()
        .any(|artifact| unit_status.present_artifacts.iter().any(|present| present == artifact))
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
) -> LaneCheckResult {
    let mut result = LaneCheckResult::default();
    for check in &lane.checks {
        let passed = evaluate_check(manifest_path, manifest, check);
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

fn evaluate_check(manifest_path: &Path, manifest: &ProgramManifest, check: &LaneCheck) -> bool {
    evaluate_probe(manifest_path, manifest, &check.probe)
}

fn evaluate_probe(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    probe: &LaneCheckProbe,
) -> bool {
    match probe {
        LaneCheckProbe::FileExists { path } => resolve_check_path(manifest_path, path).is_file(),
        LaneCheckProbe::JsonFieldEquals { path, field, equals } => {
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
            run_check_command(manifest_path, manifest, command)
                .map(|output| output.status.success())
                .unwrap_or(false)
        }
        LaneCheckProbe::CommandStdoutContains { command, contains } => {
            run_check_command(manifest_path, manifest, command)
                .map(|output| {
                    output.status.success()
                        && String::from_utf8_lossy(&output.stdout).contains(contains)
                })
                .unwrap_or(false)
        }
    }
}

fn run_check_command(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    command: &str,
) -> Option<std::process::Output> {
    let cwd = manifest.resolved_target_repo(manifest_path);
    Command::new("bash")
        .arg("-lc")
        .arg(command)
        .current_dir(cwd)
        .output()
        .ok()
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
            Some(&LaneExecutionStatus::Running)
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
        let program = evaluate_program(&myosu_fixture_path()).expect("myosu fixture should evaluate");
        let statuses: BTreeMap<String, LaneExecutionStatus> = program
            .lanes
            .iter()
            .map(|lane| (lane.lane_key.clone(), lane.status))
            .collect();

        assert_eq!(statuses.get("chain:runtime"), Some(&LaneExecutionStatus::Complete));
        assert_eq!(statuses.get("validator:oracle"), Some(&LaneExecutionStatus::Complete));
        assert_eq!(statuses.get("miner:service"), Some(&LaneExecutionStatus::Running));
        assert_eq!(statuses.get("operations:scorecard"), Some(&LaneExecutionStatus::Blocked));
        assert_eq!(statuses.get("launch:devnet"), Some(&LaneExecutionStatus::Blocked));
        assert_eq!(statuses.get("play:tui"), Some(&LaneExecutionStatus::Failed));

        let miner = program
            .lanes
            .iter()
            .find(|lane| lane.lane_key == "miner:service")
            .expect("miner lane should exist");
        assert_eq!(miner.operational_state, Some(LaneOperationalState::Healthy));
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
        assert_eq!(launch.orchestration_state, Some(LaneOrchestrationState::Waiting));
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

        assert_eq!(statuses.get("ready:program"), Some(&LaneExecutionStatus::Ready));
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
    }
}
