Goal: Raspberry Supervisor Edge Case Tests

Child work item of plan: Test Coverage for Critical Paths

Proof commands:
- `cargo nextest run -p raspberry-supervisor -- integration autodev_cycle`
- `cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed`

Required durable artifacts:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`


## Completed stages
- **preflight**: success
  - Script: `set +e
if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p raspberry-supervisor -- integration autodev_cycle && cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
else
  cargo test -p raspberry-supervisor -- integration autodev_cycle && cargo test -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
fi
true`
  - Stdout: (empty)
  - Stderr:
    ```
    (16 lines omitted)
            PASS [   0.005s] (4/5) raspberry-supervisor autodev::tests::replayable_failed_lanes_replay_source_lane_for_failed_integration_program
            FAIL [   0.505s] (5/5) raspberry-supervisor integration::tests::integrate_lane_squash_merges_run_branch_into_trunk
      stdout ───
    
        running 1 test
        test integration::tests::integrate_lane_squash_merges_run_branch_into_trunk ... FAILED
    
        failures:
    
        failures:
            integration::tests::integrate_lane_squash_merges_run_branch_into_trunk
    
        test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 118 filtered out; finished in 0.50s
    
      stderr ───
    
        thread 'integration::tests::integrate_lane_squash_merges_run_branch_into_trunk' (3409727) panicked at lib/crates/raspberry-supervisor/src/integration.rs:268:10:
        integration succeeds: Direct(Git { step: "resolve ssh push url", repo: "/home/r/.cache/rust-tmp/fabro-direct-integration-29bb3659-50a0-476e-a9dc-544753f9b49a", message: "Engine error: remote `origin` must use SSH or be convertible from GitHub HTTPS, got `/home/r/.cache/rust-tmp/.tmphG3RbA/remote.git`" })
        note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    
      Cancelling due to test failure: 
    ────────────
         Summary [   0.505s] 5 tests run: 4 passed, 1 failed, 114 skipped
            FAIL [   0.505s] (5/5) raspberry-supervisor integration::tests::integrate_lane_squash_merges_run_branch_into_trunk
    error: test run failed
    ```
- **contract**: success
  - Model: MiniMax-M2.7-highspeed, 44.5k tokens in / 230 out
- **implement**: success
  - Model: MiniMax-M2.7-highspeed, 128.0k tokens in / 506 out
  - Files: docs/rpi-edge-case-tests.md, lib/crates/raspberry-supervisor/src/integration.rs, test/fixtures/raspberry-supervisor/.raspberry/myosu-program-state.json, test/fixtures/raspberry-supervisor/.raspberry/program-state.json
- **verify**: success
  - Script: `if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p raspberry-supervisor -- integration autodev_cycle && cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
else
  cargo test -p raspberry-supervisor -- integration autodev_cycle && cargo test -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
