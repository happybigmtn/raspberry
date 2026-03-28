# Theory: Keep Fabro and Raspberry Producing Real Lane Progress

## Problem

The current proving ground is failing in fewer places, but the remaining
failures still come from deterministic gates and state evaluators reading the
wrong source of truth. The original contract-stage drift in `rXMRbro` has
mostly been contained, and the overnight run exposed two later-stage synthesis
problems that turned good work into false failures.

The first was a workflow ownership mismatch: UX implementation lanes emitted an
`acceptance_gate` that required `acceptance-evidence.md`, but no stage owned
that file. Review writes `.fabro-work/promotion.md`, not
`acceptance-evidence.md`, so healthy lanes could pass verify, quality,
challenge, and review, then fail on a deterministic missing-artifact check.

The second was audit truth leakage across branch boundaries. Implementation
audits and no-op guards were diffing against `origin/main`, which is wrong for
local controller checkouts that intentionally carry rerender or settle commits
ahead of origin. That made live lanes inherit controller-local `malinka/`
changes and report them as their own surface violations.

The newest live replay exposed a third gate-truth bug. The new
`contract_check` guard looked for `.fabro-work/contract.md` by inspecting
`git diff-tree HEAD`, but `.fabro-work` is intentionally ignored, so the guard
failed even when the contract file existed and the rest of the worktree was
clean.

The next investigation exposed a fourth truth bug on the Raspberry side.
Successful implementation runs were finishing with the correct durable
artifacts in their branch-backed worktrees, but the lane evaluator only counted
artifacts visible in the controller checkout. That meant a lane could finish
successfully, record `last_exit_status: 0`, and still be reclassified to
`ready` because the controller had not landed the artifact files locally.

The latest overnight landings exposed a fifth, subtler problem: some
test-coverage lanes can land low-value or misleading work even when the stage
graph itself is behaving correctly. In practice that showed up as duplicate
test suites instead of extensions to the canonical suite, dead `src/*_tests.rs`
files that were no longer wired into the crate, stale `spec.md` or `review.md`
artifacts drifting into source directories, and tests that asserted fake IDs or
string formatting instead of real state transitions.

The fresh `fresh5` replay exposed a sixth synthesis mismatch. Implementation
workflow goals still advertised root `spec.md` and `review.md` as the required
durable artifacts, while the actual audit and artifact model validated
lane-scoped files under `outputs/<lane>/...`. That contradiction was enough to
steer healthy runs back toward editing repo-root docs even after the quality
gate stopped treating every root artifact as an automatic failure.

The first `fresh6` replay exposed a seventh, more subtle contract-stage leak.
The contract prompt correctly said "write `.fabro-work/contract.md` only", but
the deterministic `contract_check` gate only verified that the ignored
`.fabro-work` directory was clean apart from `contract.md` and that the
post-checkpoint worktree had no unstaged changes. That let the contract-stage
checkpoint commit carry tracked `outputs/<lane>/*.md` changes without being
rejected, so contract agents could still author durable lane artifacts too
early.

## Operating Theory

Autodev only makes trustworthy progress when each gate and each milestone
classifier consumes artifacts that are explicitly owned by an earlier stage,
and when diff-scoped checks measure only the current lane's branch-local work.

That means the review stage must remain the single owner of
`.fabro-work/promotion.md`, and any later deterministic acceptance gate has to
derive its decision from that file rather than invent a parallel acceptance
artifact. In the current implementation-family workflow, `promotion.md` already
contains the only durable merge-readiness verdict the framework trusts:
`merge_ready`, `manual_proof_pending`, the review scores, and any mandatory
security verdict fields.

It also means lane diff checks need to anchor to the branch point the run
actually forked from, not to a remote branch that may lag or intentionally omit
controller-local rerender commits. In practice the right fallback order is:
prefer the local controller branch (`main`, `autodev-main`, or
`autodev/main`), then other local trunk names, then remote trunk names, and
only then fall back to the repo root commit if no branch ref exists.

