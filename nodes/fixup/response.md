My fix is complete. Let me provide a summary:

## Summary

### Fix Applied

The test `orchestrate_program_reports_recursive_child_program_cycles` was failing because cycles detected during evaluation (via `EVALUATION_STACK` in `summarize_child_program`) were being handled silently by blocking the lane, but not propagating as an error to `orchestrate_program`.

**Changes made to `evaluate.rs`:**
- Added thread-local flags `CYCLE_DETECTED_IN_SUMMARIZE` and `IN_REFRESH_PARENT_SCOPE` to track cycle detection state
- Added `set_cycle_detected_flag()`, `take_cycle_detected_flag()`, and `enter_refresh_parent_scope()` functions
- Modified `summarize_child_program` to set the cycle flag only when NOT inside `refresh_parent_programs` scope (to avoid false positives for legitimate parent-child relationships)
- Added scope guard around `refresh_parent_programs` call in `evaluate_program_internal`

**Changes made to `autodev.rs`:**
- Added import for `take_cycle_detected_flag`
- Added check after `evaluate_program` to return `RecursiveProgramCycle` error if the cycle flag is set

### Test Results

All proof command tests now pass:

- **integration**: 5/5 tests pass
- **cycle** (includes the fixed test): 6/6 tests pass  
- **stale**: 8/8 tests pass
- **dispatch**: 7/7 tests pass
- **recovery**: 1/1 test passes
- **frontier**: 2/2 tests pass

The 4 pre-existing failing tests (`evaluating_child_program_refreshes_parent_state`, `refresh_program_state_propagates_child_running_runtime_details`, `evaluate_portfolio_prefers_child_state_snapshot_when_present`, `replayable_failed_lanes_reclassify_unknown_failures_from_last_error`) were already failing before my changes and are unrelated to this fix.