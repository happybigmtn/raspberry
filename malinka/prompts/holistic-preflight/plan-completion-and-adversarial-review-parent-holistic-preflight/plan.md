# plan-completion-and-adversarial-review Holistic Preflight — Plan

Lane: `plan-completion-and-adversarial-review-parent-holistic-preflight`

Goal:
- Preflight the integrated parent plan `plan-completion-and-adversarial-review` before holistic review.

Integrated child units:
- plan-completion-and-adversarial-review-3-step-adversarial-review-prompt, plan-completion-and-adversarial-review-live-validation, plan-completion-and-adversarial-review-meta-review-feedback-loop, plan-completion-and-adversarial-review-plan-completion-detection, plan-completion-and-adversarial-review-registry-driven-plan-review-generation

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
- plan-completion-and-adversarial-review-3-step-adversarial-review-prompt, plan-completion-and-adversarial-review-live-validation, plan-completion-and-adversarial-review-meta-review-feedback-loop, plan-completion-and-adversarial-review-plan-completion-detection, plan-completion-and-adversarial-review-registry-driven-plan-review-generation

Required outputs:
- verification.md
- review.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/plan-completion-and-adversarial-review-parent-holistic-preflight/verification.md`
- `.raspberry/portfolio/plan-completion-and-adversarial-review-parent-holistic-preflight/review.md`
