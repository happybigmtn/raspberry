# Metrics Summary In Status Output Lane — Review

Review only the current slice for `performance-measurement-and-optimization-metrics-summary-in-status-output`.

Current Slice Contract:
Plan file:
- `genesis/plans/015-performance-measurement-and-optimization.md`

Child work item: `performance-measurement-and-optimization-metrics-summary-in-status-output`

Full plan context (read this for domain knowledge, design decisions, and specifications):

# Performance Measurement and Optimization

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, the operator has a dashboard showing autodev operational health: cycle time, dispatch rate, trunk landing rate, cost per landed lane, and failure classification. Performance baselines exist so regressions are detectable. The system has measurable KPIs, not just "it runs."

The proof is: `raspberry status --metrics` shows cycle time p50/p95, dispatch rate, landing rate, cost estimate, and a failure breakdown. The metrics are computed from real autodev runs, not synthetic data.

## Progress

- [ ] Define operational KPIs and baseline targets
- [ ] Instrument autodev cycle with timing spans
- [ ] Add cost estimation per provider per cycle
- [ ] Add failure classification metrics
- [ ] Build metrics summary in raspberry status output
- [ ] Establish baselines from live rXMRbro runs

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: Metrics live in the autodev report, not in an external metrics system.
  Rationale: Adding Prometheus/Grafana for a single-operator tool is over-engineering. The autodev report already captures per-cycle data. Aggregating it into a metrics summary is simpler.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: Cycle time measurement may be skewed by evolve steps (which are slow but optional). Mitigation: measure cycle time both with and without evolve, and report both.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

Current observability: the autodev report in `lib/crates/raspberry-supervisor/src/autodev.rs` tracks cycle count, dispatched lanes, and stop reason. It does NOT track: cycle time, dispatch rate, landing rate, cost, or failure classification.

The `raspberry status` command in `lib/crates/raspberry-cli/src/main.rs` shows lane-level status but no operational metrics.

Proposed KPIs:

| Metric | Definition | Baseline Target |
|--------|-----------|-----------------|
| Cycle time (p50) | Median time per autodev cycle | <30 seconds |
| Cycle time (p95) | 95th percentile cycle time | <120 seconds |
| Dispatch rate | % of cycles that dispatch work | >50% |
| Landing rate | Lanes landed per 100 cycles | >5 |
| Cost per landed lane | LLM tokens * price / landed lanes | Measured (no target) |
| Failure breakdown | % of failures by classification | Reduce "generic" to <20% |

## Milestones

### Milestone 1: Define KPIs and instrument cycle timing

Add `cycle_start_time`, `cycle_end_time`, and `cycle_phase_times` (refresh, evaluate, dispatch, watch) to the autodev report. Use `std::time::Instant` for measurement.

Key file: `lib/crates/raspberry-supervisor/src/autodev.rs`

Proof command:

    cargo nextest run -p raspberry-supervisor -- autodev cycle_timing

### Milestone 2: Cost estimation

Add per-provider token tracking to the autodev report. Estimate cost using published per-token rates for each provider. Track tokens consumed per cycle and cumulative.

Key files:
- `lib/crates/raspberry-supervisor/src/autodev.rs`
- `lib/crates/fabro-llm/src/` (token count extraction from provider responses)

Proof command:

    cargo nextest run -p raspberry-supervisor -- autodev cost

### Milestone 3: Failure classification metrics

Add failure breakdown to the autodev report: count failures by classification (verify_cycle, provider_quota, proof_script, integration_target, generic). The "generic" category should shrink as classification improves.

Proof command:

    cargo nextest run -p raspberry-supervisor -- autodev failure_breakdown

### Milestone 4: Metrics summary in status output

Add `--metrics` flag to `raspberry status` that shows:
- Cycle time: p50, p95, max
- Dispatch rate: last 10 cycles, last 100 cycles
- Landing rate: lanes landed per 100 cycles
- Cost: total, per landed lane
- Failure breakdown: top 5 failure types

Proof command:

    cargo nextest run -p raspberry-cli -- status metrics

### Milestone 5: Establish baselines

Run 200-cycle autodev on rXMRbro. Record baselines for all KPIs. Write to `genesis/artifacts/performance-baselines.md`.

Proof command:

    target-local/release/raspberry autodev \
      --manifest /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml \
      --max-cycles 200 && \
    target-local/release/raspberry status \
      --manifest /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml \
      --metrics

## Validation and Acceptance

The plan is done when:
- All 6 KPIs are measured and visible in `raspberry status --metrics`
- Cycle time timing is granular by phase
- Cost estimation appears per provider
- Failure breakdown shows >80% classified (not generic)
- Baselines are documented from a real 200-cycle run


Workflow archetype: implement

Review profile: standard

Active plan:
- `genesis/plans/001-master-plan.md`

Active spec:
- `genesis/SPEC.md`

Proof commands:
- `cargo nextest run -p raspberry-cli -- status metrics`

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
- layout_invariants_complete: yes|no
- slice_decomposition_respected: yes|no
If any mandatory security field is `no`, set `merge_ready: no`.

Review stage ownership:
- you may write or replace `.fabro-work/promotion.md` in this stage
- read `.fabro-work/quality.md` before deciding `merge_ready`
- when the slice is security-sensitive, perform a Nemesis-style pass: first-principles assumption challenge plus coupled-state consistency review
- include security findings in the review verdict when the slice touches trust boundaries, keys, funds, auth, control-plane behavior, or external process control
- prefer not to modify source code here unless a tiny correction is required to make the review judgment truthful
