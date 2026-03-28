# Comprehensive Root-Cause Remediation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `investigate` before changing any phase-owned code, and execute this plan as a bounded program of work instead of one-off patches.

**Goal:** Eliminate the systemic Fabro/Raspberry failures that keep live autodev busy without producing trustworthy landings. The objective is not to improve one lane at a time. The objective is to make the framework tell the truth about lane-owned work, prove only the owned surface, and stop spending tokens or fixup cycles on self-inflicted workflow errors.

**Architecture:** The fix spans four layers that currently interact badly:
- `fabro-synthesis` generated workflow structure and shell gates
- `fabro-workflows` stage prompt policy and outcome accounting
- controller discipline for local rerender commits versus audit baselines
- `rXMRbro` lane evidence and health commands that are currently wider than the owned slice

**Tech Stack:** Rust, generated `.fabro` workflows, shell gates, `raspberry` autodev controller state, `rXMRbro` blueprint/program manifests.

---

## Root Cause Hypothesis

The current failure cluster is not one bug. It is one bad control loop made of four coupled bugs:

1. Audit and no-op guards diff every run branch against `origin/main`, but live controllers run from a local rerender commit ahead of `origin/main`. That makes inherited controller-local changes look like lane-owned changes.
2. Workflow-owned artifacts are inconsistent. UX lanes can require `acceptance-evidence.md`, then later reject it as a surface violation. Review artifacts can also contradict verification truth and still bless a slice as merge-ready.
3. Prompt policy conflicts with gate policy. Some prompts tell fixup to repair external blockers outside the slice; other prompts tell fixup to ignore anything outside the slice. The result is non-deterministic cross-surface edits followed by deterministic audit failures.
4. Proof and codex-unblock flows are too expensive for the work they govern. Some health/proof commands are broader than the lane surface, and some codex-unblock lanes spend large token budgets producing no code delta before reaching a proof gate that was already green.

The framework therefore overstates failure, understates no-op waste, and cannot cleanly distinguish lane-local defects from controller-local inherited drift.

## Canary-Confirmed Addendum

The first 3-lane canary on the post-phase baseline exposed two remaining framework defects that were not fully eliminated by the initial six phases:

1. Worktree bootstrap is still brittle. Fresh runs can abort before `StageStarted` when a stale run worktree path or registration survives long enough for `git worktree add` to fail with `already exists` or `Could not write new index file`.
2. Exact-file ownership is still too loose. The audit surface guard currently widens an owned file such as `crates/tui/src/screens/three_card.rs` to its parent directory, which allows sibling replacements and unrelated screen churn to slip through until late stages.

These are not lane-specific bugs. They are framework truthfulness bugs and belong in this remediation program.

---

## Phase 1: Fix Baseline Truth

**Goal:** Make every audit, noop, and surface-ownership check diff against the run's actual starting commit, not `origin/main`.

**Files:**
- Modify: `lib/crates/fabro-synthesis/src/render.rs`
- Modify: `lib/crates/fabro-workflows/src/engine.rs`
- Modify: `lib/crates/fabro-workflows/src/live_state.rs`
- Test: `lib/crates/fabro-synthesis/src/render.rs`
- Test: `lib/crates/fabro-workflows/src/*`

**Steps:**
1. Introduce an explicit run baseline reference derived from the run's `base_sha`.
2. Replace all generated `git merge-base HEAD origin/main` audit/noop/surface checks with diffs against the recorded run baseline.
3. Make the baseline available to generated shell gates through environment or rendered literal.
4. Add regression tests proving inherited controller rerender commits do not count as lane-owned changes.

**Exit Criteria:**
- A lane run from a controller that is ahead of `origin/main` but clean at dispatch does not fail audit on inherited controller diffs.
- No-op guards still reject true empty implementation lanes.

---

## Phase 2: Unify Artifact Ownership

**Goal:** Stop the workflow from generating or requiring artifacts that later gates reject.

**Files:**
- Modify: `lib/crates/fabro-synthesis/src/render.rs`
- Modify: `lib/crates/fabro-workflows/src/backend/cli.rs`
- Test: `lib/crates/fabro-synthesis/src/render.rs`

**Steps:**
1. Move `acceptance-evidence.md` under a lane-scoped path or `.fabro-work/`, not repo root.
2. Ensure acceptance, review, promotion, and audit all read the same canonical artifact location.
3. Make promotion contract generation consume machine verification truth, so `merge_ready: yes` cannot coexist with critical missing fixes unless the lane is explicitly evidence-only.
4. Add regression tests for UX lanes proving acceptance evidence is both required and audit-safe.

**Exit Criteria:**
- A failed acceptance gate can be fixed without creating a later audit surface violation.
- Promotion artifacts cannot contradict verification artifacts on mandatory correctness fields.

---

## Phase 3: Resolve Prompt Policy Conflicts

**Goal:** Make the agent instructions and audit rules agree on whether out-of-surface fixes are ever allowed.

**Files:**
- Modify: `lib/crates/fabro-synthesis/src/render.rs`
- Modify: `lib/crates/fabro-workflows/src/backend/cli.rs`
- Test: `lib/crates/fabro-synthesis/src/render.rs`

**Steps:**
1. Define two explicit policies:
   - strict lane-local implementation
   - external-unblock/evidence-only remediation
