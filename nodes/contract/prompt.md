Goal: Synthesis Runtime Regression Tests

Child work item of plan: Test Coverage for Critical Paths

Proof commands:
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
  cargo nextest run -p fabro-synthesis -- render
else
  cargo test -p fabro-synthesis -- render
fi
true`
  - Stdout: (empty)
  - Stderr:
    ```
    (104 lines omitted)
    
        failures:
            render::tests::inject_workspace_verify_lanes_document_release_is_hard_gate_and_retro_is_tail
    
        test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 89 filtered out; finished in 0.00s
    
      stderr ───
    
        thread 'render::tests::inject_workspace_verify_lanes_document_release_is_hard_gate_and_retro_is_tail' (3409883) panicked at lib/crates/fabro-synthesis/src/render.rs:7260:14:
        document release unit exists
        note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    
            PASS [   0.006s] (50/68) fabro-synthesis render::tests::parent_holistic_deep_and_adjudication_use_expected_provider_failover
            PASS [   0.026s] (51/68) fabro-synthesis render::tests::bootstrap_run_config_omits_direct_integration_for_non_git_repo
            PASS [   0.005s] (52/68) fabro-synthesis render::tests::proof_commands_from_markdown_collects_commands_from_fenced_block
            PASS [   0.029s] (53/68) fabro-synthesis render::tests::bootstrap_run_config_uses_minimax_defaults_and_direct_integration
            PASS [   0.029s] (54/68) fabro-synthesis render::tests::bootstrap_run_config_targets_local_branch_when_origin_is_missing
            PASS [   0.019s] (55/68) fabro-synthesis render::tests::implementation_run_config_enables_direct_integration
    ────────────
         Summary [   0.031s] 55/68 tests run: 52 passed, 3 failed, 27 skipped
            FAIL [   0.006s] (36/68) fabro-synthesis render::tests::inject_workspace_verify_lanes_adds_conditional_parent_lanes_for_sensitive_plan
            FAIL [   0.008s] (37/68) fabro-synthesis render::tests::inject_workspace_verify_lanes_adds_parent_holistic_gauntlet
            FAIL [   0.009s] (49/68) fabro-synthesis render::tests::inject_workspace_verify_lanes_document_release_is_hard_gate_and_retro_is_tail
    warning: 13/68 tests were not run due to test failure (run with --no-fail-fast to run all tests, or run with --max-fail)
    error: test run failed
    ```


Read the implementation plan carefully. Before writing any code, write .fabro-work/contract.md defining what DONE looks like for this lane.

Format:

## Deliverables
List every file you will create or modify, one per line with backtick path.

## Acceptance Criteria
List 3-8 testable conditions that prove the implementation works. Each must be verifiable by running a command or checking file content.

## Out of Scope
List what this lane will NOT implement.

Do NOT write any source code. Only write the contract. Run `mkdir -p .fabro-work` first.