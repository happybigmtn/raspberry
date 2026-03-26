# Autodev Execution Path and Dispatch Truth — Security Review

Status: Review Complete  
Date: 2026-03-26  
Lane: `autodev-efficiency-and-dispatch`  
Reviewer: Nemesis-style security analysis  

---

## Executive Summary

**Verdict: CONDITIONAL APPROVE with security hardening required**

The specification is architecturally sound and the implementation gaps are correctly identified. However, the security review reveals **three critical trust boundary violations** and **two privilege escalation paths** that must be addressed before the slice lands to trunk.

The core issue: the autodev orchestrator operates as a **privileged dispatcher** but lacks defensive boundaries against the **unprivileged worker processes** it spawns. This creates asymmetric trust where a compromised lane worker can influence the controller's state and scheduling decisions.

---

## Correctness Assessment

| Requirement | Spec Correctness | Review Accuracy | Status |
|-------------|------------------|-----------------|--------|
| R1.1 — synth subcommand | ✅ Correct | ✅ Accurate | Spec is executable |
| R1.2 — prompt resolution | ⚠️ Partial | ✅ Accurate | Needs validation test |
| R2.1 — stale running | ✅ Correct | ✅ Accurate | Existing impl sound |
| R2.2 — keep alive | ✅ Correct | ✅ Accurate | Existing impl sound |
| R2.3 — bootstrap failure | ✅ Correct | ✅ Critical bug | **Pre-spawn errors don't transition state** |
| R3.1 — dispatch telemetry | ✅ Correct | ✅ Accurate | Missing impl |
| R3.2 — aggregate metrics | ✅ Correct | ✅ Accurate | Missing impl |
| R4.1 — evolve decoupling | ✅ Correct | ✅ Accurate | **Blocking: 120s sync call** |
| R4.2 — full budget | ✅ Correct | ✅ Accurate | Blocked by R2.1/R2.3 |

**Critical correctness finding**: The review correctly identifies that `run_fabro()` returning `DispatchError::MissingRunConfig` or `DispatchError::Spawn` does **not** transition the lane to `Failed`. The lane remains in its previous state (often `Ready` or `Running`), causing:
1. Slot leakage — the lane counts against `max_parallel` but has no worker
2. Infinite retry loops — no `BackoffRetry` is scheduled
3. State inconsistency — `program_state` claims `Running` but no PID exists

**Fix verification**: The recommended fix in dispatch.rs (lines 245-255 equivalent) correctly transitions the lane to `Failed` with `TransientLaunchFailure` / `BackoffRetry`.

---

## Milestone Fit Assessment

### Live Validation Criteria

**V1: Sustain 10 active lanes on rXMRbro**
- **Pass criteria**: `running >= 8` for 20 cycles, `idle_cycles <= 5`, `bootstrap_failures = 0` after cycle 2
- **Blockers**: R2.3 (bootstrap failures not transitioning state) will cause slot leakage. R4.1 (evolve blocking) will cause idle cycles during evolve.
- **Confidence**: HIGH — the fixes are bounded and testable.

**V2: 3 lanes land to trunk**
- **Pass criteria**: `landing_state = "landed"` for 3+ lanes
- **Blockers**: None identified. Integration lanes are already functional.
- **Confidence**: HIGH — integration logic is mature.

### Implementation Sequence Assessment

The recommended sequence (R2.3 → R3.1 → R3.2 → R4.1 → R1.2 → V1/V2) is **correct**:
1. R2.3 fixes the correctness bug that would invalidate V1
2. R3.1/R3.2 add observability needed to debug V1 failures
3. R4.1 removes the 120s blocking call that would cause V1 idle cycles
4. R1.2 is validation-only if prompt resolution already works

---

## Nemesis-Style Security Review

### Pass 1 — First-Principles Challenge

#### Trust Boundaries

**Violation 1: Symmetric privilege between controller and workers**

The autodev orchestrator runs with the same OS privileges as the fabro workers it spawns. There is no:
- UID separation (workers run as same user)
- Capability dropping (workers inherit all capabilities)
- Namespace isolation (workers share filesystem/network namespaces)

