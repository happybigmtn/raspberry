# Autodev Execution Path and Dispatch Truth — Review

**Lane:** `autodev-efficiency-and-dispatch`
**Date:** 2026-03-26
**Status:** Initial Review — First Slice

## Summary

This review assesses the current state of the autodev execution path and dispatch truth mechanisms in the `raspberry-supervisor` crate. The goal is to identify what is working, what is broken, and what needs verification before live validation on a proving-ground repo.

**Bottom line:** The execution path has solid foundations in `program_state.rs` and `evaluate.rs`, but there are three categories of issues that must be addressed before autodev can sustain 10 active lanes without manual rescue:

1. **Command surface exposure** — `fabro synth` may not be reliably exposed in all build configurations
2. **Prompt resolution correctness** — generated workflow graphs use relative paths that resolve incorrectly at runtime
3. **Dispatch telemetry gaps** — the current reporting does not explain why ready work did or did not run

## What Is Working

### Lane State Management (`program_state.rs`)

The `refresh_program_state()` function is well-structured and handles the full state reconciliation lifecycle:

- **Stale running detection**: `stale_active_progress_reason()` exists and checks for:
  - Missing run directory (`run_dir` does not exist)
  - No worker process (`worker_alive = Some(false)`)
  - Run status showing failure or death
  - Updated timestamp older than `STALE_RUNNING_GRACE_SECS` (30 seconds)

- **Failure classification**: `classify_failure()` maps stderr/stdout patterns to `FailureKind` variants, with appropriate `FailureRecoveryAction` defaults

- **State persistence**: `ProgramRuntimeState` uses atomic writes via `write_atomic()` to prevent corruption

- **Child program sync**: `sync_child_program_runtime_record()` recursively syncs child program state into parent lane records

### Lane Evaluation (`evaluate.rs`)

The evaluation logic correctly determines lane readiness:

- **Dependency satisfaction**: `dependencies_satisfied()` checks milestone keys properly
- **Active run detection**: `is_active()` checks both `run_snapshot` status and `runtime_record` for running state with no finish timestamp
- **Child program classification**: `classify_child_program_lane()` aggregates child state correctly

### Dispatch (`dispatch.rs`)

The dispatch path is mostly sound:

- **Target repo freshness check**: `ensure_target_repo_fresh_for_dispatch()` prevents dispatching to stale branches
- **Parallel execution**: Lanes are chunked by `max_parallel` and run in parallel threads
- **Maintenance mode guard**: `execute_selected_lanes()` checks for active maintenance before dispatching
- **Thread safety**: All worker threads are joined before processing results, preventing orphaned processes

### Autodev Orchestration (`autodev.rs`)

The main loop structure is correct:

- **Doctrine change detection**: `doctrine_inputs_changed()` tracks file fingerprints
- **Evolve triggering**: `should_trigger_evolve()` has logic to fast-track regeneration and avoid evolve when budget is met
- **Report generation**: `AutodevReport` captures cycle-by-cycle state with good fidelity

## Issues to Address

### Issue 1: Command Surface Exposure (Medium Severity)

**Problem**: The live failure on 2026-03-26 showed rXMRbro failing because a clean `fabro` binary did not expose `synth`. The `fabro-cli/src/main.rs` does define the `Synth` command with subcommands (`Import`, `Create`, `Evolve`, `Review`, `Genesis`), but there may be build configuration issues preventing exposure.

**Evidence**: The `Command::Synth { command }` match arm exists and handles all subcommands, but we need verification that:
1. The `synth` feature flag is enabled in both debug and release builds
2. The `fabro-synthesis` crate is properly linked

**Recommended Action**: Add a startup probe that validates command surface:

```rust
// In autodev.rs or a new validation module
pub fn validate_fabro_command_surface(fabro_bin: &Path) -> Result<(), AutodevError> {
    let required_commands = ["run", "validate", "synth"];
    for cmd in required_commands {
        let output = Command::new(fabro_bin)
            .arg(cmd)
            .arg("--help")
            .output()?;
        if !output.status.success() {
            return Err(AutodevError::CommandNotFound {
                command: cmd.to_string(),
                path: fabro_bin.display().to_string(),
            });
        }
    }
    Ok(())
}
```

**File to modify**: `lib/crates/raspberry-supervisor/src/autodev.rs` — add validation at loop start

### Issue 2: Prompt Resolution in Generated Workflows (High Severity)

**Problem**: Generated `graph.fabro` files use relative paths like `@../../prompts/...` that resolve under `~/.fabro/` at runtime instead of the target repo. This causes bootstrap validation failures when autodev dispatches lanes.

**Evidence**: The `render.rs` module in `fabro-synthesis` generates workflow graphs but the prompt path resolution logic needs verification. The `render_workflow()` function should be using run-directory-relative paths or absolute paths.

**Recommended Action**: In `fabro-synthesis/src/render.rs`, ensure generated workflow graphs use:
1. Absolute paths to the target repo's `malinka/prompts/` directory, OR
2. Paths relative to the run directory (where the workflow will execute)

**Verification command**:
```bash
# After building, check that generated workflow graphs don't use @../../prompts/...
grep -r '@../../prompts' malinka/run-configs/
```

