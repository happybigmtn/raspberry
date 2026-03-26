# Provider Policy Stabilization — Security Review

**Review Date:** 2026-03-26  
**Auditor:** Genesis (automated audit + manual verification)  
**Spec Version:** outputs/provider-policy-stabilization/spec.md

---

## Executive Summary

This review validates the specification for provider policy stabilization against the live codebase. The **spec is sound** but the **implementation gap is critical**: model selection leaks in `lib/crates/fabro-synthesis/src/render.rs` bypass the central policy entirely, creating authority drift that could route high-value review work to inappropriate models during failover scenarios.

**Verdict:** The specification correctly identifies the problem. The implementation work is scoped appropriately for Phase 0. No spec blockers.

---

## Nemesis-Style Security Review

### Pass 1 — First-Principles Challenge

**Question:** Who can trigger provider selection, and what trust boundaries are violated?

**Findings:**

1. **Authority Drift in Synthesis Layer** (`lib/crates/fabro-synthesis/src/render.rs:2058-2080`)
   - The `recurring_report_primary_target_for_lane()` function (line 2058) constructs `ModelTarget` directly using `Provider::Minimax`, `Provider::Anthropic`, and `Provider::OpenAi`
   - **Trust boundary violation:** The synthesis layer (which generates workflow graphs) is making provider selection decisions that should belong exclusively to the policy layer
   - **Attack surface:** Any compromise or bug in synthesis graph generation can force specific providers regardless of quota state or operator policy changes
   - **Privilege assumption:** The code assumes MiniMax is appropriate for holistic review minimax lanes, but this is not validated against the actual policy chain for the `Review` profile

2. **CLI Backend String-Matching for Quota Detection** (`lib/crates/fabro-workflows/src/backend/cli.rs:799-817`)
   - `cli_failure_is_retryable_for_fallback()` uses substring matching: `"you've hit your limit"`, `"usage limit has been reached"`
   - **Trust boundary violation:** Error message text is not a stable interface. Provider CLI tools can change error messages without API versioning.
   - **Authority assumption:** The CLI backend assumes it can classify errors without consulting the SDK's structured error types (`ProviderErrorKind::QuotaExceeded`)

3. **No Authentication of Provider Health** (structural gap)
   - The system lacks cryptographic or attestation-based verification that a provider is healthy
   - **Assumption:** Provider health is determined by recent success/failure heuristics, which an attacker could manipulate by causing selective network partitions

### Pass 2 — Coupled-State Review

**Question:** What paired state must remain consistent, and do mutation paths preserve consistency?

**Findings:**

1. **Policy Chain vs. Synthesis Target Coupling**
   - **Paired state:** `policy.rs` chains (`WRITE_CHAIN`, `REVIEW_CHAIN`, `SYNTH_CHAIN`) and `render.rs` lane target assignments
   - **Inconsistency path:** When `policy.rs` is updated (e.g., swapping Kimi and MiniMax order in `REVIEW_CHAIN`), `render.rs` hardcoded selections become **stale authorities**
   - **Current behavior:** The hardcoded `Provider::Minimax` for holistic review minimax lanes does NOT match the `REVIEW_CHAIN` ordering (Kimi first, MiniMax second)
   - **Risk:** This creates a **split-brain** scenario where the policy says one thing but synthesis does another

2. **Quota State vs. Fallback Eligibility Coupling**
   - **Paired state:** Provider quota exhaustion state and fallback chain traversal
   - **Correct behavior (API path):** `sdk_err.failover_eligible()` returns `true` for `QuotaExceeded` (`fabro-llm/src/error.rs:216-221`), triggering chain traversal
   - **Incorrect behavior (CLI path):** String matching in `cli_failure_is_retryable_for_fallback()` (line 799) may miss quota errors that don't match the specific phrases
   - **State mutation path:** When provider A reports quota exhaustion, the system must:
     1. Not consume retry budget (correct in both paths)
     2. Advance to provider B in chain (correct in API path, potentially incorrect in CLI path)
     3. Record provider A as quota-limited for health status (not yet implemented)

