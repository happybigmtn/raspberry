# Live Fabro Autodev Remediation

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, running `raspberry autodev --max-parallel 10` on the `fabro`
repo should behave the way a careful operator would execute the Genesis plans by
hand. The first wave should favor foundational stabilization work over broad
test-matrix expansion, spare capacity should not be stranded by harmless local
repo noise, and proof gates should fail for product reasons instead of missing
workstation tools.

The proof is: launch autodev against
`/home/r/coding/fabro/malinka/programs/fabro.yaml` with `--max-parallel 10`,
observe the first dispatch wave, then watch subsequent cycles continue filling
useful work instead of stalling on repo freshness or collapsing into low-value
test fan-out.

## Progress

- [x] (2026-03-26 18:52Z) Relaunched `fabro` autodev with `--max-parallel 10`
      using `target-local/release/raspberry` and `target-local/release/fabro`.
- [x] (2026-03-26 18:59Z) Collected live run evidence from
      `/home/r/.fabro/runs/*`, `.raspberry/fabro-autodev.json`, and the process
      table.
- [x] (2026-03-26 19:02Z) Identified three framework mismatches:
      repo-freshness dispatch starvation, over-aggressive dependency flattening,
      and non-portable `cargo nextest` proof gates.
- [x] (2026-03-26 19:19Z) Wrote and passed focused tests for repo freshness,
      explicit Genesis dependency edges, root-family ready-lane shaping, and
      portable `nextest` proof commands.
- [x] (2026-03-26 19:20Z) Patched `raspberry-supervisor` freshness logic so
      untracked local noise no longer blocks a safe fast-forward.
- [x] (2026-03-26 19:21Z) Patched `fabro-synthesis` so explicit Genesis
      dependency graph edges override coarse phase gating when they disagree.
- [x] (2026-03-26 19:22Z) Patched autodev ready-lane selection so the initial
      wave diversifies by root plan family and favors one targeted regression
      lane before broader `test-coverage-critical-paths` expansion.
- [x] (2026-03-26 19:26Z) Patched generated proof commands so `cargo nextest`
      falls back to `cargo test` when the worker environment lacks nextest, and
      threaded that normalization through explicit serialized verify commands.
- [x] (2026-03-26 19:32Z) Rebuilt release binaries and rerendered the checked
      `fabro` package from the updated blueprint.
- [ ] Run one final 10-slot autodev pass from a clean, dispatchable `fabro`
      checkout and record the post-fix frontier behavior.

## Surprises & Discoveries

- Observation: The relaunched 10-slot controller did not fill spare capacity.
  Evidence: The controller reported
  `dispatch skipped for program 'fabro': target repo is stale: local default branch is behind origin by 1 commits and the worktree is not clean`
  on cycles 1-19, while cycles 2-19 showed `ready=7`, `running=2`, and
  `dispatched=0`.

- Observation: The live autodev report was stale relative to the run dirs.
  Evidence: [`.raspberry/fabro-autodev.json`](/home/r/coding/fabro/.raspberry/fabro-autodev.json)
  still showed 5 running lanes after the three bootstrap lanes had already
  completed successfully in `/home/r/.fabro/runs/20260326-*`.

- Observation: Three foundation lanes completed successfully before the stall.
  Evidence:
  `01KMNPED223RBA6KFJEB9C16GA` (`autodev-efficiency-and-dispatch`) succeeded at
  `2026-03-26T18:38:37Z`;
  `01KMNPED21DMQJ8XQD379K3TP1` (`greenfield-bootstrap-reliability`) succeeded at
  `2026-03-26T18:39:29Z`;
  `01KMNPED2367YB0GPF0QE1ETVW` (`provider-policy-stabilization`) succeeded at
  `2026-03-26T18:39:33Z`.

- Observation: One active test lane failed for tooling reasons, not product
  reasons.
  Evidence: Run `01KMNPED23FTNQCCKZ0Z7GCPCJ`
  (`test-coverage-critical-paths-autodev-integration-test`) reached `verify`
  and failed with `error: no such command: nextest`, then entered `fixup`.

