# web-dashboard-raspberry-integration-plan-matrix Holistic Preflight — Plan

Lane: `web-dashboard-raspberry-integration-plan-matrix-parent-holistic-preflight`

Goal:
- Preflight the integrated parent plan `web-dashboard-raspberry-integration-plan-matrix` before holistic review.

Integrated child units:
- web-dashboard-raspberry-integration-plan-matrix-api-endpoint, web-dashboard-raspberry-integration-plan-matrix-react-component

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
- web-dashboard-raspberry-integration-plan-matrix-api-endpoint, web-dashboard-raspberry-integration-plan-matrix-react-component

Required outputs:
- verification.md
- review.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/web-dashboard-raspberry-integration-plan-matrix-parent-holistic-preflight/verification.md`
- `.raspberry/portfolio/web-dashboard-raspberry-integration-plan-matrix-parent-holistic-preflight/review.md`
