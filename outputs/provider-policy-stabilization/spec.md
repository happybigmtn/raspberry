# Provider Policy Stabilization — Capability Spec

**Status:** Draft — first honest reviewed slice  
**Plan:** `genesis/plans/008-provider-policy-stabilization.md`  
**Author:** Genesis  
**Date:** 2026-03-26

---

## Purpose / User-Visible Outcome

After this work lands, every model-routing decision in the binaries operators actually run is made by a single function call to `fabro_model::policy::automation_chain()`. No hardcoded model strings appear in workflow graphs, synthesis rendering, or CLI backend execution. When any provider exhausts its quota, the system falls through to the next provider in the policy chain without collapsing the active lane, and `raspberry status` shows which model each lane is using and which providers are quota-limited.

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
- Adding new providers to the policy chain (that is a separate policy decision)
- LLM API-level retries (those live in `fabro-llm/src/retry.rs` and are already correct for rate-limit 429s)
- `fabro-agent` CLI-only model routing (covered by separate exec-path work in plan 003)

---

## Current State

### The policy source of truth

`lib/crates/fabro-model/src/policy.rs` defines `automation_chain()` returning a `&'static [ModelTarget]` for each `AutomationProfile` (Write, Review, Synth). The chain includes provider + model string pairs. Functions `automation_primary_target()`, `automation_fallback_targets()`, and `automation_fallback_map()` expose read access. No mutation API exists and none is needed.

### Leaking hardcoded strings (confirmed)

The following locations reference model strings directly instead of calling `automation_chain()` or its variants:

**`fabro-synthesis/src/render.rs`:**

| Function | Lines | Hardcoded Value | Severity |
|---|---|---|---|
| `challenge_target_for_lane()` | 2011–2021 | `Provider::OpenAi`, `"gpt-5.4"` for codex-unblock lanes | Medium — still selection logic outside policy |
| `review_target_for_lane()` | 2024–2034 | `Provider::OpenAi`, `"gpt-5.4"` for codex-unblock lanes | Medium |
| `recurring_report_primary_target_for_lane()` | 2050–2084 | Mixed: `"MiniMax-M2.7-highspeed"` on `Provider::Minimax`, `"claude-opus-4-6"` on `Provider::Anthropic`, `"gpt-5.4"` on `Provider::OpenAi` with hardcoded fallback chains per lane family | **High — direct model/provider hardcoding in workflow graph generation** |
| `render_workflow_graph()` closure | 2487–2490 | `Provider::OpenAi`, `"gpt-5.4"` for llm_target on unblock lanes | Medium — cosmetic equivalent of challenge/review hardcoding |

**`fabro-workflows/src/backend/cli.rs`:**

| Location | Mechanism | Assessment |
|---|---|---|
| Lines 799–810 | `cli_failure_is_retryable_for_fallback()` does string matching on stderr to decide whether to advance the fallback chain | Works, but fragile. Quota detection relies on substring matching (`"you've hit your limit"`, `"usage limit has been reached"`). Not integrated with the structured `ProviderErrorKind::QuotaExceeded` that `fabro-llm` already defines. |

**`fabro-llm/src/error.rs`** (already correct):

`ProviderErrorKind::QuotaExceeded` is defined, `failover_eligible()` returns `true` for it, and `error_from_status_code()` maps 429 to `RateLimit` (retryable) while message inspection maps quota-exhausted strings to `QuotaExceeded`. The LLM API path is properly instrumented.

**`fabro-llm/src/retry.rs`** (already correct):

`RateLimit` (429) is `retryable()`. `QuotaExceeded` is NOT `retryable()` (same-provider retry is futile), but IS `failover_eligible()` (different provider will have independent quota). This is the correct design.

### What is missing end-to-end

1. **Structured quota failover in CLI backend**: `backend/cli.rs` runs CLI tools (claude-code, codex, pi) rather than calling the LLM API directly. When the CLI reports a quota error in its stderr, the current string-matching detector (`cli_failure_is_retryable_for_fallback`) catches it and advances the fallback chain — but this path is invisible to `fabro-llm`'s `ProviderErrorKind::QuotaExceeded`. The two systems are decoupled by architecture (CLI vs API), which is fine, but the quota signal in the CLI path is fragile (substring matching on stderr).

