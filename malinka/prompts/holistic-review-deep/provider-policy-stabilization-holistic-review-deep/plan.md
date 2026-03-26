# provider-policy-stabilization Holistic Deep Review — Plan

Lane: `provider-policy-stabilization-holistic-review-deep`

Goal:
- Deep synthesis pass for integrated parent plan `provider-policy-stabilization`.

Integrated child units:
- provider-policy-stabilization-audit-model-selection-leaks, provider-policy-stabilization-live-validation, provider-policy-stabilization-provider-health-in-status-output, provider-policy-stabilization-quota-detection-and-graceful-fallback, provider-policy-stabilization-usage-tracking

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
- provider-policy-stabilization-audit-model-selection-leaks, provider-policy-stabilization-live-validation, provider-policy-stabilization-provider-health-in-status-output, provider-policy-stabilization-quota-detection-and-graceful-fallback, provider-policy-stabilization-usage-tracking

Required outputs:
- deep-review.md
- finding-deltas.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/provider-policy-stabilization-holistic-review-deep/deep-review.md`
- `.raspberry/portfolio/provider-policy-stabilization-holistic-review-deep/finding-deltas.json`
- `.raspberry/portfolio/provider-policy-stabilization-holistic-review-deep/remediation-plan.md`
- `.raspberry/portfolio/provider-policy-stabilization-holistic-review-deep/promotion.md`
