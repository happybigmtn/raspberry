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
- `fabro synth evolve --help` works from any binary path autodev is configured with
- No generated workflow references `@../../prompts/...` resolving under `~/.fabro/`
- `running: 10, failed: 0` on rXMRbro sustains across 20+ cycles without bootstrap validation failures
- Every cycle report explains `ready_but_undispatched` and `idle_cycles`
- `synth evolve` runs in the background; dispatch is never blocked waiting for it

---

## Requirement 1: Runtime Path Consistency

### R1.1 — `synth` subcommand must be present in all `fabro` binaries

**What**: Every `fabro` binary used as `settings.fabro_bin` must expose the `synth` subcommand family: `synth create`, `synth evolve`, `synth review`, `synth import`, `synth genesis`.

**Why**: The autodev loop invokes `fabro synth evolve` (see `autodev.rs → run_synth_evolve`). If the binary lacks `synth`, the entire loop fails on the first evolve trigger.

**Validation**:
```
$ fabro synth --help
# Must show: create, evolve, review, import, genesis subcommands
```

**Proof**:
```rust
// fabro-cli/src/main.rs must route `Command::Synth { command }` to the handler.
// The handler lives in fabro-cli/src/commands/synth.rs.
// This must NOT be behind a feature flag that omits synthesis from default builds.
Command::Synth { command } => match command {
    commands::synth::SynthCommand::Import(args) => commands::synth::import_command(&args)?,
    commands::synth::SynthCommand::Create(args) => commands::synth::create_command(&args)?,
    commands::synth::SynthCommand::Evolve(args) => commands::synth::evolve_command(&args)?,
    commands::synth::SynthCommand::Review(args) => commands::synth::review_command(&args)?,
    commands::synth::SynthCommand::Genesis(args) => commands::synth::genesis_command(&args)?,
},
```

**Current state**: The `Synth` variant exists in the CLI enum and is routed in `main.rs`. No feature gate guards it. The concern in the plan was about binary provenance — the controller may be pointed at a binary compiled without `fabro-synthesis`. This is fixed by ensuring `fabro-synthesis` is always included in the default features.

**Implementation**: Verify `fabro-synthesis` is `default = []` (always compiled) in `lib/crates/fabro-cli/Cargo.toml` and not behind an optional feature flag.

### R1.2 — `fabro validate` must accept run configs without requiring `~/.fabro/prompts`

**What**: The `fabro validate <run-config>` command must succeed when the run config's workflow graph uses `@prompts/...` references, as long as those prompt files exist relative to the workflow graph file location or the run config's directory.

**Why**: Generated workflows copy `graph.fabro` into per-lane run directories. Prompt refs like `@prompts/foo.md` must resolve from the copied graph's location, not from `~/.fabro/prompts`.

**Root cause**: Copied `graph.fabro` files referenced prompts as `@../../prompts/...` which resolved under `~/.fabro/` at runtime instead of the target repo.

**Validation**:
```bash
# Create a temp dir simulating a copied run environment
mkdir -p /tmp/fabro-validate-test/malinka/run-configs/investigate
mkdir -p /tmp/fabro-validate-test/malinka/prompts
cp malinka/run-configs/investigate/baccarat-investigate.toml /tmp/fabro-validate-test/malinka/run-configs/
cp malinka/workflows/baccarat-investigate.fabro /tmp/fabro-validate-test/malinka/run-configs/
echo "# Prompt content" > /tmp/fabro-validate-test/malinka/prompts/baccarat-prompt.md
cd /tmp/fabro-validate-test
fabro validate malinka/run-configs/baccarat-investigate.toml
# Must succeed without: "file not found: ~/.fabro/prompts/..."
```

**Implementation**: The workflow graph parser in `fabro-graphviz` must resolve `@` references relative to the graph file's directory. The resolution context must be passed from the call site (run config path → graph path → resolve prompts relative to graph). Currently the parser may fall back to `~/.fabro/prompts` when the path starts with `../../`, which is wrong for detached run environments.

---

## Requirement 2: Stale Lane Truth

### R2.1 — `refresh_program_state` must classify `running` lanes as `failed` when the worker process is dead