**Attack scenario**: A compromised lane worker (via malicious dependency in target_repo) can:
1. Write to the controller's state file (`program-state.json`) if path is predictable
2. Hijack the `fabro_bin` path to execute arbitrary code on next dispatch
3. Exhaust dispatch slots by crashing immediately, triggering the R2.3 bug

**Evidence** (autodev.rs:2300):
```rust
let output = evolve.output().map_err(|source| AutodevError::Spawn {
    step: "synth evolve".to_string(),
    program: manifest.program.clone(),
    source,
})?;
```
The `evolve` command inherits the controller's environment and working directory without sanitization.

**Authority Assumptions**

**Violation 2: Implicit authority from manifest file presence**

The system assumes that the existence of `manifest_path` implies authority to execute the program it defines. No additional authentication is performed between:
- Manifest loading and lane dispatch
- Synth evolve triggering and blueprint modification
- State file write and subsequent dispatch decisions

**Attack scenario**: An attacker with write access to the manifest directory (but not the controller) can:
1. Add a malicious lane to the manifest
2. The controller will dispatch it on next cycle without signature verification
3. The lane runs with controller's privileges

**Evidence** (autodev.rs:384):
```rust
let manifest = ProgramManifest::load(&manifest_path)?;
// No signature check, no hash verification
```

**Dangerous Actions**

**Violation 3: Unvalidated path construction for command execution**

Multiple paths are constructed from user-controlled input and passed to `Command::new()` without validation:

| Path Source | Usage | Risk |
|-------------|-------|------|
| `settings.fabro_bin` | `Command::new(fabro_bin)` | Path injection — attacker controls binary executed |
| `manifest.resolved_target_repo()` | `current_dir(&target_repo)` | Directory traversal — worker runs in attacker-controlled CWD |
| `run_config` | `arg(run_config)` | Argument injection — shell metacharacters in path |
| `temp_dir` | `fs::copy(&source_blueprint, &copied)` | Symlink attack — predictable temp path |

**Evidence** (dispatch.rs:510-535):
```rust
fn run_fabro(fabro_bin: &Path, target_repo: &Path, run_config: &Path, ...) {
    // No validation that fabro_bin is the expected binary
    // No validation that target_repo is within expected bounds
    // No validation that run_config doesn't contain shell metacharacters
    let mut command = Command::new(fabro_bin);
    command.current_dir(target_repo)
           .arg(run_config);  // Passed directly as argument
}
```

**Critical**: The `run_config` path is passed as an argument to `fabro run --detach`. If `run_config` contains spaces or special characters, argument injection is possible depending on how fabro parses arguments.

---

### Pass 2 — Coupled-State Review

#### Paired State Surfaces

**Coupling 1: Lane status vs Worker process lifecycle**

The system maintains two sources of truth for "is this lane running?":
1. `LaneRuntimeRecord.status` — persisted in `program-state.json`
2. Worker process existence — checked via `/proc/<pid>` (Linux only)

**Inconsistency found**: On macOS (non-Linux), `worker_process_alive()` always returns `Some(false)`:

```rust
#[cfg(not(target_os = "linux"))]
{
    let _ = pid;
    Some(false)  // BUG: macOS lanes marked stale after 30s
}
```

This means macOS deployments will incorrectly transition long-running lanes to `Failed` after `STALE_RUNNING_GRACE_SECS` (30s), even when healthy.

**Recommendation**: Either implement macOS PID checking (via `kill(pid, 0)`) or document macOS as unsupported for production autodev.

**Coupling 2: Evolve frontier vs Actual manifest changes**

The `last_evolve_frontier` field tracks what frontier signature triggered evolve, but there's no verification that evolve actually changed the manifest:

```rust
if should_trigger_evolve(...) {
    run_synth_evolve(...)?;  // May succeed but make no changes
    last_evolve_frontier = Some(frontier_before);  // Recorded regardless
}
```

**Asymmetry**: The system assumes evolve is idempotent and always beneficial. If evolve corrupts the manifest, the controller continues with corrupted state.

**Recommendation**: Hash the manifest before/after evolve and only update `last_evolve_frontier` if content changed.

**Coupling 3: Dispatch slot accounting vs Actual process count**

