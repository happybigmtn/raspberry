# plan-completion-and-adversarial-review Holistic Deep Review — Plan

Lane: `plan-completion-and-adversarial-review-parent-holistic-review-deep`

Goal:
- Deep synthesis pass for integrated parent plan `plan-completion-and-adversarial-review`.

Integrated child units:
- plan-completion-and-adversarial-review-3-step-adversarial-review-prompt, plan-completion-and-adversarial-review-live-validation, plan-completion-and-adversarial-review-meta-review-feedback-loop, plan-completion-and-adversarial-review-plan-completion-detection, plan-completion-and-adversarial-review-registry-driven-plan-review-generation

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
- plan-completion-and-adversarial-review-3-step-adversarial-review-prompt, plan-completion-and-adversarial-review-live-validation, plan-completion-and-adversarial-review-meta-review-feedback-loop, plan-completion-and-adversarial-review-plan-completion-detection, plan-completion-and-adversarial-review-registry-driven-plan-review-generation

Required outputs:
- deep-review.md
- finding-deltas.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/plan-completion-and-adversarial-review-parent-holistic-review-deep/deep-review.md`
- `.raspberry/portfolio/plan-completion-and-adversarial-review-parent-holistic-review-deep/finding-deltas.json`
- `.raspberry/portfolio/plan-completion-and-adversarial-review-parent-holistic-review-deep/remediation-plan.md`
- `.raspberry/portfolio/plan-completion-and-adversarial-review-parent-holistic-review-deep/promotion.md`
