# Theory: Keep Fabro and Raspberry Producing Real Lane Progress

## Problem

The current proving ground is failing in fewer places, but the remaining
failures still come from deterministic gates reading the wrong source of truth.
The original contract-stage drift in `rXMRbro` has mostly been contained, and
the overnight run exposed two later-stage synthesis problems that turned good
work into false failures.

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

## Operating Theory

Autodev only makes trustworthy progress when each gate consumes artifacts that
are explicitly owned by an earlier stage, and when diff-scoped checks measure
only the current lane's branch-local work.

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
`git status --porcelain` is otherwise empty. That matches the actual contract
stage contract: one ignored file, no product edits yet.

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

These are both the same class of bug: a deterministic gate using the wrong
source of truth. One gate looked for an artifact nobody owned, and the other
looked at a diff base that belonged to controller operations rather than the
lane.

## Open Questions

The remaining live question is how much later-stage fixup pollution is still
left once the acceptance, diff-baseline, and contract-gate truth bugs are
rolled back into the active controller and replayed. The evidence so far says
the system is moving in the right direction, but the proving pass still needs
to confirm that fresh rerendered runs advance cleanly beyond contract, review,
and audit instead of simply failing at the next deterministic gate.

The other open question is operational rather than synthetic: controller
freshness still matters. Even with better workflow generation, a dirty or
behind controller checkout can still stop dispatch. That is a controller-runbook
problem, not a lane-logic problem, and it should stay separate from synthesis
debugging.
