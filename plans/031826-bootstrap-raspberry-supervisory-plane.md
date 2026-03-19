# Bootstrap Raspberry Supervisory Plane

This ExecPlan is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

`PLANS.md` is checked into the repository root and this document must be
maintained in accordance with it. This plan also depends on the migration spec
at `specs/031826-raspberry-malinka-control-plane-port.md`.

## Purpose / Big Picture

After this slice lands, this repository will have its own Raspberry-native
supervisory surface for Fabro-style programs. A contributor will be able to run
a Raspberry CLI to plan, inspect status, watch, and execute a multi-lane
program described by a manifest, without depending on the old Malinka repo.
This is the first concrete step toward making Raspberry the place where
execution and supervision live together.

## Progress

- [x] (2026-03-18 22:20Z) Wrote the migration spec
  `specs/031826-raspberry-malinka-control-plane-port.md`.
- [x] (2026-03-18 22:20Z) Wrote this ExecPlan for the first migration slice.
- [x] (2026-03-18 22:58Z) Created `lib/crates/raspberry-supervisor/` with a
  program manifest model, lane evaluation logic, program runtime state, and a
  minimal dispatch bridge.
- [x] (2026-03-18 23:00Z) Created `lib/crates/raspberry-cli/` with `plan`,
  `status`, `watch`, and `execute` subcommands.
- [x] (2026-03-18 23:04Z) Added repository-local fixtures under
  `test/fixtures/raspberry-supervisor/` covering complete, ready, blocked,
  failed, and running lane states.
- [x] (2026-03-18 23:12Z) Added unit and CLI tests proving the supervisor
  works without the old Malinka repo.
- [x] (2026-03-18 23:16Z) Verified the CLI against the repository-local
  fixture program, including `execute` via a fake `fabro` binary.
- [ ] Follow-on work continues in
  `plans/031826-port-and-generalize-fabro-dispatch-for-myosu.md`.

## Surprises & Discoveries

- Observation: this repository had no `SPEC.md`, `specs/`, or `plans/`
  convention before this migration work started.
  Evidence: repository root inspection on 2026-03-18 showed only `PLANS.md` at
  the root and no existing `specs/` or `plans/` directories.

- Observation: the current Fabro workspace structure is already well-suited to
  layered crates because `Cargo.toml` includes all `lib/crates/*` members
  automatically.
  Evidence: root `Cargo.toml` uses `members = ["lib/crates/*"]`.

- Observation: lane dependencies must be tracked at lane scope, not only at
  unit scope.
  Evidence: the first evaluation pass incorrectly marked `runtime:page` as
  blocked even though `runtime:chapter` was complete, because the dependency
  key used `unit@milestone` while lane dependencies were declared as
  `unit:lane@milestone`. Fixing the satisfied milestone set to include
  lane-scoped managed milestones made both the supervisor tests and CLI tests
  pass.

- Observation: a fake `fabro` binary is enough to prove the bootstrap `execute`
  boundary without introducing a second real demo workflow in the first slice.
  Evidence: the CLI test `execute_updates_program_state_using_fake_fabro`
  successfully exercised `execute`, updated `.raspberry/program-state.json`,
  and verified the lane moved to `complete`.

## Decision Log

- Decision: the first slice ports only supervisory behavior, not recurring
  compilation or trust/landing semantics.
  Rationale: the smallest useful milestone is Raspberry-native supervision over
  existing Fabro-style run truth. Pulling in recurring and trust semantics in
  the same slice would make the first port too wide and would blur whether the
  basic Raspberry control-plane shell is sound.
  Date/Author: 2026-03-18 / Codex

- Decision: introduce new Raspberry crates rather than modifying generic
  `fabro-*` crates for control-plane policy.
  Rationale: the migration spec requires a clean boundary where Fabro core
  remains the execution substrate and Raspberry owns layered supervision.
  Date/Author: 2026-03-18 / Codex

- Decision: make repository-local fixtures part of the first slice.
  Rationale: `PLANS.md` requires self-contained validation. A novice with only
  this repository must be able to prove the supervisory surface works without
  checking out the old Malinka repo.
  Date/Author: 2026-03-18 / Codex

- Decision: keep the first supervisory slice strictly additive to preserve easy
  upstream Fabro sync.
  Rationale: upstream Fabro server mode and other execution-plane improvements
  should be adoptable in Raspberry. This first slice therefore introduces new
  `raspberry-*` crates instead of reshaping generic Fabro core beyond small,
  generally useful improvements.
  Date/Author: 2026-03-18 / Codex

