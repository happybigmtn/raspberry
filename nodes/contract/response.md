Contract written to `.fabro-work/contract.md`. Here's a summary:

## Summary

**Root Cause**: The failing test `integrate_lane_squash_merges_run_branch_into_trunk` creates a local file path as a git remote, but the `fabro-workflows` direct integration code correctly requires SSH or GitHub HTTPS URLs (per `resolve_ssh_push_url` in `git.rs:207`).

**Fix Required**: The test needs to use a mock SSH/HTTPS-compatible remote setup instead of a local file path. This is purely a test infrastructure fix—no production code changes.

**Deliverables**:
1. Fix `lib/crates/raspberry-supervisor/src/integration.rs` - make the integration test use a valid remote URL format
2. Create `docs/rpi-edge-case-tests.md` - document the edge cases

**Acceptance Criteria**:
- Both proof commands pass (exit 0)
- No regressions in other tests
- Tests use mockable remote configurations (no network dependencies)