# Autodev Execution Path and Dispatch Truth — Specification

## Purpose

Fix three classes of failure in the autodev loop on `rXMRbro`:

1. **Runtime-path consistency** — binaries expose the commands autodev calls; workflow graphs resolve prompts from the target repo, not from `~/.fabro/`.
2. **Lane-state truth** — stale `running` lanes are reclassified before dispatch; stale `failed` lanes are re-evaluated after render changes.
3. **Dispatch telemetry** — operators can observe why ready work did or did not run in a given cycle.

## Gate Criterion (Phase 0)

> On rXMRbro, `raspberry autodev --max-parallel 10` sustains 10 running lanes for at least 20 cycles, produces zero bootstrap-time validation failures caused by missing CLI subcommands or unresolved prompt/workflow refs, and lands at least 3 lanes to trunk.

## 1. Runtime Path Self-Consistency

### 1.1 `fabro synth` command availability in both builds

**Problem**: The autodev loop calls `synth evolve` via `run_synth_evolve()` (autodev.rs:2333). The `Synth` enum variant is registered in `main.rs:181`, but the binary availability was unverified for debug and release builds.

**Requirement**: Both `cargo build -p fabro-cli` and `cargo build --release -p fabro-cli` must produce binaries that accept `fabro synth --help` and list all subcommands: `create`, `evolve`, `import`, `genesis`, `review`.

**Verification**:
```bash
cargo build --release -p fabro-cli && target/release/fabro synth --help | grep -q "create\|evolve"
cargo build -p fabro-cli && target/debug/fabro synth --help | grep -q "create\|evolve"
```

### 1.2 Prompt reference resolution in copied workflow graphs

**Problem**: Copied `graph.fabro` files use `@../../prompts/...` syntax. When copied to `~/.fabro/runs/<run-id>/`, these resolve under `~/.fabro/` instead of the target repo.

**Current resolution**: `run_fabro()` in dispatch.rs:496 sets `current_dir = target_repo` before invoking `fabro run`. If the copied graph contains paths relative to `target_repo`, they resolve correctly. However, the render path was copying graphs with `malinka/`-relative paths.

**Requirement**: The copied graph must contain paths that resolve from `target_repo` when `fabro run` is invoked with `current_dir = target_repo`.

**Verification**: Inspect `~/.fabro/runs/<run-id>/graph.fabro` for a lane whose manifest references a prompt. The `@<path>` references must point into the target repo, not into `~/.fabro/`.

### 1.3 Pre-flight validation before dispatch

**Problem**: `run_fabro()` (dispatch.rs:502) returns `DispatchError::MissingRunConfig` if the run config is absent, but does not validate the graph file or prompt resolvability.

**Requirement**: Before dispatching, validate:
- The run config file exists.
- The graph file referenced by the run config exists.
- Prompt paths in the graph resolve (fail-fast, not bootstrap failure).

**Verification**: Dispatch a lane whose run config points to a non-existent graph. The dispatch is rejected with `DispatchError::MissingRunConfig` or a new `RuntimePathInvalid` variant before a slot is consumed.

### 1.4 No local-only shims

**Problem**: Autodev previously required a `prompts/` symlink in the target repo root and `$HOME/.fabro/prompts` to exist.

**Requirement**: A synthesized package runs on a fresh clone with only `fabro install` pre-work. No manual symlinks or environment variables.

**Verification**: On a fresh clone, `raspberry autodev --max-parallel 10` dispatches lanes without operator-created symlinks.

## 2. Stale Lane State Reconciliation

### 2.1 Stale `running` lanes detected before dispatch

**Problem**: `refresh_program_state()` (program_state.rs:383) correctly detects stale running lanes and updates their status to `Failed` with `failure_kind = TransientLaunchFailure`. However, `evaluate_program()` (evaluate.rs:194) is called after refresh and re-evaluates using `classify_lane()` (evaluate.rs:941), which calls `is_active()` (evaluate.rs:1021).

