# error-handling-hardening Holistic Adjudication — Plan

Lane: `error-handling-hardening-parent-holistic-review-adjudication`

Goal:
- Final parent adjudication pass for integrated plan `error-handling-hardening`.

Integrated child units:
- error-handling-hardening-audit-autodev-critical-path, error-handling-hardening-integration-validation, error-handling-hardening-live-autodev-validation

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
- error-handling-hardening-audit-autodev-critical-path, error-handling-hardening-integration-validation, error-handling-hardening-live-autodev-validation

Required outputs:
- adjudication-verdict.md
- confirmed-findings.json
- rejected-findings.json
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/error-handling-hardening-parent-holistic-review-adjudication/adjudication-verdict.md`
- `.raspberry/portfolio/error-handling-hardening-parent-holistic-review-adjudication/confirmed-findings.json`
- `.raspberry/portfolio/error-handling-hardening-parent-holistic-review-adjudication/rejected-findings.json`
- `.raspberry/portfolio/error-handling-hardening-parent-holistic-review-adjudication/promotion.md`
