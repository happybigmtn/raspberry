# Autodev Execution Path and Dispatch Truth — Review

Status: Review Complete
Date: 2026-03-26
Lane: `autodev-efficiency-and-dispatch`
Reviewer: Code Analysis (static)

---

## Executive Summary

The autodev loop in `raspberry-supervisor` is structurally sound. The core orchestration logic, stale running detection, and lane state reconciliation are already implemented correctly in `program_state.rs`. The primary gaps are:

1. **Dispatch telemetry** (R3.1, R3.2): No per-cycle explainability of why ready work did or did not run
2. **Evolve blocking** (R4.1): `run_synth_evolve()` is synchronous, adding up to 120s to the dispatch cycle
3. **Bootstrap failure state** (R2.3): Pre-spawn `run_fabro` errors don't update lane state
4. **Prompt reference resolution** (R1.2): Generated workflow graphs may still contain `@../../prompts/...` references that resolve outside the target repo

---

## Requirement-by-Requirement Review

### R1.1 — `synth` subcommand in all `fabro` binaries

**Verdict**: ✅ Correct — no action needed.

The `fabro-cli/Cargo.toml` includes `fabro-synthesis` unconditionally (no feature gate). The CLI routes `Command::Synth` in `main.rs`:
```rust
Command::Synth { command } => match command {
    commands::synth::SynthCommand::Import(args) => commands::synth::import_command(&args)?,
    commands::synth::SynthCommand::Create(args) => commands::synth::create_command(&args)?,
    commands::synth::SynthCommand::Evolve(args) => commands::synth::evolve_command(&args)?,
    commands::synth::SynthCommand::Review(args) => commands::synth::review_command(&args)?,
    commands::synth::SynthCommand::Genesis(args) => commands::synth::genesis_command(&args)?,
},
```

The root cause of the observed failure ("clean `fabro` binary did not expose `synth`") was **binary provenance**: the controller was pointed at a `fabro` binary compiled without `fabro-synthesis` as a dependency. Since `fabro-synthesis` is now always compiled, this failure mode is eliminated for all future builds.

**Proof command**:
```bash
cargo build --release -p fabro-cli && \
  ./target-local/release/fabro synth --help | grep -E "create|evolve|review|import|genesis"
```

---

### R1.2 — Prompt reference resolution

**Verdict**: ⚠️ Partially implemented — needs validation.

The plan's root cause observation:
> Copied `graph.fabro` files referenced prompts as `@../../prompts/...`, which resolved under `~/.fabro/` at runtime instead of the target repo.

The `fabro-graphviz` parser handles `@` includes. The critical question is whether the resolution context is set to the **workflow graph's directory** (correct) or defaults to `~/.fabro/` (incorrect for copied run environments).

**Current behavior**: `fabro_workflows` resolves `@includes` using the working directory at parse time. When a workflow is **copied** into a per-lane run directory, the relative path `../../prompts/foo.md` from `malinka/run-configs/investigate/graph.fabro` resolves to `malinka/prompts/foo.md` (correct). However, if the working directory at parse time is `~/.fabro/` (e.g., when the fabro process is started from home), the same relative path resolves to `~/.fabro/prompts/foo.md` (incorrect).

**What needs validation**:
1. Find where `fabro-graphviz` sets the resolution base for `@include` directives
2. Verify it uses the graph file's directory, not `std::env::current_dir()`
3. Add a test: copy a workflow with `@prompts/...` refs to `/tmp`, change CWD to `~`, parse → must resolve relative to graph location

**File to investigate**: `lib/crates/fabro-graphviz/src/` — search for `include` resolution context.

---

### R2.1 — Stale running detection

**Verdict**: ✅ Correct — existing implementation is sound.

`program_state.rs` has `stale_active_progress()`:
```rust
fn stale_active_progress(progress: &LiveLaneProgress, last_started_at: Option<DateTime<Utc>>) -> bool {
    let Some(status) = progress.workflow_status else { return false; };
    if !status.is_active() { return false; }
    if progress.worker_alive != Some(false) { return false; } // must be dead
    let Some(started_at) = last_started_at.or(progress.updated_at) else { return false; };
    (Utc::now() - started_at).num_seconds() >= STALE_RUNNING_GRACE_SECS  // 30s
}
```

