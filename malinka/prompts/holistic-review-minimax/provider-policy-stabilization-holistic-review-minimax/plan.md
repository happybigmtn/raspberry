# provider-policy-stabilization Holistic Review Minimax — Plan

Lane: `provider-policy-stabilization-holistic-review-minimax`

Goal:
- First-pass holistic parent review for integrated plan `provider-policy-stabilization`.

Integrated child units:
- provider-policy-stabilization-audit-model-selection-leaks, provider-policy-stabilization-live-validation, provider-policy-stabilization-provider-health-in-status-output, provider-policy-stabilization-quota-detection-and-graceful-fallback, provider-policy-stabilization-usage-tracking

This is the breadth-first `/review` style pass. Inspect the integrated diff, parent plan intent, landed child artifacts, and the current trunk state together.

Required outputs:
- `holistic-review.md` with structured findings across correctness, trust boundaries, UX, performance, deployability, and documentation
- `finding-index.json` with normalized findings, severities, and touched surfaces
- `remediation-plan.md` with concrete follow-up work or explicit justification for no action
- `promotion.md` with a first-pass ready/not-ready verdict

Do not merely summarize child artifacts. Normalize the state of the whole parent implementation.

Context:
- Integrated child units:
- provider-policy-stabilization-audit-model-selection-leaks, provider-policy-stabilization-live-validation, provider-policy-stabilization-provider-health-in-status-output, provider-policy-stabilization-quota-detection-and-graceful-fallback, provider-policy-stabilization-usage-tracking

Required outputs:
- holistic-review.md
- finding-index.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/provider-policy-stabilization-holistic-review-minimax/holistic-review.md`
- `.raspberry/portfolio/provider-policy-stabilization-holistic-review-minimax/finding-index.json`
- `.raspberry/portfolio/provider-policy-stabilization-holistic-review-minimax/remediation-plan.md`
- `.raspberry/portfolio/provider-policy-stabilization-holistic-review-minimax/promotion.md`
