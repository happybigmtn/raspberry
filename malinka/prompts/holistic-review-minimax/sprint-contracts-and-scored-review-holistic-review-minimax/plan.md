# sprint-contracts-and-scored-review Holistic Review Minimax — Plan

Lane: `sprint-contracts-and-scored-review-holistic-review-minimax`

Goal:
- First-pass holistic parent review for integrated plan `sprint-contracts-and-scored-review`.

Integrated child units:
- sprint-contracts-and-scored-review-challenge-stage-a-b-assessment, sprint-contracts-and-scored-review-contract-completeness-metrics, sprint-contracts-and-scored-review-decision-and-implementation, sprint-contracts-and-scored-review-review-score-distribution

This is the breadth-first `/review` style pass. Inspect the integrated diff, parent plan intent, landed child artifacts, and the current trunk state together.

Required outputs:
- `holistic-review.md` with structured findings across correctness, trust boundaries, UX, performance, deployability, and documentation
- `finding-index.json` with normalized findings, severities, and touched surfaces
- `remediation-plan.md` with concrete follow-up work or explicit justification for no action
- `promotion.md` with a first-pass ready/not-ready verdict

Do not merely summarize child artifacts. Normalize the state of the whole parent implementation.

Context:
- Integrated child units:
- sprint-contracts-and-scored-review-challenge-stage-a-b-assessment, sprint-contracts-and-scored-review-contract-completeness-metrics, sprint-contracts-and-scored-review-decision-and-implementation, sprint-contracts-and-scored-review-review-score-distribution

Required outputs:
- holistic-review.md
- finding-index.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/sprint-contracts-and-scored-review-holistic-review-minimax/holistic-review.md`
- `.raspberry/portfolio/sprint-contracts-and-scored-review-holistic-review-minimax/finding-index.json`
- `.raspberry/portfolio/sprint-contracts-and-scored-review-holistic-review-minimax/remediation-plan.md`
- `.raspberry/portfolio/sprint-contracts-and-scored-review-holistic-review-minimax/promotion.md`
