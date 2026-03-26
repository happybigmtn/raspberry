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


Read the implementation plan carefully. Before writing any code, write .fabro-work/contract.md defining what DONE looks like for this lane.

Format:

## Deliverables
List every file you will create or modify, one per line with backtick path.

## Acceptance Criteria
List 3-8 testable conditions that prove the implementation works. Each must be verifiable by running a command or checking file content.

## Out of Scope
List what this lane will NOT implement.

Do NOT write any source code. Only write the contract. Run `mkdir -p .fabro-work` first.