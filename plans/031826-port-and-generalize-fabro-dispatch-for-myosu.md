# Port and Generalize Fabro Dispatch for Myosu-Scale Programs

This ExecPlan is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

`PLANS.md` is checked into the repository root and this document must be
maintained in accordance with it. This plan depends on
`specs/031826-raspberry-malinka-control-plane-port.md`.

## Purpose / Big Picture

After this slice lands, Raspberry supervision will no longer be a bootstrap
approximation inspired by old Malinka behavior. It will instead be a direct,
reduced port of the proven `fabro_dispatch` semantics, generalized so it can
supervise a broad operational repository like Myosu rather than only
documentation-shaped programs.

The user-visible outcome is that Raspberry gains a program manifest and runtime
model that can handle larger scopes: code-heavy units, operational units,
service-like units, and doctrine or recurring control-plane work in the same
repo. The proof is not just a prettier `plan` command. The proof is that a
Myosu-shaped fixture or proving-ground manifest can be classified and
dispatched sensibly.

## Progress

- [x] (2026-03-18 23:35Z) Drafted this follow-on ExecPlan for the
  `fabro_dispatch` reduction port.
- [x] (2026-03-18 23:54Z) Read the old `fabro_dispatch` implementation and
  identified the first port target: manifest normalization and dependency
  semantics.
- [x] (2026-03-19 00:02Z) Replaced the bootstrap manifest loader with a
  generalized loader that supports both the bootstrap map-shaped schema and a
  richer list-shaped schema closer to the old bridge.
- [x] (2026-03-19 00:07Z) Ported generalized dependency semantics so a lane can
  depend on a unit milestone or a specific lane milestone.
- [x] (2026-03-19 00:10Z) Added a Myosu-shaped proving-ground fixture with
  chain, validator, miner, operations, and gameplay-style units.
- [x] (2026-03-19 00:12Z) Proved the generalized manifest through supervisor
  tests and a direct `raspberry plan` run on the Myosu fixture.
- [x] (2026-03-19 00:40Z) Reviewed the full Myosu spec corpus and extracted
  the broader workload classes and control-plane requirements it imposes.
- [x] (2026-03-19 00:55Z) Ported richer lane runtime records and live lane
  refresh behavior from the old bridge into `raspberry-supervisor`.
- [x] (2026-03-19 00:57Z) Proved richer `status` output on both the bootstrap
  and Myosu-shaped fixtures, including Fabro run ids, current stages, last
  completed stages, usage summaries, and files read/written.
- [x] (2026-03-19 01:08Z) Added lane kinds, proof profiles, and scoped checks
  so the Myosu fixture can model service health and proof/precondition
  semantics explicitly.
- [x] (2026-03-19 01:18Z) Added command-backed checks so lane readiness and
  health can derive from command exit status or stdout, not only fixture files.
- [x] (2026-03-19 01:24Z) Added first-class operational state for running
  service lanes, so the supervisor can surface `healthy` vs `degraded` instead
  of only dumping raw check lists.
- [x] (2026-03-19 01:31Z) Added a small derived taxonomy over checks so the
  supervisor distinguishes preconditions, proof state, and operational health.
- [x] (2026-03-19 01:42Z) Promoted proof and service semantics into explicit
  lane-level state sources, with checks now acting as fallback rather than the
  only source of truth.
- [x] (2026-03-19 02:02Z) Added an explicit orchestration-state contract and a
  dedicated launch/devnet-style lane to the Myosu fixture.
- [x] (2026-03-19 13:10Z) Added a stable Fabro run-inspection adapter in
  `fabro-workflows`, switched `fabro inspect` to use it, and removed
  Raspberry's `run.toml`-matching heuristic over `~/.fabro/runs/`.
- [x] (2026-03-19 13:10Z) Switched Raspberry execute to detached Fabro runs so
  lanes persist authoritative Fabro run ids immediately instead of backfilling
  them later from raw run discovery.
