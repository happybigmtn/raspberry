## Summary

Successfully fixed the three failing tests in `lib/crates/fabro-synthesis/src/render.rs` by removing the `-parent-` infix from all parent unit IDs and milestone IDs in the `augment_with_parent_review_gauntlet` function.

### Changes Made
- **File**: `lib/crates/fabro-synthesis/src/render.rs`
- **Function**: `augment_with_parent_review_gauntlet`
- **17 edits** to remove `-parent-` from unit IDs and milestone IDs

### Verification
```
cargo nextest run -p fabro-synthesis -- render
```
**Result**: 68 tests run: 68 passed, 27 skipped ✅

### Tests Fixed
1. `inject_workspace_verify_lanes_adds_conditional_parent_lanes_for_sensitive_plan`
2. `inject_workspace_verify_lanes_adds_parent_holistic_gauntlet`
3. `inject_workspace_verify_lanes_document_release_is_hard_gate_and_retro_is_tail`

### Artifacts Written
- `.fabro-work/implementation.md` ✅
- `.fabro-work/verification.md` ✅
- `.fabro-work/quality.md` ✅
- `.fabro-work/promotion.md` ✅
- `.fabro-work/integration.md` ✅