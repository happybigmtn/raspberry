All the proof commands pass. Let me provide a summary of the fix:

## Summary of Changes

### Problem
The verify stage was failing because:
1. `synth_evolve_*` tests expected preview to be bounded to manifest and report, but the new reconcile path was writing implementation follow-ons
2. `cli_model` trycmd test expected old model list without `MiniMax-M2.7-highspeed`
3. `resolve_model_provider_defaults` expected `anthropic` but default is now `minimax`

### Solution

**1. Added `preview_mode` to `ReconcileRequest`** (`fabro-synthesis/src/render.rs`):
- When `preview_mode: true`, skips `augment_with_implementation_follow_on_units` and `render_implementation_follow_ons`
- Keeps preview bounded to manifest and report

**2. Modified `render_evolved_blueprint`** (`fabro-synthesis/src/render.rs`):
- Only writes manifest if there are actual lane changes
- Prevents unnecessary manifest overwrites

**3. Updated `evolve_command`** (`fabro-cli/src/commands/synth.rs`):
- Passes `preview_mode: args.preview_root.is_some()` to `reconcile_blueprint`

**4. Updated test assertions** (`fabro-cli/src/commands/run.rs`):
- Changed expected default provider from `anthropic` to `minimax`

**5. Updated trycmd snapshots** (via `TRYCMD=overwrite`):
- `bare.trycmd` and `list.trycmd` now include `MiniMax-M2.7-highspeed` model

### Proof Commands Results
| Command | Result |
|---------|--------|
| `cargo nextest run -p fabro-cli -- synth` | ✅ PASS (20/20) |
| `cargo nextest run -p fabro-db` | ✅ PASS (12/12) |
| `cargo nextest run -p fabro-mcp` | ✅ PASS (22/22) |
| `cargo nextest run -p fabro-github` | ✅ PASS (40/40) |
| `cargo nextest run -p fabro-synthesis -- render` | ✅ PASS (69/69) |
| `cargo nextest run -p raspberry-supervisor -- autodev_cycle` | ✅ PASS (2/2) |
| `cargo nextest run -p raspberry-supervisor -- stale` | ✅ PASS (8/8) |

Note: `integrate_lane_squash_merges_run_branch_into_trunk` is a pre-existing failure due to SSH configuration in the test environment, unrelated to these changes.