2. **Provider health in status output**: `DispatchOutcome` (in `raspberry-supervisor/src/dispatch.rs`) records `exit_status`, `stdout`, `stderr`, and `fabro_run_id` — but not which model was used, which provider was hit, or whether quota was encountered. The `AutodevReport` has no `usage_by_provider` field.

3. **Usage tracking per provider**: `last_usage_summary: Option<String>` exists in lane state (`evaluate.rs`, `program_state.rs`) but is a freeform string with no per-provider breakdown. No `usage_by_provider` field exists in `AutodevCycleReport` or `AutodevReport`.

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

When a CLI invocation returns a failure whose stderr contains a quota-exhausted signal, `cli_failure_is_retryable_for_fallback()` must return `true` and the CLI backend must advance to the next provider in the chain without counting it against the per-provider retry budget.

When an LLM API call returns `SdkError::Provider { kind: QuotaExceeded, .. }`, the caller (which may be `backend/cli.rs` using the API path or `fabro-agent` directly) must advance to the next provider in the chain.

Quota-exhausted is **not** retried within the same provider — the quota state will not clear within a lane's execution window.

### Provider health and usage reporting contract

`raspberry status` output must include, per active lane:
- The lane key
- The assigned model (e.g., `MiniMax-M2.7-highspeed`)
- The provider (e.g., `minimax`)
- Any provider health signal: `ok`, `quota_limited`, `auth_error`, `unavailable`

`AutodevCycleReport` must include a `usage_by_provider` map with:
- `provider`: string identifier
- `input_tokens`: cumulative for the cycle
- `output_tokens`: cumulative for the cycle
- `estimated_cost_usd`: approximate cost
- `quota_remaining`: if reported by the provider API, else `null`

---

## Adoption Path

1. **Audit gate** (Milestone 1): confirm the grep target returns 0 non-comment, non-test results for model string literals outside policy.rs.
2. **Render gate** (Milestone 1 continued): replace all hardcoded `ModelTarget` constructions in `render.rs` with calls to `automation_primary_target()` and `automation_fallback_targets()`, parameterized by lane profile.
3. **CLI backend gate** (Milestone 2): add `ProviderErrorKind::QuotaExceeded` handling to `cli_failure_is_retryable_for_fallback()` by checking for additional quota signals in stderr, and document that the CLI path is a parallel failover system to the API path.
4. **Status gate** (Milestone 3): add `ProviderHealth` struct and `provider_health` field to `DispatchOutcome`; surface in `raspberry status` output.
5. **Usage gate** (Milestone 4): add `usage_by_provider` to `AutodevCycleReport`; wire from `provider_used.json` written by the CLI backend and from LLM API usage response fields.
6. **Live gate** (Milestone 5): run 50-cycle autodev and observe fallback behavior.

---

## Acceptance Criteria

| Criterion | Evidence |
|---|---|
| Zero model string literals outside policy.rs in non-test code | `grep` returns 0 results for the target pattern |
| Quota exhaustion triggers provider fallback in CLI backend | `cli_failure_is_retryable_for_fallback()` returns true for quota signals; fallback counter increments |
| Quota exhaustion triggers provider fallback in LLM API path | `failover_eligible()` is true for `QuotaExceeded`; chain advances on that error |
| `raspberry status` shows per-lane model and provider | `ProviderHealth` appears in status output |
| `AutodevCycleReport` has `usage_by_provider` | Field present in `AutodevReport` JSON |
| 50-cycle live validation succeeds | No lane failures attributable to quota without fallback |

---

## Non-Goals

- Modifying the policy chain ordering (Write/Review/Synth profiles are the decision of the operator and are not part of this spec)
- Adding API-key rotation or credential refresh mid-lane (separate infra concern)
- Changing the Sandbox trait or execution environment selection
