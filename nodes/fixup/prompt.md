Goal: Fabro Db Baseline Tests

Child work item of plan: Test Coverage for Critical Paths

Proof commands:
- `cargo nextest run -p fabro-cli -- synth`
- `cargo nextest run -p fabro-db`
- `cargo nextest run -p fabro-mcp && cargo nextest run -p fabro-github`
- `cargo nextest run -p fabro-synthesis -- render`

Required durable artifacts:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`


## Completed stages
- **preflight**: success
  - Script: `set +e
if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p fabro-cli -- synth && cargo nextest run -p fabro-db && cargo nextest run -p fabro-mcp && cargo nextest run -p fabro-github && cargo nextest run -p fabro-synthesis -- render
else
  cargo test -p fabro-cli -- synth && cargo test -p fabro-db && cargo test -p fabro-mcp && cargo test -p fabro-github && cargo test -p fabro-synthesis -- render
fi
true`
  - Stdout: (empty)
  - Stderr:
    ```
    (294 lines omitted)
          /home/r/.cache/rust-tmp/.tmpky1qQ9/preview/malinka/prompts/implement/miner-service/plan.md
          /home/r/.cache/rust-tmp/.tmpky1qQ9/preview/malinka/prompts/implement/miner-service/review.md
          /home/r/.cache/rust-tmp/.tmpky1qQ9/preview/malinka/prompts/implement/miner-service/challenge.md
          /home/r/.cache/rust-tmp/.tmpky1qQ9/preview/malinka/prompts/implement/miner-service/polish.md
          /home/r/.cache/rust-tmp/.tmpky1qQ9/preview/malinka/workflows/implementation/miner-service-implement-codex-unblock.fabro
          /home/r/.cache/rust-tmp/.tmpky1qQ9/preview/malinka/run-configs/codex-unblock/miner-service-implement-codex-unblock.toml
          /home/r/.cache/rust-tmp/.tmpky1qQ9/preview/malinka/prompts/implementation/miner-service-implement-codex-unblock/plan.md
          /home/r/.cache/rust-tmp/.tmpky1qQ9/preview/malinka/prompts/implementation/miner-service-implement-codex-unblock/review.md
          /home/r/.cache/rust-tmp/.tmpky1qQ9/preview/malinka/prompts/implementation/miner-service-implement-codex-unblock/challenge.md
          /home/r/.cache/rust-tmp/.tmpky1qQ9/preview/malinka/prompts/implementation/miner-service-implement-codex-unblock/polish.md
          /home/r/.cache/rust-tmp/.tmpky1qQ9/preview/malinka/programs/myosu-miner-service-implementation.yaml
        ```
    
        stderr=""
    
        note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    
            PASS [   0.389s] (12/13) fabro-cli::synth synth_create_writes_plan_mapping_snapshots
            PASS [   0.450s] (13/13) fabro-cli::synth synth_create_refreshes_heuristic_mappings_when_plan_changes
    ────────────
         Summary [   0.453s] 13 tests run: 10 passed, 3 failed, 311 skipped
            FAIL [   0.167s] ( 7/13) fabro-cli::synth synth_evolve_updates_existing_package
            FAIL [   0.295s] (10/13) fabro-cli::synth synth_evolve_can_import_current_package_without_blueprint_flag
            FAIL [   0.303s] (11/13) fabro-cli::synth synth_evolve_preview_stays_bounded_to_manifest_and_report
    error: test run failed
    ```
- **contract**: success
  - Model: MiniMax-M2.7-highspeed, 11.4k tokens in / 292 out
- **implement**: success
  - Model: MiniMax-M2.7-highspeed, 69.8k tokens in / 440 out
  - Files: lib/crates/fabro-cli/tests/synth.rs
- **verify**: fail
  - Script: `if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p fabro-cli -- synth && cargo nextest run -p fabro-db && cargo nextest run -p fabro-mcp && cargo nextest run -p fabro-github && cargo nextest run -p fabro-synthesis -- render
else
  cargo test -p fabro-cli -- synth && cargo test -p fabro-db && cargo test -p fabro-mcp && cargo test -p fabro-github && cargo test -p fabro-synthesis -- render
fi`
  - Stdout: (empty)
  - Stderr:
    ```
    (200 lines omitted)
      stderr ───
    
        thread 'render::tests::inject_workspace_verify_lanes_adds_parent_holistic_gauntlet' (3647324) panicked at lib/crates/fabro-synthesis/src/render.rs:6919:14:
        preflight unit exists
        note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    
            PASS [   0.005s] (44/68) fabro-synthesis render::tests::manual_notes_from_markdown_keeps_manual_proof_lines
            PASS [   0.005s] (45/68) fabro-synthesis render::tests::parent_holistic_minimax_workflow_uses_minimax_first_pass
            PASS [   0.004s] (46/68) fabro-synthesis render::tests::promotion_contract_requires_security_fields_for_sensitive_lane
            PASS [   0.023s] (47/68) fabro-synthesis render::tests::bootstrap_run_config_targets_local_branch_when_origin_is_missing
            PASS [   0.005s] (48/68) fabro-synthesis render::tests::normalize_lane_verify_command_wraps_explicit_nextest_command
            PASS [   0.006s] (49/68) fabro-synthesis render::tests::observability_notes_from_markdown_keeps_structured_log_signals
            PASS [   0.005s] (50/68) fabro-synthesis render::tests::proof_commands_from_markdown_collects_commands_from_fenced_block
            PASS [   0.024s] (51/68) fabro-synthesis render::tests::bootstrap_run_config_uses_minimax_defaults_and_direct_integration
            PASS [   0.006s] (52/68) fabro-synthesis render::tests::parent_holistic_deep_and_adjudication_use_expected_provider_failover
            PASS [   0.006s] (53/68) fabro-synthesis render::tests::normalize_blueprint_lane_kinds_downgrades_invalid_implementation_integration_kind
            PASS [   0.007s] (54/68) fabro-synthesis render::tests::prompt_context_block_extracts_named_section
            PASS [   0.017s] (55/68) fabro-synthesis render::tests::implementation_run_config_enables_direct_integration
    ────────────
         Summary [   0.027s] 55/68 tests run: 52 passed, 3 failed, 27 skipped
            FAIL [   0.006s] (36/68) fabro-synthesis render::tests::inject_workspace_verify_lanes_document_release_is_hard_gate_and_retro_is_tail
            FAIL [   0.009s] (39/68) fabro-synthesis render::tests::inject_workspace_verify_lanes_adds_conditional_parent_lanes_for_sensitive_plan
            FAIL [   0.009s] (43/68) fabro-synthesis render::tests::inject_workspace_verify_lanes_adds_parent_holistic_gauntlet
    warning: 13/68 tests were not run due to test failure (run with --no-fail-fast to run all tests, or run with --max-fail)
    error: test run failed
    ```

## Context
- failure_class: canceled
- failure_signature: verify|canceled|script failed with exit code: <n> ## stderr blocking waiting for file lock on package cache blocking waiting for file lock on package cache blocking waiting for file lock on package cache compiling fabro-cli v0.<n>.<n> (/home/r/.fabro/runs/


# Fabro Db Baseline Tests Lane — Fixup

Fix only the current slice for `test-coverage-critical-paths-fabro-db-baseline-tests`.


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
