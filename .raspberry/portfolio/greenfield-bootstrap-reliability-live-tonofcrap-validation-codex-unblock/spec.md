## Deliverables
- `.fabro-work/contract.md`
- `malinka/workflows/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation.fabro`
- `malinka/prompts/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation/plan.md`
- `malinka/prompts/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation/review.md`
- `.raspberry/portfolio/greenfield-bootstrap-reliability-live-tonofcrap-validation-codex-unblock/implementation.md`
- `.raspberry/portfolio/greenfield-bootstrap-reliability-live-tonofcrap-validation-codex-unblock/verification.md`
- `.raspberry/portfolio/greenfield-bootstrap-reliability-live-tonofcrap-validation-codex-unblock/integration.md`

## Acceptance Criteria
- `cargo check --workspace` exits 0 from the repo root after the unblock change set lands.
- Inspecting `malinka/workflows/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation.fabro` shows the source lane quality gate no longer performs a repo-wide lane-sizing scan that can fail on unrelated monoliths like `lib/crates/fabro-synthesis/src/render.rs`; the check is removed for this validation-only lane or scoped to lane-owned surfaces.
- Inspecting `malinka/prompts/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation/plan.md` and `malinka/prompts/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation/review.md` shows explicit guidance that a smoke run is partial evidence and must not be reported as full Milestone 5 completion without real 30-cycle `raspberry autodev` evidence.
- `.raspberry/portfolio/greenfield-bootstrap-reliability-live-tonofcrap-validation-codex-unblock/verification.md` cites failed source run `01KMNTNRQ98FRA9RZ7BWYBQ6AQ` and records both blockers observed there: `quality_ready: no` because `lane_sizing_debt: yes`, and the prior verification artifact claiming success from only a 5-cycle live run.
- `git diff --name-only` for the unblock change set does not include `lib/crates/fabro-synthesis/src/render.rs`; the fix stays lane-local unless a new blocker is discovered and the contract is updated first.

## Out of Scope
- Running a fresh 30-cycle `raspberry autodev` validation on `tonofcrap` in this unblock lane.
- Refactoring or decomposing `lib/crates/fabro-synthesis/src/render.rs`.
- Broad changes to shared synthesis or quality-gate behavior for unrelated lanes.
- Modifying the `tonofcrap` repository itself.
