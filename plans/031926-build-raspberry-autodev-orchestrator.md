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
- [x] (2026-03-19 16:00Z) Taught the orchestrator to advance settled child
  programs and taught synthesis to discover newly created child manifests so a
  repo-wide portfolio can expand itself into the next implementation round.
- [x] (2026-03-19 16:12Z) Fixed a live-apply synthesis corruption bug where
  preserving an existing lane package in-place could self-copy files and
  clobber workflows/run-configs to empty files.
- [x] (2026-03-19 16:18Z) Fixed the top-level loop to reload the manifest each
  cycle so newly synthesized child programs actually enter scheduling instead
  of remaining invisible until a manual restart.
- [x] (2026-03-19 16:24Z) Switched the proving setup from disposable `/tmp`
  repo copies to a real Myosu git worktree (`autodev-live`) so autodev writes
  into a commit-capable branch with real repo data.
- [x] (2026-03-19 16:31Z) Proved the worktree-backed loop can expand the
  portfolio automatically and dispatch newly synthesized implementation
  programs such as `play-tui-implementation` and the platform implementation
  frontiers.
- [x] (2026-03-19 16:34Z) Confirmed via the live worktree report that the
  top-level loop stayed active through six cycles with four generated child
  implementation programs in flight, not just one synthetic expansion step.
- [x] (2026-03-19 16:37Z) Captured the first live child-implementation status
  snapshot from the real worktree loop: all four generated child programs are
  now `running`, and at least the `play-tui` and `poker-engine` implementation
  children are failing their running proof checks because the generated proof
  commands reference packages that do not yet exist.
- [x] (2026-03-19 21:49Z) Added a deterministic implementation quality pack to
  the synthesized workflow contract so `review` and `promote` consume explicit
  evidence instead of relying on prose-only artifacts.
- [ ] Harden implementation-family promotion with machine-checked blockers for:
  placeholder/stub debt, compiler warnings, stale promotion artifacts, and
  artifact/code mismatch signals.
- [ ] Teach the autonomous control plane to surface quality-gate truth in
  runtime state/TUI terms so a lane that is "running in promote" is legible as
  "blocked on auth", "blocked on warnings", or "blocked on placeholder debt".
- [x] (2026-03-19 22:20Z) Added native Codex slot-rotator logic to the Fabro
  CLI backend so OpenAI/Codex stages can launch with a selected `CODEX_HOME`
  from the saved slot pool, record slot metadata, and cool down slots on
  quota/auth failures.

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

- Observation: the first live-apply synth preservation path was unsafe when the
  output repo and current repo were the same directory.
  Evidence: a worktree-backed autodev run zeroed out
  `fabro/run-configs/bootstrap/chain-pallet-restart.toml` and the paired
  workflow until the `copy_file()` path learned to no-op on self-copy.

- Observation: portfolio expansion worked before portfolio scheduling because
  the orchestrator held one in-memory manifest for the whole loop.
  Evidence: `myosu.yaml` on disk already contained the new implementation child
  programs, while the running top-level loop still reported the old 7-unit
  picture until per-cycle manifest reload was added.

- Observation: a disposable `/tmp` clone was useful for crash padding but is
  the wrong long-term operator model.
  Evidence: the user correctly objected that `/tmp` is not “real production
  data,” and moving the same loop onto a git worktree immediately made the
  output commit-capable and easier to reason about.

- Observation: once the implementation-family child programs entered the
  portfolio, the top-level autodev report stopped listing ready lanes but kept
  reporting `running_after=4` across later cycles.
  Evidence: the live worktree report at
  `/home/r/coding/myosu/.worktrees/autodev-live/.raspberry/myosu-autodev.json`
  shows cycles 2-6 with no new top-level dispatches and four child programs in
  flight throughout.

- Observation: the first generated implementation frontiers are now live enough
  to expose synthesis-quality bugs in their proof commands, not just in their
  portfolio registration.
  Evidence: `myosu-play-tui-implementation` reports
  `error: package ID specification myosu-play did not match any packages`, and
  `myosu-games-poker-engine-implementation` reports
  `error: package ID specification myosu-games-poker did not match any
  packages`, both during the `Implement` stage while their running proof checks
  fail.

- Observation: the worktree has already moved beyond that earliest failure
  point; the generated crates now exist and direct manual cargo probes block on
  artifact locks instead of failing on missing package ids.
  Evidence: the worktree now contains `crates/myosu-play/`,
  `crates/myosu-games-poker/`, and `crates/myosu-sdk/`, while manual
  `cargo build -p ...` probes against those packages wait on the workspace
  artifact directory lock rather than returning “package not found”.

