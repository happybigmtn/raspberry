# Autodev Execution Path and Dispatch Truth — Review

## Review Status: APPROVED WITH REQUIRED FIXES

**Spec**: `outputs/autodev-efficiency-and-dispatch/spec.md`
**Phase 0 gate alignment**: Plan 003, Phase 0 stabilization
**Reviewer**: Supervisory plane
**Date**: 2026-03-26

---

## Summary

The spec identifies real gaps in the autodev execution path. Three blockers must be resolved before implementation; the remaining items are recommended improvements. Phase 0 gate success requires evidence of all five acceptance criteria.

| Category | Finding | Severity |
|----------|---------|----------|
| Correctness | Stale lane reclassification has a state-file durability window | Required |
| Correctness | Evolve in background thread risks partial package state | Required |
| Security | Telemetry `message` field may leak file paths or env vars | Required |
| Completeness | Bootstrap failure definition is implicit, not explicit | Recommended |
| Correctness | `dispatch_summary` is per-session but not documented as such | Recommended |

---

## Gate Alignment

The Phase 0 gate requires:
> On a proving-ground repo, `raspberry autodev --max-parallel 10` sustains 10 running lanes for at least 20 cycles, produces zero bootstrap-time validation failures caused by missing CLI subcommands or unresolved prompt/workflow refs, and lands at least 3 lanes to trunk.

| Gate Requirement | Spec Coverage | Status |
|-----------------|---------------|--------|
| 10 lanes sustained for 20 cycles | Section 2.3 (stale slots) | ✓ Covered |
| 0 bootstrap failures from runtime path | Sections 1.1–1.3 | ✓ Covered |
| 3 lanes to trunk | Indirect (requires correct execution) | ✓ Implicit |

**Gap**: "bootstrap validation failure" is not explicitly defined. The acceptance criteria should require `runtime_path_errors` to remain empty for 20 consecutive cycles.

---

## Correctness Review

### Finding 1: Stale Lane Reclassification — State-File Durability Window

**Location**: `program_state.rs:383` (`refresh_program_state`), `evaluate.rs:1021` (`is_active`)

**Analysis**: `refresh_program_state` correctly updates stale `Running` lanes to `Failed`. However, if the process crashes between the in-memory update and the fsync of `program_state.json`, the on-disk state still shows `Running`. On restart, `is_active()` falls through to the `runtime_record` alternative, which would return `true` if the record still shows `Running`.

**Severity**: Medium. The crash window is small (single write), and the consequence is a momentary slot consumption error, not data loss.

**Required fix**: Add an idempotency property — `evaluate_program` must produce the same classification if run twice with the same inputs, even if state file is mid-write. One approach: after marking a lane `Failed` due to staleness, record the detection timestamp in the state file atomically, so a subsequent load can distinguish "never checked" from "checked and found stale."

### Finding 2: Evolve Background Thread — Partial Package State Risk

**Location**: `autodev.rs:2333` (`run_synth_evolve`), Section 4 of spec

**Analysis**: Running `run_synth_evolve` in a spawned thread without atomic package updates creates a window where `malinka/` contains partially-evolved content. A dispatch that starts during this window could use an inconsistent package.

**Severity**: High. Inconsistent package state can cause non-deterministic lane outcomes.

**Required fix**: Use temp directory + atomic rename for evolve output. Alternatively, have evolve produce a manifest hash; dispatch validates the hash before using the package.

### Finding 3: Greedy Dispatch — Selection Exhaustion Not Propagated

**Location**: `autodev.rs:551` (`select_ready_lanes_for_dispatch`), Section 3.1

**Analysis**: When `select_ready_lanes_for_dispatch` returns fewer lanes than `available_slots` due to family-diversity constraints, the telemetry does not capture `SelectionExhausted { candidates: N }`.

**Severity**: Low. The spec defines the fix correctly; the finding is that the implementation must wire it through.

---

## Security Review

### Finding 4: Telemetry Message Field — Potential Information Leak

**Location**: Proposed `RuntimePathError.message` field

