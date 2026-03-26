# Error Handling Hardening

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, the Fabro/Raspberry engine will not panic during normal autodev operation. The system that supervises code quality will itself meet the error handling standards it enforces on generated code. This plan starts after execution-path consistency and critical-path tests are in place, so its work hardens a stable runtime rather than obscuring more basic bootstrap failures.

The proof is a 100-cycle autodev run against rXMRbro with zero `unwrap()`-induced panics in the engine. Failures should produce structured error messages with recovery hints, not thread panics.

## Progress

- [ ] Audit the autodev critical path for unwrap() calls
- [ ] Replace unwrap() calls in raspberry-supervisor
- [ ] Replace unwrap() calls in fabro-workflows execution path
- [ ] Replace unwrap() calls in fabro-synthesis render path
- [ ] Add error context to the 50 most dangerous unwrap() sites
- [ ] Run autodev for 100 cycles and confirm zero engine panics

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: Scope this plan to the autodev critical path, not all 2,952 unwrap() calls.
  Rationale: Replacing every unwrap() would touch every crate and take weeks. The autodev path (raspberry-supervisor → fabro-workflows → fabro-agent → fabro-llm) is where panics cause real operator pain. Peripheral crates like fabro-telemetry or fabro-devcontainer can be addressed later.
  Date/Author: 2026-03-26 / Genesis

- Decision: Run this plan after execution-path consistency and critical-path test work.
  Rationale: Recent live failures were dominated by command-surface and prompt-resolution mismatches, not engine panics. Hardening error handling is still important, but it should follow the work that makes the runtime path reproducible and testable.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: Replacing unwrap() with `?` propagation may change error behavior in callers that previously relied on panic-as-abort. Test each replacement site to ensure the error propagates to a handler that logs and recovers.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

The codebase has 2,952 `unwrap()` calls across production Rust code (excluding tests). The densest crates are:

- `fabro-workflows` (52,263 LOC) — the execution engine
- `fabro-cli` (32,311 LOC) — CLI entry points
- `fabro-llm` (16,377 LOC) — LLM provider clients
- `raspberry-supervisor` (15,049 LOC) — the autodev control plane
- `fabro-synthesis` (15,472 LOC) — plan-to-package compiler

The autodev critical path is:

```
raspberry-supervisor/src/autodev.rs
  → raspberry-supervisor/src/dispatch.rs
  → raspberry-supervisor/src/evaluate.rs
  → raspberry-supervisor/src/program_state.rs
  → fabro-workflows/src/backend/cli.rs
  → fabro-workflows/src/handler/agent.rs
  → fabro-agent/src/cli.rs
  → fabro-llm/src/provider/*.rs
```

The term "autodev critical path" means the sequence of function calls that execute during a single `raspberry autodev` cycle: evaluate program state, select ready lanes, dispatch them via Fabro, and observe results. A panic anywhere in this path kills the entire controller.

## Milestones

### Milestone 1: Audit autodev critical path

Read every `.rs` file in the autodev critical path listed above. For each `unwrap()`, classify it:

- **Safe:** the value is guaranteed non-None/non-Err by construction (e.g., `"literal".parse::<usize>().unwrap()`)
- **Dangerous:** the value depends on external input (file I/O, network, user config, deserialization)
- **Critical:** the value depends on external input AND is in the autodev hot loop

Write the audit to `genesis/artifacts/unwrap-audit.md` with file, line, classification, and suggested replacement.

Proof command:

    grep -rn "unwrap()" lib/crates/raspberry-supervisor/src/autodev.rs \
      lib/crates/raspberry-supervisor/src/dispatch.rs \
      lib/crates/raspberry-supervisor/src/evaluate.rs \
      lib/crates/raspberry-supervisor/src/program_state.rs \
      lib/crates/fabro-workflows/src/backend/cli.rs \
      lib/crates/fabro-workflows/src/handler/agent.rs | wc -l

The count should be documented as the baseline.

### Milestone 2: Harden raspberry-supervisor

Replace all "Dangerous" and "Critical" `unwrap()` calls in `raspberry-supervisor/src/` with proper error handling. Use `anyhow::Context` for adding context to errors. Each replacement must:

1. Propagate the error with `?` or handle it with a match
2. Include a context message naming the operation that failed
3. Not change the observable behavior for the success path

Proof command:

    cargo nextest run -p raspberry-supervisor

### Milestone 3: Harden fabro-workflows execution path

Replace dangerous `unwrap()` calls in `fabro-workflows/src/backend/cli.rs` and `fabro-workflows/src/handler/agent.rs`. These are the hottest paths during lane execution.

Proof command:

    cargo nextest run -p fabro-workflows -- backend cli agent

### Milestone 4: Harden fabro-synthesis render path

Replace dangerous `unwrap()` calls in `fabro-synthesis/src/render.rs` and `fabro-synthesis/src/planning.rs`. These run during `synth create/evolve`.

Proof command:

    cargo nextest run -p fabro-synthesis

### Milestone 5: Integration validation

Run `cargo clippy --workspace -- -D warnings` and confirm zero new warnings. Run `cargo nextest run --workspace` and confirm all tests pass.

Proof command:

    cargo clippy --workspace -- -D warnings && cargo nextest run --workspace

### Milestone 6: Live autodev validation

Build release binaries and run a 100-cycle autodev on rXMRbro. Monitor for panics.

Proof command:

    cargo build --release -p fabro-cli -p raspberry-cli --target-dir target-local && \
    target-local/release/raspberry autodev \
      --manifest /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml \
      --max-cycles 100 2>&1 | grep -c "panicked"

Expected: 0 panics.

## Validation and Acceptance

The plan is done when:
- All "Critical" unwrap() sites in the autodev path are replaced
- `cargo nextest run --workspace` passes
- `cargo clippy --workspace -- -D warnings` passes
- A 100-cycle autodev run produces zero engine panics
