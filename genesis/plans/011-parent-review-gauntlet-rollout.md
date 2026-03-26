# Parent Review Gauntlet Live Rollout

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, the parent review gauntlet (holistic-preflight → minimax-review → deep-review → adjudication → conditional stages → ship-readiness → document-release → retro) runs end-to-end on a real plan in a live autodev session. A plan can only be considered fully shipped after it survives the entire parent gauntlet. The operator can see the gauntlet producing real artifacts, not just synthesized lanes that exist on disk.

The proof is: in an rXMRbro autodev run, at least one plan reaches the parent gauntlet stages, and the gauntlet produces real review artifacts (preflight report, deep review, adjudication verdict, ship-readiness decision).

Provenance: This plan carries forward `plans/032526-parent-holistic-review-shipping-gauntlet.md` with live rollout milestones. The synthesis implementation is complete (committed 2026-03-26). The remaining work is live validation and workflow template refinement.

## Progress

- [x] Verify parent gauntlet lanes exist in regenerated rXMRbro package
- [ ] Run autodev until at least one plan reaches parent gauntlet stages
- [ ] Validate preflight, deep review, and adjudication produce real artifacts
- [ ] Validate conditional stages (design-review, qa, cso) trigger correctly
- [ ] Decide on dedicated parent workflow template vs recurring-report reuse
- [ ] Ship-readiness gate produces go/no-go verdict

## Surprises & Discoveries

(To be updated — carry forward from parent plan)

- From parent plan: parent `plan-review` lane arrives too late — child code is already on trunk before parent review runs. The gauntlet addresses this by running earlier stages (preflight) while children are still integrating.

## Decision Log

- Decision: Validate the existing synthesis implementation before adding new workflow templates.
  Rationale: The parent gauntlet synthesis is complete and generates the right lanes. The risk is that the generated workflows don't work correctly at runtime, not that the wrong lanes are generated. Live validation is the priority.
  Date/Author: 2026-03-26 / Genesis

- Decision: Keep recurring-report template for the first live rollout; defer dedicated parent workflow template to a follow-on plan.
  Rationale: The existing template works for report-style stages. A dedicated template is a nice-to-have that shouldn't block the first live validation.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: Parent gauntlet stages may time out because they review the aggregate of all child code (potentially thousands of lines). Mitigation: set generous timeouts for parent stages, and ensure the deep-review stage gets the diff summary rather than the full diff.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

The parent gauntlet synthesis lives in `lib/crates/fabro-synthesis/src/render.rs`. It generates these lanes for each plan prefix that has child implementation lanes:

```
holistic-preflight ──> holistic-review-minimax ──> holistic-review-deep
     |                                                     |
     v                                                     v
holistic-review-adjudication                    (conditional:)
     |                                          design-review (if UI)
     v                                          qa (if user-facing)
ship-readiness ──> document-release ──> retro   cso (if trust-sensitive)
                                                benchmark (if perf-sensitive)
```

Provider assignment:
- Minimax first pass: MiniMax M2.7
- Deep review: Opus 4.6 preferred, Codex fallback
- Adjudication: Codex preferred, Opus fallback
- Conditional stages: profile-appropriate model

The generated lanes are already visible in the regenerated rXMRbro package as units like `roulette-holistic-preflight`, `roulette-ship-readiness`, `roulette-document-release`, and `roulette-retro`. The remaining risk is runtime execution, artifact quality, and correct triggering order — not whether synthesis emits the families at all.

## Milestones

### Milestone 1: Verify gauntlet lanes in package

Regenerate rXMRbro package and confirm parent gauntlet lanes exist with correct dependencies.

Proof command:

    target-local/release/fabro --no-upgrade-check synth create \
      --target-repo /home/r/coding/rXMRbro --program rxmragent \
      --no-decompose --no-review && \
    grep -c "holistic-preflight\|holistic-review\|ship-readiness\|document-release\|retro" \
      /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml

Expected: >0 matches for each stage type.

### Milestone 2: Live gauntlet execution

Run autodev on rXMRbro until at least one plan's child lanes complete, triggering the parent gauntlet. Monitor with `raspberry tui`.

Proof command:

    target-local/release/raspberry autodev \
      --manifest /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml \
      --max-cycles 100 2>&1 | grep -E "holistic|preflight|ship-readiness"

### Milestone 3: Artifact validation

Verify that the gauntlet stages produce real artifacts:
- Preflight: `outputs/{plan}-holistic-preflight/spec.md` or `review.md`
- Deep review: `outputs/{plan}-holistic-review-deep/review.md`
- Adjudication: `outputs/{plan}-holistic-review-adjudication/review.md`
- Ship-readiness: `outputs/{plan}-ship-readiness/spec.md`

Proof command:

    find /home/r/coding/rXMRbro/outputs -name "*.md" -path "*holistic*" | head -5

### Milestone 4: Conditional stage trigger validation

Verify that conditional stages trigger correctly:
- `design-review` triggers only for plans with UI-tagged surfaces
- `qa` triggers for user-facing plans
- `cso` triggers for trust-sensitive plans

Proof command:

    grep -c "design-review\|qa\|cso" /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml

### Milestone 5: Ship-readiness verdict

Verify that the ship-readiness stage produces a go/no-go verdict that blocks or allows the `document-release` and `retro` stages.

Proof command:

    # After gauntlet runs:
    find /home/r/coding/rXMRbro/outputs -name "*.md" -path "*ship-readiness*" | head -1 | xargs grep -c "ship_ready"

## Validation and Acceptance

The plan is done when:
- Parent gauntlet stages dispatch and produce artifacts in a live run
- Conditional stages trigger based on plan surface tags
- Ship-readiness produces a go/no-go verdict
- document-release runs after ship-readiness passes
