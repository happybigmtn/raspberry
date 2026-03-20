# Keep Fabro and Raspberry Continuously Generating Work

This ExecPlan is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

`PLANS.md` is checked into the repository root and this document must be
maintained in accordance with it. This plan depends on
`plans/031926-build-skill-guided-program-synthesis.md`,
`plans/031926-build-raspberry-autodev-orchestrator.md`,
`plans/031926-harden-autonomy-and-direct-trunk-integration.md`, and the
companion coordination plan
`plans/032026-sync-paperclip-with-raspberry-frontiers.md`.

## Purpose / Big Picture

After this slice lands, Fabro and Raspberry should stop going idle just because
the currently-rendered manifest ran out of lanes. When the active frontier is
empty or spare worker capacity exists, the system should re-read the repo's
real doctrine, derive the next truthful frontier, and dispatch that work until
the actual product scope is exhausted or a concrete blocker is reached. It
should do that without exploding one empty worker slot into an unbounded backlog
of speculative child programs.

The user-visible result is that `raspberry autodev` becomes continuously useful
instead of periodically asleep. A repo such as Zend should keep moving from
bootstrap into the next implementation frontier without operator babysitting,
and a repo such as Myosu should turn failed work into the next correct replay
behavior instead of leaving the operator to infer what to do from opaque state.

## Progress

- [x] (2026-03-20 16:10Z) Re-read `PLANS.md`, the active autonomy plans, and
  the overnight proving-ground state in Zend and Myosu.
- [x] (2026-03-20 16:18Z) Identified the five most important design choices for
  the overnight problems: generative evolve, roadmap-aware parent programs,
  child-authoritative runtime truth, source-lane recovery, and source-tree
  quality gates.
- [x] (2026-03-20 16:22Z) Identified the next five improvements, then reduced
  scope during engineering review because the full set was too broad for one
  execution slice.
- [x] (2026-03-20 16:42Z) Split Paperclip coordination into a companion plan so
  this plan now reflects the Phase 1B scope decision instead of trying to solve
  two control planes in one pass.
- [x] (2026-03-20 16:55Z) Incorporated the architecture-review recommendations:
  explicit frontier budgeting, deterministic doctrine-derived candidate
  identity, and per-program doctrine-delta state.
- [x] (2026-03-20 16:56Z) Deepened
  `author_blueprint_for_evolve(...)` so parent programs merge missing
  doctrine-derived frontier units during evolve, while implementation child
  programs stay scoped to their own delivery contract.
- [x] (2026-03-20 16:56Z) Kept parent programs as living roadmaps by combining
  doctrine-derived parent-unit merge in `planning.rs` with the existing
  implementation follow-on child-program expansion in `render.rs`.
- [x] (2026-03-20 16:56Z) Let autodev use spare worker capacity to trigger
  bounded evolve, using explicit `frontier_budget` accounting instead of
  settle-only gating.
- [x] (2026-03-20 16:56Z) Persisted per-program doctrine-delta state under
  `.raspberry/` so README/spec/plan changes force a fresh evolve pass without
  one repo-global fingerprint file.
- [x] (2026-03-20 16:56Z) Made runtime truth more child-authoritative by
  preferring stored child-program state during parent refresh, surfacing shared
  recovery actions in status output, and splitting a local evaluation path for
  read-only CLI surfaces.
- [x] (2026-03-20 16:56Z) Added shared typed failure classification and routed
  replay policy through it so recovery is no longer driven only by ad hoc text
  matching.
- [x] (2026-03-20 16:56Z) Strengthened synthesis verification contracts by
  carrying explicit smoke gates from reviewed slice doctrine into
  user-visible implementation verify commands instead of relying on proof-only
  checks.
- [x] (2026-03-20 16:56Z) Tightened failure extraction so live run history keeps
  more actionable stage failures instead of collapsing everything into generic
  wrapper messages or late cycle-collapse summaries.
- [x] (2026-03-20 16:56Z) Taught autodev to honor typed `backoff_retry` and
  `refresh_from_trunk` actions as cooldown-based same-lane retries instead of
  surfacing those actions passively with no scheduler behavior behind them.
