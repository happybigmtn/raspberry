# Provider Policy Stabilization — Lane Review

**Lane:** `provider-policy-stabilization`  
**Spec:** `outputs/provider-policy-stabilization/spec.md`  
**Plan:** `genesis/plans/008-provider-policy-stabilization.md` (referenced in `genesis/plans/001-master-plan.md`)  
**Review Date:** 2026-03-26  
**Review Stage:** First honest reviewed slice — APPROVED  

---

## Executive Summary

The spec presents a coherent, bounded capability definition for centralizing model routing through `fabro_model::policy::automation_chain()`. The review validates every claim through direct source inspection. Milestone 1 findings are confirmed: there are hardcoded model string leaks in `fabro-synthesis/src/render.rs` that violate the policy centralization invariant. Milestones 2–4 identify structural gaps in dispatch outcomes, usage tracking, and CLI quota detection that require implementation work. The spec is **fit for implementation** with three open questions requiring operator input before Milestone 1 can land.

---

## Nemesis-Style Security Review

### Pass 1 — First-Principles Challenge

#### Trust Boundaries and Authority Assumptions

| Boundary | Current State | Risk |
|----------|---------------|------|
| `policy.rs` as source of truth | ✅ Validated clean. All model strings are `&'static str` constants; chains are immutable. | Low |
| `render.rs` → model routing | ❌ **BROKEN**. Four functions construct `ModelTarget` literals bypassing policy entirely. | **High** — workflow graphs embed hardcoded providers/models |
| CLI backend → `cli.rs` | ✅ Uses `automation_fallback_targets()` correctly for chain traversal. | Low |
| Quota detection in CLI path | ⚠️ Substring matching on stderr; MiniMax/Kimi patterns missing | Medium |
| Quota detection in API path | ✅ `ProviderErrorKind::QuotaExceeded` correctly instrumented. | Low |
| Provider credentials | Inherited from host via `provider.api_key_env_vars()` | Medium — no scope validation or pre-check |

**Authority Challenge**: Who can alter which model runs where?

The `automation_chain()` function returns a static slice — compile-time fixed, no runtime override. However, lane ID suffix patterns in `render.rs` **override** the policy chain entirely:

```rust
// fabro-synthesis/src/render.rs:2015-2019 — bypasses policy for codex-unblock lanes
if is_codex_unblock_lane(lane) {
    return ModelTarget { provider: Provider::OpenAi, model: "gpt-5.4" };
}
automation_primary_target(AutomationProfile::Write)
```

This is not an external attack surface (all providers are trusted), but it **violates the policy abstraction** and causes:
1. **Cost surprises** — routing to expensive Opus when Minimax was policy-intended for a lane family
2. **Capability floor violations** — `claude-opus-4-6` reviewing security-critical code that the Write profile intentionally routes to Minimax
3. **Quota exhaustion on the wrong provider** — `gpt-5.4` hitting OpenAI quota while the Synth chain's proper fallback is `gpt-5.3-codex`

#### Dangerous Actions and Trigger Points

| Dangerous Action | Who Can Trigger | Current Control |
|-----------------|-----------------|-----------------|
| Provider fallback chain exhaustion | Any quota-exhausted lane run | `cli_failure_is_retryable_for_fallback()` — currently returns true for quota signals, but also for auth and transient errors |
| Routing to expensive provider (Anthropic Opus) | Any `-holistic-review-deep` lane | Hardcoded in `render.rs:2058-2061`; bypasses policy cost ordering |
| Model version drift (`gpt-5.4` vs `gpt-5.3-codex`) | Codex-unblock and adjudication lanes | `render.rs` uses `gpt-5.4`; `policy.rs` defines `gpt-5.3-codex` as Synth fallback; unknown if intentional |

---

### Pass 2 — Coupled-State Review

#### Paired State Surfaces

| State A | State B | Consistency Check |
|---------|---------|-------------------|
| `provider_used.json` (written by `cli.rs:620`) | `DispatchOutcome` (returned to supervisor) | ❌ **GAPPED**: `DispatchOutcome` has no `provider`/`model`/`fallback_reason` fields. The JSON is written but never ingested. |
| `fabro-llm::ProviderErrorKind::QuotaExceeded` | `cli_failure_is_retryable_for_fallback()` | ❌ **DECOUPLED**: CLI path uses string matching; API path uses structured error. These can drift independently. |
| Policy chain ordering (Write: Minimax→Kimi→Opus) | Lane runtime routing | ❌ **VIOLATED**: `recurring_report_primary_target_for_lane()` ignores policy for `-holistic-review-*` lane families. |
| `AutodevCycleReport.usage_by_provider` | `last_usage_summary: Option<String>` | ❌ **MISSING**: `AutodevCycleReport` has no `usage_by_provider` field. `last_usage_summary` is a freeform string with no per-provider breakdown. |
| `cli_failure_is_retryable_for_fallback()` return | Auth error handling | ❌ **CONFLATED**: Returns `true` for `"401 unauthorized"` — auth errors advance the fallback chain instead of failing fast. |

