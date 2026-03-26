# Autodev Execution Path and Dispatch Truth — Specification

Status: Active
Date: 2026-03-26
Type: Executable Specification
Lane: `autodev-efficiency-and-dispatch`

## Purpose

This document is the **executable specification** for the autodev execution path and dispatch truth slice. It defines what "correct" means for:

1. Runtime path consistency — the `fabro` binary autodev invokes must expose all commands the loop requires
2. Prompt and artifact reference resolution — generated workflows must not depend on `~/.fabro/prompts`
3. Stale lane truth — dead `running` lanes must not consume dispatch slots
4. Dispatch telemetry — operators must be able to explain every cycle why work did or did not run
5. Evolve decoupling — `synth evolve` must not block the dispatch cycle

After this spec is implemented:
- `fabro synth --help` works from any binary path autodev is configured with
- No generated workflow references `@../../prompts/...` resolving under `~/.fabro/`
- `running: 10, failed: 0` on rXMRbro sustains across 20+ cycles without bootstrap validation failures
- Every cycle report explains `ready_but_undispatched` and `idle_cycles`
- `synth evolve` runs in a background thread; dispatch is never blocked waiting for it

---

## Requirement 1: Runtime Path Consistency

### R1.1 — `synth` subcommand must be present in all `fabro` binaries

**What**: Every `fabro` binary used as `settings.fabro_bin` must expose the `synth` subcommand family: `synth create`, `synth evolve`, `synth review`, `synth import`, `synth genesis`.

**Why**: The autodev loop invokes `fabro synth evolve` via `rerender_program_package` (autodev.rs:2242 → runs `fabro_bin synth evolve --no-review`). If the binary lacks `synth`, the entire loop fails on the first evolve trigger.

**Current state**: `Command::Synth { command }` is routed in `fabro-cli/src/main.rs` with no feature gate. `fabro-synthesis` is included in default features.

**Validation**:
```bash
$ fabro synth --help
# Must show: create, evolve, review, import, genesis subcommands
```

**Proof** (fabro-cli/src/main.rs):
```rust
Command::Synth { command } => match command {
    commands::synth::SynthCommand::Import(args) => commands::synth::import_command(&args)?,
    commands::synth::SynthCommand::Create(args) => commands::synth::create_command(&args)?,
    commands::synth::SynthCommand::Evolve(args) => commands::synth::evolve_command(&args)?,
    commands::synth::SynthCommand::Review(args) => commands::synth::review_command(&args)?,
    commands::synth::SynthCommand::Genesis(args) => commands::synth::genesis_command(&args)?,
},
```

**Risk**: The controller may be pointed at a binary compiled without synthesis. The fix is a compile-time assertion that `fabro-synthesis` is not an optional feature. Add to `fabro-cli/src/main.rs`:

```rust
// Compile-time assertion: fabro-synthesis is always compiled in.
const _: () = assert!(
    !std::cfg!(feature = "fabro-synthesis"),
    "fabro-synthesis must NOT be an optional feature"
);
```

### R1.2 — Prompt refs in copied graphs must resolve relative to the graph file location

**What**: When a lane's run directory is set up, the copied `graph.fabro` file must have `@prompts/...` references resolve relative to the graph file's directory in the run directory, not relative to `~/.fabro/prompts` or the original source tree.

**Why**: `fabro validate` and the workflow engine resolve `@` references from the graph file's location. If the copied graph uses `../../prompts/...` paths, they resolve relative to the copy location — which may not match the original structure.

**Root cause**: The blueprint renderer in `fabro-synthesis` emits `@prompts/...` refs that are correct when the run config is at `malinka/run-configs/X/`, but become wrong `../../prompts/...` when the run dir structure is flattened.

**Current state**: This is a `fabro-graphviz` resolution concern. The parser resolves `@` refs using the graph file's directory as the resolution context. The issue is ensuring the renderer emits refs that are correct after the graph is copied to the lane's run directory.

**Validation**:
```bash
# Set up a lane run directory with a copied graph
mkdir -p /tmp/lane-run/run-configs
cp malinka/run-configs/investigate/baccarat-investigate.fabro /tmp/lane-run/run-configs/
# The graph must not reference ../prompts/ or ../../prompts/
grep -E '@\.\./' /tmp/lane-run/run-configs/*.fabro  # must be empty
```

---

## Requirement 2: Stale Lane Truth