3. **Usage Tracking vs. Cost Attribution Coupling**
   - **Paired state:** Per-provider token counts and per-lane cost attribution
   - **Gap:** `AutodevCycleReport` (`lib/crates/raspberry-supervisor/src/autodev.rs:96`) lacks `usage_by_provider`, meaning cost attribution cannot be reconciled
   - **Risk:** Operator cannot verify that quota exhaustion predictions match actual usage

4. **Lane State vs. Provider Assignment Coupling**
   - **Paired state:** Lane execution state and the provider/model currently processing the lane
   - **Gap:** `AutodevCurrentSnapshot` (`lib/crates/raspberry-supervisor/src/autodev.rs:70`) does not include per-lane provider/model information
   - **Risk:** When a lane fails, operators cannot determine if the failure is provider-specific without manual log inspection

---

## Detailed Verification

### AC1: Zero Model Selection Leaks

**Verification command:**
```bash
grep -rn "Provider::(Minimax|Anthropic|OpenAi|Kimi|Gemini)" \
  lib/crates/fabro-synthesis/src/ \
  lib/crates/fabro-workflows/src/ \
  --include="*.rs" \
  | grep -v "_test\|#\[test\]\|// \|use \|mod tests" \
  | grep -v "policy.rs\|provider.rs"
```

**Results:**
- **CRITICAL:** `lib/crates/fabro-synthesis/src/render.rs:2058-2080` — `recurring_report_primary_target_for_lane()` hardcodes providers at lines 2061, 2067, 2075
- **CRITICAL:** `lib/crates/fabro-synthesis/src/render.rs:2495-2499` — `Provider::OpenAi` hardcoded in model assignment
- **Test-only:** `lib/crates/fabro-workflows/tests/` — fixtures using providers (acceptable)

**Verdict:** Spec correctly identifies the leak. AC1 will require refactoring `render.rs` to call `automation_primary_target(AutomationProfile::Review)` instead of constructing `ModelTarget` directly.

### AC2: Quota Exhaustion Triggers Fallback

**Verification:**

1. **API path** (`lib/crates/fabro-workflows/src/backend/api.rs:367-407`):
   - Uses `sdk_err.failover_eligible()` at lines 367 and 407
   - **Correct:** Properly triggers fallback chain traversal

2. **CLI path** (`lib/crates/fabro-workflows/src/backend/cli.rs:799-817`):
   - `cli_failure_is_retryable_for_fallback()` checks for `"you've hit your limit"` (line 805) and `"usage limit has been reached"` (line 806)
   - **Missing:** Does not check for `"quota"` or `"quota exceeded"` explicitly
   - **Gap:** The fabro-llm SDK correctly classifies quota errors, but CLI path doesn't use SDK error classification

3. **SDK layer** (`lib/crates/fabro-llm/src/error.rs:212-221`):
   - `failover_eligible()` returns `true` for `QuotaExceeded`
   - **Correct:** As specified in spec

**Verdict:** Spec correctly identifies the CLI gap. AC2 requires adding `"quota"` to the retryable patterns in `cli_failure_is_retryable_for_fallback()`.

### AC3: Provider Health in Status

**Verification:**
- `AutodevCurrentSnapshot` (`lib/crates/raspberry-supervisor/src/autodev.rs:70-93`) lacks:
  - Per-lane model/provider fields
  - Provider health status enumeration (`healthy`, `quota_limited`, `unavailable`)
- `render_status_table()` (`lib/crates/raspberry-supervisor/src/evaluate.rs:410`) outputs lane status but not provider info

**Verdict:** Spec correctly scopes the work. AC3 requires extending the snapshot struct and status renderer.

### AC4: Usage Tracking in Report

**Verification:**
- `AutodevCycleReport` (`lib/crates/raspberry-supervisor/src/autodev.rs:96-111`) lacks `usage_by_provider` field
- Usage is tracked at the stage level but not aggregated per provider per cycle

**Verdict:** Spec correctly scopes the work. AC4 requires adding aggregation logic.

