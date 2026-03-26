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
