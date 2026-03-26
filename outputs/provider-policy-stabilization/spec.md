# Provider Policy Stabilization — Capability Spec

**Status:** Draft — first honest reviewed slice  
**Lane:** `provider-policy-stabilization`  
**Plan:** `genesis/plans/008-provider-policy-stabilization.md`  
**Author:** Genesis  
**Date:** 2026-03-26  

---

## Purpose / User-Visible Outcome

Every model-routing decision in the binaries operators actually run is made by a single function call to `fabro_model::policy::automation_chain()` (or its variants). No hardcoded model strings appear in workflow graph generation or CLI backend execution. When any provider exhausts its quota, the system falls through to the next provider in the policy chain without collapsing the active lane, and `raspberry status` shows which model each lane is using and which providers are quota-limited.

---

## Whole-System Goal

A lane run by `raspberry autodev --max-cycles 50` completes without manual rescue even when the primary provider for every active lane hits quota. The operator can observe this happening in `raspberry status` output.

---

## Scope

This spec covers:

1. **Centralization enforcement** — eliminating hardcoded model strings from `fabro-synthesis/src/render.rs` and ensuring `fabro-workflows/src/backend/cli.rs` uses the policy chain for all runtime selection.
2. **Quota-aware fallback** — wiring `ProviderErrorKind::QuotaExceeded` from `fabro-llm` into the CLI backend fallback path so quota exhaustion triggers provider failover without consuming retry budget.
3. **Operator-visible provider health** — adding per-lane model routing and per-provider quota status to `raspberry status` output.
4. **Per-cycle usage tracking** — recording tokens consumed per provider per autodev cycle and surfacing it in the cycle report.

This spec does **not** cover:

- Adding new providers to the policy chain (separate policy decision)
- LLM API-level retries (already correct in `fabro-llm/src/retry.rs` for rate-limit 429s)
- `fabro-agent` CLI-only model routing (covered by `genesis/plans/003-exec-path.md`)

---

## Current State

### The policy source of truth

`lib/crates/fabro-model/src/policy.rs` defines `automation_chain()` returning a `&'static [ModelTarget]` for each `AutomationProfile` (`Write`, `Review`, `Synth`). The chains are:

| Profile | Primary | Fallback 1 | Fallback 2 |
|---------|---------|------------|------------|
| `Write` | `minimax / MiniMax-M2.7-highspeed` | `kimi / kimi-k2.5` | `anthropic / claude-opus-4-6` |
| `Review` | `kimi / kimi-k2.5` | `minimax / MiniMax-M2.7-highspeed` | `anthropic / claude-opus-4-6` |
| `Synth` | `anthropic / claude-opus-4-6` | `openai / gpt-5.3-codex` | `minimax / MiniMax-M2.7-highspeed` |

Exported accessors: `automation_chain()`, `automation_primary_target()`, `automation_fallback_targets()`, `automation_fallback_map()`. No mutation API exists; chains are compile-time fixed.

### Leaking hardcoded strings (confirmed)

`fabro-synthesis/src/render.rs` constructs `ModelTarget` literals instead of calling the policy API. All four locations are confirmed by grep inspection:

| Function | Lines | Hardcoded Value | Policy Violation |
|----------|-------|-----------------|------------------|
| `challenge_target_for_lane()` | 2015–2020 | `Provider::OpenAi`, `"gpt-5.4"` for codex-unblock lanes | Bypasses Write chain |
| `review_target_for_lane()` | 2028–2033 | `Provider::OpenAi`, `"gpt-5.4"` for codex-unblock lanes | Bypasses Review chain |
| `recurring_report_primary_target_for_lane()` | 2051–2070 | `minimax/MiniMax-M2.7-highspeed` for holistic-review-minimax; `anthropic/claude-opus-4-6` for holistic-review-deep; `openai/gpt-5.4` for holistic-review-adjudication | **High — lane-family hardcoding bypasses all policy profiles entirely** |
| `render_workflow_graph()` closure | 2487–2491 | `Provider::OpenAi`, `"gpt-5.4"` for unblock lanes | Cosmetic equivalent of challenge/review hardcoding |

**`gpt-5.4` vs `gpt-5.3-codex` discrepancy**: `render.rs` consistently hardcodes `gpt-5.4` for codex-unblock and adjudication lanes, but `policy.rs` defines `gpt-5.3-codex` as the Synth fallback. These are different model strings — operator must confirm which is intentional before `render.rs` is corrected.

### CLI backend fallback path

`fabro-workflows/src/backend/cli.rs` lines 799–810 implement `cli_failure_is_retryable_for_fallback()` using substring matching on lowercase stderr:

```rust
fn cli_failure_is_retryable_for_fallback(detail: &str) -> bool {
    let lower = detail.to_ascii_lowercase();
    lower.contains("you've hit your limit")
        || lower.contains("usage limit has been reached")
        || lower.contains("rate_limit")
        || lower.contains("401 unauthorized")  // auth — should NOT advance
        || lower.contains("timed out")          // transient — should retry same provider
        // ... more patterns
}
```

