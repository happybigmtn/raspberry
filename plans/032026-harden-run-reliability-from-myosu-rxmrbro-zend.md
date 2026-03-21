# Harden Run Reliability from Myosu, rXMRbro, and Zend

This ExecPlan is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

`PLANS.md` is checked into the repository root and this document must be
maintained in accordance with it. This plan depends on
`plans/031926-harden-autonomy-and-direct-trunk-integration.md`,
`plans/032026-keep-fabro-and-raspberry-continuously-generating-work.md`, and
the now-complete Paperclip overlay plan
`plans/032026-sync-paperclip-with-raspberry-frontiers.md`.

## Purpose / Big Picture

After this slice lands, the proving-ground repos should stop burning cycles on
the same non-product failure families over and over. A human or agent should be
able to look at Myosu, rXMRbro, or Zend and see real product progress or one
clear blocker, not a pile of repeated direct-integration crashes, stale
`running` lanes, or generic cycle watchdog failures.

The user-visible result is that the next bounded runs on these repos produce
truthful outcomes. Myosu should stop looping on direct-integration merge
failures and stale platform state. rXMRbro should keep the successful product
momentum it already has while no longer failing because `origin` is missing or
branch forcing assumptions are wrong. Zend should stop collapsing multiple
distinct implementation failures into repetitive `fixup` cycles or stall
watchdog endings when the framework can see a more specific recovery path.

## Progress

- [x] (2026-03-20 22:08Z) Reviewed the current Paperclip plan and confirmed it
  is mature enough to call complete.
- [x] (2026-03-20 22:11Z) Assessed current Raspberry state for Myosu,
  rXMRbro, and Zend from `.raspberry/*-state.json`.
- [x] (2026-03-20 22:14Z) Assessed recent Fabro run truth from
  `~/.fabro/runs/*/manifest.json`, `status.json`, and `conclusion.json`.
- [x] (2026-03-20 22:17Z) Identified the dominant failure families across the
  three proving grounds: direct-integration landing failures, stale running
  state, and overly-generic recovery from verify/proof failures.
- [x] (2026-03-20) Implemented direct-integration hardening so repeated
  merge/push/branch failures stop presenting as inert or misleading framework
  failures. This included local-trunk fallback when `origin` is absent,
  replayable classification for missing-remote and linked-worktree branch-lock
  failures, and a local pending-settlement mode instead of hard failing when
  `main` is checked out in the target repo worktree.
- [x] (2026-03-20) Implemented runtime-state freshness repair so stale
  `running` lanes and stale failed lanes no longer lose authoritative truth.
  This included transient-launch reclassification, direct command launch
  instead of `bash -ic`, and preservation of failed `status.json` /
  `conclusion.json` truth even when `progress.jsonl` ends with
  `WorkflowRunCompleted`.
- [x] (2026-03-20) Improved failure-to-recovery mapping so verify/proof and
  landing failures route to truthful next actions instead of generic dead ends.
  Legacy failed lanes like `provably-fair` now classify as
  `integration_target_unavailable -> replay_lane` rather than remaining inert.
- [x] (2026-03-21) Tightened implementation workflow review doctrine so
  deterministic gates stay in the fixup loop while an explicit `gpt-5.4`
  `Review` stage owns the merge-readiness verdict and folds security review
  into that stage for trust-boundary-sensitive slices.
- [ ] Re-prove the result on bounded Myosu, rXMRbro, and Zend runs and update
  this plan with the outcomes.

## Surprises & Discoveries

- Observation: the Paperclip overlay is no longer the main bottleneck.
  Evidence: the current overlay now syncs issues, `plan` documents, transition
  comments, secrets, work products, and attachments well enough to treat its
  plan as complete.

- Observation: Myosu's recent runs are dominated by framework failures, not
  product-lane work.
  Evidence: in the most recent 20 Myosu-tagged runs, 17 ended in failure and 3
  were still running. The dominant failure string is repeated
  `git merge --squash failed ... Recorded preimage ...` direct-integration
  crashes. The top-level state still shows `games:multi-game` as `running` from
  `2026-03-19T06:16:00Z`, which is too stale to trust as a live execution
  signal.

