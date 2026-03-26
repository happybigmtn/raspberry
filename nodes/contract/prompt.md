Goal: Use Codex to unblock implementation lane `greenfield-bootstrap-reliability-live-tonofcrap-validation:greenfield-bootstrap-reliability-live-tonofcrap-validation`.

Inspect the source lane's most recent failure/remediation context and apply the minimal code or harness changes needed so the source lane can pass its next replay.

Proof commands:
- `cargo check --workspace`


## Completed stages
- **preflight**: success
  - Script: `set +e
cargo check --workspace
true`
  - Stdout: (empty)
  - Stderr:
    ```
    (5 lines omitted)
       Compiling fabro-types v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-types)
       Compiling raspberry-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/raspberry-cli)
       Compiling fabro-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-cli)
        Checking fabro-db v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-db)
        Checking fabro-openai-oauth v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-openai-oauth)
        Checking fabro-telemetry v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-telemetry)
        Checking fabro-beastie v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-beastie)
        Checking fabro-validate v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-validate)
        Checking fabro-tracker v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-tracker)
        Checking fabro-config v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-config)
        Checking fabro-llm v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-llm)
        Checking fabro-interview v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-interview)
        Checking fabro-devcontainer v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-devcontainer)
        Checking fabro-mcp v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-mcp)
        Checking fabro-sandbox v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-sandbox)
        Checking fabro-agent v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-agent)
        Checking fabro-hooks v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-hooks)
        Checking fabro-retro v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-retro)
        Checking fabro-workflows v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-workflows)
        Checking raspberry-supervisor v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/raspberry-supervisor)
        Checking fabro-api v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-api)
        Checking fabro-slack v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-slack)
        Checking fabro-synthesis v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/fabro-synthesis)
        Checking raspberry-tui v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/lib/crates/raspberry-tui)
        Finished `dev` profile [unoptimized + debuginfo] target(s) in 21.26s
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