#### State Transitions Affecting Safety

**Quota Exhaustion Transition (CLI path)**:
```
Lane running on Provider P → CLI stderr contains quota signal
                          → cli_failure_is_quota_exhausted() returns true
                          → backend advances to P' in policy chain
                          → no per-provider retry budget consumed
```

**Risk — Auth Error Conflation**:
```rust
// fabro-workflows/src/backend/cli.rs:800-810 — "401 unauthorized" in the same OR chain as quota signals
fn cli_failure_is_retryable_for_fallback(detail: &str) -> bool {
    let lower = detail.to_ascii_lowercase();
    lower.contains("401 unauthorized")  // auth — should NOT advance, should fail immediately
        || lower.contains("you've hit your limit")  // quota — SHOULD advance
        || lower.contains("timed out")  // transient — should retry same provider
        // ...
}
```

A 401 Unauthorized (expired or bad credential) will incorrectly advance the fallback chain, masking the credential problem and potentially exhausting all providers with auth failures.

**Risk — Transient Error Conflation**:
Timeouts and network errors (`"timed out"`, `"connection refused"`) are included in the same function, meaning they advance the fallback chain instead of retrying the same provider. A flaky network could exhaust all providers before any real retry.

#### Secret Handling and Capability Scoping

| Aspect | Assessment |
|--------|------------|
| API keys in env vars | Standard practice; inherited from host via `provider.api_key_env_vars()` |
| Key rotation mid-lane | Not supported; spec explicitly excludes as out-of-scope |
| Provider scope validation | No pre-check that key has remaining quota before attempting call |
| `FABRO_STRICT_PROVIDER = "1"` in codex-unblock | Forces strict provider matching; additional safeguard for that lane family |

#### Idempotence and Replay Safety

| Scenario | Behavior | Risk |
|----------|----------|------|
| Lane fails with quota, replays from checkpoint | Retries same primary provider, hits quota again | Medium — no "provider exhausted" state persisted |
| Provider chain exhausted | Spec doesn't define behavior; likely panics or returns opaque error | **High** — must be defined before Milestone 4 |
| `provider_used.json` write | Written via `serde_json::to_string_pretty()` then `tokio::fs::write()` to `stage_dir` | Low — atomic pattern |
| `stage_dir` ephemeral | `provider_used.json` is written to ephemeral stage dir; not persisted for cross-cycle aggregation | **Medium** — usage tracking needs a persistent aggregation path |

#### External-Process Control and Operator Safety

| Control Surface | Assessment |
|-----------------|------------|
| CLI tool invocation (claude, codex, pi) | Sandboxed via `Sandbox` trait; environment filtered |
| Provider rotator state | `CodexRotatorConfig` with shared state path; concurrent-safe |
| `raspberry status` | Read-only view; no mutation capability |
| Lane dispatch | `fabro-workflows/src/backend/cli.rs` — returns `DispatchOutcome`; no direct process control escape |

---

## Milestone-by-Milestone Assessment

### Milestone 1: Centralization — ✅ LEAKS CONFIRMED, `gpt-5.4` DISCREPANCY FOUND

All four leak sites confirmed by grep:

| Function | Lines | Hardcoded Model | Policy Violation |
|----------|-------|-----------------|------------------|
| `challenge_target_for_lane()` | 2015–2020 | `gpt-5.4` on OpenAI for codex-unblock | Bypasses Write chain |
| `review_target_for_lane()` | 2028–2033 | `gpt-5.4` on OpenAI for codex-unblock | Bypasses Review chain |
| `recurring_report_primary_target_for_lane()` | 2051–2070 | Three hardcoded targets per lane family | **High** — bypasses all profiles |
| `render_workflow_graph()` closure | 2487–2491 | `gpt-5.4` for unblock lanes | Cosmetic equivalent of above |

