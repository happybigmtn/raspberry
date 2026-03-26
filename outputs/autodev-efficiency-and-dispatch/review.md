# Autodev Execution Path and Dispatch Truth — Security & Correctness Review

Status: Post-Spec-Polish Review
Date: 2026-03-26
Lane: `autodev-efficiency-and-dispatch`
Reviewer: Nemesis-style security analysis (post-polish)

---

## Executive Summary

**Verdict: APPROVE for implementation with tracked blockers**

The spec is executable and the identified bugs are bounded. Four bugs must be fixed before live validation; three can be filed as security debt. The dispatch telemetry additions (R3.1, R3.2) are clean additions. The background evolve refactor (R4.1) is architecturally sound.

---

## Correctness Review

### Bug R2.3 — Pre-spawn errors leave lane state unchanged (CRITICAL)

**Location**: `dispatch.rs:220-299`

**Finding**: `run_fabro` (dispatch.rs:495) returns `DispatchError::MissingRunConfig`, `DispatchError::Lease`, or `DispatchError::Spawn` before a worker process is spawned. These errors propagate via `?` from the spawned thread:

```rust
// dispatch.rs:268-270
for (lane, is_program_lane, is_integration_lane, output) in joined_results {
    let output = output?;  // DispatchError propagates here, skips mark_lane_dispatch_failed
    // ...
    mark_lane_dispatch_failed(&mut state, &lane.lane_key, &lane.run_config, &output);
}
```

When `run_fabro` returns `DispatchError::MissingRunConfig`:
1. The thread returns `Err(DispatchError::MissingRunConfig)`
2. This is not a panic, so it falls through to the error arm
3. `panic_error` is set, the loop breaks, and `execute_selected_lanes` returns `Err(PanicError)`
4. `mark_lane_dispatch_failed` is **never called** for any lane in that cycle
5. The original lane (and all other lanes in the batch) are left with stale state

**Impact**: Slot leakage, infinite retry loops, state inconsistency.

**Fix**: Wrap pre-spawn errors in `DispatchOutcome`:
```rust
Err(DispatchError::MissingRunConfig { lane, path }) => {
    joined_results.push(Ok((
        lane,
        false,
        false,
        DispatchOutcome {
            lane_key: lane.lane_key.clone(),
            exit_status: -1,
            fabro_run_id: None,
            stdout: String::new(),
            stderr: format!("pre-spawn: run config not found at {}", path.display()),
        },
    )));
}
```

**Status**: Bug confirmed. Fix is specified in spec R2.3.

---

### Bug R2.2 — macOS `worker_process_alive` always returns `Some(false)` (HIGH)

**Location**: `program_state.rs:1356-1368`

**Finding**: The non-Linux branch unconditionally returns `Some(false)`:
```rust
#[cfg(not(target_os = "linux"))]
{
    let _ = pid;
    Some(false)
}
```

On macOS, every worker appears dead after `STALE_RUNNING_GRACE_SECS` (30s), causing legitimate workers to be marked `Failed`. This affects any macOS-based autodev deployment.

**Fix**: Use `libc::kill(pid, 0)` on non-Linux platforms. Specified in R2.2.

**Status**: Bug confirmed. Fix is specified in spec R2.2.

---

### Bug R4.1 — `synth evolve` blocks dispatch for 120 seconds (MEDIUM)

**Location**: `autodev.rs:444`, `autodev.rs:2242`

**Finding**: `run_synth_evolve` is called synchronously in the orchestrator loop. With `SYNTH_EVOLVE_TIMEOUT_SECS = 120`, a slow evolve blocks dispatch for up to 2 minutes:

```rust
// autodev.rs:444
match run_synth_evolve(&manifest_path, &manifest, settings) {
    Ok(()) => { /* evolve completed */ },
    // ... timeout handling
}
// execute_selected_lanes is called AFTER evolve completes
```

**Fix**: Spawn `run_synth_evolve` in a background thread, store `JoinHandle` in orchestrator state. Skip dispatch while `pending_evolve.is_some()`. Join on next cycle. Specified in R4.1.

**Status**: Bug confirmed. Fix is specified in spec R4.1.

---

### Missing R3.1 — `DispatchState` not present in `AutodevCycleReport` (MEDIUM)

**Location**: `autodev.rs:96`

**Finding**: `AutodevCycleReport` has no field explaining why lanes were or were not dispatched each cycle. The `dispatched: Vec<DispatchOutcome>` field only shows what ran, not why other work didn't run.

**Fix**: Add `dispatch_state: Option<DispatchState>` to `AutodevCycleReport`. Populate it in the orchestrator loop before `execute_selected_lanes`. Specified in R3.1.

**Status**: Missing implementation. Specified in spec R3.1.

---

### Missing R3.2 — Aggregate counters not present in `AutodevCurrentSnapshot` (MEDIUM)

