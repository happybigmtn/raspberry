# Live Validation On Rxmrbro Lane — Review

Review only the current slice for `workspace-integration-verification-live-validation-on-rxmrbro`.

Current Slice Contract:
Plan file:
- `genesis/plans/010-workspace-integration-verification.md`

Child work item: `workspace-integration-verification-live-validation-on-rxmrbro`

Full plan context (read this for domain knowledge, design decisions, and specifications):

# Workspace Integration Verification

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, when all child implementation lanes of a unit complete, the synthesis pipeline automatically generates a workspace-verify lane that runs `cargo test --workspace` (or the language-equivalent). Cross-crate API mismatches, stale imports, and type incompatibilities are caught before the code reaches parent review. Protocol contracts between implementors and consumers are verified by auto-generated contract lanes.

The proof is: `fabro synth create` on a blueprint with >3 child lanes generates a `*-workspace-verify` lane. That lane dispatches after children complete and runs workspace-wide compilation and tests.

Provenance: This plan carries forward `plans/032426-integration-verification-and-codebase-polish.md` with implementation milestones added.

## Progress

- [ ] Add integration proof profile to render.rs
- [ ] Generate workspace-verify lanes for multi-child units
- [ ] Add BlueprintProtocol struct and protocol lane generation
- [ ] Add consistency challenge prompts for integration profile
- [ ] Validate on rXMRbro with workspace-verify lane

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: workspace-verify lanes are read-only — they report failures but do not modify source code.
  Rationale: workspace verification should surface problems for the parent review gauntlet to fix. Having workspace-verify also modify code creates unclear ownership boundaries between the verification lane and the original implementation lane.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: workspace-verify lane runs `cargo test --workspace` which takes 10+ minutes on a large project, causing the lane to hit timeout limits. Mitigation: for large workspaces, use `cargo check --workspace` first (fast fail) then `cargo test --workspace` (full verification). Set a generous timeout for integration lanes.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

The problem: each lane's `cargo test -p {crate}` passes independently, but `cargo test --workspace` may fail due to API mismatches between crates. No lane currently tests the workspace-level contract.

The fix adds two new auto-generated lane types:

1. **workspace-verify** — runs after all child lanes of a unit complete. Verifies workspace-level compilation and tests.
2. **protocol-contract-verify** — runs after both implementor and consumer lanes complete. Verifies interface boundaries between crates that share a trait.

Key files to modify:
- `lib/crates/fabro-synthesis/src/render.rs` — `augment_with_implementation_follow_on_units()` for workspace-verify generation
- `lib/crates/fabro-synthesis/src/render.rs` — `profile_max_visits()` and `profile_extra_graph_elements()` for integration profile
- `lib/crates/fabro-synthesis/src/blueprint.rs` — `BlueprintProtocol` struct for protocol contracts

```
Child lanes complete:
  game-engine-core ──> complete
  game-engine-tests ──> complete
  game-engine-rpc ──> complete
       |
       v
  game-engine-workspace-verify
  (cargo check --workspace && cargo test --workspace)
       |
       v
  game-engine-holistic-preflight (parent gauntlet)
```

## Milestones

### Milestone 1: Integration proof profile

Add `"integration"` to the proof profile system in `lib/crates/fabro-synthesis/src/render.rs`:
- `max_visits`: 6 (cross-crate issues need more fixup cycles)
- Verify command override: workspace-wide, not per-crate
- Challenge prompt: check cross-module imports and shared trait implementations

Proof command:

    cargo nextest run -p fabro-synthesis -- profile integration

### Milestone 2: Workspace-verify lane generation

In `render.rs` `augment_with_implementation_follow_on_units()`, when a unit has child implementation lanes, generate:
- Lane ID: `"{unit}-workspace-verify"`
- Template: Implementation
- Proof profile: integration
- Goal: "Verify full workspace compiles and all tests pass"
- Verify command: `"cargo check --workspace && cargo test --workspace"`
- Dependencies: all child lane IDs
- Owned surfaces: empty (read-only)

Proof command:

    cargo nextest run -p fabro-synthesis -- workspace_verify generation

### Milestone 3: BlueprintProtocol and contract lane generation

Add `BlueprintProtocol` to `lib/crates/fabro-synthesis/src/blueprint.rs`:

    pub struct BlueprintProtocol {
        pub id: String,
        pub trait_name: String,
        pub implementor_units: Vec<String>,
        pub consumer_units: Vec<String>,
        pub verification_command: String,
    }

For each protocol, generate a contract-verify lane with dependencies on all implementor + consumer lanes.

Proof command:

    cargo nextest run -p fabro-synthesis -- protocol_contract

### Milestone 4: Consistency challenge prompts

For integration-profile lanes, generate challenge prompts that specifically check:
- Narrowing casts (`as i16`, `as f64`) in arithmetic code
- Cross-crate import consistency
- Shared trait implementation patterns

Proof command:

    cargo nextest run -p fabro-synthesis -- consistency challenge

### Milestone 5: Live validation on rXMRbro

Regenerate rXMRbro package and confirm workspace-verify lanes appear. Run autodev until a workspace-verify lane dispatches and produces results.

Proof command:

    target-local/release/fabro --no-upgrade-check synth create \
      --target-repo /home/r/coding/rXMRbro --program rxmragent \
      --no-decompose --no-review && \
    grep "workspace-verify" /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml

## Validation and Acceptance

The plan is done when:
- `synth create` generates workspace-verify lanes for multi-child units
- Integration proof profile has 6 max_visits and workspace-wide verify commands
- Protocol contracts generate cross-crate verification lanes
- At least one workspace-verify lane runs in a live autodev session


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