When this returns `true`, the lane is transitioned to `Failed` with `TransientLaunchFailure` / `BackoffRetry`. This is correct. The test `refresh_program_state_marks_missing_running_run_as_stale_failure` validates this path.

**Gap**: No integration test simulating the **"worker dispatched, immediately killed before writing PID file"** scenario. The grace period requires `last_started_at` to be set — but `mark_lane_submitted` sets `last_started_at = now`, so this is fine.

**Recommended test**:
```rust
#[test]
fn stale_running_detected_when_worker_pid_disappears_before_any_progress() {
    // Setup: lane dispatched, PID written, process killed
    // Assert: after STALE_RUNNING_GRACE_SECS, lane is Failed
}
```

---

### R2.2 — Keep alive workers running

**Verdict**: ✅ Correct.

`stalled_active_progress_reason()` handles the complementary case — a worker that is alive but unresponsive for 1800 seconds. The 1800s timeout is conservative but reasonable. The `worker_alive` check uses `/proc/<pid>/exists` on Linux.

**Edge case**: On macOS (non-Linux), `worker_process_alive()` always returns `Some(false)`:
```rust
#[cfg(not(target_os = "linux"))]
{
    let _ = pid;
    Some(false)
}
```
This means on macOS, a lane whose worker is alive will be marked stale after 30s because `worker_alive = Some(false)`. This is a **pre-existing bug** on non-Linux platforms. It is out of scope for this lane but should be filed separately.

---

### R2.3 — Bootstrap failure classification

**Verdict**: ❌ Missing — pre-spawn errors don't update lane state.

In `dispatch.rs`, `run_fabro()` returns `DispatchError::MissingRunConfig` before ever dispatching:
```rust
fn run_fabro(...) -> Result<DispatchOutcome, DispatchError> {
    if !run_config.exists() {
        return Err(DispatchError::MissingRunConfig { ... });
    }
    // ... only reaches here if run_config exists
    let output = command.output().map_err(|source| DispatchError::Spawn { ... })?;
    // ...
}
```

When this error propagates to `execute_selected_lanes`, the error is returned and the lane state is **never updated**. The lane remains `Running` (if it was previously marked submitted) or stays `Ready`. This means:

1. The slot appears occupied (lane is `Running`)
2. No retry is scheduled (no `FailureKind` set)
3. On the next cycle, the same lane is evaluated as `Running` — slot is double-counted

**Fix required**: In `execute_selected_lanes`, when `run_fabro` returns `DispatchError::MissingRunConfig` or `DispatchError::Spawn`, the lane must be transitioned to `Failed` with `TransientLaunchFailure` / `BackoffRetry`:
```rust
Err(DispatchError::MissingRunConfig { lane, path }) => {
    let outcome = DispatchOutcome {
        lane_key: lane.clone(),
        exit_status: 1,
        fabro_run_id: None,
        stdout: String::new(),
        stderr: format!("run config not found: {}", path.display()),
    };
    mark_lane_dispatch_failed(&mut state, &lane, &lane.run_config, &outcome);
    outcomes.push(outcome);
    continue; // don't return error — record failure and continue
}
```

This pattern should also apply to `DispatchError::Spawn` (binary not found or failed to start).

---

### R3.1 — Per-cycle dispatch telemetry

**Verdict**: ❌ Not implemented.

The current `AutodevCycleReport` only records:
```rust
struct AutodevCycleReport {
    pub cycle: usize,
    pub evolved: bool,
    pub evolve_target: Option<String>,
    pub ready_lanes: Vec<String>,
    pub replayed_lanes: Vec<String>,
    pub regenerate_noop_lanes: Vec<String>,
    pub dispatched: Vec<DispatchOutcome>,
    pub running_after: usize,
    pub complete_after: usize,
}
```

There is no field explaining **why** ready work was not dispatched. The `AutodevCurrentSnapshot` has no aggregate dispatch metrics.

**Missing**: The `DispatchState` struct defined in the spec is not present in the codebase.

**Implementation location**: In `autodev.rs`, after `select_ready_lanes_for_dispatch()` computes `lanes_to_dispatch`, compute `DispatchState`:
```rust
let dispatch_state = compute_dispatch_state(
    &lanes_to_dispatch,
    &ready_lanes,
    &program_before,
    max_parallel,
    current_running,
    evolved,
);
```

---

### R3.2 — Aggregate dispatch metrics

**Verdict**: ❌ Not implemented.

