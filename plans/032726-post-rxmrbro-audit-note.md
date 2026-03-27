# Audit Note: Post-rXMRbro Systems Hardening

## Scope of this review branch

Branch: `codex/post-rxmrbro-systems-hardening`

Primary commit: `d87754f5`

This branch is a narrowly scoped fabro hardening slice extracted from live proving work against `rXMRbro`. It is intended for audit/review of system behavior, not as a full cleanup of the current fabro worktree.

## What this branch changes

1. It hardens autodev target-repo freshness checks so a controller checkout with a non-pushable `origin` remote is rejected before dispatch.

2. It preserves the earlier service-proof fixes already present in the staged slice:
   - service `verify` for long-running `cargo run ...` commands becomes a startup smoke
   - service `preflight` uses the raw verify command plus health probe rather than double-wrapping the smoke script

3. It keeps the recently added supervisor behavior around:
   - reclassifying stale `unknown` failures from current error text
   - upgrading recovery actions when better evidence exists
   - preferring source-lane replay after controller-path failures instead of starving behind generic unblock churn

## Why this branch exists

During live `rXMRbro` remediation, one failure mode was especially costly: a controller checkout could appear “fresh enough” for autodev, yet still fail every new branch-backed run because its `origin` remote was a local filesystem path inherited from another clone. That meant the failure was discovered too late, after dispatch and run startup, instead of being rejected at the controller boundary.

This branch closes that gap.

## Evidence that motivated it

Representative live failure text:

    failed to resolve SSH push URL: Engine error: remote `origin` must use SSH
    or be convertible from GitHub HTTPS, got `/home/r/coding/rXMRbro-autodev-fresh2`

Representative live proof-generation failure that was already folded into the staged slice:

    /bin/bash: -c: line 12: syntax error near unexpected token `&'

That shell error came from a service preflight path that double-wrapped a smoke-style verify script.

## Files auditors should focus on

- `lib/crates/raspberry-supervisor/src/autodev.rs`
- `lib/crates/raspberry-supervisor/src/dispatch.rs`
- `lib/crates/raspberry-supervisor/src/failure.rs`
- `lib/crates/fabro-synthesis/src/render.rs`
- `plans/032726-post-rxmrbro-systems-hardening.md`

## Validation already run

Focused supervisor tests:

    cargo test -p raspberry-supervisor ensure_target_repo_fresh_for_dispatch_rejects_local_path_origin
    cargo test -p raspberry-supervisor replayable_failed_lanes_upgrade_unknown_recovery_action_from_last_error
    cargo test -p raspberry-supervisor prioritized_failure_lane_keys_prefer_source_replay_after_remote_fix
    cargo test -p raspberry-supervisor replayable_failed_lanes_dispatch_codex_unblock_for_provider_access_limits

Build:

    cargo build -p fabro-cli -p raspberry-cli --target-dir target-local

Live proving-ground checks after rebuild/rerender:

    ./target-local/debug/fabro --no-upgrade-check synth create --target-repo /home/r/coding/rXMRbro-autodev-fresh3 ...
    ./target-local/debug/raspberry autodev --manifest /home/r/coding/rXMRbro-autodev-fresh3/malinka/programs/rxmragent.yaml ...

Observed result:

- fresh source lanes launched again
- the old local-path `origin` failure was absent from the new failed-lane snapshot
- the generated `house-agent-ws-accept-loop.fabro` retained the corrected single-wrapper `preflight`

## What remains out of scope

This branch does not claim to solve all fabro or `rXMRbro` issues.

Still out of scope:

- broader proof-script failures in product repos
- repo-specific warning/quality debt
- any future scheduler/replay tuning beyond the currently validated recovery logic
- cleanup of the many unrelated modified files in the fabro worktree

## Review question

The most important review question for this branch is:

“Does autodev now reject obviously broken controller provenance early enough, using the same remote-validity rule that branch-backed integration already expects later in the pipeline?”
