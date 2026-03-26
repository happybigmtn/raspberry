# Integration

## Replay Impact

On the next replay of `greenfield-bootstrap-reliability-live-tonofcrap-validation`, the source lane will:

- stop failing quality because of unrelated repo-wide lane-sizing hits such as `lib/crates/fabro-synthesis/src/render.rs`
- stop failing quality because of the repo-wide semantic-risk regex self-matching `lib/crates/fabro-synthesis/src/render.rs`
- keep its validation scope focused on the lane's own artifacts and evidence
- require truthful reporting around Milestone 5, so a short smoke run cannot be written up as full 30-cycle validation

On the next replay of `greenfield-bootstrap-reliability-live-tonofcrap-validation-codex-unblock`, the current unblock lane will also carry its contract, review, and promotion artifacts forward into the portfolio directory before `audit` validates them, so the slice no longer fails after an otherwise-green review pass.

## Why This Is Minimal

The unblock changes only the source lane's workflow and prompts. It does not alter shared synthesis logic, unrelated lanes, or the validation target repository.
