---
status: pending
priority: p2
issue_id: "012"
tags: [workflows, cli, autodev, reliability]
dependencies: []
---

# Isolate CLI stage status artifacts and restore sandbox autostop

Tighten CLI stage execution so stale repo-root status files cannot influence routing, and restore
sandbox autostop after long-running CLI stages finish.

## Problem Statement

The CLI agent path can fall back to a repo-root `status.json` when the backend response is missing
structured routing data. In long-lived worktrees that leaves stage transitions vulnerable to stale
artifacts from previous runs. The same path also disables sandbox autostop for long CLI work and
does not restore the previous interval afterward.

## Findings

- Agent execution falls back to `cat status.json` from the sandbox CWD when routing JSON is absent:
  [agent.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/handler/agent.rs#L482)
- CLI stages disable sandbox autostop before launch:
  [cli.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs#L1215)

## Proposed Solutions

### Option 1: Use stage-owned status artifacts and restore prior autostop

**Approach:** Require a stage-scoped status artifact path from the backend, refuse repo-root
fallbacks, and capture/restore the previous autostop interval around CLI execution.

**Pros:**
- Removes a source of cross-run nondeterminism
- Keeps long-running sandboxes from lingering longer than intended

**Cons:**
- Requires a small contract cleanup between backend and handler

**Effort:** 2-4 hours

**Risk:** Medium

---

### Option 2: Keep fallback behavior but namespace the status file under stage scratch

**Approach:** Continue fallback reads, but only from a stage-specific scratch path, and restore
autostop on best-effort cleanup.

**Pros:**
- Smaller surface-area change

**Cons:**
- Still relies on fallback semantics

**Effort:** 1-3 hours

**Risk:** Medium

## Recommended Action

Implement Option 1. Routing should come from stage-owned artifacts only, and CLI execution should
leave the sandbox autostop policy the way it found it.

## Technical Details

**Affected files:**
- [agent.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/handler/agent.rs)
- [cli.rs](/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs)

## Acceptance Criteria

- [ ] Stale repo-root `status.json` files cannot affect stage routing
- [ ] CLI stages restore the prior sandbox autostop interval after finishing
- [ ] Tests cover stale status artifact isolation and autostop restoration

## Work Log

### 2026-03-25 - Review Discovery

**By:** Codex

**Actions:**
- Reviewed CLI stage launch and handler fallback behavior
- Traced how routing can fall back to a generic `status.json`

**Learnings:**
- This is a good example of long-lived worktree state leaking across retries
- The fix is mostly about tightening contracts, not reworking the whole execution path
