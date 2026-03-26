# Plan Completion Detection Lane — Review

Review only the current slice for `plan-completion-and-adversarial-review-plan-completion-detection`.

Current Slice Contract:
Plan file:
- `genesis/plans/007-plan-completion-and-adversarial-review.md`

Child work item: `plan-completion-and-adversarial-review-plan-completion-detection`

Full plan context (read this for domain knowledge, design decisions, and specifications):

# Plan Completion Detection and Adversarial Review

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, when all child lanes of a plan complete and land on trunk, the autodev loop automatically triggers a 3-step adversarial review (Bug Finder → Bug Skeptic → Arbiter) on the aggregate diff. Confirmed bugs get fixed in the target repo. Bug patterns feed back as quality gate improvements. Plans stop being a scheduling concept and become a review boundary with automatic completion detection.

The proof is: in an rXMRbro autodev run, when all child lanes of a plan complete, the plan-review lane triggers automatically without operator intervention. The review produces a bug report, and confirmed bugs are either fixed or surfaced for operator review.

Provenance: This plan consolidates `plans/032526-plan-level-adversarial-review-and-recursive-improvement.md`, the plan-completion detection items from `plans/032526-e2e-autodev-review-and-remediation.md` Phase 3, and the plan-first redesign portfolio work from `plans/032126-plan-first-autodev-redesign.md`.

## Progress

- [ ] Implement plan-completion detection in autodev.rs
- [ ] Wire plan-status tracking across cycles
- [ ] Implement 3-step adversarial review prompt
- [ ] Generate plan-review lanes from plan registry
- [ ] Add meta-review feedback loop (bug patterns → quality gate rules)
- [ ] Live validation with plan completion triggering review

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: Plan completion is detected by comparing plan status matrix across autodev cycles, not by watching individual lane events.
  Rationale: The plan status matrix in `plan_status.rs` already computes per-plan completion state. Comparing across cycles is simpler and more reliable than tracking individual lane completion events.
  Date/Author: 2026-03-26 / Genesis

- Decision: Meta-review proposals (bug patterns → Fabro improvements) require operator approval before applying.
  Rationale: Allowing the system to modify its own quality gates without human review creates a feedback loop that could weaken enforcement. The meta-review writes proposals to `.fabro-work/meta-review-{plan_id}.md` for the operator to review.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: Plan-review lanes could fire before all child code is actually on trunk (race between landing and status refresh). Mitigation: plan-completion detection must verify trunk state, not just lane status.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

Plan completion detection needs to live in the autodev cycle in `lib/crates/raspberry-supervisor/src/autodev.rs`. The plan status matrix is computed by `lib/crates/raspberry-supervisor/src/plan_status.rs`, which already knows per-plan lane counts, completion counts, and overall status.

Plan-review lane generation should happen in `lib/crates/fabro-synthesis/src/render.rs`, which already generates `*-plan-review` lanes. The current implementation uses a brittle `unit.id` string-splitting approach. The genesis plan replaces this with registry-driven generation.

The 3-step adversarial review process:

```
Plan completes (all child lanes on trunk)
     |
     v
Step 1: Bug Finder (Codex/Claude)
  - Reads aggregate diff of all child integrations
  - Aggressive bug search: +1 low, +5 medium, +10 critical
     |
     v
Step 2: Bug Skeptic (Codex/Claude)
  - Challenges each bug
  - +[bug pts] for correct disproves, -2x[bug pts] for wrong dismissals
     |
     v
Step 3: Arbiter (Codex/Claude)
  - Final verdict on each disputed bug
  - Outputs confirmed bugs with severity and fix instructions
     |
     v
Fix confirmed bugs → Meta-review → Operator-approved quality gate updates
```

## Milestones

### Milestone 1: Plan-completion detection

Track plan statuses across autodev cycles in a `BTreeSet<String>` within the orchestrate loop. When a plan transitions from incomplete to complete, emit a `PlanCompleted` event.

Key file: `lib/crates/raspberry-supervisor/src/autodev.rs`

Proof command:

    cargo nextest run -p raspberry-supervisor -- autodev plan_completed

### Milestone 2: Registry-driven plan-review generation

Replace the brittle `unit.id` string-splitting plan-review generation in `render.rs` with registry-backed generation. For each plan in the registry, generate a `{plan}-plan-review` lane with dependencies on all child lane IDs.

Key file: `lib/crates/fabro-synthesis/src/render.rs`

Proof command:

    cargo nextest run -p fabro-synthesis -- plan_review registry

### Milestone 3: 3-step adversarial review prompt

Implement the Bug Finder → Bug Skeptic → Arbiter prompt chain as the plan-review workflow. The review lane should:
1. Compute the aggregate diff (`git diff` across all child integration commits)
2. Run Bug Finder with aggressive scoring
3. Run Bug Skeptic to challenge findings
4. Run Arbiter for final verdict
5. Write confirmed bugs to `.fabro-work/plan-review-{plan_id}.md`

Key file: `lib/crates/fabro-synthesis/src/render.rs` (plan-review workflow graph)

Proof command:

    cargo nextest run -p fabro-synthesis -- adversarial_review

### Milestone 4: Meta-review feedback loop

After plan-review completes, generate a meta-review prompt that analyzes bug PATTERNS (not individual bugs) and proposes Fabro-level improvements: quality gate rules, prompt improvements, convention checks. Write proposals to `.fabro-work/meta-review-{plan_id}.md`.

Key file: `lib/crates/fabro-synthesis/src/render.rs`

Proof command:

    cargo nextest run -p fabro-synthesis -- meta_review

### Milestone 5: Live validation

Run autodev on rXMRbro until at least one plan completes. Verify that:
- Plan-completion event fires
- Plan-review lane dispatches automatically
- Review produces a bug report
- Meta-review produces improvement proposals

Proof command:

    # After autodev run:
    find /home/r/coding/rXMRbro -name "plan-review-*.md" -path "*/.fabro-work/*" | head -1

## Validation and Acceptance

The plan is done when:
- Plan completion triggers review automatically in autodev
- Plan-review lanes are generated from the plan registry
- 3-step adversarial review produces actionable bug reports
- Meta-review proposes Fabro-level improvements
- At least one plan has gone through the full review cycle in a live run


Workflow archetype: implement

Review profile: standard

Active plan:
- `genesis/plans/001-master-plan.md`

Active spec:
- `genesis/SPEC.md`

Proof commands:
- `cargo nextest run -p fabro-synthesis -- plan_review registry`
- `cargo nextest run -p raspberry-supervisor -- autodev plan_completed`

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
