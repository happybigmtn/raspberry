# Autodev Execution Path and Dispatch Truth — Capability Specification

## Title and Status

**Status:** Draft for Review

**Lane:** `autodev-efficiency-and-dispatch`

## Purpose / User-Visible Outcome

After this capability lands, `raspberry autodev` runs generated packages without local-only rescue steps. Generated workflows resolve prompt and artifact references correctly, the binaries operators actually run expose the commands autodev depends on, stale state does not consume dispatch slots, and ready lanes dispatch immediately within the configured budget.

The proof is a bounded autodev run against a proving-ground repo that sustains 10 active lanes, shows `failed: 0` for newly dispatched lanes during the first live cycles, and produces zero bootstrap-time validation failures caused by missing CLI subcommands or unresolved prompt/workflow refs.

## Whole-System Goal

Make the autodev execution path boringly reliable. No dispatch metric matters until the generated package and the runtime agree about where commands and assets live.

## Scope

This spec covers the autodev runtime path in `raspberry-supervisor/src/` and the synthesis rendering pipeline in `fabro-synthesis/src/`. It does not cover:

- Review quality or promotion contracts (Plan 006)
- Provider policy or quota management (Plan 008)
- Error handling hardening for panics or unwraps (Plan 002)

## Current State

### Observed Failure Modes

1. **`fabro synth` missing from binary surface**: The `fabro-cli/src/main.rs` defines the `Synth` command, but in some build configurations the command surface may not be fully exposed. Live restart work on 2026-03-26 showed rXMRbro failing because a clean `fabro` binary did not expose `synth`.

2. **Prompt references resolving outside target repo**: Generated `graph.fabro` files copied to the run directory reference prompts as `@../../prompts/...`, which resolve under `~/.fabro/` at runtime instead of the target repo.

3. **Stale `running` lanes consuming dispatch slots**: The `program_state.rs` module has `stale_active_progress_reason()` logic, but lanes marked `running` may be dead without triggering state reconciliation before dispatch.

4. **Blocking `synth evolve` on hot path**: `run_synth_evolve()` runs synchronously before dispatch in `autodev.rs`, creating a dispatch delay even when ready work exists.

5. **No dispatch telemetry**: The current `AutodevReport` and `AutodevCycleReport` structs do not expose fields that explain why ready work did or did not run.

## Architecture / Runtime Contract

### Command Surface Invariant

The `fabro` binary must expose the following commands that autodev depends on:

- `fabro run --detach <run-config>` — lane execution
- `fabro validate <run-config>` — bootstrap validation
- `fabro synth evolve <args>` — package evolution
- `fabro synth create <args>` — package creation
- `fabro synth import <args>` — package import

These commands must work in both debug and release builds.

### Prompt Resolution Contract

Generated workflow graphs must resolve prompt references from:
1. The target repo's `malinka/prompts/` directory
2. The run directory's local `prompts/` copy
3. An explicit `prompt_context` field in the run config

Workflow graphs must NOT resolve prompts from `~/.fabro/prompts/` or any other home-directory path.

### Lane State Truth Contract

Before each dispatch cycle, the autodev loop must reconcile lane state by:
1. Checking that `running` lanes have active worker processes or live run directories
2. Detecting stale `running` state within `STALE_RUNNING_GRACE_SECS` (30 seconds)
3. Reclassifying stale `running` lanes as `failed` with `FailureKind::TransientLaunchFailure`
4. Allowing stale lanes to be redispatched immediately

### Dispatch Telemetry Contract

Each dispatch cycle must produce telemetry in operator-readable form:

| Field | Type | Description |
|-------|------|-------------|
| `dispatch_rate` | float | Ratio of dispatched lanes to available slots this cycle |
| `idle_cycles` | u32 | Consecutive cycles with zero dispatches |
| `ready_but_undispatched` | u32 | Ready lanes not dispatched due to budget |
| `failed_bootstrap_count` | u32 | Lanes that failed bootstrap validation |
| `runtime_path_errors` | u32 | Lanes that failed due to missing commands/assets |
| `stale_running_reclaimed` | u32 | Running lanes reclassified as failed this cycle |

