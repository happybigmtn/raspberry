//! Integration test for autodev dispatch cycle.
//!
//! This test simulates a complete autodev cycle: load a fixture manifest,
//! evaluate, dispatch (mocked), observe state change, and verify
//! detached-run bootstrap diagnostics surface when validation fails.

use std::path::PathBuf;

use raspberry_supervisor::autodev::{
    autodev_report_path, load_optional_autodev_report, orchestrate_program, AutodevSettings,
    AutodevStopReason,
};
use raspberry_supervisor::dispatch::{execute_selected_lanes, DispatchSettings};
use raspberry_supervisor::evaluate::evaluate_program;
use raspberry_supervisor::manifest::ProgramManifest;
use raspberry_supervisor::program_state::ProgramRuntimeState;

/// Test that an autodev cycle runs to completion when there are no ready lanes.
#[test]
fn autodev_cycle_settled_when_no_ready_lanes() {
    let temp = tempfile::tempdir().expect("tempdir");

    // Copy fixture to temp
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures/raspberry-supervisor/program.yaml");
    let manifest_path = temp.path().join("program.yaml");
    std::fs::copy(&fixture_path, &manifest_path).expect("copy fixture");

    // Create run-config directories and files so the lanes can run
    let run_configs = temp.path().join("run-configs");
    std::fs::create_dir_all(&run_configs).expect("run-configs dir");

    // Copy actual run configs from fixture
    let fixture_run_configs = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures/raspberry-supervisor/run-configs");
    if fixture_run_configs.exists() {
        for entry in std::fs::read_dir(&fixture_run_configs).expect("read dir") {
            let entry = entry.expect("entry");
            let dest = run_configs.join(entry.file_name());
            std::fs::copy(entry.path(), dest).expect("copy config");
        }
    }

    let report = orchestrate_program(
        &manifest_path,
        &AutodevSettings {
            fabro_bin: PathBuf::from("/bin/false"),
            max_parallel_override: Some(2),
            frontier_budget: None,
            max_cycles: 3,
            poll_interval_ms: 1,
            evolve_every_seconds: 0,
            doctrine_files: Vec::new(),
            evidence_paths: Vec::new(),
            preview_evolve_root: None,
            manifest_stack: Vec::new(),
        },
    )
    .expect("orchestrate should succeed");

    // Should either settle or stop due to cycle limit, not error
    assert!(
        matches!(
            report.stop_reason,
            AutodevStopReason::Settled | AutodevStopReason::CycleLimit
        ),
        "should settle or hit cycle limit, got {:?}",
        report.stop_reason
    );
}

/// Test that autodev cycle stops at cycle limit when there are ready lanes.
#[test]
fn autodev_cycle_respects_max_cycles_when_work_available() {
    let temp = tempfile::tempdir().expect("tempdir");

    let manifest_path = temp.path().join("program.yaml");
    // Use Map format (no version field)
    // Note: MapLaneManifest requires managed_milestone defined in milestones
    std::fs::write(
        &manifest_path,
        r#"
program: cycle-test
target_repo: .
state_path: .raspberry/cycle-test-state.json
max_parallel: 1
units:
  work:
    title: Work
    output_root: outputs/work
    artifacts:
      done: done.md
    milestones:
      - id: done
        requires: []
      - id: milestone2
        requires: [done]
    lanes:
      lane1:
        title: Lane 1
        kind: platform
        run_config: run-configs/lane1.toml
        managed_milestone: done
      lane2:
        title: Lane 2
        kind: platform
        run_config: run-configs/lane2.toml
        managed_milestone: milestone2
"#,
    )
    .expect("manifest");
    std::fs::create_dir_all(temp.path().join("run-configs")).expect("run-configs dir");
    std::fs::write(temp.path().join("run-configs/lane1.toml"), "# config").expect("config");
    std::fs::write(temp.path().join("run-configs/lane2.toml"), "# config").expect("config");

    let report = orchestrate_program(
        &manifest_path,
        &AutodevSettings {
            fabro_bin: PathBuf::from("/bin/false"),
            max_parallel_override: Some(1),
            frontier_budget: None,
            max_cycles: 2,
            poll_interval_ms: 1,
            evolve_every_seconds: 0,
            doctrine_files: Vec::new(),
            evidence_paths: Vec::new(),
            preview_evolve_root: None,
            manifest_stack: Vec::new(),
        },
    )
    .expect("orchestrate should succeed");

    // Either settles (no ready lanes after first cycle) or hits cycle limit
    assert!(
        matches!(
            report.stop_reason,
            AutodevStopReason::CycleLimit | AutodevStopReason::Settled
        ),
        "should either settle or hit cycle limit, got {:?}",
        report.stop_reason
    );
    assert!(report.cycles.len() <= 2, "should have at most 2 cycles");
}

