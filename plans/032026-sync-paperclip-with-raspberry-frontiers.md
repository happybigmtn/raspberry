# Sync Paperclip with Raspberry Frontiers

This ExecPlan is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

`PLANS.md` is checked into the repository root and this document must be
maintained in accordance with it. This plan depends on
`plans/031926-harden-autonomy-and-direct-trunk-integration.md` and
`plans/032026-keep-fabro-and-raspberry-continuously-generating-work.md`.

## Purpose / Big Picture

After this slice lands, Paperclip should stop being a one-time bootstrap helper
and start acting as a truthful coordination overlay on top of Raspberry. A
human or agent looking at the Paperclip board should see the same active
frontier, blocker state, and ownership boundaries that Raspberry already knows,
without Paperclip trying to become a second scheduler.

The user-visible result is that repo-local Paperclip companies become useful
for delegation, review, escalation, and heartbeat coordination while Raspberry
continues to own execution. Bootstrap should create the initial company cleanly,
and refresh should keep that company aligned to the current program frontier.

## Progress

- [x] (2026-03-20 16:10Z) Re-read the existing Paperclip bootstrap code and the
  earlier autonomy plan that first introduced it.
- [x] (2026-03-20 16:42Z) Split Paperclip work out of the no-idle autonomy plan
  so it can be designed as its own control-plane overlay instead of being mixed
  into the scheduler slice.
- [x] (2026-03-20 17:18Z) Define a stable frontier-export shape from Raspberry
  runtime state into a Paperclip synchronization model.
- [x] (2026-03-20 17:18Z) Extend `paperclip bootstrap` with a refresh path
  that keeps goals, project workspace metadata, and generated agents aligned
  to the current frontier.
- [x] (2026-03-20 17:18Z) Add synchronization for coordination issues or task
  families per active program unit or child program.
- [x] (2026-03-20 17:18Z) Ensure generated Paperclip agents route execution
  back through Raspberry instead of bypassing it.
- [x] (2026-03-20 17:18Z) Prove the flow on a repo-local Zend Paperclip
  instance.

## What Already Exists

Several pieces already exist and should be extended rather than replaced.

- `fabro paperclip bootstrap` already exists in
  `lib/crates/fabro-cli/src/commands/paperclip.rs`.
- The bootstrap path already knows how to create or load a blueprint, render a
  package, build a company bundle, import that bundle into Paperclip, create a
  goal, create a project, create a workspace, and install local CLI env files
  for generated agents.
- `paperclip.rs` already generates a CEO, an orchestrator, and unit-aligned
  agent markdowns from the current blueprint.
- Raspberry already has the authoritative frontier state in `.raspberry/*` and
  the evaluation logic in `program_state.rs`, `evaluate.rs`, and `autodev.rs`.

## NOT in Scope

This plan intentionally does not include:

- Replacing Raspberry with Paperclip for run dispatch or execution decisions.
- Re-implementing scheduler logic inside Paperclip agents.
- Redesigning the Paperclip bootstrap company schema from scratch.
- Solving the no-idle frontier-generation problem itself. That remains in
  `plans/032026-keep-fabro-and-raspberry-continuously-generating-work.md`.

## Surprises & Discoveries

- Observation: Paperclip is already good enough to be useful locally.
  Evidence: the existing bootstrap path already provisions company, goal,
  workspace, and agent CLI envs rooted at `.paperclip/`.

- Observation: the missing piece is synchronization, not bootstrap.
  Evidence: once the company exists, there is no always-on path that refreshes
  issue ownership or blocker state from the live Raspberry frontier.

- Observation: the biggest design risk is scheduler duplication.
  Evidence: both systems can talk about work, but only Raspberry currently owns
  manifest evaluation and run dispatch. If Paperclip starts making execution
  decisions independently, the two surfaces will drift.

- Observation: generated agent markdown already captures the right topology.
  Evidence: `paperclip.rs` derives unit-aligned agents from the blueprint, so
  the missing step is keeping those agents aligned to the live frontier rather
  than inventing a new agent taxonomy.

- Observation: repo-local Paperclip boot now needs a seeded config before
  `paperclipai run` can start non-interactively.
  Evidence: the first Zend proof failed with `No config found and terminal is
  non-interactive`, so bootstrap had to seed the same quickstart-style config,
  env secret, and local master key that onboarding normally writes.

- Observation: saved Paperclip ids are only hints, not durable truth, across
  fresh local homes.
  Evidence: after the new local Paperclip instance came up, the old
  `bootstrap-state.json` company id pointed at a company that no longer
  existed, so refresh needed to validate the id against the live instance and
  fall back to name matching.