fi`
  - Stdout: (empty)
  - Stderr:
    ```
    (16 lines omitted)
            PASS [   0.004s] ( 2/24) raspberry-supervisor autodev::tests::regenerable_failures_trigger_evolve_once_frontier_changes
            PASS [   0.005s] ( 3/24) raspberry-supervisor failure::tests::classify_failure_detects_deterministic_failure_cycles
            PASS [   0.005s] ( 4/24) raspberry-supervisor autodev::tests::cycle_limit_treats_zero_as_unbounded
            PASS [   0.005s] ( 5/24) raspberry-supervisor autodev::tests::ready_lane_dispatch_diversifies_initial_foundation_wave
            PASS [   0.005s] ( 6/24) raspberry-supervisor evaluate::tests::stale_runtime_complete_does_not_satisfy_managed_milestone_without_artifacts
            PASS [   0.006s] ( 7/24) raspberry-supervisor dispatch::tests::execute_selected_lanes_refuses_dispatch_during_maintenance
            PASS [   0.006s] ( 8/24) raspberry-supervisor controller_lease::tests::acquire_autodev_lease_reclaims_stale_owner
            PASS [   0.006s] ( 9/24) raspberry-supervisor failure::tests::classify_failure_detects_node_visit_cycle_limit_failures
            PASS [   0.006s] (10/24) raspberry-supervisor program_state::tests::merge_failure_detail_prefers_underlying_stage_failure_over_cycle_wrapper
            PASS [   0.007s] (11/24) raspberry-supervisor autodev::tests::has_more_cycles_respects_bounded_and_unbounded_limits
            PASS [   0.007s] (12/24) raspberry-supervisor autodev::tests::dispatchable_failed_lanes_selects_regenerable_failures_after_evolve
            PASS [   0.008s] (13/24) raspberry-supervisor autodev::tests::replayable_failed_lanes_dispatch_codex_unblock_for_regenerate_noop
            PASS [   0.008s] (14/24) raspberry-supervisor failure::tests::deterministic_recovery_actions_regenerate_lane
            PASS [   0.003s] (15/24) raspberry-supervisor program_state::tests::sync_program_state_with_evaluated_clears_stale_failure_residue_for_complete_lane
            PASS [   0.004s] (16/24) raspberry-supervisor program_state::tests::stale_active_progress_marks_run_as_stale
            PASS [   0.004s] (17/24) raspberry-supervisor program_state::tests::sync_program_state_with_evaluated_clears_stale_failure_residue_for_ready_lane
            PASS [   0.005s] (18/24) raspberry-supervisor resource_lease::tests::acquire_zend_daemon_lease_reclaims_stale_owner
            PASS [   0.015s] (19/24) raspberry-supervisor autodev::tests::orchestrate_program_reports_recursive_child_program_cycles
            PASS [   0.057s] (20/24) raspberry-supervisor autodev::tests::ensure_target_repo_fresh_for_dispatch_blocks_dirty_repo_that_is_behind
            PASS [   0.061s] (21/24) raspberry-supervisor autodev::tests::ensure_target_repo_fresh_for_dispatch_fast_forwards_with_only_untracked_noise
            PASS [   0.064s] (22/24) raspberry-supervisor autodev::tests::ensure_target_repo_fresh_for_dispatch_fast_forwards_clean_default_branch
            PASS [   0.876s] (23/24) raspberry-supervisor program_state::tests::refresh_program_state_marks_missing_running_run_as_stale_failure
            PASS [   0.917s] (24/24) raspberry-supervisor program_state::tests::refresh_program_state_clears_stale_failure_residue_for_succeeded_run_progress
    ────────────
         Summary [   0.918s] 24 tests run: 24 passed, 95 skipped
    ```
- **fixup**: success
  - Model: MiniMax-M2.7-highspeed, 108.7k tokens in / 677 out
  - Files: lib/crates/raspberry-supervisor/src/autodev.rs, lib/crates/raspberry-supervisor/src/evaluate.rs, test/fixtures/raspberry-supervisor/.raspberry/myosu-program-state.json, test/fixtures/raspberry-supervisor/.raspberry/program-state.json
- **verify**: success
  - Script: `if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p raspberry-supervisor -- integration autodev_cycle && cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
else
  cargo test -p raspberry-supervisor -- integration autodev_cycle && cargo test -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
fi`
  - Stdout: (empty)
  - Stderr:
    ```
    (16 lines omitted)
            PASS [   0.004s] ( 2/24) raspberry-supervisor autodev::tests::regenerable_failures_trigger_evolve_once_frontier_changes
            PASS [   0.005s] ( 3/24) raspberry-supervisor failure::tests::classify_failure_detects_deterministic_failure_cycles
            PASS [   0.005s] ( 4/24) raspberry-supervisor autodev::tests::cycle_limit_treats_zero_as_unbounded
            PASS [   0.005s] ( 5/24) raspberry-supervisor autodev::tests::ready_lane_dispatch_diversifies_initial_foundation_wave
            PASS [   0.005s] ( 6/24) raspberry-supervisor evaluate::tests::stale_runtime_complete_does_not_satisfy_managed_milestone_without_artifacts
            PASS [   0.006s] ( 7/24) raspberry-supervisor dispatch::tests::execute_selected_lanes_refuses_dispatch_during_maintenance
            PASS [   0.006s] ( 8/24) raspberry-supervisor controller_lease::tests::acquire_autodev_lease_reclaims_stale_owner
            PASS [   0.006s] ( 9/24) raspberry-supervisor failure::tests::classify_failure_detects_node_visit_cycle_limit_failures
            PASS [   0.006s] (10/24) raspberry-supervisor program_state::tests::merge_failure_detail_prefers_underlying_stage_failure_over_cycle_wrapper
            PASS [   0.007s] (11/24) raspberry-supervisor autodev::tests::has_more_cycles_respects_bounded_and_unbounded_limits
            PASS [   0.007s] (12/24) raspberry-supervisor autodev::tests::dispatchable_failed_lanes_selects_regenerable_failures_after_evolve
            PASS [   0.008s] (13/24) raspberry-supervisor autodev::tests::replayable_failed_lanes_dispatch_codex_unblock_for_regenerate_noop
            PASS [   0.008s] (14/24) raspberry-supervisor failure::tests::deterministic_recovery_actions_regenerate_lane
            PASS [   0.003s] (15/24) raspberry-supervisor program_state::tests::sync_program_state_with_evaluated_clears_stale_failure_residue_for_complete_lane
            PASS [   0.004s] (16/24) raspberry-supervisor program_state::tests::stale_active_progress_marks_run_as_stale
            PASS [   0.004s] (17/24) raspberry-supervisor program_state::tests::sync_program_state_with_evaluated_clears_stale_failure_residue_for_ready_lane
            PASS [   0.005s] (18/24) raspberry-supervisor resource_lease::tests::acquire_zend_daemon_lease_reclaims_stale_owner
            PASS [   0.015s] (19/24) raspberry-supervisor autodev::tests::orchestrate_program_reports_recursive_child_program_cycles
            PASS [   0.057s] (20/24) raspberry-supervisor autodev::tests::ensure_target_repo_fresh_for_dispatch_blocks_dirty_repo_that_is_behind
            PASS [   0.061s] (21/24) raspberry-supervisor autodev::tests::ensure_target_repo_fresh_for_dispatch_fast_forwards_with_only_untracked_noise
            PASS [   0.064s] (22/24) raspberry-supervisor autodev::tests::ensure_target_repo_fresh_for_dispatch_fast_forwards_clean_default_branch
            PASS [   0.876s] (23/24) raspberry-supervisor program_state::tests::refresh_program_state_marks_missing_running_run_as_stale_failure
            PASS [   0.917s] (24/24) raspberry-supervisor program_state::tests::refresh_program_state_clears_stale_failure_residue_for_succeeded_run_progress
    ────────────
         Summary [   0.918s] 24 tests run: 24 passed, 95 skipped
    ```
- **quality**: fail
  - Script: `set -e
