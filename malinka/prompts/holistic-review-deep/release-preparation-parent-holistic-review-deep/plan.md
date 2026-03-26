# release-preparation Holistic Deep Review — Plan

Lane: `release-preparation-parent-holistic-review-deep`

Goal:
- Deep synthesis pass for integrated parent plan `release-preparation`.

Integrated child units:
- release-preparation-ci-hardening, release-preparation-fresh-install-test, release-preparation-readme-and-changelog, release-preparation-release-binary-build, release-preparation-security-audit, release-preparation-version-bump-and-tag

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
- release-preparation-ci-hardening, release-preparation-fresh-install-test, release-preparation-readme-and-changelog, release-preparation-release-binary-build, release-preparation-security-audit, release-preparation-version-bump-and-tag

Required outputs:
- deep-review.md
- finding-deltas.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/release-preparation-parent-holistic-review-deep/deep-review.md`
- `.raspberry/portfolio/release-preparation-parent-holistic-review-deep/finding-deltas.json`
- `.raspberry/portfolio/release-preparation-parent-holistic-review-deep/remediation-plan.md`
- `.raspberry/portfolio/release-preparation-parent-holistic-review-deep/promotion.md`
