# genesis-onboarding-flow Holistic Review Minimax — Plan

Lane: `genesis-onboarding-flow-parent-holistic-review-minimax`

Goal:
- First-pass holistic parent review for integrated plan `genesis-onboarding-flow`.

Integrated child units:
- genesis-onboarding-flow-audit-genesis-implementation, genesis-onboarding-flow-command-surface-and-runtime-validation, genesis-onboarding-flow-operator-quickstart-documentation, genesis-onboarding-flow-repo-detection-and-adaptation, genesis-onboarding-flow-test-on-unfamiliar-repo, genesis-onboarding-flow-validation-step

This is the breadth-first `/review` style pass. Inspect the integrated diff, parent plan intent, landed child artifacts, and the current trunk state together.

Required outputs:
- `holistic-review.md` with structured findings across correctness, trust boundaries, UX, performance, deployability, and documentation
- `finding-index.json` with normalized findings, severities, and touched surfaces
- `remediation-plan.md` with concrete follow-up work or explicit justification for no action
- `promotion.md` with a first-pass ready/not-ready verdict

Do not merely summarize child artifacts. Normalize the state of the whole parent implementation.

Context:
- Integrated child units:
- genesis-onboarding-flow-audit-genesis-implementation, genesis-onboarding-flow-command-surface-and-runtime-validation, genesis-onboarding-flow-operator-quickstart-documentation, genesis-onboarding-flow-repo-detection-and-adaptation, genesis-onboarding-flow-test-on-unfamiliar-repo, genesis-onboarding-flow-validation-step

Required outputs:
- holistic-review.md
- finding-index.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/genesis-onboarding-flow-parent-holistic-review-minimax/holistic-review.md`
- `.raspberry/portfolio/genesis-onboarding-flow-parent-holistic-review-minimax/finding-index.json`
- `.raspberry/portfolio/genesis-onboarding-flow-parent-holistic-review-minimax/remediation-plan.md`
- `.raspberry/portfolio/genesis-onboarding-flow-parent-holistic-review-minimax/promotion.md`
