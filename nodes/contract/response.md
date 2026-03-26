Created `.fabro-work/contract.md`. The issue is clear:

**Problem**: The `fabro synth evolve` command with `--no-review` flag uses the deterministic reconcile path which outputs `"Mode: evolve (deterministic reconcile)"` (line 905 in `synth.rs`), but the 3 tests still expect the old format `"Mode: evolve (deterministic steering report)"`.

**Fix needed**: Update 3 test assertions in `lib/crates/fabro-cli/tests/synth.rs`:
- Line 313: `synth_evolve_updates_existing_package`
- Line 382: `synth_evolve_can_import_current_package_without_blueprint_flag`
- Line 416: `synth_evolve_preview_stays_bounded_to_manifest_and_report`

Each needs to change from:
```rust
.stdout(predicate::str::contains(
    "Mode: evolve (deterministic steering report)",
))
```
to:
```rust
.stdout(predicate::str::contains(
    "Mode: evolve (deterministic reconcile)",
))
```