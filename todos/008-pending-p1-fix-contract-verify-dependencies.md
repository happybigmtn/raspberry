---
status: pending
priority: p1
issue_id: "008"
tags: [synthesis, autodev, protocols, dependencies]
dependencies: []
---

# Fix unsatisfiable contract-verify lane dependencies

Make synthesized `*-contract-verify` lanes depend on satisfiable milestone keys so they can
actually become ready.

## Problem Statement

`render.rs` currently synthesizes contract-verification lanes with dependencies that specify only a
unit id. The supervisor does not mark bare unit ids as satisfied, so these lanes remain blocked
forever even after the relevant implementation work completes.

## Findings

- Contract-verify lanes are emitted with `lane: None` and `milestone: None` dependencies:
  [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs#L404)
- Supervisor dependency satisfaction only recognizes `unit@milestone` and
  `unit:lane@milestone` keys:
  [evaluate.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/evaluate.rs#L741)
  [evaluate.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/evaluate.rs#L1350)

## Proposed Solutions

### Option 1: Depend on explicit terminal milestones

**Approach:** Resolve each implementor/consumer dependency to a concrete milestone such as the
source lane's managed milestone or the unit's `integrated` milestone.

**Pros:**
- Matches how the supervisor already reasons about satisfaction
- Keeps contract-verify behavior deterministic

**Cons:**
- Requires choosing a consistent milestone source for heterogeneous units

**Effort:** 1-2 hours

**Risk:** Low

---

### Option 2: Teach the supervisor bare-unit dependency semantics

**Approach:** Extend evaluation to treat a bare unit dependency as some terminal unit milestone.

**Pros:**
- More forgiving toward malformed manifests

**Cons:**
- Ambiguous semantics
- Risks masking bad synthesis output elsewhere

**Effort:** 2-4 hours

**Risk:** Medium

## Recommended Action

Implement Option 1. Synthesis should emit the explicit dependency keys the supervisor already
understands, and tests should prove a generated contract-verify lane becomes ready once its
producer and consumer units land.

## Technical Details

**Affected files:**
- [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs)
- [evaluate.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/evaluate.rs)

## Acceptance Criteria

- [ ] Generated contract-verify dependencies always include satisfiable milestones
- [ ] A regression test proves a synthesized contract-verify lane reaches `ready`
- [ ] Existing protocol generation behavior stays stable for already-valid blueprints

## Work Log

### 2026-03-25 - Review Discovery

**By:** Codex

**Actions:**
- Reviewed protocol lane synthesis and supervisor dependency evaluation
- Confirmed the generated dependency keys do not match the evaluator's satisfied-key model

**Learnings:**
- This is a correctness bug, not just a throughput issue
- The cleanest fix is in synthesis, not supervisor fallback logic
