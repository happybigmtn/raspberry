# Provider Policy Stabilization — Lane Review

**Lane:** `provider-policy-stabilization`  
**Spec:** `outputs/provider-policy-stabilization/spec.md`  
**Plan:** `genesis/plans/008-provider-policy-stabilization` (referenced in 001-master-plan.md)  
**Review Date:** 2026-03-26  
**Review Stage:** First honest reviewed slice — REVIEW gate  

---

## Executive Summary

The specification presents a coherent capability definition for centralizing model routing through `fabro_model::policy::automation_chain()`. The review validates the spec's claims through direct source inspection. **Milestone 1 findings are confirmed: there are hardcoded model string leaks in `fabro-synthesis/src/render.rs` that violate the policy centralization invariant.** Milestones 2-4 identify legitimate gaps requiring implementation. The spec is **fit for implementation** with noted risks requiring mitigation.

---

## Nemesis-Style Security Review

### Pass 1 — First-Principles Challenge

#### Trust Boundaries and Authority Assumptions

| Boundary | Current State | Risk |
|----------|---------------|------|
| **Policy.rs as source of truth** | Validated clean. All model strings are `&'static str` constants. | Low — the module exports only immutable functions |
| **Synthesis → render.rs** | **BROKEN**. Functions construct `ModelTarget` literals bypassing policy.rs | **High** — workflow graphs embed hardcoded providers/models |
| **CLI backend → cli.rs** | Uses `automation_fallback_targets()` correctly for chain execution | Low — properly delegates to policy |
| **Quota detection** | String matching on stderr ("you've hit your limit", "usage limit has been reached") | Medium — fragile, relies on provider message stability |
| **Provider credentials** | Inherited from host environment via `provider.api_key_env_vars()` | Medium — no rotation or validation of key scopes |

**Authority Challenge**: Who can alter which model runs where?
- The `automation_chain()` function returns a static slice — compile-time fixed
- No runtime policy override mechanism exists (feature or bug?)
- Lane ID string patterns (`-holistic-review-minimax`, `-codex-unblock`) **override** the policy chain via the leak functions in render.rs

**Attack Surface**: A compromised lane ID suffix could force routing to an unintended provider:
```rust
// In render.rs — lane ID patterns override policy
if lane.id.ends_with("-holistic-review-minimax") {
    return Some(ModelTarget { provider: Provider::Minimax, model: "MiniMax-M2.7-highspeed" });
}
```
This is not a security vulnerability per se (all providers are trusted), but it **violates the policy abstraction** and could cause:
1. Cost surprises (routing to expensive Opus when Minimax was policy-intended)
2. Capability floor violations (MiniMax reviewing security-critical code meant for Opus)
3. Quota exhaustion on the wrong provider

#### Dangerous Actions and Trigger Points

| Dangerous Action | Who Can Trigger | Current Control |
|------------------|-----------------|-----------------|
| Provider fallback chain exhaustion | Any quota-exhausted lane run | `cli_failure_is_retryable_for_fallback()` decides; returns true for quota signals |
| Routing to expensive provider (Anthropic Opus) | Any `-holistic-review-deep` lane | Hardcoded in render.rs, bypasses policy cost ordering |
| Model capability floor bypass | Codex-unblock lanes | Hardcoded `gpt-5.4` usage bypasses Review chain ordering |

---

### Pass 2 — Coupled-State Review

#### Paired State Surfaces

| State A | State B | Consistency Check |
|---------|---------|-------------------|
| `provider_used.json` (written by CLI backend) | `DispatchOutcome` (returned to supervisor) | **GAPPED**. `DispatchOutcome` has no provider/model fields. The JSON file exists but isn't ingested into supervisor state. |
| `fabro-llm::ProviderErrorKind::QuotaExceeded` | `cli_failure_is_retryable_for_fallback()` | **DECOUPLED**. CLI path uses string matching; API path uses structured error. These can drift if providers change error messages. |
| Policy chain ordering (Write: Minimax→Kimi→Opus) | Lane runtime routing | **VIOLATED**. `recurring_report_primary_target_for_lane()` ignores policy for specific lane suffixes. |
| `AutodevCycleReport.cycles` | `last_usage_summary` (per lane) | **INCOMPLETE**. Usage is freeform string, no per-provider breakdown. |

#### State Transitions Affecting Safety

**Quota Exhaustion Transition:**
```
Lane running on Provider P → Quota hit → Fallback to P' → Success
                    ↓                ↓                  ↓
              Current: cli_failure_is_retryable_for_fallback()
                               returns true for substring match
              Desired: Structured QuotaExceeded classification
```

**Risk**: The conflation of quota errors with auth errors and timeouts in `cli_failure_is_retryable_for_fallback()` means a 401 Unauthorized could incorrectly advance the fallback chain, masking a credential problem.