- [x] (2026-03-20 16:56Z) Normalized persisted `run_config` paths during state
  refresh so live `.raspberry/*-state.json` files stop accumulating absurd
  repeated `../../` chains over time.
- [x] (2026-03-20 16:56Z) Added the first automatic resource-leasing path for
  Zend-style daemon ports by leasing a port in Raspberry, passing it through to
  `fabro run`, and restarting the live Zend/Myosu controllers on the rebuilt
  binaries.
- [x] (2026-03-20 20:00Z) Audited the live Zend/Myosu proving-ground failures
  against the actual recurring-lane contracts and confirmed the current
  "operations" style lanes are report-only bootstrap lanes, not scheduler-owned
  recovery workers.
- [x] (2026-03-20 20:00Z) Identified the next missing Fabro/Raspberry behavior:
  blocked failures need a framework-owned recovery-dispatch path instead of
  stopping at `surface_blocked` and expecting repo-local recurring review lanes
  to unblock them by implication.
- [ ] Prove the result on Zend and Myosu with bounded five-worker runs.
  Note: read-only proving succeeded on both repos, but I did not start an extra
  bounded autodev run while the live watchdog loops were already active.

## What Already Exists

Several key pieces already exist and should be reused rather than rebuilt.

- `fabro synth evolve` already has the right outer seam. The relevant code is
  `lib/crates/fabro-synthesis/src/planning.rs` and
  `lib/crates/fabro-synthesis/src/render.rs`.
- `reconcile_blueprint(...)` already knows how to expand a blueprint into
  rendered programs, workflows, prompts, and run configs.
- `augment_with_implementation_follow_on_units(...)` already proved that
  synthesis can create post-bootstrap implementation child programs for Zend.
- `raspberry autodev` already knows how to import, evolve, replay, and dispatch
  work through `lib/crates/raspberry-supervisor/src/autodev.rs`.
- Runtime truth already has one durable storage surface in `.raspberry/*`,
  backed by `lib/crates/raspberry-supervisor/src/program_state.rs` and
  `lib/crates/raspberry-supervisor/src/evaluate.rs`.
- Direct-integration capability already exists from the prior overnight work.
  This plan does not rebuild it; it assumes it and focuses on keeping the
  frontier alive and honest.

## NOT in Scope

This plan intentionally does not include the following work, even though the
overnight runs exposed it as important.

- Paperclip synchronization and always-on coordination overlays. That work now
  lives in `plans/032026-sync-paperclip-with-raspberry-frontiers.md`.
- Merge-conflict preflight and automatic trunk-refresh repair for direct
  integration. That is a real next problem, but it is not required to prove
  continuous frontier generation.
- Broad host-level resource leasing beyond the first Zend daemon-port path. The
  initial Zend-style leased-port flow is now in scope, but a more general
  multi-resource leasing model still remains deferred.
- A new scheduler, a new state store, or a new synthesis pipeline. The goal is
  to deepen the existing seams, not replace them.

## Surprises & Discoveries

- Observation: the earlier "Zend is done" state was a synthesis boundary bug,
  not a product reality.
  Evidence: the bootstrap frontier settled cleanly while the repo plans still
  implied substantial post-bootstrap work.

- Observation: once synthesis emitted implementation child programs, Zend
  immediately moved from false settlement into real follow-on work.
  Evidence: the current Zend state file now records live or failed child
  programs for `command-center-client`, `hermes-adapter`, `home-miner-service`,
  and `private-control-plane`.

- Observation: Myosu no longer primarily needs better liveness reporting.
  Evidence: the parent state is now good enough to reveal that the real next gap
  is typed recovery from deterministic failures rather than more observer code.

- Observation: the biggest scheduler bug is not "cannot dispatch work" but
  "waits too long to author more work."
  Evidence: `autodev.rs` still gates evolve on local settlement, which means
  unused worker slots can coexist with unfinished doctrine.

- Observation: the best scope cut is along control-plane boundaries, not along
  code ownership boundaries.
  Evidence: keeping Paperclip in this slice would force simultaneous changes to
  synthesis, supervisor behavior, and an external coordination layer.

- Observation: repeated evolve passes will become harmful if they do not have a
  stable identity model and a bounded backlog policy.
  Evidence: the revised no-idle behavior intentionally increases evolve
  frequency, so duplicate child programs and frontier explosion become likely
  unless the plan explicitly constrains them.