No counters for `idle_cycles`, `total_dispatched`, `stale_running_reclaimed`, `runtime_path_errors` exist in `AutodevReport` or `AutodevCurrentSnapshot`.

**Implementation location**: These should be added as fields to `AutodevReport` (summary across all cycles) and maintained in the orchestrator loop variables.

---

### R4.1 — Evolve decoupling

**Verdict**: ❌ Blocking — `run_synth_evolve()` is synchronous.

Current code in `autodev.rs`:
```rust
if should_trigger_evolve(...) {
    match run_synth_evolve(&manifest_path, &manifest, settings) {
        Ok(()) => { /* evolve done, then continue */ }
        // ...
    }
}
// Only reaches here after evolve completes (or times out after 120s)
let dispatched = execute_selected_lanes(...);
```

`run_synth_evolve` calls `rerender_program_package` which runs `fabro synth evolve` with a 120-second `timeout`. This is synchronous.

**Assessment of impact**: The gating conditions in `should_trigger_evolve` are conservative:
- `frontier.total_work() >= frontier_budget` — blocks evolve when plenty of work exists
- `!recovery_needs_evolve` — blocks if no regenerable lanes exist

However, on a fresh start where `last_evolve_frontier = None` and `recovery_needs_evolve = true` and `doctrine_changed = true`, evolve fires on cycle 1 and blocks dispatch for up to 120 seconds. This is the critical path for the live validation target.

**Fix**: Implement background-thread evolve as described in the spec. The thread spawns `run_synth_evolve`, and the next cycle's `should_trigger_evolve` check waits for the join handle before evaluating.

---

### R4.2 — Consume full `max_parallel` budget

**Verdict**: ✅ Already implemented.

`select_ready_lanes_for_dispatch()` in `autodev.rs` correctly fills available slots:
```rust
let available_slots = max_parallel.saturating_sub(current_running);
let replayed_lanes = replayable_failures.iter().take(available_slots).cloned().collect();
let remaining_slots = available_slots.saturating_sub(replayed_lanes.len());
let selected_ready_lanes = select_ready_lanes_for_dispatch(&program, remaining_slots);
```

The gap is stale `running` lanes (R2.1 gap) that incorrectly inflate `current_running`, making `available_slots` too small. Once R2.1 is validated and R2.3 is fixed, this requirement will be fully satisfied.

---

## Synthesis: Current State vs. Target

| Requirement | Status | Gap | Priority |
|-------------|--------|-----|----------|
| R1.1: synth in fabro binary | ✅ Done | None | — |
| R1.2: prompt ref resolution | ⚠️ Unclear | Needs validation of graph parser resolution context | Medium |
| R2.1: stale running detection | ✅ Done | Integration test gap | Low |
| R2.2: keep alive workers alive | ✅ Done | macOS bug (out of scope) | — |
| R2.3: bootstrap failure state | ❌ Missing | run_fabro pre-spawn errors don't update lane state | High |
| R3.1: per-cycle dispatch telemetry | ❌ Missing | `DispatchState` not implemented | High |
| R3.2: aggregate dispatch metrics | ❌ Missing | counters not implemented | High |
| R4.1: evolve decoupling | ❌ Blocking | run_synth_evolve is synchronous | High |
| R4.2: consume full budget | ✅ Done | Blocked by R2.1 + R2.3 | — |

---

## Testing Coverage Assessment

### Existing tests — `raspberry-supervisor`