- Observation: the first generated implementation child has already crossed to
  `complete` even while its stale preflight failure text still lingers in lane
  state.
  Evidence: `myosu-play-tui-implementation` now reports
  `Counts: complete=1 ready=0 running=0 blocked=0 failed=0`, even though the
  lane still shows the earlier `myosu-play` package-id failure in its last
  error field.

- Observation: the first manual smoke review shows that “artifact complete” is
  still too weak a merge signal for code-generating lanes.
  Evidence: running the built binary directly as
  `myosu-play train` from the worktree exits immediately with code 0 instead of
  entering a usable TUI loop, which matches the generated implementation note
  that `Shell::new()` is created but never run.

- Observation: the remaining quality gap is mostly a contract problem, not a
  raw model problem.
  Evidence: generated code can satisfy the current promotion contract while
  still containing obvious scaffold markers (`TODO`, `stub`, compile-only
  loops, placeholder tests), which means the workflow is rewarding the wrong
  evidence rather than failing to generate syntax.

- Observation: the first deterministic quality pack is now real and catches the
  exact kind of optimism we were still finding by hand.
  Evidence: the refreshed live `play-tui` implementation workflow now writes
  `outputs/play/tui/quality.md`, runs a `Quality Gate` before review, and its
  underlying artifact scan already finds `future slices` / `placeholder`
  language in the current `play` artifacts that would keep `quality_ready=no`.

- Observation: the host already had slot-managed Codex auth surfaces; Fabro was
  simply not using them.
  Evidence: `/home/r/.codex` plus `/home/r/.codex-slot1..5/.codex` all contain
  `auth.json`, and `~/.config/autonomy/codex-rotator.json` already defines a
  sticky slot pool with cooldown state.

- Observation: the first promotion-hardening refresh is now landing directly in
  the generated implementation package, not only in our review notes.
  Evidence: rerunning `synth evolve` for `myosu-product` in the live worktree
  rewrote `myosu-play-tui-implementation.yaml` so it now requires a new
  `promotion.md` artifact and promotes the lane only at the `merge_ready`
  milestone.

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

- Decision: the long-term autodev target should be a real git worktree, not a
  disposable `/tmp` copy.
  Rationale: a worktree keeps live changes on a real branch with commit/push
  semantics while still isolating the loop from the user's main checkout.
  Date/Author: 2026-03-19 / User + Codex

- Decision: the next autonomy-hardening slice should convert manual operator
  skepticism into a synthesized evidence contract.
  Rationale: the review/promote legs need deterministic evidence about warnings,
  placeholders, smoke behavior, and artifact/code consistency before a strong
  reviewer can make a trustworthy merge judgment.
  Date/Author: 2026-03-19 / User + Codex

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

The next slice is now explicit too: implementation-family workflows need a
deterministic quality pack, not just `implementation.md`, `verification.md`,
and `promotion.md`. The system already knows how to run proof commands and
track stage truth; the missing structural move is to synthesize quality gates
that turn our manual review heuristics into machine-checkable evidence before
`review` and `promote` are allowed to bless a lane as merge-ready.

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

The next live proving round pushed that much further. Autodev now:

- advances settled child programs instead of stopping at “current frontier set
  complete”
- lets child `synth evolve` runs create new implementation program manifests
- teaches the top-level portfolio to discover those new manifests automatically
- runs against a real Myosu git worktree instead of a disposable `/tmp` clone

The current live proof is the `autodev-live` worktree under:

- `/home/r/coding/myosu/.worktrees/autodev-live`

In that worktree, the top-level `myosu` loop has already:

- expanded `myosu.yaml` with implementation-family child programs
- made those child programs schedulable at the portfolio level
- dispatched the first implementation-family child autodev cycles

The latest live snapshot goes further than that initial dispatch:

- the top-level loop remained active for six cycles
- cycles 2 through 6 consistently reported `running_after=4`
- the four in-flight child programs are:
  - `games-multi-game-implementation`
  - `games-poker-engine-implementation`
  - `play-tui-implementation`
  - `sdk-core-implementation`

That means the orchestrator is now in the phase we wanted most: not just
generating the next frontier, but keeping the next frontier running on real
repo data inside a real worktree branch.

The latest synthesis refresh also begins to tighten promotion semantics for
those generated implementation children:

- `myosu-play-tui-implementation` now requires `promotion.md` and targets
  `merge_ready`