QUALITY_PATH='outputs/test-coverage-critical-paths-raspberry-supervisor-edge-case-tests/quality.md'
IMPLEMENTATION_PATH='outputs/test-coverage-critical-paths-raspberry-supervisor-edge-case-tests/implementation.md'
VERIFICATION_PATH='outputs/test-coverage-critical-paths-raspberry-supervisor-edge-case-tests/verification.md'
placeholder_hits=""
scan_placeholder() {
  surface="$1"
  if [ ! -e "$surface" ]; then
    return 0
  fi
  if [ -f "$surface" ]; then
    surface="$(dirname "$surface")"
  fi
  hits="$(rg -n -i -g '*.rs' -g '*.py' -g '*.js' -g '*.ts' -g '*.tsx' -g '*.md' -g 'Cargo.toml' -g '*.toml' 'TODO|stub|placeholder|not yet implemented|compile-only|for now|will implement|todo!|unimplemented!' "$surface" || true)"
  if [ -n "$hits" ]; then
    if [ -n "$placeholder_hits" ]; then
      placeholder_hits="$(printf '%s\n%s' "$placeholder_hits" "$hits")"
    else
      placeholder_hits="$hits"
    fi
  fi
}
true
external_blocker_only=no
root_artifact_hits=""
for shadow in spec.md review.md implementation.md verification.md quality.md promotion.md integration.md; do
  if [ -f "$shadow" ]; then
    root_artifact_hits="$root_artifact_hits\n$shadow"
  fi