The `is_active()` function has two alternatives:
```rust
fn is_active(run_snapshot: &RunSnapshot, runtime_record: Option<&LaneRuntimeRecord>) -> bool {
    run_snapshot.status.map(RunStatus::is_active).unwrap_or(false)  // [A]
        || runtime_record.map(|r| r.status == Running && r.last_finished_at.is_none()).unwrap_or(false)  // [B]
}
```

When the run directory is absent: [A] returns `false` (no snapshot), [B] returns `true` if `runtime_record` still shows `Running` with no `last_finished_at`. Since `refresh_program_state` already updated the record to `Failed`, [B] should return `false`. But if `evaluate_program` runs `refresh_program_state` first (evaluate.rs:222) and then `classify_lane` is called with a fresh `runtime_record` loaded from the already-corrected state, the classification should be consistent.

**Root cause confirmed**: `evaluate_program_internal` (evaluate.rs:202) calls `refresh_program_state` first, which updates stale lanes to `Failed`. Then `classify_lane` evaluates with the updated record. The classification should be consistent.

**However**, there is a race condition window: if `refresh_program_state` updates a lane to `Failed` but the process crashes before the state file is fsynced, on restart the lane may still appear `Running`.

**Fix**: After `refresh_program_state` marks a lane `Failed`, the same evaluation cycle must not re-classify it as `Running`. Add an assertion that when `runtime_record.status != Running`, `is_active()` returns `false` for that lane.

**Verification**: Create a lane with `status = Running` in `program_state.json` but delete its run directory. Run `raspberry status`. The lane is `failed`, not `running`.

### 2.2 Stale `failed` lanes re-evaluated after render changes

**Problem**: A lane marked `Failed` with `RegenerateNoop | DeterministicVerifyCycle | ProviderPolicyMismatch` becomes stale when its render inputs change after `last_finished_at`.

**Current behavior**: `refresh_program_state` calls `stale_failure_superseded_by_render()` (program_state.rs:1420) which resets the lane to `Blocked` if render inputs are newer. This is correct.

**Verification**: Touch the graph file of a lane that failed with `RegenerateNoop`. The lane transitions from `failed` to `ready` in the next evaluation.

### 2.3 Stale lanes do not consume dispatch slots

**Problem**: `available_slots = max_parallel - count(running)` (autodev.rs:546) counts all lanes with `status == Running`. Stale lanes incorrectly reduce available slots.

**Requirement**: After state refresh, stale `Running` lanes that were reclassified to `Failed` must not appear in the running count.

**Verification**: With 10 genuinely running lanes and 3 stale `Running` lanes (run dirs deleted), `available_slots` is `max(0, max_parallel - 10)`, not `max(0, max_parallel - 13)`.

## 3. Dispatch-State Telemetry

### 3.1 Cycle-level telemetry fields

Add to `AutodevCycleReport` (autodev.rs:96):

```rust
/// Why dispatch did not occur when ready lanes existed
#[serde(default, skip_serializing_if = "Option::is_none")]
pub dispatch_skip_reason: Option<DispatchSkipReason>,

/// Ready lanes not dispatched this cycle
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub ready_undispatched: Vec<String>,

/// Runtime path errors encountered during dispatch
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub runtime_path_errors: Vec<RuntimePathError>,
```

New types:
```rust
pub enum DispatchSkipReason {
    NoReadyLanes,
    SlotExhausted { available: usize, ready: usize },
    TargetRepoStale,
    MaintenanceMode,
    SelectionExhausted { candidates: usize },
}

pub struct RuntimePathError {
    pub lane_key: String,
    pub error_type: String,  // "MissingRunConfig" | "MissingProgramManifest" | "RuntimePathInvalid"
    pub message: String,
}
```

**When to populate**: `dispatch_skip_reason` when `lanes_to_dispatch` is empty and `ready_lanes` is non-empty. `runtime_path_errors` from `DispatchError::MissingRunConfig`, `MissingProgramManifest`, `Spawn`, `Lease`.

