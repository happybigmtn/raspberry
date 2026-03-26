# Autodev Execution Path and Dispatch Truth — Capability Specification

**Lane:** `autodev-efficiency-and-dispatch`  
**Status:** Spec Complete — Awaiting Implementation  
**Genesis plan:** `genesis/plans/003-autodev-efficiency-and-dispatch.md`

---

## Purpose

Make the autodev execution path boringly reliable. No dispatch metric matters until the generated package and the runtime agree about where commands and assets live.

After this capability lands, `raspberry autodev` runs generated packages without local-only rescue steps:
- Generated workflows resolve prompt and artifact references correctly
- Binaries operators actually run expose the commands autodev depends on
- Stale state does not consume dispatch slots
- Ready lanes dispatch immediately within the configured budget

**Proof:** A bounded autodev run against a proving-ground repo (rXMRbro) sustains 10 active lanes, shows `failed: 0` for newly dispatched lanes during the first live cycles, and produces zero bootstrap-time validation failures.

---

## Scope

**In scope:**
- `raspberry-supervisor/src/autodev.rs` — main orchestrator, dispatch loop
- `raspberry-supervisor/src/dispatch.rs` — lane dispatch via `fabro run --detach`
- `raspberry-supervisor/src/program_state.rs` — lane runtime state, stale detection
- `raspberry-supervisor/src/evaluate.rs` — lane readiness evaluation
- `fabro-synthesis/src/render.rs` — workflow graph rendering

**Out of scope:**
- Review quality or promotion contracts → Plan 006
- Provider policy or quota management → Plan 008
- Error handling hardening for panics or unwraps → Plan 002

---

## Current State and Known Failures

### Failure 1: `fabro synth` Missing from Binary Surface

**Observation:** Live restart work on 2026-03-26 showed rXMRbro failing because a clean `fabro` binary did not expose `synth`, even though `fabro-cli/src/commands/synth.rs` still existed.

**Root cause:** `fabro-cli/src/main.rs` defines the `Synth` command, but some build configurations may not fully expose the command surface.

**Impact:** Every dispatched lane fails bootstrap validation immediately.

**Fix location:** Add startup probe `validate_fabro_command_surface()` that checks `fabro synth --help` before entering the dispatch loop.

---

### Failure 2: Prompt References Resolving Outside Target Repo

**Observation:** Generated `graph.fabro` files copied to the run directory reference prompts as `@../../prompts/...`, which resolve under `~/.fabro/` at runtime instead of the target repo.

**Root cause:** `fabro-synthesis/src/render.rs:1915` generates:
```rust
format!("@../../prompts/{}/{}/{}.md", lane.workflow_family(), lane.slug(), name)
```

**Impact:** Bootstrap validation fails for every lane because prompt files cannot be found.

**Fix location:** Use run-directory-relative or absolute paths in `render.rs`. The workflow graph should carry enough context to validate in the detached run environment.

---

### Failure 3: Stale `running` Lanes Consuming Dispatch Slots

**Observation:** Lanes marked `running` may be dead without triggering state reconciliation before dispatch.

**Root cause:** `program_state.rs:1423-1429` has `stale_active_progress_reason()` logic, but it may not fire reliably if `worker_process_alive()` returns `None` (uncertain).

**Current behavior:** `STALE_RUNNING_GRACE_SECS = 30` (program_state.rs:22). Lane is not marked stale if `worker_alive` is uncertain.

**Impact:** Dead lanes consume worker slots indefinitely.

**Fix location:** After `STALE_RUNNING_GRACE_SECS * 2` (60s), force reclassification of lanes with `worker_alive = None` as stale.

---

### Failure 4: Blocking `synth evolve` on Hot Path

**Observation:** `run_synth_evolve()` runs synchronously before dispatch in `autodev.rs:2156-2208`, creating dispatch delay even when ready work exists.

**Root cause:** `should_trigger_evolve()` returns true and blocks on evolve before dispatch occurs.

**Impact:** Dispatch latency under load; frontier under-utilization.

**Fix location:** Move `run_synth_evolve()` to background thread with cadence gating. Dispatch should proceed independently of evolve status.

---

### Failure 5: No Dispatch Telemetry

**Observation:** `AutodevCycleReport` (autodev.rs:96) does not expose fields that explain why ready work did or did not run.

