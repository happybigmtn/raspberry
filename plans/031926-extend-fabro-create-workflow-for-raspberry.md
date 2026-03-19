# Extend `fabro-create-workflow` for Raspberry-aware authoring

This ExecPlan is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

`PLANS.md` is checked into the repository root and this document must be
maintained in accordance with it. This plan depends on
`specs/031926-workflow-authoring-knowledge-boundary.md` and
`specs/031826-raspberry-malinka-control-plane-port.md`.

## Purpose / Big Picture

After this slice lands, a contributor can install one built-in skill and use it
to author Fabro workflows that also respect Raspberry control-plane needs.
Given a prompt, plan, or Raspberry program-manifest context, the skill should
help an assistant produce a workflow graph and run config that are structurally
valid for Fabro and aligned with Raspberry lane semantics such as managed
milestones, produced artifacts, proof posture, and dependency shape.

The user-visible proof is:

    fabro skill install --for project --dir claude --force

After installation, the exported `fabro-create-workflow` package should contain
Raspberry-aware references and instructions. A contributor should be able to
point an assistant at `test/fixtures/raspberry-supervisor/myosu-program.yaml`
and get a lane-oriented workflow proposal without inventing custom prompting
from scratch.

This first slice is intentionally skill-first. It does not add a TOML-invoked
generator yet. Instead, it establishes the canonical authoring corpus that a
later generator can reuse.

## Progress

- [x] (2026-03-19 06:18Z) Reviewed the current built-in skill package in
  `skills/fabro-create-workflow/` and the installer embedding in
  `lib/crates/fabro-cli/src/skill.rs`.
- [x] (2026-03-19 06:20Z) Reviewed Fabro workflow and run-config docs, validator
  rules, and Raspberry supervisor manifests to identify where the authoring
  knowledge currently lives.
- [x] (2026-03-19 06:23Z) Wrote the boundary decision spec
  `specs/031926-workflow-authoring-knowledge-boundary.md`.
- [x] (2026-03-19 06:27Z) Refreshed the bundled Fabro authoring skill and core
  references to match current Fabro vocabulary and runtime behavior.
- [x] (2026-03-19 06:29Z) Added Raspberry-specific authoring guidance and
  examples focused on the repo obligations needed by the supervisory plane.
- [x] (2026-03-19 06:30Z) Extended the skill installer embed list so the new
  Raspberry references ship through `fabro skill install`.
- [x] (2026-03-19 06:30Z) Updated the CLI docs to describe the expanded scope
  of the built-in skill.
- [x] (2026-03-19 06:32Z) Ran installer tests, formatting checks, and a smoke
  install verifying that the Raspberry references are exported to disk.

## Surprises & Discoveries

- Observation: the built-in skill package is already a shipped product surface,
  not just repo documentation.
  Evidence: `lib/crates/fabro-cli/src/skill.rs` embeds specific files from
  `skills/fabro-create-workflow/` and `fabro skill install` writes them back
  out to `.claude/skills/` or `.agents/skills/`.

- Observation: the current skill bundle has drift from the authoritative Fabro
  docs.
  Evidence: `skills/fabro-create-workflow/references/dot-language.md` still
  starts with "DOT Language Reference for Arc Workflows" while the current docs
  live under `docs/reference/dot-language.mdx` and the CLI/docs consistently
  refer to Fabro workflows.

- Observation: the richest Raspberry authoring requirements are currently
  encoded in the supervisor manifest, not in per-lane run configs.
  Evidence: `test/fixtures/raspberry-supervisor/myosu-program.yaml` contains
  lane kind, `managed_milestone`, `proof_profile`, dependencies, checks, and
  artifact production, while the fixture TOML files in
  `test/fixtures/raspberry-supervisor/run-configs/` are placeholder comments.

