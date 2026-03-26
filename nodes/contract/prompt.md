Goal: Bootstrap Verification Gate

Child work item of plan: Greenfield Bootstrap and Runtime Asset Reliability

Proof commands:
- `cargo nextest run -p fabro-synthesis -- bootstrap_verify`

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
  cargo nextest run -p fabro-synthesis -- bootstrap_verify
else
  cargo test -p fabro-synthesis -- bootstrap_verify
fi
true`
  - Stdout: (empty)
  - Stderr:
    ```
    (2 lines omitted)
       Compiling fabro-model v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-model)
       Compiling fabro-github v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-github)
       Compiling fabro-config v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-config)
       Compiling fabro-llm v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-llm)
       Compiling fabro-mcp v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-mcp)
       Compiling fabro-sandbox v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-sandbox)
       Compiling fabro-agent v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-agent)
       Compiling fabro-graphviz v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-graphviz)
       Compiling fabro-validate v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-validate)
       Compiling fabro-git-storage v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-git-storage)
       Compiling fabro-devcontainer v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-devcontainer)
       Compiling fabro-retro v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-retro)
       Compiling fabro-hooks v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-hooks)
       Compiling fabro-interview v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-interview)
       Compiling fabro-workflows v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-workflows)
       Compiling raspberry-supervisor v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/raspberry-supervisor)
       Compiling fabro-synthesis v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT833HPXN6WF5VEVPSSPZ1/worktree/lib/crates/fabro-synthesis)
        Finished `test` profile [unoptimized + debuginfo] target(s) in 2m 35s
    ────────────
     Nextest run ID 7afbcd15-200d-444f-87cf-6e3d0fbefe83 with nextest profile: default
        Starting 0 tests across 2 binaries (95 tests skipped)
    ────────────
         Summary [   0.000s] 0 tests run: 0 passed, 95 skipped
    error: no tests to run
    (hint: use `--no-tests` to customize)
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