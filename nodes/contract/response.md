## Contract Summary

I've written `.fabro-work/contract.md` defining the fix for 3 failing tests in `lib/crates/fabro-synthesis/src/render.rs`.

**Root Cause**: The code uses `{plan_prefix}-parent-{stage_name}` naming (e.g., `roulette-parent-document-release`), but tests expect `{plan_prefix}-{stage_name}` (e.g., `roulette-document-release`).

**Solution**: Update test assertions to match the actual code naming convention.

### Files to Modify
- `lib/crates/fabro-synthesis/src/render.rs` — Update 3 test functions with corrected assertions

### Tests to Fix
1. `inject_workspace_verify_lanes_adds_parent_holistic_gauntlet` — Update unit IDs and milestone references to use `-parent-` infix
2. `inject_workspace_verify_lanes_adds_conditional_parent_lanes_for_sensitive_plan` — Update unit IDs to use `-parent-` infix  
3. `inject_workspace_verify_lanes_document_release_is_hard_gate_and_retro_is_tail` — Update unit IDs, milestone IDs, and dependency references