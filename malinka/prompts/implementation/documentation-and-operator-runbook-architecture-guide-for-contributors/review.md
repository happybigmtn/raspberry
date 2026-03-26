# Architecture Guide For Contributors Lane — Review

Review only the current slice for `documentation-and-operator-runbook-architecture-guide-for-contributors`.

Current Slice Contract:
Plan file:
- `genesis/plans/014-documentation-and-operator-runbook.md`

Child work item: `documentation-and-operator-runbook-architecture-guide-for-contributors`

Full plan context (read this for domain knowledge, design decisions, and specifications):

# Documentation and Operator Runbook

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, a new operator can learn how to use Fabro/Raspberry from documentation alone — without reading source code or existing plans. The docs cover: installation, genesis, autodev operation, TUI usage, troubleshooting, and architecture. The README reflects the current state of the project, not the upstream Fabro docs.

The proof is: a technical person who has never seen the repo follows the docs from install to running autodev on their own repo, encountering no undocumented steps.

## Progress

- [ ] Rewrite README to reflect Fabro/Raspberry reality
- [ ] Write operator quickstart guide
- [ ] Write Raspberry command reference
- [ ] Write troubleshooting guide
- [ ] Write architecture guide for contributors
- [ ] Audit all existing docs for accuracy

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: Docs live in `docs/` using Mintlify format, consistent with existing doc structure.
  Rationale: The existing `docs/` directory uses Mintlify with `.mdx` files. New docs should follow the same convention.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: Documentation goes stale as the codebase evolves. Mitigation: each numbered plan in genesis includes a documentation milestone, and the release plan (016) includes a doc freshness audit.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

Current documentation state:
- `README.md` — describes the fork's purpose and architecture, but some sections reference features that don't exist yet
- `ARCHITECTURE.md` — good internal handoff map, but not user-facing
- `autodev.md` — short restart guide, operator-internal
- `docs/` — Mintlify docs, mostly upstream Fabro content
- `docs/guides/` — some guides exist (from-specs-to-blueprint, raspberry-operator-runbook, etc.)

Missing documentation:
- No quickstart guide for new operators
- No command reference for `raspberry` CLI commands
- No troubleshooting guide for common autodev failures
- No architecture guide aimed at contributors (not just internal handoff)

## Milestones

### Milestone 1: Rewrite README

Update `README.md` to:
- Accurately describe current capabilities (not aspirational features)
- Include working install instructions
- Show the 3-command onboarding flow (genesis → autodev → tui)
- Remove references to unimplemented features
- Keep the architecture diagram accurate

Proof command:

    # Verify README links and commands are valid:
    grep -c "fabro synth genesis\|raspberry autodev\|raspberry tui" README.md

### Milestone 2: Operator quickstart guide

Write `docs/guides/quickstart.mdx` covering:
1. Installation (cargo install or curl installer)
2. `fabro synth genesis --target-repo .` on your repo
3. Review generated plans in `plans/`
4. `raspberry autodev --manifest malinka/programs/*.yaml`
5. `raspberry tui --manifest malinka/programs/*.yaml`
6. Expected output at each step

Proof command:

    test -f docs/guides/quickstart.mdx && wc -l docs/guides/quickstart.mdx

Expected: >80 lines.

### Milestone 3: Raspberry command reference

Write `docs/reference/raspberry-cli.mdx` documenting every Raspberry CLI command:
- `raspberry plan` — show program plan
- `raspberry status` — show lane status
- `raspberry watch` — watch for changes
- `raspberry execute` — dispatch one cycle
- `raspberry autodev` — autonomous execution loop
- `raspberry tui` — terminal UI
- `raspberry plan-matrix` — plan-first status view

Include flags, examples, and expected output for each.

Proof command:

    test -f docs/reference/raspberry-cli.mdx && grep -c "##" docs/reference/raspberry-cli.mdx

Expected: >= 7 command sections.

### Milestone 4: Troubleshooting guide

Write `docs/guides/troubleshooting.mdx` covering common failure modes:
- Provider quota exhaustion → fallback behavior
- Stale `running` lanes → manual reset
- Scaffold-first failures → verify bootstrap
- Review rejection spiral → check score thresholds
- Direct integration failures → check branch state
- TUI shows no data → verify manifest path