done
semantic_risk_hits="$(rg -n -i -g '*.rs' 'payout_multiplier\(\)\s+as\s+i16|numerator\s+as\s+i16|deterministic placeholder|spin made without seed being set|house doesn.t play - the player spins|Generate seed \(in real impl, comes from house via action_seed\)' . 2>/dev/null || true)"
lane_sizing_hits=""
for surface in .; do
  if [ -d "$surface" ]; then
    while IFS= read -r file; do
      lines=$(wc -l < "$file" 2>/dev/null || echo 0)
      if [ "$lines" -lt 400 ]; then
        continue
      fi
      if rg -q 'handle_input' "$file" 2>/dev/null && rg -q 'render_' "$file" 2>/dev/null && rg -q 'tick\(|ui_state|session_pnl' "$file" 2>/dev/null; then
        lane_sizing_hits="$lane_sizing_hits\n$file:$lines"
      fi
    done < <(find "$surface" -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' \) 2>/dev/null)
  fi
done
artifact_hits="$(rg -n -i 'manual proof still required|placeholder|stub implementation|not yet fully implemented|todo!|unimplemented!' "$IMPLEMENTATION_PATH" "$VERIFICATION_PATH" 2>/dev/null || true)"
test_quality_debt=no
for surface in .; do
  if [ -d "$surface" ]; then
    total_tests=$(rg -c '#\[test\]' -g '*.rs' "$surface" 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
    derive_tests=$(rg -c 'assert.*\.to_string\(\).*contains\|assert_eq!.*\.to_string\(\)\|assert_eq!.*format!.*Display' -g '*.rs' "$surface" 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
    if [ "$total_tests" -gt 5 ] && [ "$derive_tests" -gt 0 ]; then
      ratio=$((derive_tests * 100 / total_tests))
      if [ "$ratio" -gt 50 ]; then
        test_quality_debt=yes
      fi
    fi
  fi
done
warning_hits="$(rg -n 'warning:' "$IMPLEMENTATION_PATH" "$VERIFICATION_PATH" 2>/dev/null || true)"
manual_hits="$(rg -n -i 'manual proof still required|manual;' "$VERIFICATION_PATH" 2>/dev/null || true)"
placeholder_debt=no
warning_debt=no
artifact_mismatch_risk=no
manual_followup_required=no
semantic_risk_debt=no
lane_sizing_debt=no
[ -n "$placeholder_hits" ] && placeholder_debt=yes
if [ "$external_blocker_only" = no ] && [ -n "$warning_hits" ]; then warning_debt=yes; fi
if [ -n "$artifact_hits" ] || [ -n "$root_artifact_hits" ]; then artifact_mismatch_risk=yes; fi
if [ "$external_blocker_only" = no ] && [ -n "$manual_hits" ]; then manual_followup_required=yes; fi
[ -n "$semantic_risk_hits" ] && semantic_risk_debt=yes
[ -n "$lane_sizing_hits" ] && lane_sizing_debt=yes
quality_ready=yes
if [ "$placeholder_debt" = yes ] || [ "$warning_debt" = yes ] || [ "$artifact_mismatch_risk" = yes ] || [ "$manual_followup_required" = yes ] || [ "$semantic_risk_debt" = yes ] || [ "$lane_sizing_debt" = yes ] || [ "$test_quality_debt" = yes ]; then
  quality_ready=no
fi
mkdir -p "$(dirname "$QUALITY_PATH")"
cat > "$QUALITY_PATH" <<EOF
quality_ready: $quality_ready
placeholder_debt: $placeholder_debt
warning_debt: $warning_debt
test_quality_debt: $test_quality_debt
artifact_mismatch_risk: $artifact_mismatch_risk
manual_followup_required: $manual_followup_required
semantic_risk_debt: $semantic_risk_debt
lane_sizing_debt: $lane_sizing_debt
external_blocker_only: $external_blocker_only

## Touched Surfaces
- (none declared)

## Placeholder Hits
$placeholder_hits

## Artifact Consistency Hits
$artifact_hits

## Root Artifact Shadow Hits
$root_artifact_hits

## Semantic Risk Hits
$semantic_risk_hits

## Lane Sizing Hits
$lane_sizing_hits

## Warning Hits
$warning_hits

## Manual Followup Hits
$manual_hits
EOF
test "$quality_ready" = yes

if [ -f .fabro-work/contract.md ]; then
  rm -f .fabro-work/.contract-missing
  sed -n '/^## Deliverables/,/^## /p' .fabro-work/contract.md | grep '^- ' | while IFS= read -r line; do
    cfile=$(echo "$line" | sed 's/^- //' | sed 's/`//g' | tr -d ' ')
    if [ -n "$cfile" ] && echo "$cfile" | grep -qE '\.(rs|ts|tsx|js|py|go|sol|rb|json|toml|yaml|yml)$'; then
      if [ ! -e "$cfile" ]; then
        echo "$cfile" >> .fabro-work/.contract-missing
      fi
    fi
  done
  if [ -f .fabro-work/.contract-missing ]; then
    echo '## Contract Deliverables Missing' >> "$QUALITY_PATH"
    cat .fabro-work/.contract-missing >> "$QUALITY_PATH"
    rm -f .fabro-work/.contract-missing
    quality_ready=no
  fi
fi`
  - Stdout: (empty)
  - Stderr: (empty)

## Context
- failure_class: deterministic
- failure_signature: quality|deterministic|script failed with exit code: <n>


# Raspberry Supervisor Edge Case Tests Lane — Fixup

Fix only the current slice for `test-coverage-critical-paths-raspberry-supervisor-edge-case-tests`.


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Priorities:
- unblock the active slice's first proof gate — this is the #1 priority
- prefer staying within the named slice and touched surfaces
- if the proof gate fails on pre-existing issues OUTSIDE your surfaces (e.g., linter warnings in unrelated files, missing imports in dependencies), you MUST fix those issues minimally to unblock the gate — do not leave the lane stuck on problems you can solve
- preserve setup constraints before expanding implementation scope
- keep implementation and verification artifacts durable and specific
- do not create or rewrite `.fabro-work/promotion.md` during Fixup; that file is owned by the Review stage
- do not hand-author `.fabro-work/quality.md`; the Quality Gate rewrites it after verification
- ALL ephemeral files (quality.md, promotion.md, verification.md) go in `.fabro-work/`, never the repo root
