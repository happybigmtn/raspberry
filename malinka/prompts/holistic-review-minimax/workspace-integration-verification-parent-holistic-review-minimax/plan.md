# workspace-integration-verification Holistic Review Minimax — Plan

Lane: `workspace-integration-verification-parent-holistic-review-minimax`

Goal:
- First-pass holistic parent review for integrated plan `workspace-integration-verification`.

Integrated child units:
- workspace-integration-verification-blueprintprotocol-and-contract-lane-generation, workspace-integration-verification-consistency-challenge-prompts, workspace-integration-verification-live-validation-on-rxmrbro

This is the breadth-first `/review` style pass. Inspect the integrated diff, parent plan intent, landed child artifacts, and the current trunk state together.

Required outputs:
- `holistic-review.md` with structured findings across correctness, trust boundaries, UX, performance, deployability, and documentation
- `finding-index.json` with normalized findings, severities, and touched surfaces
- `remediation-plan.md` with concrete follow-up work or explicit justification for no action
- `promotion.md` with a first-pass ready/not-ready verdict

Do not merely summarize child artifacts. Normalize the state of the whole parent implementation.

Context:
- Integrated child units:
- workspace-integration-verification-blueprintprotocol-and-contract-lane-generation, workspace-integration-verification-consistency-challenge-prompts, workspace-integration-verification-live-validation-on-rxmrbro

Required outputs:
- holistic-review.md
- finding-index.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/workspace-integration-verification-parent-holistic-review-minimax/holistic-review.md`
- `.raspberry/portfolio/workspace-integration-verification-parent-holistic-review-minimax/finding-index.json`
- `.raspberry/portfolio/workspace-integration-verification-parent-holistic-review-minimax/remediation-plan.md`
- `.raspberry/portfolio/workspace-integration-verification-parent-holistic-review-minimax/promotion.md`