- Observation: `error-handling-hardening` is structurally blocked, not merely
  under-ranked.
  Evidence:
  [`malinka/programs/fabro.yaml`](/home/r/coding/fabro/malinka/programs/fabro.yaml:25)
  makes its bootstrap lane depend on
  `autodev-efficiency-and-dispatch-live-validation`,
  `greenfield-bootstrap-reliability-verify-scaffold-first-ordering`,
  `test-coverage-critical-paths-synthesis-runtime-regression-tests`, and
  `provider-policy-stabilization-live-validation`.

- Observation: The explicit Genesis dependency graph is more precise than the
  generated dependencies.
  Evidence:
  [`genesis/plans/001-master-plan.md`](/home/r/coding/fabro/genesis/plans/001-master-plan.md:110)
  says `002` depends on `003/005`, while the generated `fabro` program adds
  `004` and `008` as prerequisites via phase flattening.

- Observation: After the framework patch and package rerender, the generated
  `fabro` program now narrows `error-handling-hardening` back down to the
  intended external prerequisites.
  Evidence:
  `rg -n "error-handling-hardening|test-coverage-critical-paths-synthesis-runtime-regression-tests" malinka/programs/fabro.yaml`
  shows the `error-handling` units still depend on
  `autodev-efficiency-and-dispatch-live-validation` and the targeted regression
  lane, but no longer on
  `greenfield-bootstrap-reliability-verify-scaffold-first-ordering` or
  `provider-policy-stabilization-live-validation`.

- Observation: The rerendered workflow now contains a portable proof fallback.
  Evidence:
  `rg -n "cargo nextest --version >/dev/null 2>&1|cargo test -p raspberry-supervisor -- integration autodev_cycle" malinka/workflows/implementation/test-coverage-critical-paths-autodev-integration-test.fabro`
  shows both the `nextest` probe and the `cargo test` fallback in the generated
  `preflight` and `verify` scripts.

- Observation: The remaining live rerun blocker is checkout state, not the same
  framework failure seen earlier.
  Evidence: Relaunching autodev on the edited `fabro` checkout reports
  `ready=7`, `running=2`, `dispatched=0`, then eventually
  `target repo is stale: local default branch is behind origin by 1 commits and the worktree is not clean`.
  `git status -sb` shows the checkout is still `main...origin/main [behind 1]`
  with tracked changes from this repair pass, so the controller is correctly
  refusing to dispatch from a dirty branch.

## Decision Log

- Decision: Treat this as a framework repair loop, not a repo-local rescue.
  Rationale: The failures observed in the live run are produced by synthesis,
  scheduling, and runtime command generation. Manual lane resets would hide the
  underlying framework bugs and would not help the next proving-ground repo.
  Date/Author: 2026-03-26 / Codex

- Decision: Use the explicit Genesis dependency graph as the sharper source of
  truth when it conflicts with broad phase ordering.
  Rationale: The master plan already states narrower dependencies, and those
  edges better match how a human would execute the plans directly.
  Date/Author: 2026-03-26 / Codex

- Decision: Diversify the initial ready frontier by root plan family instead of
  greedily filling every open slot from the same family.
  Rationale: The first wave should launch the best representative from each
  foundational family. Broad test expansion is useful later, but it should not
  crowd out execution-path, bootstrap, or provider stabilization work.
  Date/Author: 2026-03-26 / Codex

- Failure scenario: Over-relaxing repo freshness could dispatch from a repo with
  tracked local edits that make fast-forward unsafe.
  Rationale: The freshness patch must distinguish tracked modifications from
  harmless untracked clutter. Tracked edits should still block or require a more
  explicit recovery path.
  Date/Author: 2026-03-26 / Codex

