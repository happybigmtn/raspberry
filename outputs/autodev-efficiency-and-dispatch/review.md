# Autodev Efficiency and Dispatch Truth — First-Review Slice Review

Lane: `autodev-efficiency-and-dispatch`
Date: 2026-03-26
Reviewer: pi coding agent (first honest slice)
Status: **Reviewed — ready for implementation**

---

## Review Summary

This review assessed four task areas against the current codebase and the findings
documented in `genesis/plans/003-autodev-efficiency-and-dispatch.md`.

**Overall verdict:** Three of four task areas have clear, single-root-cause diagnoses
backed by source evidence. One task area (synth command exposure) is not reproducible
in the current codebase. One task area (stale lane truth) is already implemented with
test coverage. The remaining work is well-scoped and ready for implementation.

---

## Task 1: Eliminate Local-Only Command and Prompt-Resolution Shims

**Status: Root cause identified. Fix is well-scoped.**

### 1a — `synth` command not exposed

The current codebase (`lib/crates/fabro-cli/src/main.rs`) properly registers the `synth`
subcommand group including `import`, `create`, `evolve`, `review`, and `genesis` variants.
The module is exported at line 28 of `commands/mod.rs` as `pub mod synth;`. Command routing
dispatches at lines 1028–1041. There is no feature-flag or conditional compilation that
would suppress this in release builds.

The 2026-03-26 failure ("clean `fabro` binary did not expose `synth`") is a stale
binary artifact or build configuration issue, not a structural CLI routing problem.

**No code change required for this sub-issue.** A bootstrap test that confirms
`fabro synth --help` exits 0 in release builds should be added to prevent regression.

### 1b — Prompt resolution for MALINKA layout

**Root cause location:** `lib/crates/fabro-workflows/src/workflow.rs`, function
`infer_repo_root_fallback` (line 147).

The function walks ancestor directories of `base_dir` looking for a FABRO package layout
(root-level `prompts/`, `workflows/`, `run-configs/`). For a MALINKA fork, these directories
are at `malinka/prompts/`, `malinka/workflows/`, `malinka/run-configs/` — not recognized.
The function returns `None`, and the fallback chain degrades.

The **mitigating factor** is that `workflow_file_fallback_dirs` (line 127) also adds
`start_cwd` as an explicit fallback. In the dispatch path, `run_fabro` in `dispatch.rs`
sets `current_dir = target_repo` on the `fabro run` command, so `start_cwd = target_repo`
and MALINKA prompts are found. This works when dispatch CWD is the target repo root.

**The fix is correct and well-scoped:** Extend `infer_repo_root_fallback` to also detect
MALINKA layout, returning the ancestor itself (not its parent, since the ancestor IS
the MALINKA package root for a MALINKA fork). This makes the fallback reliable regardless
of dispatch CWD.

The existing test `prepare_with_file_inlining_infers_repo_root_fallback_for_repo_relative_prompts`
uses the FABRO layout. A new test `prepare_with_file_inlining_infers_malinka_root_fallback`
should be added using a MALINKA fixture.

### Evidence

```rust
// fabro-workflows/src/workflow.rs:147-157 (current, FABRO-only)
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
// fabro-workflows/src/workflow.rs:127-145 (fallback chain construction)
fn workflow_file_fallback_dirs(base_dir: &Path, explicit: &[&Path]) -> Vec<PathBuf> {
    // explicit (start_cwd) → inferred repo root → ~/.fabro
    // For MALINKA: inferred = None, chain = [start_cwd, ~/.fabro]
    // start_cwd works when CWD=target_repo, breaks otherwise
}
```

---

## Task 2: Fix Stale `running` / `failed` Lane Truth Before Dispatch

**Status: Already implemented. Test coverage confirmed.**

The `refresh_program_state` function (`lib/crates/raspberry-supervisor/src/program_state.rs`,
line 383) already handles stale `running` lanes:

1. **Tracked run not found:** When `read_live_lane_progress_for_run_id` returns `None` for a
   lane with `status == Running`, the record is updated to `Failed` with
   `failure_kind = FailureKind::TransientLaunchFailure` and
   `recovery_action = FailureRecoveryAction::BackoffRetry`.

2. **Worker process disappeared:** `stale_active_progress` check with
   `STALE_RUNNING_GRACE_SECS` (30 s) → `Failed` / `BackoffRetry`.