- Observation: doctrine deltas should not preempt already-ready work.
  Evidence: the first spare-capacity implementation caused the CLI autodev
  tests to dispatch extra work because any doctrine change forced evolve even
  when ready lanes were already queued. Tightening that trigger fixed the churn.

- Observation: several fixture tests were still assuming that stale active runs
  should present as `running`.
  Evidence: the current supervisor truth surfaces mark the `p2p:chapter` and
  `miner:service` fixtures as failed stale runs, so the evaluation tests had to
  be updated to match the more honest runtime model.

- Observation: parent runtime refresh was still expensive enough to make live
  Myosu `status` unusable.
  Evidence: the first read-only Myosu status attempt timed out because parent
  refresh was falling through to full child-program evaluation. Making
  `sync_child_program_runtime_record(...)` prefer existing child state and
  routing the CLI through local evaluation restored fast top-level status.

- Observation: proof commands alone are not enough to keep user-visible
  implementation fronts honest.
  Evidence: `render.rs` already understood proof and health gates, but it still
  ignored explicit smoke commands in reviewed slice docs. Carrying those smoke
  gates into interface/service verify commands closes that gap.

- Observation: the final workflow failure message is often less actionable than
  the earlier stage-failure evidence in `progress.jsonl`.
  Evidence: the Zend command-center child ultimately reported a cycle wrapper,
  but the earlier verify-stage failures clearly showed `Errno 98`. Preferring
  richer stage failures during runtime extraction made the child status route to
  `backoff_retry` instead of generic `surface_blocked`.

- Observation: recovery labels are only half useful if autodev still treats
  them as inert metadata.
  Evidence: until this pass, `refresh_from_trunk` and `backoff_retry` rendered
  correctly in status output but never re-entered the replay queue. Adding
  cooldown-based lane replay closed that gap for the currently supported
  actions.

- Observation: runtime truth can also decay structurally even when the status is
  otherwise correct.
  Evidence: the live Zend child state had a `run_config` path that repeated
  `../../fabro/programs/` dozens of times. Normalizing and writing back the
  path during refresh fixed the state file without changing the manifest.

- Observation: the active Zend service/interface failures were already prepared
  to consume leased runtime env.
  Evidence: the relevant scripts honor `ZEND_BIND_PORT` and `ZEND_DAEMON_URL`,
  so the framework could address the current port-collision family without
  repo-specific surgery.

- Observation: the current recurring oversight lanes are not broken; they are
  simply not wired into recovery.
  Evidence: Myosu's `operations:scorecard` workflow only authors and verifies
  `spec.md` plus `review.md`, and Zend currently has no recurring oversight
  program at all. Neither repo has a lane that mutates scheduler state,
  synthesizes recovery work, or consumes `failure_kind` plus `recovery_action`
  as machine input.

- Observation: `surface_blocked` still means "stop" rather than "author the
  next unblock step."
  Evidence: `default_recovery_action(...)` currently routes
  `deterministic_verify_cycle`, `proof_script_failure`,
  `provider_policy_mismatch`, and `unknown` to `surface_blocked`, and
  `replay_target_lane(...)` turns `surface_blocked` into `None`, so autodev
  never replays or dispatches follow-on work for those families.

- Observation: the current Zend implementation wave is exposing contract drift
  more than generic scheduler weakness.
  Evidence: the latest command-center verify failure is not `Errno 98`; it is a
  capability mismatch where `alice-phone` is paired as observe-only but the
  verify script then asks it to issue a control command. Hermes also proved that
  workflow audit contracts can drift from settle outputs when `integration.md`
  is required by audit but not declared in the lane's required artifacts.

## Decision Log

- Decision: this plan now represents the Phase 1B slice from the engineering
  review, not the original all-in roadmap draft.
  Rationale: the original 10-improvement version was boiling several lakes and
  one small ocean at once. The reduced slice still achieves the core "never
  idle" goal while keeping the blast radius sane.
  Date/Author: 2026-03-20 / User + Codex

- Decision: the core goal is "no idle frontier," not "fix every failure mode in
  the same PR."
  Rationale: continuous frontier generation is the most leveraged missing
  behavior. Without it, the rest of the hardening work still leaves the system
  sleeping too often.
  Date/Author: 2026-03-20 / User + Codex

