# Autodev Execution Path and Dispatch Truth — Lane Review

**Lane:** `autodev-efficiency-and-dispatch`  
**Date:** 2026-03-26  
**Stage:** Polish — Supervisory Plane Preparation  
**Review type:** Spec alignment + implementation readiness + nemesis-style first-principles challenge

---

## Executive Summary

The spec correctly identifies five failure modes blocking reliable autodev execution. Four of five are confirmed implementation gaps requiring code changes; one (stale running detection) already has working logic that needs hardening.

**Verdict:** Spec is accurate and implementable. Four blockers must be resolved before live validation can succeed.

---

## Spec Correctness Assessment

### Claims Verified Against Source

| Claim in Spec | Source Location | Status |
|---------------|-----------------|--------|
| `STALE_RUNNING_GRACE_SECS = 30` | `program_state.rs:22` | ✅ Confirmed |
| `stale_active_progress_reason()` exists | `program_state.rs:1429` | ✅ Confirmed |
| `@../../prompts/` path in render.rs | `fabro-synthesis/src/render.rs:1915` | ✅ Confirmed |
| `AutodevCycleReport` missing telemetry fields | `autodev.rs:96` | ✅ Confirmed — fields absent |
| `should_trigger_evolve()` exists | `autodev.rs:895` | ✅ Confirmed |
| `run_synth_evolve()` blocks dispatch | `autodev.rs:2156-2208` | ✅ Confirmed |
| `DispatchOutcome` lacks error classification | `dispatch.rs:26` | ✅ Confirmed |

**Assessment:** Spec accurately reflects the current implementation state. All line number references are accurate.

---

## Implementation Readiness

### Blocking Issues (Must Fix Before Live Validation)

| # | Issue | Severity | Fix Location | Risk if Not Fixed |
|---|-------|----------|--------------|-------------------|
| 1 | Missing command surface validation | HIGH | `autodev.rs` (new startup probe) | Silent failures on clean builds |
| 2 | Prompt path resolution wrong | HIGH | `fabro-synthesis/src/render.rs:1915` | Every dispatched lane fails bootstrap |
| 3 | Blocking evolve on hot path | MEDIUM | `autodev.rs:2156-2208` | Dispatch delays under load |
| 4 | Dispatch telemetry gaps | MEDIUM | `autodev.rs:96` + new struct | Operational blindness |

### Non-Blocking Issues (Post-Milestone)

| # | Issue | Severity | Note |
|---|-------|----------|------|
| 5 | `worker_alive = None` = infinite stale window | LOW | After 60s force re-check |
| 6 | Doctrine fingerprint uses mtime only | LOW | Add content hash |
| 7 | Maintenance mode fail-open | LOW | Fail-closed on error |
| 8 | Binary provenance blind trust | MEDIUM | Add SHA-256 validation at startup |

---

## Nemesis Security Review — First-Principles Challenge

### Trust Boundaries

**1. Binary Provenance Blind Trust**  
The controller captures the fabro binary path from settings but never validates binary integrity. If an attacker replaces `fabro` on disk between cycles, subsequent synth/evolve operations execute attacker-controlled code.

**Fix:** Add SHA-256 hash validation at startup alongside command surface probe.

**2. Target Repo Privilege Escalation**  
`synth evolve` runs with full user environment inherited. A malicious `target_repo/.cargo/config.toml` can inject arbitrary code into synthesis.

**Fix:** Document as known limitation. Consider running synth evolve with sanitized environment.

**3. Doctrine File Fingerprinting**  
`doctrine_inputs_changed()` uses mtime + len, not content hash. An attacker can modify doctrine file content without changing mtime to trigger spurious regenerations.

**Fix:** Add content hash to `DoctrineFileFingerprint`.

**4. Maintenance Mode Fail-Open**  
If `load_active_maintenance()` returns an error (not `Ok(Some(_))`), execution continues normally instead of stopping.

**Fix:** Fail-closed: stop execution on maintenance load errors.

---

## Acceptance Criteria Fitness

| # | Criterion | Current Status | Achievable |
|---|-----------|----------------|-------------|
| 1 | `fabro synth --help` in both builds | Untested | ✅ With startup probe |
| 2 | `fabro run --detach` with prompt refs | Fails | ✅ Fix render.rs path |
| 3 | Stale running → failed in 30s | Partial | ⚠️ Needs `None` hardening |
| 4 | Dispatch telemetry fields | Missing | ✅ Add struct fields |
| 5 | 10 lanes sustained 20 cycles | Not tested | ⚠️ Requires fixes 1, 2, 4 |
| 6 | Zero bootstrap failures first 10 cycles | Fails | ⚠️ Requires fixes 1, 2 |

---

## Recommendations

### Immediate (Before Live Validation)

1. **Fix render.rs path resolution** — Highest impact blocker. Replace `@../../prompts/` with stable run-directory-relative or absolute paths.
2. **Add command surface validation** — Fail fast with actionable error if `fabro synth --help` fails.
3. **Run 5-lane proving test** — Before attempting 10-lane validation, verify fixes work at smaller scale.

### Short-Term (Phase 0 Completion)

4. **Add dispatch telemetry** — Required for operational visibility. All six fields needed.
5. **Decouple evolve from dispatch** — Move to background thread with cadence gating.
6. **Harden stale running detection** — Force reclassification after 60s when `worker_alive = None`.

### Security Hardening (Post-Phase 0)

7. **Add binary integrity check** — SHA-256 of fabro binary at startup.
8. **Fail-closed maintenance mode** — Stop on maintenance load errors.
9. **Content-hash doctrine files** — Prevent mtime-only manipulation.

---

## Verification Commands

```bash
# Verify command surface
cargo build --release -p fabro-cli
./target/release/fabro synth --help
./target/release/fabro run --help

# Run unit tests
cargo nextest run -p raspberry-supervisor -- autodev
cargo nextest run -p fabro-synthesis -- render

# Live validation (after fixes)
cargo build --release -p fabro-cli -p raspberry-cli
./target/release/raspberry autodev \
  --manifest /path/to/rxmragent.yaml \
  --max-parallel 10 --max-cycles 20
```

---

## Status

**Spec:** ✅ Correct and implementable  
**Review:** ✅ Complete  
**Implementation:** ⏳ Pending — four blockers identified  

**Next step:** Implement Issue 2 (prompt path resolution) and Issue 1 (command validation), then run 5-lane proving test before attempting full 10-lane validation.
