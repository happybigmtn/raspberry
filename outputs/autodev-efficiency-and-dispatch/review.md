# Autodev Execution Path and Dispatch Truth — Review

## Scope of This Review

This review evaluates the first slice of the autodev execution path and dispatch truth work. The lane is scoped to:

1. Runtime-path self-consistency (no local-only shims, prompt resolution from target repo)
2. Stale `running`/`failed` lane truth reconciliation before dispatch
3. Dispatch-state telemetry
4. Decoupling `synth evolve` from dispatch
5. Greedy dispatch consumption

## What the Codebase Already Does Well

### Stale Detection Infrastructure

The `program_state.rs` module already has sophisticated stale lane detection:

- `STALE_RUNNING_GRACE_SECS` (30s) for worker-process disappearance detection
- `ACTIVE_STALL_TIMEOUT_SECS` (1800s) for stall watchdog detection
- `stale_failure_superseded_by_render()` for stale failures that are superseded by re-renders
- `worker_process_alive()` using `/proc/<pid>/` on Linux
- `refresh_program_state()` already updates lane status to `Failed` with `TransientLaunchFailure` for stale runs

The `FailureKind` enum in `failure.rs` is comprehensive and covers all observed failure modes (17 variants with appropriate default recovery actions).

### Dispatch Selection Logic

The dispatch selection in `autodev.rs` is thoughtful:

- Family diversity constraint (`lane_root_plan_family`) prevents concentrating work from one plan in a single dispatch round
- Priority scoring (`lane_dispatch_priority_score`) boosts Phase 0 plans (autodev-efficiency, greenfield-bootstrap, etc.)
- Replay failures are selected before new ready lanes (`replayed_lanes` then `selected_ready_lanes`)
- `select_ready_lanes_for_dispatch` enforces the family diversity constraint

### Telemetry Infrastructure

`AutodevReport` and `AutodevCycleReport` provide a solid foundation for telemetry. The `current_snapshot` function already computes running counts, ready lanes, critical blockers, and nested child running lanes.

### CLI Registration

The `fabro synth` command tree is properly registered in `main.rs` with all subcommands (Import, Create, Evolve, Review, Genesis).

## Identified Gaps and Risks

### Gap 1: Stale Lane Reclassification Does Not Persist Through Evaluation

**Severity**: High — causes dispatch slot consumption by stale lanes.

**Description**: As analyzed in Section 2.1 of the spec, `refresh_program_state` correctly updates a stale lane's record to `Failed`, but the subsequent `evaluate_program` call re-classifies it as `Running` because `evaluate_lane` loads a fresh `run_snapshot` from the (absent) run directory and `classify_lane` uses `is_active(run_snapshot, runtime_record)` where the snapshot is empty/dne, falling through to the stale `runtime_record.status == Running` check.

**Evidence**:
- `evaluate.rs` lines ~830-840: `is_active` returns `true` when `run_snapshot.status` is `None` and `runtime_record` has `status == Running && last_finished_at.is_none()`.
- `refresh_program_state` (program_state.rs) correctly sets `status = Failed` and `last_finished_at = Some(now)` for stale lanes.
- After `refresh_program_state`, `evaluate_program` is called again, re-classifying based on the fresh snapshot.

**Fix required**: In `evaluate_lane`, when building `run_snapshot`, check whether the run directory exists and the worker is alive. If not, populate `run_snapshot.status = Some(RunStatus::Dead)` so that `is_active` returns `false` and `is_failed` returns `true`. This ensures `classify_lane` sees the correct status without needing to reach into the runtime record.

**Alternative fix**: In `classify_lane`, check `runtime_record.map(|r| r.status != LaneExecutionStatus::Running)` before trusting the snapshot.

### Gap 2: Runtime Path Validation Is Missing for Graph and Prompt Resolution

**Severity**: High — causes bootstrap failures in generated packages.

**Description**: `run_fabro` in `dispatch.rs` validates that `run_config.exists()` but does not validate:
- That the graph file referenced by the run config exists
- That referenced prompts are resolvable from `target_repo`
- That the copied graph (if any) has correct relative paths

The `fabro run --detach` command will fail at runtime if these are missing, but the dispatch slot is consumed and the lane goes through a slow failure path.

**Fix required**: Add a pre-flight validation step in `execute_selected_lanes` or `run_fabro` that:
1. Loads the run config
2. Resolves the graph path via `resolve_graph_path`
3. Checks the graph file exists
4. Optionally checks prompt references are resolvable

Return `DispatchError::MissingRunConfig` or a new `DispatchError::RuntimePathInvalid` variant if validation fails.