### 3.2 Program-level dispatch summary

Add to `AutodevCurrentSnapshot`:
```rust
pub struct DispatchSummary {
    pub cycles_with_dispatch: usize,
    pub idle_cycles: usize,            // ready existed but nothing dispatched
    pub total_dispatched: usize,
    pub failed_bootstrap: usize,        // runtime path errors
    pub stale_running_reclaimed: usize,  // stale running → failed transitions
}
```

Note: This is per-session. On process restart, counters reset. Document this explicitly.

### 3.3 `raspberry status` display

Show dispatch stats in human-readable form:
```
Dispatch: cycles=N idle=N total=N failed_bootstrap=N stale_reclaimed=N
```

When `idle_cycles` increases across invocations, `raspberry status --verbose` shows `ready_undispatched` lane keys.

## 4. Evolve Decoupling from Dispatch

**Problem**: `run_synth_evolve()` (autodev.rs:2333) runs synchronously in every cycle where `should_trigger_evolve` returns true, blocking dispatch.

**Requirement**: Run `synth evolve` in a background thread. Dispatch proceeds without waiting. The next cycle sees the evolved package if evolve completed.

**Implementation**: Thread-based decoupling (Option 1 in spec). Spawn `run_synth_evolve` in a spawned thread. Store `last_evolve_at` optimistically. Use atomic package updates (temp directory + rename) to prevent partial-evolution inconsistency.

**Verification**: With `evolve_every_seconds > 0` and ready lanes present, dispatch occurs in the same cycle as evolve is triggered.

## 5. Greedy Dispatch

**Problem**: `select_ready_lanes_for_dispatch` (autodev.rs:982) must fill `max_parallel` in one cycle when ready lanes exist.

**Current behavior**: The function already selects up to `available_slots` lanes. The verification is that with `max_parallel = 10` and 15 ready lanes, exactly 10 are dispatched in cycle 1.

## Acceptance Criteria

| # | Criterion | Verification |
|---|-----------|--------------|
| 1 | `fabro synth --help` works in debug and release builds | Binary smoke test |
| 2 | Prompt paths in copied graphs resolve from target repo | Inspect run-dir graph |
| 3 | Missing graph file → dispatch rejected before slot consumed | Unit test |
| 4 | Lane with deleted run dir → `raspberry status` shows `failed` | Unit test |
| 5 | Stale `running` lanes do not reduce `available_slots` | Derived count |
| 6 | `AutodevCycleReport` includes `dispatch_skip_reason`, `ready_undispatched`, `runtime_path_errors` | JSON inspection |
| 7 | `raspberry status` shows dispatch summary | Human-readable output |
| 8 | `rXMRbro`: 10 active lanes for 20 cycles, 0 bootstrap failures, 3 lanes to trunk | Live run |

## Files In Scope

| File | Role |
|------|------|
| `lib/crates/raspberry-supervisor/src/autodev.rs` | Cycle loop, telemetry structs, evolve |
| `lib/crates/raspberry-supervisor/src/dispatch.rs` | Dispatch execution, runtime path errors |
| `lib/crates/raspberry-supervisor/src/evaluate.rs` | `is_active`, `classify_lane` |
| `lib/crates/raspberry-supervisor/src/program_state.rs` | State refresh, stale detection |
| `lib/crates/fabro-cli/src/main.rs` | Synth command registration (verify only) |

## Open Questions

1. **Stale grace period**: `STALE_RUNNING_GRACE_SECS = 30`. Faster slot recovery vs. slow-start misclassification. Recommend 10s for Phase 0.
2. **First-cycle evolve blocking**: `should_trigger_evolve` returns `true` on first cycle (last_evolve_at is None). Acceptable for Phase 0.
3. **Family diversity constraint**: `select_ready_lanes_for_dispatch` limits 1 lane per plan family per cycle. Document that `ready_undispatched` includes diversity-skipped lanes.
