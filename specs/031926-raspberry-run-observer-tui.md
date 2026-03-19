# Specification: Raspberry Run Observer TUI

Status: Draft
Date: 2026-03-19
Type: Capability Spec

## Purpose / User-Visible Outcome

After this capability lands, a human operator can open a terminal user
interface over a Raspberry program manifest and watch lanes change in real
time. They can move through the program with Vim-style keys, collapse and
expand panes, inspect lane state, and drill into curated outputs and live
stage context without reading raw JSON files by hand.

The first user-visible command should be a Raspberry CLI entrypoint such as:

    raspberry tui --manifest test/fixtures/raspberry-supervisor/myosu-program.yaml

This TUI is read-only in its first phase. It exists to make the current
Raspberry control plane observable and usable by humans before adding write or
dispatch controls.

## Whole-System Goal

Current state:

- Raspberry already exposes text surfaces like `plan`, `status`, and `watch`.
- `raspberry-supervisor` already models units, lanes, milestones, proof
  profiles, checks, and best-effort live state.
- Fabro run truth exists, but the stable Fabro-to-Raspberry run-truth adapter
  is not finished yet.
- This repository already contains a Myosu-shaped fixture manifest at
  `test/fixtures/raspberry-supervisor/myosu-program.yaml`, which is enough to
  prove the first observer surface without depending on an external repo.

This capability adds:

- a terminal UI for Raspberry program observation
- Vim-style navigation over units, lanes, outputs, and live state
- collapsible panes so the operator can move between overview and detail
- graceful fallback when live run truth is partial or stale

If this capability lands:

- a human can inspect a Raspberry program frontier from one TUI
- curated lane artifacts become directly browsable
- live lane state is understandable without reading raw `status.json`,
  `state.json`, or `progress.jsonl`

Still not solved here:

- write/execute controls inside the TUI
- authoritative Fabro run-truth integration beyond the current adapter surface
- a web UI equivalent

12-month direction:

- Raspberry has both text and rich TUI operator surfaces
- the TUI can switch between multiple programs and compare frontier health
- live run detail comes from a stable Fabro inspection surface rather than raw
  run-directory heuristics

## Scope

In scope:

- read-only Raspberry observer TUI
- one CLI entrypoint under the Raspberry CLI surface
- Vim-style navigation and folding
- pane-based layout with collapsible regions
- manifest-aware overview of units and lanes
- output-file browsing for curated artifacts
- live lane-state summary, including proof/precondition/operational state where
  available
- graceful stale/offline handling when live run truth is unavailable

Out of scope for the first capability:

- editing files from the TUI
- dispatching `execute` from inside the TUI
- approving human gates from inside the TUI
- building a replacement for the Fabro web UI
- inventing a second source of truth outside Raspberry program manifests and
  curated outputs

## Current State

The Raspberry layer is now stable enough for a visualization surface if we are
careful about boundaries:

- `raspberry-supervisor` already exposes manifest and evaluated lane state
- `raspberry-cli` already owns the `plan`, `status`, `watch`, and `execute`
  command surface
- the Myosu-shaped fixture manifest already expresses platform, service,
  orchestration, and interface lanes in one proving-ground program

The main caveat is still live run truth. The current Raspberry layer can infer
live Fabro runs, but the bridge is still a temporary adapter. That means this
TUI should treat live stage/run detail as best-effort and make stale state
obvious rather than pretending it is authoritative.

## Architecture / Runtime Contract

### Operator Command

The first durable user entrypoint should be a Raspberry CLI subcommand:

    raspberry tui --manifest <path>

The TUI should take the same manifest path that `raspberry plan/status/watch`
already use.

### Planned Home

This capability belongs in the Raspberry layer. The expected implementation
shape is:

- `raspberry-cli` owns the `tui` command
- `raspberry-supervisor` remains the source of manifest and evaluated lane
  state
- a dedicated observer module or crate owns terminal rendering and keyboard
  interaction

### Data Sources

The observer TUI should compose four read-only sources:

1. **Program manifest**
   Source of truth for units, lanes, milestones, proof profiles, and expected
   output roots.

2. **Evaluated lane state**
   Source of truth for status categories such as blocked, ready, running,
   complete, failed.