### Issue 3: Dispatch Telemetry Gaps (Medium Severity)

**Problem**: The current `AutodevCycleReport` struct has fields like `dispatched`, `running_after`, `complete_after`, but lacks explicit telemetry that explains why ready work did or did not run:

- `dispatch_rate` — ratio of dispatched to available slots
- `idle_cycles` — consecutive cycles with zero dispatches
- `ready_but_undispatched` — ready lanes not dispatched due to budget
- `failed_bootstrap_count` — lanes failing bootstrap validation
- `runtime_path_errors` — lanes failing due to missing commands/assets
- `stale_running_reclaimed` — running lanes reclassified as failed

**Recommended Action**: Add a `DispatchTelemetry` struct to `autodev.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchTelemetry {
    pub dispatch_rate: f64,
    pub idle_cycles: u32,
    pub ready_but_undispatched: usize,
    pub failed_bootstrap_count: usize,
    pub runtime_path_errors: usize,
    pub stale_running_reclaimed: usize,
}
```

Update `AutodevCycleReport` to include this telemetry.

**File to modify**: `lib/crates/raspberry-supervisor/src/autodev.rs`

### Issue 4: Blocking `synth evolve` on Hot Path (Low-Medium Severity)

**Problem**: `run_synth_evolve()` runs synchronously before dispatch in the main loop. This delays dispatch even when ready work exists and evolve is not needed.

**Current flow**:
```
[refresh] → [evaluate] → [evolve (blocking)] → [dispatch] → [watch]
```

**Target flow**:
```
[refresh] → [evaluate] → [dispatch] → [watch]
                    ↓ (background, cadence-gated)
              [evolve (async)]
```

**Recommended Action**: Move evolve to a background thread with cadence gating. The dispatch cycle should not wait for evolve unless a doctrine change is detected and the frontier is settled.

**Complexity**: Medium — requires thread-safe state sharing between evolve thread and dispatch thread.

**File to modify**: `lib/crates/raspberry-supervisor/src/autodev.rs`

## Verification Checklist

Before live validation on rXMRbro:

- [ ] Verify `fabro synth --help` works in both debug and release builds
- [ ] Verify `fabro run --detach` works for a lane with prompt references
- [ ] Verify generated workflow graphs use correct prompt paths
- [ ] Verify stale running detection fires within 30 seconds
- [ ] Verify dispatch telemetry fields are populated correctly
- [ ] Run `cargo build --release -p fabro-cli -p raspberry-cli`
- [ ] Run `raspberry autodev --manifest <path> --max-parallel 10 --max-cycles 20`
- [ ] Verify controller sustains 10 running lanes for 20 cycles
- [ ] Verify zero bootstrap validation failures in first 10 cycles

## Risks and Mitigations

### Risk: Build Configuration Drift

If `fabro-synthesis` is not properly linked, the `synth` command will silently fail.

**Mitigation**: Add startup validation that probes all required commands and fails fast with actionable error.

### Risk: Prompt Resolution Still Broken After Fix

If the fix to prompt resolution is incomplete, lanes will still fail bootstrap.

**Mitigation**: Add a pre-dispatch validation step that runs `fabro validate <run-config>` and fails the lane with `FailureKind::RuntimePathError` if validation fails.

### Risk: Evolve Decoupling Causes Race Conditions

Moving evolve to a background thread could cause race conditions with dispatch.

**Mitigation**: Use a channel-based communication between evolve thread and dispatch thread. Dispatch should check evolve status but not block on it.

## Open Questions

1. **Is `fabro-synthesis` linked in all build configurations?** Need to verify `Cargo.toml` feature flags.

2. **What is the exact prompt resolution path in generated workflows?** Need to trace through `render.rs` to find where `@../../prompts/` paths are generated.

3. **Should stale running detection use a shorter or longer grace period?** Current is 30 seconds. Should this be configurable?

4. **Should dispatch telemetry be persisted to the report file or only emitted to stdout?** Currently reports are JSON files; adding telemetry fields is additive.

## Recommendations

1. **Priority 1**: Add command surface validation at autodev startup — prevents silent failures
2. **Priority 2**: Fix prompt resolution in generated workflow graphs — enables bootstrap success
3. **Priority 3**: Add dispatch telemetry fields — enables operational visibility
4. **Priority 4**: Decouple evolve from dispatch — improves dispatch responsiveness
5. **Priority 5**: Live validation on proving-ground repo — proves the execution path is boring

## Conclusion

The autodev execution path has good foundations in `program_state.rs` and `evaluate.rs`. The stale running detection logic is sound, the failure classification is well-structured, and the dispatch path is mostly correct. The three main issues are:

1. Command surface exposure verification (medium effort, high value)
2. Prompt resolution correctness (medium effort, high value)
3. Dispatch telemetry gaps (low effort, medium value)

The blocking evolve issue is lower priority but would improve dispatch responsiveness once the execution path is stable.

**Recommended next step**: Address Issue 1 (command surface validation) first, then verify Issue 2 (prompt resolution) by inspecting generated workflow graphs, then add Issue 3 (dispatch telemetry) for operational visibility. These three changes should enable successful live validation.
