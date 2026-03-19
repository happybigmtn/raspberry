# Build Raspberry Autodev Orchestrator

This ExecPlan is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

`PLANS.md` is checked into the repository root and this document must be
maintained in accordance with it. This plan depends on
`specs/031826-raspberry-malinka-control-plane-port.md`,
`plans/031826-port-and-generalize-fabro-dispatch-for-myosu.md`, and
`plans/031926-build-skill-guided-program-synthesis.md`.

## Purpose / Big Picture

After this slice lands, Raspberry should be able to run an `autodev` loop over
an existing program manifest:

1. evaluate the current program state
2. periodically run `fabro synth import` + `fabro synth evolve`
3. dispatch ready Fabro lanes
4. watch until the control plane settles or a cycle limit is reached

The first shipped loop does not need a fully model-backed evolve engine. It
should orchestrate the proven primitives that already exist:

- `raspberry plan/status/watch/execute`
- `fabro synth import`
- `fabro synth evolve`

The first proof target is a deterministic control-plane loop with safe stop
conditions and explicit scheduling knobs, not a speculative always-on daemon.

## Progress

- [x] (2026-03-19 13:44Z) Inspected the existing Raspberry/Fabro control-plane
  primitives and confirmed the orchestrator can be built by composing
  `evaluate_program`, `execute_selected_lanes`, and the existing runtime-state
  refresh path.
- [x] (2026-03-19 13:49Z) Added a new `raspberry autodev` CLI subcommand with
  bounded-cycle execution, cycle summary output, and scheduling knobs for poll
  interval and evolve cadence.
- [x] (2026-03-19 13:53Z) Added supervisor-side orchestration helpers that:
  - run periodic `synth import` + `synth evolve`
  - dispatch ready lanes
  - sleep/poll between cycles
  - stop on idle settlement or configured cycle limit
- [x] (2026-03-19 13:55Z) Added doctrine/evidence input injection for the
  evolve step so the orchestrator can feed the same context we use manually in
  the review-first loop.
- [x] (2026-03-19 14:01Z) Added a CLI test with a fake Fabro binary proving
  that the orchestrator invokes synth and run commands in the right sequence.
- [x] (2026-03-19 14:03Z) Ran targeted cargo tests and `git diff --check`.
- [x] (2026-03-19 15:35Z) Extended Raspberry so a top-level orchestration lane
  can supervise a child program manifest instead of only a raw Fabro run config.
- [x] (2026-03-19 15:37Z) Added portfolio-fixture coverage proving that
  top-level status summarizes child programs and that `execute` can tick a
  child `autodev` cycle.
- [x] (2026-03-19 15:40Z) Added a real repo-wide Myosu manifest at
  `fabro/programs/myosu.yaml` and verified that Raspberry now sees the whole
  repo as a program-of-programs frontier.

## Surprises & Discoveries

- Observation: Raspberry already owns the exact low-level surfaces the
  orchestrator should compose.
  Evidence: `raspberry-cli` already exposes `plan`, `status`, `watch`, and
  `execute`, while `raspberry-supervisor` already owns runtime-state refresh
  and lane readiness evaluation.

- Observation: the safest first autodev loop can still evolve while runs are in
  flight, because detached Fabro runs carry their own run config and do not
  depend on future manifest changes after submission.
  Evidence: the first orchestrator test passed cleanly once evolve was allowed
  to run on schedule even with existing running lanes, which matches the user's
  “every 30 minutes” expectation better than an idle-only evolve policy.

- Observation: doctrine/evidence injection belongs in the orchestrator layer,
  not in `raspberry execute`.
  Evidence: the autodev loop needs to compose `synth import` and `synth evolve`
  with the same file inputs we pass manually today, and that only exists when
  the orchestrator owns the temporary blueprint mutation step.

## Decision Log

- Decision: build the first orchestrator as a bounded CLI loop, not a daemon.
  Rationale: the user explicitly asked for “up to some limit”, and the current
  control plane is much easier to verify as a deterministic command than as a
  background service.
  Date/Author: 2026-03-19 / Codex

- Decision: place the orchestration loop in `raspberry-supervisor` and keep the
  CLI as a thin wrapper.
  Rationale: the supervisor already owns evaluation, dispatch, and runtime
  truth, so the orchestrator belongs beside those primitives rather than being
  reimplemented in `raspberry-cli`.
  Date/Author: 2026-03-19 / Codex

- Decision: run `synth evolve` on cadence even while lanes are already running.
  Rationale: detached Fabro runs are already submitted against concrete run
  configs, so evolving the repo on schedule only changes future dispatch
  direction; it does not mutate the behavior of in-flight runs.
  Date/Author: 2026-03-19 / Codex

## Outcomes & Retrospective

This plan starts from a repo that already has dispatch and run-truth
primitives, but no single command that composes periodic evolution and lane
execution. The intended outcome is a new `raspberry autodev` command that can
drive a bounded execution loop over a real supervised repo without hand-running
`synth`, `execute`, and `watch` as separate commands.

That first slice is now in place:

- `raspberry autodev` exists in
  [main.rs](/home/r/coding/fabro/lib/crates/raspberry-cli/src/main.rs)
- the orchestration loop lives in
  [autodev.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/autodev.rs)
- the supervisor public surface re-exports the new orchestrator types from
  [lib.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/lib.rs)
- the orchestrator can:
  - run periodic `synth import` + `synth evolve`
  - inject doctrine/evidence inputs into the imported blueprint
  - dispatch ready lanes through the existing detached-run path
  - stop on either settlement or configured cycle limit

The proof bar for this slice is also real:

- [CLI test](/home/r/coding/fabro/lib/crates/raspberry-cli/tests/cli.rs) now
  proves the autodev command invokes synth and dispatch in the same bounded
  loop using a fake Fabro binary.
- `cargo test -p raspberry-supervisor -p raspberry-cli -- --nocapture` passes.

What remains is the richer second half of autodev:

- a live Myosu proof that uses the real `fabro` binary instead of the fake one
- policy about when autodev should auto-apply evolve changes versus preview them
- eventually, a model-backed evolve step if we want `gpt-5.4` to participate in
  the direction-update phase rather than only deterministic synthesis

That next slice has now started too. The orchestrator is no longer limited to a
single flat program manifest:

- `LaneManifest` can now point at a child `program_manifest`
- `evaluate` can summarize child-program state as a top-level orchestration lane
- `execute` can tick one bounded child `autodev` cycle instead of only calling
  `fabro run --detach`
- `fabro-synthesis` now round-trips this manifest surface so a portfolio
  manifest survives `synth import` + `synth evolve`

The first real proving target for that new shape also exists:

- [myosu.yaml](/home/r/coding/myosu/fabro/programs/myosu.yaml)

and the first real top-level status pass shows the intended operator view:

- bootstrap complete
- chain-core complete
- traits implementation complete
- services complete
- product complete
- platform complete
- recurring ready

That is the first real end-to-end proof that Raspberry can supervise Myosu as a
repo-wide program-of-programs rather than only as one frontier at a time.