- Observation: rXMRbro is already producing real product progress, but the run
  landing path is not honest about repo topology.
  Evidence: in the most recent 14 rXMRbro runs, 4 succeeded, 7 failed, and 3
  are still running. Successful runs include `Blackjack` and `Poker` bootstrap
  plus implementation slices (`01KM6PZ87YCJ3JRWN8BYZNXRQM`,
  `01KM6PZH9TKR44GG477G2AXGMK`). The dominant failure family is
  `git push failed ... fatal: 'origin' does not appear ...`, plus a target
  branch force failure on `HouseAgent`.

- Observation: Zend's failures are now mostly lane-local and specific, but the
  framework still lets them collapse into repetitive generic endings.
  Evidence: the most recent 20 Zend runs all failed. Eight ended in
  `node "fixup" visited 3 times`, two ended in `stall watchdog: node "verify"`,
  and the current lane states show richer failures underneath:
  `command-center-client` stalled in `verify`,
  `home-miner-service` failed in proof scripts after daemon/principal bootstrap,
  and `private-control-plane` failed on device pairing/daemon connectivity.

- Observation: `projectWorkspaceId` persists reliably into Paperclip, but the
  more advanced workspace-policy fields still behave inconsistently in the
  current local Paperclip instance.
  Evidence: direct API probes show `projectWorkspaceId` on synced issues, while
  `assigneeAdapterOverrides` and project `executionWorkspacePolicy` echo on
  PATCH and disappear on fresh GET. This plan should not depend on those fields
  to solve the proving-ground run failures.

- Observation: failed run truth can still be lost even when the run artifacts
  already contain enough information to classify recovery.
  Evidence: `provably-fair:provably-fair` in
  `/home/r/coding/rXMRbro/.raspberry/rxmragent-state.json` retained only
  `status=failed` and `last_run_id`, while the underlying run
  `01KM6R1Y26F4R7RCJPMACKQGMP` still contained a failed `conclusion.json`
  reason and a terminal `WorkflowRunCompleted` event in `progress.jsonl`. The
  old refresh path let the later event overwrite the authoritative failure,
  which erased `failure_kind` and `recovery_action`.

- Observation: local-only direct integration has a second repo-shape failure
  after the “missing `origin`” case.
  Evidence: `house-agent` failed with
  `git branch -f target branch failed ... cannot force update the branch 'main'
  used by worktree at '/home/r/coding/rXMRbro'`. The work itself was valid; the
  failure came from trying to move a branch ref that is checked out in the live
  target worktree.

## Decision Log

- Decision: call the Paperclip plan complete and move the next work back into
  Fabro/Raspberry run reliability.
  Rationale: the recent proving-ground assessment shows the strongest remaining
  blockers are now in direct integration, run-state freshness, and recovery
  behavior rather than in Paperclip coordination.
  Date/Author: 2026-03-20 / Codex

- Decision: prioritize generic framework fixes before repo-specific product
  changes.
  Rationale: Myosu and rXMRbro are both dominated by landing-path failures that
  are clearly shared framework behavior, and Zend is already providing specific
  proof/verify failure signals that the framework should preserve more
  faithfully.
  Date/Author: 2026-03-20 / Codex

- Decision: use the three proving grounds as separate acceptance lenses.
  Rationale: Myosu is the best signal for stale state and repeated landing
  failure, rXMRbro is the best signal for “product work succeeds but landing is
  wrong,” and Zend is the best signal for verify/proof recovery quality.
  Date/Author: 2026-03-20 / Codex

- Decision: treat local linked-worktree branch-ref failures as a replayable
  landing condition rather than a terminal product failure.
  Rationale: in local-only repos the existence of a checked-out `main`
  worktree is normal. The framework should keep that failure actionable and
  low-repeat instead of collapsing a valid product lane into `surface_blocked`.
  Date/Author: 2026-03-20 / Codex

- Decision: implementation workflows should use one explicit stronger-model
  `Review` gate instead of relying on `Settle` as an implicit review surrogate,
  and security scrutiny should be a required dimension of that review when the
  slice touches trust boundaries.
  Rationale: `Verify` and `Quality` are deterministic gates, `Fixup` is the
  deterministic repair loop, and the stronger-model stage should be clearly
  reserved for subjective review plus merge-readiness judgment rather than
  hidden promotion logic.
  Date/Author: 2026-03-21 / Codex

## Outcomes & Retrospective

