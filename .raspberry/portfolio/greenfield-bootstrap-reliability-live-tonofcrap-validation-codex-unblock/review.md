# Deep Review Findings

## Classification

- inside lane-owned surface: yes
- outside lane-owned surface: historical yes
- owned proof gate is already green, but the next replay still had one lane-local audit blocker before this turn's harness patch

## Evidence Reviewed

- Failed source run `01KMNTNRQ98FRA9RZ7BWYBQ6AQ` recorded `quality_ready: no` because a repo-wide lane-sizing scan matched unrelated code in `lib/crates/fabro-synthesis/src/render.rs`.
- The same source run's verification artifact incorrectly treated `raspberry autodev --max-cycles 5` as full Milestone 5 success even though the contract requires a real 30-cycle run.
- Current `HEAD` already contains the source-lane remediation for those issues:
  - [malinka/workflows/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation.fabro](/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/malinka/workflows/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation.fabro) removes the repo-wide semantic-risk and lane-sizing scans for this validation-only lane.
  - [malinka/prompts/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation/plan.md](/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/malinka/prompts/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation/plan.md) and [malinka/prompts/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation/review.md](/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/malinka/prompts/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation/review.md) explicitly forbid claiming Milestone 5 completion from a smoke run.
- The current branch diff still contains only lane-owned workflow/prompt edits for the source lane, not `.rs`/`.ts` product-code changes.
- Before this patch, the source lane audit gate only accepted changed files in `*.rs`, `*.toml`, `*.py`, `*.js`, `*.ts`, `*.tsx`, `*.go`, `*.java`, `*.rb`, `*.yaml`, `*.yml`, `*.json`, `*.sol`, or `*.sh`, so it could still fail replay even after the lane-local quality and prompt fixes landed.

## Root Cause

The historical failure was mixed:

- External false positive: the source lane's old repo-wide quality scan failed on unrelated code outside the lane-owned surfaces.
- Lane-owned verification gap: the source lane allowed a misleading verification writeup that overstated a 5-cycle smoke run as full live-validation proof.

After those fixes, one lane-owned harness blocker still remained: the source lane audit step ignored the lane's actual owned `.fabro` and prompt `.md` surfaces, so a truthful replay on this branch could still fail at audit even though the substantive blocker was already resolved.

## Fix Plan For Fixup

1. Land or confirm the audit change in [malinka/workflows/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation.fabro](/home/r/.fabro/runs/20260326-01KMP4JS1ZRV66RC5JXTQ76WJC/worktree/malinka/workflows/implementation/greenfield-bootstrap-reliability-live-tonofcrap-validation.fabro) so the audit gate accepts either code-surface diffs or lane-owned workflow/prompt diffs for this validation-only slice.
2. Replay `greenfield-bootstrap-reliability-live-tonofcrap-validation` on the current branch state instead of reopening unrelated synthesis files.
3. If fixup sees a fresh failure from pre-existing external code, warnings, or dependency issues outside the lane-owned surfaces, treat that as an external blocker and fix that external surface directly rather than rewriting this validation harness again.
4. Keep Milestone 5 open until verification includes a real `raspberry autodev --max-cycles 30` run; shorter smoke runs remain diagnostic only.