- the same promotion-hardening refresh is being pushed through the generated
  platform implementation-family programs

That is the first move from “artifact landed” toward “artifact landed and is
machine-checked as merge-worthy.”

The corresponding reviewer policy is now explicit too:

- generated implementation `promote` steps should stay on `gpt-5.4`
- worker/implementation steps can continue using the faster MiniMax path
- deterministic gates still own the final hard checks, but the merge-worthiness
  adjudication remains a strong-model review step for now

The next best step is no longer portfolio expansion. It is tightening the truth
and execution surfaces around those generated implementation frontiers:

- reduce expensive/blocking evaluation in top-level status/tui paths
- verify the implementation child programs continue from “dispatched” into
  durable code/artifact progress
- commit the worktree branch once the generated implementation-frontier changes
  are coherent enough to keep
- fix generated implementation proof commands and startup slices so the child
  programs target code that can actually exist in the current repo shape
- clean up stale failure text/state after successful implementation completions
  so operator surfaces do not show “complete” and an old blocking error at the
  same time
- stop treating “some produced artifacts exist” as proof that a lane is still
  actively running; finished implementation children need to be eligible for a
  fresh rerun under the newer contract

Live follow-up note: after the lane-truth refresh, all four generated
implementation children in the worktree now report `ready` instead of
remaining forever `running`:

- `play-tui-implementation`
- `games-poker-engine-implementation`
- `games-multi-game-implementation`
- `sdk-core-implementation`

That is the exact state transition needed for the next worktree autodev pass to
re-dispatch them under the newer `merge_ready` contract.

Latest live scheduler note:

- a fresh one-cycle top-level autodev pass against the worktree now reports all
  four generated implementation children as ready
- but only dispatches two of them in that cycle because `myosu.yaml` still has
  `max_parallel = 2`

That is the first direct proof that the scheduler is starting to treat
`max_parallel` as a real concurrency cap instead of “submit everything now.”

Current child-run note:

- `games-multi-game-implementation` is now running a fresh rerun with
  `fabro_run_id=01KM3HXYKR9YRZVZHE5F34FP2T`
- `games-poker-engine-implementation` is now running a fresh rerun with
  `fabro_run_id=01KM3HX7QS8017YT3WPZECGCG9`
- both passed `Preflight`, both are in `Implement`, and both currently report
  their running proof checks as passing

That is the first concrete evidence that the refreshed implementation-family
contract is now executing live rather than only being written to disk.

Latest worktree monitor snapshot:

- the most recent top-level worktree report now lists only two ready child
  programs:
  - `play-tui-implementation`
  - `sdk-core-implementation`
- and it dispatches none in that cycle because two other implementation
  children are still occupying the portfolio's `max_parallel = 2` slots

That is the expected backpressure behavior once the concurrency cap is treated
as real scheduler state rather than as a submit-all batch size.
- introduce a stronger promotion protocol for implementation lanes so “complete”
  does not imply “ready to merge to trunk” without manual/runtime proof
- keep teaching `synth evolve` to infer missing promotion/proof gates from
  observed false-positive completions instead of relying on manual patching

Latest live review note:

- `raspberry status --manifest /home/r/coding/myosu/.worktrees/autodev-live/fabro/programs/myosu.yaml`
  now reports `complete=9 ready=2 running=0 blocked=0 failed=0`
- the two remaining ready child programs are:
  - `play-tui-implementation:program`
  - `sdk-core-implementation:program`
- poker and multi-game implementation children have advanced far enough to emit
  `promotion.md`, and direct package proof reruns in the live worktree passed:
  - `cargo test -p myosu-games-poker`
  - `cargo test -p myosu-games-liars-dice`
- however, the top-level persisted runtime file
  `/home/r/coding/myosu/.worktrees/autodev-live/.raspberry/myosu-state.json`
  still marks four implementation children as `running`, even though fresh
  evaluation now reports `running=0`

That means the implementation-family synth loop is generating meaningful,
executable slice outputs, but the top-level orchestrator still is not fully
effective as a continuous queue-draining control plane until persisted parent
state and follow-on autodev scheduling converge automatically.

Latest harness fixes (2026-03-19, later):

- `refresh_program_state()` now synchronizes orchestration/program-lane runtime
  records from child program evaluation, so the persisted parent state file no
  longer stays stuck at `running` after child implementation programs settle
- verified with a new supervisor regression:
  - `refresh_program_state_syncs_child_program_lane_statuses`
