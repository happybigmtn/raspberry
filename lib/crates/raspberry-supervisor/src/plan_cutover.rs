use std::collections::BTreeMap;

use serde_json::json;

use crate::evaluate::{EvaluatedProgram, LaneExecutionStatus};
use crate::plan_registry::{PlanCategory, PlanRegistry};
use crate::plan_status::PlanMatrix;

/// Parity comparison between legacy lane-centric truth and plan-centric truth.
#[derive(Debug, Clone)]
pub struct PlanCutoverParity {
    pub legacy_summary: serde_json::Value,
    pub plan_summary: serde_json::Value,
    pub differences: Vec<String>,
    pub cutover_safe: bool,
}

/// Three-phase cutover state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutoverPhase {
    /// Plan-first data is generated but not authoritative. Lane-centric dispatch
    /// remains active. Plan dashboard is read-only shadow.
    Shadow,
    /// Parity review: both views are live, differences are surfaced. Operator
    /// must approve cutover or roll back to Shadow.
    ParityReview,
    /// Plan-centric truth is authoritative for dispatch and dashboard. Lane
    /// detail retained as fallback/debugging aid.
    PlanFirst,
}

impl CutoverPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shadow => "shadow",
            Self::ParityReview => "parity_review",
            Self::PlanFirst => "plan_first",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "shadow" => Some(Self::Shadow),
            "parity_review" => Some(Self::ParityReview),
            "plan_first" => Some(Self::PlanFirst),
            _ => None,
        }
    }
}

/// Compare legacy lane-centric truth against plan-centric truth for the same
/// program. Returns a parity report with differences and a safety verdict.
///
/// Both inputs are pure data — no I/O needed beyond loading them.
pub fn compare_legacy_and_plan_truth(
    program: &EvaluatedProgram,
    registry: &PlanRegistry,
    matrix: &PlanMatrix,
) -> PlanCutoverParity {
    let mut differences = Vec::new();

    // Legacy summary: lane status counts
    let mut legacy_ready = 0usize;
    let mut legacy_running = 0usize;
    let mut legacy_blocked = 0usize;
    let mut legacy_failed = 0usize;
    let mut legacy_complete = 0usize;
    for lane in &program.lanes {
        match lane.status {
            LaneExecutionStatus::Ready => legacy_ready += 1,
            LaneExecutionStatus::Running => legacy_running += 1,
            LaneExecutionStatus::Blocked => legacy_blocked += 1,
            LaneExecutionStatus::Failed => legacy_failed += 1,
            LaneExecutionStatus::Complete => legacy_complete += 1,
        }
    }

    // Plan summary: plan status counts from matrix
    let mut plan_ready = 0usize;
    let mut plan_running = 0usize;
    let mut plan_blocked = 0usize;
    let mut plan_failed = 0usize;
    let mut plan_complete = 0usize;
    let mut plan_unmodeled = 0usize;
    for row in &matrix.rows {
        let s = row.current_status.as_str();
        if s.ends_with("_ready") || s == "ready" {
            plan_ready += 1;
        } else if s.ends_with("_running") || s == "running" {
            plan_running += 1;
        } else if s.ends_with("_blocked") || s == "blocked" {
            plan_blocked += 1;
        } else if s.ends_with("_failed") || s == "failed" {
            plan_failed += 1;
        } else if s.ends_with("_complete") || s == "complete" || s == "reviewed" {
            plan_complete += 1;
        } else if s == "unmodeled" {
            plan_unmodeled += 1;
        }
    }

    // Check: are there plans that have no lane representation?
    let lane_unit_ids: BTreeMap<&str, &LaneExecutionStatus> = program
        .lanes
        .iter()
        .map(|l| (l.unit_id.as_str(), &l.status))
        .collect();

    for plan in &registry.plans {
        if plan.category == PlanCategory::Meta {
            continue;
        }
        if !lane_unit_ids.contains_key(plan.plan_id.as_str()) {
            differences.push(format!(
                "plan `{}` has no lane representation in legacy frontier",
                plan.plan_id,
            ));
        }
    }

    // Check: are there lanes that have no plan representation?
    let plan_ids: BTreeMap<&str, ()> = registry
        .plans
        .iter()
        .map(|p| (p.plan_id.as_str(), ()))
        .collect();
    for lane in &program.lanes {
        let base_unit = lane
            .unit_id
            .strip_suffix("-implementation")
            .unwrap_or(&lane.unit_id);
        if !plan_ids.contains_key(base_unit) {
            differences.push(format!(
                "lane `{}` has no corresponding plan in registry",
                lane.lane_key,
            ));
        }
    }

    // Check: unmodeled plans are a risk
    if plan_unmodeled > 0 {
        differences.push(format!(
            "{plan_unmodeled} plan(s) are unmodeled — synthesis has not rendered them yet",
        ));
    }

    // Check: status divergence on failed/blocked
    if legacy_failed != plan_failed {
        differences.push(format!(
            "failed count diverges: legacy={legacy_failed}, plan={plan_failed}",
        ));
    }
    if legacy_blocked != plan_blocked {
        differences.push(format!(
            "blocked count diverges: legacy={legacy_blocked}, plan={plan_blocked}",
        ));
    }

    // Cutover is safe when:
    // - no unmodeled plans remain
    // - no orphan lanes (all lanes map to plans)
    // - no failed states in either view
    let cutover_safe = plan_unmodeled == 0
        && legacy_failed == 0
        && plan_failed == 0
        && differences
            .iter()
            .all(|d| !d.contains("no lane representation") && !d.contains("no corresponding plan"));

    let legacy_summary = json!({
        "totalLanes": program.lanes.len(),
        "ready": legacy_ready,
        "running": legacy_running,
        "blocked": legacy_blocked,
        "failed": legacy_failed,
        "complete": legacy_complete,
    });

    let plan_summary = json!({
        "totalPlans": matrix.rows.len(),
        "ready": plan_ready,
        "running": plan_running,
        "blocked": plan_blocked,
        "failed": plan_failed,
        "complete": plan_complete,
        "unmodeled": plan_unmodeled,
    });

    PlanCutoverParity {
        legacy_summary,
        plan_summary,
        differences,
        cutover_safe,
    }
}

