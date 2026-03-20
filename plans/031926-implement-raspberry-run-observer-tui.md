# Implement the Raspberry Run Observer TUI

This ExecPlan is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

`PLANS.md` is checked into the repository root and this document must be
maintained in accordance with it. This plan depends on
`specs/031926-raspberry-run-observer-tui.md`,
`specs/031826-raspberry-malinka-control-plane-port.md`, and
`plans/031826-port-and-generalize-fabro-dispatch-for-myosu.md`.

## Purpose / Big Picture

After this slice lands, an operator can run a single command and observe a
Raspberry program in a real terminal UI instead of reading plain-text
`status`/`watch` output or opening raw JSON files. They can move around with
Vim-style keys, collapse panes, inspect curated lane artifacts, and see
best-effort live run detail when available.

The first user-visible command should be:

    raspberry tui --manifest test/fixtures/raspberry-supervisor/myosu-program.yaml

The first release is read-only and must work even when live run truth is
partial or stale.

## Progress

- [x] (2026-03-19 05:34Z) Reviewed the capability spec and the current
  Raspberry CLI/supervisor code in `lib/crates/`.
- [x] (2026-03-19 05:34Z) Chose the implementation home: add the `tui`
  subcommand to `raspberry-cli`, but put the rendering/runtime logic in a new
  dedicated crate instead of bloating `raspberry-cli/src/main.rs`.
- [x] (2026-03-19 06:34Z) Added the new `raspberry-tui` crate with pane
  layout, navigation state, rendering primitives, and file preview helpers.
- [x] (2026-03-19 06:34Z) Extended `raspberry-cli` with `tui --manifest <path>`
  and kept the CLI handoff thin.
- [x] (2026-03-19 06:34Z) Reused `raspberry-supervisor` for manifest and lane
  evaluation data, adding only `ProgramManifest::resolve_lane_artifacts()` as
  the observer-specific helper.
- [x] (2026-03-19 06:34Z) Added fixture-backed tests for artifacts, stale live
  detail, folding, and rendering, then manually started the observer against
  the Myosu fixture and quit cleanly with `q`.
- [x] (2026-03-19 06:54Z) Added a dedicated completed-result summary to the
  detail pane so finished lanes call out their satisfied milestone and durable
  outputs explicitly.
- [x] (2026-03-19 07:15Z) Added best-effort recent-run matching so the detail
  pane can show successful Fabro run ids for lanes whose curated artifacts were
  produced by recent runs, even when Raspberry does not already have an
  explicit lane-to-run link in state.
- [x] (2026-03-19 14:12Z) Added durable autodev report loading so the TUI can
  surface the latest autodev cycle summary for the selected lane when a
  Raspberry autodev loop has been run against that program.
- [x] (2026-03-19 16:18Z) Redesigned the default TUI presentation into a more
  dashboard-like operator view:
  - grouped program rows by status instead of unit-only sequencing
  - added whole-program status counts to the program and state surfaces
  - switched the wide-screen layout to a dashboard split so state/detail panes
    have useful horizontal space
  - made state/detail sections more structured and less log-like
- [ ] Add a deterministic nested child-program digest for orchestration lanes
  so the state/detail panes can show what is actually running, ready, and
  blocked inside a selected child program without requiring a second manifest
  view.
- [ ] Decide whether to add a cached MiniMax-generated plain-English narrator
  over that structured child summary once the deterministic operator digest is
  in place.

## Surprises & Discoveries

- Observation: the current Raspberry CLI is still one-file thin, which is good
  news for adding a new subcommand but bad news for putting all TUI rendering
  there.
  Evidence: `lib/crates/raspberry-cli/src/main.rs` is the only source file in
  that crate today.

- Observation: `raspberry-supervisor` already exports most of the data model
  the TUI needs.
  Evidence: `lib/crates/raspberry-supervisor/src/lib.rs` already re-exports
  `ProgramManifest`, `EvaluatedProgram`, `EvaluatedLane`, `ProgramRuntimeState`,
  and refresh helpers.

- Observation: the Fabro workspace does not currently use terminal UI
  libraries, so the TUI plan must include dependency additions explicitly.
  Evidence: `Cargo.toml` has no `ratatui` or `crossterm` workspace
  dependencies yet.

- Observation: `ratatui`'s `Layout::areas()` API now uses a const-generic
  array size, so the four-pane splitter had to encode the pane count
  explicitly instead of relying on inferred vector sizing.
  Evidence: the first `cargo test -p raspberry-tui` run failed with
  `type annotations needed for [Rect; _]` until `layout.rs` declared
  `[Rect; 4]`.

