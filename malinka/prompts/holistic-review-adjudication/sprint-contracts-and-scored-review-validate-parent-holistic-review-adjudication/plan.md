# sprint-contracts-and-scored-review-validate Holistic Adjudication — Plan

Lane: `sprint-contracts-and-scored-review-validate-parent-holistic-review-adjudication`

Goal:
- Final parent adjudication pass for integrated plan `sprint-contracts-and-scored-review-validate`.

Integrated child units:
- sprint-contracts-and-scored-review-validate-contract-stage, sprint-contracts-and-scored-review-validate-scored-review

Re-adjudicate the Minimax and deep-review findings.
- confirm which findings are real and blocking
- reject weak or duplicate findings explicitly
- preserve disagreements rather than flattening them
- issue the final parent ship/no-ship judgment for this integrated plan

Required outputs:
- `adjudication-verdict.md`
- `confirmed-findings.json`
- `rejected-findings.json`
- `promotion.md`

This lane prefers Codex and may fall back to Opus 4.6 if needed.

Context:
- Integrated child units:
- sprint-contracts-and-scored-review-validate-contract-stage, sprint-contracts-and-scored-review-validate-scored-review

Required outputs:
- adjudication-verdict.md
- confirmed-findings.json
- rejected-findings.json
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/sprint-contracts-and-scored-review-validate-parent-holistic-review-adjudication/adjudication-verdict.md`
- `.raspberry/portfolio/sprint-contracts-and-scored-review-validate-parent-holistic-review-adjudication/confirmed-findings.json`
- `.raspberry/portfolio/sprint-contracts-and-scored-review-validate-parent-holistic-review-adjudication/rejected-findings.json`
- `.raspberry/portfolio/sprint-contracts-and-scored-review-validate-parent-holistic-review-adjudication/promotion.md`
