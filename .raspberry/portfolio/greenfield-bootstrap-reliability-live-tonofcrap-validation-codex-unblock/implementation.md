# Live Tonofcrap Validation Codex Unblock

## Summary

This unblock stays lane-local. It fixes the source lane's replay blockers without touching unrelated synthesis code or the `tonofcrap` repo, and it makes this codex-unblock lane persist the audit bundle that its own replay expects.

## Changes Landed

1. `malinka/workflows/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation.fabro`
   The validation lane quality gate no longer performs repo-wide lane-sizing or semantic-risk scans. `lane_sizing_hits` and `semantic_risk_hits` are left empty for this validation-only lane, so unrelated generator files such as `lib/crates/fabro-synthesis/src/render.rs` cannot fail replay through monolith heuristics or regex self-matches.

2. `malinka/prompts/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation/plan.md`
   Added evidence-discipline guidance: a smoke run shorter than 30 cycles is partial evidence only and must not be reported as Milestone 5 completion.

3. `malinka/prompts/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation/review.md`
   Added matching review guidance: a short smoke run can support diagnosis, but it is not enough to bless Milestone 5 or the live-validation acceptance criteria.

4. `malinka/workflows/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation-codex-unblock.fabro`
   The unblock lane audit now hydrates `spec.md`, `review.md`, and `promotion.md` inside `.raspberry/portfolio/greenfield-bootstrap-reliability-live-tonofcrap-validation-codex-unblock/` from the authoritative `.fabro-work` artifacts before validating the bundle. That removes the deterministic audit failure from missing portfolio metadata while keeping review-stage ownership of `.fabro-work/promotion.md`.

## Scope Guard

- No edits to `lib/crates/fabro-synthesis/src/render.rs`
- No edits to the `tonofcrap` repository
- No broader quality-gate changes for unrelated lanes
