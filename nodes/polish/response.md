Both artifacts are written. Here's a summary of the key changes made during the polish:

## spec.md — Key Polishes

| Change | Rationale |
|--------|-----------|
| Added exact file paths and line numbers for every code reference | Makes the spec executable and navigable |
| Fixed R2.3 description | Traced through actual `dispatch.rs:220-299` thread handling; clarified that pre-spawn errors propagate via `?` before `mark_lane_dispatch_failed` is reachable |
| Added R2.4 for bootstrap failure recovery | R2.3 fix alone doesn't classify the error; R2.4 specifies that `classify_failure` must recognize "pre-spawn:" stderr |
| Added concrete code snippets for `stale_active_progress` and `worker_process_alive` | Shows the actual implementation for validation |
| Added `TEMP-SYMLINK` as a must-fix in the spec | The security review identified a real symlink vulnerability in `autodev_temp_dir` |
| Added implementation table with current status per requirement | Distinguishes confirmed bugs from missing implementation |
| Added decision log entry for R2.3 fix strategy | Wrapping pre-spawn errors in `DispatchOutcome` with `exit_status=-1` flows naturally through existing machinery |

## review.md — Key Polishes

| Change | Rationale |
|--------|-----------|
| Added exact code snippets showing the R2.3 bug path | `?` propagates `DispatchError` before `mark_lane_dispatch_failed` is reached |
| Removed incorrect claim that `Spawn` error is handled | `Spawn` is wrapped into `DispatchOutcome` in `run_fabro`, but it still propagates via `?` — same bug |
| Separated TEMP-SYMLINK as a must-fix (not just security debt) | Symlink vulnerability in predictable temp path is a real exploit |
| Removed fabricated line numbers (e.g., "lines 245-255") | Replaced with actual code context from the codebase |
| Simplified to 6 concrete blockers | Removed duplicates; each item maps to one spec requirement or one confirmed bug |