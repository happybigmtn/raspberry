pub mod autodev;
mod controller_lease;
pub mod dispatch;
pub mod evaluate;
pub mod failure;
mod integration;
pub mod maintenance;
pub mod manifest;
pub mod plan_cutover;
pub mod plan_registry;
pub mod plan_status;
pub mod portfolio_scheduler;
pub mod program_state;
mod resource_lease;

pub use autodev::{
    autodev_report_path, load_optional_autodev_report, orchestrate_program,
    sync_autodev_report_with_program, AutodevCurrentSnapshot, AutodevCycleReport, AutodevError,
    AutodevProvenance, AutodevReport, AutodevSettings, AutodevStopReason, BinaryProvenance,
};
pub use dispatch::{execute_selected_lanes, DispatchOutcome, DispatchSettings};
pub use evaluate::{
    evaluate_program, evaluate_program_local, render_grouped_summary, render_status_table,
    EvaluatedLane, EvaluatedProgram, LaneExecutionStatus,
};
pub use failure::{classify_failure, default_recovery_action, FailureKind, FailureRecoveryAction};
pub use maintenance::{
    load_active_maintenance, maintenance_path, MaintenanceError, MaintenanceMode,
};
pub use manifest::{
    ArtifactKey, LaneDependency, LaneManifest, MilestoneManifest, ProgramManifest,
    ResolvedArtifact, UnitManifest,
};
pub use plan_cutover::{
    compare_legacy_and_plan_truth, render_parity_report, CutoverPhase, PlanCutoverParity,
};
pub use plan_registry::{
    load_plan_registry, load_plan_registry_from_planning_root,
    load_plan_registry_relaxed_from_planning_root, PlanCategory, PlanChildRecord,
    PlanMappingSource, PlanRecord, PlanRegistry, PlanRegistryError, ReviewProfile,
    WorkflowArchetype,
};
pub use plan_status::{
    load_plan_matrix, render_plan_matrix, PlanMatrix, PlanStatusError, PlanStatusRow,
};
pub use program_state::{
    ensure_lane_record, mark_lane_dispatch_failed, mark_lane_finished, mark_lane_started,
    mark_lane_submitted, mark_lane_regenerate_noop, refresh_program_state,
    LaneRuntimeRecord, ProgramRuntimeState,
};
