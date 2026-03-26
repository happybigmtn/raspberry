# Ci Commit Hygiene Check Lane — Review

Review only the current slice for `settlement-hygiene-and-evidence-separation-ci-commit-hygiene-check`.

Current Slice Contract:
Plan file:
- `genesis/plans/013-settlement-hygiene-and-evidence-separation.md`

Child work item: `settlement-hygiene-and-evidence-separation-ci-commit-hygiene-check`

Full plan context (read this for domain knowledge, design decisions, and specifications):

# Settlement Hygiene and Evidence Separation

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, `integrate(lane)` commits contain only the files that belong to the settled slice — no generated package churn, no evidence artifacts, no unrelated prompt rewrites. A human reviewing a trunk commit can trust that every changed file was intentionally part of that lane's work. Evidence artifacts (spec.md, review.md, quality.md) are published separately, not mixed into product commits.

The proof is: after a 20-cycle autodev run on rXMRbro, inspect each `integrate(*)` commit with `git show --stat`. No commit includes files under `malinka/**`, root `integration.md`, `.raspberry/**`, or `outputs/**/{spec,review,verification,quality,promotion}.md`.

Provenance: This plan carries forward `plans/032626-structural-remediation-from-landed-code-review.md`. The settlement hygiene implementation in `direct_integration.rs` has been completed. This plan adds live rollout validation and the evidence publication separation.

## Progress

- [ ] Verify settlement hygiene in direct_integration.rs works
- [ ] Verify evidence stripping from staged settlement commits
- [ ] Add explicit allowed-path metadata to integration lane configs
- [ ] Live validation: 20-cycle autodev run with clean commits
- [ ] Add commit hygiene check to CI

## Surprises & Discoveries

(To be updated — carry forward from parent plan)

- From parent plan: commit `fc9412733` touched `malinka/blueprints/rxmragent.yaml`, `malinka/programs/rxmragent.yaml`, many `malinka/prompts/**`, `malinka/workflows/**`, and removed root `integration.md`, even though it was labeled `integrate(red-dog)`.

## Decision Log

- Decision: Settlement commits strip generated package and evidence paths by default, with explicit opt-in for paths that must be included.
  Rationale: The default should be clean. If a lane genuinely needs to modify `malinka/**` (rare), it declares that in its run config. The common case is product code only.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: Stripping `malinka/**` from settlement commits may break lanes that intentionally modify the package (e.g., a synthesis meta-lane). Mitigation: the stripping logic checks for `integration.allowed_paths` in the run config and preserves those paths.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

Settlement commit hygiene was implemented in `lib/crates/fabro-workflows/src/direct_integration.rs`. The implementation strips:
- `malinka/**` (generated package churn)
- Root `integration.md`
- `.raspberry/**` (supervisor state)
- `outputs/**/{spec,review,verification,quality,promotion}.md` (evidence artifacts)

The invariant-driven synthesis pressure was added to `lib/crates/fabro-synthesis/src/render.rs` for roulette-style layout slices.

The lane-sizing pressure was added to `render.rs` through prompt guidance for oversized mixed-responsibility UI files.

## Milestones

### Milestone 1: Verify settlement hygiene

Confirm that `direct_integration.rs` correctly strips generated package paths. Run a test that creates a staged commit with both product files and `malinka/**` files, then verify the commit only includes product files.

Proof command:

    cargo nextest run -p fabro-workflows -- direct_integration hygiene

### Milestone 2: Evidence publication separation

Verify that evidence artifacts (`spec.md`, `review.md`, `quality.md`, `promotion.md`) are stripped from settlement commits. These artifacts should be published to a separate location (e.g., a `_evidence/` branch or a run artifact directory) rather than landing on trunk.

Proof command:

    cargo nextest run -p fabro-workflows -- direct_integration evidence

### Milestone 3: Allowed-path metadata

Add `integration.allowed_paths` to the run config model in `lib/crates/fabro-config/`. Lanes that legitimately modify `malinka/**` can declare those paths. The stripping logic in `direct_integration.rs` preserves declared paths.

Proof command:

    cargo nextest run -p fabro-config -- integration allowed_paths
    cargo nextest run -p fabro-workflows -- direct_integration allowed

### Milestone 4: Live validation

Run 20-cycle autodev on rXMRbro. After the run, inspect every `integrate(*)` commit:

Proof command:

    git log --oneline --grep="integrate(" | head -5 | while read hash rest; do
      echo "=== $hash $rest ==="
      git show --stat "$hash" | grep -E "malinka/|\.raspberry/|outputs/.*/(spec|review|quality|promotion)\.md" | wc -l
    done

Expected: 0 lines matching forbidden paths for each commit.

### Milestone 5: CI commit hygiene check

Add a CI step that checks the latest commit for settlement hygiene violations. If a commit message matches `integrate(*)` and includes forbidden paths, CI fails.

Proof command:

    # In .github/workflows/rust.yml, add:
    # git show --stat HEAD | grep -qE "^malinka/" && exit 1 || exit 0

## Validation and Acceptance

The plan is done when:
- Settlement commits contain only product files
- Evidence artifacts are published separately
- Allowed-path metadata overrides stripping when declared
- 20-cycle autodev produces only clean integration commits


Workflow archetype: implement

Review profile: standard

Active plan:
- `genesis/plans/001-master-plan.md`

Active spec:
- `genesis/SPEC.md`

Proof commands:
- `cargo nextest run -p fabro-workflows -- direct_integration hygiene`

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
- check state transitions that affect balances, commitments, randomness, payout safety, or replayability
- check secret handling, capability scoping, pairing/idempotence behavior, and privilege escalation paths

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
- overflow_safe: yes|no
- seed_binding_complete: yes|no
- house_authority_preserved: yes|no
- proof_covers_edge_cases: yes|no
- layout_invariants_complete: yes|no
- slice_decomposition_respected: yes|no
If any mandatory security field is `no`, set `merge_ready: no`.

Review stage ownership:
- you may write or replace `.fabro-work/promotion.md` in this stage
- read `.fabro-work/quality.md` before deciding `merge_ready`
- when the slice is security-sensitive, perform a Nemesis-style pass: first-principles assumption challenge plus coupled-state consistency review
- include security findings in the review verdict when the slice touches trust boundaries, keys, funds, auth, control-plane behavior, or external process control
- prefer not to modify source code here unless a tiny correction is required to make the review judgment truthful
