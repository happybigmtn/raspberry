# Autodev Execution Path and Dispatch Truth

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, `raspberry autodev` should run generated packages without local-only rescue steps. Generated workflows should resolve prompt and artifact references correctly, the binaries operators actually run should expose the commands autodev depends on, stale state should not consume dispatch slots, and ready lanes should be dispatched immediately within the configured budget.

The proof is a bounded autodev run against rXMRbro that sustains 10 active lanes, shows `failed: 0` for newly dispatched lanes during the first live cycles, and no longer needs ad hoc runtime shims to resolve generated prompt assets or command entrypoints.

Provenance: This plan consolidates findings from `plans/032326-autodev-efficiency-and-harness-engineering.md`, `plans/032026-keep-fabro-and-raspberry-continuously-generating-work.md`, `plans/032526-e2e-autodev-review-and-remediation.md` (Phase 1), and `plans/031926-build-raspberry-autodev-orchestrator.md`.

## Progress

- [x] Reproduce the current execution-path failures on a proving-ground repo
- [x] Verify that the generated autodev workflow families exist in a synthesized package
- [ ] Eliminate local-only command and prompt-resolution shims from the autodev runtime path
- [ ] Fix stale `running` and `failed` lane truth before dispatch
- [ ] Add dispatch-state telemetry that explains why ready work did or did not run
- [ ] Live validation: sustain 10 active lanes on rXMRbro without bootstrap validation failures
- [ ] Live validation: at least 3 lanes land to trunk after the runtime path is boring

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: Fix execution-path consistency before micro-optimizing dispatch rate.
  Rationale: The freshest live failures were not abstract scheduler inefficiency. They were concrete runtime mismatches: generated lanes depending on `fabro synth` while the shipped CLI did not expose it, and copied workflow graphs resolving `@../../prompts/...` under `~/.fabro` instead of the target repo. No dispatch metric matters until that path is consistent.
  Date/Author: 2026-03-26 / Genesis

- Decision: Separate dispatch optimization from review quality improvements.
  Rationale: Dispatch rate and review quality are independent variables. Improving one should not block the other. Plan 006 handles review quality.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: Improving dispatch rate without fixing runtime-path consistency or stale `running` detection could produce faster failure loops or duplicate lane execution. The fix must include command-path validation and running-state reconciliation before dispatch.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

The autodev loop lives in `lib/crates/raspberry-supervisor/src/autodev.rs`. Each cycle:

1. Refreshes program state from `.raspberry/*-state.json`
2. Evaluates lane readiness via `evaluate.rs`
3. Optionally runs `synth evolve` to update the package
4. Dispatches ready lanes via `dispatch.rs`
5. Watches for completion or timeout
6. Updates program state

The historical 83% idle rate is still useful context, but current live evidence shows the execution path itself is the first priority. During live restart work on 2026-03-26:

- `rXMRbro` initially failed because the clean `fabro` binary did not expose `synth`, even though `lib/crates/fabro-cli/src/commands/synth.rs` still existed.
- Newly dispatched lanes failed validation because copied `graph.fabro` files referenced prompts as `@../../prompts/...`, which resolved under `~/.fabro/` at runtime instead of the target repo.
- After adding a temporary prompt symlink and using a synth-enabled binary, `rXMRbro` returned to `running: 10`, `ready: 23`, `failed: 0`.

Those observations redefine this plan. The job is not only "go faster." It is "make the generated package and the runtime agree about where commands and assets live."

The remaining dispatch-specific root causes from `plans/032326-autodev-efficiency-and-harness-engineering.md` are:

- `synth evolve` blocks the dispatch cycle (it runs synchronously before dispatch)
- Stale `running` lanes that are actually dead consume worker slots
- The frontier budget accounting doesn't aggressively reclaim failed slots
- Evolve cadence timer delays dispatch even when ready work exists

