Both durable artifacts are written. Here is a summary of what the evidence shows and what the artifacts capture:

---

## What was found and documented

### `spec.md` — What the system should do when this is done

- **Centralization**: `fabro-model/src/policy.rs` is the sole source of truth. Every other call site must call `automation_chain()`, `automation_primary_target()`, or `automation_fallback_targets()`. No `ModelTarget` literals outside policy.rs (except tests).
- **Quota failover**: `ProviderErrorKind::QuotaExceeded` in `fabro-llm` is already `failover_eligible = true`. The API path is wired. The CLI path (which runs `claude-code`/`codex`/`pi` as subprocesses) uses fragile substring matching that works today but is not integrated with the structured error type.
- **Status output**: `raspberry status` must show per-lane model + provider + health flag (`ok` / `quota_limited` / etc.). `DispatchOutcome` needs a `ProviderHealth` field.
- **Usage tracking**: `AutodevCycleReport` needs a `usage_by_provider` map. The raw data exists in `provider_used.json` written by `backend/cli.rs` and in CLI output parsers, but nothing aggregates it upward.

### `review.md` — What is true today and what needs to change

Four confirmed hardcoded model leaks in `fabro-synthesis/src/render.rs`:

| Location | Hardcoded value | Severity |
|---|---|---|
| `recurring_report_primary_target_for_lane()` lines 2050–2084 | `"MiniMax-M2.7-highspeed"`, `"claude-opus-4-6"`, `"gpt-5.4"` with hardcoded per-family fallback chains | **High** — generates workflow TOML that bypasses policy.rs at runtime |
| `challenge_target_for_lane()` lines 2011–2021 | `"gpt-5.4"` on `Provider::OpenAi` for codex-unblock lanes | Medium |
| `review_target_for_lane()` lines 2024–2034 | Same | Medium |
| Workflow graph template literals lines 1917–1990 | Populated from the above | Inherits from above |

One open question flagged for operator confirmation: the hardcoded `"gpt-5.4"` string appears in render.rs but the policy chain uses `"gpt-5.3-codex"` — need to confirm whether `"gpt-5.4"` is intentional or a copy-paste error before replacing with policy.rs calls.