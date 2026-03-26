# workspace-integration-verification Holistic Deep Review — Plan

Lane: `workspace-integration-verification-holistic-review-deep`

Goal:
- Deep synthesis pass for integrated parent plan `workspace-integration-verification`.

Integrated child units:
- workspace-integration-verification-blueprintprotocol-and-contract-lane-generation, workspace-integration-verification-consistency-challenge-prompts, workspace-integration-verification-live-validation-on-rxmrbro

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
- workspace-integration-verification-blueprintprotocol-and-contract-lane-generation, workspace-integration-verification-consistency-challenge-prompts, workspace-integration-verification-live-validation-on-rxmrbro

Required outputs:
- deep-review.md
- finding-deltas.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/workspace-integration-verification-holistic-review-deep/deep-review.md`
- `.raspberry/portfolio/workspace-integration-verification-holistic-review-deep/finding-deltas.json`
- `.raspberry/portfolio/workspace-integration-verification-holistic-review-deep/remediation-plan.md`
- `.raspberry/portfolio/workspace-integration-verification-holistic-review-deep/promotion.md`