**What**: When a lane record has `status = Running` and `last_finished_at = None`, but the worker process is no longer alive (no `/proc/<pid>` on Linux, or equivalent), the refresh must transition the lane to `status = Failed` with `failure_kind = TransientLaunchFailure` and `recovery_action = BackoffRetry`.

**Why**: Stale `running` lanes consume dispatch slots (see `available_slots = max_parallel - current_running`). If 10 slots are "occupied" by dead workers, no new lanes are dispatched.

**Current state**: `program_state.rs` has `stale_active_progress()` which implements this check. It requires all three conditions:
1. `progress.workflow_status.is_active()`
2. `progress.worker_alive == Some(false)` — worker process gone
3. `(now - started_at).num_seconds() >= STALE_RUNNING_GRACE_SECS (30s)`

**Additional requirement — kill-process detection**: The `worker_alive` check uses `/proc/<pid>/exists` on Linux. This correctly detects the case where the supervisor process was killed (the PID file exists but the process is gone). However, it does NOT detect the case where the process is still running but unresponsive (stalled). The `stalled_active_progress_reason()` handles that case separately with a 1800-second timeout.

**Edge case — new lane dispatched but process immediately dies**: A lane is dispatched (`mark_lane_submitted`), the worker starts and writes its PID, then crashes before writing any progress. The grace period (30s) means the slot is wasted for up to 30 seconds. This is acceptable because we need the grace period to avoid false positives during slow startup. The fix is to keep the grace period short (≤30s) and ensure the PID file is written early in the worker process.

**Validation**:
```rust
// In program_state.rs tests, verify:
let mut state = ProgramRuntimeState::new("test");
state.lanes.insert("lane:1".to_string(), LaneRuntimeRecord {
    lane_key: "lane:1".to_string(),
    status: LaneExecutionStatus::Running,
    current_run_id: Some("01DEADWORKER0000000000000".to_string()),
    current_fabro_run_id: Some("01DEADWORKER0000000000000".to_string()),
    current_stage_label: Some("Implement".to_string()),
    last_started_at: Some(Utc::now() - Duration::seconds(60)), // past grace period
    ..Default::default()
});

// Simulate dead worker (no /proc/<pid>):
// refresh_program_state must transition this to Failed
// with TransientLaunchFailure and BackoffRetry
```

### R2.2 — `refresh_program_state` must NOT transition `running` to `failed` when the worker is still alive

**What**: A lane whose worker process is still running (PID exists, `/proc/<pid>` exists) must remain `status = Running`, regardless of how long it has been running.

**Why**: `stalled_active_progress_reason` handles genuinely stalled workers (1800s timeout). A long-running worker should not be incorrectly marked failed just because it's slow.

**Validation**: The test `refresh_program_state_prefers_long_running_worker` (if added) must verify that a lane with `worker_alive = Some(true)` and activity within 1800s stays `Running`.

### R2.3 — Bootstrap validation failures must be recoverable (not terminal)

**What**: When a lane fails at bootstrap (run config missing, workflow graph missing prompts, `fabro validate` fails) before any worker is dispatched, the failure must be classified with a recovery action that allows redispatch after the underlying cause is fixed (e.g., after `synth evolve` generates the missing files).

**Why**: On the first cycle, the generated package may not be complete. Bootstrap failures that are fixed by the next evolve should not permanently block the lane.

**Classification**:
- Run config file missing → `FailureKind::TransientLaunchFailure`, `BackoffRetry`
- Prompt reference not found → `FailureKind::TransientLaunchFailure`, `BackoffRetry`
- `fabro validate` fails with graph error → `FailureKind::TransientLaunchFailure`, `BackoffRetry`
- Workflow graph references non-existent prompt → `FailureKind::TransientLaunchFailure`, `BackoffRetry`

**Current state**: The `classify_failure()` function in `failure.rs` does not yet have explicit patterns for bootstrap-specific errors. The `run_fabro()` function in `dispatch.rs` returns `DispatchError::MissingRunConfig` before spawning, which prevents the dispatch from completing — the lane is never actually dispatched, so it stays `Running` or `Ready` rather than becoming `Failed`. This is a separate bug: if `run_fabro` fails before dispatching, the lane status is not updated.