- Decision: runtime truth and typed recovery remain in scope because they are
  part of the same loop, not separate polish.
  Rationale: a system that generates work but cannot explain or recover the next
  step still creates operator drag instead of removing it.
  Date/Author: 2026-03-20 / Codex

- Decision: Paperclip remains important but must move into a companion plan.
  Rationale: it is a second control-plane concern. Keeping it here would make
  it too easy to accidentally design two schedulers instead of one scheduler and
  one coordination overlay.
  Date/Author: 2026-03-20 / User + Codex

- Decision: merge hardening and host-resource leasing are explicitly deferred.
  Rationale: they are important, but they are not on the critical path for
  proving continuous doctrine-driven frontier generation.
  Date/Author: 2026-03-20 / Codex

- Decision: the first Zend daemon-port leasing path is now in scope despite the
  earlier deferral.
  Rationale: the later directive to finish the milestone and restart the active
  projects made it worth landing the smallest real automatic remediation path
  instead of only describing it in theory.
  Date/Author: 2026-03-20 / User + Codex

- Decision: spare-capacity evolve must use an explicit frontier budget even
  though an earlier review reply accepted the looser option.
  Rationale: the later instruction to "implement all your recommendations"
  supersedes that earlier shortcut and lets the plan adopt the safer complete
  version. The budget should default to `max_parallel + 2` so the system can
  stay productive without flooding the parent manifest with speculative ready
  work.
  Date/Author: 2026-03-20 / User + Codex

- Decision: every doctrine-derived frontier candidate must have a deterministic
  identity.
  Rationale: repeated evolve passes are now expected behavior, so candidate
  identity must come from stable inputs such as source document path, section
  anchor, and normalized unit intent rather than from transient titles alone.
  Date/Author: 2026-03-20 / Codex

- Decision: doctrine-delta state must be scoped per program or per manifest,
  not as one repo-global file.
  Rationale: a shared repo-global fingerprint would make multiple active
  programs trip over each other and produce spurious evolve churn.
  Date/Author: 2026-03-20 / Codex

- Decision: doctrine-triggered evolve must not jump ahead of already-ready
  lanes.
  Rationale: doctrine changes are a reason to mint new work when the frontier is
  settled or starved, not a reason to re-author while dispatchable work is
  already waiting.
  Date/Author: 2026-03-20 / Codex

- Decision: read-only CLI surfaces must use local evaluation instead of parent
  propagation.
  Rationale: `status`, `plan`, and `watch` are truth surfaces. They should not
  silently trigger deeper repo-wide parent refresh when the operator only asked
  for a local view of one manifest.
  Date/Author: 2026-03-20 / Codex

- Decision: recurring oversight lanes remain report surfaces, while blocked-run
  recovery becomes a Fabro/Raspberry framework responsibility.
  Rationale: report lanes can describe stale doctrine and missing evidence, but
  they are not the right place to own scheduler behavior. Recovery policy must
  be consistent across repos, consume typed runtime evidence directly, and be
  able to dispatch new work without waiting for a repo-local human-oriented
  review lane to be interpreted by hand.
  Date/Author: 2026-03-20 / User + Codex

## Outcomes & Retrospective

This plan is now partially executed rather than only reviewed. The no-idle core
loop is in place: evolve can merge missing doctrine-derived parent frontier
units, autodev now uses a bounded spare-capacity evolve policy with per-program
doctrine state, and failure recovery surfaces a shared typed classification.
The newest runtime-truth work also removed a real operator footgun: parent
status no longer needs to fall through into expensive child evaluation when
child state already exists. The remaining high-value work is live bounded
proving on Zend/Myosu, plus continuing the quality-gate hardening from the same
theory.

## Context and Orientation

The current no-idle problem spans four code areas.

`lib/crates/fabro-synthesis/src/planning.rs` and
`lib/crates/fabro-synthesis/src/render.rs` are the synthesis seam. They turn
repo doctrine and current package state into the next rendered `fabro/`
programs. The recent Zend fix already proved that this layer can emit
implementation follow-ons after bootstrap. The remaining gap is that it still
leans too heavily on the current package and not enough on unfinished doctrine.

