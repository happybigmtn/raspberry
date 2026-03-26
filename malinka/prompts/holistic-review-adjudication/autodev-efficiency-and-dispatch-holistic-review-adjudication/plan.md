# autodev-efficiency-and-dispatch Holistic Adjudication — Plan

Lane: `autodev-efficiency-and-dispatch-holistic-review-adjudication`

Goal:
- Final parent adjudication pass for integrated plan `autodev-efficiency-and-dispatch`.

Integrated child units:
- autodev-efficiency-and-dispatch-add-dispatch-state-telemetry, autodev-efficiency-and-dispatch-decouple-evolve-from-dispatch-and-consume-the-budget-greedily, autodev-efficiency-and-dispatch-freeze-the-current-failure-modes-into-reproducible-tests, autodev-efficiency-and-dispatch-live-validation, autodev-efficiency-and-dispatch-make-autodev-runtime-paths-self-consistent, autodev-efficiency-and-dispatch-reconcile-stale-running-and-failed-lane-truth

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
- autodev-efficiency-and-dispatch-add-dispatch-state-telemetry, autodev-efficiency-and-dispatch-decouple-evolve-from-dispatch-and-consume-the-budget-greedily, autodev-efficiency-and-dispatch-freeze-the-current-failure-modes-into-reproducible-tests, autodev-efficiency-and-dispatch-live-validation, autodev-efficiency-and-dispatch-make-autodev-runtime-paths-self-consistent, autodev-efficiency-and-dispatch-reconcile-stale-running-and-failed-lane-truth

Required outputs:
- adjudication-verdict.md
- confirmed-findings.json
- rejected-findings.json
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/autodev-efficiency-and-dispatch-holistic-review-adjudication/adjudication-verdict.md`
- `.raspberry/portfolio/autodev-efficiency-and-dispatch-holistic-review-adjudication/confirmed-findings.json`
- `.raspberry/portfolio/autodev-efficiency-and-dispatch-holistic-review-adjudication/rejected-findings.json`
- `.raspberry/portfolio/autodev-efficiency-and-dispatch-holistic-review-adjudication/promotion.md`