- Observation: bringing the touched Raspberry crates up to the repo's
  `-D warnings` standard also required cleaning older clippy findings in
  `raspberry-supervisor` and `raspberry-cli`.
  Evidence: `cargo clippy -p raspberry-supervisor -p raspberry-tui -p raspberry-cli -- -D warnings`
  initially failed on `for_kv_map`, `too_many_arguments`, and `ptr_arg`.

- Observation: complete lanes still felt under-explained when the detail pane
  only showed artifact preview plus generic live-detail text.
  Evidence: the new red test
  `complete_lane_detail_surfaces_completed_result_summary` failed until the
  detail pane rendered an explicit completed-result block.

- Observation: Fabro's normal inspection summary only retains the most recent
  stage's `files_written`, which is often empty for successful runs because the
  final stages are `verify` or `exit`.
  Evidence: the real successful runs under `~/.fabro/runs/20260319-*` ended
  with `WorkflowRunCompleted`, but their useful artifact writes appeared in
  earlier `StageCompleted` events inside `progress.jsonl`.

- Observation: a human reviewer needs to see autodev cycle context in the TUI,
  not just lane status and recent run ids, to understand the end-to-end loop.
  Evidence: the new observer test now proves a detail pane section that shows
  autodev stop reason, cycle count, evolve target, and whether the selected
  lane was ready/dispatched in the last autodev cycle.

## Decision Log

- Decision: implement the observer as a new crate,
  `lib/crates/raspberry-tui/`, with `raspberry-cli` owning only CLI parsing and
  handoff.
  Rationale: the TUI has enough rendering and interaction state that it should
  not live inline in `raspberry-cli/src/main.rs`.
  Date/Author: 2026-03-19 / Codex

- Decision: the first version should remain read-only and use best-effort live
  run detail from the current Raspberry/Fabro bridge.
  Rationale: the visualization layer can ship before the stable run-truth
  adapter is finished as long as it is honest about stale or unavailable live
  data.
  Date/Author: 2026-03-19 / Codex

- Decision: optimize Phase 1 for the seeded Myosu-shaped fixture and current
  proving-ground programs rather than for arbitrary future plugins.
  Rationale: the operator value is immediate if the TUI works over the current
  program stack. Generalization can follow after the first trustworthy release.
  Date/Author: 2026-03-19 / Codex

- Decision: keep the detail pane additive by showing the selected artifact
  preview first and the lane's live run section underneath it.
  Rationale: this preserves artifact drilldown without forcing the operator to
  toggle modes just to inspect stale or unavailable run detail.
  Date/Author: 2026-03-19 / Codex

- Decision: add only one new observer helper to `raspberry-supervisor`,
  `ProgramManifest::resolve_lane_artifacts()`.
  Rationale: the TUI needed a shared way to resolve produced artifact paths,
  but the rest of the observer state already existed in `EvaluatedLane`.
  Date/Author: 2026-03-19 / Codex

- Decision: complete lanes should render a dedicated completed-result summary in
  the detail pane.
  Rationale: operators need a clear success story for finished work, not just
  a selected artifact preview and a generic live/stale section.
  Date/Author: 2026-03-19 / Codex

- Decision: recent successful runs should be matched to lanes by historical
  artifact writes from the full `progress.jsonl`, not by the final progress
  summary alone.
  Rationale: successful workflows often end with stages that write no files, so
  looking only at the final summary would miss the real artifact-producing run.
  Date/Author: 2026-03-19 / Codex

- Decision: surface durable autodev cycle state in the TUI detail/state panes
  instead of forcing the reviewer back to CLI output after an autodev run.
  Rationale: the user explicitly asked for an end-to-end reviewer view of the
  autodev process, and that requires more than lane-local artifact/run detail.
  Date/Author: 2026-03-19 / Codex

## Outcomes & Retrospective

The first observer slice now exists end-to-end. `raspberry tui --manifest ...`
starts a real alternate-screen terminal UI, renders four logical panes, lets
the operator move around with Vim-style keys, shows explicit missing-artifact
states, and marks live detail as stale or unavailable when the run bridge
cannot provide fresh truth.

The latest refinement makes completed work easier to read. Finished lanes now
render a dedicated completed-result summary that names the satisfied milestone
and the durable artifacts that represent the outcome.

The new refinement after that makes successful real runs visible too. When a
lane's curated artifacts match files written by recent succeeded Fabro runs, the
detail pane now shows those run ids and workflow names as a "Recent successful
runs" section even if Raspberry state never linked the run first.

The latest refinement makes the observer more useful for the upcoming autodev
launch. When an autodev loop has been run for a program, the TUI can now show
that loop's last-cycle summary alongside lane detail so a human reviewer can
see whether the selected lane was recently ready, dispatched, and part of an
evolve cycle.

