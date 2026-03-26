## Deliverables
- `lib/crates/fabro-synthesis/src/render.rs`

## Acceptance Criteria
1. Test `inject_workspace_verify_lanes_adds_conditional_parent_lanes_for_sensitive_plan` passes — verifies `roulette-investigate`, `roulette-design-review`, `roulette-cso`, `roulette-benchmark` exist
2. Test `inject_workspace_verify_lanes_adds_parent_holistic_gauntlet` passes — verifies `roulette-holistic-preflight`, `roulette-holistic-review-minimax`, `roulette-holistic-review-deep`, `roulette-holistic-review-adjudication` exist with correct dependency chain
3. Test `inject_workspace_verify_lanes_document_release_is_hard_gate_and_retro_is_tail` passes — verifies `roulette-document-release` and `roulette-retro` units exist with correct artifacts and dependencies
4. All 68 tests in `render::tests` pass: `cargo nextest run -p fabro-synthesis -- render` returns 68/68 passed

## Out of Scope
- Modifying test expectations (tests define the correct naming convention)
- Changes to other crates or functions outside `augment_with_parent_review_gauntlet`
- Changes to production workflow generation behavior beyond test compliance

**Root cause**: The `augment_with_parent_review_gauntlet` function generates parent unit IDs with a `-parent-` segment (e.g., `roulette-parent-holistic-preflight`), but tests expect the naming pattern without that segment (e.g., `roulette-holistic-preflight`). The fix is to remove the `parent` segment from all parent unit ID format strings.