- [x] (2026-03-19 13:10Z) Replaced the remaining bootstrap `status`/`watch`
  behavior with run-id-based refresh semantics, then proved the flow through
  supervisor, CLI, and inspect-command tests.
- [ ] Prove one true Myosu lane end to end through detached Fabro dispatch,
  authoritative run-id refresh, and correct Raspberry state transitions.

## Surprises & Discoveries

- Observation: the bootstrap Raspberry supervisor proved the crate boundary,
  but it intentionally simplified the program model.
  Evidence: the first slice kept a small manifest and fixture set so the
  control-plane shell could be proven before porting the richer semantics.

- Observation: the old `fabro_dispatch` implementation already solved the hard
  supervisory questions that matter most here: lane readiness, blocked vs
  complete vs in-progress, program runtime state, watch semantics, and bounded
  execute.
  Evidence: the old implementation carries explicit `ProgramManifest`,
  `ProgramState`, lane runtime records, and `plan/status/watch/execute`
  behavior.

- Observation: Myosu is a much better proving ground than a book workflow.
  Evidence: its operating system describes a repo with chain, miners,
  validators, gameplay, doctrine, risk, evidence, and scorecard surfaces. That
  is broad enough to expose any supervision model that accidentally assumes
  “unit = chapter”.

- Observation: the first concrete generalization pressure came from dependency
  semantics, not command rendering.
  Evidence: the old bridge supported unit-scoped milestone dependencies,
  whereas the bootstrap manifest only supported lane-scoped dependencies. A
  Myosu-shaped repo naturally wants both kinds.

- Observation: supporting both map-shaped and list-shaped manifest forms lets
  Raspberry keep the working bootstrap fixture while porting the richer bridge
  semantics.
  Evidence: the bootstrap fixture still loads, and the new Myosu fixture also
  loads and evaluates in the same crate.

- Observation: Myosu’s spec corpus requires the supervisor to reason about
  multiple fundamentally different workload classes, not one generic “run a
  lane and wait for artifacts” path.
  Evidence: across the full specs, Myosu includes chain/runtime work, miner and
  validator daemons, gameplay and TUI work, devnet and launch orchestration,
  platform/SDK scaffolding, operational RPCs, incentive-layer design, and
  recurring doctrine/security/operations/learning work.

- Observation: service health and environmental preconditions are first-class
  state in Myosu, not just post-run validation.
  Evidence: miner, validator, launch, and operational specs repeatedly depend
  on facts like “chain reachable”, “subnet exists”, “axon advertised”,
  “health endpoint responds”, and “abstractions downloaded” before a unit can
  honestly be called ready or successful.

- Observation: Myosu mixes cargo-workspace build work with long-lived runtime
  services and recurring control-plane work in one repo.
  Evidence: `project.yaml` plus the spec corpus describe code-generation
  tasks, long builds, service startup, integration tests, TUI/agent experience,
  operational dashboards/RPCs, and recurring strategy/security/operations/
  learning lanes.

- Observation: the Myosu spec tree contains both zero-padded and non-zero-padded
  filenames for overlapping topics.
  Evidence: the directory contains pairs like `031626-04a-miner-binary.md` and
  `031626-miner-binary.md`. This matters because Raspberry should not assume one
  external naming convention when it ingests repo-local program metadata later.

- Observation: the richer lane runtime records immediately make the supervisor
  output feel more like the proven bridge and less like a toy manifest reader.
  Evidence: `raspberry status` now shows Fabro run ids, current stage labels,
  last completed stages, stage duration, usage, files read, files written, and
  failure summaries for both the bootstrap fixture and the Myosu-shaped
  fixture.

- Observation: lane kinds, proof profiles, and scoped checks are enough to
  start expressing Myosu’s broader operational semantics without exploding the
  manifest model yet.
  Evidence: the Myosu fixture now distinguishes platform, service,
  orchestration, and interface lanes, and it models readiness checks and
  running health checks through explicit check records.

