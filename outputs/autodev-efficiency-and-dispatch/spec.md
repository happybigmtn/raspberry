# Autodev Execution Path and Dispatch Truth — Specification

## Purpose

This spec defines the behavior that the autodev loop must exhibit after Phase 0 gate work is complete. It covers three concerns: runtime-path consistency (the binaries and generated packages agree about where commands and assets live), lane-state truth (stale `running` and `failed` lanes are reconciled before dispatch consumes worker slots), and dispatch telemetry (operators can observe why ready work did or did not run).

## Current State

The autodev loop is in `lib/crates/raspberry-supervisor/src/autodev.rs`. Each cycle:

1. Refreshes program state from `.raspberry/*-state.json` via `refresh_program_state()`
2. Re-evaluates lane statuses via `evaluate_program()` (which calls `evaluate_with_state`)
3. Optionally runs `synth evolve` synchronously before dispatch
4. Dispatches ready lanes via `dispatch.rs`
5. Watches for completion or timeout
6. Updates program state

Live observations from 2026-03-26 on rXMRbro:

- A clean `fabro` binary was suspected to not expose `synth`. The `Synth` variant IS registered in `main.rs`, but binary availability must be verified for both debug and release builds.
- Newly dispatched lanes failed bootstrap validation because copied `graph.fabro` files referenced prompts as `@../../prompts/...`, resolving under `~/.fabro/` at runtime instead of the target repo.
- After adding a temporary prompt symlink and using a synth-enabled binary, `rXMRbro` returned to `running: 10`, `ready: 23`, `failed: 0`.

## Specification

### 1. Runtime Path Self-Consistency

#### 1.1 `fabro synth` command availability

Both the debug and release `fabro` binaries must expose all synthesis commands that the autodev loop invokes: `synth create`, `synth evolve`, `synth import`, `synth genesis`.

**Verification**: `cargo build --release -p fabro-cli && target/release/fabro synth --help` exits 0 and lists all subcommands.

**Verification**: `cargo build -p fabro-cli && target/debug/fabro synth --help` exits 0 and lists all subcommands.

**Note**: The `Synth` variant is already registered in `lib/crates/fabro-cli/src/main.rs` at line 181. This item verifies the registration is not gated behind an inactive feature flag and that the binary build does not silently omit the subcommand tree.

#### 1.2 Prompt reference resolution in copied workflow graphs

Generated workflow graphs (copied to the run directory) must resolve prompt references from the target repo, not from `~/.fabro/prompts`.

Prompt references in Graphviz workflow graphs use the syntax `@<path>`. The path may be relative (e.g., `@prompts/review.md`) or contain `../` segments. When a workflow graph is copied to the detached run directory under `~/.fabro/runs/<run-id>/`, relative prompt paths must resolve to the target repo, not to the run directory.

**Acceptable resolution strategies** (any one is sufficient):

- Resolve prompt paths relative to the target repo at render time, so the copied graph contains absolute paths or paths relative to the run dir that resolve correctly.
- Embed prompt content inline in the copied graph.
- Copy prompt files to the run directory alongside the graph.
- The `run_fabro` function in `dispatch.rs` already sets `current_dir` to `target_repo`. If the graph is written with paths relative to `target_repo`, and `fabro run` is invoked with `current_dir = target_repo`, then relative paths resolve correctly.

**Verification**: Run a lane whose graph references a prompt. Inspect the copied graph in `~/.fabro/runs/<run-id>/`. The prompt path resolves to a file that exists under the target repo, not under `~/.fabro/`.

#### 1.3 Run config validation before dispatch

Before dispatching a lane, the autodev loop must validate that the run config and its graph file exist, and that the graph references are resolvable.

This is partially implemented: `run_fabro` in `dispatch.rs` returns `DispatchError::MissingRunConfig` if `run_config.exists()` is false. However, the validation does not check whether the graph file exists or whether referenced prompts/artifacts are resolvable.

**Verification**: Dispatch a lane whose run config points to a non-existent graph. The dispatch is rejected with a clear error, not a silent bootstrap failure.

