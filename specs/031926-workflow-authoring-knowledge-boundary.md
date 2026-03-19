# Decision: Workflow Authoring Intelligence Lives in `fabro-create-workflow`

Status: Draft
Date: 2026-03-19
Type: Decision Spec

## Decision

The first-class home for Fabro and Raspberry workflow-authoring knowledge is
the built-in `skills/fabro-create-workflow/` package and the files it embeds
through `lib/crates/fabro-cli/src/skill.rs`.

That means:

- the existing `fabro-create-workflow` skill remains the canonical bundled
  authoring package
- Raspberry-specific lane and control-plane guidance extends that skill instead
  of creating a separate default skill
- any later automation surface such as a TOML-driven graph generator, CLI
  command, or API endpoint must consume or derive from the same repository-local
  references instead of inventing a second body of heuristics in Rust or in an
  unrelated prompt bundle

## Why Now

The immediate user need is not only "generate a graph." The harder problem is
"encode the right graph-authoring judgment" for two layers at once:

- Fabro requirements: DOT syntax, validator rules, run-config semantics, model
  precedence, and workflow topology patterns
- Raspberry requirements: lane kinds, managed milestones, proof profiles,
  artifact production, dependency shapes, and the way the supervisor consumes
  per-lane run configs

This repository already ships a built-in workflow-authoring skill:

- `skills/fabro-create-workflow/SKILL.md`
- `lib/crates/fabro-cli/src/skill.rs`
- `fabro skill install`

That package is already the user-facing install surface for assistant-driven
workflow authoring. Improving it has immediate value for both Codex and Claude
users without adding a new runtime feature first.

The current embedded skill package also shows that the knowledge source needs a
refresh before more automation is layered on top of it. For example, the
bundled `skills/fabro-create-workflow/references/dot-language.md` still speaks
about "Arc Workflows" even though the current docs and CLI surface are Fabro.

There is another important boundary signal in the Raspberry fixtures:
`test/fixtures/raspberry-supervisor/myosu-program.yaml` expresses most of the
meaningful Raspberry requirements in the program manifest, while the associated
lane run configs are still placeholders. That means the first authoring surface
must teach agents how Fabro graphs and run configs should fit a Raspberry lane
contract; it should not pretend that the contract already lives inside a rich
run-config schema.

## Alternatives Considered

### 1. Build a TOML-driven graph generator first

This would create a new config shape similar to a run config and let users
invoke graph generation directly from TOML.

Why not first:

- it would force heuristics into code before the heuristics are stabilized
- it would create a second knowledge source unless carefully designed
- it would raise the implementation surface area before proving what the
  canonical prompt/reference corpus should be

This remains a valid follow-on capability after the authoring knowledge package
is corrected and extended.

### 2. Create a separate Raspberry-only workflow-authoring skill

This would isolate Raspberry guidance in a new package such as
`raspberry-create-workflow`.

Why not first:

- it duplicates Fabro core syntax and topology guidance
- it creates an ambiguous install story for users
- it encourages drift between generic Fabro rules and Raspberry overlays

Raspberry-specific guidance belongs as an extension layer within the existing
workflow-authoring skill unless future evidence proves the audiences are
meaningfully different.

### 3. Keep using ad hoc prompts plus repo docs

This costs nothing immediately.

Why not:

- it leaves no shipped authoring contract
- it produces inconsistent graphs across users and sessions
- it gives later automation work no stable corpus to build on

## Consequences

Positive consequences:

- one canonical authoring package for Fabro and Raspberry workflow generation
- `fabro skill install` becomes more useful immediately
- follow-on generator work can reuse a repo-local corpus instead of re-deriving
  heuristics from scattered docs and specs

Tradeoffs:

- the first slice improves authoring knowledge rather than adding deterministic
  generator automation
- some Raspberry guidance will remain advisory until later validator or product
  work makes those rules machine-checkable
- future generator work must be disciplined about reading from the skill
  references instead of bypassing them for convenience

## What Is Now Superseded

This supersedes the implicit assumption that the first Raspberry-aware graph
generation feature should begin as a new TOML surface or a separate skill.

It does not supersede:

- `specs/031826-raspberry-malinka-control-plane-port.md`
- the existing Fabro run-config and workflow docs

Instead, it defines where the authoring intelligence for those systems should
live before new automation surfaces are added.