- Observation: the local Paperclip CLI wrapper is not the right durable
  background process for repo-local bootstrap.
  Evidence: the API stayed alive during import but died after bootstrap
  returned until the start path was changed to launch the TSX-backed server
  entrypoint directly and record the real server pid.

## Decision Log

- Decision: Paperclip will be an overlay, not an alternative control plane.
  Rationale: Raspberry already owns execution truth. A second scheduler would
  increase ambiguity instead of autonomy.
  Date/Author: 2026-03-20 / User + Codex

- Decision: synchronization should start from Raspberry frontier state, not
  from raw repo doctrine or ad hoc Paperclip-side inference.
  Rationale: the most trustworthy source for "what is active, blocked, or done"
  is the supervisor state that already tracks runs.
  Date/Author: 2026-03-20 / Codex

- Decision: generated Paperclip agents should continue to route concrete work
  through Raspberry commands and scripts.
  Rationale: that preserves a single execution authority while still letting
  Paperclip handle delegation and governance.
  Date/Author: 2026-03-20 / Codex

## Outcomes & Retrospective

This plan exists because Paperclip was too important to delete but too broad to
keep inside the main no-idle autonomy slice. Splitting it out improved both
plans: the main loop can now focus on continuous frontier generation, and this
plan can focus on how humans and agents coordinate around that frontier without
creating a rival scheduler.

The finished slice now behaves like a real overlay instead of a one-shot
bootstrap. `fabro paperclip bootstrap` reads Raspberry frontier truth
read-only, refreshes goal narration and workspace metadata, regenerates agent
instructions with explicit Raspberry-only execution routes, synchronizes a
deterministic frontier issue family, and records the sync map in
`bootstrap-state.json`.

The Zend proof also shook out two recovery behaviors that are now part of the
implementation instead of tribal knowledge: bootstrap seeds a repo-local
Paperclip quickstart config when the target home has never been onboarded, and
saved company ids are revalidated against the current instance before import
tries to replace anything in place.

## Context and Orientation

The current Paperclip implementation lives in
`lib/crates/fabro-cli/src/commands/paperclip.rs`. It already performs a useful
bootstrap sequence:

    blueprint or rendered package
        |
        v
    company bundle under fabro/paperclip/<program>/
        |
        v
    repo-local Paperclip company import
        |
        v
    goal + project + workspace + local agent envs

The missing layer is live alignment:

    Raspberry program frontier
        |
        v
    sync model
        |
        v
    Paperclip goals / issues / agent scopes
        |
        v
    heartbeat coordination that still calls Raspberry for execution

The important code boundaries are:

- `lib/crates/fabro-cli/src/commands/paperclip.rs` for bundle generation,
  import, refresh, and local agent setup.
- `lib/crates/raspberry-supervisor/src/program_state.rs` and
  `lib/crates/raspberry-supervisor/src/evaluate.rs` for authoritative frontier
  state.
- `lib/crates/raspberry-supervisor/src/autodev.rs` for the scheduler actions
  that Paperclip should reference rather than replace.

Zend is the best proving ground for this plan because it already has a real
frontier, a generated package, and live implementation child programs.

## Plan of Work

### Milestone 1: Define the Sync Model

Add a small, explicit synchronization model in `paperclip.rs` that translates
the Raspberry frontier into the minimum coordination information Paperclip
needs. The model should include program or unit identity, current status,
current or last run id, typed blocker reason when available, and the
repo-relative command or script that routes execution back through Raspberry.

This model should come from Raspberry state files and evaluated program output,
not from freehand bundle generation. The goal is that a refresh run can compare
the live frontier with the previously imported company state and decide what to
create, update, or archive in Paperclip.

### Milestone 2: Refresh Goals, Projects, and Agents

Extend `paperclip bootstrap` so it can act as both initial bootstrap and safe
refresh. The existing goal, project, workspace, and generated agents should be
updated in place when the program frontier changes. The generated markdown for
agents should remain aligned to the actual frontier and should preserve the
rule that execution routes through Raspberry rather than through direct repo
mutation outside the supervisor.

### Milestone 3: Synchronize Coordination Issues

Add issue or task-family synchronization for active units and child programs.
The refresh step should create or update one coordination issue family per
frontier element that a human or agent might need to reason about: active work,
blocked work, and newly ready work. These issues should carry enough context to
be useful, but they should not become the source of truth for execution state.
Each synchronized issue family needs a deterministic sync key derived from the
stable frontier identity, such as program id plus unit or lane key, rather than
from mutable titles. Refresh runs should use that key to update existing issues
in place instead of duplicating them.