#### 1.4 No local-only shims in the autodev runtime path

The autodev loop must not require symlinks, environment variables, or files outside the target repo and `~/.fabro/` to function. Specifically:

- No `prompts/` symlink in the target repo root that autodev creates as a rescue step.
- No `$HOME/.fabro/prompts` directory that must exist for workflows to resolve prompts.
- The `fabro_bin` path in `AutodevSettings` must be a path that exists and is executable.

**Verification**: Run autodev on a fresh clone of a repo with a synthesized package. No manual setup beyond `fabro install` is required.

### 2. Stale Lane State Reconciliation

#### 2.1 Stale `running` lanes are detected and reclassified before dispatch

A lane with `status = Running` in `program_state` is stale when:

- The tracked run directory does not exist, OR
- The worker process for the run is no longer alive (checked via `/proc/<pid>/` on Linux), OR
- The run has been active longer than `STALE_RUNNING_GRACE_SECS` (currently 30 seconds) with no worker process.

`refresh_program_state()` in `program_state.rs` already detects stale running lanes. However, `evaluate_program()` re-evaluates lane status from the run snapshot without propagating the staleness decision from `refresh_program_state`.

**Root cause**: In `evaluate_lane()` in `evaluate.rs`, the `run_snapshot` is loaded from the run directory's `state.json` and `status.json`. For a stale run, these files may not exist or may not reflect the staleness. The `classify_lane` function calls `is_active(run_snapshot, runtime_record)` which checks `run_snapshot.status.is_active()` first — if the snapshot is missing or stale, this returns `false`, and then the fallback to `runtime_record.status == Running && last_finished_at.is_none()` also returns `true` (since the record is still `Running` with no finish time).

The `refresh_program_state` function correctly updates the record to `Failed` with `failure_kind = TransientLaunchFailure` and clears `current_run_id`. The problem is that after `refresh_program_state` saves the corrected state, `evaluate_program` is called again and re-classifies the lane using `is_active`, which sees the (now-absent) run snapshot and falls through to the runtime record — which may have been updated, but the evaluation uses a different code path.

**Required fix**: After `refresh_program_state` updates a stale lane to `Failed`, the subsequent `evaluate_program` call must observe this corrected state. The `classify_lane` function's `is_active` check should prefer the runtime record's authoritative status over the run snapshot when they disagree. Specifically, if `runtime_record.status != Running` (after `refresh_program_state` has corrected it), `is_active` should return `false`.

**Verification**: Create a lane with `status = Running` in state but no corresponding run directory. Run `raspberry status`. The lane is reported as `failed`, not `running`.

#### 2.2 Stale `failed` lanes are re-evaluated after render changes

A lane marked `Failed` with `failure_kind = RegenerateNoop | DeterministicVerifyCycle | ProviderPolicyMismatch` becomes stale when the run config or graph file has been updated (render inputs changed after `last_finished_at`). The `stale_failure_superseded_by_render()` function in `program_state.rs` detects this and should re-evaluate the lane.

**Current behavior**: `refresh_program_state` calls `stale_failure_superseded_by_render` and, if true, resets the lane to `Blocked` and clears failure fields. This allows the lane to be re-evaluated and potentially become `Ready`.

**Verification**: A lane that failed with `RegenerateNoop` and has its run config's graph file touched (simulating a re-render). The lane transitions from `failed` to `ready` in the next evaluation cycle.

#### 2.3 No dispatch slot consumption by stale lanes

Stale lanes must not count toward `max_parallel` for dispatch purposes. The dispatch logic in `autodev.rs` computes `available_slots = max_parallel - count(running)`. If stale `Running` lanes are counted, available slots are reduced incorrectly.

**Current behavior**: `count_lanes_with_status(&program_before, LaneExecutionStatus::Running)` includes all lanes with `status == Running` in the evaluated program. If stale lanes are misclassified as `Running`, they consume dispatch slots.

**Required fix**: After the state refresh + evaluation, the program must reflect corrected lane statuses. Stale lanes that were `Running` but are now `Failed` must not appear in the running count.

