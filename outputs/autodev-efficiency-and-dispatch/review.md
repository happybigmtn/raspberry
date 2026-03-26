# Autodev Execution Path and Dispatch Truth — Nemesis Security Review

**Lane:** `autodev-efficiency-and-dispatch`  
**Date:** 2026-03-26  
**Status:** Review Complete — Implementation Required  
**Review Type:** Nemesis-style security audit + milestone fitness assessment

---

## Executive Summary

The spec accurately describes the current implementation state and identifies four genuine issues blocking reliable autodev execution. The lane has **solid foundations** in `program_state.rs` and `evaluate.rs`, but **three of four identified issues are implementation gaps** requiring code changes before live validation can succeed.

**Verdict:** Spec is correct. Implementation incomplete. Four blocking issues confirmed.

---

## Spec Correctness Assessment

### Claims Verified Against Source

| Claim | Evidence | Status |
|-------|----------|--------|
| `stale_active_progress_reason()` logic exists | `program_state.rs:1423-1429` | ✅ Confirmed |
| `STALE_RUNNING_GRACE_SECS = 30` | `program_state.rs:22` | ✅ Confirmed |
| `@../../prompts/` path in render.rs | `fabro-synthesis/src/render.rs:1915` | ✅ Confirmed |
| `DispatchTelemetry` missing | grep finds no definition | ✅ Confirmed |
| `validate_fabro_command_surface` missing | grep finds no definition | ✅ Confirmed |
| `should_trigger_evolve()` blocks dispatch | `autodev.rs:895-965` | ✅ Confirmed |
| `run_synth_evolve()` uses `timeout` wrapper | `autodev.rs:2156-2208` | ✅ Confirmed |

### Spec Quality: ACCEPTABLE

The spec correctly identifies the failure modes and proposes reasonable remediation. The acceptance criteria are outcome-shaped and verifiable. The architecture section accurately reflects the current code structure.

**One gap:** The spec does not explicitly address the **race condition** between state refresh and dispatch that can occur when `worker_process_alive()` returns stale results due to PID reuse.

---

## Nemesis Security Review — Pass 1: First-Principles Challenge

### Trust Boundaries and Authority Assumptions

#### 1.1 Binary Provenance Blind Trust
**Location:** `autodev.rs:369` in `orchestrate_program()`

The controller captures "provenance" via `capture_autodev_provenance()` which reads the fabro binary path from settings, but **never validates**:
- That the binary is the one that was built/installed
- That the binary has not been replaced between cycles
- That the binary exposes required commands

**Attack vector:** If an attacker can replace the `fabro` binary on disk between autodev cycles, subsequent synth/evolve operations execute attacker-controlled code. The controller has no binary integrity check.

**Required mitigation:** Add command surface validation at startup (as spec Issue 1 recommends) with a hash/checksum validation of the binary.

#### 1.2 Target Repo Privilege Escalation
**Location:** `autodev.rs:2180` in `rerender_program_package()`

The `synth evolve` command runs with:
- Current directory = `target_repo`
- Full user environment inherited
- `CARGO_TARGET_DIR` redirected to a temp location

**Attack vector:** A malicious `target_repo` with a `.cargo/config.toml` can inject arbitrary code into the synthesis process. The controller does not sanitize the environment before running fabro commands in the target repo.

**Required mitigation:** Document this as a known limitation or run synth evolve in a restricted environment (minimal env vars, read-only mounts where possible).

#### 1.3 Who Can Trigger Dangerous Actions
**Dangerous action:** `run_synth_evolve()` can rewrite the entire `malinka/` package

**Trigger path:** `should_trigger_evolve()` returns true when:
- Doctrine files changed (file mtime comparison)
- Frontier settled AND frontier_progressed
- Recovery needs evolve AND spare capacity exists

**Vulnerability:** The `doctrine_inputs_changed()` function (around `autodev.rs:1015`) fingerprints files by **mtime + len**, not content hash. An attacker with write access to doctrine files can:
1. Keep mtime unchanged (touch -m preservation)
2. Modify file content
3. Trigger spurious regenerations that overwrite good work

**Required mitigation:** Add content hashing to `DoctrineFileFingerprint` (currently only stores `len` and `modified_unix_ms`).

#### 1.4 Maintenance Mode Bypass Risk
**Location:** `autodev.rs:340-357`