For ignored stage-local artifacts like `.fabro-work/contract.md`, the gate
cannot rely on git history at all. It has to inspect the filesystem directly
and separately confirm that the rest of the repo remains clean.

But the filesystem-only check is still not enough by itself. Because the
contract checkpoint creates a git commit immediately before `contract_check`,
the gate also has to verify that the latest tracked commit delta is empty.
Otherwise early `outputs/<lane>/...` artifact generation can be hidden inside
the contract checkpoint commit while the workspace still appears clean.

For milestone classification, a green run is not enough by itself, but a green
run plus the required artifact files in that run's worktree is enough to prove
the milestone. The controller checkout is not the only trustworthy artifact
surface once branch-backed implementation runs are part of the design.

For test-coverage lanes, passing proof commands is also not enough on its own.
The framework has to guard against structurally weak landings by checking for
source-artifact drift, unattached source test modules, and review prompts that
explicitly reject duplicate or fake-behavior coverage work.

For implementation lanes more generally, the workflow goal, prompt context,
quality gate, and audit gate must all agree on the same durable artifact
surface. If the goal says `spec.md` but audit expects
`outputs/test-coverage-house-protocol-tests/spec.md`, the agent will follow the
wrong contract and any later gate becomes partly luck.

## Strategy

The strategy is still to fix synthesis centrally instead of patching controller
repos by hand.

For UX implementation workflows, the acceptance gate now needs to consume the
review-owned `.fabro-work/promotion.md` and require `merge_ready: yes` plus
`manual_proof_pending: no`. The direct `review -> audit` success edge should be
removed for UX profiles so the acceptance gate is no longer bypassed by the
base workflow template. The review prompt should also say plainly that the next
gate reads `promotion.md`.

For audit and no-op guards, the generator should emit one shared local-first
diff-base expression and reuse it everywhere the workflow previously hardcoded
`git merge-base HEAD origin/main`. That keeps lane-local checks honest even
when the controller carries local rerender or integration commits that are
deliberately ahead of origin.

For `contract_check`, the generator should require the expected section headers,
assert that `.fabro-work` contains only `contract.md`, and assert that
`git status --porcelain` is otherwise empty. It also needs to assert that the
latest tracked commit delta is empty, which matches the actual contract stage
contract: one ignored file, no product edits yet, and no early durable
artifacts.

For Raspberry evaluation, unit artifact presence should remain rooted in the
controller checkout first, but it should be augmented with lane artifacts found
under the latest successful run worktree when they map to the same repo-relative
artifact paths. That preserves the stale-success guard while letting fresh
successful implementation runs count as complete.

For quality gating on implementation lanes, especially `test-coverage-*` lanes,
the generator should treat changed markdown artifacts under `src/` or `tests/`
as artifact drift, and it should fail when a changed `src/*_tests.rs` file is
not actually referenced by the crate's module tree. The review and challenge
prompts should also say plainly that duplicate suites, fake-ID assertions, and
stale source-adjacent artifact markdown are not merge-ready outcomes.

For implementation-lane artifact guidance, the generator should rewrite the
goal and prompt context to name the exact lane-scoped artifact paths under the
unit output root and explicitly forbid writing durable artifacts to repo-root
fallback paths unless the lane really owns those files.

## Key Discoveries

The overnight run confirmed that the contract-stage hardening was real. Fresh
lanes reached `contract` and completed with no stray `spec.md` or `review.md`
writes.

The next deterministic blocker was not contract anymore. UX lanes like
`red-dog-tui` and `sic-bo-tui-screen` passed review and then failed the
acceptance gate because the required `acceptance-evidence.md` file had no
authoring stage.

The `house-agent-ws-accept-loop` audit failure exposed a second truth problem:
the audit surface guard was measuring against `origin/main`, so controller-local
workflow rerender commits appeared as lane-owned violations. That explains why
`malinka/workflows/...` showed up in lane audits even when the live slice stayed
inside its owned product surface.

