# documentation-and-operator-runbook Holistic Deep Review — Plan

Lane: `documentation-and-operator-runbook-holistic-review-deep`

Goal:
- Deep synthesis pass for integrated parent plan `documentation-and-operator-runbook`.

Integrated child units:
- documentation-and-operator-runbook-architecture-guide-for-contributors, documentation-and-operator-runbook-doc-freshness-audit, documentation-and-operator-runbook-operator-quickstart-guide, documentation-and-operator-runbook-raspberry-command-reference, documentation-and-operator-runbook-rewrite-readme, documentation-and-operator-runbook-troubleshooting-guide

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
- documentation-and-operator-runbook-architecture-guide-for-contributors, documentation-and-operator-runbook-doc-freshness-audit, documentation-and-operator-runbook-operator-quickstart-guide, documentation-and-operator-runbook-raspberry-command-reference, documentation-and-operator-runbook-rewrite-readme, documentation-and-operator-runbook-troubleshooting-guide

Required outputs:
- deep-review.md
- finding-deltas.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/documentation-and-operator-runbook-holistic-review-deep/deep-review.md`
- `.raspberry/portfolio/documentation-and-operator-runbook-holistic-review-deep/finding-deltas.json`
- `.raspberry/portfolio/documentation-and-operator-runbook-holistic-review-deep/remediation-plan.md`
- `.raspberry/portfolio/documentation-and-operator-runbook-holistic-review-deep/promotion.md`