**Current fields:** `cycle`, `evolved`, `evolve_target`, `ready_lanes`, `replayed_lanes`, `regenerate_noop_lanes`, `dispatched`, `running_after`, `complete_after`.

**Missing fields:** `dispatch_rate`, `idle_cycles`, `ready_but_undispatched`, `failed_bootstrap_count`, `runtime_path_errors`, `stale_running_reclaimed`.

**Impact:** Operational blindness — cannot debug dispatch decisions.

**Fix location:** Add `DispatchTelemetry` struct and populate fields in each dispatch cycle.

---

## Architecture Contracts

### Command Surface Invariant

The `fabro` binary must expose the following commands that autodev depends on:

| Command | Purpose | Validation |
|---------|---------|------------|
| `fabro run --detach <run-config>` | Lane execution | Must work in debug and release builds |
| `fabro validate <run-config>` | Bootstrap validation | Exit 0 for valid config |
| `fabro synth evolve <args>` | Package evolution | Must be accessible |
| `fabro synth create <args>` | Package creation | Must be accessible |
| `fabro synth import <args>` | Package import | Must be accessible |

**Startup validation:** Probe each command with `--help` before entering the dispatch loop. Fail fast with actionable error if any command is missing.

---

### Prompt Resolution Contract

Generated workflow graphs must resolve prompt references from (in priority order):
1. The target repo's `malinka/prompts/` directory
2. The run directory's local `prompts/` copy
3. An explicit `prompt_context` field in the run config

**Forbidden:** Workflow graphs must NOT resolve prompts from `~/.fabro/prompts/` or any other home-directory path.

**Fix:** Replace relative path generation (`@../../prompts/...`) with run-directory-relative paths or absolute paths that are stable when the workflow is copied to `~/.fabro/runs/<run-id>/`.

---

### Lane State Truth Contract

Before each dispatch cycle, the autodev loop must reconcile lane state:

1. **Check running lanes have active workers** — verify via `worker_process_alive()`
2. **Detect stale `running` within grace period** — `STALE_RUNNING_GRACE_SECS = 30`
3. **Force reclassification after extended uncertainty** — if `worker_alive = None` for 60s+, mark stale
4. **Reclassify stale `running` as `failed`** — with `FailureKind::TransientLaunchFailure`
5. **Allow immediate redispatch** — stale lanes must not block dispatch slots

---

### Dispatch Telemetry Contract

Each dispatch cycle must produce telemetry with the following fields:

| Field | Type | Description |
|-------|------|-------------|
| `dispatch_rate` | float | Ratio of dispatched lanes to available slots this cycle |
| `idle_cycles` | u32 | Consecutive cycles with zero dispatches |
| `ready_but_undispatched` | u32 | Ready lanes not dispatched due to budget |
| `failed_bootstrap_count` | u32 | Lanes that failed bootstrap validation |
| `runtime_path_errors` | u32 | Lanes that failed due to missing commands/assets |
| `stale_running_reclaimed` | u32 | Running lanes reclassified as failed this cycle |

---

## Key Files and Interfaces

### `raspberry-supervisor/src/autodev.rs`

| Function | Line | Role |
|----------|------|------|
| `orchestrate_program()` | ~369 | Main entry point; must call dispatch after evolve |
| `should_trigger_evolve()` | ~895 | Must not block dispatch when ready lanes exist |
| `run_synth_evolve()` | ~2156 | Should run async, not blocking |
| `AutodevCycleReport` | 96 | Needs telemetry fields added |

### `raspberry-supervisor/src/dispatch.rs`

| Function | Line | Role |
|----------|------|------|
| `execute_selected_lanes()` | — | Entry point for lane dispatch |
| `run_fabro()` | — | Spawns `fabro run --detach` for a single lane |
| `DispatchOutcome` | 26 | Needs error classification fields |

### `raspberry-supervisor/src/program_state.rs`

| Function | Line | Role |
|----------|------|------|
| `refresh_program_state()` | — | Reads run progress, updates lane status |
| `stale_active_progress_reason()` | 1429 | Detects dead running lanes |
| `STALE_RUNNING_GRACE_SECS` | 22 | Currently 30 seconds |

### `raspberry-supervisor/src/evaluate.rs`

| Function | Role |
|----------|------|
| `evaluate_program()` | Main evaluation entry point |
| `classify_lane()` | Determines lane status (Running/Failed/Ready/etc.) |
| `is_active()` | Checks if lane has live progress |