Proof command:

    test -f docs/guides/troubleshooting.mdx && wc -l docs/guides/troubleshooting.mdx

### Milestone 5: Architecture guide for contributors

Write `docs/guides/architecture.mdx` covering:
- Crate dependency graph with explanation
- Data flow from plans to landed code
- Key abstractions (Sandbox trait, workflow graphs, blueprint pipeline)
- How to add a new crate
- How to add a new autodev stage

Proof command:

    test -f docs/guides/architecture.mdx && wc -l docs/guides/architecture.mdx

### Milestone 6: Doc freshness audit

Review all existing docs in `docs/` for accuracy. Remove or update any references to:
- Removed features
- Wrong file paths
- Outdated commands
- Upstream Fabro-only content

Proof command:

    # Check for broken internal links:
    grep -rn "](/.*)" docs/ --include="*.mdx" | grep -v node_modules | head -20

## Validation and Acceptance

The plan is done when:
- README accurately reflects current capabilities
- Quickstart guide covers the full onboarding flow
- Command reference documents all Raspberry CLI commands
- Troubleshooting guide covers the 6 most common failure modes
- Architecture guide helps a new contributor understand the codebase


Workflow archetype: implement

Review profile: standard

Active plan:
- `genesis/plans/001-master-plan.md`

Active spec:
- `genesis/SPEC.md`

Artifacts to write:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Nemesis-style security review
- Pass 1 — first-principles challenge: question trust boundaries, authority assumptions, and who can trigger the slice's dangerous actions
- Pass 2 — coupled-state review: identify paired state or protocol surfaces and check that every mutation path keeps them consistent or explains the asymmetry
- check secret handling, capability scoping, pairing/idempotence behavior, and privilege escalation paths
- check external-process control, operator safety, idempotent retries, and failure modes around service lifecycle

Focus on:
- slice scope discipline
- proof-gate coverage for the active slice
- touched-surface containment
- implementation and verification artifact quality
- remaining blockers before the next slice


Structural discipline
- if a new source file would exceed roughly 400 lines, split it before landing
- do not mix state transitions, input handling, rendering, and animation in one new file unless the prompt explicitly justifies that coupling
- if the slice cannot stay small, stop and update the artifacts to explain the next decomposition boundary instead of silently landing a monolith
Deterministic evidence:
- treat `.fabro-work/quality.md` as machine-generated truth about placeholder debt, warning debt, manual follow-up, and artifact mismatch risk
- if `.fabro-work/quality.md` says `quality_ready: no`, do not bless the slice as merge-ready


Score each dimension 0-10 and write `.fabro-work/promotion.md` in this exact form:

merge_ready: yes|no
manual_proof_pending: yes|no
completeness: <0-10>
correctness: <0-10>
convention: <0-10>
test_quality: <0-10>
reason: <one sentence>
next_action: <one sentence>

Scoring guide:
- completeness: 10=all deliverables present + all acceptance criteria met, 7=core present + 1-2 gaps, 4=missing deliverables, 0=skeleton
- correctness: 10=compiles + tests pass + edges handled, 7=tests pass + minor gaps, 4=some failures, 0=broken
- convention: 10=matches all project patterns, 7=minor deviations, 4=multiple violations, 0=ignores conventions
- test_quality: 10=tests import subject + verify all criteria, 7=most criteria tested, 4=structural only, 0=no tests

If `.fabro-work/contract.md` exists, verify EVERY acceptance criterion from it.
Any dimension below 6 = merge_ready: no.
If `.fabro-work/quality.md` says quality_ready: no = merge_ready: no.

For security-sensitive slices, append these mandatory fields exactly:
- layout_invariants_complete: yes|no
- slice_decomposition_respected: yes|no
If any mandatory security field is `no`, set `merge_ready: no`.

Review stage ownership:
- you may write or replace `.fabro-work/promotion.md` in this stage
- read `.fabro-work/quality.md` before deciding `merge_ready`
- when the slice is security-sensitive, perform a Nemesis-style pass: first-principles assumption challenge plus coupled-state consistency review
- include security findings in the review verdict when the slice touches trust boundaries, keys, funds, auth, control-plane behavior, or external process control
- prefer not to modify source code here unless a tiny correction is required to make the review judgment truthful