- `autodev` no longer runs `synth evolve` ahead of already-ready work
  - new policy: consume ready/running local frontier first, then evolve once
    the current frontier is locally settled
  - verified with new CLI regressions:
    - `autodev_runs_synth_and_dispatch_cycle`
    - `autodev_evolves_when_program_is_locally_settled`

This is a direct development-velocity improvement: the control plane is now
closer to “dispatch current work first, synth only when the current frontier is
exhausted,” which matches the intended autonomous loop.

Latest isolation improvement:

- Fabro processes spawned by Raspberry now receive
  `CARGO_TARGET_DIR=<target_repo>/.raspberry/cargo-target`
  in both:
  - detached lane execution (`dispatch.rs`)
  - synth import/evolve (`autodev.rs`)

Intent:

- keep autonomous cargo work off the shared default target directory
- reduce interference from external/background cargo jobs in the live repo
- make future live autodev relaunches more reliable

Note:

- the exact CLI/env regression for this cargo-target isolation still needs one
  clean verification pass after the shared target finishes recovering from
  earlier interrupted cargo builds

Latest live promotion-loop findings:

- live `play-tui` and `sdk-core` implementation runs confirmed that review /
  promote agent stages were still falling back to the project default
  Anthropic provider instead of honoring node-level `provider: openai`
- a focused `fabro-workflows` regression now covers the provider-resolution
  helper for agent runs:
  - `resolved_model_provider_prefers_node_overrides`
- live `sdk-core` also exposed a stale-artifact loophole:
  `promotion_check` could succeed from an old `promotion.md` even when review /
  promote just failed
- the implementation workflow template now clears `promotion.md` before review /
  promote so a stale promotion artifact cannot satisfy the promotion gate
- `import_existing_package()` now preserves implementation-family template
  identity and proof commands instead of collapsing implementation programs back
  to bootstrap-style workflows during synth refresh
- after rebuilding `fabro-cli`, child-package `synth evolve` on:
  - `myosu-play-tui-implementation`
  - `myosu-sdk-core-implementation`
  now rewrites their workflow files with:
  - `#review` / `#promote` on `backend: cli`, `model: gpt-5.4`, `provider: openai`
  - explicit `clear_promotion` step before review/promote
  - preserved real proof commands in preflight/verify
- live child re-dispatch under the refreshed workflows is now running:
  - `play:tui-implement` -> `01KM3SPS33KNYAFMK2RFY76HC1`
  - `sdk:core-implement` -> `01KM3SQ8SP199TQD61QSS353AB`
- the dedicated autonomous cargo target is now visible on disk at:
  - `/home/r/coding/myosu/.worktrees/autodev-live/.raspberry/cargo-target`
- after fixing synth import/evolve, refreshed child implementation workflows now
  preserve:
  - the implementation template
  - real proof commands in preflight/verify
  - `clear_promotion` before review/promote
- current live evidence from the new child runs:
  - both fresh runs are still in `Implement`
  - both are already using `.raspberry/cargo-target`
  - `implement` remains on the Anthropic/MiniMax CLI path, which is expected
  - the next decisive checkpoint is whether `review` / `promote` switch away
    from the old Anthropic 404 path once those stages are reached
- an additional operational finding from the proving ground:
  - direct non-interactive dispatches still miss the MiniMax/Anthropic auth
    exported only in `.bashrc`, which caused fresh non-interactive implement
    runs to 401 and retry
  - interactive-shell re-dispatches of the same child lanes then progressed
    through preflight and back into implement correctly
  - this points at the next harness gap: autonomous Fabro launches should not
    depend on interactive shell exports for provider auth

Latest auth-path fix:

- `fabro-cli` run assembly now merges and resolves project-level `fabro.toml`
  sandbox env into the backend env, instead of only forwarding per-run-config
  sandbox env
- Raspberry Fabro spawns now go through `bash -ic` so autonomous launches can
  inherit the same `.bashrc`-backed provider exports the interactive retries
  previously depended on
- proving-ground result:
  - fresh non-interactive `play:tui-implement` / `sdk:core-implement` dispatches
    now submit successfully under the fixed harness
  - the new `play:tui-implement` run `01KM3T2ADPSXBFN17FGPD0XW2K` is actively
    executing agent work in `Implement` instead of immediately dying on auth
- prompt-contract tightening:
  - implementation/fixup prompts now explicitly forbid writing `promotion.md`
  - promotion prompts now explicitly state that `promotion.md` is owned by the
    Promote stage
  - this came directly from the fresh `play` proving-ground run, which showed
    `Implement` reaching forward into promotion artifacts
