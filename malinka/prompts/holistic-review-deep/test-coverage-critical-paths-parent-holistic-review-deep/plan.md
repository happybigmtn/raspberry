# test-coverage-critical-paths Holistic Deep Review — Plan

Lane: `test-coverage-critical-paths-parent-holistic-review-deep`

Goal:
- Deep synthesis pass for integrated parent plan `test-coverage-critical-paths`.

Integrated child units:
- test-coverage-critical-paths-autodev-integration-test, test-coverage-critical-paths-ci-preservation-and-hardening, test-coverage-critical-paths-fabro-db-baseline-tests, test-coverage-critical-paths-minimal-coverage-for-fabro-mcp-and-fabro-github, test-coverage-critical-paths-raspberry-supervisor-edge-case-tests, test-coverage-critical-paths-synthesis-runtime-regression-tests

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
- test-coverage-critical-paths-autodev-integration-test, test-coverage-critical-paths-ci-preservation-and-hardening, test-coverage-critical-paths-fabro-db-baseline-tests, test-coverage-critical-paths-minimal-coverage-for-fabro-mcp-and-fabro-github, test-coverage-critical-paths-raspberry-supervisor-edge-case-tests, test-coverage-critical-paths-synthesis-runtime-regression-tests

Required outputs:
- deep-review.md
- finding-deltas.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/test-coverage-critical-paths-parent-holistic-review-deep/deep-review.md`
- `.raspberry/portfolio/test-coverage-critical-paths-parent-holistic-review-deep/finding-deltas.json`
- `.raspberry/portfolio/test-coverage-critical-paths-parent-holistic-review-deep/remediation-plan.md`
- `.raspberry/portfolio/test-coverage-critical-paths-parent-holistic-review-deep/promotion.md`
