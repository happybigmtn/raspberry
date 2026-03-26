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
