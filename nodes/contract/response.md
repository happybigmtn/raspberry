## Contract Created: `.fabro-work/contract.md`

**Problem**: Duplicate field initializers in `raspberry-tui/src/app.rs` cause build failure.

**Fix Required**: Remove the second occurrence of `failure_kind: None,` and `recovery_action: None,` at lines 1277-1278 (keep the first occurrence at lines 1264-1265).

### Deliverables
- `lib/crates/raspberry-tui/src/app.rs` — remove duplicate fields

### Acceptance Criteria
1. `cargo check -p raspberry-tui` succeeds
2. `cargo nextest run --workspace` completes without compilation errors  
3. Duplicate fields removed (appear exactly once)
4. All proof commands pass