**Location**: `autodev.rs:70`

**Finding**: `AutodevCurrentSnapshot` lacks `idle_cycles`, `total_dispatched`, `stale_running_reclaimed`, `bootstrap_failures`, `runtime_path_errors`, and `dispatch_rate`. These are needed to assess overall dispatch efficiency without parsing every cycle report.

**Fix**: Add fields to `AutodevCurrentSnapshot` struct. Increment counters in the orchestrator loop. Specified in R3.2.

**Status**: Missing implementation. Specified in spec R3.2.

---

## Security Review

### TEMP-SYMLINK — Predictable temp directory allows symlink attacks (HIGH)

**Location**: `autodev.rs:2259-2267`

**Finding**: `autodev_temp_dir` constructs a predictable path:
```rust
let path = std::env::temp_dir().join(format!(
    "raspberry-autodev-{}-{}-{}",
    program,
    std::process::id(),
    chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
));
fs::create_dir_all(&path)?;
```

An attacker who can predict the `{program}-{pid}-{timestamp}` tuple can create a symlink at that path before the controller writes to it, causing writes to an arbitrary location.

**Mitigation**: Use `tempfile::TempDir` which creates directories with `O_EXCL` flags, preventing symlink attacks:
```rust
use tempfile::TempDir;
let temp_dir = TempDir::new_in(std::env::temp_dir(), "raspberry-autodev")?;
let path = temp_dir.path().to_path_buf();
```

**Status**: Filed as security debt. Fix should be included in this slice.

---

### FABRIC-BIN — `fabro_bin` path not validated before execution (MEDIUM)

**Location**: `dispatch.rs:516`

**Finding**: `settings.fabro_bin` is passed directly to `Command::new(fabro_bin)` without validation. If an attacker can modify the settings file or environment, they can cause arbitrary binary execution.

**Current mitigation**: The binary path comes from the controller's own settings, not from user input in the target repo.

**Recommendation**: Log the binary's SHA256 on startup. File as security debt.

**Status**: Security debt.

---

### ENV-LEAK — Leased environment variables may contain secrets (MEDIUM)

**Location**: `dispatch.rs:526-532`

**Finding**: `resource_lease::env_for_run_config` returns environment variables injected into the worker process. These are visible in `/proc/<pid>/environ` to any user with PID read access.

**Current mitigation**: Leased env is intended for sandbox configuration (resource limits, not secrets). No secrets are currently leased.

**Recommendation**: Document that leased env must not contain secrets. Add `FABRO_LEASE_SECRET_*` redaction in logs.

**Status**: Security debt.

---

## Live Validation Assessment

### V1: Sustain 10 active lanes on rXMRbro

**Blockers**: R2.3 (pre-spawn errors don't update state), R4.1 (evolve blocks dispatch)

**Confidence**: HIGH after fixes — both bugs are bounded and testable.

### V2: 3 lanes land to trunk

**Blockers**: None from this review. Integration logic is mature.

**Confidence**: HIGH — this is the primary validation criterion.

---

## Remaining Blockers Before Live Validation

| Priority | ID | Description | File |
|----------|----|-------------|------|
| Must Fix | R2.3 | Pre-spawn errors don't call `mark_lane_dispatch_failed` | dispatch.rs:268 |
| Must Fix | R2.2 | macOS `worker_process_alive` always returns `false` | program_state.rs:1366 |
| Must Fix | R4.1 | `run_synth_evolve` blocks dispatch for 120s | autodev.rs:444 |
| Must Fix | TEMP-SYMLINK | Predictable temp dir path | autodev.rs:2259 |
| Must Implement | R3.1 | Add `DispatchState` to `AutodevCycleReport` | autodev.rs:96 |
| Must Implement | R3.2 | Add aggregate counters to snapshot | autodev.rs:70 |

---

## Risk Summary

| Risk | Severity | Likelihood | Status |
|------|----------|------------|--------|
| Pre-spawn error slot leak | **HIGH** | High | Bug confirmed |
| macOS stale worker detection | **HIGH** | High (on macOS) | Bug confirmed |
| Evolve blocking dispatch | **MEDIUM** | High | Bug confirmed |
| Symlink attack on temp dir | **HIGH** | Medium | Security debt (fix in slice) |
| Secret leakage via env vars | **MEDIUM** | Low | Security debt |
| fabro_bin path injection | **MEDIUM** | Low | Security debt |

---

## Approval Recommendation

**APPROVE for implementation** subject to:
1. All "Must Fix" bugs are corrected before live validation
2. R3.1 and R3.2 (dispatch telemetry) are implemented
3. V1 + V2 pass on rXMRbro

Security debt (TEMP-SYMLINK fix, FABRIC-BIN logging, ENV-LEAK documentation) may be filed as follow-up issues.
