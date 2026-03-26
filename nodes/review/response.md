Based on my review of the spec, review, and source code verification, here is my assessment for the `autodev-efficiency-and-dispatch` lane.

---

# Autodev Efficiency and Dispatch Truth — Lane Review

**Date:** 2026-03-26  
**Status:** **Approved for Implementation**  
**Reviewer:** pi coding agent (second-pass verification)

---

## Correctness Assessment

### Finding 1: Prompt Resolution (MALINKA Layout) — **CORRECT**

Verified `lib/crates/fabro-workflows/src/workflow.rs:147-157`:

```rust
fn infer_repo_root_fallback(base_dir: &Path) -> Option<std::path::PathBuf> {
    for ancestor in base_dir.ancestors() {
        let has_package_layout = ancestor.join("prompts").is_dir()
            && ancestor.join("workflows").is_dir()
            && ancestor.join("run-configs").is_dir();
        if has_package_layout {
            return ancestor.parent().map(std::path::Path::to_path_buf);
        }
    }
    None
}
```

The function only recognizes FABRO layout. For MALINKA repos with `malinka/prompts/`, `malinka/workflows/`, `malinka/run-configs/`, this returns `None`, degrading the fallback chain.

**Fix validation:** The proposed fix correctly adds MALINKA detection and returns `Some(ancestor)` (not `ancestor.parent()`) because the MALINKA directory IS the package root.

### Finding 2: Stale Running Lane Truth — **CORRECT**

Verified `lib/crates/raspberry-supervisor/src/program_state.rs:486-520`:

When `progress` is `None` and `record.status == Running`, the code marks the lane as:
- `status = Failed`
- `failure_kind = TransientLaunchFailure`
- `recovery_action = BackoffRetry`

Additional checks at lines 550+ via `stale_active_progress()` handle worker process disappearance. The reclamation logic is already implemented and correct.

### Finding 3: Dispatch Telemetry — **CORRECT**

Verified `lib/crates/raspberry-supervisor/src/autodev.rs:96-108`:

`AutodevCycleReport` lacks:
- `idle_cycles` / `idle_cycle: bool`
- `ready_but_undispatched: usize`
- `stale_running_reclaimed: usize`

The telemetry gap is confirmed. The proposed additive changes (with `serde(default, skip_serializing_if)`) are backward-compatible.

### Finding 4: Synth Command Exposure — **CORRECT**

Verified `lib/crates/fabro-cli/src/main.rs:192`:

```rust
/// Program synthesis operations
Synth {
    #[command(subcommand)]
    command: commands::synth::SynthCommand,
},
```

The `synth` command IS properly registered with all subcommands (`import`, `create`, `evolve`, `review`, `genesis`). The 2026-03-26 failure was indeed a stale binary or build configuration issue, not structural.

---

## Milestone Fit

| Phase 0 Gate Criteria | Status |
|----------------------|--------|
| 10 active lanes sustained | Pending implementation |
| Zero bootstrap validation failures | Pending `infer_repo_root_fallback` fix |
| 3 lanes to trunk | Pending live validation |

This work directly addresses the Phase 0 stabilization gate. The fixes are prerequisites for reliable autodev operation on proving-ground repos.

---

## Nemesis Security Review

### Pass 1 — First-Principles Challenge

**Trust Boundaries:**
- `infer_repo_root_fallback` performs filesystem traversal based on `base_dir`. The fix adds directory existence checks (`is_dir()`) which are safe — no path resolution outside the ancestor chain.
- No authority escalation: The function only returns paths, it doesn't perform any privileged operations.

**Authority Assumptions:**
- The dispatch path assumes `current_dir = target_repo` sets the execution context correctly. This is a reasonable assumption but the fix makes it defense-in-depth rather than load-bearing.
- No secrets are accessed in the changed code paths.

**Dangerous Action Triggers:**
- `refresh_program_state` can mark lanes as `Failed` and trigger `BackoffRetry`. This is the intended recovery behavior, not a security concern.
- The telemetry fields are read-only observations; they don't affect control flow beyond reporting.

### Pass 2 — Coupled-State Review

**Paired State Surfaces:**

1. **Lane Status + Progress Files:**
   - `refresh_program_state` reads `progress.jsonl` and `state.json` to determine lane status.
   - **Consistency check:** The code correctly handles the case where files are missing (mark as failed) or stale (reclaim).
   - **Asymmetry explained:** Write paths go through `run_fabro` and engine completion; read path goes through `refresh_program_state`. This asymmetry is intentional and documented.

2. **Fallback Chain Resolution:**
   - `workflow_file_fallback_dirs` builds: `[explicit_dirs] → [inferred_repo_root] → [~/.fabro]`
   - **Consistency risk:** If `infer_repo_root_fallback` returns wrong path, resolution falls through to `~/.fabro`, potentially loading wrong prompts.
   - **Mitigation:** The fix adds correct MALINKA detection. Both FABRO and MALINKA layouts should be tested.

3. **Autodev Cycle Report + Runtime State:**
   - The report is a log of decisions; runtime state is the source of truth.
   - **Consistency:** The new telemetry fields (`idle_cycles`, `ready_but_undispatched`, `stale_running_reclaimed`) are derived from loop state, not coupled mutable state. Safe to add.

**Secret Handling:** No secrets in changed code.

**Capability Scoping:** No new capabilities introduced.

**Privilege Escalation:** No escalation paths identified.

**Idempotence:** The `infer_repo_root_fallback` function is pure (no side effects). The telemetry additions are idempotent writes to the report vector.

---

## Remaining Blockers

| Blocker | Severity | Resolution |
|---------|----------|------------|
| MALINKA layout unit test | Required | Add test `prepare_with_file_inlining_infers_malinka_root_fallback` |
| Stale lane reclamation test | Recommended | Add test `refresh_program_state_marks_missing_running_run_as_stale_failure` (exists at line 1913 but should verify new behavior) |
| FABRO layout regression test | Required | Ensure existing test still passes after fix |

---

## Recommendations

1. **Implement the `infer_repo_root_fallback` fix first** — it unblocks prompt resolution and is the critical path for live validation.

2. **Add both layout detection patterns** in the same function with a comment explaining the return value difference:
   ```rust
   // FABRO: return ancestor.parent() (the repo root containing the package)
   // MALINKA: return ancestor (the malinka/ dir IS the package root)
   ```

3. **Keep telemetry additive only** — do not remove or rename existing `AutodevCycleReport` fields.

4. **Consider the `start_cwd` dependency** — The dispatch path currently relies on `current_dir = target_repo`. After the fix, this becomes a fallback rather than load-bearing behavior. Document this change in the function's doc comment.

---

## Verdict

**Approved for implementation.** The findings are accurate, the fixes are well-scoped, and the security review reveals no concerning issues. The work aligns with Phase 0 stabilization goals and unblocks the proving-ground live validation gate.