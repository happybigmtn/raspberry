# release-preparation Holistic Adjudication — Plan

Lane: `release-preparation-parent-holistic-review-adjudication`

Goal:
- Final parent adjudication pass for integrated plan `release-preparation`.

Integrated child units:
- release-preparation-ci-hardening, release-preparation-fresh-install-test, release-preparation-readme-and-changelog, release-preparation-release-binary-build, release-preparation-security-audit, release-preparation-version-bump-and-tag

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
- release-preparation-ci-hardening, release-preparation-fresh-install-test, release-preparation-readme-and-changelog, release-preparation-release-binary-build, release-preparation-security-audit, release-preparation-version-bump-and-tag

Required outputs:
- adjudication-verdict.md
- confirmed-findings.json
- rejected-findings.json
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/release-preparation-parent-holistic-review-adjudication/adjudication-verdict.md`
- `.raspberry/portfolio/release-preparation-parent-holistic-review-adjudication/confirmed-findings.json`
- `.raspberry/portfolio/release-preparation-parent-holistic-review-adjudication/rejected-findings.json`
- `.raspberry/portfolio/release-preparation-parent-holistic-review-adjudication/promotion.md`
