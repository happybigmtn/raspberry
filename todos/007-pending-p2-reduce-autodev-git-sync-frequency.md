---
status: pending
priority: p2
issue_id: "007"
tags: [raspberry, autodev, git, throughput]
dependencies: []
---

# Reduce autodev git sync frequency

Make target-repo sync smarter so busy autodev loops do not run `git fetch origin` after nearly
every dispatch cycle.

## Problem Statement

The autodev loop comments say target-repo sync should only happen when completion may have changed,
but the current implementation still fetches after every non-empty dispatch or evolve. On an active
program that dispatches work every cycle, this turns repo sync into a constant per-cycle tax even
when no integration artifact could have landed.

## Findings

- The controller tracks `last_complete_count` and describes sync as completion-driven:
  [autodev.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/autodev.rs#L369)
- In practice it still calls `sync_target_repo_to_origin()` whenever `dispatched` is non-empty or
  `evolved` is true:
  [autodev.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/autodev.rs#L572)
- `sync_target_repo_to_origin()` performs a real `git fetch origin --quiet` plus branch/cleanliness
  checks:
  [autodev.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/autodev.rs#L1193)

## Proposed Solutions

### Option 1: Sync only after integration-capable completions

**Approach:** Gate sync on observed completion deltas or dispatch outcomes that can actually land
integration artifacts.

**Pros:**
- Preserves correctness while dropping most unnecessary fetches
- Small conceptual change

**Cons:**
- Needs a reliable signal for “an integration may have landed”

**Effort:** 2-4 hours

**Risk:** Medium

---

### Option 2: Add sync cooldown/backoff

**Approach:** Even when sync is needed, cap it to a minimum interval.

**Pros:**
- Easy to implement
- Bounds worst-case fetch churn

**Cons:**
- Can delay artifact visibility slightly

**Effort:** 1-2 hours

**Risk:** Low

## Recommended Action

Combine both: add an integration-aware trigger and a short cooldown so busy loops cannot fetch on
every poll.

## Acceptance Criteria

- [ ] Busy autodev loops no longer fetch after every non-empty dispatch cycle
- [ ] Integration artifact visibility still converges without manual intervention
- [ ] Tests cover both “needs sync” and “skip sync” paths

## Work Log

### 2026-03-25 - Review Discovery

**By:** Codex

**Actions:**
- Traced sync gating in the main controller loop
- Compared the comments and actual sync trigger conditions

**Learnings:**
- The current logic is more conservative than the comment suggests and likely costs steady-state throughput
