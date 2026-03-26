# provider-policy-stabilization Investigate — Plan

Lane: `provider-policy-stabilization-investigate`

Goal:
- Root-cause investigation for integrated parent plan `provider-policy-stabilization`.

Run this lane as the non-interactive `/investigate` stage for plans whose risk profile or review findings justify deeper causal analysis.
Explain what actually failed or could fail, what evidence supports that judgment, and what remediation path follows from the root cause.

Required outputs:
- `investigation.md`
- `promotion.md`

Do not substitute fixes for root cause analysis.

Context:
- Required outputs:
- investigation.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/provider-policy-stabilization-investigate/investigation.md`
- `.raspberry/portfolio/provider-policy-stabilization-investigate/promotion.md`
