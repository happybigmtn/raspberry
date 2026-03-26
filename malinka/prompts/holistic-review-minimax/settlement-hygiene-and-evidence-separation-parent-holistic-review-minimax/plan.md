# settlement-hygiene-and-evidence-separation Holistic Review Minimax — Plan

Lane: `settlement-hygiene-and-evidence-separation-parent-holistic-review-minimax`

Goal:
- First-pass holistic parent review for integrated plan `settlement-hygiene-and-evidence-separation`.

Integrated child units:
- settlement-hygiene-and-evidence-separation-allowed-path-metadata, settlement-hygiene-and-evidence-separation-ci-commit-hygiene-check, settlement-hygiene-and-evidence-separation-evidence-publication-separation, settlement-hygiene-and-evidence-separation-live-validation, settlement-hygiene-and-evidence-separation-verify-settlement-hygiene

This is the breadth-first `/review` style pass. Inspect the integrated diff, parent plan intent, landed child artifacts, and the current trunk state together.

Required outputs:
- `holistic-review.md` with structured findings across correctness, trust boundaries, UX, performance, deployability, and documentation
- `finding-index.json` with normalized findings, severities, and touched surfaces
- `remediation-plan.md` with concrete follow-up work or explicit justification for no action
- `promotion.md` with a first-pass ready/not-ready verdict

Do not merely summarize child artifacts. Normalize the state of the whole parent implementation.

Context:
- Integrated child units:
- settlement-hygiene-and-evidence-separation-allowed-path-metadata, settlement-hygiene-and-evidence-separation-ci-commit-hygiene-check, settlement-hygiene-and-evidence-separation-evidence-publication-separation, settlement-hygiene-and-evidence-separation-live-validation, settlement-hygiene-and-evidence-separation-verify-settlement-hygiene

Required outputs:
- holistic-review.md
- finding-index.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/settlement-hygiene-and-evidence-separation-parent-holistic-review-minimax/holistic-review.md`
- `.raspberry/portfolio/settlement-hygiene-and-evidence-separation-parent-holistic-review-minimax/finding-index.json`
- `.raspberry/portfolio/settlement-hygiene-and-evidence-separation-parent-holistic-review-minimax/remediation-plan.md`
- `.raspberry/portfolio/settlement-hygiene-and-evidence-separation-parent-holistic-review-minimax/promotion.md`