- Observation: command-backed checks fit naturally into the same scoped-check
  model as file and JSON probes.
  Evidence: the Myosu fixture now uses `command_succeeds` for `chain_ready` and
  `command_stdout_contains` for `validator_proof_passed`, while the rest of the
  evaluator logic stayed stable.

- Observation: adding operational state on top of the check model is a low-risk
  way to move from “raw facts” toward “real control-plane semantics.”
  Evidence: the running miner service lane now renders as operationally
  `healthy` while still preserving the underlying running check list.

- Observation: a small taxonomy over checks is enough to separate “waiting on
  preconditions” from “waiting on proof” without introducing a full-blown new
  orchestration engine.
  Evidence: the Myosu fixture can now distinguish `preconditions=met`,
  `proof_state=failed`, and `operational=healthy` in the rendered status.

- Observation: explicit lane-level proof and service sources fit cleanly on top
  of the same probe vocabulary as checks.
  Evidence: the Myosu fixture now uses `proof_source` for
  `operations:scorecard` and `service_source` for `miner:service`, while
  categorized checks remain available as fallback and supporting detail.

- Observation: launch-style work benefits from its own orchestration-state
  contract rather than being forced into proof or service semantics.
  Evidence: the Myosu fixture now includes `launch:devnet` with
  `orchestration=waiting`, which is more truthful than calling it a generic
  blocked lane with no explicit launch state.

- Observation: the biggest remaining gap after the semantic port was not the
  manifest model, but Raspberry's way of discovering Fabro runs.
  Evidence: the supervisor had enough lane kinds, checks, and state contracts
  to model Myosu-shaped work, but it still matched `run.toml` contents across
  `~/.fabro/runs/` instead of following a stable run-id-based inspection
  contract.

- Observation: Fabro's existing detached run mode is already the right submit
  contract for Raspberry.
  Evidence: `fabro run --detach ...` prints a clean run id immediately and
  creates a submitted run record before the background worker starts, so
  Raspberry can persist the authoritative run id without inventing a second
  queueing or submission protocol.

## Decision Log

- Decision: the next slice should treat the old `fabro_dispatch` implementation
  as the primary semantic reference rather than continuing a greenfield
  bootstrap rewrite.
  Rationale: the bootstrap slice proved the crate boundaries and test harness.
  The proven supervisory behavior already exists; the next efficient move is to
  port and reduce it.
  Date/Author: 2026-03-18 / Codex

- Decision: Myosu should be the first non-book proving ground for the port.
  Rationale: the goal is repo-agnostic supervision. Myosu’s `OS.md` describes a
  much broader operational environment than a documentation workflow, so it is
  a stronger test of generality.
  Date/Author: 2026-03-18 / Codex

- Decision: the port should preserve behavior, not structure.
  Rationale: `fabro_dispatch.rs` is valuable because of its semantics, but it is
  entangled with old Malinka-specific state and CLI conventions. The port
  should keep what works and delete baggage.
  Date/Author: 2026-03-18 / Codex

- Decision: start the real port with manifest normalization and dependency
  semantics rather than with watch-loop code.
  Rationale: the first hard generalization boundary is the program model. If
  the manifest still assumes a book workflow, richer watch or execute behavior
  will only reinforce the wrong abstraction.
  Date/Author: 2026-03-19 / Codex

- Decision: keep both the bootstrap map-shaped manifest and the richer
  list-shaped manifest valid during the transition.
  Rationale: this keeps the already-proven bootstrap fixture as a safety net
  while enabling a more faithful port of the old bridge semantics.
  Date/Author: 2026-03-19 / Codex

- Decision: the next generalization target after manifest shape is lane state
  kind, not renderer polish.
  Rationale: after the full Myosu review, the biggest remaining gap is that
  Raspberry still assumes lane success is mostly an artifact-completion story.
  Myosu requires service-health, proof-profile, orchestration, and recurring
  control-plane semantics.
  Date/Author: 2026-03-19 / Codex

