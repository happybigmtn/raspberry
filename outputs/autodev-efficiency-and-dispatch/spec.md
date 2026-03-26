# Autodev Efficiency and Dispatch Truth — Spec

## Status

Draft for first-review slice. Captures the durable spec for autodev execution-path
consistency and dispatch-state telemetry. Implementation is pending; this document
is the reviewed contract that code must satisfy.

## Purpose / User-Visible Outcome

After this work lands, `raspberry autodev` sustains 10 active lanes on a proving-ground
repo (rXMRbro) without any local-only shims: no prompt symlinks, no debug-only command
surface, no manual state scrubbing. Lanes that are genuinely dead stop claiming worker
slots, and the operator can see why ready work did or did not run from the autodev report.

## Whole-System Goal

Autodev dispatch is boringly reliable. The generated package and the runtime agree about
where commands and assets live. Stale `running`/`failed` lane truth is reconciled before
each dispatch cycle. Dispatch telemetry explains every decision in operator language.

---

## Root-Cause Findings

### Finding 1: `infer_repo_root_fallback` Does Not Recognize MALINKA Layout

**File:** `lib/crates/fabro-workflows/src/workflow.rs`
**Function:** `infer_repo_root_fallback` (line 147)

**Symptom:** Generated workflow graphs reference prompts as `@malinka/prompts/{family}/{slug}/plan.md`.
These references fail to resolve at runtime when the dispatch CWD is not the target repo root,
causing bootstrap validation failures.

**Root cause:** `infer_repo_root_fallback` walks ancestor directories looking for a FABRO
package layout (root-level `prompts/`, `workflows/`, `run-configs/`). For a MALINKA fork,
these directories are at `malinka/prompts/`, `malinka/workflows/`, `malinka/run-configs/`.
The function finds neither layout, returns `None`, and the fallback chain degrades to
`start_cwd + ~/.fabro`.

The **mitigating factor** is that `workflow_file_fallback_dirs` (line 127) also adds
`start_cwd` as an explicit fallback. In the dispatch path, `run_fabro` in
`dispatch.rs` sets `current_dir = target_repo` on the `fabro run` command, so
`start_cwd = target_repo` and MALINKA prompts are found. **However**, this depends on
the dispatch CWD being the target repo root — it breaks in any subdirectory invocation.

**Fix:** Extend `infer_repo_root_fallback` to also detect MALINKA layout. For FABRO,
return the ancestor's parent (the repo root). For MALINKA, return the ancestor itself
(the MALINKA package root, which is the repo root for a MALINKA fork).

```rust
fn infer_repo_root_fallback(base_dir: &Path) -> Option<PathBuf> {
    for ancestor in base_dir.ancestors() {
        // FABRO layout: prompts/, workflows/, run-configs/ at repo root
        // The ancestor's parent is the repo root
        let has_fabro_layout = ancestor.join("prompts").is_dir()
            && ancestor.join("workflows").is_dir()
            && ancestor.join("run-configs").is_dir();
        if has_fabro_layout {
            return ancestor.parent().map(PathBuf::from);
        }

        // MALINKA layout: malinka/prompts/, malinka/workflows/, malinka/run-configs/
        // The ancestor itself is the MALINKA package root (= repo root for MALINKA forks)
        let has_malinka_layout = ancestor.join("malinka/prompts").is_dir()
            && ancestor.join("malinka/workflows").is_dir()
            && ancestor.join("malinka/run-configs").is_dir();
        if has_malinka_layout {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}
```

### Finding 2: Stale `running` / `failed` Lane Truth Is Already Handled

**File:** `lib/crates/raspberry-supervisor/src/program_state.rs`
**Function:** `refresh_program_state` (line 383)

The `refresh_program_state` function already handles stale running lanes. The existing test
`refresh_program_state_marks_missing_running_run_as_stale_failure` (line 1913) covers the
case where a lane is marked `Running` but the corresponding run directory does not exist:
the lane is reclassified to `Failed` with `FailureKind::TransientLaunchFailure` and
`recovery_action = FailureRecoveryAction::BackoffRetry`.

Additional stale-running cases handled:
- Worker process disappeared: `stale_active_progress` check with `STALE_RUNNING_GRACE_SECS`
  (30 s) → `Failed` / `BackoffRetry`
- Stall watchdog: `stalled_active_progress_reason` check with `ACTIVE_STALL_TIMEOUT_SECS`
  (1800 s) → `Failed` / `StallWatchdog`

**Decision:** No code change needed. Confirm test coverage is sufficient via the existing
test suite.

### Finding 3: Dispatch-State Telemetry Is Absent from `AutodevCycleReport`

**File:** `lib/crates/raspberry-supervisor/src/autodev.rs`
**Struct:** `AutodevCycleReport` (line 96)

The `AutodevCycleReport` struct captures dispatched outcomes and running/complete counts
but omits fields needed for operator-language dispatch diagnostics:

| Missing field | Type | Purpose |
|---|---|---|
| `idle_cycle` | `bool` | `true` when no dispatch occurred because no lanes were ready |
| `ready_but_undispatched` | `usize` | Ready lanes not dispatched this cycle (should be 0 when slots available) |
| `stale_running_reclaimed` | `usize` | Lanes reclassified `running → failed` in this cycle |

`AutodevCurrentSnapshot` (line 70) also lacks:
| Missing field | Type | Purpose |
|---|---|---|
| `dispatch_rate` | `f64` | Fraction of cycles in which at least one lane was dispatched |

All new fields use `#[serde(default, skip_serializing_if = "...")]` so the change is
backward-compatible with existing report consumers.

### Finding 4: `synth` Command Is Properly Exposed in CLI

**File:** `lib/crates/fabro-cli/src/main.rs`
**Module:** `commands::synth` (registered line 183; dispatched lines 1028–1041)

The `synth` subcommand group (`import`, `create`, `evolve`, `review`, `genesis`) is
properly registered in the CLI. No feature-flag or conditional compilation suppresses it
in release builds. The 2026-03-26 failure ("clean `fabro` binary did not expose `synth`")
is a stale binary artifact, not a structural routing problem.

**Decision:** No structural change needed. Add a bootstrap test that confirms
`fabro synth --help` exits 0 in release builds.

---

## Runtime Contract

### Execution Path

```
raspberry autodev
  └─ evaluate_program
       └─ dispatch (dispatch.rs :: run_fabro)
            ├─ current_dir = target_repo          ← explicit CWD set here
            └─ fabro run --detach {run_config}
                 └─ WorkflowBuilder::prepare_with_file_inlining_and_fallbacks
                      ├─ base_dir = malinka/workflows/{family}/
                      └─ fallback_dirs = [target_repo, ~/.fabro]
                           └─ FileInliningTransform resolves @malinka/prompts/...
```

Critical invariant: `target_repo` must be in the fallback chain. The `infer_repo_root_fallback`
fix ensures this for both FABRO and MALINKA layouts without depending on dispatch CWD.

### MALINKA Layout Detection

The fix adds MALINKA layout detection alongside the existing FABRO detection in
`infer_repo_root_fallback`. For MALINKA repos, the function returns the ancestor itself
(not its parent, since the ancestor IS the MALINKA package root). This makes the runtime
path reliable regardless of where `fabro run` is invoked from.

---

## Acceptance Criteria

| # | Criterion | Evidence |
|---|-----------|----------|
| 1 | `infer_repo_root_fallback` returns `target_repo` for both FABRO and MALINKA layouts | New unit test `prepare_with_file_inlining_infers_malinka_root_fallback` |
| 2 | Generated MALINKA workflows resolve prompts without `~/.fabro/prompts` fallback | Integration test with MALINKA fixture |
| 3 | Stale `running` lanes reclassified to `failed` within one refresh cycle | `cargo nextest run -p raspberry-supervisor -- refresh_program_state_marks_missing_running_run_as_stale_failure` |
| 4 | `AutodevCycleReport` has `idle_cycle`, `ready_but_undispatched`, `stale_running_reclaimed` | Code inspection + unit test |
| 5 | `AutodevCurrentSnapshot` has `dispatch_rate` | Code inspection |
| 6 | `fabro synth --help` succeeds in release binary | Bootstrap test in CI |
| 7 | Live: rXMRbro sustains 10 active lanes for 20 cycles | `raspberry autodev --max-parallel 10 --max-cycles 20` |
| 8 | Live: zero bootstrap validation failures from prompt resolution | Autodev cycle report `failed: 0` for newly dispatched lanes |
| 9 | Live: at least 3 lanes land to trunk after stable execution path | `git log --oneline` on rXMRbro |

---

## Failure Modes and Mitigations

| Failure Mode | Detection | Mitigation |
|---|---|---|
| Prompt ref resolves to `~/.fabro` instead of target repo | `runtime_path_errors` telemetry | Fix `infer_repo_root_fallback` |
| Stale `running` lane consumes dispatch slot | `stale_running_reclaimed` in report | Existing `refresh_program_state` logic |
| `synth evolve` blocks dispatch cycle | Cycle timing in `AutodevCycleReport` | Non-fatal; timeout skips cycle |
| Worker thread panics during dispatch | `DispatchError::WorkerPanicked` | Join all threads before processing results |
| Target repo diverged from origin | `TargetRepoFreshness` enum | Auto-heal or block dispatch |

---

## Non-Goals

- Reducing dispatch latency or increasing dispatch rate (Plan 006 handles review quality;
  dispatch optimization follows only after execution path is stable)
- Adding new synthesis commands or changing the synthesis data model
- Modifying the Paperclip dashboard or coordination surface
- Changing lane readiness evaluation logic (that is Plan 010)
