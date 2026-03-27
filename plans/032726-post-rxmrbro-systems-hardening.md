# Post-rXMRbro Systems Hardening

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [PLANS.md](/home/r/coding/fabro/PLANS.md).

## Purpose / Big Picture

After this change, `fabro` and `raspberry` should fail earlier, recover more accurately, and generate fewer self-inflicted proofs when used against a real proving-ground repository like `rXMRbro`. The point is not just to make one repo pass; it is to prevent autodev from repeatedly converting controller mistakes, stale recovery metadata, and generator drift into opaque lane failures.

The visible proof is a rerun against the repaired `rXMRbro` baseline where controller provenance problems are caught before dispatch, replay selection prefers the right source-lane recoveries, and generated service workflows do not rediscover the shell/proof bugs we already observed live.

## Progress

- [x] (2026-03-27 14:54Z) Promoted the ad hoc `rXMRbro`-side fabro findings into this repo-local ExecPlan under `fabro/plans/`.
- [x] (2026-03-27 15:03Z) Hardened controller provenance checks in `raspberry-supervisor` so a target repo with a non-pushable `origin` now fails the freshness gate before dispatch.
- [x] (2026-03-27 15:04Z) Added and passed a focused supervisor test freezing the local-path `origin` failure mode.
- [x] (2026-03-27 15:05Z) Re-ran the focused supervisor tests for recovery reclassification and replay prioritization; they still pass with the new provenance outcome.
- [x] (2026-03-27 15:10Z) Rebuilt `fabro` and `raspberry`, rerendered `/home/r/coding/rXMRbro-autodev-fresh3`, and revalidated that `house-agent-ws-accept-loop.fabro` still has the corrected single-wrapper `preflight` and smoke-style `verify`.
- [x] (2026-03-27 15:18Z) Relaunched `rXMRbro` autodev with the rebuilt binaries and confirmed fresh source-lane replays are starting without the old local-path `origin` push failure.
- [x] (2026-03-27 15:19Z) Reviewed replay behavior after the repaired `rXMRbro` baseline: source-lane recovery is active again and does not currently need another immediate priority patch.
- [x] (2026-03-27 15:20Z) Consolidated the remaining postmortem notes; the unresolved items are now mostly follow-up opportunities rather than blockers to this hardening pass.

## Surprises & Discoveries

- Observation: A target repo can look “fresh enough to dispatch” even when its `origin` remote is a local filesystem path that cannot be used for branch-backed run pushes.
  Evidence: Fresh source lanes in `rXMRbro-autodev-fresh3` failed with `failed to resolve SSH push URL ... remote origin must use SSH or be convertible from GitHub HTTPS, got /home/r/coding/rXMRbro-autodev-fresh2` only after dispatch had already happened.

- Observation: The existing `fabro_workflows::git::resolve_ssh_push_url` helper was already the right policy source for this failure mode; the problem was that autodev freshness checks were not using it.
  Evidence: After wiring `ensure_target_repo_fresh_for_dispatch` through that helper, the new `ensure_target_repo_fresh_for_dispatch_rejects_local_path_origin` supervisor test passed without needing a new remote parser.

- Observation: Reclassification and replay prioritization were both real failure domains, not just operator confusion.
  Evidence: The supervisor previously stored lanes as `unknown/surface_blocked`, then later text evidence implied `integration_target_unavailable`, but the stale recovery action remained in force until `recovery_decision_for_lane` was introduced.

- Observation: Service proof generation had two distinct live bugs in sequence: first it treated long-running `cargo run` commands as blocking verifies, then preflight double-wrapped the corrected smoke script and produced an invalid shell command.
  Evidence: The replayed `house-agent-ws-accept-loop` run in `rXMRbro` showed both failure modes in separate attempts.

- Observation: After the provenance hardening and repository-baseline repair, fresh `rXMRbro` source lanes now start directly instead of rediscovering the old controller-remote failure.
  Evidence: The fresh autodev wave includes source lanes such as `house-agent-ws-accept-loop`, `test-coverage-wallet-rpc-tests`, `three-card-poker`, `tower`, and `wheel` with new run IDs, and the fresh `house-agent-ws-accept-loop` log starts at `preflight` instead of failing on `resolve SSH push URL`.

- Observation: The remaining failure surface in the current `rXMRbro` proving run is mostly proof-script and product debt, not the original controller-provenance bug.
  Evidence: The current failed-lane set is dominated by `proof_script_failure`, plus one `transient_launch_failure` for `sic-bo-tui-screen`, while the old local-path `origin` failure is absent from the fresh failed-lane snapshot.

## Decision Log