The latest refinement after that makes the TUI materially more usable as a live
operator dashboard. Instead of a narrow four-column log wall, the default wide
layout now emphasizes:

- a left-side dashboard lane list grouped by `RUNNING`, `READY`, `BLOCKED`,
  `FAILED`, and `COMPLETE`
- top-level status counts for the visible program frontier
- a wider state pane with explicit overview, selected-lane, and check sections
- a detail pane that leads with operational summaries before dropping into raw
  artifact text

The next dashboard step is now clear from live use against the Myosu autodev
worktree: orchestration lanes still compress a whole child program into one
summary line. The operator needs one more layer:

- a deterministic child-program digest in the state/detail panes
- optionally, later, a cached MiniMax narration layer over that structured
  digest rather than an LLM in the render loop

The most useful follow-on work is no longer basic rendering. It is refinement:
better search ergonomics, deeper scrolling, and eventually replacing the
best-effort live section with a more authoritative Fabro inspection surface.

## Context and Orientation

The implementation lives entirely in this repository:

- `lib/crates/raspberry-cli/`
- `lib/crates/raspberry-supervisor/`

The first proving-ground input is already present in this repo:

- `test/fixtures/raspberry-supervisor/myosu-program.yaml`

The observer TUI should compose these sources:

1. manifest structure
2. evaluated lane state
3. curated outputs or fixture artifact roots
4. best-effort live run detail

It must not create a second source of truth.

## Milestones

### Milestone 1: New crate and CLI subcommand

At the end of this milestone, the Fabro workspace contains a new
`raspberry-tui` crate and the `raspberry` CLI recognizes a `tui` subcommand.
The proof is that `cargo check -p raspberry-cli` and
`cargo check -p raspberry-tui` both pass.

This milestone is complete.

### Milestone 2: Pane model and navigation

At the end of this milestone, the TUI can render a four-pane layout, move focus
with Vim-style keys, and collapse/expand panes. The proof is fixture-backed
unit tests plus one manual run against the Myosu-shaped fixture manifest.

This milestone is complete.

### Milestone 3: Artifact and lane drilldown

At the end of this milestone, selecting a lane shows its state and lists its
curated artifacts, and selecting an artifact shows file contents or a missing
state. The proof is a manual run where available fixture artifacts are visible
and missing artifacts in another lane are explicit.

This milestone is complete.

### Milestone 4: Best-effort live run detail

At the end of this milestone, the detail pane can show live stage/run
information when available and mark it as unavailable or stale otherwise. The
proof is a fixture-backed rendering test plus a manual run against a manifest
with live state.

This milestone is complete for the current best-effort run-truth adapter.

## Plan of Work

Add a new crate named `raspberry-tui` under `lib/crates/`. This crate should
own:

- application state
- pane focus and collapse state
- Vim-style key handling
- terminal rendering
- file viewing helpers for curated outputs

Keep `raspberry-cli` thin. Add a new `Commands::Tui(TuiArgs)` variant and a
small `run_tui()` handoff that loads the manifest path and starts the TUI.

Reuse `raspberry-supervisor` as the data source. If the TUI needs a helper that
does not exist yet, add it to `raspberry-supervisor` only if it is generally
useful to all observer surfaces. Do not duplicate evaluation logic in the TUI
crate.

The first phase should target one-program-at-a-time viewing. Do not add
multi-program switching, dispatch controls, or edit actions in this slice.

## Concrete Steps

Work from the repository root.

1. Add the new crate and shared dependencies.

   Modify:
   - `Cargo.toml`

   Create:
   - `lib/crates/raspberry-tui/Cargo.toml`
   - `lib/crates/raspberry-tui/src/lib.rs`
   - `lib/crates/raspberry-tui/src/app.rs`
   - `lib/crates/raspberry-tui/src/layout.rs`
   - `lib/crates/raspberry-tui/src/keys.rs`
   - `lib/crates/raspberry-tui/src/render.rs`
   - `lib/crates/raspberry-tui/src/files.rs`

   Add workspace dependencies for:
   - `ratatui`
   - `crossterm`

2. Add the CLI subcommand.

   Modify:
   - `lib/crates/raspberry-cli/src/main.rs`
   - `lib/crates/raspberry-cli/Cargo.toml`

   The new CLI surface should accept:

       raspberry tui --manifest <path>

3. Implement the pane and navigation model.

   The first version should support:
   - left/right pane focus: `h`, `l`
   - up/down selection: `j`, `k`
   - top/bottom: `gg`, `G`
   - open detail: `Enter`
   - collapse/fold: `za`, `zo`, `zc`, `zR`, `zM`
   - search/filter entry: `/`
   - refresh: `r`
   - quit: `q`

