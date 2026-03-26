Done. I've bootstrapped the first honest reviewed slice for the Provider Policy Stabilization lane.

## Artifacts Created

| Artifact | Path | Size |
|----------|------|------|
| spec.md | `outputs/provider-policy-stabilization/spec.md` | 209 lines |
| review.md | `outputs/provider-policy-stabilization/review.md` | 365 lines |

## Audit Summary

**9 findings** documented in review.md:

### Critical (1)
1. **`fabro-synthesis/src/render.rs:2055-2090`** — `recurring_report_primary_target_for_lane()` hardcodes `Provider::Minimax`, `Provider::Anthropic`, `Provider::OpenAi` with specific models, bypassing `policy.rs` for holistic review lanes.

### Medium (3)
2. **`fabro-workflows/src/backend/cli.rs`** — `cli_failure_is_retryable_for_fallback()` checks for CLI-specific quota messages ("you've hit your limit") but not API-level `QuotaExceeded` errors.
3. **`raspberry-supervisor/src/autodev.rs`** — `AutodevCurrentSnapshot` lacks per-lane model/provider/health fields.
4. **`raspberry-supervisor/src/autodev.rs`** — `AutodevCycleReport` lacks `usage_by_provider` tracking.

### Low/Informational (5)
5. Stylesheet tests with hardcoded model names (test fixtures only)
6. Transform tests with hardcoded model names (test fixtures only)
7. Integration test fixtures with model names (test fixtures only)

### Positive Findings
- `fabro-llm/src/error.rs` already has `ProviderErrorKind::QuotaExceeded` and `failover_eligible()` correctly returns `true` for quota errors
- CLI backend's `build_cli_attempt_targets()` correctly uses `central_policy_fallback_targets()` from policy.rs

## Spec Highlights

- **Target:** Zero model selection decisions outside `policy.rs`
- **Acceptance criteria:** 4 measurable gates (leak count, quota fallback, status output, usage tracking)
- **Migration phases:** 3 phases (audit+seal, status+tracking, live validation)
- **Policy chains preserved:** MiniMax→Kimi→Opus (Write), Kimi→MiniMax→Opus (Review), Opus→Codex→MiniMax (Synth)