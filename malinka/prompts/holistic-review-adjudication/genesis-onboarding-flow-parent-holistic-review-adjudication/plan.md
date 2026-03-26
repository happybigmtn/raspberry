# genesis-onboarding-flow Holistic Adjudication — Plan

Lane: `genesis-onboarding-flow-parent-holistic-review-adjudication`

Goal:
- Final parent adjudication pass for integrated plan `genesis-onboarding-flow`.

Integrated child units:
- genesis-onboarding-flow-audit-genesis-implementation, genesis-onboarding-flow-command-surface-and-runtime-validation, genesis-onboarding-flow-operator-quickstart-documentation, genesis-onboarding-flow-repo-detection-and-adaptation, genesis-onboarding-flow-test-on-unfamiliar-repo, genesis-onboarding-flow-validation-step

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
- genesis-onboarding-flow-audit-genesis-implementation, genesis-onboarding-flow-command-surface-and-runtime-validation, genesis-onboarding-flow-operator-quickstart-documentation, genesis-onboarding-flow-repo-detection-and-adaptation, genesis-onboarding-flow-test-on-unfamiliar-repo, genesis-onboarding-flow-validation-step

Required outputs:
- adjudication-verdict.md
- confirmed-findings.json
- rejected-findings.json
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/genesis-onboarding-flow-parent-holistic-review-adjudication/adjudication-verdict.md`
- `.raspberry/portfolio/genesis-onboarding-flow-parent-holistic-review-adjudication/confirmed-findings.json`
- `.raspberry/portfolio/genesis-onboarding-flow-parent-holistic-review-adjudication/rejected-findings.json`
- `.raspberry/portfolio/genesis-onboarding-flow-parent-holistic-review-adjudication/promotion.md`
