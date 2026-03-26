# test-coverage-critical-paths QA — Plan

Lane: `test-coverage-critical-paths-qa`

Goal:
- Integrated QA pass for parent plan `test-coverage-critical-paths`.

Exercise the end-to-end parent flow as far as this repo can support non-interactively. Focus on integrated regressions, broken workflows, missing validation, and ship-risk.

Required outputs:
- `qa-report.md`
- `promotion.md`

Tag findings by severity and include concrete repro steps.

Context:
- Required outputs:
- qa-report.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/test-coverage-critical-paths-qa/qa-report.md`
- `.raspberry/portfolio/test-coverage-critical-paths-qa/promotion.md`
