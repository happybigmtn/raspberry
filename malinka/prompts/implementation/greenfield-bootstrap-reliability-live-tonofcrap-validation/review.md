# Live Tonofcrap Validation Lane — Review

Review only the current slice for `greenfield-bootstrap-reliability-live-tonofcrap-validation`.

Current Slice Contract:
Plan file:
- `genesis/plans/004-greenfield-bootstrap-reliability.md`

Child work item: `greenfield-bootstrap-reliability-live-tonofcrap-validation`

Full plan context (read this for domain knowledge, design decisions, and specifications):

# Greenfield Bootstrap and Runtime Asset Reliability

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, `fabro synth create` on a new repo produces a package where the scaffold/infrastructure lane completes before any feature lane dispatches, and the generated runtime assets resolve correctly when the workflow is copied into `~/.fabro/runs/`. Agents will never write TypeScript into a repo with no `package.json`, Rust code with no `Cargo.toml`, or prompt references that only work from the original repo checkout.

The proof is: run `fabro synth create` on a fresh repo, then `raspberry autodev`. The scaffold lane completes first, feature lanes dispatch only after scaffold verification passes, and no lane fails because of missing project infrastructure or prompt/workflow assets resolving relative to the wrong root.

Provenance: This plan replaces `plans/032426-greenfield-bootstrapping-and-code-quality.md` with a structured ExecPlan. It also incorporates the scaffold-first ordering from commit `6d0853f4` and the bootstrap guard from commit `cb0c016e`.

## Progress

- [ ] Verify scaffold-first ordering works in planning.rs
- [ ] Add bootstrap verification gate to render.rs
- [ ] Make generated prompt and workflow asset refs runtime-stable
- [ ] Add type-aware quality checks for TypeScript projects
- [ ] Validate on tonofcrap with 30-cycle autodev run
- [ ] Validate on a fresh Rust project with scaffold dependency

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: Treat runtime asset resolution as part of bootstrap correctness.
  Rationale: Fresh-repo bootstrapping is not only about scaffold ordering. A generated package that validates in-repo but fails after Fabro copies `graph.fabro` into a detached run dir is still broken for new operators. Prompt/workflow refs must survive the runtime handoff.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: False positive infrastructure detection — a plan named "infrastructure" that is actually a feature plan gets treated as a scaffold dependency, blocking everything. Mitigation: infrastructure detection should check both plan name AND category metadata.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: bootstrap verification passes in the target repo but detached runs still fail because `@../../prompts/...` resolves under `~/.fabro/` instead of the project root. Mitigation: generated workflows must use runtime-stable asset references or copy the required prompt assets into the run context.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

The greenfield bootstrap problem was identified on the tonofcrap project (TypeScript/React/Convex on TON). The root cause chain is documented in `plans/032426-greenfield-bootstrapping-and-code-quality.md`:

1. `project-scaffold` runs in parallel with feature lanes instead of before them
2. Agents write `.ts` files into a repo with no `package.json` or `tsconfig.json`
3. The verify gate (`npx convex dev --typecheck`) fails because `npx` has nothing to run
4. Quality gate passes with `any[]` parameters because there's no schema to check against

Two fixes have already been partially implemented:
- Scaffold-first ordering in `lib/crates/fabro-synthesis/src/planning.rs` (commit `6d0853f4`)
- Bootstrap guard for fresh projects in `fabro-workflows/src/` (commit `cb0c016e`)

The remaining work is bootstrap verification, runtime-stable asset resolution, and type-aware quality enforcement.

```
Current greenfield flow:
  scaffold ──┐
  feature-1 ─┤  (all parallel — scaffold may not finish first)
  feature-2 ─┘

Target greenfield flow:
  scaffold ──> [bootstrap verify] ──> feature-1 ──┐
                                   ──> feature-2 ──┘
```

## Milestones

### Milestone 1: Verify scaffold-first ordering

Confirm that `derive_registry_plan_intents()` in `lib/crates/fabro-synthesis/src/planning.rs` correctly injects scaffold plans as dependencies for non-infrastructure plans. Write a test with a blueprint that has both infrastructure and feature plans.

Proof command:

    cargo nextest run -p fabro-synthesis -- scaffold_first

### Milestone 2: Bootstrap verification gate

Add a bootstrap verification step to `render_workflow_graph()` in `lib/crates/fabro-synthesis/src/render.rs`. For the scaffold lane, insert a verification node that checks language-specific project health markers:

- Node.js/TypeScript: `package.json` exists, `node_modules/` populated, `tsconfig.json` present
- Rust: `Cargo.toml` valid, `cargo check` passes
- Python: `pyproject.toml` or `requirements.txt` exists, `pip install` passes

Proof command:

    cargo nextest run -p fabro-synthesis -- bootstrap_verify

### Milestone 3: Runtime-stable asset resolution

Ensure generated workflows and copied run graphs can resolve all prompt and supporting asset references from the detached run environment. The operator should not need a global `~/.fabro/prompts` symlink to make freshly generated lanes validate.

Key files:
- `lib/crates/fabro-synthesis/src/render.rs`
- `lib/crates/fabro-workflows/src/handler/agent.rs`
- `lib/crates/fabro-cli/src/commands/run.rs`

Proof command:

    /home/r/.cache/cargo-target/debug/fabro validate /home/r/coding/rXMRbro/malinka/run-configs/investigate/baccarat-investigate.toml

### Milestone 4: Type-aware quality for TypeScript

Extend `implementation_quality_command()` in `lib/crates/fabro-synthesis/src/render.rs` for TypeScript projects:
- Check that `.ts` files don't use `any` in exported function signatures
- Check that test files import from the module they claim to test
- Check that schema file exists if declared in plan context

Proof command:

    cargo nextest run -p fabro-synthesis -- quality typescript

### Milestone 5: Live tonofcrap validation

Regenerate tonofcrap package and run 30-cycle autodev. Scaffold must complete first. Feature lanes must not fail due to missing infrastructure.

Review rule:
- A 5-cycle or similarly short smoke run can support diagnosis, but it is not sufficient evidence to mark this milestone complete.

Proof command:

    target-local/release/fabro --no-upgrade-check synth create \
      --target-repo /home/r/coding/tonofcrap --program repo \
      --blueprint /home/r/coding/tonofcrap/malinka/blueprints/repo.yaml \
      --no-decompose --no-review && \
    target-local/release/raspberry autodev \
      --manifest /home/r/coding/tonofcrap/malinka/programs/repo.yaml \
      --max-cycles 30

### Milestone 6: Fresh Rust project validation

Create a minimal test fixture Rust project (Cargo workspace with 2 crates). Run `fabro synth genesis` → `raspberry autodev` and confirm scaffold-first ordering works.

Proof command:

    cargo nextest run -p fabro-synthesis -- greenfield_rust

## Validation and Acceptance

The plan is done when:
- scaffold plans always dispatch before feature plans
- bootstrap verification confirms project health before downstream lanes
- generated prompt/workflow assets resolve correctly in detached run dirs
- TypeScript quality gate catches `any` usage and missing imports
- tonofcrap and a fresh Rust project produce no infrastructure- or asset-resolution-caused failures


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

Evidence discipline
- Treat any smoke run shorter than the contract's 30-cycle `raspberry autodev` proof as partial evidence only.
- Do not score Milestone 5 complete, or bless the live-validation acceptance criteria as satisfied, unless a real `raspberry autodev --max-cycles 30` run was executed and summarized.
- If the verification artifact claims full success from only a shorter run or from static artifact inspection, record that as a blocking finding.

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