- Failure scenario: A naive `cargo nextest` fallback could silently weaken proof
  coverage or break commands with unusual argument shapes.
  Rationale: The portability fix must preserve `cargo nextest` when installed
  and only degrade to `cargo test` in a way that keeps the command semantics as
  close as possible.
  Date/Author: 2026-03-26 / Codex

## Outcomes & Retrospective

(To be filled as milestones land and the rerun evidence is collected.)

## Context and Orientation

The current live run and the fixes it requires span four parts of the codebase.

- `lib/crates/raspberry-supervisor/src/autodev.rs`
  This owns ready-lane prioritization, cycle orchestration, autodev reporting,
  and target-repo freshness checks before dispatch.

- `lib/crates/raspberry-supervisor/src/dispatch.rs`
  This calls `ensure_target_repo_fresh_for_dispatch(...)` and blocks dispatch
  when the target repo is considered stale.

- `lib/crates/fabro-synthesis/src/planning.rs`
  This turns the plan registry plus Genesis plan corpus into generated program
  units and lane dependencies. The live `error-handling-hardening` dependency
  shape comes from this layer.

- `lib/crates/fabro-synthesis/src/render.rs`
  This generates verify and preflight shell commands for implementation-family
  workflows. The `cargo nextest` portability issue originates here.

Live evidence gathered so far:

- Controller launch command:

      /home/r/coding/fabro/target-local/release/raspberry autodev \
        --manifest /home/r/coding/fabro/malinka/programs/fabro.yaml \
        --fabro-bin /home/r/coding/fabro/target-local/release/fabro \
        --max-parallel 10 \
        --max-cycles 1000000 \
        --poll-interval-ms 500 \
        --evolve-every-seconds 1800

- Initial controller output:

      [autodev] cycle=1 evolve=skipped ready=9 replayed=0 regenerate_noop=0 dispatched=5 running=5 complete=1
      [autodev] dispatch skipped for program `fabro`: target repo is stale: local default branch is behind origin by 1 commits and the worktree is not clean

- Subsequent cycles:

      [autodev] cycle=2..19 evolve=skipped ready=7 replayed=0 regenerate_noop=0 dispatched=0 running=2 complete=1

- Target repo dirt at the time of observation:
  untracked `.raspberry/` artifacts, untracked `target-local/` build outputs,
  and developer-local notebook files. The freshness gate treated all of this as
  blocking noise.

## Milestones

### Milestone 1: Freeze the live failure modes in tests

Add focused tests for the behaviors observed in the live `fabro` run.

Required tests:

- `raspberry-supervisor/src/autodev.rs`
  A repo that is behind origin but only has untracked local noise should still
  fast-forward successfully.
- `raspberry-supervisor/src/autodev.rs`
  Ready-lane selection should diversify the initial wave across root plan
  families and choose a targeted regression lane over broad `test-coverage`
  expansion.
- `fabro-synthesis/src/planning.rs`
  Explicit `depends on` clauses from the Genesis dependency graph should
  override broad prior-phase injection when both are present.
- `fabro-synthesis/src/render.rs`
  Verify-command generation should preserve `cargo nextest` when available and
  produce a portable fallback when it is not.

Proof commands:

    cargo nextest run -p raspberry-supervisor -- target_repo_fresh ready_lane_dispatch
    cargo nextest run -p fabro-synthesis -- planning render

### Milestone 2: Make dispatch freshness pragmatic

Patch target-repo freshness checks so harmless untracked clutter does not block
safe fast-forward dispatch. Tracked modifications must still block if they make
dispatch unsafe.

Likely files:

- `lib/crates/raspberry-supervisor/src/autodev.rs`
- `lib/crates/raspberry-supervisor/src/dispatch.rs`

Acceptance:

- Relaunching the controller on `fabro` no longer stalls with
  `BehindWithLocalChanges` when the repo only has untracked local artifacts.
- A repo with tracked local edits still does not dispatch blindly.

Proof commands:

    cargo nextest run -p raspberry-supervisor -- target_repo_fresh

### Milestone 3: Make synthesis honor the real Genesis graph

