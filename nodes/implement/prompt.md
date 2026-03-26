Goal: Use Codex to unblock implementation lane `test-coverage-critical-paths-ci-preservation-and-hardening:test-coverage-critical-paths-ci-preservation-and-hardening`.

Inspect the source lane's most recent failure/remediation context and apply the minimal code or harness changes needed so the source lane can pass its next replay.

Proof commands:
- `cargo check --workspace`


## Completed stages
- **preflight**: success
  - Script: `set +e
cargo check --workspace
true`
  - Stdout: (empty)
  - Stderr:
    ```
    (5 lines omitted)
        Checking fabro-llm v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-llm)
        Checking fabro-mcp v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-mcp)
        Checking fabro-sandbox v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-sandbox)
        Checking fabro-validate v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-validate)
        Checking fabro-devcontainer v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-devcontainer)
        Checking fabro-interview v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-interview)
        Checking fabro-git-storage v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-git-storage)
       Compiling fabro-types v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-types)
        Checking fabro-agent v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-agent)
       Compiling fabro-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-cli)
       Compiling raspberry-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/raspberry-cli)
        Checking fabro-telemetry v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-telemetry)
        Checking fabro-hooks v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-hooks)
        Checking fabro-retro v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-retro)
        Checking fabro-workflows v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-workflows)
        Checking fabro-db v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-db)
        Checking fabro-openai-oauth v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-openai-oauth)
        Checking fabro-tracker v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-tracker)
        Checking fabro-beastie v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-beastie)
        Checking raspberry-supervisor v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/raspberry-supervisor)
        Checking fabro-api v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-api)
        Checking fabro-slack v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-slack)
        Checking raspberry-tui v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/raspberry-tui)
        Checking fabro-synthesis v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-synthesis)
        Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.25s
    ```
- **contract**: success
  - Model: gpt-5.4, 897.7k tokens in / 9.4k out


# Ci Preservation And Hardening Lane Codex Unblock — Plan

Lane: `test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock`

Goal:
- Use Codex to unblock implementation lane `test-coverage-critical-paths-ci-preservation-and-hardening:test-coverage-critical-paths-ci-preservation-and-hardening`.

Inspect the source lane's most recent failure/remediation context and apply the minimal code or harness changes needed so the source lane can pass its next replay.

Proof commands:
- `cargo check --workspace`


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

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
Target blocked lane: `test-coverage-critical-paths-ci-preservation-and-hardening:test-coverage-critical-paths-ci-preservation-and-hardening`.
Recovery objective: unblock the source lane so it can be replayed successfully.
This lane is dispatched only after the source lane is marked `surface_blocked`.
Focus on minimal, high-confidence changes that remove the blocker.
Read the target lane's latest artifacts and remediation notes before editing.
If the owned proof gate is already green and the only remaining blocker is outside the owned surface, do not invent more code changes. Write the unblock artifacts truthfully, explain the external blocker, and stop.
Keep the scope narrow: fix the blocker, verify, integrate, and stop.
This lane is distinct from the parent holistic deep/adjudication review path.