- Observation: Raspberry guidance therefore has to be manifest-aware and
  artifact-aware, not only run-config-aware.
  Evidence: the supervisor evaluates readiness and completion from manifest
  fields such as `produces`, milestone requirements, and lane checks in
  `lib/crates/raspberry-supervisor/src/manifest.rs` and
  `lib/crates/raspberry-supervisor/src/evaluate.rs`.

- Observation: the most useful Raspberry framing is a repo-readiness checklist,
  not just a new graph template.
  Evidence: the new `raspberry-authoring.md` reference became much clearer once
  it was organized around the supervisory-plane questions "what work exists",
  "what proves success", "what blocks readiness", "what proves correctness",
  and "what shows health".

- Observation: the installer smoke check is cheap and valuable for this skill.
  Evidence: `fabro skill install --for project --dir claude --force` wrote the
  new `raspberry-authoring.md` and `raspberry-examples.md` files into the
  installed skill directory exactly as intended.

## Decision Log

- Decision: make the first slice a skill and reference-package upgrade instead
  of a generator runtime feature.
  Rationale: the canonical heuristics need one authoritative home before they
  can safely be automated behind a TOML or CLI surface.
  Date/Author: 2026-03-19 / Codex

- Decision: extend the existing `fabro-create-workflow` skill instead of
  creating a separate Raspberry-only default skill.
  Rationale: users already have one shipped install path, and the Raspberry
  rules are overlays on Fabro workflow authoring rather than a separate graph
  language.
  Date/Author: 2026-03-19 / Codex

- Decision: teach Raspberry authoring from the supervisor manifest model and
  fixtures rather than from placeholder lane run configs.
  Rationale: the manifest expresses the actual control-plane contract today, so
  the skill should optimize for the real consumer.
  Date/Author: 2026-03-19 / Codex

- Decision: frame the Raspberry guidance around repo obligations to the
  supervisory plane instead of around a speculative future generator interface.
  Rationale: the user's core question is what a supervised repo like Myosu must
  expose. That question stays stable even while Raspberry's higher-level design
  is still evolving.
  Date/Author: 2026-03-19 / Codex

## Outcomes & Retrospective

This slice landed successfully. The repo now has:

- a boundary decision saying the built-in workflow skill is the canonical home
  for Fabro and Raspberry authoring heuristics
- refreshed Fabro core references in the bundled skill package
- new Raspberry references that answer the repo-facing supervisory-plane
  question directly
- installer coverage proving those files ship through `fabro skill install`

The main lesson is that the right first deliverable was not "a generator". It
was a clearer statement of the contract a supervised repo must satisfy. That
contract now lives in the skill package and can be reused by any later TOML or
CLI generation surface.

## Context and Orientation

The current built-in workflow-authoring package lives in:

- `skills/fabro-create-workflow/SKILL.md`
- `skills/fabro-create-workflow/references/dot-language.md`
- `skills/fabro-create-workflow/references/run-configuration.md`
- `skills/fabro-create-workflow/references/example-workflows.md`

Those files are embedded and installed by:

- `lib/crates/fabro-cli/src/skill.rs`

The authoritative Fabro product docs that the skill should match live in:

- `docs/reference/dot-language.mdx`
- `docs/execution/run-configuration.mdx`
- `docs/reference/cli.mdx`

The authoritative Fabro structural rules also live in code:

- `lib/crates/fabro-config/src/run.rs` for run-config parsing and graph path
  resolution
- `lib/crates/fabro-config/src/project.rs` for resolving `.toml` workflows to
  DOT files
- `lib/crates/fabro-validate/src/rules.rs` for the validator rules that every
  generated graph must satisfy

The Raspberry control-plane contract that the skill must learn from lives in:

- `lib/crates/raspberry-supervisor/src/manifest.rs`
- `lib/crates/raspberry-supervisor/src/evaluate.rs`
- `test/fixtures/raspberry-supervisor/myosu-program.yaml`

