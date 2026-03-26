# Autodev Execution Path and Dispatch Truth — Review

## Review Status: **APPROVED WITH BLOCKERS**

This review evaluates the specification for the first slice of autodev execution path and dispatch truth work (Plan 003, Phase 0). The spec is well-researched and identifies real gaps. However, **security review reveals trust boundary concerns that must be addressed before implementation proceeds.**

---

## Summary Assessment

| Criterion | Rating | Evidence |
|-----------|--------|----------|
| Correctness | ✓ Good | Identifies 5 concrete gaps with code locations |
| Milestone Fit | ✓ Good | Aligns with Phase 0 gate (10 lanes, 0 bootstrap failures) |
| Remaining Blockers | △ Medium | Security gaps in trust boundaries; threading race conditions |

---

## Correctness Review

### Gap 1: Stale Lane Reclassification (Section 2.1)
**Verdict**: Correct diagnosis. The spec correctly identifies that `refresh_program_state` updates stale lanes to `Failed`, but `evaluate_program` re-classifies them as `Running` because `evaluate_lane` loads a fresh `run_snapshot` from an absent run directory.

**Proposed Fix Assessment**: The fix in `evaluate.rs` to check staleness when building `run_snapshot` is sound. However, the alternative fix (preferring runtime record over snapshot) is **riskier**—it creates a second source of truth that could diverge.

**Recommendation**: Implement the first fix (snapshot-level staleness detection) and add a consistency assertion that `runtime_record.status` and `run_snapshot.status` must agree after refresh.

### Gap 2: Runtime Path Validation (Section 2.2)
**Verdict**: Correct diagnosis. Pre-flight validation should catch missing graphs before dispatch consumes a slot.

**Concern**: The proposed `DispatchError::RuntimePathInvalid` variant needs careful error classification—is this a `TransientLaunchFailure` (retryable) or `PermanentSetupFailure` (requires operator intervention)?

### Gap 3: Evolve Blocking (Section 4)
**Verdict**: Correct diagnosis. Synchronous `synth evolve` blocks dispatch.

**Concern**: See Security Pass 2 (Coupled State) for race condition analysis.

### Gap 4: Telemetry Completeness (Section 3)
**Verdict**: Correct and necessary. The proposed `AutodevCycleReport` extensions are well-designed.

### Gap 5: Prompt Resolution (Section 1.2)
**Verdict**: Partial. The spec acknowledges three resolution strategies but does not mandate which one to implement.

**Risk**: Without a mandated strategy, different code paths may implement different resolutions, leading to inconsistent behavior.

---

## Milestone Fit Review

### Phase 0 Gate Alignment

The 180-Day Plan (genesis/plans/001-master-plan.md) defines the Phase 0 gate as:
> "On a proving-ground repo, `raspberry autodev --max-parallel 10` sustains 10 running lanes for at least 20 cycles, produces zero bootstrap-time validation failures caused by missing CLI subcommands or unresolved prompt/workflow refs, and lands at least 3 lanes to trunk."

**Alignment Check**:

| Gate Requirement | Spec Coverage | Verdict |
|------------------|---------------|---------|
| 10 lanes sustained | Section 2.3 (stale lanes don't consume slots) | ✓ Covered |
| 0 bootstrap failures | Sections 1.1, 1.2, 1.3 (runtime path validation) | ✓ Covered |
| 3 lanes to trunk | Indirect—requires correct execution | ✓ Implicit |

**Gap**: The spec does not explicitly define "bootstrap validation failure" telemetry. The acceptance criteria should specify that `runtime_path_errors` in the cycle report must remain empty for 20 consecutive cycles.

---

## Nemesis-Style Security Review

### Pass 1: First-Principles Challenge

#### Trust Boundaries

**Question**: Who can trigger the dangerous actions this slice enables?

| Action | Current Authority | Risk |
|--------|-------------------|------|
| Dispatch lane execution | `raspberry autodev` or `raspberry execute` | Low—requires local manifest access |
| Mark lane as stale/failed | `refresh_program_state()` internal | **Medium—no audit trail** |
| Trigger `synth evolve` | `should_trigger_evolve()` logic | **Medium—can mutate checked-in package** |
| Modify `program_state.json` | Any process with write access to `.raspberry/` | **High—no integrity checks** |

**Finding**: The spec does not address integrity of `.raspberry/*-state.json` files. A malicious or buggy process could:
1. Mark a completed lane as `Ready` to trigger re-execution
2. Clear `failed` status to bypass failure classification
3. Modify `last_evolve_at` to force or skip evolution

**Recommendation**: Add a section on state file integrity—at minimum, document that `.raspberry/` should be writable only by the autodev operator user.

#### Authority Assumptions

**Question**: What authority does the autodev loop assume it has?

The spec assumes:
- Write access to `~/.fabro/runs/<run-id>/` (run directories)
- Write access to `.raspberry/*-state.json` (program state)
- Execute permission on `fabro_bin` (CLI binary)
- Read access to target repo

**Missing**: The spec does not specify what happens when any of these assumptions are violated. The error handling should be explicit:
- Permission denied on state file → `PermanentSetupFailure` with operator alert
- `fabro_bin` not executable → `TransientLaunchFailure` (may be temporary build issue)
- Run directory not creatable → `PermanentSetupFailure`

#### Secret Handling

**Question**: Are secrets exposed in the new telemetry fields?**

Review of proposed `RuntimePathError`:
```rust
pub struct RuntimePathError {
    pub lane_key: String,
    pub error_type: String,
    pub message: String,
}
```

**Risk**: If `message` contains:
- File paths that include home directory (PII)
- Environment variable dumps
- Command-line arguments with tokens

**Recommendation**: Add a requirement that `RuntimePathError.message` must be redacted via `fabro-util::redaction` before serialization.

### Pass 2: Coupled-State Review

#### Paired State Surfaces

**Paired Surface 1: `program_state.json` ↔ `~/.fabro/runs/<run-id>/`**

These two state surfaces must agree:
- `program_state` records which run ID is current for a lane
- Run directory contains the actual run state

**Inconsistency Scenario**:
1. `refresh_program_state` detects stale run, updates record to `Failed`
2. Before `program_state.json` is written, process crashes
3. On restart, `program_state` still shows `Running`, run directory is gone
4. `evaluate_program` loads fresh snapshot from absent directory → reclassifies as `Running`

**Mitigation in Spec**: The spec's Gap 1 fix (snapshot-level staleness detection) helps, but **does not eliminate the window** between state file write and evaluation.

**Recommendation**: Add idempotency requirement—`evaluate_program` must produce the same classification if run twice with the same inputs, even if state file is mid-write.

**Paired Surface 2: `last_evolve_at` ↔ `malinka/` package content**

When `synth evolve` runs in a background thread (Gap 3 fix):
- Main thread sets `last_evolve_at = Some(now)` optimistically
- Evolve thread mutates `malinka/` files
- If evolve fails, `malinka/` is in partially-evolved state

**Race Condition**:
```
Cycle N:   Evolve starts, sets last_evolve_at
Cycle N+1: Dispatch uses evolved package (may be partial)
Cycle N+2: Evolve fails, package reverted (or not)
```

**Finding**: The spec's "Option 1 (thread-based)" approach has a **serious consistency issue**—the next cycle may see partially-evolved package state.

**Recommendation**: Change the threading model:
- Option A: Evolve to temp directory, atomic rename on success
- Option B: Evolve produces a manifest hash; dispatch validates hash before using
- Option C: Block only on first evolve (package creation), not subsequent evolves

**Paired Surface 3: `dispatch_summary` ↔ actual dispatch history**

Rolling statistics in `AutodevCurrentSnapshot.dispatch_summary` are derived state:
```rust
pub struct DispatchSummary {
    pub cycles_with_dispatch: usize,
    pub idle_cycles: usize,
    pub total_dispatched: usize,
    pub failed_bootstrap: usize,
    pub stale_running_reclaimed: usize,
}
```

**Risk**: If the autodev process restarts, these counters reset to zero. An operator monitoring `idle_cycles` would see a sudden drop, misinterpreting it as recovery.

**Recommendation**: Either:
1. Persist `dispatch_summary` to `.raspberry/dispatch-summary.json` across restarts
2. Change to timestamp-based metrics ("idle cycles in last hour") that are recomputed from run history
3. Document that these are per-session metrics, not durable truth

#### Idempotence and Pairing Behavior

**Question**: Is every mutation path idempotent?**

| Mutation | Idempotent? | Evidence |
|----------|-------------|----------|
| `refresh_program_state` → stale lane to Failed | **No** | Sets `last_finished_at = Some(now)`; re-running would set different timestamp |
| `run_fabro` → spawn worker | **Yes** | Uses UUID-based run ID; duplicate spawns would have different IDs |
| `stale_failure_superseded_by_render` → reset to Blocked | **Yes** | Deterministic check of file mtime vs `last_finished_at` |

**Concern**: Non-idempotent `last_finished_at` assignment means crash-recovery may see different timestamps for the same logical event. This affects:
- `stale_failure_superseded_by_render` logic (compares mtimes)
- Human auditing of when failures occurred

**Recommendation**: Consider using the run directory's deletion timestamp (if available) or a deterministic timestamp based on the state file's own mtime.

#### Privilege Escalation Paths

**Question**: Can this slice enable privilege escalation?**

**Path 1: Symlink Attack on Run Directory**

The spec mentions eliminating "local-only shims" including symlinks. However, the autodev loop creates directories under `~/.fabro/runs/<run-id>/`. If an attacker can:
1. Predict or control the run ID
2. Create a symlink at that path pointing to a sensitive directory
3. The autodev loop writes worker output to that path

**Mitigation**: The spec does not address run ID generation. UUIDv4 (random) is safer than predictable sequences.

**Path 2: Command Injection via `fabro_bin` Path**

`AutodevSettings.fabro_bin` is a path string passed to `Command::new`. If this path contains shell metacharacters, it could execute unintended commands.

**Mitigation**: The spec should require that `fabro_bin` is validated as an absolute path to an executable file before use.

---

## Remaining Blockers

### Blocker 1: Security — State File Integrity
**Severity**: High
**Requirement**: Add integrity requirements for `.raspberry/*-state.json`:
- Document permission model (owner-write-only)
- Add optional checksum or consider append-only log structure
- Validate state file on load, fail closed on corruption

### Blocker 2: Security — Telemetry Redaction
**Severity**: Medium
**Requirement**: Mandate that `RuntimePathError.message` is redacted before serialization to prevent accidental secret exposure.

### Blocker 3: Correctness — Evolve Race Condition
**Severity**: High
**Requirement**: Revise Gap 3 fix to use atomic package updates (temp directory + rename) rather than in-place mutation.

### Blocker 4: Correctness — Dispatch Summary Durability
**Severity**: Low
**Requirement**: Clarify whether `dispatch_summary` is per-session or durable. If per-session, document this explicitly. If durable, specify persistence mechanism.

### Blocker 5: Completeness — Bootstrap Failure Definition
**Severity**: Low
**Requirement**: Add explicit definition of "bootstrap validation failure" to acceptance criteria, with telemetry field that must remain zero for gate success.

---

## Recommendations for Implementation Order

1. **Gap 1** (stale lane reclassification) + **Blocker 3** (evolve atomicity) — These fix slot consumption and package consistency
2. **Blocker 2** (telemetry redaction) — Security hygiene
3. **Gap 4** (telemetry fields) — Enables observability for gate validation
4. **Gap 2** (pre-flight validation) — Reduces bootstrap failures
5. **Gap 5** (prompt resolution) — Polish, can be deferred if temporary symlink works
6. **Blocker 1** (state integrity) — Documentation and validation, can be post-Phase 0

---

## Open Questions from Spec

| Question | Reviewer Response |
|----------|-------------------|
| Priority dispatch vs. family diversity? | Document that `ready_undispatched` includes diversity-skipped lanes; no change needed |
| Evolve blocking on first cycle? | Acceptable for Phase 0; first-cycle latency is not the gate metric |
| Stale grace period (30s vs 0s)? | 30s is conservative; consider 10s for faster slot recovery without risking slow-start misclassification |

---

## Conclusion

The specification is technically sound and well-aligned with the Phase 0 gate. The security review identified **trust boundary and coupled-state issues** that should be addressed before or during implementation:

1. State file integrity and permission model
2. Telemetry redaction requirements
3. Atomic evolve updates to prevent race conditions

**Approval**: The spec is approved for implementation with the blockers noted above. The implementation PR should include:
- Evidence that `runtime_path_errors` remains empty for 20 cycles on rXMRbro
- Evidence that stale lanes are correctly reclassified (unit test)
- Evidence that evolve does not block dispatch (timing logs)
- Security review sign-off on telemetry and state handling