**Analysis**: If `message` contains file paths (which include home directory paths on Linux), environment variable dumps, or command-line arguments, these could leak into telemetry outputs.

**Severity**: Medium.

**Required fix**: All `RuntimePathError.message` values must be processed through `fabro-util::redaction` before serialization. Specifically:
- Home directory paths must be redacted to `~`
- Environment variable contents must be redacted unless the variable name is on an explicit allow-list
- Command-line arguments must not include secret tokens

### Finding 5: State File Permission Model — Not Documented

**Location**: `.raspberry/*-state.json`

**Analysis**: The spec does not address who can write to state files. A malicious or buggy process could:
- Mark a completed lane as `Ready` to trigger re-execution
- Clear `failed` status to bypass failure classification
- Modify `last_finished_at` to break `stale_failure_superseded_by_render` logic

**Severity**: Medium (for multi-user systems; low for single-operator use).

**Recommended fix**: Document that `.raspberry/` must be writable only by the autodev operator user. Consider adding file permission checks or optional checksums for integrity validation in a follow-up plan.

---

## Completeness Review

### Finding 6: Bootstrap Failure Definition — Implicit Not Explicit

**Location**: Acceptance criteria

**Analysis**: The gate criterion says "zero bootstrap-time validation failures caused by missing CLI subcommands or unresolved prompt/workflow refs." The spec defines `runtime_path_errors` but does not explicitly require it to be empty for 20 consecutive cycles.

**Recommended fix**: Add to acceptance criterion #8:
> `runtime_path_errors` in `AutodevReport` JSON is empty for all 20 consecutive cycles.

### Finding 7: `dispatch_summary` Per-Session vs. Durable — Undocumented

**Location**: Section 3.2

**Analysis**: If the autodev process restarts, `DispatchSummary` counters reset to zero. An operator monitoring `idle_cycles` would see a sudden drop, potentially misinterpreting it as recovery.

**Recommended fix**: Document explicitly: "These are per-session counters. On process restart, all counters reset to zero."

---

## Implementation Order

| Priority | Item | Rationale |
|----------|------|-----------|
| 1 | Finding 2 (evolve atomicity) | Prevents inconsistent package state in background evolve |
| 2 | Finding 4 (telemetry redaction) | Security hygiene before live telemetry |
| 3 | Section 2.1 (stale reclassification) | Fixes slot consumption accuracy |
| 4 | Section 3.1 (telemetry fields) | Enables gate validation observability |
| 5 | Section 1.3 (pre-flight validation) | Reduces bootstrap failures |
| 6 | Section 1.2 (prompt resolution) | Can be deferred if symlink workaround holds |
| 7 | Finding 1 (state durability window) | Low-probability crash scenario |
| 8 | Finding 5 (state file permissions) | Documentation, post-Phase 0 |

---

## Required Evidence for Phase 0 Gate

Implementation PRs must include:

1. `runtime_path_errors` remains empty for 20 consecutive cycles on rXMRbro (JSON report inspection)
2. Lane with deleted run directory is reported as `failed` in `raspberry status` (unit test or manual verification)
3. Stale `Running` lanes do not reduce `available_slots` (derived from `raspberry status` counts before/after)
4. Evolve does not block dispatch — dispatch occurs in same cycle as evolve trigger (timing logs showing dispatch timestamp vs evolve completion timestamp)
5. `raspberry status` shows dispatch summary fields (human-readable output inspection)

---

## Open Questions — Resolved

| Question | Resolution |
|----------|------------|
| Priority dispatch vs. family diversity? | Document that `ready_undispatched` includes diversity-skipped lanes. No code change needed. |
| Evolve blocking on first cycle? | Acceptable for Phase 0. First-cycle latency is not the gate metric. |
| Stale grace period (30s vs 0s)? | Recommend 10s for faster slot recovery. 30s is conservative but acceptable. |

---

## Conclusion

The spec is approved for implementation. The required fixes (Findings 1, 2, 4) must be addressed in the implementation PR. The recommended improvements (Findings 3, 6, 7) should be addressed where convenient but do not block Phase 0 gate passage.
