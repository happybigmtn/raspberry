---
status: pending
priority: p2
issue_id: "011"
tags: [synthesis, autodev, review, performance]
dependencies: []
---

# Fix plan-review grouping and cut no-op render churn

Make synthesized aggregate review lanes group the right child units and stop rewriting unchanged
lane files on repeated synth passes.

## Problem Statement

The current aggregation heuristic still mis-groups multi-hyphen plan ids, which can skip intended
`*-plan-review` and `*-codex-review` lanes for real plans. Repeated `synth create` also rewrites
lane artifacts even when nothing materially changed, adding avoidable I/O and controller churn.

## Findings

- Plan grouping strips only one trailing path segment and breaks on names like
  `provably-fair-clean-docs` vs `provably-fair-integration-tests`:
  [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs#L487)
- No-op renders still call `render_lane()` and rewrite files even when lanes are equivalent:
  [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs#L48)
  [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs#L909)
  [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs#L2518)

## Proposed Solutions

### Option 1: Use explicit grouping metadata and skip equivalent lane writes

**Approach:** Carry a real plan/group id from planning into render, and short-circuit rendering when
`lane_equivalent()` is true.

**Pros:**
- Eliminates heuristic drift
- Reduces repeated synth I/O in steady-state autodev

**Cons:**
- Requires a small planning-to-render contract change

**Effort:** 3-6 hours

**Risk:** Medium

---

### Option 2: Improve the heuristic and compare file contents before write

**Approach:** Compute a better shared-prefix heuristic and add content checks before each atomic
write.

**Pros:**
- Smaller data-model change

**Cons:**
- Heuristics can still regress
- More hidden logic in render

**Effort:** 2-4 hours

**Risk:** Medium

## Recommended Action

Implement Option 1 if possible. The long-term fix is to stop guessing plan families from unit ids,
and the short-term render path should skip writing unchanged lane files.

## Technical Details

**Affected files:**
- [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs)
- [planning.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/planning.rs)

## Acceptance Criteria

- [ ] Multi-hyphen plan children synthesize the expected aggregate review lanes
- [ ] Repeated no-op synth passes avoid rewriting unchanged lane files
- [ ] Regression tests cover both grouping and no-op render behavior

## Work Log

### 2026-03-25 - Review Discovery

**By:** Codex

**Actions:**
- Reviewed aggregate review-lane generation and repeated render paths
- Validated that the current prefix heuristic is too weak for real multi-hyphen plan names

**Learnings:**
- This is partly a review-quality issue and partly a steady-state throughput issue
- A clean fix likely needs planning metadata, not a more clever string hack
