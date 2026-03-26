# Audit Autodev Critical Path Lane — Review

Review only the current slice for `error-handling-hardening-audit-autodev-critical-path`.

Current Slice Contract:
Plan file:
- `genesis/plans/002-error-handling-hardening.md`

Child work item: `error-handling-hardening-audit-autodev-critical-path`

Full plan context (read this for domain knowledge, design decisions, and specifications):

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


Workflow archetype: implement

Review profile: standard

Active plan:
- `genesis/plans/001-master-plan.md`

Active spec:
- `genesis/SPEC.md`

Artifacts to write:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Nemesis-style security review
- Pass 1 — first-principles challenge: question trust boundaries, authority assumptions, and who can trigger the slice's dangerous actions
- Pass 2 — coupled-state review: identify paired state or protocol surfaces and check that every mutation path keeps them consistent or explains the asymmetry
- check secret handling, capability scoping, pairing/idempotence behavior, and privilege escalation paths
- check external-process control, operator safety, idempotent retries, and failure modes around service lifecycle

Focus on:
- slice scope discipline
- proof-gate coverage for the active slice
- touched-surface containment
- implementation and verification artifact quality
- remaining blockers before the next slice


Structural discipline
- if a new source file would exceed roughly 400 lines, split it before landing
- do not mix state transitions, input handling, rendering, and animation in one new file unless the prompt explicitly justifies that coupling
- if the slice cannot stay small, stop and update the artifacts to explain the next decomposition boundary instead of silently landing a monolith
Deterministic evidence:
- treat `.fabro-work/quality.md` as machine-generated truth about placeholder debt, warning debt, manual follow-up, and artifact mismatch risk
- if `.fabro-work/quality.md` says `quality_ready: no`, do not bless the slice as merge-ready


Score each dimension 0-10 and write `.fabro-work/promotion.md` in this exact form:

merge_ready: yes|no
manual_proof_pending: yes|no
completeness: <0-10>
correctness: <0-10>
convention: <0-10>
test_quality: <0-10>
reason: <one sentence>
next_action: <one sentence>

Scoring guide:
- completeness: 10=all deliverables present + all acceptance criteria met, 7=core present + 1-2 gaps, 4=missing deliverables, 0=skeleton
- correctness: 10=compiles + tests pass + edges handled, 7=tests pass + minor gaps, 4=some failures, 0=broken
- convention: 10=matches all project patterns, 7=minor deviations, 4=multiple violations, 0=ignores conventions
- test_quality: 10=tests import subject + verify all criteria, 7=most criteria tested, 4=structural only, 0=no tests

If `.fabro-work/contract.md` exists, verify EVERY acceptance criterion from it.
Any dimension below 6 = merge_ready: no.
If `.fabro-work/quality.md` says quality_ready: no = merge_ready: no.

For security-sensitive slices, append these mandatory fields exactly:
- overflow_safe: yes|no
- seed_binding_complete: yes|no
- house_authority_preserved: yes|no
- proof_covers_edge_cases: yes|no
- layout_invariants_complete: yes|no
- slice_decomposition_respected: yes|no
If any mandatory security field is `no`, set `merge_ready: no`.

Review stage ownership:
- you may write or replace `.fabro-work/promotion.md` in this stage
- read `.fabro-work/quality.md` before deciding `merge_ready`
- when the slice is security-sensitive, perform a Nemesis-style pass: first-principles assumption challenge plus coupled-state consistency review
- include security findings in the review verdict when the slice touches trust boundaries, keys, funds, auth, control-plane behavior, or external process control
- prefer not to modify source code here unless a tiny correction is required to make the review judgment truthful
