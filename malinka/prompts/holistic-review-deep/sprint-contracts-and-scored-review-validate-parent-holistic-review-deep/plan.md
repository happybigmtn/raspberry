# sprint-contracts-and-scored-review-validate Holistic Deep Review — Plan

Lane: `sprint-contracts-and-scored-review-validate-parent-holistic-review-deep`

Goal:
- Deep synthesis pass for integrated parent plan `sprint-contracts-and-scored-review-validate`.

Integrated child units:
- sprint-contracts-and-scored-review-validate-contract-stage, sprint-contracts-and-scored-review-validate-scored-review

Re-read the full parent state after the Minimax pass.
- collapse duplicates and sharpen weak evidence
- identify systemic edge cases or cross-child interactions that the first pass may have missed
- refine the remediation plan where the first pass was broad or ambiguous
- preserve uncertainty explicitly when evidence is incomplete

Required outputs:
- `deep-review.md`
- `finding-deltas.json`
- `remediation-plan.md`
- `promotion.md`

This lane prefers Opus 4.6 and may fall back to Codex if needed.

Context:
- Integrated child units:
- sprint-contracts-and-scored-review-validate-contract-stage, sprint-contracts-and-scored-review-validate-scored-review

Required outputs:
- deep-review.md
- finding-deltas.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/sprint-contracts-and-scored-review-validate-parent-holistic-review-deep/deep-review.md`
- `.raspberry/portfolio/sprint-contracts-and-scored-review-validate-parent-holistic-review-deep/finding-deltas.json`
- `.raspberry/portfolio/sprint-contracts-and-scored-review-validate-parent-holistic-review-deep/remediation-plan.md`
- `.raspberry/portfolio/sprint-contracts-and-scored-review-validate-parent-holistic-review-deep/promotion.md`
