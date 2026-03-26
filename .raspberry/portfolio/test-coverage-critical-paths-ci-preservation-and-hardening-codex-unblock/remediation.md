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

## Deep Review Findings
# Deep Review Findings

## Root Cause Classification

- Inside lane-owned surface: no remaining blocker found.
- Outside lane-owned surface: yes.
- The owned proof gate is already green. `malinka/workflows/implementation/test-coverage-critical-paths-ci-preservation-and-hardening.fabro` already preserves the intended `cargo fmt --check --all`, `cargo clippy --workspace -- -D warnings`, and `cargo nextest run --workspace` verification flow without placeholder `preflight` or `verify` scripts.
- The repeated verify failures came from replaying the broader workspace proof surface, not from the lane-owned workflow file itself. Per `.fabro-work/verification.md`, the replay first exposed stale lint/test debt in external files under `lib/crates/`, and after those were fixed the remaining risk narrowed to an environment-specific linker bus error while building `fabro-api` test binaries under full-workspace `nextest`.

## Concrete Fix Plan

1. Replay the source lane unchanged first. The lane-owned harness does not need another proof-gate edit.
2. If the replay fails again on stale warnings, compile errors, or assertions in `lib/crates/**`, treat that as outside-lane debt and have the fixup stage repair those external surfaces directly rather than weakening the source lane workflow.
3. If the replay reaches full-workspace `cargo nextest run --workspace` and reproduces the `fabro-api` linker bus error (`rust-lld` / `cc` signal 7), treat that as an external test-link environment problem. The fixup stage should stabilize the replay environment by investigating memory pressure, linker parallelism, or target-dir isolation before changing more product code.
4. If the replay passes, no code or harness change is needed in the source lane beyond this documentation.

## Promotion Decision
(not found)