The `max_parallel` budget is computed as:
```rust
let available_slots = max_parallel.saturating_sub(current_running);
```

Where `current_running` is derived from `program_state.lanes` with status `Running`.

**Inconsistency**: Due to R2.3, a lane can be `status = Running` in state but have no actual worker process. This creates a "phantom slot" that reduces available parallelism without contributing work.

**Evidence**: The stale detection in `program_state.rs` only triggers after `STALE_RUNNING_GRACE_SECS` (30s). During those 30s, the slot is wasted.

---

### Secret Handling

**Finding: Lease environment variables passed without audit**

The `resource_lease::env_for_run_config()` function returns environment variables that are injected into the worker process:

```rust
let leased_env = resource_lease::env_for_run_config(target_repo, lane_key, run_config)?;
// ...
for (key, value) in entries {
    command.env(key, value);
}
```

**Risk**: If lease configuration contains secrets (API keys, tokens), they are:
1. Passed to potentially untrusted worker processes
2. Visible in `/proc/<pid>/environ` to any user with PID read access
3. Not redacted in logs (dispatch.rs logs command but not env)

**Recommendation**: 
1. Document that leased env should not contain secrets
2. Add `FABRO_LEASE_SECRET_*` naming convention for automatic redaction in logs
3. Consider file-based secret passing (tmpfs-mounted files) instead of environment variables

### Capability Scoping

**Finding: No capability-based restrictions**

The system relies entirely on OS-level permissions. Workers can:
- Access the entire filesystem (subject to UNIX permissions)
- Make network connections
- Spawn child processes
- Consume arbitrary CPU/memory

**Specific gap**: The `fabro synth evolve` command runs with full controller privileges and can:
- Modify any file in `target_repo`
- Execute arbitrary code via `.git/hooks` if target_repo is git-controlled
- Access controller's SSH keys if running in user context with SSH agent

**Recommendation**: Consider using `clone()` with namespaces or a container runtime to restrict worker capabilities.

### Pairing/Idempotence Behavior

**Finding: Regenerate fingerprint doesn't verify determinism**

The `lane_render_fingerprints` function computes SHA256 of run_config and graph, but:
1. No verification that the same inputs produce the same outputs across runs
2. No detection of non-deterministic synthesis

**Risk**: A non-deterministic `synth evolve` could produce different outputs for the same inputs, but the fingerprint would cache the old result, preventing legitimate regeneration.

**Finding: Consecutive failures counter without backoff randomization**

```rust
pub consecutive_failures: u32,
```

The `BackoffRetry` action uses fixed timeouts (`TRANSIENT_LAUNCH_RETRY_MIN_SECS = 15`). This creates synchronized retry storms if multiple lanes fail simultaneously (e.g., due to an external service outage).

**Recommendation**: Add jitter to backoff calculations (`retry_after_secs = base + rand(0, base/2)`).

### Privilege Escalation Paths

**Path 1: `fabro_bin` hijacking via settings**

**Steps**:
1. Attacker gains write access to controller's config (or environment)
2. Modifies `settings.fabro_bin` to point to attacker-controlled binary
3. Controller spawns attacker binary on next dispatch
4. Attacker binary runs with controller's privileges

**Mitigation**: Verify `fabro_bin` against a known hash or signature before execution. At minimum, log the binary's SHA256 on startup.

**Path 2: Symlink attack on predictable temp directory**

**Steps**:
1. Attacker monitors `autodev_temp_dir()` pattern: `/tmp/raspberry-autodev-{program}-{pid}-{timestamp}`
2. Creates symlink from predicted path to sensitive file (e.g., `~/.ssh/authorized_keys`)
3. Controller writes blueprint to temp dir, following symlink, overwriting sensitive file

**Code** (autodev.rs:2415):
```rust
let path = std::env::temp_dir().join(format!(
    "raspberry-autodev-{}-{}-{}",
    program,
    std::process::id(),
    chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
));
fs::create_dir_all(&path)?;
```

**Mitigation**: 
1. Use `tempfile::TempDir` which creates directories with `O_EXCL` flags
2. Verify path is not a symlink before writing
3. Use `std::fs::symlink_metadata()` to check file type

