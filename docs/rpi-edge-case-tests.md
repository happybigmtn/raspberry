# Raspberry Pi Edge Case Tests

This document describes the edge case tests implemented for the `raspberry-supervisor` crate.

## Overview

The edge case tests verify critical failure modes and boundary conditions in the autodev dispatch cycle, stale state detection, recovery mechanisms, cycle limits, and frontier budget management.

## Test Categories

### Stale State Tests

Tests for detecting and handling stale program state:

- `stale_active_progress_marks_run_as_stale` - Verifies that runs with stale active progress are properly detected
- `sync_program_state_with_evaluated_clears_stale_failure_residue_for_ready_lane` - Tests stale failure cleanup for ready lanes
- `sync_program_state_with_evaluated_clears_stale_failure_residue_for_complete_lane` - Tests stale failure cleanup for complete lanes
- `refresh_program_state_clears_stale_failure_residue_for_succeeded_run_progress` - Verifies cleanup after successful runs
- `refresh_program_state_marks_missing_running_run_as_stale_failure` - Detects missing running runs as stale
- `stale_runtime_complete_does_not_satisfy_managed_milestone_without_artifacts` - Ensures stale completions don't satisfy milestones
- `acquire_autodev_lease_reclaims_stale_owner` - Lease recovery for stale owners
- `acquire_zend_daemon_lease_reclaims_stale_owner` - Daemon lease recovery

### Dispatch Tests

Tests for dispatch behavior under various conditions:

- `ready_lane_dispatch_diversifies_initial_foundation_wave` - Diversification in initial dispatch
- `execute_selected_lanes_refuses_dispatch_during_maintenance` - Blocks dispatch during maintenance
- `dispatchable_failed_lanes_selects_regenerable_failures_after_evolve` - Selects regenerable failures
- `replayable_failed_lanes_dispatch_codex_unblock_for_regenerate_noop` - Codex unblocking
- `ensure_target_repo_fresh_for_dispatch_blocks_dirty_repo_that_is_behind` - Blocks dirty repos
- `ensure_target_repo_fresh_for_dispatch_fast_forwards_clean_default_branch` - Fast-forward for clean repos
- `ensure_target_repo_fresh_for_dispatch_fast_forwards_with_only_untracked_noise` - Handles untracked noise

### Recovery Tests

Tests for recovery from failures:

- `deterministic_recovery_actions_regenerate_lane` - Regeneration as recovery action

### Cycle Tests

Tests for cycle limit handling:

- `cycle_limit_treats_zero_as_unbounded` - Zero cycle limit means unbounded
- `has_more_cycles_respects_bounded_and_unbounded_limits` - Cycle limit respect
- `orchestrate_program_reports_recursive_child_program_cycles` - Detects recursive cycles

### Frontier Budget Tests

Tests for frontier budget accounting:

- `spare_capacity_evolve_stays_bounded_by_frontier_budget` - Evolution respects budget
- `regenerable_failures_trigger_evolve_once_frontier_changes` - Frontier changes trigger evolve

### Integration Tests

Tests for lane integration:

- `integrate_lane_squash_merges_run_branch_into_trunk` - Squash merge integration
- `classify_failure_detects_integration_conflicts` - Detects integration conflicts
- `classify_failure_detects_integration_target_unavailable` - Detects unavailable targets
- `integration_replay_targets_source_lane_when_run_was_branchless` - Replay for branchless runs
- `replayable_failed_lanes_replay_source_lane_for_failed_integration_program` - Replay for failed integrations

## Test Naming Conventions

Tests follow a descriptive naming pattern: `<concept>_<condition>_<expected_outcome>`

Examples:
- `stale_active_progress_marks_run_as_stale` - Concept: stale_active_progress, Condition: (none), Expected: marks_run_as_stale
- `ensure_target_repo_fresh_for_dispatch_blocks_dirty_repo_that_is_behind` - Concept: ensure_target_repo_fresh, Condition: dirty_repo_that_is_behind, Expected: blocks_dispatch

## Expected Behaviors

### Autodev Cycle Termination

The autodev cycle terminates when:
1. All lanes reach a terminal state (complete, failed, blocked)
2. A cycle limit is reached (if bounded)
3. A critical failure occurs that cannot be recovered

### Stale Target Repo Detection

A target repo is considered stale when:
1. The local branch is behind the remote
2. There are uncommitted changes that could conflict
3. A previously running dispatch has not made progress

### Dispatch Failure Recovery

When dispatch fails, the system:
1. Classifies the failure type (integration conflict, target unavailable, etc.)
2. Determines if regeneration is possible
3. Schedules replay if recoverable

### Frontier Budget Management

Frontier budget limits how much evolution can occur:
1. Spare capacity is allocated based on available slots
2. Evolution respects the frontier budget
3. Changes to frontier trigger re-evaluation

### Malformed Manifest Handling

When a manifest is malformed:
1. Parsing fails with a descriptive error
2. The lane is marked as failed
3. The error is recorded for debugging
