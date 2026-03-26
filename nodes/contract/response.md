The contract is written at `.fabro-work/contract.md`. Here's a summary of the situation and plan:

**Root cause of preflight failure:**
The test `integrate_lane_squash_merges_run_branch_into_trunk` creates a bare git repo at a local path and uses it as `origin`. When `integrate_run` calls `resolve_ssh_push_url` in `fabro-workflows/src/git.rs`, that function only accepts SSH (`git@…`, `ssh://…`) or GitHub HTTPS URLs. A raw local filesystem path (`/home/r/.cache/rust-tmp/.tmpohclm1/remote.git`) is rejected.

**Fix strategy (2 files, minimal scope):**
1. `lib/crates/fabro-workflows/src/git.rs` — extend `resolve_ssh_push_url` to also accept `file://` URLs (a standard git remote URL scheme that works for local push/fetch)
2. `lib/crates/raspberry-supervisor/src/integration.rs` — change the test's `origin` remote URL from a bare path to a `file://` URL (e.g., `file:///tmp/…/remote.git`)

The `file://` scheme is a valid, standard git remote URL. This fix is narrowly targeted: it makes unit tests work with local remotes without relaxing validation for non-`file://` local paths (the existing error message for bare local paths is intentionally preserved for the non-`file://` case).