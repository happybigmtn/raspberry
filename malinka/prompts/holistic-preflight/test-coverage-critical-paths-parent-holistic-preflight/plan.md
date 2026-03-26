# test-coverage-critical-paths Holistic Preflight — Plan

Lane: `test-coverage-critical-paths-parent-holistic-preflight`

Goal:
- Preflight the integrated parent plan `test-coverage-critical-paths` before holistic review.

Integrated child units:
- test-coverage-critical-paths-autodev-integration-test, test-coverage-critical-paths-ci-preservation-and-hardening, test-coverage-critical-paths-fabro-db-baseline-tests, test-coverage-critical-paths-minimal-coverage-for-fabro-mcp-and-fabro-github, test-coverage-critical-paths-raspberry-supervisor-edge-case-tests, test-coverage-critical-paths-synthesis-runtime-regression-tests

Your job:
1. Confirm every child integration artifact exists and is readable.
2. Confirm child review artifacts exist where available.
3. Record the exact integrated surface area that the parent gauntlet must inspect.
4. Call out any missing evidence, stale artifacts, or ambiguous ownership before expensive parent review begins.

Required durable artifacts:
- `verification.md` (what was checked, what artifacts were present, what is missing)
- `review.md` (a concise go/no-go summary for parent holistic review)

This lane is command-driven and report-first. Do not modify product code.

Context:
- Integrated child units:
- test-coverage-critical-paths-autodev-integration-test, test-coverage-critical-paths-ci-preservation-and-hardening, test-coverage-critical-paths-fabro-db-baseline-tests, test-coverage-critical-paths-minimal-coverage-for-fabro-mcp-and-fabro-github, test-coverage-critical-paths-raspberry-supervisor-edge-case-tests, test-coverage-critical-paths-synthesis-runtime-regression-tests

Required outputs:
- verification.md
- review.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/test-coverage-critical-paths-parent-holistic-preflight/verification.md`
- `.raspberry/portfolio/test-coverage-critical-paths-parent-holistic-preflight/review.md`
