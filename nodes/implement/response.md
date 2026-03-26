## Summary

I fixed the compilation errors that were blocking the workspace test suite:

### Changes Made

1. **`lib/crates/raspberry-tui/src/app.rs`**
   - Removed duplicate `failure_kind` and `recovery_action` field initializers (lines 1277-1278)
   - Added missing `consecutive_failures: 0` field to `EvaluatedLane` initialization

2. **`lib/crates/fabro-model/src/catalog.rs`**
   - Fixed test `minimax_m2_7_resolves_to_highspeed` to compare `Provider::Minimax` instead of string `"minimax"`

### Verification
- ✅ `cargo check --workspace` - workspace compiles
- ✅ `cargo nextest run -p raspberry-tui` - 12/12 tests pass
- ✅ `cargo nextest run -p fabro-model minimax_m2_7` - passes
- ✅ `cargo nextest run -p fabro-db` - 5/5 tests pass
- ✅ `cargo nextest run -p fabro-mcp` - 17/17 tests pass
- ✅ `cargo nextest run -p fabro-github` - 40/40 tests pass
- ✅ `cargo nextest run -p raspberry-supervisor -- integration autodev_cycle` - 5/5 tests pass

### Pre-existing Failures (Not Related to This Slice)
- fabro-cli::synth (2 tests) - fixture assertion mismatches
- fabro-model (2 tests) - fallback chain tests
- fabro-synthesis (3 tests) - workspace verify tests
- raspberry-supervisor (1 test) - cycle detection

### Artifacts Created
All artifacts written to `.fabro-work/`:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`