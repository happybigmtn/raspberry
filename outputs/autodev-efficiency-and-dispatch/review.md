# Autodev Efficiency and Dispatch Truth — First-Review Slice Review

Lane: `autodev-efficiency-and-dispatch`
Date: 2026-03-26
Reviewer: pi coding agent (first honest slice)
Status: **Reviewed — ready for implementation**

---

## Review Summary

This review covers the first slice of the `autodev-efficiency-and-dispatch` lane.
The review assessed four task areas against the current codebase and the findings
documented in `genesis/plans/003-autodev-efficiency-and-dispatch.md`.

**Overall verdict:** Three of four task areas have clear, single-root-cause diagnoses
backed by source evidence. One task area (synth command exposure) is not reproducible
in the current codebase. One task area (stale lane truth) is already handled by
existing code. The remaining work is well-scoped and ready for implementation.

---

## Task 1: Eliminate Local-Only Command and Prompt-Resolution Shims

**Status: Root cause identified. Fix is well-scoped.**

### Review Finding

The plan documents two classes of failure:
1. The `fabro` binary did not expose `synth` at runtime
2. Generated workflow graphs referenced prompts as `@../../prompts/...` which
   resolved under `~/.fabro/` at runtime instead of the target repo

**Finding 1a (synth command):** The current codebase (`fabro-cli/src/main.rs`) properly
registers the `synth` subcommand group including `import`, `create`, `evolve`, `review`,
and `genesis` variants. The `mod.rs` exports `pub mod synth;`. The command routing
dispatches to `commands::synth::*` for all variants. There is no evidence of a
feature-flag or conditional compilation that would suppress this in release builds.

The 2026-03-26 failure ("clean `fabro` binary did not expose `synth`") is likely
a stale binary artifact or a build configuration issue, not a structural CLI routing
problem. **No code change is required for this sub-issue.**

**Recommendation:** Add a bootstrap test to `raspberry-supervisor` that confirms
`fabro synth --help` exits 0, and document the expected build target (`--release`)
in the test so this class of failure is caught by CI.

**Finding 1b (prompt resolution):** The root cause of prompt resolution failure is
a gap in `infer_repo_root_fallback` (`fabro-workflows/src/workflow.rs:147-157`).
This function is called from `workflow_file_fallback_dirs` to build the fallback chain
for `@file` reference resolution in `FileInliningTransform`.

The function walks ancestor directories of `base_dir` (the directory containing the
graph file) looking for a FABRO package layout (root-level `prompts/`, `workflows/`,
`run-configs/`). For a MALINKA fork, these directories are at `malinka/prompts/`,
`malinka/workflows/`, `malinka/run-configs/`, which are not recognized. The function
returns `None`, and the fallback chain degrades.

The mitigating factor is that `workflow_file_fallback_dirs` also includes `start_cwd`
as an explicit fallback. In the dispatch path (`dispatch.rs::run_fabro`), the command
is executed with `current_dir = target_repo`, so `start_cwd = target_repo` and the
MALINKA prompts directory IS found in the fallback chain. **However**, this depends
on the dispatch CWD being the target repo root, which is not guaranteed in all
execution environments or test scenarios.

**The fix is correct and well-scoped:** Extend `infer_repo_root_fallback` to also
detect MALINKA layout and return the ancestor (not its parent, since the ancestor
is already the MALINKA package root). This makes the fallback chain reliable regardless
of dispatch CWD.

**Implementation note:** The existing test
`prepare_with_file_inlining_infers_repo_root_fallback_for_repo_relative_prompts`
uses the FABRO layout. A new test using the MALINKA layout should be added.

### Evidence

```rust
// fabro-workflows/src/workflow.rs:147-157
fn infer_repo_root_fallback(base_dir: &Path) -> Option<std::path::PathBuf> {
    for ancestor in base_dir.ancestors() {
        let has_package_layout = ancestor.join("prompts").is_dir()
            && ancestor.join("workflows").is_dir()
            && ancestor.join("run-configs").is_dir();
        if has_package_layout {
            return ancestor.parent().map(std::path::Path::to_path_buf);
        }
    }
    None  // ← MALINKA layout not recognized
}
```

```rust
// fabro-workflows/src/workflow.rs:127-145
fn workflow_file_fallback_dirs(base_dir: &Path, explicit: &[&Path]) -> Vec<PathBuf> {
    // explicit (start_cwd) → inferred repo root → ~/.fabro
    // For MALINKA: inferred = None, chain = [start_cwd, ~/.fabro]
    // start_cwd works when CWD=target_repo, fails otherwise
}
```

---

## Task 2: Fix Stale `running` and `failed` Lane Truth Before Dispatch

**Status: Already implemented. Verification needed.**

### Review Finding

The `refresh_program_state` function (`raspberry-supervisor/src/program_state.rs`) is
called by the autodev evaluation cycle and already handles the stale `running` lane
case. Specifically:

1. **Tracked run not found:** When `read_live_lane_progress_for_run_id` returns `None`
   for a lane with `status == Running`, the record is updated to `Failed` with
   `failure_kind = FailureKind::TransientLaunchFailure` and `recovery_action =
   FailureRecoveryAction::BackoffRetry`. This is the exact case observed during
   the 2026-03-26 restart work.

2. **Worker process disappeared:** The `stale_active_progress` check detects when
   a tracked run has been active without worker activity for `STALE_RUNNING_GRACE_SECS`
   (30 seconds). The lane is reclassified to `Failed`.

