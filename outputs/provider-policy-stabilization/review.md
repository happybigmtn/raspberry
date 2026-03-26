# Provider Policy Stabilization — Review

## Audit Date

2026-03-26

## Auditor

Genesis (automated audit + review)

---

## Finding 1: Hardcoded Model Selection in fabro-synthesis/render.rs

**Severity:** Critical

**File:** `lib/crates/fabro-synthesis/src/render.rs`

**Lines:** 2026-2090

### Description

The `recurring_report_primary_target_for_lane()` function and related lane target functions hardcode specific providers and models, bypassing `policy.rs` entirely for holistic review lanes.

### Evidence

```rust
// Lines 2055-2069
fn recurring_report_primary_target_for_lane(lane: &BlueprintLane) -> Option<ModelTarget> {
    if is_parent_holistic_review_minimax_lane(lane) {
        return Some(ModelTarget {
            provider: Provider::Minimax,
            model: "MiniMax-M2.7-highspeed",
        });
    }
    if is_parent_holistic_review_deep_lane(lane) {
        return Some(ModelTarget {
            provider: Provider::Anthropic,
            model: "claude-opus-4-6",
        });
    }
    if is_parent_holistic_review_adjudication_lane(lane)
        || is_post_completion_codex_review_lane(lane)
    {
        return Some(ModelTarget {
            provider: Provider::OpenAi,
            model: "gpt-5.4",
        });
    }
    None
}
```

```rust
// Lines 2026-2039
fn challenge_target_for_lane(lane: &BlueprintLane) -> ModelTarget {
    if let Some(target) = recurring_report_primary_target_for_lane(lane) {
        return target;
    }
    if is_codex_unblock_lane(lane) {
        return ModelTarget {
            provider: Provider::OpenAi,
            model: "gpt-5.4",
        };
    }
    automation_primary_target(AutomationProfile::Write)
}
```

### Impact

Holistic review lanes (minimax pass, deep pass, adjudication pass) do not respect the automation policy chains. When the policy changes, these lanes do not automatically update.

### Required Fix

Replace hardcoded `ModelTarget` returns with calls to `automation_primary_target()` with the appropriate `AutomationProfile`, or add lane-type-specific profiles to `policy.rs`.

### Verification Command

```bash
grep -n "Provider::Minimax\|Provider::Anthropic\|Provider::OpenAi" \
  lib/crates/fabro-synthesis/src/render.rs | \
  grep -v "use\|//" | wc -l
# Should be 0 after fix
```

---

## Finding 2: CLI Fallback Missing Quota-Specific Check

**Severity:** Medium

**File:** `lib/crates/fabro-workflows/src/backend/cli.rs`

**Function:** `cli_failure_is_retryable_for_fallback()`

**Line:** ~1355

### Description

The CLI fallback function checks for "you've hit your limit" and "usage limit has been reached" in lowercase error details, but does not explicitly check for `QuotaExceeded` errors that come through the API path. The CLI and API paths may report quota errors differently.

### Evidence

```rust
fn cli_failure_is_retryable_for_fallback(detail: &str) -> bool {
    let lower = detail.to_ascii_lowercase();
    lower.contains("not logged in")
        || lower.contains("/login")
        || lower.contains("selected model")
        || lower.contains("may not exist or you may not have access to it")
        || lower.contains("you've hit your limit")        // <-- CLI-specific
        || lower.contains("usage limit has been reached") // <-- CLI-specific
        || lower.contains("rate_limit")
        // ... more patterns
}
```

### Impact

If a provider returns a quota error through the API path (not CLI), the CLI backend may not recognize it as retryable for fallback.

### Required Fix

Add explicit `QuotaExceeded` check or add "quota" to the retryable patterns. The fabro-llm SDK already classifies quota errors correctly; the issue is the CLI path doesn't go through SDK error classification.

### Verification Command

```bash
cargo nextest run -p fabro-workflows -- provider_fallback_quota
```

---

## Finding 3: Stylesheet Test Fixtures with Hardcoded Models

**Severity:** Low (test code only)

**File:** `lib/crates/fabro-workflows/src/stylesheet.rs`

**Lines:** 86-210

### Description

Test fixtures use hardcoded model names like "sonnet", "opus", "gpt" in test cases. These are test fixtures, not production code, but they could cause confusion.

### Evidence

```rust
#[test]
fn apply_class_overrides_universal() {
    let ss = parse_stylesheet("* { model: sonnet; } .code { model: opus; }").unwrap();
    // ...
}
```

### Impact

Low — these are test assertions that verify stylesheet parsing behavior, not production selection logic. The model names are used as CSS-like selectors in the stylesheet DSL.

### Recommendation

Document that stylesheet model values are arbitrary strings validated by the policy layer at render time, not by stylesheet.rs itself.

---

## Finding 4: Transform Tests with Hardcoded Models

**Severity:** Low (test code only)

**File:** `lib/crates/fabro-workflows/src/transform.rs`

**Lines:** 681-790

### Description

Transform tests use hardcoded model names in test cases.

### Impact

Low — test fixtures only.

---

## Finding 5: Test Integration Files with Hardcoded Models

**Severity:** Informational

**Files:**
- `fabro-workflows/tests/integration.rs` (many test fixtures)
- `fabro-agent/tests/parity_matrix.rs`
- `fabro-agent/tests/guardrails.rs`