- Decision: Start the fabro-side hardening pass with controller provenance validation.
  Rationale: It is the clearest remaining system bug that still allows obviously broken controller state to reach live lanes before failing.
  Date/Author: 2026-03-27 / Codex

- Decision: Reuse `fabro_workflows::git::resolve_ssh_push_url` inside autodev freshness checks instead of adding an autodev-only remote validation rule.
  Rationale: The workflows layer already defines the canonical branch-backed push contract. Reusing it prevents drift between “fresh enough to dispatch” and “valid enough to push run branches”.
  Date/Author: 2026-03-27 / Codex

- Decision: Treat the `rXMRbro` baseline repair as the evidence source, not as a one-off special case.
  Rationale: The point of this plan is to encode system protections that generalize to future proving-ground repos, not to add repo-specific exceptions.
  Date/Author: 2026-03-27 / Codex

- Decision: Keep using focused supervisor and synthesis tests as the primary safety net while hardening the system.
  Rationale: These bugs are about orchestration behavior, and the fastest safe way to iterate is to freeze them into tight unit/integration tests before relying on another live run.
  Date/Author: 2026-03-27 / Codex

## Outcomes & Retrospective

This hardening pass completed the first post-`rXMRbro` systems milestone successfully. The supervisor now rejects one of the clearest controller-provenance failures before dispatch, using the same SSH push-target policy that the workflows layer already enforces later during integration. That closes the gap where a controller checkout could look “fresh” yet still be doomed to fail as soon as a run branch tried to push.

The next `rXMRbro` proving run showed the expected improvement: fresh source lanes launched without rediscovering the old local-path `origin` failure, and the generated `house-agent-ws-accept-loop` workflow still carried the corrected service proof shape. The system is not “done” in the sense of zero remaining product or proof debt, but the particular fabro-side failure domain targeted by this plan is now meaningfully hardened.

## Context and Orientation

The relevant fabro surfaces are these:

- `lib/crates/raspberry-supervisor/src/autodev.rs`
  This is the center of truth for autodev dispatch freshness, replay selection, and cycle orchestration. If a target repo should be rejected before dispatch, or a failed lane should be replayed differently, the logic lives here.

- `lib/crates/raspberry-supervisor/src/failure.rs`
  This classifies lane failures into named kinds and defines default recovery actions. It matters because late or stale classification caused the supervisor to keep the wrong recovery behavior even after clearer evidence was available.

- `lib/crates/fabro-synthesis/src/render.rs`
  This generates implementation workflows, including service preflight, verify, and health scripts. The invalid duplicated preflight shell script seen in `rXMRbro` came from this layer.

- `lib/crates/raspberry-cli/src/main.rs`
  This owns `sync-controller`. It already creates clean controller worktrees correctly when used as intended, but it is part of the provenance story because operators rely on it to avoid dirty human checkouts.

- `lib/crates/fabro-workflows/src/git.rs`
  This already defines `resolve_ssh_push_url(repo, remote)`. That helper encodes the exact policy for whether `origin` is a usable branch-backed push target. The supervisor should reuse this policy rather than hand-rolling a looser one.

The key lesson from `rXMRbro` is that the system must guard against three classes of failure.

The first class is controller provenance failure. A controller checkout can be on the right branch and still be unusable if its remote configuration is wrong for run-branch pushes.

The second class is recovery truth drift. A lane can be recorded with one failure kind and recovery action, while newer evidence points to a different, more useful classification. The supervisor must recompute from current evidence instead of trusting stale metadata too much.

The third class is generated proof drift. Service lanes need a consistent model for preflight, verify, and health. If those stages are normalized separately, they can diverge and generate invalid shell or misleading proof scope.

## Plan of Work

The first milestone is provenance hardening. Change the autodev target-repo freshness gate so it rejects a repo whose `origin` cannot be resolved into a valid SSH push target. This should happen before dispatch and should produce an operator-facing reason that clearly explains why the controller repo is unsafe for branch-backed runs. Reuse `fabro_workflows::git::resolve_ssh_push_url` instead of inventing a second notion of valid `origin`.

The second milestone is replay-safety verification. Once the provenance gate is in place, rerun the focused supervisor tests for reclassification and replay ordering. The recent fixes to `recovery_decision_for_lane` and `prioritized_failure_lane_keys` must still behave correctly after the new freshness outcome is introduced.

The third milestone is post-baseline reproving. With the repaired `rXMRbro` human repo and the updated fabro binary, rerender the clean controller and inspect the generated `house-agent-ws-accept-loop.fabro` again. Then perform a fresh proving run and watch whether source-lane recovery now bypasses the old controller-path failure entirely.

