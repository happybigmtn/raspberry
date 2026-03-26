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
    error: no such command: `nextest`
    
    help: a command with a similar name exists: `test`
    
    help: view all installed commands with `cargo --list`
    help: find a package to install `nextest` with `cargo search cargo-nextest`
    ```

## Context
- failure_class: deterministic
- failure_signature: verify|deterministic|script failed with exit code: <n> ## stderr error: no such command: `nextest` help: a command with a similar name exists: `test` help: view all installed commands with `cargo --list` help: find a package to install `nextest` with `cargo sea


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