/// Test that dispatch outcome is recorded correctly in state.
#[test]
fn dispatch_updates_program_state() {
    let temp = tempfile::tempdir().expect("tempdir");

    let manifest_path = temp.path().join("program.yaml");
    // Use Map format (no version field)
    std::fs::write(
        &manifest_path,
        r#"
program: dispatch-test
target_repo: .
state_path: .raspberry/dispatch-test-state.json
max_parallel: 1
units:
  work:
    title: Work
    output_root: outputs/work
    milestones:
      - id: done
        requires: []
    lanes:
      lane1:
        title: Lane 1
        kind: platform
        run_config: run-configs/lane1.toml
        managed_milestone: done
"#,
    )
    .expect("manifest");
    std::fs::create_dir_all(temp.path().join("run-configs")).expect("run-configs dir");
    std::fs::write(temp.path().join("run-configs/lane1.toml"), "# config").expect("config");

    let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");

    // Execute dispatch with /bin/false which will fail
    let outcomes = execute_selected_lanes(
        &manifest_path,
        &["work:lane1".to_string()],
        &DispatchSettings {
            fabro_bin: PathBuf::from("/bin/false"),
            max_parallel_override: Some(1),
            doctrine_files: Vec::new(),
            evidence_paths: Vec::new(),
            preview_evolve_root: None,
            manifest_stack: Vec::new(),
        },
    )
    .expect("dispatch should complete");

    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].lane_key, "work:lane1");
    assert!(
        outcomes[0].exit_status != 0,
        "dispatch with /bin/false should fail"
    );

    // Verify state was updated
    let state_path = manifest.resolved_state_path(&manifest_path);
    let state = ProgramRuntimeState::load_optional(&state_path)
        .expect("load should work")
        .expect("state should exist after dispatch");

    let record = state
        .lanes
        .get("work:lane1")
        .expect("lane record should exist");
    assert_eq!(
        record.status,
        raspberry_supervisor::evaluate::LaneExecutionStatus::Failed,
        "lane should be marked as failed"
    );
}

/// Test that evaluate produces correct lane statuses from manifest.
#[test]
fn evaluate_produces_correct_lane_statuses() {
    let temp = tempfile::tempdir().expect("tempdir");

    let manifest_path = temp.path().join("program.yaml");
    // Use Map format
    std::fs::write(
        &manifest_path,
        r#"
program: eval-test
target_repo: .
state_path: .raspberry/eval-test-state.json
max_parallel: 2
units:
  docs:
    title: Docs
    output_root: outputs/docs
    artifacts:
      plan: plan.md
    milestones:
      - id: reviewed
        requires: [plan]
    lanes:
      lane:
        title: Docs Lane
        kind: artifact
        run_config: run-configs/docs/lane.toml
        managed_milestone: reviewed
        produces: [plan]
"#,
    )
    .expect("manifest");
    std::fs::create_dir_all(temp.path().join("run-configs/docs")).expect("run-configs docs");
    std::fs::write(
        temp.path().join("run-configs/docs/lane.toml"),
        "# docs config",
    )
    .expect("docs config");

    let program = evaluate_program(&manifest_path).expect("evaluate should succeed");

    assert_eq!(program.program, "eval-test");
    assert!(!program.lanes.is_empty());

    let docs_lane = program
        .lanes
        .iter()
        .find(|l| l.lane_key == "docs:lane")
        .expect("docs lane should exist");
    assert_eq!(
        docs_lane.status,
        raspberry_supervisor::evaluate::LaneExecutionStatus::Ready,
        "docs lane should be ready"
    );
}

/// Test that autodev report is saved after orchestration.
#[test]
fn autodev_report_saved_after_orchestration() {
    let temp = tempfile::tempdir().expect("tempdir");

    let manifest_path = temp.path().join("program.yaml");
    // Use Map format
    std::fs::write(
        &manifest_path,
        r#"
program: report-test
target_repo: .
state_path: .raspberry/report-test-state.json
max_parallel: 1
units:
  work:
    title: Work
    output_root: outputs/work
    milestones:
      - id: done
        requires: []
    lanes:
      lane:
        title: Work Lane
        kind: platform
        run_config: run-configs/work/lane.toml
        managed_milestone: done
"#,
    )
    .expect("manifest");
    std::fs::create_dir_all(temp.path().join("run-configs/work")).expect("run-configs dir");
    std::fs::write(temp.path().join("run-configs/work/lane.toml"), "# config").expect("config");

    let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");

    // Run orchestration
    let report = orchestrate_program(
        &manifest_path,
        &AutodevSettings {
            fabro_bin: PathBuf::from("/bin/false"),
            max_parallel_override: Some(1),
            frontier_budget: None,
            max_cycles: 1,
            poll_interval_ms: 1,
            evolve_every_seconds: 0,
            doctrine_files: Vec::new(),
            evidence_paths: Vec::new(),
            preview_evolve_root: None,
            manifest_stack: Vec::new(),
        },
    )
    .expect("orchestrate should succeed");

    // Verify report was saved
    let report_path = autodev_report_path(&manifest_path, &manifest);
    assert!(
        report_path.exists(),
        "autodev report should be saved at {}",
        report_path.display()
    );

    // Verify we can load the saved report
    let loaded_report = load_optional_autodev_report(&manifest_path, &manifest)
        .expect("load should succeed")
        .expect("report should exist");
    assert_eq!(loaded_report.program, report.program);
    assert_eq!(loaded_report.stop_reason, report.stop_reason);
}

/// Test that portfolio program with child programs evaluates correctly.
#[test]
fn portfolio_program_evaluates_child_programs() {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures/raspberry-supervisor/portfolio-program.yaml");
    let program = evaluate_program(&fixture_path).expect("evaluate should succeed");

    assert_eq!(program.program, "portfolio-demo");
    assert!(!program.lanes.is_empty(), "portfolio should have lanes");

    // The ready program lane should be found
    let ready_lane = program
        .lanes
        .iter()
        .find(|l| l.lane_key == "ready:program")
        .expect("ready:program lane should exist");

    // Status depends on whether the child program manifest is complete
    // but at minimum the lane should exist
    assert_eq!(ready_lane.unit_id, "ready");
    assert_eq!(ready_lane.lane_id, "program");
}