**Issues**:
1. Quota signals (`"you've hit your limit"`, `"usage limit has been reached"`) are correctly included and return `true` — the failover path advances correctly.
2. `"401 unauthorized"` is **conflated** with quota signals — returns `true`, causing auth errors to advance the fallback chain instead of failing fast.
3. `"timed out"` and network patterns are also included — these are transient and should retry the **same** provider, not advance the chain.
4. MiniMax and Kimi quota error messages are **not** explicitly handled; their specific error strings are unknown and need operator research.
5. The CLI path is architecturally decoupled from `fabro-llm::ProviderErrorKind::QuotaExceeded` — no structured error flows between them.

### `fabro-llm` error handling (already correct)

`fabro-llm/src/error.rs`:
- `ProviderErrorKind::QuotaExceeded` is defined.
- `failover_eligible()` returns `true` for `QuotaExceeded` — correct, since a different provider has independent quota.
- `retryable()` returns `false` for `QuotaExceeded` — correct, same-provider retry is futile.
- `error_from_status_code()` maps HTTP 429 to `RateLimit` (retryable); message inspection maps quota-exhausted strings to `QuotaExceeded`.

### Missing provider/model fields in dispatch and report types

**`DispatchOutcome`** (`raspberry-supervisor/src/dispatch.rs:17-24`):
```rust
pub struct DispatchOutcome {
    pub lane_key: String,
    pub exit_status: i32,
    pub fabro_run_id: Option<String>,
    pub stdout: String,
    pub stderr: String,
    // MISSING: provider, model, provider_health, fallback_reason
}
```

**`AutodevCycleReport`** (`raspberry-supervisor/src/autodev.rs:96-109`):
```rust
pub struct AutodevCycleReport {
    pub cycle: usize,
    pub evolved: bool,
    pub evolve_target: Option<String>,
    pub ready_lanes: Vec<String>,
    pub replayed_lanes: Vec<String>,
    pub regenerate_noop_lanes: Vec<String>,
    pub dispatched: Vec<DispatchOutcome>,
    pub running_after: usize,
    pub complete_after: usize,
    // MISSING: usage_by_provider
}
```

`raspberry-supervisor/src/program_state.rs` has `last_usage_summary: Option<String>` in lane runtime records — a freeform string (e.g. `"anthropic: 10 in / 20 out"`). This is not parsed or aggregated per-provider.

`provider_used.json` is written by `backend/cli.rs:620` and contains `requested_provider`, `requested_model`, `provider`, `model`, and `fallback_reason` — but this data is not ingested into `DispatchOutcome` or `AutodevCycleReport`.

---

## Architecture / Runtime Contract

### Model routing contract

Every call site that needs to select a model for a lane must call exactly one of:

- `automation_primary_target(profile)` — returns the first `ModelTarget` in the chain
- `automation_chain(profile)` — returns the full chain as a slice
- `automation_fallback_targets(profile)` — returns all targets after the primary

No call site may construct a `ModelTarget` literal or reference a provider/model string directly. The only permitted exceptions are:
- Test code (`#[cfg(test)]` modules)
- Display/logging strings that are not used for routing decisions

### Quota failover contract

**CLI path** (`backend/cli.rs`): When a CLI invocation returns a failure whose stderr contains a quota-exhausted signal, `cli_failure_is_retryable_for_fallback()` must return `true` and the backend must advance to the next provider in the chain without counting it against the per-provider retry budget.

**API path** (`fabro-llm`): When an LLM API call returns `SdkError::Provider { kind: QuotaExceeded, .. }`, the caller must advance to the next provider in the chain. `failover_eligible()` on `SdkError` returns `true` for `QuotaExceeded`.

Quota-exhausted is **not** retried within the same provider. Chain exhaustion (all providers fail) must produce a structured error, not a panic.

**`cli_failure_is_retryable_for_fallback()` must be split** into:
- `cli_failure_is_quota_exhausted(detail: &str) -> bool` — returns `true` only for quota signals, advances the fallback chain
- `cli_failure_is_transient(detail: &str) -> bool` — returns `true` for timeouts, network errors; advances same-provider retry
- `cli_failure_is_auth_error(detail: &str) -> bool` — returns `true` for 401/unauthorized; fails fast without chain advance

### Provider health and usage reporting contract

`raspberry status` output must include, per active lane:
- The lane key
- The assigned model (e.g., `MiniMax-M2.7-highspeed`)
- The provider (e.g., `minimax`)
- Any provider health signal: `ok`, `quota_limited`, `auth_error`, `unavailable`

`DispatchOutcome` must include:
```rust
pub struct DispatchOutcome {
    // ... existing fields ...
    pub provider: Option<String>,
    pub model: Option<String>,
    pub provider_health: Option<ProviderHealth>,
    pub fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderHealth {
    Ok,
    QuotaLimited,
    AuthError,
    Unavailable,
}
```

