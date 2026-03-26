Goal: Autodev Integration Test

Child work item of plan: Test Coverage for Critical Paths

Proof commands:
- `cargo nextest run --workspace`
- `cargo nextest run -p fabro-cli -- synth`
- `cargo nextest run -p fabro-db`
- `cargo nextest run -p fabro-mcp && cargo nextest run -p fabro-github`
- `cargo nextest run -p fabro-synthesis -- render`
- `cargo nextest run -p raspberry-supervisor -- integration autodev_cycle`
- `cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed`

Required durable artifacts:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`


## Completed stages
- **preflight**: success
  - Script: `set +e
cargo nextest run --workspace && cargo nextest run -p fabro-cli -- synth && cargo nextest run -p fabro-db && cargo nextest run -p fabro-mcp && cargo nextest run -p fabro-github && cargo nextest run -p fabro-synthesis -- render && cargo nextest run -p raspberry-supervisor -- integration autodev_cycle && cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
true`
  - Stdout: (empty)
  - Stderr:
    ```
    error: no such command: `nextest`
    
    help: a command with a similar name exists: `test`
    
    help: view all installed commands with `cargo --list`
    help: find a package to install `nextest` with `cargo search cargo-nextest`
    ```
- **contract**: success
  - Model: MiniMax-M2.7-highspeed, 30.5k tokens in / 295 out
- **implement**: success
  - Model: MiniMax-M2.7-highspeed, 161.7k tokens in / 1.1k out
  - Files: Cargo.lock, lib/crates/fabro-cli/src/commands/synth.rs, lib/crates/fabro-db/Cargo.toml, lib/crates/fabro-db/src/lib.rs, lib/crates/fabro-mcp/src/lib.rs, lib/crates/fabro-synthesis/src/render.rs, lib/crates/raspberry-supervisor/src/lib.rs, lib/crates/raspberry-supervisor/tests/integration_tests.rs, 2 more repo file(s)
- **verify**: fail
  - Script: `cargo nextest run --workspace && cargo nextest run -p fabro-cli -- synth && cargo nextest run -p fabro-db && cargo nextest run -p fabro-mcp && cargo nextest run -p fabro-github && cargo nextest run -p fabro-synthesis -- render && cargo nextest run -p raspberry-supervisor -- integration autodev_cycle && cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed`
  - Stdout: (empty)
  - Stderr:
    ```
    (752 lines omitted)
        (test timed out)
    
     TERMINATING [>  4.000s] (─────────) fabro-cli::trycmd cli_run
         TIMEOUT [   4.006s] ( 500/3777) fabro-cli::trycmd cli_run
      stdout ───
    
        running 1 test
      stderr ───
        Testing tests/cmd/run/help.trycmd:2 ... ok 10ms 831us 570ns
    
        (test timed out)
    
    ────────────
         Summary [   4.688s] 500/3777 tests run: 491 passed, 3 failed, 6 timed out, 178 skipped
            FAIL [   0.185s] ( 481/3777) fabro-cli::synth synth_evolve_updates_existing_package
            FAIL [   0.300s] ( 485/3777) fabro-cli::synth synth_evolve_preview_stays_bounded_to_manifest_and_report
            FAIL [   0.078s] ( 490/3777) fabro-cli::trycmd cli_model
         TIMEOUT [   4.002s] ( 495/3777) fabro-cli::cli dry_run_create_start_attach_works_with_default_run_lookup
         TIMEOUT [   4.004s] ( 496/3777) fabro-cli::cli dry_run_detach_attach_works_with_default_run_lookup
         TIMEOUT [   4.002s] ( 497/3777) fabro-cli::cli dry_run_writes_jsonl_and_live_json
         TIMEOUT [   4.004s] ( 498/3777) fabro-cli::cli standalone_file_run_uses_file_stem_slug_for_lookup
         TIMEOUT [   4.003s] ( 499/3777) fabro-cli::cli start_by_workflow_name_prefers_newly_created_submitted_run
         TIMEOUT [   4.006s] ( 500/3777) fabro-cli::trycmd cli_run
    warning: 3277/3777 tests were not run due to test failure (run with --no-fail-fast to run all tests, or run with --max-fail)
    error: test run failed
    ```
- **fixup**: success
  - Model: MiniMax-M2.7-highspeed, 59.3k tokens in / 450 out
  - Files: lib/crates/fabro-cli/tests/synth.rs, lib/crates/fabro-model/src/catalog.rs, lib/crates/raspberry-tui/src/app.rs
- **verify**: fail
  - Script: `cargo nextest run --workspace && cargo nextest run -p fabro-cli -- synth && cargo nextest run -p fabro-db && cargo nextest run -p fabro-mcp && cargo nextest run -p fabro-github && cargo nextest run -p fabro-synthesis -- render && cargo nextest run -p raspberry-supervisor -- integration autodev_cycle && cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed`
  - Stdout: (empty)
  - Stderr:
    ```
    (752 lines omitted)
        (test timed out)
    
     TERMINATING [>  4.000s] (─────────) fabro-cli::trycmd cli_run
         TIMEOUT [   4.006s] ( 500/3777) fabro-cli::trycmd cli_run
      stdout ───
    
        running 1 test
      stderr ───
        Testing tests/cmd/run/help.trycmd:2 ... ok 10ms 831us 570ns
    
        (test timed out)
    
    ────────────
         Summary [   4.688s] 500/3777 tests run: 491 passed, 3 failed, 6 timed out, 178 skipped
            FAIL [   0.185s] ( 481/3777) fabro-cli::synth synth_evolve_updates_existing_package
            FAIL [   0.300s] ( 485/3777) fabro-cli::synth synth_evolve_preview_stays_bounded_to_manifest_and_report
            FAIL [   0.078s] ( 490/3777) fabro-cli::trycmd cli_model
         TIMEOUT [   4.002s] ( 495/3777) fabro-cli::cli dry_run_create_start_attach_works_with_default_run_lookup
         TIMEOUT [   4.004s] ( 496/3777) fabro-cli::cli dry_run_detach_attach_works_with_default_run_lookup
         TIMEOUT [   4.002s] ( 497/3777) fabro-cli::cli dry_run_writes_jsonl_and_live_json
         TIMEOUT [   4.004s] ( 498/3777) fabro-cli::cli standalone_file_run_uses_file_stem_slug_for_lookup
         TIMEOUT [   4.003s] ( 499/3777) fabro-cli::cli start_by_workflow_name_prefers_newly_created_submitted_run
         TIMEOUT [   4.006s] ( 500/3777) fabro-cli::trycmd cli_run
    warning: 3277/3777 tests were not run due to test failure (run with --no-fail-fast to run all tests, or run with --max-fail)
    error: test run failed
    ```

## Context
- failure_class: canceled
- failure_signature: verify|canceled|script failed with exit code: <n> ## stderr blocking waiting for file lock on artifact directory compiling fabro-model v0.<n>.<n> (/home/r/.fabro/runs/<hex>-01kmnped23ftnqcckz0z7gcpcj/worktree/lib/crates/fabro-model) compiling fabro-cli v0.


# Autodev Integration Test Lane — Fixup

Fix only the current slice for `test-coverage-critical-paths-autodev-integration-test`.


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Priorities:
- unblock the active slice's first proof gate — this is the #1 priority
- prefer staying within the named slice and touched surfaces
- if the proof gate fails on pre-existing issues OUTSIDE your surfaces (e.g., linter warnings in unrelated files, missing imports in dependencies), you MUST fix those issues minimally to unblock the gate — do not leave the lane stuck on problems you can solve
- preserve setup constraints before expanding implementation scope
- keep implementation and verification artifacts durable and specific
- do not create or rewrite `.fabro-work/promotion.md` during Fixup; that file is owned by the Review stage
- do not hand-author `.fabro-work/quality.md`; the Quality Gate rewrites it after verification
- ALL ephemeral files (quality.md, promotion.md, verification.md) go in `.fabro-work/`, never the repo root