/// Render the parity report as a human-readable string.
pub fn render_parity_report(parity: &PlanCutoverParity) -> String {
    let mut lines = vec![
        "# Plan Cutover Parity Report".to_string(),
        String::new(),
        format!(
            "## Legacy (lane-centric)\n\n```json\n{}\n```",
            serde_json::to_string_pretty(&parity.legacy_summary).unwrap_or_default()
        ),
        String::new(),
        format!(
            "## Plan-first\n\n```json\n{}\n```",
            serde_json::to_string_pretty(&parity.plan_summary).unwrap_or_default()
        ),
        String::new(),
    ];

    if parity.differences.is_empty() {
        lines.push("## Differences\n\nNone.".to_string());
    } else {
        lines.push("## Differences\n".to_string());
        for diff in &parity.differences {
            lines.push(format!("- {diff}"));
        }
    }

    lines.push(String::new());
    lines.push(format!(
        "## Verdict\n\nCutover safe: **{}**",
        if parity.cutover_safe { "YES" } else { "NO" }
    ));

    if !parity.cutover_safe {
        lines.push(String::new());
        lines.push("### Rollback\n\nTo roll back to lane-centric dispatch, set cutover phase to `shadow` in the program state. The legacy lane scheduler will resume and the plan dashboard will become read-only.".to_string());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluate::EvaluatedLane;
    use crate::manifest::LaneKind;
    use crate::plan_registry::*;
    use std::path::PathBuf;

    fn test_lane(unit_id: &str, lane_id: &str, status: LaneExecutionStatus) -> EvaluatedLane {
        EvaluatedLane {
            lane_key: format!("{unit_id}:{lane_id}"),
            unit_id: unit_id.to_string(),
            unit_title: unit_id.to_string(),
            lane_id: lane_id.to_string(),
            lane_title: lane_id.to_string(),
            lane_kind: LaneKind::Artifact,
            status,
            operational_state: None,
            precondition_state: None,
            proof_state: None,
            orchestration_state: None,
            detail: String::new(),
            managed_milestone: "reviewed".to_string(),
            proof_profile: None,
            run_config: PathBuf::new(),
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
            last_files_read: vec![],
            last_files_written: vec![],
            last_stdout_snippet: None,
            last_stderr_snippet: None,
            ready_checks_passing: vec![],
            ready_checks_failing: vec![],
            running_checks_passing: vec![],
            running_checks_failing: vec![],
            consecutive_failures: 0,
        }
    }

    fn test_plan_record(id: &str, category: PlanCategory) -> PlanRecord {
        PlanRecord {
            plan_id: id.to_string(),
            path: PathBuf::from(format!("plans/{id}.md")),
            title: id.to_string(),
            category,
            composite: false,
            dependency_plan_ids: vec![],
            mapping_contract_path: None,
            mapping_source: PlanMappingSource::Inferred,
            bootstrap_required: true,
            implementation_required: true,
            declared_child_ids: vec![],
            children: vec![],
        }
    }

    fn test_status_row(plan_id: &str, status: &str) -> crate::plan_status::PlanStatusRow {
        crate::plan_status::PlanStatusRow {
            plan_id: plan_id.to_string(),
            plan_file: format!("plans/{plan_id}.md"),
            title: plan_id.to_string(),
            category: "game".to_string(),
            composite: false,
            mapping_status: "mapped".to_string(),
            child_count: 0,
            represented_in_blueprint: true,
            has_bootstrap_lane: true,
            has_implementation_lane: false,
            has_real_verify_gate: false,
            current_status: status.to_string(),
            current_risk: String::new(),
            next_operator_move: "inspect".to_string(),
        }
    }

    #[test]
    fn parity_reports_unmodeled_plans_as_unsafe() {
        let program = EvaluatedProgram {
            program: "test".to_string(),
            max_parallel: 3,
            runtime_max_parallel: None,
            lanes: vec![test_lane("craps", "craps", LaneExecutionStatus::Ready)],
        };
        let registry = PlanRegistry {
            plans: vec![
                test_plan_record("craps", PlanCategory::Game),
                test_plan_record("poker", PlanCategory::Game),
            ],
        };
        let matrix = PlanMatrix {
            program: "test".to_string(),
            rows: vec![
                test_status_row("craps", "bootstrap_ready"),
                test_status_row("poker", "unmodeled"),
            ],
        };

        let parity = compare_legacy_and_plan_truth(&program, &registry, &matrix);

        assert!(!parity.cutover_safe);
        assert!(parity.differences.iter().any(|d| d.contains("unmodeled")));
        assert!(parity
            .differences
            .iter()
            .any(|d| d.contains("poker") && d.contains("no lane representation")));
    }

    #[test]
    fn parity_safe_when_plans_and_lanes_align() {
        let program = EvaluatedProgram {
            program: "test".to_string(),
            max_parallel: 3,
            runtime_max_parallel: None,
            lanes: vec![
                test_lane("craps", "craps", LaneExecutionStatus::Ready),
                test_lane("poker", "poker", LaneExecutionStatus::Complete),
            ],
        };
        let registry = PlanRegistry {
            plans: vec![
                test_plan_record("craps", PlanCategory::Game),
                test_plan_record("poker", PlanCategory::Game),
            ],
        };
        let matrix = PlanMatrix {
            program: "test".to_string(),
            rows: vec![
                test_status_row("craps", "bootstrap_ready"),
                test_status_row("poker", "reviewed"),
            ],
        };

        let parity = compare_legacy_and_plan_truth(&program, &registry, &matrix);

        assert!(parity.cutover_safe);
        assert!(parity.differences.is_empty());
    }

    #[test]
    fn parity_unsafe_when_failures_present() {
        let program = EvaluatedProgram {
            program: "test".to_string(),
            max_parallel: 3,
            runtime_max_parallel: None,
            lanes: vec![test_lane("craps", "craps", LaneExecutionStatus::Failed)],
        };
        let registry = PlanRegistry {
            plans: vec![test_plan_record("craps", PlanCategory::Game)],
        };
        let matrix = PlanMatrix {
            program: "test".to_string(),
            rows: vec![test_status_row("craps", "bootstrap_failed")],
        };

        let parity = compare_legacy_and_plan_truth(&program, &registry, &matrix);

        assert!(!parity.cutover_safe);
    }

    #[test]
    fn render_parity_report_includes_verdict() {
        let parity = PlanCutoverParity {
            legacy_summary: json!({"totalLanes": 2}),
            plan_summary: json!({"totalPlans": 2}),
            differences: vec!["test diff".to_string()],
            cutover_safe: false,
        };

        let report = render_parity_report(&parity);

        assert!(report.contains("Cutover safe: **NO**"));
        assert!(report.contains("test diff"));
        assert!(report.contains("Rollback"));
    }
}