`lib/crates/raspberry-supervisor/src/autodev.rs` is the scheduler seam. It
decides whether to evolve, which failed lanes are replayable, and which lanes
to dispatch. Today it still mostly treats evolve as a settle-time action. That
is the main reason the system can appear idle even when the product roadmap is
not exhausted.

`lib/crates/raspberry-supervisor/src/program_state.rs` and
`lib/crates/raspberry-supervisor/src/evaluate.rs` are the truth seam. They
produce the `.raspberry/*-state.json` files and the CLI/TUI summaries operators
actually read. These files have already improved a lot, but they still need
canonical path cleanup, better supersession behavior, and typed failure state.

Zend and Myosu are the proving grounds. Zend is the clean continuation case: a
repo that should keep moving after bootstrap. Myosu is the messy long-lived
implementation case: a repo with real failures, stale history, and the need for
typed recovery.

The target loop after this plan lands is:

    doctrine files
        |
        v
    evolved frontier
        |
        v
    ready/running lanes up to max_parallel
        |
        +--> bounded ready backlog up to frontier_budget
        |
        v
    truthful runtime state
        |
        +--> typed recovery when failures occur
        |
        +--> fresh evolve when spare capacity or doctrine deltas exist

## Plan of Work

### Milestone 1: Continuous Frontier Generation

Deepen `author_blueprint_for_evolve(...)` in
`lib/crates/fabro-synthesis/src/planning.rs` so it derives frontier candidates
from `README.md`, `SPEC.md`, `specs/`, and `plans/` rather than mostly
re-importing the current package. The key new behavior is that unfinished plan
fronts become structured candidate units before the render step. This should be
deterministic and conservative: the goal is the smallest truthful next frontier,
not speculative lane explosion.

Every doctrine-derived frontier candidate must also carry a deterministic
identity derived from stable inputs. The identity should be based on the source
document path, the nearest stable section anchor or equivalent structural
location, and a normalized expression of the inferred unit intent. Repeated
evolve passes over unchanged doctrine must therefore re-identify the same
candidate instead of minting slightly renamed duplicates.

In `lib/crates/fabro-synthesis/src/render.rs`, keep treating the parent program
as the durable roadmap. The renderer should continue to add follow-on child
programs when upstream work settles, and it should preserve lineage and
dependency ordering so the rendered program still reads like the repo's plan
instead of like a bag of independent jobs.

### Milestone 2: Spare-Capacity Evolve and Doctrine-Delta Triggering

Extend `lib/crates/raspberry-supervisor/src/autodev.rs` so evolve is not only a
local-settlement privilege. If the repo has fewer active lanes than
`max_parallel`, and the doctrine-derived frontier implies more work, autodev
should be allowed to run import plus evolve and then dispatch whatever became
ready. This should still respect bounded cycles and backoff, but an empty slot
should become a reason to search for more work.

This behavior needs an explicit frontier budget. The default policy should be
that autodev may continue minting ready work while `running < max_parallel`, but
the combined frontier of `ready + running + replayable_failed` should stay below
`frontier_budget`, which should default to `max_parallel + 2` unless configured
otherwise. This keeps the system productive without turning one open slot into
an unreadable backlog explosion.

Add a small doctrine-delta state file under `.raspberry/` that records the last
observed hash or modification fingerprint for `README.md`, `SPEC.md`, `specs/`,
and `plans/`. This file must be scoped per manifest or per program, for example
via a filename derived from the manifest path or the program id, instead of a
single repo-global fingerprint file. If any of those inputs change, autodev
should force a fresh evolve pass even when the prior frontier looked settled.
The fingerprint logic should stay cheap: only doctrine inputs should be scanned,
and the state file should preserve enough per-file metadata to avoid expensive
full-tree rehashing on every autodev cycle.

### Milestone 3: Canonical Truth and Typed Recovery

Continue the runtime-truth cleanup in
`lib/crates/raspberry-supervisor/src/program_state.rs` and
`lib/crates/raspberry-supervisor/src/evaluate.rs` so parent state is directly
actionable. The next changes should canonicalize program-manifest and
run-config paths, ensure fresh runs clear stale failure residue, and preserve
the best available run-dir evidence when tracked run ids are missing or old.