This plan is being created from a concrete proving-ground assessment rather
than from theoretical architecture review. That is the right move. The current
system is already rich enough to expose its own next problems. The next useful
slice is not a new capability surface; it is making the existing autonomous
loop stop lying, looping, or failing for repo-shape reasons.

This is now the active run-reliability workstream rather than a speculative
plan. The direct-integration path is harder to fool, the run-state refresh path
no longer loses failed-conclusion truth, and the live `rXMRbro` controller now
surfaces top-level landing failures as replayable
`integration_target_unavailable` states instead of opaque inert failures.

The remaining acceptance work is proving the same sharper behavior on bounded
Myosu and Zend runs and recording those outcomes back into this document. Until
that happens, this plan should stay open and take precedence over new
run-reliability ideas.

When this plan is complete, the success criterion is not merely “some code was
refactored.” The success criterion is that new bounded runs on Myosu, rXMRbro,
and Zend either land cleanly or fail once with a truthful, actionable reason,
without repeating the same integration crash pattern across many consecutive
runs.

## Context and Orientation

The relevant framework files are all inside this repository.

Direct integration — the path that tries to land a successful run back onto the
target branch — lives in
`lib/crates/fabro-workflows/src/direct_integration.rs`. The CLI path that calls
it and turns failures into overall run conclusions lives in
`lib/crates/fabro-cli/src/commands/run.rs`.

Failure classification and scheduler recovery live in Raspberry supervisor:

- `lib/crates/raspberry-supervisor/src/failure.rs`
- `lib/crates/raspberry-supervisor/src/autodev.rs`
- `lib/crates/raspberry-supervisor/src/program_state.rs`

These files already understand failure kinds such as integration conflict,
environment collision, verify stall, and deterministic failure cycles. The next
work is to deepen their behavior so they stop producing repeated, low-signal
failures in real repos.

The proving grounds are:

- `/home/r/coding/myosu`
- `/home/r/coding/rXMRbro`
- `/home/r/coding/zend`

The current relevant state files are:

- `/home/r/coding/myosu/.raspberry/myosu-state.json`
- `/home/r/coding/myosu/.raspberry/myosu-platform-state.json`
- `/home/r/coding/myosu/.raspberry/myosu-product-state.json`
- `/home/r/coding/rXMRbro/.raspberry/rxmragent-state.json`
- `/home/r/coding/rXMRbro/.raspberry/rxmragent-*-implementation-state.json`
- `/home/r/coding/zend/.raspberry/zend-state.json`
- `/home/r/coding/zend/.raspberry/zend-*-implementation-state.json`

The recent run truth lives under `~/.fabro/runs/<run-id>/`.

### Detailed Repo Assessment

#### Myosu

Myosu currently looks framework-blocked rather than product-blocked.

The top-level state in `myosu-state.json` still shows only one live lane:
`recurring:program`, while `myosu-platform-state.json` still marks
`games:multi-game` as `running` with `current_run_id` `01KM2BS4ASVRXVT2ND1GVVMKJ0`
from 2026-03-19. That signal is stale enough that it should not continue to
block truthful planning.

The product state is also telling us something useful: `agent:experience` is
`ready`, while `play:tui` is `blocked`. So there is real frontier information
available if we stop drowning it in landing-path noise.

The most recent 20 Myosu-tagged runs are overwhelmingly bad:

- 17 `fail`
- 3 still `running`

The dominant failure family is repeated direct-integration crash output around
`git merge --squash failed ... Recorded preimage ...`. One recent run
(`01KM6Q2M6B4017BKDMXZ5986RJ`) instead failed with
`goal gate unsatisfied for node verify and no retry target`, which is also a
framework-level signal worth hardening. Two current runs are still active:

- `01KM6SVGFXHVETGJZ7MN3HTMMM` (`ChainRuntimeRestart`)
- `01KM6SXA8QR5AWHVVYJC4QSY1J` (`AgentIntegration`)

This repo is the best proving ground for stale-state repair and for making
direct integration fail once, clearly, instead of many times.

#### rXMRbro

rXMRbro is healthier than Myosu. It is already proving that product work can
advance:

- `01KM6PFBB5MA1YKEYXA12AAX1B` (`Blackjack`) succeeded
- `01KM6PFBB5R4YM0AJZA92VMJNA` (`Poker`) succeeded

The top-level supervisor state also shows meaningful product motion:

- `casino-core-implementation:program` is `running`
- `house-agent-implementation:program` is `running`
- `monero-infrastructure-implementation:program` is `running`
- `blackjack-implementation:program` is `ready`
- `poker-implementation:program` is `ready`
- `provably-fair:provably-fair` is `failed`

The last 14 runs break down as:

- 4 `success`
- 7 `fail`
- 3 `running`

The dominant failure family is not product logic. It is direct integration
assuming repo/remote behavior that is not present:

- repeated `git push failed ... origin does not appear ...`
- one `git branch -f target branch failed ...`

That means rXMRbro is the best proving ground for “product code is real, but
landing semantics are wrong.” A good fix here should preserve the already-good
implementation progress while removing remote/topology assumptions from the
landing path.

#### Zend

Zend is no longer failing opaquely. It is failing specifically.

The top-level Zend state shows:

- `hermes-adapter-implementation:program` is `ready`
- `command-center-client-implementation:program` is `failed`
- `home-miner-service-implementation:program` is `failed`
- `private-control-plane-implementation:program` is `failed`

The child implementation states show three distinct families:

1. `command-center-client`:
   `stall watchdog: node "verify" had no activity for 1800s`

2. `home-miner-service`:
   proof script failure after daemon start and principal bootstrap

3. `private-control-plane`:
   proof script failure caused by idempotent pairing/device state plus daemon
   connectivity issues

The last 20 Zend-tagged runs all failed. Eight ended in
`node "fixup" visited 3 times`, two ended in verify stall watchdog, and several
others still died in direct integration merge-squash failures.

That makes Zend the best proving ground for preserving richer stage failures,
separating proof-script idempotence problems from generic cycle collapse, and
making verify stalls route to one truthful next action instead of repeated
fixup churn.

## Plan of Work

### Milestone 1: Direct Integration Must Stop Dominating the Error Stream

The first milestone is inside
`lib/crates/fabro-workflows/src/direct_integration.rs` and
`lib/crates/fabro-cli/src/commands/run.rs`.

The direct-integration path already supports local-only landing when `origin`
is missing, but the recent runs show that the proving grounds are still
producing repeated merge-squash, push, and target-branch-force failures. This
milestone must make those failures deterministic, low-repeat, and more
informative.

Concretely:

- distinguish “cannot land because repo topology has no usable remote” from
  “cannot land because merge conflict/preimage exists”
- avoid retrying the same landing failure family across many consecutive runs
  when the source tree did not materially change
- make the recorded failure reason short, stable, and classifiable so
  Raspberry recovery does not thrash

Acceptance for this milestone is that Myosu and rXMRbro no longer emit long
chains of near-identical direct-integration failures across consecutive runs.

### Milestone 2: Running State Must Expire or Repair When It Is Clearly Stale

The second milestone is inside
`lib/crates/raspberry-supervisor/src/program_state.rs` and
`lib/crates/raspberry-supervisor/src/evaluate.rs`.

The proving-ground state files show clearly stale `running` records that should
not remain authoritative forever. A lane that has no active live run, no recent
progress, and no corroborating child state must stop presenting as live running
truth.

Concretely:

- define a stale-running repair rule based on available run/state evidence
- clear or downgrade stale `current_run_id` / `current_stage_label` records
  during refresh
- ensure the repaired state still preserves the best last failure or last
  completion evidence rather than erasing useful history

Acceptance for this milestone is that Myosu no longer shows obviously stale
long-dead platform work as `running`.

### Milestone 3: Recovery Should Reflect the Actual Failure, Not the Last Wrapper

The third milestone is inside
`lib/crates/raspberry-supervisor/src/failure.rs`,
`lib/crates/raspberry-supervisor/src/autodev.rs`, and
`lib/crates/fabro-workflows/src/engine.rs`.

Zend’s current failures show that the framework is still too willing to collapse
specific proof or verify problems into generic `fixup` loop or cycle endings.
That makes reruns noisy and hides the next correct move.

Concretely:

- preserve proof-script and verify-stage failure evidence over later wrapper
  failures when it is more actionable
- distinguish at least these families in scheduler-facing behavior:
  verify stall, deterministic proof-script failure, integration conflict, and
  daemon/resource collision
- make the resulting recovery action deterministic enough that the next bounded
  run either retries the right thing once or stops with a clear blocker