In this repository, a **lane** is one schedulable stream of work inside a
Raspberry program manifest. A **managed milestone** is the lifecycle checkpoint
that the lane is responsible for satisfying. A **proof profile** is a compact
label that describes what evidence matters for that lane. A **produced
artifact** is a durable output file that contributes to the milestone contract.

The skill must remain honest about the boundary: it teaches how to author a
Fabro graph and run config that fit a Raspberry lane, but it does not become a
second supervisor or validator.

## Milestones

### Milestone 1: Refresh the existing bundled skill against current Fabro docs

At the end of this milestone, the existing `fabro-create-workflow` bundle no
longer has obvious vocabulary or contract drift from current Fabro docs. The
proof is that the bundled references align with `docs/reference/dot-language.mdx`,
`docs/execution/run-configuration.mdx`, and the current validator/parser rules.

### Milestone 2: Add Raspberry-aware authoring guidance and examples

At the end of this milestone, the built-in skill teaches how to go from a
Raspberry lane description, plan, or manifest to a Fabro workflow and run
config that satisfy both Fabro structural rules and Raspberry lane semantics.
The proof is that the skill package contains explicit Raspberry references and
example patterns for at least platform, service, and orchestration lanes.

### Milestone 3: Ship and validate the expanded skill package

At the end of this milestone, `fabro skill install` exports the expanded skill
package, the Rust installer tests pass, and a manual smoke prompt using the
Myosu fixture produces a lane-aware workflow proposal. The proof is both
automated tests and an install-to-disk check.

## Plan of Work

First, refresh the existing Fabro guidance instead of layering Raspberry notes
on top of drift. Update `skills/fabro-create-workflow/SKILL.md` so it clearly
explains when to read Fabro core references versus Raspberry references. Tighten
the trigger language so the skill covers both plain Fabro workflow authoring
and Fabro-for-Raspberry lane authoring.

Then, update the bundled reference files to match the authoritative docs and
runtime rules. The most important corrections are:

- current Fabro naming and terminology
- required start/exit/goal structure
- current validator rules
- run-config precedence and graph-path resolution
- the actual shipped workflow shapes and node types

After the Fabro core refresh, add Raspberry-specific guidance in new reference
files instead of bloating `SKILL.md`. The core skill body should stay concise
and route readers into the right reference file. The Raspberry references
should explain:

- how to read a Raspberry program manifest before proposing a graph
- how lane kind changes the recommended topology
- how `managed_milestone`, `produces`, and milestone requirements should shape
  graph outputs
- how service, orchestration, interface, platform, and recurring lanes differ
- how proof profiles and lane checks should influence verify/review/health nodes
- what run-config and file-layout conventions make a lane easy for the
  supervisor to consume later

Add concrete examples derived from the Myosu-shaped fixture. Each example
should explain what the lane is trying to achieve, which Fabro topology it
should bias toward, what artifacts it must produce, and what common mistakes to
avoid.

Once the new references exist, extend `lib/crates/fabro-cli/src/skill.rs` so
`fabro skill install` embeds and installs them. Update the installer tests so a
missing embedded file becomes a test failure instead of silent drift.

Finally, update the user-facing docs in `docs/reference/cli.mdx` to say that
the built-in skill teaches both Fabro workflow authoring and Raspberry-aware
lane authoring. Add a short changelog entry if the repository's release notes
for this date are being maintained during implementation.

## Concrete Steps

Work from the repository root.

1. Refresh the current skill body and core references.

   Modify:
   - `skills/fabro-create-workflow/SKILL.md`
   - `skills/fabro-create-workflow/references/dot-language.md`
   - `skills/fabro-create-workflow/references/run-configuration.md`
   - `skills/fabro-create-workflow/references/example-workflows.md`

   Use these repo-local sources as ground truth:
   - `docs/reference/dot-language.mdx`
   - `docs/execution/run-configuration.mdx`
   - `lib/crates/fabro-validate/src/rules.rs`
   - `lib/crates/fabro-config/src/run.rs`
   - `lib/crates/fabro-config/src/project.rs`

