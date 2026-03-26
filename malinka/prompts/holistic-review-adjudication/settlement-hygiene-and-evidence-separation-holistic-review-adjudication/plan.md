# settlement-hygiene-and-evidence-separation Holistic Adjudication — Plan

Lane: `settlement-hygiene-and-evidence-separation-holistic-review-adjudication`

Goal:
- Final parent adjudication pass for integrated plan `settlement-hygiene-and-evidence-separation`.

Integrated child units:
- settlement-hygiene-and-evidence-separation-allowed-path-metadata, settlement-hygiene-and-evidence-separation-ci-commit-hygiene-check, settlement-hygiene-and-evidence-separation-evidence-publication-separation, settlement-hygiene-and-evidence-separation-live-validation, settlement-hygiene-and-evidence-separation-verify-settlement-hygiene

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
- settlement-hygiene-and-evidence-separation-allowed-path-metadata, settlement-hygiene-and-evidence-separation-ci-commit-hygiene-check, settlement-hygiene-and-evidence-separation-evidence-publication-separation, settlement-hygiene-and-evidence-separation-live-validation, settlement-hygiene-and-evidence-separation-verify-settlement-hygiene

Required outputs:
- adjudication-verdict.md
- confirmed-findings.json
- rejected-findings.json
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/settlement-hygiene-and-evidence-separation-holistic-review-adjudication/adjudication-verdict.md`
- `.raspberry/portfolio/settlement-hygiene-and-evidence-separation-holistic-review-adjudication/confirmed-findings.json`
- `.raspberry/portfolio/settlement-hygiene-and-evidence-separation-holistic-review-adjudication/rejected-findings.json`
- `.raspberry/portfolio/settlement-hygiene-and-evidence-separation-holistic-review-adjudication/promotion.md`