4. Render lane overview and artifact drilldown.

   The selected lane should show:
   - lane key and title
   - status
   - proof profile
   - preconditions / proof state / operational or orchestration state if present
   - artifact list for the lane's output root

   The detail pane should:
   - render file contents for selected artifacts
   - show explicit "missing" state when files are absent
   - show live run detail when available

5. Add tests and one manual proof loop.

   Add unit tests in:
   - `lib/crates/raspberry-tui/tests/`
   - `lib/crates/raspberry-cli/tests/cli.rs`

   Manual check:

       cargo run -p raspberry-cli -- tui --manifest test/fixtures/raspberry-supervisor/myosu-program.yaml

## Validation and Acceptance

Run these commands from the repository root:

    cargo check -p raspberry-tui
    cargo check -p raspberry-cli
    cargo clippy -p raspberry-supervisor -p raspberry-tui -p raspberry-cli -- -D warnings
    cargo test -p raspberry-tui
    cargo test -p raspberry-cli
    cargo test -p raspberry-supervisor

Then manually run:

    cargo run -p raspberry-cli -- tui --manifest test/fixtures/raspberry-supervisor/myosu-program.yaml

Acceptance is complete when:

- the `tui` subcommand starts successfully
- the operator can move focus with `h` / `l`
- the operator can move selection with `j` / `k`
- the operator can collapse and expand panes with Vim-style fold commands
- a lane with available artifacts exposes those files in the detail pane
- a lane without outputs shows explicit missing-artifact state
- live run detail is shown when present and marked unavailable/stale when not

## Idempotence and Recovery

This slice is additive. If the TUI implementation becomes unstable, the
existing `plan`, `status`, `watch`, and `execute` commands must remain
available and unchanged.

If a rendering or input bug blocks the TUI, a contributor should be able to:

1. remove the `tui` subcommand wiring from `raspberry-cli`
2. keep the `raspberry-tui` crate compiling in isolation
3. iterate there without destabilizing the rest of Raspberry

## Artifacts and Notes

Important proving inputs:

    test/fixtures/raspberry-supervisor/myosu-program.yaml

Verification completed during this implementation:

    cargo check -p raspberry-tui
    cargo check -p raspberry-cli
    cargo clippy -p raspberry-supervisor -p raspberry-tui -p raspberry-cli -- -D warnings
    cargo test -p raspberry-tui
    cargo test -p raspberry-cli
    cargo test -p raspberry-supervisor
    cargo run -p raspberry-cli -- tui --manifest test/fixtures/raspberry-supervisor/myosu-program.yaml

The manual proof used a PTY, confirmed that the observer rendered the four-pane
surface over the Myosu fixture, and exited cleanly on `q`.

An extended manual proof may later use a real external proving-ground manifest,
such as the Myosu repository's current program stack, but that is not required
for the first Fabro-owned implementation slice.

## Interfaces and Dependencies

Expected interface ownership after this slice:

- `raspberry-cli` owns the command-line entrypoint
- `raspberry-tui` owns terminal rendering and interaction
- `raspberry-supervisor` owns manifest loading and evaluated lane state

The TUI must remain read-only in Phase 1. Execution and writes stay in the
existing Raspberry/Fabro flows until the run-truth adapter is stronger.

Revision note (2026-03-19): Updated this ExecPlan after implementation to
record the new `raspberry-tui` crate, the thin supervisor helper, the lint
cleanup needed to satisfy `-D warnings`, the completed-result detail pane
improvement, the recent successful-run matching behavior, and the exact
verification evidence for the observer slice.

Revision note (2026-03-19, later): The dashboard selection model now treats
the grouped, filtered program list as the single source of truth for `j` / `k`
navigation, top/bottom jumps, and selected-lane fallback. Added regression
coverage for grouped-order navigation and filtered-order navigation with a stale
hidden selection. Also gated plain-English narration refresh behind
`RASPBERRY_TUI_ENABLE_NARRATION=1` so the TUI stays cache-first by default and
test/manual startup does not block on live model calls. The child-program
digest observer proof now uses the lightweight portfolio fixture instead of the
live Myosu worktree so the `raspberry-tui` package tests stay deterministic and
fast.

Revision note (2026-03-19, async narration): When narration is enabled, the TUI
now loads any cached operator summary immediately and refreshes it on a
background worker. The main loop polls for completed narration results on a
short tick rather than blocking inside `App::load` or the render path. This
keeps startup responsive while still allowing lane-selection and manual-refresh
driven summaries to converge in the background.
