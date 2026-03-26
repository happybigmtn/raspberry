# Remediation Notes (auto-captured from failed audit)

## Quality Gate
(not found)

## Verification Findings
# Challenge Note

The slice is larger than the nominal source-lane surface. The source workflow already had the intended proof-gate hardening in `malinka/workflows/implementation/test-coverage-critical-paths-ci-preservation-and-hardening.fabro`, but replaying its real proof surface exposed external workspace blockers outside the lane-owned file set.

## Automated Proof Summary

- `cargo check --workspace`: passed.
- `cargo clippy --workspace -- -D warnings`: passed after fixing workspace lint debt in `raspberry-supervisor`, `fabro-synthesis`, and `fabro-cli`.
- `cargo nextest run -p fabro-cli -- synth_evolve_updates_existing_package synth_evolve_can_import_current_package_without_blueprint_flag synth_evolve_preview_stays_bounded_to_manifest_and_report`: passed after updating stale synth expectations to the current deterministic reconcile behavior.
- `CARGO_TARGET_DIR=/tmp/fabro-ci-hardening-target cargo nextest run --workspace --no-fail-fast`: did not complete cleanly; after the code-level test and compile failures were fixed, it hit a linker bus error while compiling `fabro-api` tests (`rust-lld`/`cc` exited on signal 7). That looks environment/resource-specific rather than a newly rediscovered semantic regression.

## Concrete Gaps Found

- Outside lane-owned surface: yes. Workspace proof was blocked by stale fixtures/assertions in `lib/crates/raspberry-supervisor/src/evaluate.rs`, `lib/crates/fabro-synthesis/src/planning.rs`, `lib/crates/fabro-synthesis/src/render.rs`, `lib/crates/fabro-cli/src/commands/synth.rs`, `lib/crates/fabro-cli/tests/synth.rs`, `lib/crates/raspberry-tui/src/app.rs`, and `lib/crates/fabro-model/src/catalog.rs`.
- The first proof gate for this unblock lane is satisfied (`cargo check --workspace` is green), and the source lane’s `clippy` blocker is also gone.
- The remaining replay risk is no longer a stale test expectation or compile error in product code; it is the full-workspace `nextest` environment potentially reproducing the linker bus error during `fabro-api` test binary linking.

## Next Fixup Target

If the source lane replay still fails, the next fixup target should be the `fabro-api` test-link environment rather than more lane logic. Re-run full `cargo nextest run --workspace` in the normal replay environment first; if the linker bus error reproduces there, investigate memory/parallelism or linker stability before changing more product code.

## Fixup Validation

- `cargo test -p fabro-api --no-run`: passed, and linked all `fabro-api` test binaries successfully.
- `cargo nextest run -p fabro-api`: passed, with all 76 `fabro-api` tests green.
- `cargo check --workspace`: passed after the harness-only patch, confirming the workspace still compiles cleanly.
- The remaining deterministic blocker for this codex-unblock slice was the workflow harness itself: `quality` wrote only the durable portfolio file, while `review` and `audit` expected `.fabro-work/quality.md`, and `audit` validated durable `spec.md`/`review.md`/`promotion.md` paths without syncing the ephemeral review artifacts first.
- The fixup updates the current slice so `quality` is written to both `.fabro-work/quality.md` and the durable portfolio path, `review` is required to emit `.fabro-work/review.md`, and `audit` now syncs the ephemeral artifacts into the durable portfolio directory before validating them.

## Deep Review Findings
# Deep Review Findings

## Root Cause Classification

- Inside lane-owned surface: yes.
- Outside lane-owned surface: not the remaining deterministic blocker.
- The owned proof gate is already green: `cargo check --workspace` passes in the current worktree.
- The latest remediation context shows that the original workspace proof failures in external `lib/crates/**` surfaces were fixed, but the source lane still had a lane-owned harness mismatch: its `quality` stage needed to publish the machine-generated report to `.fabro-work/quality.md` as well as `outputs/test-coverage-critical-paths-ci-preservation-and-hardening/quality.md`.
- That mismatch mattered because the source lane's own review prompt treats `.fabro-work/quality.md` as the authoritative quality signal, and the audit remediation capture also reads `.fabro-work/quality.md`.
- The source workflow now contains that harness fix in `malinka/workflows/implementation/test-coverage-critical-paths-ci-preservation-and-hardening.fabro`, so no further lane-owned product-code edit is indicated by the current evidence.

## Concrete Fix Plan

1. Replay the source lane with the current workflow as-is. Do not weaken or rewrite its proof commands.
2. Treat the lane-owned harness issue as resolved by the existing dual-write quality step that emits both `.fabro-work/quality.md` and `outputs/test-coverage-critical-paths-ci-preservation-and-hardening/quality.md`.
3. If replay fails again on warnings, compile errors, or test assertions in external `lib/crates/**` files, treat that as outside-lane debt and have the fixup stage repair those external surfaces directly rather than editing the source lane workflow again.
4. If replay reaches review/audit cleanly after the harness fix, no additional source-lane changes are needed.

## Promotion Decision
(not found)
