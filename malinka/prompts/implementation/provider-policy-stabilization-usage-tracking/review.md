# Usage Tracking Lane — Review

Review only the current slice for `provider-policy-stabilization-usage-tracking`.

Current Slice Contract:
Plan file:
- `genesis/plans/008-provider-policy-stabilization.md`

Child work item: `provider-policy-stabilization-usage-tracking`

Full plan context (read this for domain knowledge, design decisions, and specifications):

# Provider Policy Stabilization

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, model routing is stable, quota-aware, and centrally controlled in the binaries operators actually run. No execution path outside `fabro-model/src/policy.rs` makes provider-selection decisions, quota exhaustion triggers graceful fallback rather than collapsing active lanes, and operators can see which models are being used and which provider failures are blocking work.

The proof is: run autodev for 50 cycles. If any provider hits quota, the system falls through to the next provider without lane failure. `raspberry status` shows the current model routing for each active lane.

Provenance: This plan consolidates remaining items from `plans/032326-centralize-provider-policy-and-live-autodev-recovery.md` and the provider routing chaos identified in the assessment.

## Progress

- [ ] Audit all code paths that select models outside policy.rs
- [ ] Add quota detection and graceful fallback to fabro-llm providers
- [ ] Add provider health status to raspberry status output
- [ ] Add usage tracking per provider per cycle
- [ ] Live validation: quota exhaustion triggers fallback without lane failure

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: Provider policy must be a single function call, not distributed logic.
  Rationale: The assessment found model selection decisions leaking into `render.rs`, `cli.rs`, `synth.rs`, and `backend/cli.rs` even after the policy.rs centralization (commit 032326). Every leak is a potential Claude-plus-MiniMax mismatch crash.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: Overly aggressive fallback could route critical review work to a weak model (e.g., MiniMax reviewing security code). Mitigation: fallback chains must respect minimum capability requirements per profile (review cannot fall below Opus/Kimi tier).
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

The provider policy lives in `lib/crates/fabro-model/src/policy.rs`. It defines three profiles:

| Profile | Primary | Fallback 1 | Fallback 2 |
|---------|---------|------------|------------|
| Write | MiniMax M2.7 | Kimi K2.5 | Claude Opus 4.6 |
| Review | Kimi K2.5 | MiniMax M2.7 | Claude Opus 4.6 |
| Synth | Claude Opus 4.6 | GPT-5.3 Codex | MiniMax M2.7 |

The policy was centralized in commit `032326`, but live work on 2026-03-26 showed a broader issue: command entrypoints and runtime wiring can drift from the policy logic even when `policy.rs` itself is correct. Provider stabilization therefore includes making sure the binaries exposed to operators, synthesis, and autodev all honor the same routing rules.

Several consumers still make independent decisions:
- `fabro-cli/src/commands/synth.rs` — synth evolve provider selection
- `fabro-synthesis/src/render.rs` — review node model assignment
- `fabro-workflows/src/backend/cli.rs` — fallback chain execution

The LLM provider implementations live in `lib/crates/fabro-llm/src/provider/`:
- `anthropic.rs` — Claude (Opus, Sonnet, Haiku)
- `openai.rs` — GPT, Codex
- `gemini.rs` — Gemini
- `openai_compat.rs` — MiniMax, Kimi, and other OpenAI-compatible providers

```
policy.rs (source of truth)
     |
     v
render.rs ──> model assignment in workflow graphs
synth.rs ──> model selection for synthesis work
backend/cli.rs ──> runtime model selection + fallback
     |
     v
fabro-llm/src/provider/*.rs ──> actual API calls
```

## Milestones

### Milestone 1: Audit model selection leaks

Search for all code paths that reference specific model names (MiniMax, Opus, Claude, GPT, Kimi) outside of `policy.rs`. Document each leak with file path, line number, and whether it's a selection decision or just a display string.

Proof command:

    grep -rn "minimax\|opus\|claude\|gpt-5\|kimi" \
      lib/crates/fabro-cli/src/ \
      lib/crates/fabro-synthesis/src/ \
      lib/crates/fabro-workflows/src/ \
      --include="*.rs" | grep -v "policy.rs" | grep -v test | grep -v "//.*" | wc -l

Target: 0 selection decisions outside policy.rs.

### Milestone 2: Quota detection and graceful fallback

Add quota detection to `lib/crates/fabro-llm/src/provider/openai.rs` and `openai_compat.rs`. When a 429 (rate limit) or quota-exceeded error is returned, classify it as `ProviderQuotaExhausted` rather than a generic retry. In `lib/crates/fabro-workflows/src/backend/cli.rs`, route `ProviderQuotaExhausted` to the next provider in the fallback chain without consuming a retry count.

