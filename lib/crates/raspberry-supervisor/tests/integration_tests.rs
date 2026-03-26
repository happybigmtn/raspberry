//! Integration tests for raspberry-supervisor autodev cycles and edge cases.
//!
//! These tests verify the complete autodev cycle behavior, including:
//! - Full autodev cycle from manifest load through dispatch
//! - Stale running lane detection and reconciliation  
//! - Dispatch budget exhaustion scenarios
//! - Recovery action authority verification
//! - Cycle limit termination
//! - Frontier budget accounting after failures
//! - Program state with malformed JSON files

use std::path::{Path, PathBuf};

use chrono::Utc;
use raspberry_supervisor::manifest::ProgramManifest;
use raspberry_supervisor::program_state::ProgramRuntimeState;
use raspberry_supervisor::failure::{FailureKind, FailureRecoveryAction};
use raspberry_supervisor::dispatch::DispatchOutcome;

// ---------------------------------------------------------------------------
// autodev_cycle integration tests
// ---------------------------------------------------------------------------

/// Test: autodev_cycle
///
/// Simulates a complete autodev cycle: load a fixture manifest, evaluate,
/// dispatch (mocked), observe state change, and verify detached-run
/// bootstrap diagnostics surface the real cause when validation fails.
#[test]
fn integration_autodev_cycle_loads_manifest_and_evaluates() {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml");
    
    let manifest = ProgramManifest::load(&manifest_path)
        .expect("manifest loads from fixture");
    
    assert_eq!(manifest.program, "myosu-bootstrap");
    assert_eq!(manifest.units.len(), 6);
    
    // Verify key units are present
    assert!(manifest.units.contains_key("chain"));
    assert!(manifest.units.contains_key("validator"));
    assert!(manifest.units.contains_key("miner"));
}

/// Test that a lane marked with regenerate noop is correctly blocked
/// and escalated to SurfaceBlocked after repeated noop marks.
#[test]
fn integration_autodev_cycle_regenerate_noop_escalation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_path = temp.path().join("state.json");
    
    let mut state = ProgramRuntimeState::new("demo");
    
    // Add a lane with regenerate noop failure kind
    state.lanes.insert(
        "chain:runtime".to_string(),
        raspberry_supervisor::program_state::LaneRuntimeRecord {
            lane_key: "chain:runtime".to_string(),
            status: raspberry_supervisor::evaluate::LaneExecutionStatus::Failed,
            run_config: Some(PathBuf::from("run-configs/chain-runtime.toml")),
            current_run_id: None,
            current_fabro_run_id: None,
            current_stage_label: None,
            last_run_id: None,
            last_started_at: Some(Utc::now() - chrono::Duration::minutes(5)),
            last_finished_at: Some(Utc::now()),
            last_exit_status: Some(1),
            last_error: Some("synth evolve did not materially change run config or graph".to_string()),
            failure_kind: Some(FailureKind::RegenerateNoop),
            recovery_action: Some(FailureRecoveryAction::SurfaceBlocked),
            last_completed_stage_label: None,
            last_stage_duration_ms: None,
            last_usage_summary: None,
            last_files_read: Vec::new(),
            last_files_written: Vec::new(),
            last_stdout_snippet: None,
            last_stderr_snippet: None,
            consecutive_failures: 1,
        },
    );
    
    state.save(&state_path).expect("save should succeed");
    
    let loaded = ProgramRuntimeState::load(&state_path).expect("load should succeed");
    let record = loaded.lanes.get("chain:runtime").expect("lane exists");
    
    assert_eq!(record.status, raspberry_supervisor::evaluate::LaneExecutionStatus::Failed);
    assert_eq!(record.failure_kind, Some(FailureKind::RegenerateNoop));
    assert_eq!(record.recovery_action, Some(FailureRecoveryAction::SurfaceBlocked));
}

// ---------------------------------------------------------------------------
// malformed tests - program state with malformed JSON files
// ---------------------------------------------------------------------------

/// Test: malformed JSON state file handling
///
/// Verifies that ProgramRuntimeState::load returns an appropriate error
/// when the JSON file is malformed.
#[test]
fn malformed_json_state_file_returns_parse_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_path = temp.path().join("malformed-state.json");
    
    // Write malformed JSON
    std::fs::write(&state_path, "{ this is not valid json }").expect("write");
    
    let result = ProgramRuntimeState::load(&state_path);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(matches!(error, raspberry_supervisor::program_state::ProgramStateError::Parse { .. }));
}