Then extend `lib/crates/raspberry-supervisor/src/autodev.rs` with a typed
failure taxonomy. At minimum the system should distinguish between integration
conflict, deterministic verify cycle, provider-policy mismatch, direct proof
script failure, and generic environment collision. Each type should have an
explicit default action, such as replay source lane, refresh from trunk, back
off and retry later, or mark blocked with a precise reason. This taxonomy should
not live as ad hoc string matching in three different places. One shared
classification type should feed `autodev.rs`, `program_state.rs`, and
`evaluate.rs` so execution policy and operator-facing truth cannot drift apart.

This milestone now needs one additional step: blocked failures cannot terminate
at metadata. Add a framework-owned recovery-dispatch path so the supervisor can
turn selected blocked failures into actionable Fabro work. That path should be
responsible for authoring or replaying the smallest truthful remediation slice,
for example:

- proof-scope drift, where verify still targets the wrong package or wrong
  capability contract
- artifact-contract drift, where audit requires outputs the workflow never
  declared
- resource-lease conflicts that need a leased port, daemon URL, or similar
  runtime input rather than another repo-local patch loop
- idempotency gaps where bootstrap/pair/control scripts cannot be rerun
  truthfully inside one worktree

The important boundary is that repo-local recurring lanes may still report these
problems, but Fabro/Raspberry should own the decision to dispatch recovery work.
Otherwise every repo needs to reinvent a bespoke operations lane that only a
human can interpret.

### Milestone 4: Honest Quality and Verification

Strengthen the synthesis-generated quality and verify contracts in
`lib/crates/fabro-synthesis/src/render.rs` so the system does not reward empty
movement. The quality gate should keep inspecting the touched source tree for
placeholder debt and mismatches between artifacts and code. The verify contract
should bias toward runnable smoke behavior when the planned feature is
user-visible. The goal is that new frontier generation and honest settlement do
not drift apart.

This milestone should also absorb the currently observed proving-ground issue
families:

- capability-contract mismatch, where generated verify scripts ask an
  observe-only client to perform control actions
- artifact-contract mismatch, where audit expects `integration.md` or
  `merge_ready: yes` without the workflow declaring or owning that contract
- over-eager artifact heuristics, where legitimate slice notes like "future
  slice" are treated as hard artifact inconsistency instead of scoped work
  boundaries
- stale proof bundles, where generated verify commands still include packages or
  commands outside the active slice

## Concrete Steps

All commands below run from `/home/r/coding/fabro` unless a different working
directory is named explicitly.

Start by proving the current baseline:

    cargo test -p fabro-synthesis implementation_follow_on --lib
    cargo test -p raspberry-supervisor 'refresh_program_state_' --lib
    cargo test -p raspberry-supervisor replay_ --lib
    cargo build -p fabro-cli --target-dir /home/r/coding/fabro/target-local
    cargo build -p raspberry-cli --target-dir /home/r/coding/fabro/target-local
    git -C /home/r/coding/fabro diff --check

After the frontier-generation edits land, prove the result on Zend:

    /home/r/coding/fabro/target-local/debug/fabro --no-upgrade-check synth evolve \
      --target-repo /home/r/coding/zend \
      --program zend

    /home/r/coding/fabro/target-local/debug/raspberry status \
      --manifest /home/r/coding/zend/fabro/programs/zend.yaml \
      --fabro-bin /home/r/coding/fabro/target-local/debug/fabro

Expected observation: when the current rendered frontier is exhausted but the
repo doctrine still implies unfinished work, Zend reports fresh ready or
running child programs instead of returning to a false settled state.

Run the same evolve command a second time without changing doctrine:

    /home/r/coding/fabro/target-local/debug/fabro --no-upgrade-check synth evolve \
      --target-repo /home/r/coding/zend \
      --program zend

Expected observation: no duplicate child programs are created, and the rendered
frontier remains structurally stable under repeated evolve passes.

After the scheduler changes land, prove spare-capacity evolve:

    /home/r/coding/fabro/target-local/debug/raspberry autodev \
      --manifest /home/r/coding/zend/fabro/programs/zend.yaml \
      --fabro-bin /home/r/coding/fabro/target-local/debug/fabro \
      --poll-interval-seconds 5 \
      --evolve-every-seconds 0 \
      --max-cycles 3

