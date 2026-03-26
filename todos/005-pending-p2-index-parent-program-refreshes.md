---
status: pending
priority: p2
issue_id: "005"
tags: [raspberry, autodev, evaluation, parent-programs]
dependencies: []
---

# Index parent program refreshes

Replace the current full scan of every program manifest on each child evaluation with a targeted
parent refresh strategy.

## Problem Statement

Whenever a program is evaluated with parent propagation enabled, Raspberry scans every
`malinka/programs/*.yaml`, loads each manifest, checks whether it references the current child, and
then re-evaluates matching parents. On larger repos this turns each child progress update into an
O(number of programs) filesystem walk plus repeated manifest parsing.

## Findings

- `refresh_parent_programs()` walks every program manifest in `malinka/programs` on each call:
  [evaluate.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/evaluate.rs#L281)
- It reparses each candidate manifest just to answer “does this parent reference my child?”:
  [evaluate.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/evaluate.rs#L299)
- Matching parents are synchronously re-evaluated in the same call chain:
  [evaluate.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/evaluate.rs#L309)

## Proposed Solutions

### Option 1: Build a reverse child→parent index once per process

**Approach:** Cache program-manifest relationships and invalidate on manifest mtime change.

**Pros:**
- Removes repeated directory scans and manifest reparsing
- Keeps parent propagation behavior intact

**Cons:**
- Needs cache invalidation logic

**Effort:** 3-5 hours

**Risk:** Medium

---

### Option 2: Persist explicit parent references in runtime state

**Approach:** Record parent programs alongside child-program lanes and refresh only those known
parents.

**Pros:**
- Very cheap steady-state lookup
- Keeps the runtime hot path local to the active program graph

**Cons:**
- More data to maintain during synth/regeneration

**Effort:** 4-6 hours

**Risk:** Medium

## Recommended Action

Start with Option 1. A small reverse index keyed by canonical child manifest path should remove the
majority of the repeated work without changing on-disk formats.

## Acceptance Criteria

- [ ] Parent refresh no longer scans all program manifests on every child evaluation
- [ ] Parent propagation still updates dependent programs correctly
- [ ] Tests cover index invalidation when a parent manifest changes

## Work Log

### 2026-03-25 - Review Discovery

**By:** Codex

**Actions:**
- Traced the parent propagation branch from `evaluate_program_internal()`
- Measured the shape of the scan/reparse/re-evaluate loop in code

**Learnings:**
- The current design is safe but scales poorly as the number of programs grows