- Decision: use the full Myosu corpus, not just `OS.md`, as the requirement
  baseline for generalization.
  Rationale: `OS.md` captures scope, but the individual specs are what reveal
  the real operational diversity: daemons, integration harnesses, SDK-style
  platform work, RPC surfaces, and recurring compiler-owned lanes.
  Date/Author: 2026-03-19 / Codex

- Decision: port program-state and live-refresh semantics before introducing
  service-health or proof-profile semantics.
  Rationale: the old bridge’s runtime-state model is the proven base. Porting
  it first gives Raspberry a stable operator surface before we add new state
  kinds for Myosu-specific service and orchestration concerns.
  Date/Author: 2026-03-19 / Codex

- Decision: implement the first Myosu-scale semantics using scoped checks
  before inventing a larger state machine.
  Rationale: scoped checks (`ready` vs `running`) and a small lane-kind enum let
  Raspberry express chain/bootstrap readiness, service health, and proof-like
  prerequisites with low structural risk while we continue porting old bridge
  behavior.
  Date/Author: 2026-03-19 / Codex

- Decision: add command-backed probes before inventing a bespoke proof or
  health subsystem.
  Rationale: command output is already a real part of how Myosu proves work and
  checks readiness. Supporting command-based probes now lets Raspberry model
  that reality directly while keeping the manifest surface small.
  Date/Author: 2026-03-19 / Codex

- Decision: make operational state an interpretation layer over checks instead
  of a separate independent input surface.
  Rationale: the current state of the port is still intentionally small.
  Deriving `healthy` / `degraded` from scoped service checks gives Raspberry a
  more truthful operator surface without prematurely freezing a larger service
  state machine.
  Date/Author: 2026-03-19 / Codex

- Decision: derive `preconditions` and `proof_state` from categorized checks
  before introducing a dedicated orchestration-state subsystem.
  Rationale: Myosu needs those distinctions immediately, but the smallest
  truthful implementation is to derive them from explicit check kinds rather
  than invent a parallel state source too early.
  Date/Author: 2026-03-19 / Codex

- Decision: promote proof and service semantics to explicit lane-level sources
  before building any larger orchestration framework.
  Rationale: this keeps the state model compact while making room for
  more authoritative sources of truth than generic checks alone. Checks now
  support the control plane, but do not have to be the only place meaning
  comes from.
  Date/Author: 2026-03-19 / Codex

- Decision: add orchestration state as its own explicit contract before
  designing a larger orchestration engine.
  Rationale: Myosu’s launch/devnet work clearly needs a way to say “waiting on
  environment” that is neither proof failure nor service degradation. A small
  `orchestration_state_path` gives Raspberry that meaning without overshooting
  into a heavy workflow scheduler design.
  Date/Author: 2026-03-19 / Codex

- Decision: the next Fabro-core change should be a stable inspect-by-run-id
  adapter, and Raspberry should consume that instead of scraping raw run
  directories.
  Rationale: Fabro already documents `inspect` as the stable machine surface.
  Moving the adapter into Fabro's library layer makes the run-truth contract
  upstreamable while removing the most brittle part of the old Raspberry port.
  Date/Author: 2026-03-19 / User + Codex

- Decision: Raspberry execute should submit detached Fabro runs and persist the
  returned run id immediately.
  Rationale: this makes the Fabro run id the primary control-plane link to
  execution truth, simplifies refresh logic, and sets up status/watch for a
  real Myosu proving flow rather than another bootstrap-only code path.
  Date/Author: 2026-03-19 / User + Codex

## Outcomes & Retrospective

The first part of this port is now real. Raspberry no longer assumes that the
program manifest must be book-shaped. It can parse a richer bridge-like schema
and it can classify a Myosu-shaped program fixture meaningfully:

- `chain:runtime` completes
- `validator:oracle` completes with preconditions met
- `miner:service` is running
- `operations:scorecard` is blocked on proof state
- `play:tui` is failed

That is the first concrete proof that the supervisory model is moving away from
chapter/page orchestration and toward broader repo supervision.

The full Myosu spec review clarifies the next requirement: Raspberry’s control
plane must evolve beyond artifact milestones alone. To be truly general, it
must model service health, proof posture, orchestration preconditions, and
recurring control-plane work in the same supervisory program.

The good news is that the port now has a stronger base to build on. Raspberry
can already refresh live lane state from Fabro run truth and surface that in
`status`. That means the next semantics we add will land on a control plane
that already exposes meaningful runtime facts.

That next semantics slice has now started landing too: the Myosu fixture can
express lane kind, proof profile, readiness checks, and running health checks.
The control plane still needs richer real-world service and proof integration,
but it no longer assumes every lane is just an artifact pipeline.

It also now has a genuine bridge to repo behavior beyond static files: checks
can run shell commands in the target repo and decide readiness from either exit
status or stdout contents.

Running service lanes can now also surface an explicit operational state
derived from those checks, which is the first real move beyond artifact-centric
status.

The control plane also now distinguishes three different meanings that had been
collapsed before:

- `preconditions=met|failed`
- `proof_state=met|failed`
- `operational=healthy|degraded` for running service lanes

And launch-style orchestration lanes can now surface:

- `orchestration=ready|waiting|blocked`

And those states can now come from explicit lane-level contracts, not only
derived check categories:

- `proof_state_path`
- `service_state_path`
- `orchestration_state_path`

## Context and Orientation

The old Malinka `fabro_dispatch` subsystem did four important things that the
bootstrap Raspberry slice only approximated:

- it defined a richer program manifest with units, lanes, milestones,
  dependencies, and produced artifacts
- it persisted program runtime state and lane runtime records
- it exposed `plan`, `status`, `watch`, and bounded `execute`
- it refreshed lane state from live Fabro run truth while work was in flight

The bootstrap Raspberry slice already created the right host boundary in this
repo:

- `lib/crates/raspberry-supervisor/`
- `lib/crates/raspberry-cli/`
- `test/fixtures/raspberry-supervisor/`

What it did not yet do is port the richer supervision semantics.

Myosu is the proving-ground repo shape we should keep in mind while porting.
Its `OS.md` describes a single repo with multiple technical layers, operational
artifacts, doctrine, evidence, and long-lived subsystems. That means the new
program manifest cannot assume:

- one output root per unit is enough
- every unit is content
- milestones are only “chapter reviewed” style milestones
- the main consumer is a docs artifact

The full Myosu spec corpus sharpens that further. Raspberry supervision must be
able to represent:

- code-generation and compile/proof lanes
- long-running miner, validator, and node service lanes
- integration and launch orchestration lanes
- UI, TUI, spectator, and agent-experience lanes
- platform and SDK lanes
- recurring strategy, security, operations, and learning lanes

It also needs to reason about:

- artifact milestones
- proof milestones
- service-health milestones
- orchestration preconditions
- recurring compiler-owned outputs

The current bridge toward that goal is:

- lane kinds
- proof profiles
- scoped checks (`ready` vs `running`)
- check probe kinds:
  - file exists
  - JSON field equals
  - command succeeds
  - command stdout contains text
- derived control-plane state:
  - preconditions met / failed
  - proof_state met / failed
  - operational healthy / degraded for running service lanes
- explicit lane-level state sources:
  - `proof_source`
  - `service_source`

In this plan, a **program model** means the schema and evaluation logic that
describes what work exists, what depends on what, and what state each lane is
currently in. A **proving-ground fixture** means a repository-local test
fixture that is shaped like a broad repo, not a toy chapter workflow.

## Milestones

### Milestone 1: Inventory and reduction map

