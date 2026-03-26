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
- **contract**: success
  - Model: MiniMax-M2.7-highspeed, 22.6k tokens in / 152 out
- **implement**: fail
- **verify**: fail
  - Script: `if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p fabro-synthesis -- bootstrap_verify
else
  cargo test -p fabro-synthesis -- bootstrap_verify
fi`
  - Stdout: (empty)
  - Stderr:
    ```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.17s
    ────────────
     Nextest run ID 7f9f4d57-e1af-4ac7-a190-f1aae19802fb with nextest profile: default
        Starting 0 tests across 2 binaries (95 tests skipped)
    ────────────
         Summary [   0.000s] 0 tests run: 0 passed, 95 skipped
    error: no tests to run
    (hint: use `--no-tests` to customize)
    ```

## Context
- failure_class: deterministic
- failure_signature: verify|deterministic|script failed with exit code: <n> ## stderr finished `test` profile [unoptimized + debuginfo] target(s) in <n>.17s ──────────── nextest run id <hex>-e1af-4ac7-a190-<hex> with nextest profile: default starting <n> tes


# Bootstrap Verification Gate Lane — Fixup

Fix only the current slice for `greenfield-bootstrap-reliability-bootstrap-verification-gate`.


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
