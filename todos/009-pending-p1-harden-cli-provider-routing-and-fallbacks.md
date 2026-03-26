---
status: pending
priority: p1
issue_id: "009"
tags: [workflows, providers, autodev, reliability]
dependencies: []
---

# Harden CLI provider routing and same-stage fallback behavior

Align CLI-backed autodev stages with the same provider policy and same-stage recovery behavior as
the API path.

## Problem Statement

The CLI execution path currently makes provider decisions with a separate routing path, and timed
out or connectivity-stalled attempts do not immediately fall through to the next provider in the
same stage. That means live autodev can diverge from the intended provider order and waste whole
controller cycles waiting for replay instead of recovering inline.

## Findings

- CLI routing does not consume the same resolved provider chain as the API backend:
  [cli.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs#L639)
  [cli.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs#L700)
  [run.rs](/home/r/coding/fabro/lib/crates/fabro-cli/src/commands/run.rs#L1518)
- Timeout handling produces a generic attempt failure rather than a same-stage provider fallback:
  [cli.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs#L1228)
  [cli.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs#L1423)
  [cli.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs#L724)

## Proposed Solutions

### Option 1: Unify provider-chain resolution for API and CLI backends

**Approach:** Build a single ordered attempt chain upstream and pass it into both backends, then
expand fallback eligibility to include timeouts and transport failures.

**Pros:**
- Restores consistent behavior across provider execution modes
- Improves unattended recovery during provider incidents

**Cons:**
- Touches shared provider-selection plumbing

**Effort:** 3-5 hours

**Risk:** Medium

---

### Option 2: Keep separate paths but mirror the policy more fully

**Approach:** Re-implement API parity logic inside the CLI backend and broaden its fallback rules.

**Pros:**
- Less upstream interface churn

**Cons:**
- Duplicates policy and invites future drift

**Effort:** 4-6 hours

**Risk:** Medium

## Recommended Action

Implement Option 1. Provider order, strict-provider handling, and fallback eligibility should be a
single shared policy, and timed-out primary attempts should fall through in the same stage.

## Technical Details

**Affected files:**
- [cli.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs)
- [policy.rs](/home/r/coding/fabro/lib/crates/fabro-model/src/policy.rs)
- [run.rs](/home/r/coding/fabro/lib/crates/fabro-cli/src/commands/run.rs)

## Acceptance Criteria

- [ ] CLI and API backends choose the same ordered providers for the same request
- [ ] Timed-out or connectivity-failed CLI attempts can fall through to the next provider
- [ ] Regression tests cover provider-order parity and timeout-triggered fallback

## Work Log

### 2026-03-25 - Review Discovery

**By:** Codex

**Actions:**
- Reviewed provider routing in the CLI backend and compared it with API-side policy wiring
- Traced timeout handling through the attempt loop

**Learnings:**
- The current split makes provider behavior harder to reason about end to end
- Same-stage fallback is one of the highest-leverage throughput improvements during provider trouble
