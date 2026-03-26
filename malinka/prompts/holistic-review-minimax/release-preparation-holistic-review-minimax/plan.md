# release-preparation Holistic Review Minimax — Plan

Lane: `release-preparation-holistic-review-minimax`

Goal:
- First-pass holistic parent review for integrated plan `release-preparation`.

Integrated child units:
- release-preparation-ci-hardening, release-preparation-fresh-install-test, release-preparation-readme-and-changelog, release-preparation-release-binary-build, release-preparation-security-audit, release-preparation-version-bump-and-tag

This is the breadth-first `/review` style pass. Inspect the integrated diff, parent plan intent, landed child artifacts, and the current trunk state together.

Required outputs:
- `holistic-review.md` with structured findings across correctness, trust boundaries, UX, performance, deployability, and documentation
- `finding-index.json` with normalized findings, severities, and touched surfaces
- `remediation-plan.md` with concrete follow-up work or explicit justification for no action
- `promotion.md` with a first-pass ready/not-ready verdict

Do not merely summarize child artifacts. Normalize the state of the whole parent implementation.

Context:
- Integrated child units:
- release-preparation-ci-hardening, release-preparation-fresh-install-test, release-preparation-readme-and-changelog, release-preparation-release-binary-build, release-preparation-security-audit, release-preparation-version-bump-and-tag

Required outputs:
- holistic-review.md
- finding-index.json
- remediation-plan.md
- promotion.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/release-preparation-holistic-review-minimax/holistic-review.md`
- `.raspberry/portfolio/release-preparation-holistic-review-minimax/finding-index.json`
- `.raspberry/portfolio/release-preparation-holistic-review-minimax/remediation-plan.md`
- `.raspberry/portfolio/release-preparation-holistic-review-minimax/promotion.md`