**Additionally confirmed**:
- `custom_fallback_section_for_lane()` at lines 2074–2081 hardcodes `[llm.fallbacks]` TOML strings (`"anthropic = [\"gpt-5.4\"]"`, `"openai = [\"claude-opus-4-6\"]"`) instead of deriving from `automation_fallback_targets()`.
- Test assertions throughout the file (lines ~8389–8565) assert on hardcoded strings — these tests validate the **wrong behavior** and must be updated alongside the fix.
- `gpt-5.4` vs `gpt-5.3-codex`: policy.rs defines `gpt-5.3-codex` as the Synth chain's OpenAI fallback; `render.rs` uses `gpt-5.4` everywhere. This is a genuine discrepancy requiring operator confirmation.

### Milestone 2: Quota Detection and Graceful Fallback — ⚠️ PARTIAL GAP

**API path** (`fabro-llm`): ✅ Correct.

- `ProviderErrorKind::QuotaExceeded` exists and is used in `failover_eligible()`.
- `retryable()` returns `false` for `QuotaExceeded` (correct — same-provider retry futile).
- `failover_eligible()` returns `true` for `QuotaExceeded` (correct — different provider has independent quota).

**CLI path** (`fabro-workflows`): ⚠️ Multiple gaps.

1. **Auth/transient conflation**: `"401 unauthorized"` and `"timed out"` are in the same `or` chain as quota signals — they advance the fallback chain when they should not.
2. **MiniMax/Kimi patterns missing**: Current patterns cover OpenAI/Anthropic-style messages; no coverage for MiniMax or Kimi quota errors.
3. **No structured `QuotaExceeded`**: CLI path uses substring matching exclusively; no integration with `ProviderErrorKind::QuotaExceeded`.

The spec's prescription (split into three focused helpers) directly addresses these gaps.

### Milestone 3: Provider Health in Status Output — ✅ GAP VALIDATED

**Current state confirmed**: `DispatchOutcome` (`raspberry-supervisor/src/dispatch.rs:17-24`) has no `provider`, `model`, `provider_health`, or `fallback_reason` fields.

**`provider_used.json` confirmed written** at `cli.rs:620` but not ingested into `DispatchOutcome`. The spec correctly identifies this as the integration point.

### Milestone 4: Usage Tracking Per Provider Per Cycle — ✅ GAP VALIDATED

**Current state confirmed**: `AutodevCycleReport` (`raspberry-supervisor/src/autodev.rs:96-109`) has no `usage_by_provider` field. `last_usage_summary: Option<String>` in lane records is a freeform string with no per-provider breakdown.

**`provider_used.json` contains** token counts and provider info but is written to ephemeral `stage_dir` — not persisted for cross-cycle aggregation. Spec correctly identifies this as requiring a persistent aggregation mechanism.

### Milestone 5: Live Validation — ❌ NOT ASSESSABLE

Requires 50-cycle autodev run in an environment with pre-exhausted primary providers. Cannot assess without execution.

---

## Correctness Assessment

| Claim in Spec | Verification | Result |
|---------------|-------------|--------|
| `policy.rs` exports clean static chains | Read `lib/crates/fabro-model/src/policy.rs` | ✅ Confirmed |
| `gpt-5.4` hardcoded in render.rs at lines 2018, 2031, 2068, 2490 | `grep -n "gpt-5\." lib/crates/fabro-synthesis/src/render.rs` | ✅ Confirmed |
| `MiniMax-M2.7-highspeed` hardcoded in render.rs | `grep -n "MiniMax" lib/crates/fabro-synthesis/src/render.rs` | ✅ Confirmed |
| `DispatchOutcome` has no provider/model fields | Read `raspberry-supervisor/src/dispatch.rs:17-24` | ✅ Confirmed |
| `AutodevCycleReport` has no `usage_by_provider` | Read `raspberry-supervisor/src/autodev.rs:96-109` | ✅ Confirmed |
| `failover_eligible()` returns true for `QuotaExceeded` | Read `fabro-llm/src/error.rs:212` | ✅ Confirmed |
| `provider_used.json` written by `cli.rs:620` | Confirmed via grep | ✅ Confirmed |
| `cli_failure_is_retryable_for_fallback()` includes `"401 unauthorized"` and `"timed out"` | Read `fabro-workflows/src/backend/cli.rs:800-810` | ✅ Confirmed — these should NOT advance the chain |
| `provider_used.json` not ingested into `DispatchOutcome` | Confirmed gap | ✅ Confirmed |

---

## Security Risks Summary