At the end of this milestone, the repository contains a written behavioral map
from the old `fabro_dispatch` subsystem to the current Raspberry crates. The
proof is that every important concept from the old bridge has an explicit
destination: keep, adapt, or delete.

### Milestone 2: Generalized manifest and state model

At the end of this milestone, `raspberry-supervisor` uses a generalized program
manifest and runtime state model derived from the proven bridge rather than the
bootstrap simplification. The proof is that existing bootstrap fixture tests
still pass and a Myosu-shaped fixture can also be parsed and classified.

### Milestone 3: Myosu-shaped proving ground

At the end of this milestone, the new supervisor can classify and render a
broader program manifest that resembles Myosu’s repo shape. The proof is that
tests show meaningful ready, blocked, running, complete, and failed states
across multiple kinds of units.

### Milestone 4: Add service, proof, and orchestration semantics

At the end of this milestone, the program model can express more than artifact
completion. It can represent service-health state, named proof profiles,
environment or orchestration preconditions, and at least one recurring-style
lane with compiler-owned output semantics. The proof is that a Myosu-shaped
fixture can model a service lane and an orchestration lane without lying about
their state.

### Milestone 5: Replace bootstrap internals

At the end of this milestone, the current Raspberry CLI commands are backed by
the ported-and-reduced supervision core rather than the first-principles
bootstrap implementation. The proof is that `plan`, `status`, `watch`, and
bounded `execute` still pass, and the bootstrap-only logic has been deleted or
minimized.

## Plan of Work

Start by reading the old `fabro_dispatch` behavior carefully and extracting the
actual supervisory primitives: manifest shapes, lane state classification,
program state persistence, lane-runtime refresh, and watch semantics. Do not
start by copying code. Start by naming the concepts and deciding where each one
belongs inside `raspberry-supervisor`.

Once the concept map exists, replace the bootstrap manifest model with a richer
generalized one. The generalized model must still support the current book-like
fixture, but it must also support a larger operational fixture modeled after
Myosu. In practice, that means the model should be able to describe units that
stand for code subsystems, operations lanes, and doctrine or recurring lanes,
not only content products.

After the generalized model is in place, port the richer runtime state ideas:
lane runtime records, program run state, and refresh-from-live-run-truth
behavior. Keep the data model small, but port the behavior faithfully.

Only after those behaviors exist should the bootstrap evaluator and renderers
be replaced. The goal is not “have more code from the old repo present.” The
goal is “the new supervisor behaves like the proven bridge, without depending
on old-Malinka-only assumptions.”

## Concrete Steps

Work from the repository root.

1. Port the generalized manifest semantics from the old bridge.
2. Keep both bootstrap and bridge-shaped manifests loading during transition.
3. Add a Myosu-shaped fixture and prove evaluation against it.
4. Port the richer program-state and live-lane-refresh semantics next.
5. Add lane kinds, proof profiles, and service/precondition state needed for
   Myosu-scale supervision.
6. Replace bootstrap-only `status` and `watch` behavior after the richer state
   model lands.

## Validation and Acceptance

Run these commands from the repository root:

    cargo test -p raspberry-supervisor
    cargo test -p raspberry-cli
    cargo run -p raspberry-cli -- plan --manifest test/fixtures/raspberry-supervisor/program.yaml
    cargo run -p raspberry-cli -- status --manifest test/fixtures/raspberry-supervisor/program.yaml

Then add equivalent validation for the Myosu-shaped proving-ground fixture.

The acceptance bar for this slice is:

- the bootstrap fixture still works
- a broader Myosu-shaped fixture also works
- no command behavior regresses
- the resulting model is visibly more general than the book-shaped bootstrap
- the next planned extension is driven by the actual Myosu workload classes,
  not by assumptions inherited from the book workflow

