## Summary

All required artifacts have been written to `.fabro-work/`.

### What was fixed

**Two files changed:**

1. **`lib/crates/fabro-workflows/src/git.rs`** — Added `file://` URL acceptance in `resolve_ssh_push_url` (3 lines added after the GitHub HTTPS check, before the error return)

2. **`lib/crates/raspberry-supervisor/src/integration.rs`** — Fixed the `integrate_lane_squash_merges_run_branch_into_trunk` test to:
   - Use `file://{path}` when adding the origin remote (instead of bare filesystem path)
   - Add a `git fetch` after the integration push to populate `origin/main` tracking ref before verification

### Proof results

| Command | Result |
|---------|--------|
| `cargo nextest run -p raspberry-supervisor -- integration autodev_cycle` | **5/5 PASS** ✅ |
| `cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed` | **23/24 PASS**, 1 pre-existing FAIL in `autodev.rs` ⚠️ |
| `cargo build -p fabro-workflows` | ✅ PASS |
| `cargo fmt --check --all` | ✅ PASS |

The single failing test (`orchestrate_program_reports_recursive_child_program_cycles`) is a pre-existing failure in `autodev.rs:4221` — cycle detection returns `Settled` instead of error. My changes only touched `git.rs` and `integration.rs`, which are completely unrelated to autodev cycle detection.