### R2.1 — `stale_active_progress` must classify `running` lanes as `failed` when the worker process is dead

**What**: When a lane record has `status = Running` and the worker process is no longer alive, the refresh must transition the lane to `Failed` with `failure_kind = TransientLaunchFailure` and `recovery_action = BackoffRetry`.

**Why**: Stale `running` lanes consume dispatch slots (available_slots = max_parallel - current_running). If 10 slots are "occupied" by dead workers, no new lanes are dispatched.

**Current state**: Implemented correctly in `program_state.rs:1413`.

**Implementation** (program_state.rs:1413):
```rust
fn stale_active_progress(
    progress: &LiveLaneProgress,
    last_started_at: Option<DateTime<Utc>>,
) -> bool {
    let Some(status) = progress.workflow_status else {
        return false;
    };
    if !status.is_active() {
        return false;
    }
    if progress.worker_alive != Some(false) {
        return false;
    }
    let Some(started_at) = last_started_at.or(progress.updated_at) else {
        return false;
    };
    (Utc::now() - started_at).num_seconds() >= STALE_RUNNING_GRACE_SECS
}
```

**Validation**: The existing test `stale_active_progress_marks_run_as_stale` (program_state.rs:2609) covers the three-condition check (active status, dead worker, past grace period).

### R2.2 — `worker_process_alive` must correctly detect dead processes on all platforms

**What**: `worker_process_alive` (program_state.rs:1356) must return `Some(true)` when the worker PID exists and the process is alive, and `Some(false)` when the PID file is absent or the process is gone. On macOS, use `kill(pid, 0)` instead of `/proc/<pid>/exists`.

**Why**: On Linux, the implementation correctly uses `Path::new("/proc").join(pid).exists()`. On non-Linux (macOS, BSD), this always returns `Some(false)`, causing legitimate long-running workers to be incorrectly marked as stale after `STALE_RUNNING_GRACE_SECS` (30s).

**Current state**: The non-Linux branch unconditionally returns `Some(false)`:
```rust
#[cfg(not(target_os = "linux"))]
{
    let _ = pid;
    Some(false)  // BUG: all non-Linux workers appear dead after 30s
}
```

**Fix**: Implement macOS/BSD support using `kill(pid, 0)`:
```rust
#[cfg(not(target_os = "linux"))]
{
    // Use kill(pid, 0) which checks process existence without sending a signal
    let result = libc::kill(pid as libc::pid_t, 0);
    Some(result == 0 || errno::errno().0 == libc::EPERM)
}
```

### R2.3 — Pre-spawn errors from `run_fabro` must transition lane state to `Failed`

**What**: When `run_fabro` (dispatch.rs:495) returns `DispatchError::MissingRunConfig`, `DispatchError::Lease`, or `DispatchError::Spawn`, the lane must be transitioned to `Failed` (not left in `Ready` or `Running`) and the error must be recorded.

**Why**: Currently, these errors propagate up via `?` from the spawned thread. This causes two problems:
1. The `panic_error` handler fires, setting `PanicError` and causing an early return
2. No other lanes in that cycle are processed
3. The original lane's state is never updated — it stays `Ready` or `Running`

**Root cause**: `execute_selected_lanes` (dispatch.rs:220) spawns lanes into threads. Each thread returns `Result<(Lane, ...), DispatchError>`. When `run_fabro` returns an error variant, the `?` operator propagates it:
```rust
let output = output?;  // propagates DispatchError, never reaches mark_lane_dispatch_failed
```

**Fix**: Add an early-retry path for pre-spawn errors before the thread join. In `execute_selected_lanes`, before joining threads, check each future for pre-spawn errors:

```rust
// Check for pre-spawn errors before joining
let mut pre_spawn_failures = Vec::new();
for (lane_key, handle) in &handles {
    // Peek at the result without consuming it
    // If it's an Err containing MissingRunConfig/Lease/Spawn, record and continue
}
```

Alternatively (simpler): wrap pre-spawn errors in `DispatchOutcome` with `exit_status = -1` and `stderr = "pre-spawn: ..."` so they flow through the normal `mark_lane_dispatch_failed` path.

**Proof of current bug** (dispatch.rs:220-299):
```rust
for (lane_key, handle) in handles {
    match handle.join() {
        Ok(result) => joined_results.push(result),  // DispatchError goes straight to ?
        Err(payload) => { /* panic handling */ }
    }
}
// ...
for (lane, ..., output) in joined_results {
    let output = output?;  // <-- if output is Err, function returns here
    // mark_lane_dispatch_failed is never reached for pre-spawn errors
}
```

