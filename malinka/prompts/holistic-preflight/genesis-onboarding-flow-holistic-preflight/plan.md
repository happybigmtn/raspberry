# genesis-onboarding-flow Holistic Preflight — Plan

Lane: `genesis-onboarding-flow-holistic-preflight`

Goal:
- Preflight the integrated parent plan `genesis-onboarding-flow` before holistic review.

Integrated child units:
- genesis-onboarding-flow-audit-genesis-implementation, genesis-onboarding-flow-command-surface-and-runtime-validation, genesis-onboarding-flow-operator-quickstart-documentation, genesis-onboarding-flow-repo-detection-and-adaptation, genesis-onboarding-flow-test-on-unfamiliar-repo, genesis-onboarding-flow-validation-step

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
- genesis-onboarding-flow-audit-genesis-implementation, genesis-onboarding-flow-command-surface-and-runtime-validation, genesis-onboarding-flow-operator-quickstart-documentation, genesis-onboarding-flow-repo-detection-and-adaptation, genesis-onboarding-flow-test-on-unfamiliar-repo, genesis-onboarding-flow-validation-step

Required outputs:
- verification.md
- review.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/genesis-onboarding-flow-holistic-preflight/verification.md`
- `.raspberry/portfolio/genesis-onboarding-flow-holistic-preflight/review.md`
