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
if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run --workspace && cargo nextest run -p fabro-cli -- synth && cargo nextest run -p fabro-db && cargo nextest run -p fabro-mcp && cargo nextest run -p fabro-github && cargo nextest run -p fabro-synthesis -- render && cargo nextest run -p raspberry-supervisor -- integration autodev_cycle && cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
else
  cargo test --workspace && cargo test -p fabro-cli -- synth && cargo test -p fabro-db && cargo test -p fabro-mcp && cargo test -p fabro-github && cargo test -p fabro-synthesis -- render && cargo test -p raspberry-supervisor -- integration autodev_cycle && cargo test -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed
fi
true`
  - Stdout: (empty)
  - Stderr:
    ```
    (29 lines omitted)
       Compiling raspberry-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY4EVV8YYR6SM2CZT0/worktree/lib/crates/raspberry-cli)
       Compiling fabro-api v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY4EVV8YYR6SM2CZT0/worktree/lib/crates/fabro-api)
       Compiling fabro-slack v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY4EVV8YYR6SM2CZT0/worktree/lib/crates/fabro-slack)
    error[E0062]: field `failure_kind` specified more than once
        --> lib/crates/raspberry-tui/src/app.rs:1277:13
         |
    1264 |             failure_kind: None,
         |             ------------------ first use of `failure_kind`
    ...
    1277 |             failure_kind: None,
         |             ^^^^^^^^^^^^ used more than once
    
    error[E0062]: field `recovery_action` specified more than once
        --> lib/crates/raspberry-tui/src/app.rs:1278:13
         |
    1265 |             recovery_action: None,
         |             --------------------- first use of `recovery_action`
    ...
    1278 |             recovery_action: None,
         |             ^^^^^^^^^^^^^^^ used more than once
    
    For more information about this error, try `rustc --explain E0062`.
    error: could not compile `raspberry-tui` (lib test) due to 2 previous errors
    warning: build failed, waiting for other jobs to finish...
    error: command `/home/r/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo test --no-run --message-format json-render-diagnostics --workspace` exited with code 101
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