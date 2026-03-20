pub mod autodev;
pub mod dispatch;
pub mod evaluate;
pub mod failure;
mod integration;
pub mod manifest;
pub mod program_state;
mod resource_lease;

pub use autodev::{
    autodev_report_path, load_optional_autodev_report, orchestrate_program,
    sync_autodev_report_with_program, AutodevCurrentSnapshot, AutodevCycleReport, AutodevError,
    AutodevReport, AutodevSettings, AutodevStopReason,
};
pub use dispatch::{execute_selected_lanes, DispatchOutcome, DispatchSettings};
pub use evaluate::{
    evaluate_program, evaluate_program_local, render_grouped_summary, render_status_table,
    EvaluatedLane, EvaluatedProgram, LaneExecutionStatus,
};
pub use failure::{classify_failure, default_recovery_action, FailureKind, FailureRecoveryAction};
pub use manifest::{
    ArtifactKey, LaneDependency, LaneManifest, MilestoneManifest, ProgramManifest,
    ResolvedArtifact, UnitManifest,
};
pub use program_state::{refresh_program_state, LaneRuntimeRecord, ProgramRuntimeState};