### Gap 3: `synth evolve` Blocks Dispatch on Every Triggered Cycle

**Severity**: Medium — reduces effective dispatch rate.

**Description**: When `should_trigger_evolve` returns `true`, `run_synth_evolve` runs synchronously before dispatch. For a 120-second timeout (`SYNTH_EVOLVE_TIMEOUT_SECS`), this can block the entire dispatch cycle.

The `should_trigger_evolve` logic already has a fast-track for regenerate (`should_fast_track_regenerate_evolve`) that bypasses some checks, but it still runs synchronously.

**Fix required**: Move `run_synth_evolve` to a spawned thread. Store the evolve result in an `Arc<Mutex<Option<EvolveResult>>>` that the next cycle checks. Dispatch proceeds without waiting.

### Gap 4: Dispatch Telemetry Is Incomplete

**Severity**: Medium — operators cannot diagnose why work did not run.

**Description**: `AutodevCycleReport` captures `dispatched` outcomes but does not capture:
- Why ready lanes were not selected (family diversity, slot exhaustion, skip reasons)
- Runtime path errors during dispatch
- Rolling idle cycle count

**Fix required**: Add the fields defined in Section 3.1 of the spec to `AutodevCycleReport` and populate them in the dispatch decision logic.

### Gap 5: Prompt Resolution Path Is Unverified

**Severity**: Medium — may cause silent failures.

**Description**: The plan references that `@../../prompts/...` paths resolve under `~/.fabro/` at runtime. The fix should ensure that either:
- Paths are made absolute at render time
- Prompt files are copied alongside the graph
- `fabro run` is invoked with the correct `current_dir` (it is — `target_repo`)

The last point means that if the graph contains `@prompts/...` (relative to target repo root), and `fabro run` is invoked with `current_dir = target_repo`, the paths resolve correctly. The issue was specifically with `../` segments in the path that went above the target repo.

**Fix required**: In `fabro-synthesis/src/render.rs`, when resolving prompt references in rendered graphs, strip any `../` segments that would escape the target repo, or convert relative paths to be anchored at the target repo root.

## Specific Code Locations Requiring Changes

### 1. `lib/crates/raspberry-supervisor/src/evaluate.rs`

**Change 1**: In `load_run_snapshot`, add staleness detection:

```rust
fn load_run_snapshot(run_dir: &Path) -> RunSnapshot {
    // Existing: load status, live_state
    // New: check if run_dir exists and worker is alive
    let status = RunStatusRecord::load(&run_dir.join("status.json"))
        .ok()
        .map(|record| record.status);
    
    let live_state = RunLiveState::load(&run_dir.join("state.json")).ok();
    
    // New: if directory doesn't exist or worker is dead, mark as Dead
    let status = status.or_else(|| {
        if !run_dir.exists() {
            Some(RunStatus::Dead)
        } else if worker_process_alive(run_dir) == Some(false) {
            Some(RunStatus::Dead)
        } else {
            None
        }
    });
    
    RunSnapshot { status, ... }
}
```

**Change 2**: In `is_active`, prefer the snapshot's explicit non-active status:

```rust
fn is_active(run_snapshot: &RunSnapshot, runtime_record: Option<&LaneRuntimeRecord>) -> bool {
    // If snapshot says not active, believe it
    if run_snapshot.status.map(|s| !s.is_active()).unwrap_or(false) {
        return false;
    }
    // Fall through to runtime record check
    runtime_record
        .map(|record| {
            record.status == LaneExecutionStatus::Running && record.last_finished_at.is_none()
        })
        .unwrap_or(false)
}
```

### 2. `lib/crates/raspberry-supervisor/src/dispatch.rs`

**Change**: In `run_fabro`, add pre-flight validation:

```rust
fn run_fabro(...) -> Result<DispatchOutcome, DispatchError> {
    // Existing: check run_config.exists()
    if !run_config.exists() { ... }
    
    // New: validate graph file exists
    let config = load_run_config(run_config).map_err(...)?;
    let graph_path = resolve_graph_path(run_config, &config.graph);
    if !graph_path.exists() {
        return Err(DispatchError::MissingRunConfig { 
            lane: lane_key.to_string(), 
            path: graph_path 
        });
    }
    
    // ... continue with dispatch
}
```

### 3. `lib/crates/raspberry-supervisor/src/autodev.rs`

**Change 1**: Add new telemetry fields to `AutodevCycleReport`.

**Change 2**: Spawn `run_synth_evolve` in a thread:

```rust
// Instead of:
run_synth_evolve(&manifest_path, &manifest, settings)?;

// Do:
let evolve_handle = thread::spawn({
    let manifest_path = manifest_path.clone();
    let manifest = manifest.clone();
    let settings = settings.clone();
    move || run_synth_evolve(&manifest_path, &manifest, &settings)
});

// Dispatch proceeds immediately
// ... dispatch logic ...

// After dispatch, check evolve
let evolve_result = evolve_handle.join().unwrap_or(Err(...));
```

### 4. `lib/crates/fabro-synthesis/src/render.rs`

**Change**: In prompt path resolution, handle `../` escape attempts. The specific change depends on how prompt references are currently resolved in the rendered graph. If paths use `@prompts/...` (relative to target repo root) and `fabro run` uses `current_dir = target_repo`, the paths already resolve correctly. The issue is specifically with paths containing `../` that escape the target repo.

## Test Plan

### Unit Tests

1. **`evaluate_lane` with stale run snapshot**: Create a scenario where a lane has a runtime record with `status = Running` but the run directory does not exist. Assert that `evaluate_lane` returns `LaneExecutionStatus::Failed`.

2. **`dispatch.rs` pre-flight validation**: Dispatch a lane whose run config references a non-existent graph. Assert that `DispatchError::MissingRunConfig` or `RuntimePathInvalid` is returned, not a silent bootstrap failure.

3. **Telemetry fields round-trip**: Serialize and deserialize an `AutodevCycleReport` with all new telemetry fields. Assert no fields are dropped.

### Integration Tests

4. **Stale lane does not consume dispatch slot**: Start autodev with `max_parallel = 2`. Create 2 lanes: one genuinely running, one stale (run dir deleted). Assert that only 1 slot is consumed and the stale lane is reported as failed.

5. **Evolve does not block dispatch**: With `evolve_every_seconds = 60`, trigger evolve and verify dispatch occurs within the same cycle (before evolve completes). This requires mocking `run_synth_evolve` to take > 1 second.

6. **Prompt resolution**: Create a workflow that references `@prompts/review.md`. Verify the prompt resolves from the target repo, not `~/.fabro/prompts`.

### Live Validation (rXMRbro)

7. `raspberry autodev --max-parallel 10 --max-cycles 20` — sustain 10 active lanes for 20 cycles without bootstrap failures.

8. After 20 cycles, `raspberry status` shows `dispatch_summary` with `idle_cycles < total_cycles * 0.5` (less than 50% idle cycles).

9. At least 3 lanes land to trunk (integration lane reaches `landed` state in `trunk_landing` telemetry).

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Fixing `is_active` in `evaluate_lane` changes behavior for non-stale lanes | Low | Medium | Add test coverage for non-stale running lanes to ensure they remain `Running` |
| Pre-flight validation changes dispatch error handling behavior | Medium | Medium | The new validation returns the same `DispatchError::MissingRunConfig` variant; behavior is additive |
| Threading `synth evolve` introduces race conditions | Low | High | Use `Arc<Mutex<Option<EvolveResult>>>` shared state; validate evolve result is consumed before next cycle's dispatch |
| Prompt resolution fix breaks existing working paths | Low | High | Verify existing tests pass; add specific tests for the `../` escape case |

## Open Questions (for operator decision)

1. **Stale grace period**: 30 seconds may be too long for a 5-second poll interval. Consider reducing to 0 for immediate detection, or adding a separate "long-running" check (e.g., 30s for stale, 1800s for stall watchdog).

2. **Evolve decoupling**: The thread-based approach requires handling evolve failures in the next cycle. Ensure the error handling path (`last_evolve_at = Some(Instant::now())` on error) is preserved.

3. **Telemetry verbosity**: `ready_undispatched` could be large on programs with many ready lanes. Consider capping the list to the first 20 lane keys to avoid report bloat.

## Conclusion

The codebase has strong foundations for stale detection, failure classification, and dispatch selection. The primary gap is the persistence of stale lane corrections through the evaluation pipeline. The secondary gaps are pre-flight runtime path validation, evolve decoupling, and telemetry field completeness. The fixes are well-scoped and testable.

The review recommends proceeding with:
1. Fix Gap 1 (stale lane reclassification) first — highest impact, clearest fix
2. Fix Gap 4 (telemetry) second — enables live validation observability
3. Fix Gap 2 (pre-flight validation) third — prevents silent bootstrap failures
4. Fix Gap 3 (evolve decoupling) fourth — moderate complexity, good dispatch rate improvement
5. Fix Gap 5 (prompt resolution) fifth — depends on understanding the specific escape path