2. Add Raspberry-specific authoring references.

   Create:
   - `skills/fabro-create-workflow/references/raspberry-authoring.md`
   - `skills/fabro-create-workflow/references/raspberry-examples.md`

   Those files should cover:
   - manifest-first authoring workflow
   - lane kinds and their topology biases
   - milestone/artifact alignment rules
   - proof/service/orchestration concerns
   - examples grounded in
     `test/fixtures/raspberry-supervisor/myosu-program.yaml`

3. Extend the installer embed list and tests.

   Modify:
   - `lib/crates/fabro-cli/src/skill.rs`

   The Rust tests in that file should fail if the new reference files are not
   embedded or not written to disk during installation.

4. Update user-facing docs.

   Modify:
   - `docs/reference/cli.mdx`

   Optionally create or modify:
   - `docs/changelog/2026-03-19.mdx`

   Only add the changelog entry if the implementation slice is being shipped as
   a documented user-facing improvement.

5. Run validation and a manual smoke check.

   Automated checks:
   - Rust tests for the installer
   - any formatting or linting required for touched Rust files

   Manual smoke check:
   - install the skill to a temporary project location
   - inspect the installed files
   - use the installed skill plus
     `test/fixtures/raspberry-supervisor/myosu-program.yaml` to prompt an
     assistant for one lane-oriented workflow proposal

## Validation and Acceptance

Run these commands from the repository root:

    cargo test -p fabro-cli skill

If Rust formatting changed while touching `lib/crates/fabro-cli/src/skill.rs`,
also run:

    cargo fmt --check --all

Then install the skill into a throwaway project path or the current project:

    fabro skill install --for project --dir claude --force

Acceptance is complete when:

- the installer tests pass
- the installed `fabro-create-workflow` directory contains the new Raspberry
  reference files
- `SKILL.md` clearly tells the reader when to use Fabro core references versus
  Raspberry references
- the Raspberry references teach lane-aware authoring using the current
  supervisor manifest model rather than placeholder TOML assumptions
- a manual prompt against the Myosu fixture yields a proposal that names the
  lane kind, dependencies, produced artifacts, managed milestone, and suggested
  Fabro topology

## Idempotence and Recovery

This slice is additive and safe to repeat.

Re-running `fabro skill install --force` should simply overwrite the installed
skill package with the current embedded files. That is the intended recovery
path if installed files drift or become partially edited.

If the new Raspberry references prove too noisy, the rollback path is to remove
them from `skills/fabro-create-workflow/` and from
`lib/crates/fabro-cli/src/skill.rs`, then re-run the installer tests. No data
migration is involved.

## Artifacts and Notes

The most important artifact for this slice is the installed skill directory
written by `fabro skill install`. During implementation, capture one short
transcript showing:

- the install command
- the list of installed files
- the presence of the Raspberry references

The most important content artifact is one example prompt/output pair showing
that the skill can reason from `myosu-program.yaml` to a lane-aware workflow
proposal. Keep the transcript short and store only what proves that the skill
is now teaching the intended behavior.

## Interfaces and Dependencies

No new Rust dependencies are required for this slice.

The stable interfaces at the end of this slice should be:

- the existing `fabro skill install` CLI surface
- an expanded `fabro-create-workflow` package layout under
  `skills/fabro-create-workflow/`
- an expanded `SKILL_FILES` list in `lib/crates/fabro-cli/src/skill.rs`

The key file-level contract is:

- every reference file shipped in the skill package must also be embedded in
  `lib/crates/fabro-cli/src/skill.rs`
- `SKILL.md` must remain the routing layer and should stay concise, pushing
  detailed Raspberry rules into `references/`
- Raspberry guidance must be sourced from repo-local manifest and supervisor
  behavior, not from external repos or chat history
