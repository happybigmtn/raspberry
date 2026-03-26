# Sprint Contracts and Scored Review Lane — Plan

Lane: `sprint-contracts-and-scored-review`

Goal:
- Sprint Contracts and Scored Review

Bootstrap the first honest reviewed slice for this frontier.

Inputs:
- `README.md`
- `SPEC.md`
- `PLANS.md`
- `AGENTS.md`
- `CLAUDE.md`
- `genesis/plans/001-master-plan.md`

Current frontier tasks:
- Verify existing contract stage works end-to-end
- Verify existing scored review produces valid scores
- Add contract completeness metrics to autodev report
- Add review score distribution to autodev report
- Run A/B test: 10 lanes with challenge stage vs 10 without
- Decide whether to keep or remove challenge stage based on data

Required durable artifacts:
- `outputs/sprint-contracts-and-scored-review/spec.md`
- `outputs/sprint-contracts-and-scored-review/review.md`

Context:
- Plan file:
- `genesis/plans/006-sprint-contracts-and-scored-review.md`

Full plan context (read this for domain knowledge, design decisions, and specifications):

# Sprint Contracts and Scored Review

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, every implementation lane writes a sprint contract before coding, every review produces scored dimensions with hard thresholds, and the quality gate verifies contract deliverables exist. The review reject rate becomes measurable, and agents can no longer declare "merge_ready: yes" without checking specific criteria.

The proof is: run 10 lanes through the autodev pipeline. All 10 produce `.fabro-work/contract.md` before implementation. All 10 produce scored `promotion.md` with dimension scores. At least 2 are rejected by score thresholds rather than passing by default.

Provenance: This plan enhances `plans/032426-harness-redesign-sprint-contracts-and-evaluation.md`. Sprint contracts and scored review have been partially implemented (commits `6a89dc3c`, `08f01ca1`, `6404ab69`). This plan adds measurement, validation, and the Phase 4 simplification assessment.

## Progress

- [ ] Verify existing contract stage works end-to-end
- [ ] Verify existing scored review produces valid scores
- [ ] Add contract completeness metrics to autodev report
- [ ] Add review score distribution to autodev report
- [ ] Run A/B test: 10 lanes with challenge stage vs 10 without
- [ ] Decide whether to keep or remove challenge stage based on data

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: Build measurement before new features.
  Rationale: Sprint contracts and scored review are already partially implemented. The gap is measurement — we don't know if they improve quality. Adding metrics first lets us make data-driven decisions about Phase 4 (challenge stage simplification).
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: Scored review with hard thresholds may cause all lanes to fail review on the first try, creating a retry spiral. Mitigation: start with threshold=5 (lenient), measure rejection rate, then tighten to threshold=6 if rejection rate is <20%.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

The workflow DAG currently includes a contract stage between preflight and implement:

```
start → preflight → contract → implement → verify → quality → challenge → review → audit → exit
                                              ↑                                        |
                                              └──── fixup ←────────────────────────────┘
```

Key files:
- `lib/crates/fabro-synthesis/src/render.rs` — renders the workflow graph including contract and review nodes
- `lib/crates/fabro-synthesis/src/render.rs` — `implementation_quality_command()` for contract-aware quality gate
- `lib/crates/fabro-synthesis/src/render.rs` — `implementation_promotion_contract_command()` for score parsing

The contract prompt tells the agent to write `.fabro-work/contract.md` with Deliverables, Acceptance Criteria, and Out of Scope sections. The review prompt tells the agent to score Completeness (3x weight), Correctness (2x), Convention (2x), and Test Quality (1x) on a 0-10 scale.

The existing score parsing in the promotion gate checks:

    grep -Eq '^completeness: [6-9]$|^completeness: 10$' .fabro-work/promotion.md

## Milestones

### Milestone 1: Validate contract stage

Run 5 lanes through autodev with contract stage enabled. Verify:
- Each lane produces `.fabro-work/contract.md`
- Contract has Deliverables, Acceptance Criteria, Out of Scope sections
- Implement stage references the contract in its output

Proof command:

    # After a 5-lane autodev run, check artifacts:
    find /home/r/coding/rXMRbro -name "contract.md" -path "*/.fabro-work/*" | wc -l

Expected: >= 3 contract files found.

### Milestone 2: Validate scored review

Verify that review stages produce `promotion.md` with valid scored dimensions. Check that the promotion gate correctly rejects scores below threshold.

Proof command:

    find /home/r/coding/rXMRbro -name "promotion.md" -path "*/.fabro-work/*" \
      -exec grep -l "completeness:" {} \; | wc -l

### Milestone 3: Contract completeness metrics

Add `contracts_written`, `contracts_verified`, `contracts_missing_deliverables` counters to the autodev report in `lib/crates/raspberry-supervisor/src/autodev.rs`.

Proof command:

    cargo nextest run -p raspberry-supervisor -- autodev contract_metrics

### Milestone 4: Review score distribution

Add `review_scores`, `review_rejections`, `review_acceptances` to the autodev report. Track mean and min scores per dimension across all reviewed lanes.

Proof command:

    cargo nextest run -p raspberry-supervisor -- autodev review_metrics

### Milestone 5: Challenge stage A/B assessment

Run 10 lanes with challenge stage enabled and 10 without (same plans, different configs). Compare:
- Quality gate pass rate
- Review scores (mean per dimension)
- Number of fixup cycles required

This is an assessment milestone, not a code change. Document results in `genesis/artifacts/challenge-ab-results.md`.

Proof command:

    # Manual comparison of two autodev runs with different render.rs profile settings

### Milestone 6: Decision and implementation

Based on A/B results: if challenge adds <10% improvement to review scores, add a `"lean"` profile to `render.rs` that skips the challenge stage. Otherwise, keep challenge and document the improvement.

Proof command:

    cargo nextest run -p fabro-synthesis -- profile lean

## Validation and Acceptance

The plan is done when:
- Contract and scored review are validated end-to-end
- Autodev report shows contract and review metrics
- Challenge stage A/B decision is documented with data
- If lean profile added, it works without breaking existing lanes


Active plan:
- `genesis/plans/001-master-plan.md`

Active spec:
- `genesis/SPEC.md`

Mapping notes:
- composite plan mapped from plan structure; humans may refine the checked-in contract later

Open tasks:
- Verify existing contract stage works end-to-end
- Verify existing scored review produces valid scores
- Add contract completeness metrics to autodev report
- Add review score distribution to autodev report
- Run A/B test: 10 lanes with challenge stage vs 10 without
- Decide whether to keep or remove challenge stage based on data

Artifacts to write:
- `spec.md`
- `review.md`