If `load_active_maintenance()` returns an error instead of `Ok(Some(maintenance))`, the code continues to normal execution. The error case falls through to `let _lease = acquire_autodev_lease(...)`.

**Attack vector:** A filesystem permissions issue or corrupted maintenance file could silently bypass maintenance mode.

**Required mitigation:** Maintenance load errors should fail closed (stop execution), not fail open.

---

## Nemesis Security Review — Pass 2: Coupled-State Consistency

### 2.1 Lane State Coupling: RuntimeRecord ↔ RunSnapshot ↔ Worker Process

**The three sources of truth:**
1. `LaneRuntimeRecord` in `.raspberry/*-state.json` (persisted state)
2. `RunSnapshot` from `~/.fabro/runs/<run-id>/` (fabro's view)
3. Worker process existence via `worker_process_alive()` (kernel view)

**Current consistency check:**
```rust
// program_state.rs:1423
if progress.worker_alive != Some(false) {
    return None;  // Not stale if alive OR uncertain
}
// ... grace period check
```

**Asymmetry identified:** If `worker_process_alive()` returns `None` (uncertain), the lane is **not** marked stale even if the grace period has passed. This is correct for avoiding false positives but creates an **infinite stale window** on systems where `/proc/<pid>` is unreliable.

**Required fix:** After `STALE_RUNNING_GRACE_SECS * 2` (60s), if `worker_alive` is still `None`, force a status re-check via `fabro status` or mark as stale.

### 2.2 Dispatch Budget Coupling: max_parallel ↔ Running Count

**The budget calculation:**
```rust
// autodev.rs:457
let max_parallel = settings
    .max_parallel_override
    .unwrap_or(manifest.max_parallel)
    .max(1);
let frontier_budget = resolve_frontier_budget(settings, max_parallel);
```

**Race condition:** Between `evaluate_program()` (line 448) and `execute_selected_lanes()` (line 600), a dispatched lane could complete, freeing a slot. However, the running count is not re-checked before dispatch.

**Impact:** May dispatch fewer lanes than capacity allows. Not a security issue but violates the "dispatch immediately within budget" acceptance criterion.

### 2.3 Synth Evolve State Coupling: Last Evolve Frontier

**The coupled state:**
```rust
// autodev.rs:323-324
let mut last_evolve_at = None::<Instant>;
let mut last_evolve_frontier = None::<FrontierSignature>;
```

**Inconsistency risk:** If the autodev process crashes after `run_synth_evolve()` succeeds but before `last_evolve_frontier = Some(frontier_before)` is persisted, the next restart will:
1. See evolved package (files on disk changed)
2. Have `last_evolve_frontier = None` in memory
3. Possibly trigger duplicate evolve on first cycle

**Mitigation:** Acceptable — duplicate evolve is idempotent (rerender produces same output).

### 2.4 Lease/State File Consistency

**The coupling:** `acquire_autodev_lease()` creates a lock file while `ProgramRuntimeState` tracks lane status.

**Risk:** If lease acquisition succeeds but state file is corrupted, the controller could:
1. Hold the lease (preventing other controllers)
2. Have inconsistent lane status (dispatching to dead lanes)

**Verification:** The `ProgramRuntimeState` uses atomic writes via `write_atomic()`, but there's no checksum/validation on read. Corrupted JSON could produce `LaneRuntimeRecord` with `status: Running` but `current_run_id: None`.

**Required mitigation:** Add schema version validation and graceful degradation on parse errors.

### 2.5 Prompt Path Resolution Coupling

**The bug confirmed:** `render.rs:1915` generates:
```rust
format!("@../../prompts/{}/{}/{}.md", lane.workflow_family(), lane.slug(), name)
```

**Resolution chain at runtime:**
1. Run config is at `malinka/run-configs/<lane>.yaml`
2. Graph reference `@../../prompts/...` resolves relative to graph location
3. If graph is copied to `~/.fabro/runs/<run-id>/`, `@../../` goes to `~/.fabro/`

**Inconsistency:** The render assumes a specific directory structure that does not hold when workflows are dispatched.

**Fix required:** Use run-directory-relative paths or absolute paths. Spec Issue 2 correctly identifies this.

---

## Milestone Fitness Assessment

### Acceptance Criteria Review

| # | Criterion | Blocked By | Achievable? |
|---|-----------|------------|-------------|
| 1 | `fabro synth --help` works | Issue 1 (validation) | ✅ Yes — add startup probe |
| 2 | `fabro run --detach` with prompt refs | Issue 2 (path resolution) | ✅ Yes — fix render.rs path |
| 3 | Stale running reclassified in 30s | Works today | ✅ Already true |
| 4 | Dispatch telemetry fields | Issue 3 (telemetry gaps) | ✅ Yes — add struct fields |
| 5 | 10 lanes sustained 20 cycles | Issues 1, 2, 4 | ⚠️ Requires fixes 1, 2, 4 |
| 6 | Zero bootstrap failures first 10 cycles | Issues 1, 2 | ⚠️ Requires fixes 1, 2 |

### Live Validation Gate Readiness: NOT READY

**Blockers for 10-lane sustained run:**
1. **Issue 1** — Missing `synth` command will cause immediate bootstrap failure on clean builds
2. **Issue 2** — Wrong prompt paths will cause bootstrap validation failures
3. **Issue 4** — Blocking evolve will cause dispatch delays under load

**Missing evidence needed:**
- Proof that `fabro synth` is linked in release builds
- Generated workflow graph showing correct `@prompts/` paths
- Cycle report showing telemetry fields populated
- 20-cycle trace from proving-ground repo

---

## Remaining Blockers (Ranked)

### Blocker 1: Prompt Path Resolution (HIGH)
**File:** `lib/crates/fabro-synthesis/src/render.rs:1915`  
**Issue:** `@../../prompts/` resolves to wrong directory  
**Fix:** Use run-directory-relative paths or absolute paths  
**Risk if not fixed:** Every dispatched lane fails bootstrap validation

### Blocker 2: Command Surface Validation (HIGH)
**File:** `lib/crates/raspberry-supervisor/src/autodev.rs` (new code)  
**Issue:** No startup probe for required commands  
**Fix:** Add `validate_fabro_command_surface()` at startup  
**Risk if not fixed:** Silent failures on clean builds; operator confusion

### Blocker 3: Blocking Evolve on Hot Path (MEDIUM)
**File:** `lib/crates/raspberry-supervisor/src/autodev.rs:475-510`  
**Issue:** `run_synth_evolve()` blocks dispatch  
**Fix:** Move evolve to background thread with cadence gating  
**Risk if not fixed:** Dispatch delays under load; frontier under-utilization

### Blocker 4: Dispatch Telemetry Gaps (MEDIUM)
**File:** `lib/crates/raspberry-supervisor/src/autodev.rs`  
**Issue:** `AutodevCycleReport` lacks diagnostic fields  
**Fix:** Add `DispatchTelemetry` struct per spec  
**Risk if not fixed:** Operational blindness — cannot debug why ready work doesn't run

---

## Security Findings Summary

| Severity | Finding | Mitigation |
|----------|---------|------------|
| Medium | Binary provenance blind trust | Add hash validation to startup probe |
| Medium | Target repo env injection risk | Document limitation; consider sandboxing |
| Low | Maintenance mode fail-open | Change error handling to fail-closed |
| Low | Doctrine mtime-only fingerprinting | Add content hash to fingerprint |
| Low | Uncertain worker_alive = infinite stale | Add fallback timeout for None case |

---

## Recommendations

### Immediate (Pre-Validation)
1. **Fix render.rs path resolution** — This is the highest-impact blocker
2. **Add command surface validation** — Fail fast with actionable errors
3. **Run 5-lane proving test** before attempting 10-lane validation

### Short-Term (Phase 0 Completion)
4. **Add dispatch telemetry** — Required for operational visibility
5. **Decouple evolve from dispatch** — Improves throughput

### Security Hardening (Post-Phase 0)
6. **Add binary integrity check** — SHA-256 of fabro bin at startup
7. **Fail-closed maintenance mode** — Don't proceed on maintenance load errors
8. **Content-hash doctrine files** — Prevent mtime manipulation

---

## Conclusion

The spec is **correct and implementable**. The review identified **four confirmed blockers** (Issues 1-4) that must be resolved before live validation. The nemesis review found **five security findings** ranging from Low to Medium severity — none block Phase 0, but all should be addressed before production deployment.

**Next step:** Implement Issue 2 (prompt path resolution) and Issue 1 (command validation), then run a 5-lane proving test to verify the execution path before attempting the full 10-lane validation.

**Status:** Review complete. Awaiting implementation of blockers 1-4.
