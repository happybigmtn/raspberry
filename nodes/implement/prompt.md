Goal: Raspberry Supervisor Edge Case Tests

Child work item of plan: Test Coverage for Critical Paths

Proof commands:
- `cargo nextest run -p raspberry-supervisor -- integration autodev_cycle`
- `cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed`

Required durable artifacts:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`


## Completed stages
- **preflight**: success
  - Script: `set +e
if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p raspberry-supervisor -- integration autodev_cycle && cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
else
  cargo test -p raspberry-supervisor -- integration autodev_cycle && cargo test -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
fi
true`
  - Stdout: (empty)
  - Stderr:
    ```
    (25 lines omitted)
            PASS [   0.006s] (4/5) raspberry-supervisor autodev::tests::replayable_failed_lanes_replay_source_lane_for_failed_integration_program
            FAIL [   0.118s] (5/5) raspberry-supervisor integration::tests::integrate_lane_squash_merges_run_branch_into_trunk
      stdout ───
    
        running 1 test
        test integration::tests::integrate_lane_squash_merges_run_branch_into_trunk ... FAILED
    
        failures:
    
        failures:
            integration::tests::integrate_lane_squash_merges_run_branch_into_trunk
    
        test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 116 filtered out; finished in 0.11s
    
      stderr ───
    
        thread 'integration::tests::integrate_lane_squash_merges_run_branch_into_trunk' (2120169) panicked at lib/crates/raspberry-supervisor/src/integration.rs:268:10:
        integration succeeds: Direct(Git { step: "resolve ssh push url", repo: "/home/r/.cache/rust-tmp/fabro-direct-integration-8621fa6b-3e6e-4302-9111-4079fc762a81", message: "Engine error: remote `origin` must use SSH or be convertible from GitHub HTTPS, got `/home/r/.cache/rust-tmp/.tmpohclm1/remote.git`" })
        note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    
      Cancelling due to test failure: 
    ────────────
         Summary [   0.119s] 5 tests run: 4 passed, 1 failed, 112 skipped
            FAIL [   0.118s] (5/5) raspberry-supervisor integration::tests::integrate_lane_squash_merges_run_branch_into_trunk
    error: test run failed
    ```
- **contract**: success
  - Model: MiniMax-M2.7-highspeed, 33.4k tokens in / 333 out


# Raspberry Supervisor Edge Case Tests Lane — Plan

Lane: `test-coverage-critical-paths-raspberry-supervisor-edge-case-tests`

Goal:
- Raspberry Supervisor Edge Case Tests

Child work item of plan: Test Coverage for Critical Paths

Proof commands:
- `cargo nextest run -p raspberry-supervisor -- integration autodev_cycle`
- `cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed`

Required durable artifacts:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Layout/domain invariant tests (required for this slice even if not called out above):
- layout invariant test proving the rendered board/grid contains no duplicate domain values

Decomposition pressure
- if a new source file would exceed roughly 400 lines, split it before landing
- do not mix state transitions, input handling, rendering, and animation in one new file unless the prompt explicitly justifies that coupling
- if the slice cannot stay small, stop and update the artifacts to explain the next decomposition boundary instead of silently landing a monolith

Sprint contract:
- Read `.fabro-work/contract.md` — the contract stage wrote it before you. It lists the exact deliverables and acceptance criteria.
- You MUST satisfy ALL acceptance criteria from the contract.
- You MUST create ALL files listed in the contract's Deliverables section.
- If the contract is missing or empty, write your own `.fabro-work/contract.md` before coding.


Implementation quality:
- implement functionality completely — every function must do real work, not return defaults or skip the action
- BEHAVIORAL STUBS ARE WORSE THAN COMPILATION FAILURES: a function that compiles but does not perform its stated purpose will be caught by the adversarial challenge stage and rejected
- tests must verify behavioral outcomes (given X input, assert Y output), not just compilation or derive macros (Display, Clone, PartialEq)
- include at least one FULL LIFECYCLE test that drives from initial state through multiple actions to terminal state
- do not duplicate tests — one test per behavior, not five tests for the same Display output

Design conventions (the challenge stage WILL reject violations):
- Settlement arithmetic: Chips is i16 (max 32767). ALL payout math MUST widen to i32 or i64 FIRST to prevent overflow. CORRECT: `let payout = (i32::from(bet) * 3 / 2) as Chips;` WRONG: `(bet as f64 * 1.5) as Chips` (float truncation). WRONG: `bet * 95 / 100` (i16 overflow for bet > 345)
- No `unwrap()` in production code — use `?`, `unwrap_or`, `if let`, or return an error
- Use shared error types from `error.rs`: `GameError::IllegalAction`, `GameError::InvalidState`, `VerifyError::InvalidState`
- Use `Settlement::new(delta)` for wins/losses and `Settlement::push()` for ties

Stage ownership:
- do not write `.fabro-work/promotion.md` during Plan/Implement
- do not hand-author `.fabro-work/quality.md`; it is regenerated by the Quality Gate
- `.fabro-work/promotion.md` is owned by the Review stage only
- keep source edits inside the named slice and touched surfaces
- ALL ephemeral workflow files (quality.md, promotion.md, verification.md, deep-review-findings.md) MUST be written to the `.fabro-work/` directory, never the repo root


Full Slice Contract:
Plan file:
- `genesis/plans/005-test-coverage-critical-paths.md`

Child work item: `test-coverage-critical-paths-raspberry-supervisor-edge-case-tests`

Full plan context (read this for domain knowledge, design decisions, and specifications):

# Test Coverage for Critical Paths

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, every crate in the autodev critical path has meaningful test coverage for the failure modes we now know are real: generated package/runtime mismatches, detached-run validation failures, stale frontier truth, and workspace-level regressions. The two crates with zero tests (`fabro-db`, `fabro-types`) get baseline coverage, and synthesis/autodev regressions are pinned down with targeted tests before they reappear overnight.

The proof is: introduce a deliberate regression (e.g., break a SQL migration in fabro-db), push to a branch, and watch CI fail with a specific test name.

## Progress

- [ ] Add tests to fabro-db (schema migration, WAL mode, basic CRUD)
- [ ] Add edge case tests to raspberry-supervisor (stale state, race conditions)
- [ ] Add integration tests for autodev dispatch cycle and detached-run validation
- [ ] Preserve and extend CI coverage for synthesis/autodev regressions
- [ ] Add fabro-mcp and fabro-github minimal test coverage

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: Focus test additions on failure modes, not happy paths.
  Rationale: The highest-value new tests are for the failures that actually stopped proving-ground runs: stale state, malformed runtime paths, detached-run validation failures, and command-surface mismatches. Those are more urgent than broad happy-path expansion.
  Date/Author: 2026-03-26 / Genesis

- Decision: Do not pursue code coverage percentages as a target.
  Rationale: 80% coverage on fabro-workflows (52K LOC) would require thousands of tests for marginal benefit. Instead, target specific failure modes identified in the assessment.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: New tests may be flaky if they depend on file I/O timing or network state. All new tests must be deterministic — use in-memory SQLite for db tests, fixture files for state tests, no network calls.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

Current test landscape:

| Crate | Tests | LOC | Gap |
|-------|-------|-----|-----|
| `fabro-db` | 0 | ~1,500 | **Zero coverage** — SQLite, WAL, migrations |
| `fabro-types` | 0 | ~5,000 | Auto-generated from OpenAPI, low risk |
| `fabro-mcp` | 7 | ~1,800 | MCP protocol client barely tested |
| `fabro-github` | 4 | ~1,200 | JWT signing, installation tokens barely tested |
| `raspberry-supervisor` | 114 | 15,049 | Missing: stale running state, dispatch races |
| `fabro-synthesis` | 88 | 15,472 | Missing: edge cases in render.rs |

CI config lives in `.github/workflows/rust.yml`. The repo already has fmt, clippy, and nextest checks. The real gap is that critical synthesis/autodev regressions are not yet captured by focused tests, so CI can stay green while proving-ground runs still fail.

## Milestones

### Milestone 1: fabro-db baseline tests

Add tests for:
- Database creation with WAL mode
- Schema migration (apply all migrations, verify tables exist)
- Basic CRUD operations (insert, query, update, delete)
- Concurrent read during write (WAL mode correctness)
- Corrupt/missing database file handling

All tests must use in-memory SQLite (`:memory:`) or temp files.

Key file: `lib/crates/fabro-db/src/lib.rs` (or `lib/crates/fabro-db/src/`)

Proof command:

    cargo nextest run -p fabro-db

Expected: 5+ new tests, all passing.

### Milestone 2: raspberry-supervisor edge case tests

Add tests for:
- Stale `running` lane detection and reconciliation
- Dispatch with max_parallel budget exhaustion
- Recovery action authority (persisted vs recomputed)
- Cycle limit termination behavior
- Frontier budget accounting after failures
- Program state with malformed JSON files

Key files:
- `lib/crates/raspberry-supervisor/src/program_state.rs`
- `lib/crates/raspberry-supervisor/src/dispatch.rs`
- `lib/crates/raspberry-supervisor/src/autodev.rs`

Proof command:

    cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed

### Milestone 3: Autodev integration test

Add a fixture-based integration test that simulates a complete autodev cycle: load a fixture manifest, evaluate, dispatch (mocked), observe state change, and verify detached-run bootstrap diagnostics surface the real cause when validation fails.

Key file: `lib/crates/raspberry-supervisor/tests/` (new integration test file)

Proof command:

    cargo nextest run -p raspberry-supervisor -- integration autodev_cycle

### Milestone 4: Synthesis/runtime regression tests

Add targeted regression tests for the failures observed during live restart work:
- generated workflows depending on a `fabro` binary that does not expose required subcommands
- copied run graphs failing validation because prompt refs resolve under the wrong root
- detached runs collapsing to generic `Validation failed` without actionable diagnostics

Key files:
- `lib/crates/fabro-cli/src/main.rs`
- `lib/crates/fabro-cli/src/commands/synth.rs`
- `lib/crates/fabro-synthesis/src/render.rs`
- `lib/crates/fabro-workflows/src/`

Proof command:

    cargo nextest run -p fabro-cli -- synth
    cargo nextest run -p fabro-synthesis -- render

### Milestone 5: CI preservation and hardening

Update `.github/workflows/rust.yml` only where needed to make sure the new synthesis/autodev regression tests run in CI and fail loudly. Preserve the existing fmt/clippy/nextest checks rather than "adding clippy" from scratch.

Proof command:

    cargo clippy --workspace -- -D warnings && \
    cargo fmt --check --all && \
    cargo nextest run --workspace

### Milestone 6: Minimal coverage for fabro-mcp and fabro-github

Add 3-5 tests each for:
- `fabro-mcp`: message serialization, tool call parsing, protocol handshake
- `fabro-github`: JWT generation, installation token request structure, PR creation payload

Proof command:

    cargo nextest run -p fabro-mcp && cargo nextest run -p fabro-github

## Validation and Acceptance

The plan is done when:
- `fabro-db` has >5 tests covering schema and CRUD
- `raspberry-supervisor` has edge case tests for stale state and dispatch races
- An autodev integration test exists and passes
- synthesis/runtime regressions are covered by tests that fail before proving-ground autodev does
- A deliberate regression in fabro-db is caught by CI


Workflow archetype: implement

Review profile: standard

Active plan:
- `genesis/plans/001-master-plan.md`

Active spec:
- `genesis/SPEC.md`

Proof commands:
- `cargo nextest run -p raspberry-supervisor -- integration autodev_cycle`
- `cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed`

Artifacts to write:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`