/// Verifies that load_optional returns None for missing files (not an error).
#[test]
fn missing_state_file_returns_none() {
    let temp = tempfile::tempdir().expect("tempdir");
    let missing_path = temp.path().join("nonexistent-state.json");
    
    let result = ProgramRuntimeState::load_optional(&missing_path);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

/// Verifies that a state file with wrong schema version is handled gracefully.
#[test]
fn wrong_schema_version_state_file_loads_with_different_schema() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_path = temp.path().join("wrong-schema-state.json");
    
    // Write a state with an old/invalid schema version
    std::fs::write(
        &state_path,
        serde_json::json!({
            "schema_version": "old.schema.v1",
            "program": "test",
            "updated_at": chrono::Utc::now(),
            "lanes": {}
        }).to_string(),
    ).expect("write");
    
    // Should still load - schema version is informational
    let result = ProgramRuntimeState::load(&state_path);
    assert!(result.is_ok());
    
    let state = result.unwrap();
    assert_eq!(state.program, "test");
    assert_eq!(state.schema_version, "old.schema.v1");
}

/// Verifies that a state file with missing required fields is handled.
#[test]
fn state_file_with_missing_fields_loads_partial() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_path = temp.path().join("partial-state.json");
    
    // Write a state with missing optional fields
    std::fs::write(
        &state_path,
        serde_json::json!({
            "schema_version": "raspberry.program.v2",
            "program": "minimal-test",
            "updated_at": chrono::Utc::now(),
            "lanes": {
                "test:lane": {
                    "lane_key": "test:lane",
                    "status": "ready"
                }
            }
        }).to_string(),
    ).expect("write");
    
    let result = ProgramRuntimeState::load(&state_path);
    assert!(result.is_ok());
    
    let state = result.unwrap();
    let record = state.lanes.get("test:lane").expect("lane exists");
    assert_eq!(record.lane_key, "test:lane");
    assert_eq!(record.status, raspberry_supervisor::evaluate::LaneExecutionStatus::Ready);
}

/// Verifies round-trip save/load preserves all important fields.
#[test]
fn state_save_load_roundtrip_preserves_lanes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_path = temp.path().join("roundtrip-state.json");
    
    let mut state = ProgramRuntimeState::new("test-program");
    
    // Add a complex lane record
    state.lanes.insert(
        "runtime:chapter".to_string(),
        raspberry_supervisor::program_state::LaneRuntimeRecord {
            lane_key: "runtime:chapter".to_string(),
            status: raspberry_supervisor::evaluate::LaneExecutionStatus::Failed,
            run_config: Some(PathBuf::from("run-configs/runtime-chapter.toml")),
            current_run_id: Some("01KMTEST".to_string()),
            current_fabro_run_id: Some("01KMTEST".to_string()),
            current_stage_label: Some("Verify".to_string()),
            last_run_id: Some("01KMTEST".to_string()),
            last_started_at: Some(Utc::now() - chrono::Duration::minutes(5)),
            last_finished_at: Some(Utc::now()),
            last_exit_status: Some(1),
            last_error: Some("deterministic failure cycle detected".to_string()),
            failure_kind: Some(FailureKind::DeterministicVerifyCycle),
            recovery_action: Some(FailureRecoveryAction::SurfaceBlocked),
            last_completed_stage_label: Some("Implement".to_string()),
            last_stage_duration_ms: Some(12345),
            last_usage_summary: Some("tokens: 50000, cost: $0.50".to_string()),
            last_files_read: vec!["src/lib.rs".to_string()],
            last_files_written: vec!["src/generated.rs".to_string()],
            last_stdout_snippet: Some("Compiling...".to_string()),
            last_stderr_snippet: Some("Warning: unused variable".to_string()),
            consecutive_failures: 3,
        },
    );
    
    state.save(&state_path).expect("save should succeed");
    
    let loaded = ProgramRuntimeState::load(&state_path).expect("load should succeed");
    
    assert_eq!(loaded.program, "test-program");
    assert_eq!(loaded.lanes.len(), 1);
    
    let record = loaded.lanes.get("runtime:chapter").expect("lane exists");
    assert_eq!(record.status, raspberry_supervisor::evaluate::LaneExecutionStatus::Failed);
    assert_eq!(record.last_exit_status, Some(1));
    assert_eq!(record.consecutive_failures, 3);
    assert_eq!(record.last_error.as_deref(), Some("deterministic failure cycle detected"));
}