**Path 3: Blueprint injection via doctrine_files**

**Steps**:
1. Attacker controls content of doctrine file (e.g., via git commit)
2. `inject_inputs_into_blueprint` blindly copies doctrine file paths into blueprint
3. YAML injection possible if doctrine file contains malicious YAML

**Code** (autodev.rs:2475):
```rust
inputs_map.insert(
    Value::String("doctrine_files".to_string()),
    Value::Sequence(
        doctrine_files
            .iter()
            .map(|path| Value::String(path.display().to_string()))
            .collect(),
    ),
);
```

**Risk**: If doctrine file paths are later evaluated as YAML (not just strings), code execution is possible.

**Mitigation**: Validate that doctrine file paths are valid UTF-8 and don't contain YAML metacharacters (`:`, `{`, `}`, `[`, `]`).

---

## Remaining Blockers

### Must Fix (Security)

1. **TEMP-SYMLINK**: `autodev_temp_dir` is predictable and vulnerable to symlink attacks. Use `tempfile::TempDir` with `O_EXCL`.

2. **MACOS-STALE**: macOS incorrectly reports all workers as dead after 30s. Implement `kill(pid, 0)` for macOS or document as unsupported.

3. **ENV-LEAK**: Leased environment variables may contain secrets visible to workers. Add documentation and redaction for `*_SECRET_*` vars.

### Must Fix (Correctness)

4. **R2.3-BOOTSTRAP**: Pre-spawn errors in `run_fabro` don't transition lane state. Implement the fix from the review (lines 245-255).

5. **R4.1-BLOCKING**: `run_synth_evolve` blocks dispatch for 120s. Implement background thread approach from spec.

### Should Fix (Observability)

6. **R3.1-TELEMETRY**: Add `DispatchState` to `AutodevCycleReport` per spec.

7. **R3.2-METRICS**: Add aggregate counters (`idle_cycles`, `stale_running_reclaimed`) per spec.

### Validation Required

8. **R1.2-PROMPT**: Verify `fabro-graphviz` resolves `@prompts/...` relative to graph file, not CWD.

---

## Risk Summary

| Risk | Severity | Likelihood | Mitigation Status |
|------|----------|------------|-------------------|
| Symlink attack on temp dir | **HIGH** | Medium | Not mitigated |
| macOS stale worker detection | **MEDIUM** | High (on macOS) | Not mitigated |
| Secret leakage via env vars | **MEDIUM** | Medium | Not documented |
| fabro_bin path injection | **MEDIUM** | Low | Not mitigated |
| Bootstrap failure slot leak | **HIGH** | High | Fix identified |
| Evolve blocking dispatch | **MEDIUM** | High | Fix identified |

---

## Approval Recommendation

**APPROVE for implementation** with the following conditions:

1. **Security fixes must be implemented in this slice**:
   - TEMP-SYMLINK (use tempfile::TempDir)
   - MACOS-STALE (implement kill(pid, 0) or document)

2. **Correctness fixes must be implemented in this slice**:
   - R2.3-BOOTSTRAP (pre-spawn error state transition)
   - R4.1-BLOCKING (background evolve thread)

3. **Live validation must pass**:
   - V1: 10 lanes sustained on rXMRbro
   - V2: 3 lanes landed to trunk

4. **Security debt filed for follow-up** (acceptable to defer):
   - ENV-LEAK (secret handling documentation)
   - fabro_bin path validation
   - Capability-based sandboxing

The specification is sound, the review is accurate, and the security issues are manageable with the identified mitigations.

---

## Appendix: Code References

| File | Line | Context |
|------|------|---------|
| autodev.rs | 2300 | `run_synth_evolve` command construction |
| autodev.rs | 2415 | `autodev_temp_dir` predictable path |
| autodev.rs | 2475 | `inject_inputs_into_blueprint` doctrine injection |
| dispatch.rs | 510-535 | `run_fabro` unvalidated paths |
| program_state.rs | ~140 | macOS `worker_process_alive` returns false |
| failure.rs | 18-34 | `FailureKind` and `FailureRecoveryAction` enums |