The next live replay proved that the new `contract_check` bug was real. A fresh
`test-coverage-casino-core-tests` run completed `contract`, started
`contract_check`, and failed deterministically even though
`.fabro-work/contract.md` existed and `git status` was clean. The cause was
exactly the ignored-artifact mismatch: the gate was asking git for a file git
was designed to ignore.

The next live status probe showed the same bug class in Raspberry state
tracking. `test-coverage-casino-core-tests` and its unblock helper both had
successful `WorkflowRunCompleted` events and correct artifact files in their
run worktrees, but the stored lane state stayed `ready` because evaluation only
looked at the controller checkout. Running `raspberry status` with the patched
evaluator reclassified the lane to `complete`, which confirmed the state logic
bug without needing a new synthesis change.

The overnight quality sweep also showed that some landed work was weak without
being obviously broken. The wallet RPC coverage landing carried dead source
test artifacts after coverage had moved elsewhere, and the house-session count
test could pass without proving real decrement behavior. Those are exactly the
sorts of outcomes the existing verify and review stages are too permissive
about for test-coverage lanes.

The fresh `fresh5` run proved the new quality gate is live: the
`test-coverage-house-protocol-tests` lane reached `quality` and executed the
generated duplicate-test/source-artifact checks successfully. That same replay
also showed the remaining prompt mismatch clearly: fresh implementation lanes
were still touching root `spec.md` and `review.md` because the workflow goal
was instructing them to do so, even though the real durable artifacts lived
under `outputs/<lane>/...`.

The first `fresh6` run proved the scoped artifact rewrite was partially right
but not complete. Fresh contract stages stopped logging repo-root artifact
writes and instead wrote lane-scoped `outputs/<lane>/...` files, which is
better, but that work was still happening during `contract`. The old
`contract_check` passed because it only looked for a clean post-commit
workspace, not for tracked changes carried by the contract checkpoint commit.

The next live run exposed two more structural truths. First, even a correct
`contract_check` script is not a real gate if the workflow graph still routes
unconditionally from `contract_check` into `implement`; in that shape a failed
gate just becomes advisory. Second, local-run safety cannot rely on a
"best-effort" worktree path. If worktree setup or initialization fails and the
runner quietly falls back to a repo-less local sandbox, the lane will continue
into agent stages and only fail later with nonsense verify errors such as
"could not find Cargo.toml", which hides the real infrastructure fault.

These are all the same class of bug: a deterministic gate or classifier using
the wrong source of truth. One gate looked for an artifact nobody owned,
another looked at a diff base that belonged to controller operations rather
than the lane, another trusted git for an ignored file, and the evaluator
trusted only controller-local artifacts even though the fresh proof lived in
the successful run worktree. The latest replay extends that pattern: a
contract-stage gate trusted the live filesystem but ignored the tracked diff
that had just been committed, and the local runner trusted a degraded sandbox
state instead of treating failed worktree setup as the actual blocker.

## Open Questions

The remaining live question is how much later-stage fixup pollution is still
left once the acceptance, diff-baseline, contract-gate, and milestone
classification truth bugs are rolled back into the active controller and
replayed. The evidence so far says the system is moving in the right direction,
but the proving pass still needs to confirm that fresh rerendered runs advance
cleanly beyond contract, review, audit, and lane completion instead of simply
failing at the next deterministic gate. The immediate proving task is narrower:
confirm that fresh runs on the new binary now abort early on worktree setup
failure and that a failing `contract_check` really loops back to `contract`
instead of slipping into `implement`.

The other open question is operational rather than synthetic: controller
freshness still matters. Even with better workflow generation, a dirty or
behind controller checkout can still stop dispatch. That is a controller-runbook
problem, not a lane-logic problem, and it should stay separate from synthesis
debugging.