3. **Stall watchdog:** `stalled_active_progress_reason` check with
   `ACTIVE_STALL_TIMEOUT_SECS` (1800 s) → `Failed` / `StallWatchdog`.

Test coverage exists: `refresh_program_state_marks_missing_running_run_as_stale_failure`
(line 1913) exercises the "tracked run not found" path directly.

**No code change required.** The gap is test coverage for the other two stale-running paths
(worker disappearance and stall watchdog), which should be added as follow-on work.

---

## Task 3: Add Dispatch-State Telemetry

**Status: Structurally identified. Three new fields needed in `AutodevCycleReport`.**

The `AutodevCycleReport` struct (`lib/crates/raspberry-supervisor/src/autodev.rs`, line 96)
captures dispatched outcomes and running/complete counts. Missing for operator-language
dispatch diagnostics:

| New field | Type | Derivation |
|---|---|---|
| `idle_cycle: bool` | `bool` | `true` when `dispatched` is empty and `ready_lanes` is empty |
| `ready_but_undispatched: usize` | `usize` | `ready_lanes.len() - dispatched.len()` when slots were available |
| `stale_running_reclaimed: usize` | `usize` | Count of lanes reclassified `running → failed` in this cycle |

`AutodevCurrentSnapshot` (line 70) needs:
| New field | Type | Derivation |
|---|---|---|
| `dispatch_rate: f64` | `f64` | Fraction of cycles with at least one dispatch |

All additions use `#[serde(default, skip_serializing_if = "...")]` for backward compatibility.

**The fix is additive and backward-compatible.**

---

## Task 4: Live Validation

**Status: Pending implementation. Prerequisites from Tasks 1 and 3 must land first.**

Live validation (10 active lanes on rXMRbro for 20 cycles, zero bootstrap failures,
3 lanes to trunk) is blocked on the `infer_repo_root_fallback` fix and the telemetry
additions. The `refresh_program_state` logic (Task 2) is already in place.

Validation protocol:
1. Build release binaries: `cargo build --release -p fabro-cli -p fabro-workflows -p raspberry-cli`
2. Run autodev: `target-local/release/raspberry autodev --manifest ... --max-parallel 10 --max-cycles 20`
3. Inspect report: `failed: 0` for newly dispatched lanes; `running: 10` sustained

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `infer_repo_root_fallback` change breaks FABRO layout detection | Low | High | Add both FABRO and MALINKA checks; existing tests must pass |
| Adding telemetry fields breaks JSON report consumers | Low | Medium | Use `skip_serializing_if` defaults |
| Stale lane truth test is flaky (timing-dependent) | Medium | Low | Mock filesystem rather than relying on wall clock |
| `synth` command not in release binary | Low | High | Bootstrap test verifies `--help` succeeds |

---

## Recommendations (Priority Order)

1. **Implement `infer_repo_root_fallback` fix first** — it unblocks prompt resolution
   for MALINKA and the live validation gate.
2. **Add both FABRO and MALINKA layout checks** in the same function, with a comment
   explaining why the return value differs (FABRO → parent; MALINKA → ancestor itself).
3. **Add test** `prepare_with_file_inlining_infers_malinka_root_fallback` to
   `lib/crates/fabro-workflows/src/workflow.rs` with a MALINKA fixture.
4. **Do not change the dispatch cycle ordering** (evolve before dispatch) without first
   measuring actual cycle time. The existing timeout handling already prevents evolve
   from permanently blocking dispatch.
5. **Keep `AutodevCycleReport` additive** — do not remove or rename existing fields.
6. **Add bootstrap test** for `fabro synth --help` in release builds.

---

## Open Questions

1. **Is `start_cwd` always `target_repo` in the dispatch path?** `run_fabro` sets
   `current_dir = target_repo` via `Command::new(fabro_bin).current_dir(target_repo)`.
   This is the correct behavior and is the primary fallback for MALINKA prompt resolution
   today. The `infer_repo_root_fallback` fix is defense-in-depth.

2. **Should `refresh_program_state` be called before or after `evaluate_program`?**
   Currently called inside `evaluate_program_internal` via `sync_program_state_with_evaluated`.
   The autodev cycle's `refresh → evaluate → evolve → dispatch → watch → update` sequence
   already includes a state refresh. The stale running detection is on the hot path.

3. **What is the `dispatch_rate` formula?** Suggested: fraction of cycles in which at
   least one lane was dispatched. This maps directly to the "idle cycles" framing in the
   report.