```rust
// Current implementation — quota conflated with auth
fn cli_failure_is_retryable_for_fallback(detail: &str) -> bool {
    lower.contains("you've hit your limit")  // Quota — should advance chain
        || lower.contains("401 unauthorized")  // Auth — should NOT advance, should fail
        || lower.contains("timed out")  // Transient — should retry same provider
}
```

#### Secret Handling and Capability Scoping

| Aspect | Assessment |
|--------|------------|
| API keys in env vars | Standard practice; keys inherited from host via `provider.api_key_env_vars()` |
| Key rotation mid-lane | **Not supported**. Spec explicitly excludes this as out-of-scope. Valid decision. |
| Provider scope validation | No validation that key has quota before attempting call. This is the quota detection gap. |

#### Idempotence and Replay Safety

| Scenario | Behavior | Risk |
|----------|----------|------|
| Lane fails with quota, replays from checkpoint | Will retry same primary provider, hit quota again | Medium — no "provider exhausted" state recorded |
| Provider chain exhausted | Current code has no exhaust handling | **High** — unclear what happens when all providers fail |
| Partial provider_used.json write | JSON written atomically via `serde_json::to_string_pretty()` then `tokio::fs::write()` | Low — atomic write pattern |

#### External-Process Control and Operator Safety

| Control Surface | Assessment |
|-----------------|------------|
| CLI tool invocation (claude, codex, pi) | Sandboxed via `Sandbox` trait; environment filtered |
| Provider rotator state | `CodexRotatorConfig` with shared state path; concurrent-safe |
| `raspberry status` | Read-only view; no mutation capability |

---

## Milestone-by-Milestone Assessment

### Milestone 1: Audit Model Selection Leaks — **CONFIRMED LEAKS**

**Status**: Spec claims accurate. Grep and read inspection validate all four leak points in `fabro-synthesis/src/render.rs`:

| Function | Lines | Hardcoded Model | Policy Violation |
|----------|-------|-----------------|------------------|
| `challenge_target_for_lane()` | 2011–2021 | `gpt-5.4` for codex-unblock | Uses OpenAI, not Write chain |
| `review_target_for_lane()` | 2024–2034 | `gpt-5.4` for codex-unblock | Uses OpenAI, not Review chain |
| `recurring_report_primary_target_for_lane()` | 2050–2084 | Multiple per lane family | Completely bypasses policy.rs |
| `render_workflow_graph()` closure | 2487–2490 | `gpt-5.4` for unblock lanes | Cosmetic equivalent |

**Critical Finding**: `gpt-5.4` appears in hardcoded strings, but policy.rs uses `gpt-5.3-codex` for Synth chain. The spec flags this as needing operator confirmation. This is a **version drift risk**.

### Milestone 2: Quota Detection and Graceful Fallback — **PARTIAL GAP**

