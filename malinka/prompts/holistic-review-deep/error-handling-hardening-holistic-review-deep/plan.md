# error-handling-hardening Holistic Deep Review — Plan

Lane: `error-handling-hardening-holistic-review-deep`

Goal:
- Deep synthesis pass for integrated parent plan `error-handling-hardening`.

Integrated child units:
- error-handling-hardening-audit-autodev-critical-path, error-handling-hardening-live-autodev-validation

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
- error-handling-hardening-audit-autodev-critical-path, error-handling-hardening-live-autodev-validation

Required outputs:
- deep-review.md
- finding-deltas.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/error-handling-hardening-holistic-review-deep/deep-review.md`
- `.raspberry/portfolio/error-handling-hardening-holistic-review-deep/finding-deltas.json`
- `.raspberry/portfolio/error-handling-hardening-holistic-review-deep/remediation-plan.md`
- `.raspberry/portfolio/error-handling-hardening-holistic-review-deep/promotion.md`
