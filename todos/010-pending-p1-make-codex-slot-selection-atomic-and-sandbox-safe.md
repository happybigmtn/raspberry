---
status: pending
priority: p1
issue_id: "010"
tags: [workflows, codex, rotator, autodev]
dependencies: []
---

# Make Codex slot selection atomic and sandbox-safe

Harden the Codex key-rotator path so parallel lanes do not collide on the same slot and remote
sandboxes do not receive unusable host-only `CODEX_HOME` paths.

## Problem Statement

The current rotator reads and writes slot state on the controller host without any inter-process
reservation, then injects the selected `CODEX_HOME` into the sandbox. Under 5-way parallel autodev
this can assign the same slot to multiple lanes at once, and in non-local sandboxes the injected
path may not exist at all.

## Findings

- Slot selection reads shared state optimistically and marks selection in a second non-atomic step:
  [cli.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs#L838)
  [cli.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs#L887)
  [cli.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs#L1017)
- The chosen host-side `codex_home` is exported directly into OpenAI runs:
  [cli.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs#L1240)

## Proposed Solutions

### Option 1: Add a locked reservation step with sandbox-aware validation

**Approach:** Guard slot selection and reservation with a file lock, and only inject `CODEX_HOME`
when the sandbox can actually access that path.

**Pros:**
- Prevents hot-slot collisions under parallel load
- Avoids deterministic failures in remote sandboxes

**Cons:**
- Needs small state-machine changes in the rotator

**Effort:** 3-5 hours

**Risk:** Medium

---

### Option 2: Keep shared state optimistic but validate after launch

**Approach:** Continue optimistic slot selection but detect collisions or missing paths after the
process starts and rotate away.

**Pros:**
- Smaller implementation delta

**Cons:**
- Failures still happen on the hot path
- Worse operator experience under contention

**Effort:** 2-4 hours

**Risk:** Medium

## Recommended Action

Implement Option 1. The reservation should be atomic, and the rotator should refuse to inject a
host-only auth home into non-local sandboxes unless it has been provisioned there explicitly.

## Technical Details

**Affected files:**
- [cli.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs)

## Acceptance Criteria

- [ ] Parallel Codex launches do not reserve the same slot simultaneously
- [ ] Remote or isolated sandboxes do not receive unusable host-only `CODEX_HOME` paths
- [ ] Tests cover slot contention and sandbox-inaccessible auth homes

## Work Log

### 2026-03-25 - Review Discovery

**By:** Codex

**Actions:**
- Reviewed Codex rotator loading, selection, and state persistence
- Compared the controller-host path assumptions with sandbox launch behavior

**Learnings:**
- The current rotator is close to workable but still lacks reservation semantics
- This is both a reliability issue and a parallelism limiter
