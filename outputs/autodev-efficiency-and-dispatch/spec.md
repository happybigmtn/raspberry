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

## Root-Cause Findings

### Finding 1: `infer_repo_root_fallback` Does Not Recognize MALINKA Layout

**Location:** `fabro-workflows/src/workflow.rs` — `infer_repo_root_fallback()`

**Symptom:** Generated workflow graphs reference prompts as `@malinka/prompts/{family}/{slug}/plan.md`.
These references fail to resolve at runtime, causing bootstrap validation failures.

**Root cause:** The `infer_repo_root_fallback` function walks ancestor directories looking for
a FABRO package layout (root-level `prompts/`, `workflows/`, `run-configs/`). For a MALINKA
fork, the layout is at `malinka/prompts/`, `malinka/workflows/`, `malinka/run-configs/`.
The function finds neither FABRO nor MALINKA layout, returns `None`, and the fallback
chain degrades to `start_cwd + ~/.fabro`. Since `~/.fabro` is not the target repo,
prompt resolution fails silently (the unresolved `@malinka/prompts/...` string is passed
through unchanged and the prompt handler fails).

**Evidence:** The `workflow_file_fallback_dirs` function constructs the fallback chain:
```rust
// explicit dirs → inferred repo root → ~/.fabro
```
For MALINKA repos, the inferred repo root is absent, so the fallback chain is
`[target_repo, ~/.fabro]` for FABRO repos vs. `[target_repo]` for MALINKA repos
(where `target_repo` comes from `start_cwd`, which is set to `target_repo` in the
dispatch command's `current_dir`). The `start_cwd` fallback does work when CWD is
correctly set to the target repo root — but the explicit fallback is `[start_cwd]`,
not the target repo root's ancestor scan. When `base_dir` is
`malinka/workflows/{family}/` and the prompt reference is `malinka/prompts/...`,
the file resolves relative to `base_dir` first, fails, then falls back to
`[target_repo]` (from `start_cwd`). In `resolve_file_ref_with_fallbacks`, the
target repo contains `malinka/prompts/...`, so the file IS found. **This means
the `start_cwd` fallback DOES work for the MALINKA case when CWD is the target
repo root** — but the `infer_repo_root_fallback` function is still wrong because
it provides no ancestor-based repo root fallback when CWD is not the target repo.

**When it fails:** When the dispatch `current_dir` is not the target repo root (e.g.,
when `fabro run` is invoked from a subdirectory), `start_cwd` is the wrong directory
and the MALINKA layout is not detected, causing prompt resolution to fall through to
`~/.fabro` and fail.

**Decision:** Fix `infer_repo_root_fallback` to recognize both FABRO and MALINKA layouts.
FABRO layout: `prompts/`, `workflows/`, `run-configs/` at the same level.
MALINKA layout: `malinka/prompts/`, `malinka/workflows/`, `malinka/run-configs/` at
the same level. Return the ancestor for FABRO (its parent is the repo root) and the
ancestor itself for MALINKA (the ancestor is already the MALINKA package root).

### Finding 2: Stale `running` / `failed` Lane Truth Is Already Handled

**Location:** `raspberry-supervisor/src/program_state.rs` — `refresh_program_state()`

The `refresh_program_state` function already handles stale running lanes:
- If a tracked run cannot be found during state refresh → `FailureKind::TransientLaunchFailure`
- If a tracked run remains active after its worker process disappeared → same
- Stalled active progress → `FailureKind::StallWatchdog`

These cases are detected and the lane status is updated to `Failed` with the
appropriate recovery action (`BackoffRetry`). The stale running reclamation is
already implemented.

**Decision:** No code change needed. Verify through tests that the existing
`refresh_program_state` logic correctly handles the observed stale `running` case
from the 2026-03-26 live failures.

### Finding 3: Dispatch-State Telemetry Is Absent from `AutodevCycleReport`

**Location:** `raspberry-supervisor/src/autodev.rs` — `AutodevCycleReport`

The `AutodevCycleReport` struct captures dispatched outcomes and running/complete counts
but does not include:
- `idle_cycles` — cycles where no dispatch occurred because no lanes were ready
- `ready_but_undispatched` — lanes that were ready but not dispatched (should be 0
  when slots are available and budget is not exhausted)
- `stale_running_reclaimed` — lanes reclassified from `running` to `failed` in this cycle

The `AutodevCurrentSnapshot` does not include `dispatch_rate` or `idle_cycle_count`.

**Decision:** Add these fields to the respective structs and populate them in the
orchestration loop.

### Finding 4: Synth Command Is Properly Exposed in CLI

**Location:** `fabro-cli/src/main.rs` and `fabro-cli/src/commands/synth.rs`

The `synth` subcommand (including `import`, `create`, `evolve`, `review`, `genesis`)
is properly registered in the CLI. No evidence of a missing command entrypoint was
found in the current codebase. The 2026-03-26 failure where "the clean `fabro` binary
did not expose `synth`" is likely a stale binary or a feature-flag issue, not a
structural CLI routing problem.

**Decision:** No structural change needed. Verify that the `synth` command is included
in the release binary build and add a bootstrap test that confirms `fabro synth --help`
succeeds.

## Runtime Contract

### Execution Path

1. `raspberry autodev` loads the program manifest and evaluates lane readiness.
2. For each ready lane, `dispatch.rs` calls `fabro run --detach {run_config}` with
   `current_dir = target_repo`.
3. `fabro run` resolves the graph path relative to the run config's directory
   (`malinka/workflows/{family}/{slug}.fabro`).
4. `WorkflowBuilder::prepare_with_file_inlining_and_fallbacks` is called with:
   - `base_dir = malinka/workflows/{family}/`
   - `fallback_dirs = [target_repo, ~/.fabro]` (from `workflow_file_fallback_dirs`)
5. `FileInliningTransform` resolves `@malinka/prompts/...` references:
   - Try `base_dir/malinka/prompts/...` → exists, resolves correctly
   - If not: try `fallback_dirs[0]/malinka/prompts/...` → target_repo, resolves correctly

The critical invariant: `target_repo` must be in the fallback chain. This is guaranteed
when `infer_repo_root_fallback` returns `target_repo` (FABRO layout) or when
`start_cwd` is `target_repo` (dispatch sets `current_dir = target_repo`).

### MALINKA Layout Fix

The fix to `infer_repo_root_fallback` adds MALINKA layout detection:

```rust
fn infer_repo_root_fallback(base_dir: &Path) -> Option<PathBuf> {
    for ancestor in base_dir.ancestors() {
        // FABRO layout: prompts/, workflows/, run-configs/ at root
        // The ancestor's parent is the repo root
        let has_fabro_layout = ancestor.join("prompts").is_dir()
            && ancestor.join("workflows").is_dir()
            && ancestor.join("run-configs").is_dir();
        if has_fabro_layout {
            return ancestor.parent().map(PathBuf::from);
        }

        // MALINKA layout: malinka/prompts/, malinka/workflows/, malinka/run-configs/
        // The ancestor ITSELF is the MALINKA package root
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

This change makes `infer_repo_root_fallback` return `target_repo` for MALINKA repos
without relying on the `start_cwd` fallback, making the runtime path robust regardless
of the dispatch CWD.

## Acceptance Criteria

| # | Criterion | Evidence |
|---|-----------|----------|
| 1 | `infer_repo_root_fallback` returns `target_repo` for both FABRO and MALINKA layouts | New unit test |
| 2 | Generated MALINKA workflows resolve prompts correctly without `~/.fabro/prompts` | Integration test |
| 3 | Stale `running` lanes are reclassified to `failed` within one refresh cycle | `cargo nextest run -p raspberry-supervisor -- program_state` |
| 4 | `AutodevCycleReport` includes `idle_cycles`, `ready_but_undispatched`, `stale_running_reclaimed` | Code inspection + test |
| 5 | `AutodevCurrentSnapshot` includes `dispatch_rate` | Code inspection |
| 6 | `fabro synth --help` succeeds in release binary | `cargo build --release -p fabro-cli && target-local/release/fabro synth --help` |
| 7 | Live: rXMRbro sustains 10 active lanes for 20 cycles | `raspberry autodev --max-parallel 10 --max-cycles 20` |
| 8 | Live: zero bootstrap validation failures from prompt resolution or missing commands | Autodev cycle report |
| 9 | Live: at least 3 lanes land to trunk after execution path is stable | `git log --oneline` on rXMRbro |

## Failure Modes and Mitigations

| Failure Mode | Detection | Mitigation |
|---|---|---|
| Prompt ref resolves to `~/.fabro` instead of target repo | `runtime_path_errors` telemetry field | Fix `infer_repo_root_fallback` |
| Stale `running` lane consumes dispatch slot | `stale_running_reclaimed` telemetry field | Existing `refresh_program_state` logic |
| `synth evolve` blocks dispatch cycle | Cycle timing in `AutodevCycleReport` | Already non-fatal (timeout skips cycle) |
| Worker thread panics during dispatch | `DispatchError::WorkerPanicked` | Join all threads before processing results (already done) |
| Target repo diverged from origin | `TargetRepoFreshness` enum | Auto-heal or block dispatch |

## Non-Goals

- Reducing dispatch latency or increasing dispatch rate (Plan 006 handles review quality;
  dispatch optimization follows only after execution path is stable)
- Adding new synthesis commands or changing the synthesis data model
- Modifying the Paperclip dashboard or coordination surface
- Changing the lane readiness evaluation logic (that is Plan 010)