**Verification**: With 10 genuinely running lanes and 3 stale `Running` lanes (run dirs deleted), `available_slots` is computed as `max(0, max_parallel - 10)`, not `max(0, max_parallel - 13)`.

### 3. Dispatch-State Telemetry

#### 3.1 Cycle-level dispatch telemetry fields

The `AutodevCycleReport` struct must expose why dispatch did or did not consume available slots. Add the following fields to `AutodevCycleReport`:

```rust
pub struct AutodevCycleReport {
    // ... existing fields ...
    
    /// Why ready work did not run this cycle
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_skip_reason: Option<DispatchSkipReason>,
    
    /// Lanes that were ready but not dispatched this cycle
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ready_undispatched: Vec<String>,
    
    /// Runtime path errors encountered during dispatch this cycle
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_path_errors: Vec<RuntimePathError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DispatchSkipReason {
    /// No lanes were ready
    NoReadyLanes,
    /// No available dispatch slots (max_parallel reached)
    SlotExhausted { available: usize, ready: usize },
    /// Target repo is stale
    TargetRepoStale,
    /// Program is in maintenance mode
    MaintenanceMode,
    /// No lanes passed dispatch selection (e.g., family diversity constraint)
    SelectionExhausted { candidates: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePathError {
    pub lane_key: String,
    pub error_type: String,
    pub message: String,
}
```

**When to populate**:
- `dispatch_skip_reason`: Set when `lanes_to_dispatch` is empty and `ready_lanes` is non-empty (after selection). Captures why the selection logic did not produce work.
- `ready_undispatched`: List of ready lane keys that were not selected (after family-diversity filtering).
- `runtime_path_errors`: Captures `DispatchError::MissingRunConfig`, `DispatchError::MissingProgramManifest`, and similar runtime-path failures encountered during `execute_selected_lanes`.

#### 3.2 Program-level dispatch summary

Add a dispatch summary to `AutodevCurrentSnapshot`:

```rust
pub struct AutodevCurrentSnapshot {
    // ... existing fields ...
    
    /// Rolling dispatch statistics (per-program)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_summary: Option<DispatchSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchSummary {
    /// Total cycles with at least one dispatch
    pub cycles_with_dispatch: usize,
    /// Total cycles where ready work existed but was not dispatched
    pub idle_cycles: usize,
    /// Total dispatch outcomes this session
    pub total_dispatched: usize,
    /// Failed bootstrap count (dispatch failed due to runtime path error)
    pub failed_bootstrap: usize,
    /// Stale running lanes reclaimed this session
    pub stale_running_reclaimed: usize,
}
```

**Note**: `idle_cycles` counts cycles where ready lanes existed but `lanes_to_dispatch` was empty. This is distinct from "settled" (no ready and no running lanes).

#### 3.3 `raspberry status` display of dispatch telemetry

The `raspberry status` output must show the `dispatch_summary` fields in human-readable form. At minimum:

```
Dispatch: cycles_with_dispatch=N idle_cycles=N total_dispatched=N failed_bootstrap=N stale_reclaimed=N
```

When `idle_cycles` increases over multiple invocations, operators should investigate (e.g., `raspberry status --verbose` shows which lanes were ready but undispatched).

### 4. Decouple Evolve from Dispatch (Milestone 4)

`run_synth_evolve()` currently runs synchronously before dispatch in every cycle where `should_trigger_evolve` returns true. This delays dispatch and can cause the cycle to exceed the poll interval.

**Target behavior**: `run_synth_evolve` runs in a background task (separate thread or async task) when triggered. Dispatch proceeds immediately without waiting for evolve to complete. The next cycle sees the evolved package if evolve completed before the next poll.

**Implementation options** (choose one):

1. **Thread-based decoupling**: Spawn `run_synth_evolve` in a spawned thread. Store `last_evolve_at` optimistically. On the next cycle, check if evolve completed and reload the package.
2. **Async decoupling**: Make the autodev loop async and use `tokio::spawn` for evolve.
3. **Cadence-gated**: Only run evolve when `frontier.ready == 0 && frontier.running == 0` (locally settled). This is already partially implemented in `should_trigger_evolve` but the evolve call still blocks.