- Decision: keep the first `execute` implementation as a direct `fabro run`
  bridge rather than adding a Raspberry adapter layer.
  Rationale: the migration spec allows a temporary bridge, and the smallest
  truthful implementation is to call the local `fabro` binary directly while
  Raspberry owns program manifests and program state. An adapter layer can wait
  until there is evidence that the direct bridge is too limiting.
  Date/Author: 2026-03-18 / Codex

## Outcomes & Retrospective

Milestone 1 landed successfully, and enough of Milestones 2 and 3 landed to
prove the boundary. The repository now has:

- `raspberry-supervisor`, a new control-plane library crate
- `raspberry-cli`, a new CLI binary crate
- repository-local fixture programs and run-truth fixtures
- passing tests for `plan`, `status`, `watch`, and a bootstrap `execute` path

The main lesson from this slice is that the Raspberry control-plane boundary
does feel natural when hosted in this repo. The first bridge can stay thin:
Raspberry owns program manifests and program state, while Fabro remains the
execution substrate. The next active plan should port and generalize the proven
`fabro_dispatch` semantics rather than keep elaborating the bootstrap model in
place.

## Context and Orientation

Raspberry is the Fabro fork in this working tree. Fabro core already provides
the execution substrate: staged workflow runs, run state, checkpoints, and the
CLI/API used to execute workflows. What is missing is a Raspberry-native
control plane that treats multiple runs as one program with named units, lanes,
milestones, and operator truth.

In the old Malinka repo, the relevant idea was a supervisory layer above Fabro
that could:

- describe a program in one manifest
- decide which lanes were ready or blocked
- render `plan`, `status`, and `watch` surfaces
- execute a bounded set of Fabro run configs
- persist program-level state separately from raw workflow run truth

This plan ports only that first supervisory layer.

The key repository areas to know are:

- `lib/crates/fabro-cli/` for the existing Fabro CLI model
- `lib/crates/fabro-workflows/` for run truth such as `state.json`,
  `progress.jsonl`, and `checkpoint.json`
- `SPEC.md` for spec rules in this repository
- `PLANS.md` for ExecPlan rules in this repository
- `specs/031826-raspberry-malinka-control-plane-port.md` for the migration
  architecture this plan implements

A **program manifest** is a file that describes a larger body of work made of
multiple units and lanes. A **unit** is the thing being produced. A **lane** is
an independently schedulable stream of work for that unit. A **milestone** is a
durable lifecycle checkpoint like `reviewed` or `publish_ready`.

## Milestones

### Milestone 1: Bootstrap Raspberry crates and manifest model

At the end of this milestone, the repository contains a new Raspberry library
crate for parsing a program manifest and computing lane readiness from
repository-local fixture state, plus a new Raspberry CLI crate with a minimal
binary. The proof is that `cargo test` passes for the new crate and `cargo run`
can render a human-readable `plan` output from a fixture manifest.

### Milestone 2: Add status and watch surfaces over stable run truth

At the end of this milestone, the Raspberry CLI can show current lane status
and a live watch view by reading Fabro-style stable run truth such as
`state.json`, `status.json`, and `progress.jsonl`. The proof is that fixture
tests exercise running, blocked, complete, and failed lanes, and a local watch
command prints stable summaries instead of forcing users to read raw journals.

### Milestone 3: Add bounded execute over local Fabro-style programs

At the end of this milestone, the Raspberry CLI can invoke the local `fabro`
binary for selected run configs, persist program-level state, and update status
after execution. The proof is that a repository-local integration test or demo
can execute a toy program end to end and show updated lane state.

## Plan of Work

Create two new crates under `lib/crates/`. The first crate is
`raspberry-supervisor`, which owns the program manifest schema, lane dependency
evaluation, program state files, and helpers that read Fabro run truth. The
second crate is `raspberry-cli`, which exposes the command-line surface.

In the library crate, define a manifest model that is intentionally small:
program metadata, units, lanes, artifacts, milestones, and lane dependencies.
Do not port recurring compilation or trust policy here yet. The first goal is
to answer "what should run now?" and "what is the current state?" reliably.

Add a small state layer in the library crate that persists program runtime
state in a repository-local file. That state should track the selected lanes,
current lane status, latest Fabro run identifier if one exists, and concise
operator summaries. Use the existing Fabro run truth as the source for
workflow-level facts instead of duplicating workflow internals.

In the CLI crate, add four subcommands:

- `plan`, which reads a manifest and prints ready, blocked, and complete lanes
- `status`, which prints the same information plus current runtime state
- `watch`, which periodically refreshes status and emits new progress
  information
- `execute`, which dispatches selected lanes by calling the local `fabro`
  binary on run configs described by the manifest

Use repository-local test fixtures for the first two milestones. Create a
fixture program manifest and sample run directories beneath `test/fixtures/`
that contain minimal `status.json`, `state.json`, `progress.jsonl`, and
`conclusion.json` files. These fixtures must be enough for a novice to run the
tests and understand what "blocked", "ready", and "complete" mean.

For Milestone 3, either add a tiny repository-local demo workflow or use a
fake `fabro` binary in tests. The first implementation chose the fake binary
because it kept the bootstrap slice small and still proved the bridge shape.

## Concrete Steps

Work from the repository root.

1. Create `lib/crates/raspberry-supervisor/` with:
   - `Cargo.toml`
   - `src/lib.rs`
   - `manifest.rs`
   - `evaluate.rs`
   - `program_state.rs`
   - `dispatch.rs`

2. Create `lib/crates/raspberry-cli/` with:
   - `Cargo.toml`
   - `src/main.rs`
   - `tests/cli.rs`

3. Create `test/fixtures/raspberry-supervisor/` with:
   - `program.yaml`
   - run config placeholders
   - output artifacts
   - sample Fabro `status.json` and `state.json` files

4. Keep `execute` as a direct `fabro run` bridge for this slice and verify it
   with a fake binary in tests.

## Validation and Acceptance

Run these commands from the repository root:

    cargo test -p raspberry-supervisor
    cargo test -p raspberry-cli
    cargo run -p raspberry-cli -- plan --manifest test/fixtures/raspberry-supervisor/program.yaml
    cargo run -p raspberry-cli -- status --manifest test/fixtures/raspberry-supervisor/program.yaml

The expected early acceptance behavior is:

- `plan` prints at least one `ready` lane and one `blocked` lane from the
  fixture manifest
- `status` prints lane state plus the linked Fabro run truth where available
- all tests pass without requiring the old Malinka repo

For Milestone 3, add this acceptance behavior:

    cargo run -p raspberry-cli -- execute --manifest test/fixtures/raspberry-supervisor/program.yaml

The command must update program runtime state and show that one selected lane
was dispatched or simulated successfully.

## Idempotence and Recovery

This work should be additive. Creating the new crates and fixtures is safe to
repeat. If `execute` is being developed against a fake `fabro` target, keep the
fake target in tests only so repeated runs do not mutate repository state. If a
real demo workflow is used, ensure it writes only under a dedicated test or
demo directory and can be deleted and recreated safely.

If the CLI shape changes during implementation, update both this plan and the
fixture expectations immediately. Do not let command names drift away from what
the plan says.

## Artifacts and Notes

When implementation starts, record the most useful evidence here:

- `cargo test -p raspberry-supervisor`
  Result: 6 tests passed

- `cargo test -p raspberry-cli`
  Result: 4 CLI tests passed

- `raspberry plan --manifest test/fixtures/raspberry-supervisor/program.yaml`
  Output shape:
    Program: raspberry-demo
    Complete:
      - runtime:chapter — managed milestone `reviewed` satisfied
    Ready:
      - runtime:page — dependencies satisfied
    Running:
      - p2p:chapter — run active at stage `Review`
    Blocked:
      - consensus:page — waiting on consensus:chapter@reviewed
    Failed:
      - consensus:chapter — LLM error: builder error

Keep examples short and focused on proof of behavior.

## Interfaces and Dependencies

The first slice should use only repository-local crates and the Rust standard
toolchain already present here. Prefer new crates under `lib/crates/` rather
than adding large external dependencies.

At the end of Milestone 1, these interfaces must exist conceptually:

- a manifest loader that reads a program manifest from disk
- a readiness evaluator that classifies lanes as blocked, ready, running,
  complete, or failed
- a runtime state store that persists program-level truth
- a CLI entrypoint that exposes `plan`, `status`, `watch`, and `execute`

Stable names may change during implementation, but the responsibilities should
remain separate: parsing, evaluation, persistence, and command rendering should
not collapse into one file.

Revision note: 2026-03-18 / Codex
This plan was updated after implementation of the bootstrap slice to reflect
the crates, fixtures, direct `fabro run` bridge, passing tests, and the
lane-scoped milestone discovery that changed the readiness model.
