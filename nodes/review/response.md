## Review Complete

### Artifacts Written
- `outputs/provider-policy-stabilization/review.md` — Comprehensive nemesis-style security review

### Key Findings

| Finding | Severity | Location |
|---------|----------|----------|
| **Hardcoded model selection** | Critical | `fabro-synthesis/src/render.rs:2058-2079` |
| CLI quota detection gap | Medium | `fabro-workflows/src/backend/cli.rs:799-817` |
| Missing provider health in status | Medium | `raspberry-supervisor/src/autodev.rs` |
| Missing usage tracking | Medium | `raspberry-supervisor/src/autodev.rs` |

### Verified Invariants
- ✅ **API path correctly handles quota** — Uses `sdk_err.failover_eligible()` which returns true for `QuotaExceeded`
- ✅ **Policy.rs chains are well-structured** — `WRITE_CHAIN`, `REVIEW_CHAIN`, `SYNTH_CHAIN` properly defined
- ✅ **CLI backend uses central policy for fallback chains** — `central_policy_fallback_targets()` called correctly
- ✅ **Process cleanup safe** — `cleanup_cli_scratch()` called before fallback attempts

### Security Review Summary

**Pass 1 (Trust Boundaries):**
- Authority drift identified: synthesis layer bypassing policy layer for model selection
- Error message text is not a stable interface for quota detection
- No cryptographic provider attestation (acceptable for Phase 0)

**Pass 2 (Coupled State):**
- **Split-brain risk:** `render.rs` hardcoded MiniMax for holistic review lanes vs. REVIEW_CHAIN ordering (Kimi first)
- Quota state must not consume retry budget when triggering fallback
- Missing per-lane provider context prevents debugging provider-specific failures

### Verdict
**Spec is sound.** Implementation work is appropriately scoped for Phase 0. The critical blocker (hardcoded model selection) is clearly identified and fixable by replacing direct `ModelTarget` construction with `automation_primary_target()` calls.