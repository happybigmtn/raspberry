# End-to-End Autodev Review and Remediation

Date: 2026-03-25

## Goal

Audit the full autodev pipeline using recent `rXMRbro` and `tonofcrap` runs,
identify the highest-value remediation work, implement the critical fixes, and
relaunch both repos with 5 active agents on the repaired harness.

## Recently Observed Evidence

- `rXMRbro` produced strong autonomous progress, but the control plane showed
  stale or misleading status behavior under long-lived operation.
- `tonofcrap` exposed two systemic startup failures:
  `pi-coding-agent` global npm install races and prompt validation failures from
  shell-style `$surface` tokens appearing inside inlined prompt files.
- `raspberry status` on large frontiers degraded badly because it kept scanning
  historical run directories for settled lanes.
- The existing `plan-review` direction is partially synthesized into blueprints
  but not yet wired into autodev plan-completion detection or a dedicated
  Codex-first review workflow.

## Audit Findings

### Fixed In This Pass

- [x] Prompt expansion no longer treats unrelated shell/code `$name` tokens in
      inlined prompt files as Fabro runtime variables.
  - File: `lib/crates/fabro-workflows/src/handler/agent.rs`
  - Evidence: `Undefined variable: $surface` failures during implementation
    stages are cleared in fresh release-run state.

- [x] CLI bootstrap no longer installs agent CLIs into the shared host-global
      npm prefix during parallel autodev lane startup.
  - File: `lib/crates/fabro-workflows/src/backend/cli.rs`
  - Fix: use a Fabro-managed npm prefix plus a shared npm-global install lock.

- [x] Supervisor runtime refresh no longer scans stale `last_run_id`s for
      settled lanes, and missing tracked running runs are downgraded instead of
      lingering forever.
  - File: `lib/crates/raspberry-supervisor/src/program_state.rs`
  - Impact: `raspberry status` becomes responsive again on large frontiers.

### Remaining Remediation

- [ ] Make persisted `lane.recovery_action` authoritative during autodev replay
- [x] Make persisted `lane.recovery_action` authoritative during autodev replay
      selection.
  - Problem: replay escalation from `ReplayLane` to `SurfaceBlocked` can be
    ignored if autodev recomputes behavior from `failure_kind`.
  - Source: `lib/crates/raspberry-supervisor/src/autodev.rs`

- [x] Stop mutating the autodev report heartbeat from read-only status paths.
  - Problem: `updated_at` can look fresh even when the controller loop is not
    making progress.
  - Source: `lib/crates/raspberry-supervisor/src/autodev.rs`

- [x] Treat `ControllerLeaseError::AlreadyRunning` as non-fatal everywhere child
      programs are advanced or dispatched.
  - Problem: parent and child semantics are inconsistent.
  - Source: `lib/crates/raspberry-supervisor/src/dispatch.rs`

- [ ] Tighten daemon/resource lease truth instead of assuming lane state alone
      proves daemon health.
  - Problem: resource lease reuse is stale-state prone.
  - Source: `lib/crates/raspberry-supervisor/src/resource_lease.rs`

- [x] Remove destructive steady-state trunk sync behavior.
  - Problem: `git reset --hard origin/main` is too aggressive for a standing
    controller.
  - Source: `lib/crates/raspberry-supervisor/src/autodev.rs`

- [x] Add timeout/cleanup discipline around detached CLI stages.
  - Problem: wedged background CLIs can stall a lane indefinitely and leak
    detached processes.
  - Source: `lib/crates/fabro-workflows/src/backend/cli.rs`

- [x] Make active autodev reports explicitly represent an in-progress loop.
  - Problem: `stop_reason: CycleLimit` is misleading while the controller is
    still actively running.
  - Source: `lib/crates/raspberry-supervisor/src/autodev.rs`

- [ ] Wire actual plan-completion detection into autodev.
  - Problem: the promised trigger for post-plan review does not exist.
  - Source: `lib/crates/raspberry-supervisor/src/autodev.rs`,
    `lib/crates/raspberry-supervisor/src/plan_status.rs`

- [ ] Replace heuristic `plan-review` generation with registry-driven
      plan-review units and milestones.
  - Problem: current grouping is based on brittle `unit.id` string splitting.
  - Source: `lib/crates/fabro-synthesis/src/render.rs`

- [x] Route review-family automation back to a Codex/OpenAI-first chain instead
      of the generic Kimi-first review path.
  - Problem: current plan-review direction is not actually Codex-driven.
  - Source: `lib/crates/fabro-model/src/policy.rs`,
    `lib/crates/fabro-synthesis/src/render.rs`

- [ ] Split target-repo bug fixing from Fabro harness self-modification.
  - Problem: the current synthesized plan-review goal tells one lane to both
    fix product bugs and immediately mutate Fabro policy.
  - Source: `lib/crates/fabro-synthesis/src/render.rs`

- [ ] Add a first-class `PlanReview` workflow/template instead of reusing the
      generic implementation graph.
  - Problem: there is no true bug-finder / skeptic / arbiter flow today.
  - Source: `lib/crates/fabro-synthesis/src/render.rs`

## Execution Order

### Phase 1 — Stabilize Core Controller Truth

- [x] Prompt/runtime interpolation safety
- [x] CLI bootstrap isolation
- [x] Settled-lane run lookup reduction
- [x] Recovery-action authority
- [x] Active-report heartbeat truth
- [x] Already-running child dispatch semantics
- [x] Active report stop-state truthfulness

### Phase 2 — Harden Long-Lived Harness Behavior

- [ ] Resource lease liveness validation
- [x] Non-destructive repo sync policy
- [ ] Watchdog/lease semantics alignment

### Phase 3 — Improve Review Quality and Recursive Harness Learning

- [ ] Plan-completion detection in autodev
- [ ] Registry-driven plan-review unit generation
- [ ] Codex-first adversarial review profile
- [ ] First-class plan-review workflow family
- [ ] Report-only meta-review with explicit operator approval gate for harness
      edits

## Verification Requirements

- Targeted unit tests for:
  - prompt variable expansion behavior
  - CLI install path/locking behavior
  - stale running lane recovery
  - recovery-action override semantics
  - active report/heartbeat semantics
  - plan-review generation and routing

- Live checks:
  - `raspberry status` on `rXMRbro` returns promptly
  - `fabro run --detach` on formerly failing `tonofcrap` lanes reaches live
    agent stages without `$surface` or global npm corruption failures
  - fresh 5-agent autodev runs start in both repos on the release binaries

## Done Condition

- Both repos can sustain 5 active autodev lanes at once on the rebuilt release
  harness.
- The live controller state reflects real progress rather than stale or
  read-refreshed truth.
- The next layer of review quality work is wired through an explicit
  plan-completion → Codex review path instead of only existing as a synthesis
  note.