**Validation**:
```rust
// When run_fabro returns MissingRunConfig for lane "foo:1":
// After execute_selected_lanes returns Error:
// - "foo:1" must have status = Failed in program_state.json
// - error message must be "run config ... does not exist"
// - recovery_action must be BackoffRetry
```

### R2.4 — Bootstrap validation failures must be recoverable (not terminal)

**What**: When a lane fails at bootstrap (run config missing, workflow graph missing prompts, `fabro validate` fails) before any worker is dispatched, the failure must be classified as `TransientLaunchFailure` with `BackoffRetry` so the lane can be retried after `synth evolve` generates the missing files.

**Why**: On the first cycle, the generated package may not be complete. Bootstrap failures that are fixed by the next evolve must not permanently block the lane.

**Current state**: `classify_failure` (failure.rs:72) does not have explicit patterns for bootstrap-specific errors (missing run config, missing prompt file). R2.3 (above) is the prerequisite fix.

**Fix**: The pre-spawn error path (R2.3) must construct a `DispatchOutcome` with:
- `exit_status = -1` (indicates pre-spawn failure)
- `stderr = "pre-spawn: <error description>"`
- `fabro_run_id = None`

Then `mark_lane_dispatch_failed` calls `classify_failure` with this stderr, which must classify "pre-spawn" errors as `TransientLaunchFailure`. Add to `classify_failure`:

```rust
if combined.contains("pre-spawn") {
    return Some(FailureKind::TransientLaunchFailure);
}
```

---

## Requirement 3: Dispatch Telemetry

### R3.1 — Every cycle must emit why ready work was or was not dispatched

**What**: The `AutodevCycleReport` (autodev.rs:96) must include a `dispatch_state` field that explains the dispatch decision in operator language.

**Why**: Without per-cycle explainability, operators cannot diagnose why `max_parallel: 10` results in fewer than 10 running lanes.

**Current state**: `AutodevCycleReport` has `dispatched: Vec<DispatchOutcome>` but no structured reason field.