## Key Files and Interfaces

### raspberry-supervisor/src/autodev.rs

The main orchestrator. Key functions to verify:

- `orchestrate_program()` — must call dispatch after evolve completes
- `should_trigger_evolve()` — must not block dispatch when ready lanes exist
- `run_synth_evolve()` — should run async/cadence-gated, not blocking

### raspberry-supervisor/src/dispatch.rs

Dispatches lanes via `fabro run --detach`. Key functions:

- `execute_selected_lanes()` — entry point for lane dispatch
- `run_fabro()` — spawns `fabro run --detach` for a single lane

### raspberry-supervisor/src/program_state.rs

Manages lane runtime state. Key functions:

- `refresh_program_state()` — reads run progress, updates lane status
- `stale_active_progress_reason()` — detects dead running lanes
- `LiveLaneProgress` — tracks run progress including `worker_alive`

### raspberry-supervisor/src/evaluate.rs

Evaluates lane readiness. Key functions:

- `evaluate_program()` — main evaluation entry point
- `classify_lane()` — determines lane status (Running/Failed/Ready/etc.)
- `is_active()` — checks if lane has live progress

### fabro-synthesis/src/render.rs

Renders workflow graphs. Key functions:

- `render_lane()` — renders a single lane's workflow graph
- `render_workflow()` — writes the graph.fabro file

### fabro-cli/src/main.rs

CLI entry point. Key invariants:

- `Synth` command with subcommands must be registered
- Commands must be accessible in both debug and release builds

## Failure Handling

### Missing Command Surface

If `fabro` binary does not expose a required command:
- Detect at autodev startup by probing `fabro <command> --help`
- Fail fast with actionable error message
- Do not proceed to dispatch

### Runtime Path Resolution Failures

If a dispatched lane fails due to missing prompt or asset:
- Classify as `FailureKind::RuntimePathError`
- Mark lane as `failed` with `recovery_action = FailureRecoveryAction::SurfaceBlocked`
- Emit `runtime_path_errors` telemetry increment
- Do NOT auto-replay; require manual intervention or regeneration

### Stale Running Detection

If a lane is marked `running` but has no live worker:
- Detect via `worker_process_alive()` returning `Some(false)`
- Reclassify as `failed` with `FailureKind::TransientLaunchFailure`
- Set `recovery_action = FailureRecoveryAction::BackoffRetry`
- Increment `stale_running_reclaimed` telemetry

## Adoption Path

1. **Phase 1**: Verify command surface in both debug and release builds
2. **Phase 2**: Fix prompt resolution in generated workflow graphs
3. **Phase 3**: Ensure stale running detection fires before dispatch
4. **Phase 4**: Add dispatch telemetry fields
5. **Phase 5**: Decouple evolve from dispatch (async evolve)
6. **Phase 6**: Live validation on proving-ground repo

## Acceptance Criteria

1. `fabro synth --help` works in both debug and release builds
2. `fabro run --detach <run-config>` succeeds for a lane whose workflow uses `@prompts/...` references
3. A lane marked `running` with a dead worker process is reclassified as `failed` within 30 seconds
4. `raspberry autodev` report includes `dispatch_rate`, `idle_cycles`, `ready_but_undispatched`, `failed_bootstrap_count`, `runtime_path_errors`, and `stale_running_reclaimed` fields
5. A 10-lane autodev run sustains 10 `running` lanes for 20 consecutive cycles without manual intervention
6. Zero bootstrap validation failures in the first 10 dispatch cycles

## Non-Goals

- Improving dispatch rate beyond consuming the full `max_parallel` budget (handled in future optimization pass)
- Review quality scoring or promotion contracts (Plan 006)
- Provider policy stabilization (Plan 008)