---

## State Transition Analysis

### Provider State Machine

```
Healthy ──[quota error]──> QuotaLimited ──[time passes]──> Healthy
    │                            │
    │                            └── status shows "quota_limited"
    │                            └── new lanes skip this provider
    │
    └──[other error]──> Unavailable ──[health check/recovery]──> Healthy
```

**Safety properties:**
1. **No work loss:** When provider transitions to `QuotaLimited`, in-flight work continues but new work routes to fallback
2. **No infinite loop:** Fallback chain has finite length; exhaustion results in hard failure (acceptable)
3. **Eventual recovery:** Quota state is not persisted; next cycle re-evaluates from `Healthy`

### Lane State Machine with Provider Context

```
Ready ──[dispatch]──> Running(provider=A, model=X)
                           │
                           ├──[success]──> Complete
                           ├──[quota error]──> Running(provider=B, model=Y)  [fallback]
                           └──[hard failure]──> Failed
```

**Invariant:** Lane state transitions must log provider context for debugging.

---

## Secret Handling Review

**Question:** Are API keys and credentials handled safely during provider failover?

**Findings:**
1. API keys are passed via environment variables to CLI subprocesses (verified in `cli.rs:244-300`)
2. No logging of API keys in fallback path (verified — tracing only logs provider/model names)
3. **No issue:** Secrets are not part of the policy state being stabilized

---

## Idempotence and Retry Safety

**Question:** Are provider fallback operations idempotent?

**Findings:**
1. **Retry count consumption:** Spec correctly states quota exhaustion should not consume retry count
2. **Idempotence:** Each provider attempt is independent; no shared mutable state between attempts
3. **Safety:** LLM calls are inherently non-idempotent (non-deterministic), but this is acceptable for the use case

---

## External-Process Control

**Question:** Are CLI provider processes safely managed during failover?

**Findings:**
1. `cleanup_cli_scratch()` is called before falling back (verified in `cli.rs:1820-1840`)
2. Process cleanup uses PID tracking (verified)
3. **No orphan process risk:** Cleanup happens in the `continue` path before attempting next provider

---

## Summary of Blockers

| Blocker | Severity | Location | Resolution |
|---------|----------|----------|------------|
| Hardcoded model selection in `recurring_report_primary_target_for_lane` | **Critical** | `lib/crates/fabro-synthesis/src/render.rs:2058-2080` | Replace with `automation_primary_target()` calls |
| Hardcoded model selection at line 2497 | **Critical** | `lib/crates/fabro-synthesis/src/render.rs:2497` | Replace with policy-based resolution |
| CLI quota detection gap | Medium | `lib/crates/fabro-workflows/src/backend/cli.rs:799-817` | Add `"quota"` to retryable patterns |
| Missing provider health fields | Medium | `lib/crates/raspberry-supervisor/src/autodev.rs:70-93` | Extend `AutodevCurrentSnapshot` |
| Missing usage tracking | Medium | `lib/crates/raspberry-supervisor/src/autodev.rs:96-111` | Add `usage_by_provider` to `AutodevCycleReport` |

---

## Recommendations

1. **Immediate (Phase 1):** Fix the hardcoded model selection in `render.rs`. This is the only critical blocker.

2. **Before Phase 2:** Fix CLI quota detection to ensure parity with API path.

3. **Phase 2 scope:** Provider health and usage tracking are appropriately scoped for the next slice.

4. **Testing:** Add a test that artificially exhausts quota (via mock provider) and verifies fallback without lane failure.

---

## Sign-off

| Criterion | Status |
|-----------|--------|
| Spec correctness | ✅ Sound — correctly identifies problems and proposes viable solutions |
| Milestone fit | ✅ Appropriate for Phase 0 (stabilization) |
| Implementation accuracy | ⚠️ Line numbers updated to match live codebase |
| Remaining blockers | 2 critical, 3 medium — all scoped and documented |

**Recommendation:** Approve spec. Proceed to implementation with priority on `render.rs` refactoring.