3. **Curated outputs**
   Read directly from the lane output roots so operators can inspect durable
   artifacts like `spec.md`, `review.md`, `implementation.md`, and
   `verification.md`.

4. **Best-effort live run detail**
   Current run id, current stage label, last completed stage, and recent file
   telemetry when available from Raspberry/Fabro bridging.

### Pane Model

The first version should use four logical panes:

1. **Program pane**
   Tree view of units and lanes.

2. **State pane**
   Lane summary: status, proof profile, preconditions, proof state,
   operational/orchestration state.

3. **Artifacts pane**
   Lists curated output files for the selected lane and shows their presence or
   absence.

4. **Detail pane**
   Shows either artifact contents or live run detail for the selected lane.

Each pane must be collapsible so the operator can move between:

- compact overview
- one-pane focus
- balanced multi-pane inspection

On wide terminals, the default open-pane presentation should favor a dashboard
composition instead of a flat four-column split:

- program/dashboard pane on the left
- state pane across the upper-right
- artifacts and detail across the lower-right

The layout should bias toward preserving readable width for state and detail
text over keeping all panes equal width.

### Navigation Contract

The operator model should be Vim-like:

- `h` / `l` move pane focus left/right
- `j` / `k` move selection down/up
- `gg` jump to top of current list
- `G` jump to bottom of current list
- `Enter` open selected lane or artifact detail
- `Tab` cycles focus when Vim-style lateral movement is ambiguous
- `za` toggles collapse on the focused pane
- `zo` opens the focused pane
- `zc` closes the focused pane
- `zR` opens all panes
- `zM` closes all secondary panes
- `/` starts filtering/search
- `r` refreshes the current view
- `q` quits

The first release does not need every Vim command, but the ones above should be
treated as the intended stable contract.

### State Presentation Rules

- blocked lanes should say what they are waiting on
- running lanes should show current stage and health if available
- failed lanes should show last error and last failed proof or check
- complete lanes should show which milestone is satisfied
- the overview must clearly differentiate active work from completed work at a
  glance, not only through inline prose
- missing outputs should be obvious, not silently ignored
- stale live data should be marked as stale

The program pane should behave like a dashboard list, not a raw manifest dump:

- show whole-program counts for running, ready, blocked, failed, complete
- group lanes by status in operator-priority order
- make currently running work visually stronger than completed work

The state pane should be structured enough that an operator can answer:

- what is selected
- what state it is in
- what checks are currently passing or failing
- what run/autodev context is attached to it

### Minimum Terminal Behavior

- if the terminal is too small, show a clear size warning instead of rendering
  broken panes
- the interface should degrade to fewer panes before becoming unusable
- no mouse dependency is required

## Adoption Path

### Phase 1: Read-only fixture and proving-ground observer

Render one manifest at a time with unit/lane tree, lane summary, and artifact
viewer. Best-effort live run detail is optional but should be surfaced if
present.

### Phase 2: Live stage drilldown

Use the stabilized Fabro-to-Raspberry adapter to show authoritative run ids,
stage progression, and recent output context.

### Phase 3: Multi-program operator mode

Allow switching between programs and comparing frontier health without
restarting the command.

## Acceptance Criteria

The first capability is acceptable when a contributor can:

1. Run:

       raspberry tui --manifest test/fixtures/raspberry-supervisor/myosu-program.yaml

2. See a lane tree for the Myosu-shaped proving-ground program.

3. Move between lanes using `j` / `k`, change pane focus with `h` / `l`, and
   collapse panes with `za`.

4. Open a lane with artifacts and read them in the detail pane.

5. Open a lane without outputs yet and see an explicit missing-artifact state.

6. Open a running or blocked lane and see the current state summary without
   dropping into raw JSON files.

An extended manual proof may also use a real external proving-ground manifest
such as the Myosu repository's current `fabro/programs/myosu-bootstrap.yaml`,
but that is not required for the first Fabro-owned capability.

## Failure Handling

- If the manifest cannot be loaded, fail fast with a readable error.
- If a lane output file is missing, show a missing-artifact entry in the
  artifacts pane and keep the TUI running.
- If live run detail cannot be loaded, mark the detail pane as unavailable or
  stale, but keep the static manifest and output panes usable.
- If the terminal is too small, show a minimum-size message and let the user
  resize or quit.
