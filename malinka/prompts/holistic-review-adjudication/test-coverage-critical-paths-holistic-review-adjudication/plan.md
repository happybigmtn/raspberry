# test-coverage-critical-paths Holistic Adjudication — Plan

Lane: `test-coverage-critical-paths-holistic-review-adjudication`

Goal:
- Final parent adjudication pass for integrated plan `test-coverage-critical-paths`.

Integrated child units:
- test-coverage-critical-paths-autodev-integration-test, test-coverage-critical-paths-ci-preservation-and-hardening, test-coverage-critical-paths-fabro-db-baseline-tests, test-coverage-critical-paths-minimal-coverage-for-fabro-mcp-and-fabro-github, test-coverage-critical-paths-raspberry-supervisor-edge-case-tests, test-coverage-critical-paths-synthesis-runtime-regression-tests

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
- test-coverage-critical-paths-autodev-integration-test, test-coverage-critical-paths-ci-preservation-and-hardening, test-coverage-critical-paths-fabro-db-baseline-tests, test-coverage-critical-paths-minimal-coverage-for-fabro-mcp-and-fabro-github, test-coverage-critical-paths-raspberry-supervisor-edge-case-tests, test-coverage-critical-paths-synthesis-runtime-regression-tests

Required outputs:
- adjudication-verdict.md
- confirmed-findings.json
- rejected-findings.json
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/test-coverage-critical-paths-holistic-review-adjudication/adjudication-verdict.md`
- `.raspberry/portfolio/test-coverage-critical-paths-holistic-review-adjudication/confirmed-findings.json`
- `.raspberry/portfolio/test-coverage-critical-paths-holistic-review-adjudication/rejected-findings.json`
- `.raspberry/portfolio/test-coverage-critical-paths-holistic-review-adjudication/promotion.md`