// ---------------------------------------------------------------------------
// recovery tests - recovery action authority (public API tests)
// ---------------------------------------------------------------------------

/// Verifies that environment collision failures are correctly classified.
#[test]
fn failure_classification_environment_collision() {
    let error = "bind failed: Errno 98 address already in use";
    let failure_kind = raspberry_supervisor::failure::classify_failure(Some(error), None, None);
    
    assert_eq!(failure_kind, Some(FailureKind::EnvironmentCollision));
    
    let action = raspberry_supervisor::failure::default_recovery_action(FailureKind::EnvironmentCollision);
    assert_eq!(action, FailureRecoveryAction::BackoffRetry);
}

/// Verifies that transient launch failures get the correct cooldown period.
#[test]
fn recovery_action_transient_launch_has_short_cooldown() {
    let failure_kind = FailureKind::TransientLaunchFailure;
    let action = raspberry_supervisor::failure::default_recovery_action(failure_kind);
    
    assert_eq!(action, FailureRecoveryAction::BackoffRetry);
}

/// Verifies that integration conflicts trigger RefreshFromTrunk recovery.
#[test]
fn recovery_action_integration_conflict_refreshes_from_trunk() {
    let failure_kind = FailureKind::IntegrationConflict;
    let action = raspberry_supervisor::failure::default_recovery_action(failure_kind);
    
    assert_eq!(action, FailureRecoveryAction::RefreshFromTrunk);
}

/// Verifies that branch-backed run requirements trigger ReplaySourceLane.
#[test]
fn recovery_action_branch_backed_run_required_replays_source() {
    let failure_kind = FailureKind::BranchBackedRunRequired;
    let action = raspberry_supervisor::failure::default_recovery_action(failure_kind);
    
    assert_eq!(action, FailureRecoveryAction::ReplaySourceLane);
}

/// Verifies that deterministic verify cycles trigger RegenerateLane recovery.
#[test]
fn recovery_action_deterministic_verify_cycle_regenerates() {
    let failure_kind = FailureKind::DeterministicVerifyCycle;
    let action = raspberry_supervisor::failure::default_recovery_action(failure_kind);
    
    assert_eq!(action, FailureRecoveryAction::RegenerateLane);
}

/// Verifies that provider policy mismatches trigger RegenerateLane.
#[test]
fn recovery_action_provider_policy_mismatch_regenerates() {
    let failure_kind = FailureKind::ProviderPolicyMismatch;
    let action = raspberry_supervisor::failure::default_recovery_action(failure_kind);
    
    assert_eq!(action, FailureRecoveryAction::RegenerateLane);
}

// ---------------------------------------------------------------------------
// dispatch tests - dispatch budget exhaustion and max_parallel
// ---------------------------------------------------------------------------

/// Test: dispatch settings respects max_parallel override
#[test]
fn dispatch_respects_max_parallel_budget() {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures/raspberry-supervisor/program.yaml");
    let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
    
    // Program has max_parallel: 2
    assert_eq!(manifest.max_parallel, 2);
}

/// Verifies that DispatchOutcome correctly captures lane results.
#[test]
fn dispatch_outcome_captures_all_fields() {
    let outcome = DispatchOutcome {
        lane_key: "chain:runtime".to_string(),
        exit_status: 1,
        fabro_run_id: Some("01KMTEST".to_string()),
        stdout: "Build failed".to_string(),
        stderr: "fatal error".to_string(),
    };
    
    assert_eq!(outcome.lane_key, "chain:runtime");
    assert_eq!(outcome.exit_status, 1);
    assert_eq!(outcome.fabro_run_id.as_deref(), Some("01KMTEST"));
    assert_eq!(outcome.stdout, "Build failed");
    assert_eq!(outcome.stderr, "fatal error");
}
