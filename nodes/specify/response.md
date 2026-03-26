Both durable artifacts are in place. Here is a summary of the findings and decisions documented in each:

---

## `outputs/autodev-efficiency-and-dispatch/spec.md` (198 lines)

The durable spec captures **four root-cause findings** from the 2026-03-26 live evidence:

1. **`infer_repo_root_fallback` does not recognize MALINKA layout** — The function in `fabro-workflows/src/workflow.rs` only detects FABRO package layout (root-level `prompts/`, `workflows/`, `run-configs/`). For MALINKA forks these are at `malinka/prompts/`, `malinka/workflows/`, `malinka/run-configs/`, which are invisible. The fallback chain degrades to `start_cwd + ~/.fabro`, and `~/.fabro` is not the target repo, causing prompt resolution to fail silently. The fix adds MALINKA layout detection alongside the existing FABRO check.

2. **Synth command is properly exposed** — The CLI registration in `main.rs` and `commands/synth.rs` is structurally sound. The observed "synth not found" failure is a stale binary issue, not a routing problem. A bootstrap test is recommended but no structural change is needed.

3. **Stale `running`/`failed` lane truth is already handled** — `refresh_program_state` in `program_state.rs` already reclassifies dead running lanes (`FailureKind::TransientLaunchFailure`) and stalled lanes (`FailureKind::StallWatchdog`). No code change needed; a unit test is recommended.

4. **Dispatch-state telemetry is absent** — `AutodevCycleReport` and `AutodevCurrentSnapshot` are missing `idle_cycle`, `ready_but_undispatched`, `stale_running_reclaimed`, and `dispatch_rate` fields. These are additive and backward-compatible.

The spec includes a complete runtime contract explaining the execution path from `raspberry autodev` dispatch through `FileInliningTransform` prompt resolution, plus nine acceptance criteria with evidence types.

---

## `outputs/autodev-efficiency-and-dispatch/review.md` (253 lines)

The first-review document provides:

- **Finding 1a**: `synth` command exposure — not reproducible; bootstrap test recommended
- **Finding 1b**: `infer_repo_root_fallback` — root cause confirmed, fix is well-scoped to one function
- **Task 2**: Stale lane truth — already implemented; unit test recommended
- **Task 3**: Telemetry — three new fields identified with derivability notes
- **Task 4**: Live validation — correctly gated behind Tasks 1 and 3
- **Risk table**: Four risks assessed with likelihood, impact, and mitigations
- **Six recommendations**: Prioritized implementation order (fix `infer_repo_root_fallback` first, add MALINKA test, add stale-lane test, keep `AutodevCycleReport` additive, measure cycle time before reordering evolve)
- **Three open questions**: `start_cwd` reliability, `refresh_program_state` timing, `dispatch_rate` formula