```
Current cycle:
  [refresh] → [evaluate] → [evolve (blocking)] → [dispatch] → [watch]
                                 ↑ THIS BLOCKS

Target cycle:
  [refresh] → [evaluate] → [dispatch] → [watch]
                    ↓ (background, decoupled)
              [evolve (async, cadence-gated)]
```

## Milestones

### Milestone 1: Freeze the current failure modes into reproducible tests

Turn the observed failures into deterministic tests:
- autodev invoking a `fabro` binary without `synth`
- generated prompt refs resolving outside the target repo
- stale `running` / `failed` state preventing redispatch after recovery

Proof command:

    cargo nextest run -p raspberry-supervisor -- autodev
    cargo nextest run -p fabro-cli -- synth
    cargo nextest run -p fabro-synthesis -- render

### Milestone 2: Make autodev runtime paths self-consistent

Ensure the binaries and generated workflows agree about command and asset resolution:
- `fabro` release/debug binaries both expose the synthesis commands autodev calls
- generated prompt references resolve from the target repo or run dir without relying on `~/.fabro/prompts`
- copied workflow graphs carry enough context to validate in the detached run environment

Key files:
- `lib/crates/fabro-cli/src/main.rs`
- `lib/crates/fabro-cli/src/commands/synth.rs`
- `lib/crates/fabro-synthesis/src/render.rs`

Proof command:

    /home/r/.cache/cargo-target/debug/fabro synth --help
    /home/r/.cache/cargo-target/debug/fabro validate /home/r/coding/rXMRbro/malinka/run-configs/investigate/baccarat-investigate.toml

### Milestone 3: Reconcile stale running and failed lane truth

Ensure `program_state.rs` and the autodev cycle distinguish:
- genuinely active runs
- dead/stale runs
- bootstrap validation failures that should be fixable after regeneration
- terminal failures that should block the lane

Key files:
- `lib/crates/raspberry-supervisor/src/program_state.rs`
- `lib/crates/raspberry-supervisor/src/evaluate.rs`
- `lib/crates/raspberry-supervisor/src/autodev.rs`

Proof command:

    cargo nextest run -p raspberry-supervisor -- program_state running

### Milestone 4: Decouple evolve from dispatch and consume the budget greedily

Move `run_synth_evolve()` off the hot dispatch path and ensure `dispatch.rs` consumes the full `max_parallel` budget in one cycle when ready lanes exist.

Key files:
- `lib/crates/raspberry-supervisor/src/autodev.rs`
- `lib/crates/raspberry-supervisor/src/dispatch.rs`

Proof command:

    cargo nextest run -p raspberry-supervisor -- dispatch parallel autodev

### Milestone 5: Add dispatch-state telemetry

Add telemetry that explains the frontier in operator language:
- `dispatch_rate`
- `idle_cycles`
- `ready_but_undispatched`
- `failed_bootstrap_count`
- `runtime_path_errors`
- `stale_running_reclaimed`

Proof command:

    cargo nextest run -p raspberry-supervisor -- autodev report
    # Also: raspberry status should show dispatch_rate field

### Milestone 6: Live validation

Build the binaries operators will actually use and run autodev on rXMRbro with `--max-parallel 10`. No local prompt symlink, no debug-only command surface, no manual state scrubbing should be required.

Proof command:

    cargo build --release -p fabro-cli -p raspberry-cli --target-dir target-local && \
    target-local/release/raspberry autodev \
      --manifest /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml \
      --max-parallel 10 --max-cycles 20

Expected: the controller remains `InProgress`, holds 10 running lanes for sustained cycles, and produces zero prompt-resolution or missing-command bootstrap failures.

## Validation and Acceptance

The plan is done when:
- the generated package and the autodev runtime agree about command and asset resolution
- stale `running` / `failed` lane truth is reconciled before dispatch
- autodev sustains 10 active lanes on a proving-ground repo without local-only shims
- dispatch telemetry explains why work did or did not run
- at least 3 lanes land to trunk once the execution path is boring