`AutodevCycleReport` must include:
```rust
pub struct AutodevCycleReport {
    // ... existing fields ...
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub usage_by_provider: Vec<ProviderCycleUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCycleUsage {
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub estimated_cost_usd: Option<f64>,
}
```

Data sources: `provider_used.json` (written by CLI backend) and LLM API usage response headers.

---

## Adoption Path

### Milestone 1: Centralization gate

1. **Audit gate**: `grep` confirms 0 non-test, non-comment results for model string literals (`"gpt-5."`, `"MiniMax"`, `"claude-opus"`, `"kimi-"`) outside `policy.rs` and test files.
2. **Render gate**: Replace all hardcoded `ModelTarget` constructions in `render.rs` with calls to `automation_primary_target()` and `automation_fallback_targets()`, parameterized by lane profile.
   - Confirm `gpt-5.4` vs `gpt-5.3-codex` with operator before changing codex-unblock/adjudication lanes.
   - `recurring_report_primary_target_for_lane()` must delegate to `automation_primary_target(AutomationProfile::Review)` (or `Synth` depending on lane family) rather than hardcoding per-lane-family targets.
   - `custom_fallback_section_for_lane()` must be updated to use `automation_fallback_targets()` output instead of hand-written `[llm.fallbacks]` TOML strings.
   - Tests in `render.rs` that assert on hardcoded model strings must be updated to reflect policy-chain values.

### Milestone 2: Quota detection gate

1. Split `cli_failure_is_retryable_for_fallback()` into three focused helpers.
2. Add MiniMax and Kimi quota signal patterns (operator to provide error message samples).
3. Add `ProviderHealth` enum to `raspberry-supervisor/src/dispatch.rs`.
4. Add `provider`, `model`, `provider_health`, `fallback_reason` fields to `DispatchOutcome`.
5. Ingest `provider_used.json` into `DispatchOutcome` after CLI execution completes.
6. Wire `ProviderHealth::QuotaLimited` into status output.

### Milestone 3: Usage tracking gate

1. Add `usage_by_provider: Vec<ProviderCycleUsage>` to `AutodevCycleReport`.
2. Parse `provider_used.json` for token counts and aggregate into `ProviderCycleUsage` per cycle.
3. Backfill `last_usage_summary` population from `usage_by_provider` (for display in `raspberry status`).

### Milestone 4: Live validation gate

Run `raspberry autodev --max-cycles 50` in an environment where the primary provider is pre-exhausted (e.g., via mock or quota-pre-set credential) and observe that:
- All lanes complete without manual rescue
- `raspberry status` shows `quota_limited` health for the primary and `ok` for the fallback
- `AutodevCycleReport.usage_by_provider` reflects fallback provider usage

---

## Acceptance Criteria

| Criterion | Evidence |
|-----------|----------|
| Zero model string literals outside policy.rs in non-test code | `grep -r '"gpt-5\.\|"MiniMax\|"claude-opus\|"kimi-' lib/crates/ --include="*.rs" | grep -v 'policy\.rs\|#\[cfg(test)'` returns 0 results |
| Quota exhaustion triggers provider fallback in CLI backend | `cli_failure_is_quota_exhausted()` returns `true` for all known quota signals; does not return `true` for auth or transient errors |
| Quota exhaustion triggers provider fallback in LLM API path | `failover_eligible()` is `true` for `QuotaExceeded`; chain advances on that error |
| `raspberry status` shows per-lane model and provider | `ProviderHealth` appears in status output with `provider` and `model` fields |
| `AutodevCycleReport` has `usage_by_provider` | Field present in `AutodevReport` JSON schema and `raspberry-cli` output |
| Chain exhaustion produces structured error | All providers in chain returning `QuotaExceeded` or failing produces a named error variant, not a panic |

---

## Non-Goals

- Modifying the policy chain ordering (Write/Review/Synth profiles are the operator's decision)
- Adding API-key rotation or credential refresh mid-lane (separate infra concern)
- Changing the Sandbox trait or execution environment selection
- Modifying `fabro-agent` direct API routing (covered by plan 003)

---

## Open Questions (Require Operator Input)

1. **`gpt-5.4` vs `gpt-5.3-codex`**: Which is correct for codex-unblock and adjudication lanes? The hardcoded value in `render.rs` is `gpt-5.4`; the policy chain's Synth fallback is `gpt-5.3-codex`. Confirm intent before Milestone 1 lands.

2. **MiniMax/Kimi quota signals**: What error message strings does MiniMax return when quota is exhausted? Kimi? These must be added to `cli_failure_is_quota_exhausted()` to avoid silent fallback failure for those providers.

3. **Chain exhaustion behavior**: When all providers in the chain return quota errors, what is the desired lane outcome? `failed` with a structured error? `blocked` pending manual intervention?
