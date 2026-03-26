All tests pass. Here's a summary of the fixup:

## Summary

Fixed two failing tests in `raspberry-supervisor`:

### 1. `integration::tests::integrate_lane_squash_merges_run_branch_into_trunk`
- **Problem**: When git clones a repo with a `file://` remote, the clone stores the remote URL as just the absolute path (no `file://` prefix). `resolve_ssh_push_url` didn't handle absolute paths.
- **Fix**: Added check in `fabro-workflows/src/git.rs` to accept absolute paths as `file://` URLs.

### 2. `autodev::tests::orchestrate_program_reports_recursive_child_program_cycles`
- **Problem**: Cycle detection wasn't triggered because `refresh_parent_programs` skips the current manifest via `same_manifest_path`, bypassing the cycle check.
- **Fix**: Modified `raspberry-supervisor/src/evaluate.rs` to:
  - Add `RecursiveEvaluationCycle` error variant
  - Check evaluation stack BEFORE `same_manifest_path` in `refresh_parent_programs`

### Files Modified
- `lib/crates/fabro-workflows/src/git.rs`
- `lib/crates/raspberry-supervisor/src/evaluate.rs`
- `.fabro-work/verification.md` (updated proof results)
- `.fabro-work/implementation.md` (updated with both fixes)

### Proof Results
- `cargo nextest run -p raspberry-supervisor -- integration autodev_cycle`: **5/5 PASS**
- `cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed`: **24/24 PASS**