Acceptance for this milestone is that Zend’s failing implementation lanes stop
repeating the same `fixup` cycle endings when the richer underlying failure is
already known.

## Concrete Steps

All commands below run from `/home/r/coding/fabro` unless stated otherwise.

Start by recording the current proving-ground truth:

    jq '{program,lanes}' /home/r/coding/myosu/.raspberry/myosu-state.json
    jq '{program,lanes}' /home/r/coding/rXMRbro/.raspberry/rxmragent-state.json
    jq '{program,lanes}' /home/r/coding/zend/.raspberry/zend-state.json

Capture recent run outcomes:

    find ~/.fabro/runs -maxdepth 2 -name manifest.json | while read -r m; do
      dir=$(dirname "$m")
      host=$(jq -r 'if type=="object" then (.host_repo_path // "") else "" end' "$m")
      case "$host" in
        *myosu*|*rXMRbro*|*zend*)
          jq -r '[.run_id,.workflow_name,.start_time,.host_repo_path] | @tsv' "$m"
        ;;
      esac
    done | sort -k3 | tail -n 60

When implementing Milestone 1, work in:

    lib/crates/fabro-workflows/src/direct_integration.rs
    lib/crates/fabro-cli/src/commands/run.rs

When implementing Milestone 2, work in:

    lib/crates/raspberry-supervisor/src/program_state.rs
    lib/crates/raspberry-supervisor/src/evaluate.rs

When implementing Milestone 3, work in:

    lib/crates/raspberry-supervisor/src/failure.rs
    lib/crates/raspberry-supervisor/src/autodev.rs
    lib/crates/fabro-workflows/src/engine.rs

## Validation and Acceptance

Validation is not “all tests compile.” Validation is proving that recent
failure families change shape in the three proving grounds.

At minimum:

1. Run targeted Rust tests for the touched crates.
2. Re-run one bounded proving-ground command per repo.
3. Compare the new failure or success shape against the assessment captured in
   this plan.

The proving-ground acceptance targets are:

- **Myosu**: repeated direct-integration merge failures should stop dominating
  consecutive runs, and stale long-dead platform work should not remain
  `running`.
- **rXMRbro**: product work should still be able to run, but runs should no
  longer die merely because `origin` is absent or a target branch cannot be
  forced.
- **Zend**: failing implementation lanes should surface their real failure
  family directly instead of ending mainly in generic `fixup` cycles.

## Idempotence and Recovery

This plan must be implemented in a way that is safe to retry.

- State refresh changes should be idempotent: running them twice should not
  further mutate a lane that is already repaired.
- Direct-integration hardening should not destroy existing run branches or repo
  state. Prefer earlier classification and safer exit over more destructive git
  behavior.
- When a proving-ground rerun still fails, record the exact run id and updated
  failure reason back into this plan instead of guessing.

## Artifacts and Notes

Important recent evidence already collected:

- Myosu stale platform run: `games:multi-game` in
  `/home/r/coding/myosu/.raspberry/myosu-platform-state.json`
- Myosu repeated direct-integration failures:
  `01KM6KMB1V4YN90DVTCWCDM283`, `01KM6M51EB1AFBHY9SZEKN0BBA`,
  `01KM6QNKSS9MT9X7DTVNBNZ13Y`
- rXMRbro successful product runs:
  `01KM6PFBB5MA1YKEYXA12AAX1B`, `01KM6PFBB5R4YM0AJZA92VMJNA`
- rXMRbro remote/topology failures:
  `01KM6QQRW1PRFZ47YCBT59E1N2`, `01KM6QWKZ9G8KQ5A0Q07AN9CRW`
- Zend verify/proof failures:
  `01KM6NC440JB10WN834E86J1GM`, `01KM6NAJWYAYYJH9C98B5RJ7CE`,
  `01KM6MTED2N5JN3SACRBDFXEE9`

## Interfaces and Dependencies

Do not introduce a new scheduler or a new state store.

The implementation should continue to use:

- `fabro_workflows::direct_integration`
- `raspberry_supervisor::refresh_program_state`
- `raspberry_supervisor::classify_failure`
- `raspberry_supervisor::FailureRecoveryAction`

If a new helper or type is necessary, define it in one of the existing crates
above and keep the API narrow. The success criterion is sharper behavior from
the current system, not a new abstraction layer.