**Fix required**: When `run_fabro` returns `DispatchError::MissingRunConfig` (or any pre-spawn error), the dispatch module must call `mark_lane_dispatch_failed()` with an appropriate failure record, not just return an error.

---

## Requirement 3: Dispatch Telemetry

### R3.1 — Every cycle must emit why ready work was or was not dispatched

**What**: The `AutodevCycleReport` must include a `dispatch_state` field that explains the dispatch decision in operator language. This field must be present in every cycle report, not just cycles with dispatches.

**Why**: The historical 83% idle rate was opaque. Without per-cycle explainability, operators cannot diagnose why their `max_parallel: 10` configuration results in fewer than 10 running lanes.

**Schema**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchState {
    /// Why ready work was or was not dispatched this cycle.
    /// One of the following mutually-exclusive reasons:
    pub reason: DispatchStateReason,

    /// For `Reason::Dispatched`: how many lanes were dispatched.
    pub dispatched_count: usize,

    /// For `Reason::MaxParallelFull`: number of slots consumed by running lanes.
    pub running_count: usize,

    /// For `Reason::MaxParallelFull`: max_parallel setting.
    pub max_parallel: usize,

    /// For `Reason::NoReadyLanes`: number of lanes that were blocked/failed.
    pub blocked_or_failed_count: usize,

    /// For `Reason::ReadyButUndispatched`: keys of lanes that were ready
    /// but not dispatched, with a reason per lane.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ready_undispatched: Vec<ReadyUndispatchedLane>,

    /// For `Reason::Evolving`: if evolve was running and blocked dispatch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evolve_blocked: Option<EvolveBlockInfo>,
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
    /// Ready lanes existed but were not dispatched (evolve blocking, maintenance, etc.).
    ReadyButUndispatched,
    /// Controller is settling (no running, no ready, no failed to replay).
    Settling,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyUndispatchedLane {
    pub lane_key: String,
    pub reason: String,
    /// Why this specific lane was skipped:
    /// - "max_parallel_reached" — slots exhausted by higher-priority lanes
    /// - "evolve_blocking" — evolve was running
    /// - "target_repo_stale" — repo not fresh enough for dispatch
    /// - "maintenance_mode" — program in maintenance
    /// - "unknown" — unclassified
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolveBlockInfo {
    pub evolve_duration_ms: u64,
    pub evolve_error: Option<String>,
}
```

**Placement**: Add `dispatch_state: DispatchState` to `AutodevCycleReport`. Compute it just before `execute_selected_lanes` is called. Populate `AutodevCycleReport` with it.

### R3.2 — Aggregate dispatch metrics must be accessible without parsing cycle reports

**What**: The `AutodevCurrentSnapshot` (returned in `AutodevReport.current`) must include aggregate dispatch metrics that summarize the entire run.

**Schema additions to `AutodevCurrentSnapshot`**:
```rust
pub struct AutodevCurrentSnapshot {
    // ... existing fields ...

    /// Total cycles where no lanes were dispatched.
    pub idle_cycles: usize,

    /// Total lanes dispatched since the run started.
    pub total_dispatched: usize,

    /// Total lanes that landed to trunk (integration complete).
    pub total_landed: usize,

    /// Total stale `running` lanes detected and reclaimed this run.
    pub stale_running_reclaimed: usize,

    /// Total bootstrap failures (run config missing, prompt not found, etc.).
    pub bootstrap_failures: usize,

    /// Total runtime path errors (fabro command not found, etc.).
    pub runtime_path_errors: usize,

    /// Current dispatch rate: dispatched / (dispatched + idle_cycles).
    pub dispatch_rate: f64,
}
```

**Computation**: Maintain running counters in the orchestrator loop:
- `idle_cycles`: increment when `lanes_to_dispatch.is_empty()` and `!program_before.lanes.iter().any(|l| l.status == Running)`
- `stale_running_reclaimed`: increment when `refresh_program_state` transitions a `Running` lane to `Failed`
- `runtime_path_errors`: increment when `run_fabro()` or `run_synth_evolve()` returns a spawn/io error

---

## Requirement 4: Evolve Decoupling

### R4.1 — `synth evolve` must not block the dispatch cycle

**What**: When `should_trigger_evolve` returns `true`, the `run_synth_evolve()` call must execute in a background thread or be deferred to a future cycle, such that `execute_selected_lanes()` is called in the same cycle.

**Why**: With a 120-second timeout, blocking `synth evolve` can delay dispatch by up to 2 minutes per cycle. In a 5-second poll interval environment, this means up to 24 cycles worth of dispatch opportunities are lost.

**Architecture**:
```
Current (blocking):
  [refresh] → [evaluate] → [evolve (120s blocking)] → [dispatch] → [watch]

Target (non-blocking):
  [refresh] → [evaluate] → [dispatch] ─────────────────────────────────┐
       ↓ (background, async)                                           │
  [evolve (120s, runs in background thread)] ─────────────────────────►│
                                        ↓                               │
                              [evaluate post-evolve]                    │
                                        ↓                               │
                              [watch] ◄────────────────────────────────┘
```

**Implementation strategy**:

Add a `pending_evolve: Option<JoinHandle<Result<()>>>` field to the orchestrator state. When `should_trigger_evolve` returns `true`:

1. **If no evolve is currently running**: spawn `run_synth_evolve` in a background thread, store the `JoinHandle`. Record `last_evolve_at = Instant::now()` and `last_evolve_frontier = Some(frontier_before)`. Proceed to dispatch.
2. **If an evolve is currently running** (handle not yet complete): skip dispatching new lanes (`lanes_to_dispatch = Vec::new()`) — wait for evolve to finish before dispatching, because evolve may invalidate the ready lane set.
3. **On the next cycle**: if the evolve thread has completed, join it. If it succeeded, reload the manifest and re-evaluate. If it failed, log the error and continue.

**Critical invariant**: We must never dispatch lanes using a manifest that has been invalidated by an in-progress evolve. The `frontier_before` frontier signature is captured before evolve starts; if evolve changes the manifest, dispatching based on `program_before` would use stale lane definitions.

**Alternative (simpler) approach — evolve every N cycles, never blocking**:

Move evolve to a timer-based trigger that runs between dispatch cycles. The autodev loop dispatches every `poll_interval_ms`. Evolve runs every `evolve_every_seconds` seconds, but only when no lanes are currently running (`frontier.running == 0`). This is the simplest change: add a `last_evolve_check: Instant` field, and only call `run_synth_evolve` when:
```rust
if last_evolve_check.elapsed() >= evolve_every
    && frontier.running == 0
    && (locally_settled || !has_ready_before)
{
    // run evolve (blocking), but it's acceptable because nothing is running
}
```

This is acceptable because evolve when nothing is running doesn't delay any dispatch opportunity.

**Chosen approach**: Implement the background-thread approach (Option 1) because it allows evolve to run even when slots are occupied, which is needed for the `RegenerateLane` recovery action.

### R4.2 — Dispatch must consume the full `max_parallel` budget when ready lanes exist

**What**: In any cycle where `available_slots > 0` and `ready_lanes > 0`, `lanes_to_dispatch` must contain at least `min(available_slots, ready_lanes)` lanes.

**Why**: The `select_ready_lanes_for_dispatch()` function already implements this correctly. The bug was that stale `running` lanes consumed slots without actually running, and evolve blocking delayed dispatch. This requirement ensures the fix is validated.

**Validation**:
```rust
// In a test with 15 ready lanes and max_parallel=10:
// After dispatch, running count must be 10 (or less if fewer than 10 were ready)
assert!(program_after.lanes.iter().filter(|l| l.status == Running).count() <= 10);
assert!(program_after.lanes.iter().filter(|l| l.status == Running).count() > 0); // unless no lanes were ready
```

---

## Requirement 5: Live Validation Criteria

### V1 — Sustain 10 active lanes on rXMRbro

**Command**:
```bash
cargo build --release -p fabro-cli -p raspberry-cli --target-dir target-local && \
target-local/release/raspberry autodev \
  --manifest /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml \
  --max-parallel 10 --max-cycles 20
```

**Pass criteria**:
1. Controller enters `InProgress` state and maintains it for all 20 cycles
2. Every cycle report shows `running >= 8` (allowing 2 slots for brief gaps during lane completion)
3. `AutodevCurrentSnapshot.idle_cycles <= 5` for the full 20-cycle run
4. `failed_bootstrap_count = 0` for all cycles 3-20 (bootstrap failures in cycles 1-2 are acceptable)
5. No `runtime_path_errors` — the `fabro` binary must expose `synth`, and prompt refs must resolve correctly

### V2 — At least 3 lanes land to trunk

**Pass criteria**:
After the full 20-cycle run or when the controller settles, at least 3 lanes have `landing_state = "landed"` in their `trunk_delivery_state_for_lane()` detail, meaning their integration artifacts were pushed to `origin/main`.

---

## Implementation Order

1. **R1.1**: Verify `fabro-synthesis` is always compiled in `fabro-cli`. Add a test that `fabro synth --help` succeeds.
2. **R2.1 + R2.2**: Existing `stale_active_progress` logic is correct. Add integration tests simulating dead workers. Verify no regression.
3. **R2.3**: Fix `dispatch.rs` to call `mark_lane_dispatch_failed()` when `run_fabro` returns pre-spawn errors. Add `DispatchError` variants to the failure classifier.
4. **R3.1**: Add `DispatchState` to `AutodevCycleReport`. Compute it before `execute_selected_lanes`. Populate `ready_undispatched` with the reason each ready-but-undispatched lane was skipped.
5. **R3.2**: Add aggregate counters to `AutodevCurrentSnapshot`. Increment in the orchestrator loop.
6. **R4.1**: Refactor `run_synth_evolve` to run in a background thread. Store `pending_evolve` handle. Wait for it on the next cycle before dispatching.
7. **V1 + V2**: Live validation on rXMRbro.

---

## Key Files and Their Roles

| File | Role in This Spec |
|------|-------------------|
| `lib/crates/fabro-cli/src/main.rs` | Routes `Command::Synth` — must always include synthesis commands |
| `lib/crates/fabro-cli/src/commands/synth.rs` | Implements `synth evolve` — called by autodev |
| `lib/crates/raspberry-supervisor/src/autodev.rs` | Orchestrator loop — must emit dispatch telemetry, decouple evolve |
| `lib/crates/raspberry-supervisor/src/dispatch.rs` | `run_fabro()` — must update lane state on pre-spawn errors |
| `lib/crates/raspberry-supervisor/src/program_state.rs` | `refresh_program_state`, `stale_active_progress` — stale lane detection |
| `lib/crates/raspberry-supervisor/src/failure.rs` | `classify_failure` — must classify bootstrap errors as `TransientLaunchFailure` |
| `lib/crates/fabro-graphviz/src/` | Workflow graph parser — must resolve `@prompts/...` relative to graph file |
| `lib/crates/fabro-synthesis/src/render.rs` | Blueprint renderer — must emit workflow graphs with repo-relative prompt refs |

---

## Decision Log

- **Decision**: Implement background-thread evolve (R4.1) instead of interval-gated evolve.
  **Rationale**: Interval-gated evolve (`only when running == 0`) prevents evolve from running during the most active cycles, when regenerable failed lanes most need it. Background threading allows evolve to run during active dispatch without blocking.
  **Date/Author**: 2026-03-26 / Spec Author

- **Decision**: Emit dispatch telemetry as structured data in `AutodevCycleReport` rather than log lines.
  **Rationale**: Log lines are not machine-parseable in dashboards. The `AutodevReport` JSON is already persisted and parsed by the Paperclip dashboard. Adding `dispatch_state` to the cycle report makes it observable without adding new infrastructure.
  **Date/Author**: 2026-03-26 / Spec Author

- **Failure scenario**: If the background evolve thread takes longer than `poll_interval_ms` to complete, subsequent cycles may dispatch based on a pre-evolve manifest. This is acceptable because the evolve thread will be joined on the next cycle, and the stale manifest will be detected by the doctrine fingerprint or by a subsequent evolve re-trigger. The invariant "never dispatch using an invalidated manifest" is preserved by waiting for evolve to complete before the next dispatch.
  **Date/Author**: 2026-03-26 / Spec Author
