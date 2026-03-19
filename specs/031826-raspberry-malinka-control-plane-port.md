# Specification: Raspberry Malinka Control Plane Port

Status: Draft
Date: 2026-03-18
Type: Migration / Port Spec
Supersedes: the older assumption that the default worker runtime should be
rebuilt inside the Malinka repository

## Purpose / User-Visible Outcome

After this migration, Raspberry becomes the place where both execution and
supervision live together. A contributor can run one repository and get the
production-grade staged execution model from Fabro core together with the
higher-level supervisory behavior that made Malinka valuable: program
manifests, lane readiness, recurring draft-to-canonical compilation, trust
policy, adjudication, and landing control.

The immediate user-visible effect is not a new end-user feature. It is a cleaner
product boundary: Raspberry owns the execution substrate, and Malinka logic is
ported into Raspberry as layered control-plane crates and binaries rather than
reimplemented in a weaker repo.

## Whole-System Goal

Current state:

- Fabro already provides the strongest execution substrate in practice: staged
  workflow runs, checkpoints, retries, observability, and durable run truth.
- Malinka already provides the strongest high-level supervisory ideas in
  practice: program manifests, lane readiness, operator truth, recurring
  compilation, trust policy, adjudication, and landing rules.
- Those two strengths currently live in separate repos, joined only by an
  external bridge.

This spec adds:

- a Raspberry-first repo strategy
- a port inventory for Malinka control-plane concepts
- a clean boundary between generic Fabro core and Raspberry-specific control
  plane
- phased migration rules and parity gates

If this spec lands:

- Raspberry becomes the primary execution and control-plane repository
- the old Malinka repo stops being the place where the default runtime is built
- Fabro runtime mechanics are no longer duplicated in a second codebase

Still not solved here:

- the exact crate-by-crate implementation of every ported subsystem
- upstream contribution strategy for generic improvements that should flow back
  to Fabro
- long-term branding between "Fabro", "Raspberry", and any retained "Malinka"
  CLI surface

12-month direction:

- a single Raspberry repository with a stable Fabro-derived execution kernel,
  layered Raspberry supervisory crates, and no dependence on the old Malinka
  repo for default runtime behavior

## Current State

- This repository already contains the Fabro execution substrate.
- `PLANS.md` now defines how execution plans should be written.
- No `specs/` or `plans/` convention existed in this repo before this document.
- The old Malinka repo proved out several control-plane ideas, but those ideas
  are not yet native to Raspberry.

## Terms

An **execution substrate** is the code that actually runs staged work: graphs,
retries, checkpoints, sandboxes, run truth, and tool-using agents.

A **control plane** is the code that decides what should run, in what order,
under what policy, and what counts as safe, proven, and ready to land.

A **recurring compiler** is the subsystem that takes permissive
model-authored draft outputs and turns them into trusted canonical artifacts
that the repo is willing to treat as real state.

A **lane** is one independently schedulable stream of work inside a larger
program, such as chapter generation or page assembly.

A **milestone** is a durable lifecycle checkpoint for a lane or unit, such as
`reviewed` or `publish_ready`.

## Target Architecture

Raspberry should have two layers:

1. Fabro core, which stays generic and owns:
   - staged workflow execution
   - checkpoints and run truth
   - sandboxes and execution backends
   - context / fidelity / graph mechanics
   - generic workflow CLI and API

2. Raspberry control plane, which is layered on top and owns:
   - program manifests, units, lanes, and milestones
   - supervisory dispatch policy
   - recurring draft-to-canonical compilation
   - trust, proof, adjudication, and landing gates
   - operator surfaces over program state

The key rule is that Raspberry-specific policy should not be smeared through
generic Fabro core crates when a layered crate or binary can own it cleanly.

## Upstream Mergeability

Raspberry must remain able to absorb new upstream Fabro functionality on a
regular basis. This is a thin-fork strategy, not a hard fork.

That means:

- generic execution-plane behavior remains upstream-shaped whenever possible
- Raspberry-specific behavior should live in additive `raspberry-*` crates,
  binaries, manifests, and docs
- improvements that are generally useful to Fabro should be written so they can
  be upstreamed cleanly
- major upstream features, such as server mode, should be adopted into
  Raspberry rather than reimplemented in a competing substrate

Every significant change should be classified as one of three types:

1. **Upstream candidate** — generic Fabro improvements that should ideally land
   upstream as well
2. **Raspberry layer** — supervisory, trust, recurring, or operator logic that
   is specific to Raspberry
3. **Temporary bridge** — migration glue with an explicit future removal point

If a proposed change would make upstream sync materially harder by deeply
rewriting generic Fabro core for Raspberry-specific reasons, the default answer
should be "do not do that here".

## What Ports Directly

The following concepts should move into Raspberry with minimal conceptual
change:

- program supervision over units, lanes, and milestones
- durable program runtime state and watch/status surfaces
- queue and session truth for admitted work
- trust and proof enforcement around control writes and landing
- recurring draft-to-canonical compilation
- operator views that summarize program truth for humans
- planning and selection logic for what runs next

