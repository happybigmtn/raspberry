Both artifacts are verified against the codebase. Here's a summary of what was produced:

---

## Deliverables

### `outputs/autodev-efficiency-and-dispatch/spec.md` (388 lines)

**Executable specification** covering 5 requirements with:
- **R1.1** — Verified: `fabro-synthesis` is unconditionally compiled in `fabro-cli/Cargo.toml`. No action needed.
- **R1.2** — Needs investigation: prompt reference resolution context in `fabro-graphviz`. May already be correct; needs a specific test.
- **R2.1/2.2** — Verified: `stale_active_progress()` and `stalled_active_progress_reason()` in `program_state.rs` implement the correct logic. Grace period is 30s, stall watchdog is 1800s.
- **R2.3** — **Bug found**: `run_fabro()` returns `DispatchError::MissingRunConfig` before dispatching, but the caller in `execute_selected_lanes` doesn't update lane state — leaving stale `Running` slots.
- **R3.1/3.2** — Not implemented: `AutodevCycleReport` lacks `DispatchState`. No aggregate counters (`idle_cycles`, `dispatch_rate`, etc.).
- **R4.1** — **Blocking**: `run_synth_evolve()` is synchronous with 120s timeout. When triggered on cycle 1 of a fresh start, blocks dispatch for up to 2 minutes.
- **R4.2** — Verified: `select_ready_lanes_for_dispatch()` already fills the budget correctly. Blocked by R2.3 (stale slots) and R4.1 (blocking evolve).

Includes full `DispatchState` and `AutodevCurrentSnapshot` schema additions, a 6-step implementation order, and a decision log.

---

### `outputs/autodev-efficiency-and-dispatch/review.md` (329 lines)

**Critical review** with:
- Requirement-by-requirement verdicts with exact file/line citations
- Gap analysis for `fabro-graphviz` include resolution context
- macOS `worker_process_alive()` bug noted (out of scope but filed)
- 9 missing tests enumerated with descriptions
- Recommended implementation sequence: R2.3 → R3.1 → R3.2 → R4.1 → R1.2 → live validation
- Risk matrix for background evolve threading