Expected observation: if fewer than five Zend workers are active and doctrine
still implies more work, the bounded autodev run evolves and dispatches more
work before exiting, but it does not create a ready backlog larger than the
configured frontier budget.

Add targeted regression coverage for the new loop behavior:

    cargo test -p fabro-synthesis evolve_ --lib
    cargo test -p raspberry-supervisor doctrine_ --lib
    cargo test -p raspberry-supervisor frontier_ --lib
    cargo test -p raspberry-supervisor replay_ --lib

Expected observation: repeated evolve identity, per-program doctrine state,
frontier-budget behavior, and typed recovery all have direct regression tests.

After the runtime-truth and recovery changes land, inspect the proving-ground
state files directly:

    sed -n '1,220p' /home/r/coding/zend/.raspberry/zend-state.json
    sed -n '1,220p' /home/r/coding/myosu/.worktrees/autodev-live/.raspberry/myosu-state.json

Expected observation: state entries show canonical paths, fresh current or last
run ids, and typed blocker information without stale "running plus old failure"
mixtures.

Finally, rerun bounded work on Myosu:

    /home/r/coding/fabro/target-local/debug/raspberry autodev \
      --manifest /home/r/coding/myosu/.worktrees/autodev-live/fabro/programs/myosu.yaml \
      --fabro-bin /home/r/coding/fabro/target-local/debug/fabro \
      --poll-interval-seconds 5 \
      --evolve-every-seconds 0 \
      --max-cycles 3

Expected observation: failed Myosu fronts now route to typed recovery outcomes
instead of remaining generic failed records that require operator archaeology.

## Validation and Acceptance

This plan is complete when all of the following behaviors are observable.

Zend must never return to the old state where bootstrap is complete and the
parent program simply stops despite unfinished product scope. `synth evolve`
plus `raspberry status` must either produce more work or explain, in typed
terms, why the next frontier cannot yet be generated.

Repeated evolve passes over unchanged doctrine must be idempotent. Running the
same evolve command twice in a row must not create duplicate child programs or
rename the same frontier unnecessarily.

Autodev must use spare capacity intelligently. With `max_parallel=5`, an empty
worker slot in a repo with unfinished doctrine must trigger evolve, replay, or
dispatch instead of sleep. At the same time, the ready backlog must remain
bounded by the explicit frontier budget so the parent program stays legible.

Runtime truth must be trustworthy. The `.raspberry/` state files for Zend and
Myosu must be understandable on their own, with canonical paths, current or
last run ids, and blocker state that matches the underlying runs.

Recovery must be more specific than "failed." Myosu and Zend failure records
must distinguish between failure families and route to an explicit next action
instead of leaving the operator to infer intent from raw stderr.

Shared classification must remain coherent across execution and presentation. A
failure type recognized by autodev must be the same type that shows up in the
runtime truth surfaces, not a second hand-written label.

Doctrine-delta state must be safe for multi-program repos. A fingerprint update
for one manifest or program must not spuriously trigger evolve churn for a
different program in the same repository.

## Idempotence and Recovery

Every step in this plan should be safely repeatable. `fabro synth evolve`
should remain deterministic and additive with respect to the truthful frontier.
If an evolve pass emits obviously bad new work, the operator should be able to
inspect the rendered `fabro/programs/` output, fix the authoring or render
logic, and rerun the same command without destructive repo surgery.

Bounded `autodev` runs are the preferred proving method while this work lands.
If a live proving-ground run gets into a bad state, recovery should be limited
to stopping that bounded loop, cleaning the specific generated state or run that
is bad, and rerunning the bounded command. This plan does not permit hard
resets.

## Artifacts and Notes

The current Zend state proves the need for continuous frontier generation and
typed recovery:

    command-center-client-implementation:program -> verify/fixup loop on capability-contract drift
    hermes-adapter-implementation:program -> settle/audit contract drift around integration artifacts
    home-miner-service-implementation:program -> replayed after quality/artifact mismatch and now re-running
    private-control-plane-implementation:program -> failed after verify/fixup could not converge truthfully

