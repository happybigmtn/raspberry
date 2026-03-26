# Provider Policy Stabilization — Specification

## Status

Draft — First honest reviewed slice (2026-03-26)

## Purpose / User-Visible Outcome

After this work lands, model routing is stable, quota-aware, and centrally controlled. No execution path outside `fabro-model/src/policy.rs` makes provider-selection decisions. When a provider hits quota, the system falls through to the next provider in the fallback chain without collapsing active lanes. Operators can see which models are being used per lane via `raspberry status` and which provider failures are blocking work.

## Whole-System Goal

The autodev loop runs 50 cycles without lane failure caused by provider selection drift or quota exhaustion. `raspberry status` shows the current model routing for each active lane and provider health status.

## Scope

This spec covers the first implementation slice (Phase 0) for provider policy stabilization:

1. **Audit and seal model selection leaks** — identify all code paths that reference specific models outside policy.rs
2. **Quota detection integration** — wire `ProviderErrorKind::QuotaExceeded` into the CLI fallback path
3. **Provider health in status** — add provider routing and quota-limited status to raspberry status output
4. **Usage tracking** — add per-provider token/cost tracking to autodev report

## Current State

### Policy Architecture

The provider policy lives in `lib/crates/fabro-model/src/policy.rs`. It defines three profiles:

| Profile | Primary | Fallback 1 | Fallback 2 |
|---------|---------|------------|------------|
| Write | MiniMax M2.7 (pi) | Kimi K2.5 (pi) | Claude Opus 4.6 (claude CLI) |
| Review | Kimi K2.5 (pi) | MiniMax M2.7 (pi) | Claude Opus 4.6 (claude CLI) |
| Synth | Claude Opus 4.6 (claude CLI) | GPT-5.3 Codex (codex CLI) | MiniMax M2.7 (pi) |

Key functions:
- `automation_chain(profile)` — returns full fallback chain
- `automation_primary_target(profile)` — returns first target
- `automation_fallback_targets(profile)` — returns fallbacks
- `automation_profile_for_target(provider, model)` — reverse lookup

### Existing Quota Detection

The `fabro-llm/src/error.rs` already defines:
- `ProviderErrorKind::QuotaExceeded` — distinct from `RateLimit`
- `failover_eligible()` — returns true for `QuotaExceeded`, meaning a different provider won't share the same quota
- 429 errors map to `RateLimit`, which is retryable (same-provider backoff)
- Message-based classification can identify quota errors (e.g., "usage limit reached", "quota exceeded")

### CLI Backend Fallback

`fabro-workflows/src/backend/cli.rs` implements fallback via:
- `build_cli_attempt_targets()` — builds fallback chain from policy
- `cli_failure_is_retryable_for_fallback()` — determines if a CLI failure triggers fallback
- Fallback reasons include: auth failures, rate limits, "you've hit your limit", "usage limit has been reached"

### Known Leaks

**Critical** (selection decisions, not test code):
1. `fabro-synthesis/src/render.rs:2055-2090` — `recurring_report_primary_target_for_lane()` hardcodes `Provider::Minimax` with `MiniMax-M2.7-highspeed`, `Provider::Anthropic` with `claude-opus-4-6`, and `Provider::OpenAi` with `gpt-5.4`. This bypasses policy.rs entirely for holistic review lanes.
2. `fabro-synthesis/src/render.rs:2026-2040` — `challenge_target_for_lane()` and `review_target_for_lane()` hardcode `Provider::OpenAi` with `gpt-5.4` for codex-unblock lanes.
3. `fabro-workflows/src/backend/cli.rs` — `cli_failure_is_retryable_for_fallback()` checks for "you've hit your limit" and "usage limit has been reached" in lowercase error details, but does not specifically check for `QuotaExceeded` from the API path. The CLI and API paths may have different error formats.

**Non-critical** (test fixtures, display strings):
- `fabro-workflows/src/stylesheet.rs:86-210` — Test fixtures with hardcoded model names (sonnet, opus, gpt)
- `fabro-workflows/src/transform.rs:681-790` — Transform tests with hardcoded model names
- Various test files in `fabro-workflows/tests/integration.rs` — Test fixtures with model names

## Target Architecture

### Invariant: Single Source of Truth

All model selection decisions must flow through `policy.rs`:

```
policy.rs (automation_chain, automation_primary_target, automation_fallback_targets)
     |
     +---> fabro-synthesis/src/render.rs (model assignment in workflow graphs)
     +---> fabro-workflows/src/backend/cli.rs (runtime model selection + fallback)
     +---> fabro-cli/src/commands/synth.rs (synthesis model selection)
     |
     v
fabro-llm/src/provider/*.rs (actual API calls)
```

Any code that references a specific provider or model name outside of `policy.rs` must be treated as a bug.

### Fallback Chain Behavior

1. When CLI command fails with a retryable error (including quota exhaustion), try next target in chain
2. When API call fails with `QuotaExceeded`, trigger provider failover (do not consume retry count)
3. Fallback chain respects profile minimum capability requirements:
   - Review cannot fall below Opus/Kimi tier (enforced by chain order)
   - Synth prefers Opus first, Codex second, MiniMax third

### Status Output Contract

`raspberry status` output must include:

```yaml
lanes:
  - id: lane-123
    status: running
    model: kimi-k2.5
    provider: kimi
    provider_health: healthy  # or "quota_limited", "unavailable"
  - id: lane-456
    status: blocked
    blocked_reason: "provider quota exhausted: minimax"
```

### Usage Tracking Contract

Autodev report must include:

```yaml
cycles:
  - cycle: 5
    usage_by_provider:
      - provider: kimi
        total_tokens: 45000
        cost_estimate_usd: 0.45
        quota_remaining: unknown  # if provider API provides it
      - provider: anthropic
        total_tokens: 12000
        cost_estimate_usd: 1.20
        quota_remaining: "250000000"  # from API headers if available
```

## Acceptance Criteria

### AC1: Zero Model Selection Leaks

```
grep -rn "Provider::(Minimax|Kimi|Anthropic|OpenAi|Gemini)" \
  lib/crates/fabro-synthesis/src/ \
  lib/crates/fabro-workflows/src/ \
  --include="*.rs" \
  | grep -v "_test\|#\[test\]\|// " \
  | grep -v "policy.rs" \
  | wc -l
```

Target: 0 matches in production code paths.

### AC2: Quota Exhaustion Triggers Fallback

- When `fabro-llm` returns `ProviderErrorKind::QuotaExceeded`, the CLI backend must fall through to the next provider in the fallback chain
- The fallback must not consume retry count (quota exhaustion is not a transient error)
- Lane must not fail — work continues on next provider

Proof: Run autodev, artificially exhaust one provider's quota, observe fallback without lane failure.

### AC3: Provider Health in Status

`raspberry status --manifest <prog.yaml>` shows:
- Current model and provider per running lane
- Provider health status: `healthy`, `quota_limited`, or `unavailable`

### AC4: Usage Tracking in Report

Autodev report at `.raspberry/autodev-report.json` includes:
- `usage_by_provider` array per cycle
- `total_tokens`, `cost_estimate_usd`, `quota_remaining` per provider

## Non-Goals

- Changing the policy chains themselves (MiniMax/Kimi/Opus ordering is out of scope)
- Implementing provider health probes (health check is inference-only, not active probing)
- Multi-provider parallel requests (single provider at a time)

## Migration Phases

### Phase 1: Audit and Seal (this slice)

1. Audit all production code paths for model selection leaks
2. Document findings in review.md
3. Fix critical leaks in `fabro-synthesis/src/render.rs`
4. Ensure `cli_failure_is_retryable_for_fallback()` catches quota errors

### Phase 2: Status and Tracking (next slice)

1. Add provider routing to `raspberry status` output
2. Add `usage_by_provider` to autodev report
3. Wire quota-limited status from provider health checks

### Phase 3: Live Validation (follow-up slice)

1. Run 50-cycle autodev with artificial quota exhaustion
2. Verify fallback without lane failure
3. Verify status output accuracy

## Consequences

### Positive

- Provider failures no longer collapse active lanes
- Operator visibility into model routing per lane
- Single source of truth eliminates drift between policy and execution
- Cost tracking enables quota budget management

### Risks

- Overly aggressive fallback could route review work to weak model (mitigated by chain order)
- Status output may be stale if provider health is not re-checked frequently (acceptable for Phase 1)

## What This Supersedes

This spec consolidates remaining items from `plans/032326-centralize-provider-policy-and-live-autodev-recovery.md` which was partially completed but had remaining drift between policy.rs and consumers.