2. Remove the current contradiction where one prompt says "ignore outside-surface failures" and another says "you MUST fix those issues."
3. Tighten exact-file ownership so a lane that owns `foo.rs` does not silently gain permission to write sibling files under the same directory unless the plan explicitly names them.
4. Make the contract gate reject source deliverables outside the lane's owned surfaces so off-scope file plans fail before implementation token burn.
5. Tie audit behavior to the chosen policy so allowed cross-surface work is deliberate, named, and machine-checkable.
6. Add tests proving standard implementation lanes reject cross-surface edits while unblock lanes can declare evidence-only completion without touching foreign code.

**Exit Criteria:**
- An implementation lane cannot both be told to stay in-slice and to fix foreign code.
- Cross-surface remediation becomes explicit policy, not accidental behavior.
- Exact-file lanes cannot create sibling replacement files without an explicit surface declaration.

---

## Phase 4: Narrow Proof and Health Commands

**Goal:** Ensure live proof commands validate the owned slice instead of pulling in unrelated workspace debt.

**Files:**
- Modify: `lib/crates/fabro-synthesis/src/render.rs`
- Modify: `malinka/programs/rxmragent.yaml`
- Modify: `malinka/blueprints/rxmragent.yaml`
- Regenerate: controller workflows for `rXMRbro`

**Steps:**
1. Add synthesis-side validation for health commands that are broader than the lane's proof/owned surface.
2. Normalize obvious wide service-health commands such as `cargo test -- --nocapture health` when the lane's verify command is narrower or non-test-based.
3. Audit all active `rXMRbro` service lanes for health/proof commands that compile unrelated crates or suites.
4. Rewrite those proof commands to package-, binary-, or test-target-specific commands.

**Exit Criteria:**
- `house-agent-ws-accept-loop` and similar lanes prove their slice without inheriting unrelated warnings from `casino-core` or unrelated TUI surfaces.
- Generated workflows no longer synthesize obviously broad health gates without an explicit reason.

---

## Phase 5: Cut Codex-Unblock Waste

**Goal:** Stop expensive agent stages from burning tokens when the lane has no code delta to produce.

**Files:**
- Modify: `lib/crates/fabro-synthesis/src/render.rs`
- Modify: `lib/crates/fabro-workflows/src/backend/cli.rs`
- Modify: `lib/crates/fabro-workflows/src/handler/agent.rs`
- Test: `lib/crates/fabro-synthesis/src/render.rs`
- Test: `lib/crates/fabro-workflows/src/*`

**Steps:**
1. Add an earlier evidence-only/no-op exit for codex-unblock lanes whose preflight proof is already green and whose contract/implement stages produce no touched files.
2. Make stage accounting distinguish "useful no-op evidence lane" from "expensive empty implementation loop."
3. Add regression tests proving codex-unblock lanes do not spend full contract+implement budgets when they have no owned code changes to make.
4. Add telemetry/reporting for per-stage empty-diff token burn so future regressions are visible.

**Exit Criteria:**
- A codex-unblock lane with green owned proof and no required code delta exits cheaply.
- Empty-diff high-cost stages become observable and testable.

---

## Phase 6: Controller Discipline and Rollout

**Goal:** Make live controllers trustworthy proving grounds instead of moving baselines.

**Files:**
- Modify: `lib/crates/raspberry-cli/*` if needed for baseline propagation
- Modify: controller sync/rerender docs and scripts
- Update: `THEORY.MD`

**Steps:**
1. Require controller launches from a clean checkout whose local commits are either pushed or explicitly recorded as the run baseline.
2. Harden worktree bootstrap so stale run directories and stale git worktree registrations are pruned before new lane execution starts.
3. Add a preflight warning or hard failure when controller-local commits would poison audit baselines.
4. Document and automate the clean controller launch path for `rXMRbro`.
5. Rerender a fresh clean controller only after Phases 1-5 land together.

**Exit Criteria:**
- A live controller cannot silently run with a poisoned diff baseline.
- The rollout path itself encodes the controller hygiene rules we kept rediscovering manually.
- Fresh canary lanes do not fail at bootstrap due to stale worktree path or registration collisions.

---

## Verification Program

Run these in order after implementation:

1. Targeted unit tests for `fabro-synthesis` around:
   - audit baseline scoping
   - acceptance artifact ownership
   - unblock promotion truth
   - health command normalization
   - empty-diff codex-unblock exits
2. Targeted unit/integration tests for `fabro-workflows` around:
   - recorded run baseline propagation
   - stage outcome accounting for empty diffs
3. Fresh controller rerender in a clean checkout.
4. Live canary with 3 lanes first:
   - one UX lane
   - one narrow service lane
   - one codex-unblock lane
5. Full 10-lane autodev restart only after the canary proves:
   - no inherited audit surface violations
   - no acceptance-evidence self-failures
   - no broad health-command proof failures
   - no expensive empty-diff codex-unblock loops

---

## Non-Goals

- Do not tune individual lane code quality in isolation before the framework truthfulness bugs are fixed.
- Do not treat controller counts alone as success.
- Do not accept another round of incremental live hotfixes without a clean rerender and canary.

---

## Definition of Done

This remediation is done only when all of the following are true in the same fresh live run:

- audit and noop checks use the run baseline, not `origin/main`
- UX acceptance artifacts cannot self-trigger later audit failure
- implementation prompts and audit policy agree on out-of-surface behavior
- narrow service lanes prove only their owned surface
- codex-unblock lanes with no required code changes exit cheaply
- a 10-lane `rXMRbro` controller advances without the current recurring false-failure classes
