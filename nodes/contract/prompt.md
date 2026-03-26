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
    (102 lines omitted)
        test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 89 filtered out; finished in 0.00s
    
      stderr ───
    
        thread 'render::tests::inject_workspace_verify_lanes_document_release_is_hard_gate_and_retro_is_tail' (2136066) panicked at lib/crates/fabro-synthesis/src/render.rs:7260:14:
        document release unit exists
        note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    
            PASS [   0.007s] (47/68) fabro-synthesis render::tests::observability_notes_from_markdown_keeps_structured_log_signals
            PASS [   0.007s] (48/68) fabro-synthesis render::tests::parent_holistic_deep_and_adjudication_use_expected_provider_failover
            PASS [   0.004s] (49/68) fabro-synthesis render::tests::proof_commands_from_markdown_collects_commands_from_fenced_block
            PASS [   0.005s] (50/68) fabro-synthesis render::tests::prompt_context_block_extracts_named_section
            PASS [   0.007s] (51/68) fabro-synthesis render::tests::manual_notes_from_markdown_keeps_manual_proof_lines
            PASS [   0.022s] (52/68) fabro-synthesis render::tests::bootstrap_run_config_omits_direct_integration_for_non_git_repo
            PASS [   0.005s] (53/68) fabro-synthesis render::tests::raw_lane_refs_finds_lane_like_tokens
            PASS [   0.031s] (54/68) fabro-synthesis render::tests::implementation_run_config_enables_direct_integration
            PASS [   0.039s] (55/68) fabro-synthesis render::tests::bootstrap_run_config_uses_minimax_defaults_and_direct_integration
            PASS [   0.040s] (56/68) fabro-synthesis render::tests::bootstrap_run_config_targets_local_branch_when_origin_is_missing
    ────────────
         Summary [   0.040s] 56/68 tests run: 53 passed, 3 failed, 27 skipped
            FAIL [   0.006s] (37/68) fabro-synthesis render::tests::inject_workspace_verify_lanes_adds_conditional_parent_lanes_for_sensitive_plan
            FAIL [   0.006s] (41/68) fabro-synthesis render::tests::inject_workspace_verify_lanes_adds_parent_holistic_gauntlet
            FAIL [   0.007s] (46/68) fabro-synthesis render::tests::inject_workspace_verify_lanes_document_release_is_hard_gate_and_retro_is_tail
    warning: 12/68 tests were not run due to test failure (run with --no-fail-fast to run all tests, or run with --max-fail)
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