Proof command:

    cargo nextest run -p fabro-llm -- quota_exhausted
    cargo nextest run -p fabro-workflows -- provider_fallback quota

### Milestone 3: Provider health in status output

Add current provider routing and health status to `raspberry status` output. Show which model is assigned to each active lane and whether any providers are quota-limited.

Key file: `lib/crates/raspberry-supervisor/src/autodev.rs` (report generation)

Proof command:

    cargo nextest run -p raspberry-supervisor -- status provider

### Milestone 4: Usage tracking

Track tokens consumed per provider per autodev cycle. Add `usage_by_provider` to the autodev report with total tokens, cost estimate, and quota remaining (if available from provider API).

Proof command:

    cargo nextest run -p raspberry-supervisor -- usage_tracking

### Milestone 5: Live validation

Run proving-ground autodev with the binaries operators actually use. Artificially exhaust one provider's quota (or wait for natural exhaustion) and verify the system falls through to the next provider without lane failure or command-surface drift.

Proof command:

    target-local/release/raspberry autodev \
      --manifest /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml \
      --max-cycles 50 2>&1 | grep -E "quota|fallback|provider"

## Validation and Acceptance

The plan is done when:
- Zero model selection decisions exist outside policy.rs
- Quota exhaustion triggers graceful fallback
- `raspberry status` shows provider routing per lane
- Usage tracking per provider appears in autodev report


Workflow archetype: implement

Review profile: standard

Active plan:
- `genesis/plans/001-master-plan.md`

Active spec:
- `genesis/SPEC.md`

Proof commands:
- `cargo nextest run -p raspberry-supervisor -- usage_tracking`

Artifacts to write:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Nemesis-style security review
- Pass 1 — first-principles challenge: question trust boundaries, authority assumptions, and who can trigger the slice's dangerous actions
- Pass 2 — coupled-state review: identify paired state or protocol surfaces and check that every mutation path keeps them consistent or explains the asymmetry
- check state transitions that affect balances, commitments, randomness, payout safety, or replayability
- check secret handling, capability scoping, pairing/idempotence behavior, and privilege escalation paths
- check external-process control, operator safety, idempotent retries, and failure modes around service lifecycle

Focus on:
- slice scope discipline
- proof-gate coverage for the active slice
- touched-surface containment
- implementation and verification artifact quality
- remaining blockers before the next slice


Structural discipline
- if a new source file would exceed roughly 400 lines, split it before landing
- do not mix state transitions, input handling, rendering, and animation in one new file unless the prompt explicitly justifies that coupling
- if the slice cannot stay small, stop and update the artifacts to explain the next decomposition boundary instead of silently landing a monolith
Deterministic evidence:
- treat `.fabro-work/quality.md` as machine-generated truth about placeholder debt, warning debt, manual follow-up, and artifact mismatch risk
- if `.fabro-work/quality.md` says `quality_ready: no`, do not bless the slice as merge-ready


Score each dimension 0-10 and write `.fabro-work/promotion.md` in this exact form:

merge_ready: yes|no
manual_proof_pending: yes|no
completeness: <0-10>
correctness: <0-10>
convention: <0-10>
test_quality: <0-10>
reason: <one sentence>
next_action: <one sentence>

Scoring guide:
- completeness: 10=all deliverables present + all acceptance criteria met, 7=core present + 1-2 gaps, 4=missing deliverables, 0=skeleton
- correctness: 10=compiles + tests pass + edges handled, 7=tests pass + minor gaps, 4=some failures, 0=broken
- convention: 10=matches all project patterns, 7=minor deviations, 4=multiple violations, 0=ignores conventions
- test_quality: 10=tests import subject + verify all criteria, 7=most criteria tested, 4=structural only, 0=no tests

If `.fabro-work/contract.md` exists, verify EVERY acceptance criterion from it.
Any dimension below 6 = merge_ready: no.
If `.fabro-work/quality.md` says quality_ready: no = merge_ready: no.

For security-sensitive slices, append these mandatory fields exactly:
- seed_binding_complete: yes|no
- house_authority_preserved: yes|no
- proof_covers_edge_cases: yes|no
- layout_invariants_complete: yes|no
- slice_decomposition_respected: yes|no
If any mandatory security field is `no`, set `merge_ready: no`.

Review stage ownership:
- you may write or replace `.fabro-work/promotion.md` in this stage
- read `.fabro-work/quality.md` before deciding `merge_ready`
- when the slice is security-sensitive, perform a Nemesis-style pass: first-principles assumption challenge plus coupled-state consistency review
- include security findings in the review verdict when the slice touches trust boundaries, keys, funds, auth, control-plane behavior, or external process control
- prefer not to modify source code here unless a tiny correction is required to make the review judgment truthful