**API Path (fabro-llm)**: Correctly instrumented.
- `ProviderErrorKind::QuotaExceeded` exists
- `failover_eligible()` returns `true` for it
- `retryable()` returns `false` for it (correct — don't retry same provider)

**CLI Path (fabro-workflows)**:
- String-matching detection exists but is fragile
- Quota signals conflated with auth errors
- No structured `QuotaExceeded` classification

**Gap**: MiniMax and Kimi quota error messages not explicitly handled. Current patterns:
```rust
"you've hit your limit"       // OpenAI/Codex style
"usage limit has been reached" // Anthropic style
"rate_limit"                   // Generic
```

Missing patterns for Asian providers (MiniMax, Kimi) may cause fallback failure.

### Milestone 3: Provider Health in Status Output — **GAP IDENTIFIED**

**Current State**:
```rust
// DispatchOutcome in dispatch.rs
pub struct DispatchOutcome {
    pub lane_key: String,
    pub exit_status: i32,
    pub fabro_run_id: Option<String>,
    pub stdout: String,
    pub stderr: String,
}
```

**Missing**: No `provider`, `model`, `provider_health` fields.

**`provider_used.json` already contains**:
- `requested_provider`, `requested_model`
- `provider`, `model` (actual)
- `fallback_reason`

**Gap**: This JSON is written but not ingested into `DispatchOutcome` or `AutodevCurrentSnapshot`.

### Milestone 4: Usage Tracking Per Provider Per Cycle — **GAP IDENTIFIED**

**Current State**: `last_usage_summary: Option<String>` is freeform:
```rust
// Example from program_state.rs test
"last_usage_summary": "anthropic: 10 in / 20 out"
```

**Required**: `usage_by_provider: BTreeMap<Provider, ProviderCycleUsage>` in `AutodevCycleReport`.

**Gap**: CLI backends write `provider_used.json` but don't include token counts. API path has usage in response headers but not aggregated per-cycle.

### Milestone 5: Live Validation — **NOT ASSESSABLE**

Requires 50-cycle autodev run. Cannot validate without execution.

---

## Correctness Assessment

| Criterion | Verdict | Evidence |
|-----------|---------|----------|
| Spec correctly identifies leak sites | ✅ Correct | Grep validated; all four functions confirmed |
| Spec correctly describes API path | ✅ Correct | `failover_eligible()` and `retryable()` behavior confirmed |
| Spec correctly identifies gaps | ✅ Correct | Milestones 3-4 gaps validated through source read |
| Acceptance criteria are verifiable | ✅ Correct | Grep-based criteria are objective; live validation criterion is measurable |
| Architecture contracts are sound | ⚠️ Risk | The "no call site may construct ModelTarget literal" contract is currently violated; spec acknowledges this |

---

## Milestone Fit Assessment

This lane is appropriately scoped for a first honest reviewed slice:

| Aspect | Assessment |
|--------|------------|
| Bounded scope | Yes — 4 milestones with clear deliverables |
| Independence | Yes — doesn't block on other genesis plans |
| Value | High — quota fallback failure currently collapses lanes |
| Verifiability | Yes — grep for hardcoded strings; observe fallback in test |
| Rollback safety | Yes — additive changes to status output; no schema migrations |

---

## Remaining Blockers

### Blockers for Spec Approval

1. **Operator confirmation needed**: Is `gpt-5.4` (in render.rs hardcoding) intentionally different from `gpt-5.3-codex` (in policy.rs)? Or is this a copy-paste error? Spec flags this; decision required before render.rs fix.

2. **Missing quota signals**: MiniMax and Kimi error message patterns need to be added to `cli_failure_is_retryable_for_fallback()` or quota fallback will fail for those providers.

### Blockers for Implementation (not spec)

1. **Provider chain exhaust behavior**: Spec doesn't define what happens when all providers in the chain fail. Current code likely panics or returns opaque error. Should be defined.

2. **`provider_used.json` location**: Written to `stage_dir` which is ephemeral. For usage tracking across cycles, spec should clarify aggregation mechanism.

---

## Security Risks Summary

| Risk | Severity | Mitigation in Spec |
|------|----------|-------------------|
| Hardcoded routing bypasses capability floor | Medium | Spec requires removing all hardcoded targets |
| String-matching quota detection misses new providers | Medium | Spec recommends broader signal coverage |
| Quota/auth/timeout conflation | Low-Medium | Spec recommends separating `cli_failure_is_quota_exhausted()` helper |
| No provider exhaust handling | Medium | Should be added to Milestone 2 acceptance |

---

## Recommendations

### For Spec (before implementation)

1. **Add explicit acceptance**: When all providers in chain are exhausted, system must return structured error (not panic).

2. **Clarify `gpt-5.4` vs `gpt-5.3-codex`**: Document whether this is intentional or a bug to fix.

3. **Add MiniMax/Kimi quota signals**: Research and include their quota error message patterns.

### For Implementation

1. **Start with render.rs**: Highest confidence fix; restores policy invariant.

2. **Add test fixtures**: Create simulated provider responses with quota errors for each provider to validate `cli_failure_is_retryable_for_fallback()`.

3. **ProviderHealth enum**: Define as `ok`, `quota_limited`, `auth_error`, `unavailable` with clear transition semantics.

---

## Final Verdict

| Aspect | Rating |
|--------|--------|
| **Correctness** | ✅ Good — claims validated |
| **Completeness** | ⚠️ Good with gaps noted — provider exhaust behavior undefined |
| **Security posture** | ⚠️ Acceptable — risks identified and mitigatable |
| **Milestone fit** | ✅ Appropriate for first slice |
| **Implementation readiness** | ✅ Approved with notes |

**Decision**: **APPROVE for implementation** with the following conditions:
1. Operator confirms `gpt-5.4` vs `gpt-5.3-codex` discrepancy
2. Spec adds chain-exhaustion error handling requirement
3. Implementation adds MiniMax/Kimi quota signal patterns

---

## Evidence Log

| Claim | Verification Method | Result |
|-------|---------------------|--------|
| Hardcoded `gpt-5.4` in render.rs | `grep -n "gpt-5\." lib/crates/fabro-synthesis/src/render.rs` | Confirmed lines 2018, 2031, 2068, 2490 |
| Hardcoded `MiniMax-M2.7-highspeed` | `grep -n "MiniMax" lib/crates/fabro-synthesis/src/render.rs` | Confirmed lines 2054, 2076, 2079, test lines |
| `automation_chain()` exports | Read `lib/crates/fabro-model/src/policy.rs` | Confirmed clean, static chains |
| `failover_eligible()` behavior | Read `lib/crates/fabro-llm/src/error.rs:212` | Confirmed returns true for QuotaExceeded |
| `provider_used.json` written | Read `lib/crates/fabro-workflows/src/backend/cli.rs:620` | Confirmed write location |
| `DispatchOutcome` fields | Read `lib/crates/raspberry-supervisor/src/dispatch.rs:17-24` | Confirmed no provider/model fields |