**Recommended**: Option 1 (thread-based) — minimal API surface change, works with the current sync codebase.

**Verification**: With `evolve_every_seconds > 0` and ready lanes present, dispatch occurs in the same cycle as evolve is triggered (no blocking delay).

### 5. Greedy Dispatch (Milestone 4)

The dispatch logic must consume the full `max_parallel` budget in one cycle when ready lanes exist. Currently:

```rust
let selected_ready_lanes = select_ready_lanes_for_dispatch(&program, remaining_slots, &replayed_lanes);
```

If `remaining_slots > 0` and `ready_lanes > 0`, `select_ready_lanes_for_dispatch` should return up to `remaining_slots` lanes, subject to family-diversity constraints.

**Verification**: With `max_parallel = 10`, 15 ready lanes from 5 distinct families, and no running lanes: exactly 10 lanes are dispatched in the first cycle. The remaining 5 are dispatched in cycle 2 (if slots free up).

## Acceptance Criteria

1. **Runtime path**: A synthesized package runs on a fresh clone without local-only shims. `fabro synth --help` works in both debug and release builds. Workflow graphs resolve prompt references from the target repo.
2. **Stale state**: A lane with a deleted run directory is reported as `failed` (not `running`) in `raspberry status`. Stale lanes do not consume dispatch slots.
3. **Telemetry**: `AutodevReport` JSON shows `dispatch_skip_reason`, `ready_undispatched`, `runtime_path_errors`, and `dispatch_summary` fields. `raspberry status` shows dispatch stats.
4. **Live validation**: `raspberry autodev --max-parallel 10` on rXMRbro sustains 10 active lanes for 20 cycles without bootstrap failures. At least 3 lanes land to trunk.
5. **Evolve decoupling**: With evolve enabled, dispatch is not blocked by evolve. Ready lanes are dispatched within the first poll interval of the cycle.

## Non-Goals

- This spec does not address review quality or sprint contracts. Those are handled in Plan 006.
- This spec does not add new failure classification categories beyond what exists in `failure.rs`.
- This spec does not change the Graphviz workflow graph format or the synthesis render logic (beyond ensuring prompt paths are resolvable).

## Files In Scope

- `lib/crates/raspberry-supervisor/src/autodev.rs` — autodev loop, cycle telemetry
- `lib/crates/raspberry-supervisor/src/dispatch.rs` — dispatch execution, runtime path errors
- `lib/crates/raspberry-supervisor/src/evaluate.rs` — lane classification, `is_active` logic
- `lib/crates/raspberry-supervisor/src/program_state.rs` — state refresh, stale detection
- `lib/crates/raspberry-supervisor/src/failure.rs` — failure classification (read-only)
- `lib/crates/fabro-cli/src/main.rs` — CLI command registration (read-only, verify)
- `lib/crates/fabro-synthesis/src/render.rs` — workflow graph rendering (read-only, verify prompt resolution)
- `lib/crates/fabro-cli/src/commands/synth.rs` — synth command (read-only, verify registration)

## Open Questions

1. **Priority dispatch vs. family diversity**: The current `select_ready_lanes_for_dispatch` enforces family diversity (max 1 lane per plan family per dispatch round). This limits parallelism when many lanes share a prefix. Is this the intended behavior? If so, the telemetry should note that `ready_undispatched` includes lanes skipped due to family diversity.

2. **Evolve blocking on first cycle**: Even when `evolve_every_seconds = 0`, `should_trigger_evolve` returns `true` on the first cycle (because `last_evolve_at` is `None`). This means the first cycle always runs evolve synchronously. Is this acceptable, or should the first cycle skip evolve to get to dispatch faster?

3. **Stale grace period**: `STALE_RUNNING_GRACE_SECS = 30` means a dead lane stays `Running` for 30 seconds before being reclassified. Is this appropriate for autodev cycles that poll every 5 seconds? Reducing this to 0 (immediate detection) would reclaim slots faster but might misclassify slow-starting runs.
