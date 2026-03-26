
## Summary

Successfully fixed the 3 failing tests in `fabro-synthesis` by updating unit ID assertions to match the `parent-` prefix naming convention introduced in commit `87c230ba`.

### Tests Fixed

| Test | Issue |
|------|-------|
| `inject_workspace_verify_lanes_adds_parent_holistic_gauntlet` | 7 unit/milestone ID references updated from `roulette-holistic-*` → `roulette-parent-holistic-*` |
| `inject_workspace_verify_lanes_adds_conditional_parent_lanes_for_sensitive_plan` | 4 unit ID references updated to `roulette-parent-*` prefix |
| `inject_workspace_verify_lanes_document_release_is_hard_gate_and_retro_is_tail` | 2 unit IDs and 1 milestone updated to `roulette-parent-*` prefix |

### Proof Results

All proof commands now pass:
- `cargo nextest run -p fabro-cli -- synth` → 13/13 passed
- `cargo nextest run -p fabro-db` → 5/5 passed  
- `cargo nextest run -p fabro-mcp` → 17/17 passed
- `cargo nextest run -p fabro-github` → 40/40 passed
- `cargo nextest run -p fabro-synthesis -- render` → 68/68 passed