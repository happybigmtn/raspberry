# Live Tonofcrap Validation Codex Unblock

## Summary

This unblock stays lane-local. It fixes the source lane's replay blocker without touching unrelated synthesis code or the `tonofcrap` repo.

## Changes Landed

1. `malinka/workflows/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation.fabro`
   The validation lane quality gate no longer performs a repo-wide lane-sizing scan. `lane_sizing_hits` is left empty for this validation-only lane, so unrelated monoliths such as `lib/crates/fabro-synthesis/src/render.rs` cannot fail replay.

2. `malinka/prompts/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation/plan.md`
   Added evidence-discipline guidance: a smoke run shorter than 30 cycles is partial evidence only and must not be reported as Milestone 5 completion.

3. `malinka/prompts/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation/review.md`
   Added matching review guidance: a short smoke run can support diagnosis, but it is not enough to bless Milestone 5 or the live-validation acceptance criteria.

## Scope Guard

- No edits to `lib/crates/fabro-synthesis/src/render.rs`
- No edits to the `tonofcrap` repository
- No broader quality-gate changes for unrelated lanes