3. **Stall watchdog:** The `stalled_active_progress_reason` check detects when a
   lane has been running beyond `ACTIVE_STALL_TIMEOUT_SECS` (1800 seconds) with no
   stage completion. The lane is reclassified with `FailureKind::StallWatchdog`.

**No code change is required.** The stale running reclamation is already present
and correct. The gap is test coverage: there are no unit tests that specifically
exercise the stale running lane path in `refresh_program_state`.

**Recommendation:** Add a unit test to `raspberry-supervisor` that:
- Creates a `ProgramRuntimeState` with a lane marked `Running` but with no
  corresponding `progress.jsonl` or `state.json` in the expected run directory
- Calls `refresh_program_state`
- Asserts the lane transitions to `Failed` with `FailureKind::TransientLaunchFailure`

This test should live in the `program_state` module and be discoverable by
`cargo nextest run -p raspberry-supervisor -- program_state`.

### Evidence

```rust
// program_state.rs — tracked run not found case:
let Some(mut progress) = progress else {
    if record.status == LaneExecutionStatus::Running {
        // ... mark Failed, TransientLaunchFailure, BackoffRetry
    }
};

// stale_active_progress check:
if stale_active_progress(&progress, record.last_started_at) {
    // mark Failed, worker disappeared, BackoffRetry
}
```

---

## Task 3: Add Dispatch-State Telemetry

**Status: Structurally identified. Three new fields needed.**

### Review Finding

The `AutodevCycleReport` struct captures:
- `dispatched: Vec<DispatchOutcome>`
- `running_after: usize`
- `complete_after: usize`
- `ready_lanes: Vec<String>`
- `replayed_lanes: Vec<String>`
- `regenerate_noop_lanes: Vec<String>`

Missing fields for operator-language dispatch telemetry:
1. **`idle_cycle: bool`** — true when no dispatch occurred because no lanes were ready
2. **`ready_but_undispatched: usize`** — lanes that were ready but not dispatched this
   cycle (should be 0 when `available_slots > 0` and lanes are available)
3. **`stale_running_reclaimed: usize`** — lanes reclassified from `running` to `failed`
   in this cycle

The `AutodevCurrentSnapshot` struct should include:
- **`dispatch_rate: f64`** — fraction of cycles in which dispatch occurred, or
  rolling mean dispatch rate over the session

These fields are derivable from existing data in the orchestration loop but are
not currently persisted to the report.

**The fix is additive and backward-compatible.** All new fields use
`#[serde(default, skip_serializing_if = ...)]` to avoid breaking existing report consumers.

---

## Task 4: Live Validation

**Status: Pending implementation. Prerequisites must land first.**

Live validation (10 active lanes on rXMRbro for 20 cycles, zero bootstrap failures,
3 lanes to trunk) depends on the fixes from Tasks 1 and 3 landing first. The
`infer_repo_root_fallback` fix is the blocking prerequisite.

The validation protocol:
1. Build release binaries: `cargo build --release -p fabro-cli -p raspberry-cli`
2. Run autodev: `target-local/release/raspberry autodev --manifest ... --max-parallel 10 --max-cycles 20`
3. Inspect report: Check `failed: 0` for newly dispatched lanes and `running: 10` sustained

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `infer_repo_root_fallback` change breaks FABRO layout detection | Low | High | Add both FABRO and MALINKA checks; existing tests must pass |
| Adding telemetry fields breaks JSON report consumers | Low | Medium | Use `skip_serializing_if` defaults; version the schema if needed |
| Stale lane truth test is flaky (timing-dependent) | Medium | Low | Mock the filesystem in the unit test rather than relying on wall clock |
| `synth` command not in release binary | Low | High | Add bootstrap test that verifies `--help` succeeds |

---

## Recommendations

1. **Implement `infer_repo_root_fallback` fix first** — it unblocks both the
   prompt resolution fix and the live validation gate.
2. **Add both FABRO and MALINKA layout checks** in the same function, with
   a comment explaining why the return value differs between layouts (FABRO returns
   parent; MALINKA returns the ancestor itself).
3. **Add a new test** `prepare_with_file_inlining_infers_malinka_root_fallback`
   to `fabro-workflows/src/workflow.rs` that exercises the MALINKA path.
4. **Add a stale lane test** to `raspberry-supervisor/src/program_state.rs`
   as described above.
5. **Do not change the dispatch cycle ordering** (evolve before dispatch) without
   first measuring the actual cycle time impact. The existing timeout handling
   already prevents evolve from permanently blocking dispatch.
6. **Keep `AutodevCycleReport` additive** — do not remove or rename existing
   fields; only add new ones.

---

## Open Questions

1. **Is `start_cwd` always `target_repo` in the dispatch path?** The `run_fabro`
   function sets `current_dir = target_repo` via `Command::new(fabro_bin).current_dir(target_repo)`.
   This is the correct behavior and is the primary fallback for MALINKA prompt
   resolution today. The `infer_repo_root_fallback` fix is still correct and
   valuable as a defense-in-depth measure.

2. **Should `refresh_program_state` be called before or after `evaluate_program`?**
   Currently `refresh_program_state` is called inside `evaluate_program_internal` via
   `sync_program_state_with_evaluated`. This means the autodev cycle's
   `refresh → evaluate → evolve → dispatch → watch → update` sequence already
   includes a state refresh. The stale running detection is on the hot path.

3. **What is the `dispatch_rate` formula?** Suggested: `dispatched_lanes /
   max_parallel` per cycle, averaged over the session. Alternatively: fraction
   of cycles in which at least one lane was dispatched. The operator language
   suggests the latter ("idle cycles" framing).
