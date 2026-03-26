# greenfield-bootstrap-reliability Holistic Preflight — Plan

Lane: `greenfield-bootstrap-reliability-holistic-preflight`

Goal:
- Preflight the integrated parent plan `greenfield-bootstrap-reliability` before holistic review.

Integrated child units:
- greenfield-bootstrap-reliability-bootstrap-verification-gate, greenfield-bootstrap-reliability-fresh-rust-project-validation, greenfield-bootstrap-reliability-live-tonofcrap-validation, greenfield-bootstrap-reliability-runtime-stable-asset-resolution, greenfield-bootstrap-reliability-type-aware-quality-for-typescript, greenfield-bootstrap-reliability-verify-scaffold-first-ordering

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
- greenfield-bootstrap-reliability-bootstrap-verification-gate, greenfield-bootstrap-reliability-fresh-rust-project-validation, greenfield-bootstrap-reliability-live-tonofcrap-validation, greenfield-bootstrap-reliability-runtime-stable-asset-resolution, greenfield-bootstrap-reliability-type-aware-quality-for-typescript, greenfield-bootstrap-reliability-verify-scaffold-first-ordering

Required outputs:
- verification.md
- review.md

Write durable artifacts only to these exact lane-scoped paths:
- `.raspberry/portfolio/greenfield-bootstrap-reliability-holistic-preflight/verification.md`
- `.raspberry/portfolio/greenfield-bootstrap-reliability-holistic-preflight/review.md`