| Risk | Severity | Spec Mitigation |
|------|----------|----------------|
| Hardcoded routing bypasses capability/cost floor | Medium | Milestone 1 removes all hardcoded targets |
| `gpt-5.4` vs `gpt-5.3-codex` version drift | Medium | Operator must confirm; unknown if intentional |
| Auth error (401) advances fallback chain | Medium | Spec requires `cli_failure_is_auth_error()` helper; auth failures must fail fast |
| Transient error (timeout) advances fallback chain | Medium | Spec requires `cli_failure_is_transient()` helper; transient errors must retry same provider |
| MiniMax/Kimi quota signals not detected | Medium | Spec requires operator to provide error message samples; cannot validate until provided |
| No chain-exhaustion handling | Medium | Spec open question #3; must be resolved before implementation |
| `provider_used.json` in ephemeral stage_dir | Medium | Spec requires persistent aggregation path for cross-cycle usage |

---

## Recommendations

### Required Before Implementation (Spec Conditions)

1. **Operator confirms `gpt-5.4` vs `gpt-5.3-codex`**: Are codex-unblock and adjudication lanes intentionally using `gpt-5.4` (not in the policy chain) instead of `gpt-5.3-codex` (which IS in the policy chain as the Synth fallback)? If `gpt-5.4` is wrong, Milestone 1 fix changes the model these lanes actually use.

2. **Operator provides MiniMax and Kimi quota error messages**: What string(s) do these providers return when quota is exhausted? Without this, Milestone 2 will have incomplete coverage and fallback will silently fail for those providers.

3. **Chain exhaustion behavior must be defined**: When all providers in the chain return `QuotaExceeded` (or fail), what is the lane outcome? The spec's Open Question #3 must be answered before Milestone 4 can be implemented.

### For Implementation

1. **Start with `render.rs` Milestone 1**: Highest confidence fix; restores the policy invariant. Update tests simultaneously — current tests assert on the **wrong** (hardcoded) behavior.

2. **Add test fixtures for quota scenarios**: Simulate provider responses with quota errors for each provider (OpenAI, Anthropic, MiniMax, Kimi) to validate the three-helper split in `cli_failure_is_retryable_for_fallback()`.

3. **Add `ProviderHealth` enum first**: Define as `Ok`, `QuotaLimited`, `AuthError`, `Unavailable` with clear transition semantics. This is a pure additive change with no existing callers to break.

4. **Persist `provider_used.json` to a stable location**: The current write to ephemeral `stage_dir` must be mirrored or moved to enable cross-cycle usage aggregation.

---

## Final Verdict

| Aspect | Rating |
|--------|--------|
| **Correctness** | ✅ Good — all claims validated against source |
| **Completeness** | ⚠️ Good with open questions — chain exhaustion undefined; MiniMax/Kimi signals missing |
| **Security posture** | ⚠️ Acceptable — risks identified and mitigatable; auth/transient conflation is a real bug |
| **Milestone fit** | ✅ Appropriate for first slice |
| **Implementation readiness** | ✅ Approved with conditions |

**Decision**: **APPROVE for implementation** subject to:
1. Operator confirming `gpt-5.4` vs `gpt-5.3-codex` intent
2. Operator providing MiniMax and Kimi quota error message strings
3. Operator answering chain exhaustion behavior question

---

## Evidence Log

| Claim | Verification Method | Result |
|-------|---------------------|--------|
| Hardcoded `gpt-5.4` at lines 2018, 2031, 2068, 2490 | `grep -n "gpt-5\." lib/crates/fabro-synthesis/src/render.rs` | ✅ Confirmed |
| Hardcoded `MiniMax-M2.7-highspeed` in recurring_report | `grep -n "MiniMax" lib/crates/fabro-synthesis/src/render.rs` | ✅ Confirmed at 2054, 2076, 2079 |
| `automation_chain()` exports | Read `lib/crates/fabro-model/src/policy.rs` | ✅ Static chains, clean |
| `failover_eligible()` behavior | Read `lib/crates/fabro-llm/src/error.rs:212` | ✅ Returns true for QuotaExceeded |
| `DispatchOutcome` field list | Read `raspberry-supervisor/src/dispatch.rs:17-24` | ✅ No provider/model/health fields |
| `AutodevCycleReport` field list | Read `raspberry-supervisor/src/autodev.rs:96-109` | ✅ No usage_by_provider field |
| `cli_failure_is_retryable_for_fallback()` contents | Read `lib/crates/fabro-workflows/src/backend/cli.rs:800-810` | ✅ "401 unauthorized" and "timed out" conflated with quota |
| `provider_used.json` write location | Confirmed at `cli.rs:620` | ✅ Written to stage_dir; not ingested |
| `last_usage_summary` is freeform string | Read `program_state.rs` and `evaluate.rs` | ✅ `"anthropic: 10 in / 20 out"` format, not parsed |
