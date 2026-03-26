I wrote the replay diagnosis to [.fabro-work/deep-review-findings.md](/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/.fabro-work/deep-review-findings.md). It explicitly classifies the repeated verify failure as external-only: inside lane-owned surface, no remaining blocker found; outside lane-owned surface, yes. The note tells fixup to replay the source lane unchanged first, then only fix external `lib/crates/**` debt or the `fabro-api` `nextest` linker environment if the `rust-lld`/`cc` signal 7 issue comes back.

No source-lane code or harness edit was added, because the owned workflow already preserves the intended fmt/clippy/workspace-nextest proof gate and the current evidence does not justify weakening it.

Verification: `cargo check --workspace` passed.