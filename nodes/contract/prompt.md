Goal: Use Codex to unblock implementation lane `test-coverage-critical-paths-ci-preservation-and-hardening:test-coverage-critical-paths-ci-preservation-and-hardening`.

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
       Compiling fabro-types v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-types)
       Compiling fabro-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-cli)
       Compiling raspberry-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/raspberry-cli)
        Checking fabro-db v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-db)
        Checking fabro-telemetry v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-telemetry)
        Checking fabro-openai-oauth v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-openai-oauth)
        Checking fabro-beastie v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-beastie)
        Checking fabro-validate v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-validate)
        Checking fabro-tracker v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-tracker)
        Checking fabro-config v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-config)
        Checking fabro-llm v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-llm)
        Checking fabro-interview v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-interview)
        Checking fabro-devcontainer v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-devcontainer)
        Checking fabro-sandbox v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-sandbox)
        Checking fabro-mcp v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-mcp)
        Checking fabro-agent v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-agent)
        Checking fabro-retro v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-retro)
        Checking fabro-hooks v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-hooks)
        Checking fabro-workflows v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-workflows)
        Checking raspberry-supervisor v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/raspberry-supervisor)
        Checking fabro-api v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-api)
        Checking fabro-slack v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-slack)
        Checking fabro-synthesis v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/fabro-synthesis)
        Checking raspberry-tui v0.176.2 (/home/r/.fabro/runs/20260326-01KMPDR722Z5PSNKA3YXWRDGZG/worktree/lib/crates/raspberry-tui)
        Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.21s
    ```


Read the implementation plan carefully. Before writing any code, run `mkdir -p .fabro-work` and write `.fabro-work/contract.md` defining what DONE looks like for this lane.

Format:

## Deliverables
List every file you will create or modify, one per line with backtick path.

## Acceptance Criteria
List 3-8 testable conditions that prove the implementation works. Each must be verifiable by running a command or checking file content.

## Out of Scope
List what this lane will NOT implement.

This is an unblock lane. Prefer the smallest lane-local workflow, prompt, or harness repair that lets the source lane pass its next replay. Do not broaden into unrelated repo cleanup.

Do NOT write any source code in this stage. Do NOT create new docs, broad cleanup tasks, or unrelated package edits unless the lane explicitly requires them. Finish by writing the contract only.