The current Myosu state proves the need for canonical truth and typed replay:

    games-multi-game-implementation:program -> failed after deterministic cycle detection
    games-poker-engine-implementation:program -> failed after slice progress and quality/audit drift
    play-tui-implementation:program -> complete

These are the concrete proving-ground inputs for this plan, not hypothetical
edge cases.

### Current Live Issues (2026-03-20)

Zend is no longer idling at bootstrap, but the implementation wave is still
exposing framework-level recovery gaps:

- `command-center-client-implementation:program`
  - currently back in `fixup` after a deterministic verify failure
  - practical issue: the latest failure is a capability-contract mismatch, not
    a port collision
  - reproduced evidence: `bootstrap_home_miner.sh` pairs `alice-phone` with
    `observe`, then verify runs `set_mining_mode.sh --client alice-phone`, which
    fails with `GatewayUnauthorized`
- `hermes-adapter-implementation:program`
  - currently in `settle`
  - practical issue: the lane already proved a workflow-contract drift where
    audit required `integration.md` even though settle did not declare it
- `home-miner-service-implementation:program`
  - auto-replayed and is currently back in `implement`
  - practical issue: the last failed run first hit environment-collision noise,
    then later failed its quality gate because the artifact heuristic treated
    "future slice" wording as a hard inconsistency
  - adjacent issue: the generated workflow still contains a malformed verify
    command sequence, so contract drift remains likely even after the current
    rerun
- `private-control-plane-implementation:program`
  - currently failed
  - practical issue: repeated verify/fixup passes hit daemon lifecycle and proof
    drift, but the final state still collapsed to `surface_blocked`, so no
    framework-owned recovery work was authored

Myosu is also settled-failed rather than actively running:

- `bootstrap:program`
  - failed with a cycle-collapse summary because child implementation programs
    failed and the parent orchestration loop exhausted itself
- `games-multi-game-implementation:program`
  - classified as `deterministic_verify_cycle`
  - current action: `surface_blocked`
  - practical issue: the implementation loop is repeating a deterministic
    verify/fixup pattern because the generated verify contract drifted beyond
    the active slice and still referenced packages such as `myosu-play`
- `games-poker-engine-implementation:program`
  - currently failed under the same bootstrap frontier
  - practical issue: direct package tests pass, but quality/audit semantics are
    still mismatched with slice-scoped progress, so the lane collapses instead
    of settling truthfully

Current control-plane diagnosis:

- repo-local recurring lanes are not the missing automation
- the real missing automation is a Fabro/Raspberry recovery-dispatch path for
  blocked failures that are currently classified, surfaced, and then abandoned

These repo-specific failures should be treated as the next proving-ground
remediation queue once the control-plane work is sufficiently stable.

## Interfaces and Dependencies

In `lib/crates/fabro-synthesis/src/planning.rs`, keep
`author_blueprint_for_evolve(...)` as the single authorship entry point, but
extend it so it emits structured doctrine-derived frontier candidates rather
than mostly importing the existing package. Those candidates must carry a
deterministic identity derived from stable doctrine inputs so repeated evolve
passes stay idempotent.

In `lib/crates/fabro-synthesis/src/render.rs`, preserve
`reconcile_blueprint(req: ReconcileRequest<'_>) -> Result<ReconcileReport, RenderError>`
as the outer render contract while deepening its ability to add follow-on child
programs and stronger quality contracts.

In `lib/crates/raspberry-supervisor/src/autodev.rs`, keep the current
bounded-cycle orchestrator and extend it with spare-capacity evolve,
doctrine-delta triggering, typed recovery, and an explicit frontier-budget rule
that defaults to `max_parallel + 2`.

In `lib/crates/raspberry-supervisor/src/program_state.rs` and
`lib/crates/raspberry-supervisor/src/evaluate.rs`, keep the existing state-file
schema stable enough for current tools while making the surfaces cleaner and
more authoritative. The failure-classification type consumed here should be the
same one used by autodev policy.

Plan revised on 2026-03-20 after engineering review to reflect the agreed Phase
1B scope. Paperclip synchronization moved into a separate companion plan so
this file stays focused on continuous frontier generation, truthful state, and
typed recovery. It was revised again on the same day to incorporate the review
recommendations for bounded backlog growth, deterministic candidate identity,
and per-program doctrine-delta state.