**Schema addition to `AutodevCycleReport`**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutodevCycleReport {
    pub cycle: usize,
    pub evolved: bool,
    pub evolve_target: Option<String>,
    pub ready_lanes: Vec<String>,
    #[serde(default)]
    pub replayed_lanes: Vec<String>,
    #[serde(default)]
    pub regenerate_noop_lanes: Vec<String>,
    pub dispatched: Vec<DispatchOutcome>,
    pub running_after: usize,
    pub complete_after: usize,
    // NEW:
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_state: Option<DispatchState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchState {
    pub reason: DispatchStateReason,
    pub dispatched_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ready_undispatched: Vec<ReadyUndispatchedLane>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle_explanation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchStateReason {
    /// Lanes were dispatched this cycle.
    Dispatched,
    /// No ready lanes existed at cycle start.
    NoReadyLanes,
    /// Max parallel was reached; no slots available.
    MaxParallelFull,
    /// Ready lanes existed but were not dispatched (evolve blocking, etc.).
    ReadyButUndispatched,
    /// Controller is settling (no running, no ready, no failed to replay).
    Settling,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyUndispatchedLane {
    pub lane_key: String,
    pub reason: String,
    // reason values:
    // - "max_parallel_reached" — slots exhausted
    // - "evolve_blocking" — evolve running in background
    // - "maintenance_mode" — program in maintenance
    // - "target_repo_stale" — repo not fresh enough
    // - "unknown" — unclassified
}
```

**Computation**: Compute `DispatchState` in the orchestrator loop (autodev.rs) just before `execute_selected_lanes` is called, after `should_trigger_evolve` is evaluated:

```rust
let dispatch_state = compute_dispatch_state(
    &ready_lanes,
    &lanes_to_dispatch,
    available_slots,
    max_parallel,
    evolve_in_progress,
    &program_before,
);
// Include in AutodevCycleReport
report.cycles.push(AutodevCycleReport {
    // ... existing fields ...
    dispatch_state: Some(dispatch_state),
});
```

### R3.2 — Aggregate dispatch metrics in `AutodevCurrentSnapshot`

**What**: `AutodevCurrentSnapshot` (autodev.rs:70) must include aggregate counters summarizing the entire autodev run.

**Current state**: `AutodevCurrentSnapshot` has `ready`, `running`, `blocked`, `failed`, `complete` but no aggregate counters.

**Schema additions**:
```rust
pub struct AutodevCurrentSnapshot {
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_parallel: Option<usize>,
    pub ready: usize,
    pub running: usize,
    pub blocked: usize,
    pub failed: usize,
    pub complete: usize,
    #[serde(default)]
    pub ready_lanes: Vec<String>,
    #[serde(default)]
    pub running_lanes: Vec<String>,
    #[serde(default)]
    pub failed_lanes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub critical_blockers: Vec<CriticalBlocker>,
    // NEW:
    /// Total cycles with no dispatched lanes since run started.
    pub idle_cycles: usize,
    /// Total lanes dispatched since run started.
    pub total_dispatched: usize,
    /// Total lanes that completed (landing_state = landed).
    pub total_landed: usize,
    /// Total stale running lanes detected and reclaimed this run.
    pub stale_running_reclaimed: usize,
    /// Total bootstrap failures (run config missing, etc.).
    pub bootstrap_failures: usize,
    /// Total runtime-path errors (fabro not found, etc.).
    pub runtime_path_errors: usize,
    /// Dispatch rate: dispatched / (dispatched + idle_cycles).
    pub dispatch_rate: f64,
}
```

**Maintenance**: Increment counters in the orchestrator loop:
- `idle_cycles++` when `lanes_to_dispatch.is_empty()` and `frontier_before.running == 0`
- `total_dispatched += lanes_to_dispatch.len()`
- `stale_running_reclaimed++` when `refresh_program_state` transitions `Running → Failed`
- `bootstrap_failures++` when a pre-spawn error (R2.3) is encountered
- `runtime_path_errors++` when `run_fabro` returns `DispatchError::Spawn` (binary not found)

---

## Requirement 4: Evolve Decoupling

### R4.1 — `synth evolve` must not block the dispatch cycle

**What**: `run_synth_evolve` (autodev.rs:2242) must execute in a background thread such that `execute_selected_lanes` is called in the same cycle.

**Why**: `run_synth_evolve` has a 120-second timeout (`SYNTH_EVOLVE_TIMEOUT_SECS = 120`). Blocking evolve delays dispatch by up to 2 minutes per cycle.

**Current state**: `run_synth_evolve` is called synchronously in the orchestrator loop (autodev.rs:444). The `match run_synth_evolve(...)` block completes before dispatch begins.

**Architecture**:
```
Current (blocking):
  [refresh] → [evaluate] → [evolve (120s blocking)] → [dispatch] → [watch]

Target (non-blocking):
  [refresh] → [evaluate] → [dispatch] ────────────────────────────┐
       ↓ (background)                                              │
  [evolve (120s, background thread)] ────────────────────────────►│
                                        ↓                          │
                              [join on next cycle]                 │
                                        ↓                          │
                              [reload manifest, re-evaluate]       │
                                        ↓                          │
                              [watch] ◄────────────────────────────┘
```

**Implementation**:

Add to the orchestrator state struct:
```rust
struct OrchestratorState {
    // ... existing fields ...
    pending_evolve: Option<tokio::task::JoinHandle<Result<(), AutodevError>>>,
    evolve_completed_this_cycle: bool,
}
```

Refactor the orchestrator loop (autodev.rs:426-495):
```rust
// 1. If evolve was triggered and no evolve is currently running, spawn it
if should_trigger_evolve(...) && state.pending_evolve.is_none() {
    let manifest_path = manifest_path.clone();
    let manifest = manifest.clone();
    let settings = settings.clone();
    state.pending_evolve = Some(tokio::task::spawn_blocking(move || {
        run_synth_evolve(&manifest_path, &manifest, &settings)
    }));
}

// 2. If an evolve is running, check if it completed
if let Some(handle) = state.pending_evolve.as_mut() {
    if handle.is_finished() {
        match handle.join().unwrap() {
            Ok(()) => {
                last_evolve_at = Some(Instant::now());
                last_evolve_frontier = Some(frontier_before);
                evolved = true;
                state.evolve_completed_this_cycle = true;
            }
            Err(e) => { /* log and continue */ }
        }
        state.pending_evolve = None;
    }
}

// 3. Dispatch ALWAYS happens in the same cycle (unless another evolve is running)
if state.pending_evolve.is_none() {
    execute_selected_lanes(...);
} else {
    // Skip dispatch this cycle — waiting for evolve to complete
    // This is safe because the manifest may change
}
```

**Invariant**: We must never dispatch using a manifest that has been invalidated by an in-progress evolve. The background-thread approach preserves this because dispatch is skipped while `pending_evolve.is_some()`.

### R4.2 — Dispatch must consume the full `max_parallel` budget when ready lanes exist

**What**: When `available_slots > 0` and `ready_lanes > 0`, `lanes_to_dispatch` must contain at least `min(available_slots, ready_lanes)` lanes.

**Why**: Confirmed by R2.1 (stale slots) and R2.3 (pre-spawn errors). This requirement validates the combined fix.

**Validation**:
```rust
// In integration test with 15 ready lanes and max_parallel=10:
// After dispatch, running count must be 10 (or fewer if fewer than 10 were ready)
assert!(program_after.lanes.iter().filter(|l| l.status == Running).count() <= 10);
```

---

## Requirement 5: Live Validation Criteria

### V1 — Sustain 10 active lanes on rXMRbro

**Command**:
```bash
raspberry autodev \
  --manifest /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml \
  --max-parallel 10 --max-cycles 20
```

**Pass criteria**:
1. Controller maintains `InProgress` state for all 20 cycles
2. Every cycle report shows `running >= 8` (allowing 2 slots for brief gaps)
3. `AutodevCurrentSnapshot.idle_cycles <= 5` for the full run
4. `bootstrap_failures = 0` for cycles 3-20 (cycles 1-2 bootstrap failures are acceptable)
5. `runtime_path_errors = 0` — `fabro` binary must expose `synth`

### V2 — At least 3 lanes land to trunk

**Pass criteria**: After 20 cycles or when controller settles, at least 3 lanes have `landing_state = "landed"` in their `trunk_delivery_state_for_lane()` detail.

---

## Implementation Order

| Step | Requirement | Key File | Status |
|------|-------------|----------|--------|
| 1 | R2.3 — pre-spawn error state transition | dispatch.rs:220-299 | **Bug** |
| 2 | R2.2 — macOS `worker_process_alive` | program_state.rs:1356 | **Bug** |
| 3 | R3.1 — `DispatchState` in `AutodevCycleReport` | autodev.rs:96 | **Missing** |
| 4 | R3.2 — aggregate counters in snapshot | autodev.rs:70 | **Missing** |
| 5 | R4.1 — background evolve thread | autodev.rs:426-495 | **Missing** |
| 6 | R2.4 — bootstrap failure recovery | dispatch.rs + failure.rs | **Bug** |
| 7 | V1 + V2 — live validation | rXMRbro | **Unvalidated** |

---

## Key Files

| File | Role |
|------|------|
| `lib/crates/raspberry-supervisor/src/autodev.rs` | Orchestrator loop — must emit dispatch telemetry, decouple evolve |
| `lib/crates/raspberry-supervisor/src/dispatch.rs` | `execute_selected_lanes`, `run_fabro` — must update lane state on pre-spawn errors |
| `lib/crates/raspberry-supervisor/src/program_state.rs` | `stale_active_progress`, `worker_process_alive` — stale lane detection |
| `lib/crates/raspberry-supervisor/src/failure.rs` | `classify_failure` — must classify bootstrap/pre-spawn errors as `TransientLaunchFailure` |
| `fabro-cli/src/main.rs` | Routes `Command::Synth` — must always include synthesis commands |

---

## Decision Log

- **Decision**: Implement background-thread evolve (R4.1) instead of interval-gated evolve.
  **Rationale**: Interval-gated evolve prevents evolve from running during active dispatch cycles. Background threading allows evolve to run even when slots are occupied.
  **Date**: 2026-03-26

- **Decision**: Emit dispatch telemetry as structured data in `AutodevCycleReport` rather than log lines.
  **Rationale**: The `AutodevReport` JSON is already persisted and parsed by the Paperclip dashboard. Adding `dispatch_state` to the cycle report makes it observable without new infrastructure.
  **Date**: 2026-03-26

- **Decision**: Fix R2.3 by wrapping pre-spawn errors in `DispatchOutcome` with `exit_status = -1` instead of adding a special-case error path.
  **Rationale**: This flows naturally through `mark_lane_dispatch_failed` and `classify_failure` without special-casing. The stderr string `"pre-spawn: <error>"` triggers the `TransientLaunchFailure` classification.
  **Date**: 2026-03-26