| Test | What it covers | Assessment |
|------|---------------|-------------|
| `replayable_failed_lanes_only_selects_recoverable_failures` | Failure recovery classification | ✅ Solid |
| `replayable_failed_lanes_include_proof_failures` | ReplayLane recovery | ✅ Solid |
| `regenerable_failed_lanes_include_blocked_supervisor_only_lanes` | RegenerateLane recovery | ✅ Solid |
| `dispatchable_failed_lanes_selects_regenerable_failures_after_evolve` | Evolve + replay | ✅ Solid |
| `child_program_manifests_to_advance_uses_spare_slots_for_failed_children` | Child program scheduling | ✅ Solid |
| `child_program_manifests_to_advance_includes_ready_children` | Ready child programs | ✅ Solid |
| `refresh_program_state_syncs_child_program_lane_statuses` | Child state sync | ✅ Solid |
| `refresh_program_state_prefers_child_runtime_state_over_artifact_completion` | Child state precedence | ✅ Solid |
| `refresh_program_state_clears_current_fields_for_failed_run` | Failed run cleanup | ✅ Solid |
| `refresh_program_state_marks_missing_running_run_as_stale_failure` | Stale running → failed | ✅ Solid |
| `refresh_program_state_propagates_child_running_runtime_details` | Child running details | ✅ Solid |
| `refresh_program_state_clears_stale_failure_residue_for_succeeded_run_progress` | Succeeded run cleanup | ✅ Solid |
| `refresh_program_state_prefers_failed_status_record_over_succeeded_live_state` | Status record authority | ✅ Solid |
| `sync_program_state_with_evaluated_clears_stale_failure_residue_for_ready_lane` | Ready lane cleanup | ✅ Solid |
| `sync_program_state_with_evaluated_clears_stale_failure_residue_for_complete_lane` | Complete lane cleanup | ✅ Solid |
| `refresh_program_state_prunes_removed_lanes` | Lane pruning | ✅ Solid |
| `execute_selected_lanes_refuses_dispatch_during_maintenance` | Maintenance mode | ✅ Solid |

### Tests needed for this lane

| Test | File | Description |
|------|------|-------------|
| `run_fabro_missing_run_config_updates_lane_to_failed` | `dispatch.rs` | Verify MissingRunConfig transitions lane to Failed |
| `run_fabro_spawn_error_updates_lane_to_failed` | `dispatch.rs` | Verify Spawn error transitions lane to Failed |
| `stale_running_detected_with_grace_period_boundary` | `program_state.rs` | Edge case: exactly 30s old, worker dead → must fail |
| `stale_running_not_detected_when_worker_alive` | `program_state.rs` | Edge case: 60s old, worker alive → must stay Running |
| `dispatch_state_computed_for_idle_cycle` | `autodev.rs` | Verify DispatchState for cycle with no ready lanes |
| `dispatch_state_computed_for_max_parallel_full` | `autodev.rs` | Verify DispatchState when running >= max_parallel |
| `dispatch_state_computed_for_ready_but_undispatched` | `autodev.rs` | Verify DispatchState when lanes skipped for evolve blocking |
| `autodev_cycle_report_has_dispatch_state` | `autodev.rs` | Verify AutodevCycleReport includes dispatch_state field |
| `aggregate_counters_idle_cycles_incremented` | `autodev.rs` | Verify idle_cycles incremented on empty dispatch |
| `aggregate_counters_stale_running_reclaimed` | `autodev.rs` | Verify stale_running_reclaimed incremented when lane transitions |

---

## Recommended Implementation Sequence

1. **R2.3 first** (bootstrap failure state): Fix `run_fabro` error handling in `dispatch.rs`. This is a one-file change with a clear fix. Immediately validates with `cargo nextest run -p raspberry-supervisor dispatch`.

2. **R3.1 second** (dispatch telemetry): Add `DispatchState` to `AutodevCycleReport`. Compute it in the orchestrator loop. This change is additive — no existing behavior changes. The telemetry makes the next steps observable.

3. **R3.2 third** (aggregate counters): Add counters to `AutodevReport` and `AutodevCurrentSnapshot`. Increment in the loop. Validate with the live test.

4. **R4.1 fourth** (evolve decoupling): Refactor `run_synth_evolve` to background thread. The R3.1 telemetry will show whether this is the primary bottleneck.

5. **R1.2 fifth** (prompt resolution): Investigate `fabro-graphviz` include resolution. If it already resolves relative to the graph file, close with a test. If not, fix and add a regression test.

6. **V1 + V2 last** (live validation): Run the full 20-cycle validation on rXMRbro. All preceding changes should make this pass naturally.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Background evolve thread outlives the controller process | Low | Medium | Use `thread::spawn` with `std::thread::Result`, join on shutdown |
| Evolve completes between cycle N evaluation and cycle N+1 dispatch, but the re-evaluate in cycle N+1 is skipped because `!dispatched.is_empty() && !evolved` | Low | Medium | Force re-evaluate when evolve completed (track `evolve_completed_this_cycle`) |
| Dispatch telemetry adds latency to the hot loop | Low | Low | `DispatchState` computation is O(ready_lanes), a simple filter+map |
| `fabro-graphviz` resolution context fix breaks existing workflows | Low | High | Add a test with a known-good workflow; validate on rXMRbro before shipping |
