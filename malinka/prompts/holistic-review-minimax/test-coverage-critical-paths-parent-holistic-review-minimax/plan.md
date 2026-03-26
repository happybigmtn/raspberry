# test-coverage-critical-paths Holistic Review Minimax — Plan

Lane: `test-coverage-critical-paths-parent-holistic-review-minimax`

Goal:
- First-pass holistic parent review for integrated plan `test-coverage-critical-paths`.

Integrated child units:
- test-coverage-critical-paths-autodev-integration-test, test-coverage-critical-paths-ci-preservation-and-hardening, test-coverage-critical-paths-fabro-db-baseline-tests, test-coverage-critical-paths-minimal-coverage-for-fabro-mcp-and-fabro-github, test-coverage-critical-paths-raspberry-supervisor-edge-case-tests, test-coverage-critical-paths-synthesis-runtime-regression-tests

This is the breadth-first `/review` style pass. Inspect the integrated diff, parent plan intent, landed child artifacts, and the current trunk state together.

Required outputs:
- `holistic-review.md` with structured findings across correctness, trust boundaries, UX, performance, deployability, and documentation
- `finding-index.json` with normalized findings, severities, and touched surfaces
- `remediation-plan.md` with concrete follow-up work or explicit justification for no action
- `promotion.md` with a first-pass ready/not-ready verdict

Do not merely summarize child artifacts. Normalize the state of the whole parent implementation.

Context:
- Integrated child units:
- test-coverage-critical-paths-autodev-integration-test, test-coverage-critical-paths-ci-preservation-and-hardening, test-coverage-critical-paths-fabro-db-baseline-tests, test-coverage-critical-paths-minimal-coverage-for-fabro-mcp-and-fabro-github, test-coverage-critical-paths-raspberry-supervisor-edge-case-tests, test-coverage-critical-paths-synthesis-runtime-regression-tests

Required outputs:
- holistic-review.md
- finding-index.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/test-coverage-critical-paths-parent-holistic-review-minimax/holistic-review.md`
- `.raspberry/portfolio/test-coverage-critical-paths-parent-holistic-review-minimax/finding-index.json`
- `.raspberry/portfolio/test-coverage-critical-paths-parent-holistic-review-minimax/remediation-plan.md`
- `.raspberry/portfolio/test-coverage-critical-paths-parent-holistic-review-minimax/promotion.md`
