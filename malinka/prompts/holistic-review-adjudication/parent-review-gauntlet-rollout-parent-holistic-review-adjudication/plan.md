# parent-review-gauntlet-rollout Holistic Adjudication — Plan

Lane: `parent-review-gauntlet-rollout-parent-holistic-review-adjudication`

Goal:
- Final parent adjudication pass for integrated plan `parent-review-gauntlet-rollout`.

Integrated child units:
- parent-review-gauntlet-rollout-artifact-validation, parent-review-gauntlet-rollout-conditional-stage-trigger-validation, parent-review-gauntlet-rollout-live-gauntlet-execution, parent-review-gauntlet-rollout-ship-readiness-verdict, parent-review-gauntlet-rollout-verify-gauntlet-lanes-in-package

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
- parent-review-gauntlet-rollout-artifact-validation, parent-review-gauntlet-rollout-conditional-stage-trigger-validation, parent-review-gauntlet-rollout-live-gauntlet-execution, parent-review-gauntlet-rollout-ship-readiness-verdict, parent-review-gauntlet-rollout-verify-gauntlet-lanes-in-package

Required outputs:
- adjudication-verdict.md
- confirmed-findings.json
- rejected-findings.json
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/parent-review-gauntlet-rollout-parent-holistic-review-adjudication/adjudication-verdict.md`
- `.raspberry/portfolio/parent-review-gauntlet-rollout-parent-holistic-review-adjudication/confirmed-findings.json`
- `.raspberry/portfolio/parent-review-gauntlet-rollout-parent-holistic-review-adjudication/rejected-findings.json`
- `.raspberry/portfolio/parent-review-gauntlet-rollout-parent-holistic-review-adjudication/promotion.md`
