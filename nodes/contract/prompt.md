Goal: Minimal Coverage For Fabro Mcp And Fabro Github

Child work item of plan: Test Coverage for Critical Paths

Proof commands:
- `cargo nextest run -p fabro-cli -- synth`
- `cargo nextest run -p fabro-db`
- `cargo nextest run -p fabro-mcp && cargo nextest run -p fabro-github`
- `cargo nextest run -p fabro-synthesis -- render`
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
if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p fabro-cli -- synth && cargo nextest run -p fabro-db && cargo nextest run -p fabro-mcp && cargo nextest run -p fabro-github && cargo nextest run -p fabro-synthesis -- render && cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
else
  cargo test -p fabro-cli -- synth && cargo test -p fabro-db && cargo test -p fabro-mcp && cargo test -p fabro-github && cargo test -p fabro-synthesis -- render && cargo test -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
fi
true`
  - Stdout: (empty)
  - Stderr:
    ```
    (292 lines omitted)
          /home/r/.cache/rust-tmp/.tmptNQQBp/preview/malinka/prompts/implement/miner-service/plan.md
          /home/r/.cache/rust-tmp/.tmptNQQBp/preview/malinka/prompts/implement/miner-service/review.md
          /home/r/.cache/rust-tmp/.tmptNQQBp/preview/malinka/prompts/implement/miner-service/challenge.md
          /home/r/.cache/rust-tmp/.tmptNQQBp/preview/malinka/prompts/implement/miner-service/polish.md
          /home/r/.cache/rust-tmp/.tmptNQQBp/preview/malinka/workflows/implementation/miner-service-implement-codex-unblock.fabro
          /home/r/.cache/rust-tmp/.tmptNQQBp/preview/malinka/run-configs/codex-unblock/miner-service-implement-codex-unblock.toml
          /home/r/.cache/rust-tmp/.tmptNQQBp/preview/malinka/prompts/implementation/miner-service-implement-codex-unblock/plan.md
          /home/r/.cache/rust-tmp/.tmptNQQBp/preview/malinka/prompts/implementation/miner-service-implement-codex-unblock/review.md
          /home/r/.cache/rust-tmp/.tmptNQQBp/preview/malinka/prompts/implementation/miner-service-implement-codex-unblock/challenge.md
          /home/r/.cache/rust-tmp/.tmptNQQBp/preview/malinka/prompts/implementation/miner-service-implement-codex-unblock/polish.md
          /home/r/.cache/rust-tmp/.tmptNQQBp/preview/malinka/programs/myosu-miner-service-implementation.yaml
        ```
    
        stderr=""
    
        note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    
            PASS [   0.079s] (12/13) fabro-cli::synth synth_create_writes_plan_mapping_snapshots
            PASS [   0.126s] (13/13) fabro-cli::synth synth_create_refreshes_heuristic_mappings_when_plan_changes
    ────────────
         Summary [   0.126s] 13 tests run: 10 passed, 3 failed, 311 skipped
            FAIL [   0.033s] ( 9/13) fabro-cli::synth synth_evolve_updates_existing_package
            FAIL [   0.034s] (10/13) fabro-cli::synth synth_evolve_can_import_current_package_without_blueprint_flag
            FAIL [   0.039s] (11/13) fabro-cli::synth synth_evolve_preview_stays_bounded_to_manifest_and_report
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