## What Ports Selectively

The following concepts are worth extracting, but not preserving in the same
shape:

- shared-tree lease ideas
- recovery logic that assumes a control plane above the execution substrate
- doctrine/company overlays that attach policy metadata to control artifacts
- any existing glue that mixes supervisory policy with the old execution
  runtime model

## What Does Not Port

The following should not become primary Raspberry systems:

- Malinka's old engine adapter layer
- monolithic worker-turn mechanics
- heavy per-worker runtime homes and scratch-tree execution
- any second implementation of staged graph execution that duplicates Fabro
  core

If Raspberry needs a capability that Fabro core already has, the default move
is to reuse or extend Fabro core, not recreate it in a Raspberry layer.

## First-Class Migration Goals

### RMP-01: Raspberry can supervise external Fabro-style programs natively

Raspberry must gain its own native supervisory surface over existing program
manifests and lane state. The initial target is parity with the current bridge:
plan, status, watch, and bounded execute over multi-lane programs.

### RMP-02: Raspberry owns durable program truth

Program state, lane runtime records, summaries, and operator views must live in
Raspberry-native state surfaces rather than depending on an external repo for
their supervisory model.

### RMP-03: Raspberry ports recurring draft-to-canonical compilation

Recurring work must become a Raspberry-owned compiler layer that reads drafts,
produces canonical artifacts, and enforces trust semantics without asking the
model to author canonical truth directly.

### RMP-04: Raspberry ports trust, proof, adjudication, and landing gates

The stronger Malinka semantics around what may write, what must be proven, and
what can land must survive the migration as a layered policy plane.

### RMP-05: The old Malinka repo can freeze without blocking Raspberry

The migration is only complete when Raspberry can continue evolving without the
old Malinka repo being the primary place where runtime behavior is designed or
implemented.

## Transitional Bridges

During migration, Raspberry may temporarily include bridge code that still
supervises external Fabro-native run configs and workflows. That is acceptable
as long as the bridge is explicitly transitional and points toward Raspberry as
the primary home of the control plane.

The bridge is a means of bootstrapping parity, not the target architecture.
It must not become a second long-lived execution substrate inside Raspberry.

## Generalization Requirement

The first proven bridge in the old Malinka repo was shaped around a technical
book program. That is useful evidence, but it is not a sufficient target
architecture. Raspberry supervision must generalize to any repository that
needs staged execution under a larger operating model.

The first proving ground for that generalization should be a broad repository
like Myosu rather than another documentation-only program. A broad repo in this
context means:

- multiple technical domains in one repo
- code, operations artifacts, and doctrine artifacts all matter
- execution units may represent services, crates, subsystems, or operator
  lanes rather than only content chapters
- the program model must not assume that output is just "write a chapter" or
  "render a page"

Any `fabro_dispatch` port that preserves book-specific assumptions instead of
abstracting them into repo-agnostic concepts is incomplete.

## Migration Phases

### Phase 1: Bootstrap supervisory parity inside Raspberry

Create Raspberry-native crates and binaries that can supervise existing
Fabro-style programs with plan, status, watch, and execute semantics.

### Phase 2: Port recurring compiler and trust plane

Move recurring draft-to-canonical compilation and landing/trust rules into
Raspberry-native crates.

### Phase 3: Make Raspberry the primary control-plane repo

Stop designing new default runtime behavior in the old Malinka repo. New work
targets Raspberry first.

### Phase 4: Freeze old Malinka as migration reference

Retain the old repo only as a source of historical semantics, parity checks, or
documentation until it is no longer needed.

## Parity Gates

Before Phase 3:

- Raspberry must supervise real multi-lane programs without depending on the
  old Malinka repo at runtime.
- Raspberry must expose operator truth for those programs through native
  commands and state files.
- The first recurring compiler path must run in Raspberry-native code.
- Upstream Fabro sync must still be straightforward enough that a new upstream
  execution-plane feature can be adopted without architectural surgery.

Before Phase 4:

- Raspberry must own the default control-plane entrypoint
- the old Malinka repo must no longer be the primary place for runtime design
- any remaining bridge code must be explicitly marked as compatibility-only

## Non-goals

- rewriting Fabro core into Raspberry under a different name
- pushing Raspberry-specific trust policy into generic Fabro workflow crates
- completing every Malinka port in one implementation slice
- preserving the old Malinka runtime mechanics merely because they already
  exist
- forking major upstream Fabro features, such as server mode, instead of
  consuming them

## First Implementation Slice

The first canonical implementation slice for this migration is:

- bootstrap Raspberry-native supervision over existing Fabro-style programs
- keep Fabro core untouched except for generic improvements
- do not port recurring compilation or landing policy in the same slice

The next canonical slice after bootstrap is:

- port and generalize the proven `fabro_dispatch` supervisory semantics
- remove book-shaped assumptions from the manifest and state model
- use a larger proving ground, such as Myosu, to validate that the control
  plane scales beyond documentation workflows

That slice is defined by the accompanying plan in
`plans/031826-bootstrap-raspberry-supervisory-plane.md`.