### Description

Extensive test fixtures with model names used for integration testing.

### Impact

None for production — these are test assertions.

---

## Finding 6: API Provider Already Handles Quota Correctly

**Severity:** N/A (positive finding)

**File:** `lib/crates/fabro-llm/src/error.rs`

### Description

The fabro-llm SDK already has:
- `ProviderErrorKind::QuotaExceeded` — distinct from `RateLimit`
- `failover_eligible()` returns `true` for `QuotaExceeded`
- 429 errors map to `RateLimit` (retryable, same-provider backoff)

### Evidence

```rust
#[must_use]
pub const fn failover_eligible(&self) -> bool {
    if self.retryable() {
        return true;
    }
    matches!(
        self,
        Self::Provider {
            kind: ProviderErrorKind::QuotaExceeded,
            ..
        } | Self::RequestTimeout { .. }
    )
}
```

### Impact

Positive — the SDK layer is already correctly classified. The gap is in the CLI path which doesn't use SDK error classification.

---

## Finding 7: CLI Backend Uses Central Policy for Fallback Chain

**Severity:** N/A (positive finding)

**File:** `lib/crates/fabro-workflows/src/backend/cli.rs`

### Description

The `build_cli_attempt_targets()` function correctly uses `central_policy_fallback_targets()` to build the fallback chain:

```rust
fn build_cli_attempt_targets(...) -> Vec<CliAttemptTarget> {
    // ...
    let fallback_targets = if configured_fallback_chain.is_empty() {
        central_policy_fallback_targets(initial.0, &initial.1)  // <-- Uses policy.rs
    } else {
        configured_fallback_chain...
    };
}
```

### Impact

Positive — the CLI backend already respects the central policy for fallback chains.

---

## Finding 8: No Provider Health Status in Raspberry Status

**Severity:** Medium

**File:** `lib/crates/raspberry-supervisor/src/autodev.rs`

### Description

The `AutodevCurrentSnapshot` struct does not include provider health information. The status output only shows lane counts and ready/blocked/running/failed lanes, not the model routing or provider health per lane.

### Evidence

```rust
pub struct AutodevCurrentSnapshot {
    pub updated_at: DateTime<Utc>,
    pub ready: usize,
    pub running: usize,
    pub blocked: usize,
    pub failed: usize,
    pub complete: usize,
    pub ready_lanes: Vec<String>,
    pub running_lanes: Vec<String>,
    pub failed_lanes: Vec<String>,
    pub critical_blockers: Vec<CriticalBlocker>,
    // Missing: per-lane model/provider/health
}
```

### Required Fix

Add `lanes` field with per-lane model routing and provider health status.

---

## Finding 9: No Usage Tracking Per Provider

**Severity:** Medium

**File:** `lib/crates/raspberry-supervisor/src/autodev.rs`

### Description

The `AutodevCycleReport` struct tracks lane outcomes but not usage metrics per provider.

### Evidence

```rust
pub struct AutodevCycleReport {
    pub cycle: usize,
    pub evolved: bool,
    pub evolve_target: Option<String>,
    pub ready_lanes: Vec<String>,
    pub dispatched_lanes: Vec<LaneDispatch>,
    pub lane_outcomes: Vec<LaneOutcome>,
    // Missing: usage_by_provider
}
```

### Required Fix

Add `usage_by_provider: Vec<ProviderUsage>` to track tokens and cost per provider per cycle.

---

## Summary of Required Changes

| # | File | Change | Severity |
|---|------|--------|----------|
| 1 | `fabro-synthesis/src/render.rs` | Replace hardcoded `ModelTarget` with `automation_primary_target()` calls | Critical |
| 2 | `fabro-workflows/src/backend/cli.rs` | Add "quota" to `cli_failure_is_retryable_for_fallback()` patterns | Medium |
| 3 | `raspberry-supervisor/src/autodev.rs` | Add per-lane model/provider/health to status output | Medium |
| 4 | `raspberry-supervisor/src/autodev.rs` | Add `usage_by_provider` to cycle report | Medium |

---

## Open Questions

1. **Lane-specific profiles:** Should holistic review lanes have their own `AutomationProfile` in policy.rs, or should they use the standard `Review` profile? Currently they have hardcoded overrides that differ from the standard review chain.

2. **Provider health determination:** How should we determine if a provider is "quota_limited" vs "healthy"? The SDK provides rate limit info from response headers, but quota exhaustion may not be detected until after a request fails.

3. **Usage tracking granularity:** Should usage be tracked per lane per cycle, or aggregated per provider per cycle? Per-lane enables better cost attribution but increases report size.

---

## Decision Log

- **Decision:** fabro-synthesis render.rs must call policy.rs functions, not construct ModelTarget directly
  - Rationale: The assessment found model selection decisions leaking into render.rs even after the policy.rs centralization. Every leak is a potential MiniMax-plus-Claude mismatch.
  - Date: 2026-03-26

- **Decision:** CLI fallback for quota errors must not consume retry count
  - Rationale: Quota exhaustion on Provider A does not imply Provider B is exhausted. The fallback chain should proceed without penalty.
  - Date: 2026-03-26

- **Decision:** Provider health status is informational, not a gate
  - Rationale: Health status reflects recent failures, not current quota state. Lanes should not be blocked based on stale health data.
  - Date: 2026-03-26
