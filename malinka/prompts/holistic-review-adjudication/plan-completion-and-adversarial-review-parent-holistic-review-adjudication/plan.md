# plan-completion-and-adversarial-review Holistic Adjudication — Plan

Lane: `plan-completion-and-adversarial-review-parent-holistic-review-adjudication`

Goal:
- Final parent adjudication pass for integrated plan `plan-completion-and-adversarial-review`.

Integrated child units:
- plan-completion-and-adversarial-review-3-step-adversarial-review-prompt, plan-completion-and-adversarial-review-live-validation, plan-completion-and-adversarial-review-meta-review-feedback-loop, plan-completion-and-adversarial-review-plan-completion-detection, plan-completion-and-adversarial-review-registry-driven-plan-review-generation

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
- plan-completion-and-adversarial-review-3-step-adversarial-review-prompt, plan-completion-and-adversarial-review-live-validation, plan-completion-and-adversarial-review-meta-review-feedback-loop, plan-completion-and-adversarial-review-plan-completion-detection, plan-completion-and-adversarial-review-registry-driven-plan-review-generation

Required outputs:
- adjudication-verdict.md
- confirmed-findings.json
- rejected-findings.json
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/plan-completion-and-adversarial-review-parent-holistic-review-adjudication/adjudication-verdict.md`
- `.raspberry/portfolio/plan-completion-and-adversarial-review-parent-holistic-review-adjudication/confirmed-findings.json`
- `.raspberry/portfolio/plan-completion-and-adversarial-review-parent-holistic-review-adjudication/rejected-findings.json`
- `.raspberry/portfolio/plan-completion-and-adversarial-review-parent-holistic-review-adjudication/promotion.md`