Current proof points:

    cargo test -p raspberry-supervisor
    cargo test -p raspberry-cli
    cargo run -p raspberry-cli -- plan --manifest test/fixtures/raspberry-supervisor/myosu-program.yaml
    cargo run -p raspberry-cli -- status --manifest test/fixtures/raspberry-supervisor/myosu-program.yaml

Observed Myosu-shaped plan output:

    Program: myosu-bootstrap
    Max parallel: 3
    Complete:
      - chain:runtime [platform] — managed milestone `reviewed` satisfied | proof=cargo_workspace
      - validator:oracle [service] — managed milestone `specified` satisfied | proof=validator_tests | preconditions=met
    Running:
      - miner:service [service] — run active at stage `Service Bringup`; checks passing: miner_http_ok, training_active | proof=miner_tests | operational=healthy
    Blocked:
      - operations:scorecard [orchestration] — waiting on proof checks: validator_proof_passed | proof=ops_smoke | proof_state=failed
    Failed:
      - play:tui [interface] — terminal snapshot mismatch | proof=tui_snapshots

Observed Myosu-shaped status output highlights:

    miner:service [running|service] ... stage=Service Bringup ... last_completed_stage=Spec
      operational=healthy
      usage: gpt-5.4: 1400 in / 600 out
      files_read: crates/myosu-miner/src/main.rs
      files_written: myosu/outputs/miner/miner_spec.md
      running_checks_passing: miner_http_ok, training_active
    validator:oracle [complete|service] ... proof_profile=validator_tests | preconditions=met
    operations:scorecard [blocked|orchestration] ...
      ready_checks_failing: validator_proof_passed
      proof_state=failed
    play:tui [failed|interface] ... last_completed_stage=Layout ...
      error: terminal snapshot mismatch

Reviewed Myosu workload classes from the full spec corpus:

- chain/runtime and pallet work
- miner and validator service work
- gameplay, TUI, spectator, and agent-experience work
- integration and launch orchestration work
- SDK and platform-extension work
- operational RPC and dashboard work
- recurring strategy, security, operations, and learning work

## Idempotence and Recovery

Treat the existing bootstrap supervisor as the fallback path while porting.
Prefer additive changes followed by deletion only after tests pass. If a
generalization attempt breaks the bootstrap fixture, restore the last passing
manifest/state model and port the behavior in smaller pieces.

Do not mix recurring compiler work or trust/landing work into this slice. If a
change starts to pull those systems in, stop and split the work.

## Artifacts and Notes

When implementation begins, record:

- the concept map from old `fabro_dispatch` to Raspberry crates
- the Myosu-shaped fixture manifest
- passing test transcripts for both fixtures
- one before/after example showing a book-shaped assumption that was removed

Current artifact note:

- The removed assumption so far is “dependencies are always lane-scoped and
  every program is shaped like chapter/page content.” The new manifest and
  evaluator now support unit-scoped milestone dependencies and a broader
  Myosu-shaped program.
- The next assumption to remove is “lane success is mostly an artifact
  existence question.” The full Myosu spec corpus makes clear that service
  health, proof state, orchestration preconditions, and recurring compiler
  semantics must become first-class in the control plane.
- The immediate base for that next step is now in place: lane runtime refresh
  and richer `status` output have been ported from the old bridge into the
  Raspberry supervisor.
- The next step after that is to replace fixture-backed checks with more real
  service and proof integrations where appropriate, and then decide which of
  those deserve first-class state beyond the current derived `healthy` /
  `degraded` layer.
- The current bridge toward that goal is explicit lane kind + scoped checks.
  The next step is to replace fixture-backed checks with more real service and
  proof integrations where appropriate.

## Interfaces and Dependencies

Use the existing Raspberry crate split:

- `raspberry-supervisor` for manifest/state/evaluation/dispatch logic
- `raspberry-cli` for command rendering and argument parsing

Do not add a second execution substrate. Keep the direct `fabro run` bridge
until there is a proven reason to replace it. The result of this plan should be
a better control-plane core, not a new runtime.
