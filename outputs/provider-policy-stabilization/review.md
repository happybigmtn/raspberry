# Provider Policy Stabilization — First Slice Review

**Lane:** `provider-policy-stabilization`  
**Plan:** `genesis/plans/008-provider-policy-stabilization.md`  
**Review date:** 2026-03-26  
**Reviewer:** Genesis (first honest reviewed slice)

---

## Evidence Gathered

This review is based on direct inspection of the current codebase without running tests or building the project. The following surfaces were examined:

| Surface | Files inspected | Lines |
|---|---|---|
| Policy source of truth | `lib/crates/fabro-model/src/policy.rs` | 98 |
| Synthesis model assignment | `lib/crates/fabro-synthesis/src/render.rs` | ~2100 (grep + targeted read) |
| CLI backend fallback logic | `lib/crates/fabro-workflows/src/backend/cli.rs` | ~2450 (grep + targeted read) |
| LLM error taxonomy | `lib/crates/fabro-llm/src/error.rs` | ~800 (full read) |
| LLM retry logic | `lib/crates/fabro-llm/src/retry.rs` | ~200 (full read) |
| Provider implementations | `lib/crates/fabro-llm/src/providers/*.rs` | grep + targeted reads |
| Raspberry status output | `lib/crates/raspberry-supervisor/src/autodev.rs` | ~4200 (grep + targeted read) |
| Dispatch outcome | `lib/crates/raspberry-supervisor/src/dispatch.rs` | ~550 (full read) |

---

## Milestone 1 Assessment: Audit Model Selection Leaks

### What exists today

`lib/crates/fabro-model/src/policy.rs` is clean. It exports `automation_chain()`, `automation_primary_target()`, `automation_fallback_targets()`, and `automation_fallback_map()`. All model/provider strings are defined as `&'static str` constants in the chain arrays. The module has tests covering all three profiles.

### What is leaking

**`fabro-synthesis/src/render.rs`** — four confirmed leak points:

1. **`recurring_report_primary_target_for_lane()` (lines 2050–2084)**  
   Returns hardcoded `ModelTarget` values keyed on lane ID string patterns:
   - Parent holistic review minimax lanes → `Provider::Minimax`, `"MiniMax-M2.7-highspeed"`
   - Non-minimax parent lanes → `Provider::Anthropic`, `"claude-opus-4-6"`
   - Codex review lanes → `Provider::OpenAi`, `"gpt-5.4"`  
   Also emits hardcoded fallback sections into workflow graphs (`"[llm.fallbacks]\nanthropic = [\"gpt-5.4\"]"` and inverse) rather than reading from `automation_fallback_map()`.

2. **`challenge_target_for_lane()` (lines 2011–2021)**  
   Returns `Provider::OpenAi, "gpt-5.4"` for codex-unblock lanes. No call to `automation_chain()`.

3. **`review_target_for_lane()` (lines 2024–2034)**  
   Same pattern as challenge: hardcodes `gpt-5.4` on `Provider::OpenAi` for codex-unblock lanes.

4. **Workflow graph template literals (lines 1917–1990)**  
   The Graphviz digraph strings embed `{model}` and `{provider}` attributes for `#review`, `#challenge`, `#deep_review`, `#escalation` nodes. These are populated from the return values of the functions above, so fixing the functions fixes the graphs.

**Assessment: High confidence leak.** The synthesis path generates workflow TOML files that land in the repo. Any hardcoded model in a generated workflow graph means `raspberry autodev` will route those lanes through the hardcoded provider regardless of what `policy.rs` says.

**`fabro-workflows/src/backend/cli.rs`** — CLI fallback uses string matching:

`cli_failure_is_retryable_for_fallback()` (lines 799–810) checks stderr strings for quota signals:
```rust
lower.contains("you've hit your limit")
    || lower.contains("usage limit has been reached")
    || lower.contains("rate_limit")
    || ...
```
This works but is fragile. The signals checked are all quota-adjacent, but the function conflates "quota exhausted" with "auth error" and "timeout". The plan's requirement is that "quota exhaustion triggers graceful fallback" — the string-matching approach satisfies the requirement but doesn't integrate with the structured `ProviderErrorKind::QuotaExceeded` that `fabro-llm` already defines.

**`fabro-llm/src/error.rs` and `fabro-llm/src/retry.rs`** — Correctly instrumented:

`ProviderErrorKind::QuotaExceeded` exists and is used in `error_from_status_code()` (message inspection path). `failover_eligible()` returns `true` for it. The API path is properly wired. No changes needed here.

**Other surfaces checked and found clean:**

- `fabro-cli/src/commands/synth.rs`: References to `"claude"`, `"MiniMax-M2.7-highspeed"` etc. are in command template strings for the `claude` and `pi` CLI invocations — these are provider/CLI tool selection, not model routing. The model string is interpolated from `policy.rs`-derived values in the synthesis path. (One exception: `run_opus_decomposition()` uses the string `"opus-{}"` as a path template convention, not a model selector — benign.)
- `fabro-agent/src/`: No model string hardcoding found outside policy.rs.
- `raspberry-supervisor/src/`: No model string hardcoding found outside policy.rs.

---

## Milestone 2 Assessment: Quota Detection and Graceful Fallback

### CLI path (fabro-workflows backend/cli.rs)