The fourth milestone is plan consolidation. Revisit the remaining items in the `rXMRbro` note and either:

- convert them into concrete fabro tasks with tests and acceptance criteria, or
- mark them as already handled by the work now present in `autodev.rs`, `failure.rs`, and `render.rs`.

## Concrete Steps

To implement provenance hardening, inspect and modify:

- `lib/crates/raspberry-supervisor/src/autodev.rs`
- `lib/crates/raspberry-supervisor/src/failure.rs` only if a new explicit freshness outcome needs a matching operator-facing description
- `lib/crates/raspberry-cli/tests/cli.rs` only if the `sync-controller` contract needs new verification

The first proof commands for this plan are:

Working directory: `/home/r/coding/fabro`

    cargo test -p raspberry-supervisor replayable_failed_lanes_upgrade_unknown_recovery_action_from_last_error
    cargo test -p raspberry-supervisor prioritized_failure_lane_keys_prefer_source_replay_after_remote_fix

After adding the provenance gate, run the new focused test that freezes the local-path `origin` failure. Then rerun:

    cargo test -p raspberry-supervisor <new provenance test name>
    cargo test -p raspberry-supervisor replayable_failed_lanes_reclassify_unknown_failures_from_last_error
    cargo test -p raspberry-supervisor replayable_failed_lanes_dispatch_codex_unblock_for_provider_access_limits
    cargo build -p fabro-cli -p raspberry-cli --target-dir target-local

Once the binary is rebuilt and the `rXMRbro` human repo remains green, rerender the clean controller:

    ./target-local/debug/fabro --no-upgrade-check synth create \
      --target-repo /home/r/coding/rXMRbro-autodev-fresh3 \
      --program rxmragent \
      --blueprint /home/r/coding/rXMRbro-autodev-fresh3/malinka/blueprints/rxmragent.yaml \
      --no-decompose --no-review

Then inspect:

    sed -n '1,40p' /home/r/coding/rXMRbro-autodev-fresh3/malinka/workflows/implementation/house-agent-ws-accept-loop.fabro

and, when ready to reproving autodev again:

    ./target-local/debug/raspberry autodev \
      --manifest /home/r/coding/rXMRbro-autodev-fresh3/malinka/programs/rxmragent.yaml \
      --fabro-bin /home/r/coding/fabro/target-local/debug/fabro \
      --max-parallel 10 \
      --max-cycles 0 \
      --poll-interval-ms 1000 \
      --evolve-every-seconds 0

## Validation and Acceptance

Acceptance is met when:

1. A target repo with a bad `origin` push target is rejected before dispatch with a clear operator-facing reason.
2. The focused supervisor replay/reclassification tests still pass after the provenance change.
3. The rebuilt binary can regenerate `rXMRbro`’s controller package successfully.
4. The generated `house-agent-ws-accept-loop.fabro` still contains the corrected single-wrapper preflight and smoke-style verify scripts.
5. A fresh `rXMRbro` proving run no longer wastes a source-lane replay on the old local-path `origin` controller failure.

These conditions are now met.

## Idempotence and Recovery

The tests and rebuild commands in this plan are safe to rerun. The provenance gate should only make autodev stricter about obviously broken target repos; it should not mutate repository history on its own.

If the new freshness rule is too strict and blocks a legitimate local-only proving flow, the test added in this plan should make that obvious. In that case, update the plan and encode the intended exception explicitly rather than weakening the rule ad hoc.

## Artifacts and Notes

The seed evidence for this plan lives outside the fabro repo in:

- `/home/r/coding/rXMRbro/plan/008-restore-autodev-baseline/fabro-systems-plan.md`
- `/home/r/.fabro/runs/20260327-01KMQS2B69AVS0QN9RRSE7QWG7/progress.jsonl`

These records captured the exact failure shapes that motivated this plan:

    failed to resolve SSH push URL ... got `/home/r/coding/rXMRbro-autodev-fresh2`
    /bin/bash: -c: line 12: syntax error near unexpected token `&`

## Interfaces and Dependencies

This plan should preserve and reuse:

- `fabro_workflows::git::resolve_ssh_push_url`
- `raspberry-supervisor`’s existing recovery tests around `recovery_decision_for_lane`
- `fabro-synthesis`’s current service verify/preflight normalization model

Do not add a second independent parser for remote validity if the existing git helper can be reused. Do not add repo-specific special cases for `rXMRbro`; encode general branch-backed controller rules instead.

Change note: created on 2026-03-27 after the `rXMRbro` baseline repair surfaced the remaining fabro-side failure domains clearly enough to promote them into a repo-local ExecPlan.
