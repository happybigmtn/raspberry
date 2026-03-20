# Theory: Raspberry Run Observer TUI

## Problem

Raspberry can already explain a program in plain text through `plan`, `status`,
and `watch`, but the operator experience breaks down once someone needs to
compare lane state, curated artifacts, and best-effort Fabro run detail at the
same time. The current information is present in
`lib/crates/raspberry-supervisor/`, but it is spread across text tables, JSON
state files, and output directories. The missing piece is not another source of
truth. It is a read-only terminal surface that composes the existing sources
honestly.

## Operating Theory

The core leverage point is the split that already exists between
`raspberry-cli` and `raspberry-supervisor`. The supervisor crate already knows
how to load a manifest, evaluate lane state, resolve run metadata, and read the
program runtime state. That means the TUI should stay thin on domain logic and
focus on presentation, navigation, and file inspection. The only supervisor
helper that turned out to be necessary was
`ProgramManifest::resolve_lane_artifacts()`, which keeps the artifact lookup
rules in the manifest layer instead of duplicating them in the TUI.

The TUI itself should treat the selected lane as the center of the world. Once a
lane is selected, every other pane can derive from it: summary state from
`EvaluatedLane`, curated files from the manifest's produced artifacts, and live
detail from the lane's run metadata. This keeps the rendering model stable even
when live run truth is partial or stale. That theory held up in the finished
implementation: the app state is mostly selection, focus, folding, and filter
state, while the data model remains owned by the supervisor crate.

## Strategy

The implementation has now landed in the same layered shape that the initial
theory predicted. `raspberry-cli` owns parsing and handoff. `raspberry-tui`
owns the interaction model, rendering, and artifact previewing. The supervisor
crate remains the source of truth for manifests, evaluated lanes, and resolved
artifact paths. The key strategic choice was to keep the observer read-only and
additive, which made it possible to prove real operator value without inventing
new execution or mutation surfaces.

The constraint to keep in view is honesty around live data. If a lane is not
actively running, the TUI should present the run section as stale or unavailable
rather than pretending it is current. The finished detail pane follows that
rule directly: it always shows the selected artifact preview first, adds a
dedicated completed-result block for finished lanes, and then renders a live
section labeled `live`, `stale`, or `unavailable`. It now also adds a
best-effort "Recent successful runs" section by matching a lane's curated
artifact paths against historical `progress.jsonl` writes from recent succeeded
Fabro runs.

## Key Discoveries

Initially the likely risk was that the observer would need a large new adapter
layer in `raspberry-supervisor`, but the implementation proved that one narrow
helper was enough. `evaluate.rs`, `manifest.rs`, and `program_state.rs` already
contained the necessary status, proof, orchestration, operational, and run
fields, so the TUI could stay presentation-focused.

Another important discovery is that the Myosu fixture already exercises the
interesting cases the TUI needs: present artifacts, missing artifacts, a running
service lane, a blocked orchestration lane, and a failed interface lane with a
recorded error. That makes fixture-backed testing realistic without introducing
new fake data.

A third discovery is that keeping the detail pane additive is better than
mode-switching. Showing the selected artifact and the run section together made
the first read-only observer easier to understand and easier to test.

Another discovery is that finished work still needs explicit narration. Artifact
preview alone does not tell the operator what completed, so the detail pane now
spells out the satisfied milestone and the durable outputs for complete lanes.

One more discovery came from the real `01KM…` runs: Fabro's ordinary inspection
summary is too lossy for this observer use case because the final successful
stages often write no files. Matching lanes to runs therefore has to read the
full `progress.jsonl` history, not just the last progress snapshot.

A final implementation discovery is that the repo's lint bar matters to the
shape of the work. Making the new TUI crate clippy-clean also required cleaning
older Raspberry lint findings, which improved the local baseline and reduced the
chance that the observer would ship on top of avoidable technical debt.

## Open Questions

The biggest remaining question is not architectural anymore. It is how far to
push operator ergonomics in the next slice: better search behavior, more
intentional scrolling, and perhaps a stronger notion of selected detail modes if
the combined artifact-plus-live pane becomes crowded in larger programs.

The other open question sits outside the TUI crate. The current live section is
honest, but it is still only as good as the existing Fabro-to-Raspberry bridge.
Once a stronger inspection surface exists, the TUI should adopt it without
changing the interaction model it now has.
