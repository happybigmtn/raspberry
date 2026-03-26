# web-dashboard-raspberry-integration Holistic Adjudication — Plan

Lane: `web-dashboard-raspberry-integration-holistic-review-adjudication`

Goal:
- Final parent adjudication pass for integrated plan `web-dashboard-raspberry-integration`.

Integrated child units:
- web-dashboard-raspberry-integration-autodev-status-api-endpoint, web-dashboard-raspberry-integration-design-review, web-dashboard-raspberry-integration-lane-detail-component, web-dashboard-raspberry-integration-sse-live-updates

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
- web-dashboard-raspberry-integration-autodev-status-api-endpoint, web-dashboard-raspberry-integration-design-review, web-dashboard-raspberry-integration-lane-detail-component, web-dashboard-raspberry-integration-sse-live-updates

Required outputs:
- adjudication-verdict.md
- confirmed-findings.json
- rejected-findings.json
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/web-dashboard-raspberry-integration-holistic-review-adjudication/adjudication-verdict.md`
- `.raspberry/portfolio/web-dashboard-raspberry-integration-holistic-review-adjudication/confirmed-findings.json`
- `.raspberry/portfolio/web-dashboard-raspberry-integration-holistic-review-adjudication/rejected-findings.json`
- `.raspberry/portfolio/web-dashboard-raspberry-integration-holistic-review-adjudication/promotion.md`