### `fabro-synthesis/src/render.rs`

| Function | Line | Role |
|----------|------|------|
| `render_lane()` | — | Renders a single lane's workflow graph |
| `render_workflow()` | — | Writes the graph.fabro file |
| **prompt_path closure** | **1915** | **Generates wrong `@../../prompts/` path — fix required** |

### `fabro-cli/src/main.rs`

- `Synth` command with subcommands must be registered
- Commands must be accessible in both debug and release builds

---

## Failure Handling

### Missing Command Surface → Fail Fast

```
If `fabro <command> --help` fails:
  → Log actionable error with missing command name
  → Do NOT proceed to dispatch
  → Exit with non-zero status
```

### Runtime Path Resolution Failure → Classify and Block

```
If lane fails due to missing prompt or asset:
  → Classify as FailureKind::RuntimePathError
  → Mark lane as failed with recovery_action = FailureRecoveryAction::SurfaceBlocked
  → Increment runtime_path_errors telemetry
  → Do NOT auto-replay
  → Require manual intervention or regeneration
```

### Stale Running Detection → Reclaim Slot

```
If lane is running but worker is dead:
  → Detect via worker_process_alive() returning Some(false)
  → Reclassify as failed with FailureKind::TransientLaunchFailure
  → Set recovery_action = FailureRecoveryAction::BackoffRetry
  → Increment stale_running_reclaimed telemetry
  → Slot becomes available for redispatch
```

---

## Milestones

### Milestone 1: Freeze Failure Modes into Tests

Create deterministic tests for:
- `fabro` binary without `synth` command
- Generated prompt refs resolving outside target repo
- Stale `running`/`failed` state preventing redispatch

**Proof:** `cargo nextest run -p raspberry-supervisor -- autodev`

### Milestone 2: Self-Consistent Runtime Paths

- `fabro` release/debug binaries both expose synthesis commands
- Generated prompt references resolve from target repo or run dir
- Copied workflow graphs validate in detached run environment

**Proof:** `fabro synth --help` works in both build modes.

### Milestone 3: Stale Running Reconciliation

- Dead `running` lanes reclassified within 60s even when `worker_alive = None`
- Dispatch slots reclaimed immediately

**Proof:** `cargo nextest run -p raspberry-supervisor -- program_state`

### Milestone 4: Decouple Evolve from Dispatch

- `run_synth_evolve()` runs async/cadence-gated, not blocking
- `dispatch.rs` consumes full `max_parallel` budget in one cycle

**Proof:** Ready lanes dispatch without waiting for evolve.

### Milestone 5: Dispatch Telemetry

- `AutodevCycleReport` includes all telemetry fields
- Operator can explain why work did or did not run

**Proof:** `raspberry status` shows dispatch telemetry fields.

### Milestone 6: Live Validation

- `raspberry autodev --max-parallel 10` on rXMRbro
- Sustains 10 running lanes for 20 consecutive cycles
- Zero bootstrap validation failures

**Proof:**
```bash
cargo build --release -p fabro-cli -p raspberry-cli
./target/release/raspberry autodev \
  --manifest /path/to/rxmragent.yaml \
  --max-parallel 10 --max-cycles 20
```

---

## Acceptance Criteria

| # | Criterion | Blocked By |
|---|-----------|------------|
| 1 | `fabro synth --help` works in debug and release builds | Failure 1 |
| 2 | `fabro run --detach` succeeds for lane with `@prompts/` references | Failure 2 |
| 3 | Lane marked `running` with dead worker reclassified as `failed` within 30s | Failure 3 |
| 4 | `AutodevCycleReport` includes `dispatch_rate`, `idle_cycles`, `ready_but_undispatched`, `failed_bootstrap_count`, `runtime_path_errors`, `stale_running_reclaimed` | Failure 5 |
| 5 | 10-lane autodev run sustains 10 running lanes for 20 cycles | Failures 1, 2, 4 |
| 6 | Zero bootstrap validation failures in first 10 dispatch cycles | Failures 1, 2 |

---

## Non-Goals

- Improving dispatch rate beyond consuming the full `max_parallel` budget (future optimization pass)
- Review quality scoring or promotion contracts (Plan 006)
- Provider policy stabilization (Plan 008)