The CLI backend already has a fallback chain execution mechanism (lines 758–799, `build_cli_attempt_targets()`). It reads from `automation_fallback_targets()` via `central_policy_fallback_targets()`. When a CLI invocation fails, `cli_failure_is_retryable_for_fallback()` decides whether to advance.

**Gap identified:** The string-matching signals for quota (`"you've hit your limit"`, `"usage limit has been reached"`) are present, but there is no structured `QuotaExceeded` classification. If a new quota signal appears (e.g., a different wording from MiniMax or Kimi), it won't be caught without a code change.

**Fix needed:** Extend `cli_failure_is_retryable_for_fallback()` to cover additional quota signals. Additionally, the function currently conflates quota errors with auth/timeouts — consider separating a `cli_failure_is_quota_exhausted()` helper for clarity and for future wiring into a `ProviderHealth` struct.

### API path (fabro-llm)

Already correct. `failover_eligible()` on `SdkError` returns `true` for `QuotaExceeded`. No code changes required — only validation testing.

---

## Milestone 3 Assessment: Provider Health in Status Output

**Current state:** `DispatchOutcome` (in `raspberry-supervisor/src/dispatch.rs`) has:
```rust
pub struct DispatchOutcome {
    pub lane_key: String,
    pub exit_status: i32,
    pub fabro_run_id: Option<String>,
    pub stdout: String,
    pub stderr: String,
}
```

No model, provider, or health signal. `raspberry status` reads `AutodevCurrentSnapshot` from `autodev.rs`, which has lane key lists and blocker counts but no model/provider routing information.

**Gap identified:** No field exists to carry provider health information from the dispatch layer up to the status surface. This requires:
1. Adding a `ProviderHealth` struct to `dispatch.rs` or a shared module
2. Populating it from `provider_used.json` written by `backend/cli.rs` (which already records `requested_provider`, `requested_model`, `provider`, `model`, and `fallback_reason`)
3. Including it in `AutodevCurrentSnapshot` and rendering it in `raspberry status`

---

## Milestone 4 Assessment: Usage Tracking Per Provider Per Cycle

**Current state:** `last_usage_summary: Option<String>` exists in `program_state.rs` lane state and `evaluate.rs`. It is a freeform string with no per-provider breakdown. `AutodevCycleReport` has no usage fields.

**Gap identified:** No per-provider token tracking. The `provider_used.json` written by `backend/cli.rs` contains the requested and actual provider/model but no usage tokens. The `parse_codex_ndjson()`, `parse_claude_ndjson()`, and `parse_pi_json()` functions in `backend/cli.rs` extract token counts from CLI output, but these are not aggregated upward.

**Fix needed:**  
1. Aggregate token counts from `provider_used.json` per provider per cycle  
2. Add `usage_by_provider: BTreeMap<Provider, ProviderCycleUsage>` to `AutodevCycleReport`  
3. Write a summary into the autodev cycle JSON report

---

## Milestone 5 Assessment: Live Validation

**Cannot assess without running.** Requires proving-ground autodev execution against a live repo. This is the gate criterion and must be run after Milestones 1–4 are complete.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Fallback chain bypasses minimum capability floor (e.g., MiniMax reviewing security code) | Medium | High | The plan's Decision Log explicitly calls this out. The policy chains are ordered with capability floors (Review: Kimi → MiniMax → Opus). The implementation must not reorder or skip the floor. |
| Substring quota detection misses new quota messages from providers | Medium | Medium | After implementing Milestone 2, add a test that simulates quota error output and verify fallback fires. |
| `provider_used.json` is not written on early-exit paths (e.g., sandbox failure before CLI invocation) | Low | Low | Health shows as `unknown` rather than `ok`; acceptable for v1 |
| Per-cycle usage aggregation requires parsing `provider_used.json` files across concurrent lane runs | Low | Medium | Use a shared append-only log format and aggregate at cycle boundary, not per-lane |

---

## Implementation Order Recommendation

1. **Fix render.rs hardcoded targets** — highest-confidence change, directly restores the policy.rs invariant
2. **Add `provider_used.json` aggregation to `AutodevCycleReport`** — enables Milestone 4 incrementally
3. **Add `ProviderHealth` to `DispatchOutcome` and `AutodevCurrentSnapshot`** — enables Milestone 3
4. **Extend `cli_failure_is_retryable_for_fallback()` with broader quota signals** — stabilizes Milestone 2
5. **Write validation tests** for quota fallback in both CLI and API paths
6. **Live validation run** — Milestone 5 gate

---

## Open Questions

1. Should `provider_used.json` be written to the run directory (persisted across resume) or only to the cycle report (ephemeral)? The current code writes it to `stage_dir`, which is ephemeral per run. For usage tracking across cycles, the aggregator needs either persistent writes or a pipeline from the run log.

2. Does `raspberry status` need to show historical provider health, or only the current cycle? The spec says "current model routing for each active lane" — suggest current-cycle-only for v1, with historical trend data as a future enhancement.

3. The `gpt-5.4` model string appears in multiple places in `render.rs` but the policy chain uses `"gpt-5.3-codex"` as the Synth fallback. Is `gpt-5.4` intentional (a newer model not yet in policy.rs) or a copy-paste error? **This needs operator confirmation before the hardcoded values are replaced with policy.rs calls.**