The intended flow is:

    Raspberry says "this unit is running/failed/ready"
        |
        v
    Paperclip issue family mirrors that state
        |
        v
    Paperclip agents comment, triage, review, escalate
        |
        v
    actual execution still happens through Raspberry

### Milestone 4: Heartbeat Behavior that Honors Raspberry

Adjust generated Paperclip agent instructions and helper scripts so heartbeat
agents do not bypass the supervisor. Their allowed actions should be things such
as inspecting the current frontier, asking Raspberry to evaluate or refresh,
posting review or blocker notes, and escalating issues. They should not silently
create parallel execution flows that mutate repo state independently of
Raspberry.

## Concrete Steps

All commands below run from `/home/r/coding/fabro` unless stated otherwise.

Start by proving the current bootstrap still works mechanically:

    cargo build -p fabro-cli --target-dir /home/r/coding/fabro/target-local

    /home/r/coding/fabro/target-local/debug/fabro paperclip bootstrap \
      --target-repo /home/r/coding/zend \
      --program zend \
      --apply false

Expected observation: the command writes a deterministic bundle under
`/home/r/coding/zend/fabro/paperclip/zend/` without mutating a live Paperclip
instance.

After the sync-model work lands, rerun with apply enabled against the local
Paperclip instance:

    /home/r/coding/fabro/target-local/debug/fabro paperclip bootstrap \
      --target-repo /home/r/coding/zend \
      --program zend \
      --apply true

Expected observation: the repo-local company, goal, project, workspace, and
generated agents are either created or refreshed in place rather than duplicated.

After coordination-issue sync lands, rerun the same command after changing the
Zend frontier, then inspect the generated state and bootstrap-state metadata:

    sed -n '1,220p' /home/r/coding/zend/fabro/paperclip/zend/bootstrap-state.json
    sed -n '1,220p' /home/r/coding/zend/.raspberry/zend-state.json

Expected observation: the imported Paperclip entities now reflect the same live
frontier that Raspberry reports.

Run the refresh a second time without changing the frontier:

    /home/r/coding/fabro/target-local/debug/fabro paperclip bootstrap \
      --target-repo /home/r/coding/zend \
      --program zend \
      --apply true

Expected observation: no duplicate coordination issues are created because the
sync layer reuses deterministic frontier keys.

## Validation and Acceptance

This plan is complete when all of the following behaviors are observable.

Running `fabro paperclip bootstrap --apply true` twice against the same repo
must refresh the existing Paperclip company state instead of duplicating goals,
projects, generated agents, or synchronized issue families.

The generated Paperclip entities must mirror Raspberry frontier truth. If Zend
has active implementation child programs, the Paperclip coordination layer must
show that active frontier and its blocker state rather than stale bootstrap-only
state.

Generated Paperclip agents must continue to route execution through Raspberry.
Their markdown and helper scripts should make it obvious that Raspberry is the
execution authority and Paperclip is the coordination layer.

## Idempotence and Recovery

This plan must remain safe for repeated local bootstrap and refresh runs.
Existing company state should be updated in place when possible. If refresh
fails partway through, the operator should be able to rerun the same bootstrap
command after fixing the immediate problem without deleting the entire local
Paperclip instance.

The repo-local `.paperclip/` directory should remain the default isolated data
root for proving this work so mistakes do not spill into unrelated instances.

## Artifacts and Notes

The existing bootstrap path already writes the important proving artifacts:

    fabro/paperclip/<program>/paperclip.manifest.json
    fabro/paperclip/<program>/bootstrap-state.json
    fabro/paperclip/<program>/scripts/run-paperclip.sh
    fabro/paperclip/<program>/scripts/raspberry-orchestrator.sh

Those files are the right place to prove that refresh semantics and generated
agent instructions are aligned to Raspberry.

## Interfaces and Dependencies

In `lib/crates/fabro-cli/src/commands/paperclip.rs`, preserve the existing
bootstrap entry point and extend it with a synchronization model plus safe
refresh behavior for companies, goals, projects, workspaces, generated agents,
and coordination issues. Coordination issue refresh must use deterministic sync
keys derived from Raspberry frontier identity instead of title matching.

The synchronization logic should consume Raspberry frontier truth from the
existing `.raspberry/` surfaces rather than creating a separate state source.

Plan created on 2026-03-20 as the companion to the no-idle autonomy plan. This
split keeps execution-authority work in one file and coordination-overlay work
in another so both can be implemented cleanly.
