# provider-policy-stabilization Land And Deploy — Plan

Lane: `provider-policy-stabilization-parent-land-and-deploy`

Goal:
- Deploy-aware verification for parent plan `provider-policy-stabilization`.

This lane exists only for plans whose integrated surface suggests deploy or service exposure. Record what would be landed, what health evidence exists, and any canary or rollout blockers.

Required outputs:
- `deploy-verification.md`
- `promotion.md`

Context:
- Required outputs:
- deploy-verification.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/provider-policy-stabilization-parent-land-and-deploy/deploy-verification.md`
- `.raspberry/portfolio/provider-policy-stabilization-parent-land-and-deploy/promotion.md`
