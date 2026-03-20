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
- [ ] Deepen `author_blueprint_for_evolve` so doctrine files author missing
  frontier work instead of only reconciling the current package.
- [ ] Keep parent programs as living roadmaps that continue to emit follow-on
  child programs after each settled frontier.
- [ ] Let autodev use spare worker capacity to trigger evolve and dispatch more
  work instead of waiting for full local settlement.
- [ ] Persist doctrine-delta state under `.raspberry/` so plan or spec changes
  force fresh frontier generation.
- [ ] Make runtime truth canonical and child-authoritative so parent state is
  directly actionable.
- [ ] Add typed recovery policies that route deterministic failures to the
  correct replay behavior.
- [ ] Strengthen quality and verification contracts enough that "new work
  generated" and "work honestly settled" stay aligned.
- [ ] Prove the result on Zend and Myosu with bounded five-worker runs.

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
- Host-level resource leasing such as port reservation for proof daemons. The
  Zend `Errno 98` failure is real, but it should land after the no-idle core is
  working.
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

## Outcomes & Retrospective

This plan now has a cleaner shape than the first draft. The earlier version was
useful as a landscape map, but it mixed the immediate execution loop with later
control-plane coordination and environment hardening. The revised version keeps
the core promise intact: the system should continuously derive and run the next
truthful work frontier, surface honest state, and route failures to the correct
next action.

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

### Milestone 4: Honest Quality and Verification

Strengthen the synthesis-generated quality and verify contracts in
`lib/crates/fabro-synthesis/src/render.rs` so the system does not reward empty
movement. The quality gate should keep inspecting the touched source tree for
placeholder debt and mismatches between artifacts and code. The verify contract
should bias toward runnable smoke behavior when the planned feature is
user-visible. The goal is that new frontier generation and honest settlement do
not drift apart.

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

    command-center-client-implementation:program -> failed at Verify
    hermes-adapter-implementation:program -> failed during direct integration
    home-miner-service-implementation:program -> failed with Errno 98
    private-control-plane-implementation:program -> running

The current Myosu state proves the need for canonical truth and typed replay:

    games-multi-game-implementation:program -> failed after deterministic cycle detection
    games-poker-engine-implementation:program -> failed at Verify
    play-tui-implementation:program -> complete

These are the concrete proving-ground inputs for this plan, not hypothetical
edge cases.

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
