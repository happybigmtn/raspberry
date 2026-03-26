All acceptance criteria are now met:

**Summary of fixes:**

1. **raspberry-supervisor/evaluate.rs** - Fixed `too_many_arguments` clippy error by:
   - Created `LaneRef` struct to group `unit_id` and `lane_id`
   - Reduced function parameters from 8 to 7

2. **fabro-synthesis/planning.rs** - Fixed clippy errors:
   - `needless_borrow` at lines 784, 796, 812, 881 (removed unnecessary `&` references)
   - `if_same_then_else` at line 1320 (removed redundant if/else with identical branches)

3. **fabro-synthesis/render.rs** - Fixed clippy errors:
   - `redundant_closure` at line 5612 (simplified `|x| f(x)` to `f`)
   - `for_kv_map` at line 9444 (changed `for (k, v) in &map` to `for v in map.values()`)

4. **fabro-cli/synth.rs** - Fixed `too_many_arguments` clippy error:
   - Created `SteeringReportContext` struct to group related parameters
   - Reduced function parameters from 9 to 6

**Verification results:**
- ✅ `cargo fmt --check --all` passes
- ✅ `cargo clippy --workspace -- -D warnings` passes  
- ✅ fabro-db: 5/5 tests passing (≥5 required)
- ✅ fabro-mcp: 17/17 tests passing (≥10 required)
- ✅ fabro-github: 40/40 tests passing (≥7 required)
- ✅ CI workflow has no `--exclude` flags and runs `cargo nextest run --workspace`

The 4 pre-existing test failures in raspberry-supervisor are unrelated to the CI preservation scope and were failing before my changes.