Patch planning synthesis so explicit dependency edges from
`genesis/plans/001-master-plan.md` override coarse phase-wide prerequisites when
they disagree. The generated `fabro` manifest should stop adding `004` and `008`
as prerequisites for `002` if the graph only calls for `003/005`.

Likely file:

- `lib/crates/fabro-synthesis/src/planning.rs`

Acceptance:

- Regenerated `fabro` manifest gives `error-handling-hardening` only the
  dependencies the Genesis graph actually requires.

Proof commands:

    cargo nextest run -p fabro-synthesis -- planning
    target-local/release/fabro --no-upgrade-check synth create \
      --target-repo /home/r/coding/fabro \
      --program fabro \
      --blueprint /home/r/coding/fabro/malinka/blueprints/fabro.yaml \
      --no-decompose --no-review

### Milestone 4: Shape the initial frontier like a human would

Patch autodev ready-lane selection so the first wave preserves breadth across
root plan families. Within `test-coverage-critical-paths`, targeted regression
or edge-case work should outrank broad CI or full-autodev expansion lanes.

Likely file:

- `lib/crates/raspberry-supervisor/src/autodev.rs`

Acceptance:

- On the regenerated `fabro` manifest, the first ready wave favors:
  `autodev-efficiency-and-dispatch`,
  `greenfield-bootstrap-reliability`,
  `provider-policy-stabilization`,
  and one targeted `test-coverage` lane before additional `test-coverage`
  expansion.

Proof commands:

    cargo nextest run -p raspberry-supervisor -- ready_lane_dispatch

### Milestone 5: Make proof commands portable across proving grounds

Patch verify-command generation so lanes do not fail just because
`cargo nextest` is not installed in the worker environment. The preferred tool
should remain `cargo nextest`; the fallback should keep the proof meaningful.

Likely file:

- `lib/crates/fabro-synthesis/src/render.rs`

Acceptance:

- Regenerated implementation workflows contain a portable proof command for
  `cargo nextest`-based checks.
- A lane equivalent to `test-coverage-critical-paths-autodev-integration-test`
  does not fail immediately with `no such command: nextest`.

Proof commands:

    cargo nextest run -p fabro-synthesis -- render

### Milestone 6: Rebuild, relaunch, and capture the new live evidence

Rebuild `fabro` and `raspberry`, relaunch autodev with `--max-parallel 10`,
then record the new first-wave ordering and the next few cycles in this file.

Proof commands:

    cargo build --release -p fabro-cli -p raspberry-cli --target-dir target-local

    target-local/release/raspberry autodev \
      --manifest /home/r/coding/fabro/malinka/programs/fabro.yaml \
      --fabro-bin /home/r/coding/fabro/target-local/release/fabro \
      --max-parallel 10 \
      --max-cycles 20 \
      --poll-interval-ms 500 \
      --evolve-every-seconds 1800

Expected:

- The controller fills more than 2 live slots after the first three foundation
  lanes settle.
- The first wave is shaped by plan family, not by greedy test expansion.
- The autodev report stays closer to the real run-dir/process truth.

Current status:

- Rebuild completed.
- Package rerender completed.
- Relaunch attempted on the live `fabro` checkout.
- Remaining blocker: the checkout is still behind `origin/main` and dirty from
  the current repair pass, so a final dispatchable rerun requires either:
  1. a clean fast-forwarded `fabro` checkout with these changes committed, or
  2. a clean proving-ground clone of `fabro` carrying the same patch set.

## Validation and Acceptance

This repair is complete when the `fabro` proving ground behaves like the Genesis
roadmap rather than fighting it:

- `raspberry autodev --max-parallel 10` does not strand spare capacity because
  of harmless local repo noise.
- The first dispatch wave reflects human-prioritized stabilization breadth.
- `error-handling-hardening` becomes available at the right time based on the
  explicit Genesis dependency graph, not on over-broad phase flattening.
- Test-oriented lanes fail for real regressions, not immediately because the
  worker lacks `cargo nextest`.
