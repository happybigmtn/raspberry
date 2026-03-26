---
status: pending
priority: p1
issue_id: "004"
tags: [raspberry, autodev, evaluation, performance]
dependencies: []
---

# Stop double evaluation when loading autodev report state

Avoid the extra `evaluate_program()` pass that currently happens while reading autodev report data
for status surfaces.

## Problem Statement

`evaluate_program_internal()` loads the optional autodev report to expose runtime `max_parallel`.
But `load_optional_autodev_report()` computes a fresh `current_snapshot` by calling
`evaluate_program()` again. That means a single status evaluation can perform two program
evaluations, doubling manifest/state work on the hot path.

## Findings

- `evaluate_program_internal()` invokes `load_optional_autodev_report()` during every evaluation:
  [evaluate.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/evaluate.rs#L224)
- `load_optional_autodev_report()` immediately calls `evaluate_program(manifest_path)` again to
  rebuild `report.current`:
  [autodev.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/autodev.rs#L1292)
- `sync_autodev_report_with_program()` is currently a no-op, so the code pays the extra read-time
  evaluation cost instead of updating snapshots when the controller already has a fresh program in
  hand:
  [autodev.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/autodev.rs#L1315)

## Proposed Solutions

### Option 1: Make report sync write the current snapshot

**Approach:** Update `sync_autodev_report_with_program()` so the controller writes the current
snapshot whenever it already has an evaluated program, and make report loading purely deserializing.

**Pros:**
- Removes duplicate evaluation from the status path
- Keeps the report as the single source of controller-facing runtime state

**Cons:**
- Needs care to avoid reviving the old “status read mutates heartbeat” bug

**Effort:** 2-3 hours

**Risk:** Medium

---

### Option 2: Split runtime-parallel metadata from the autodev report

**Approach:** Store runtime-only display metadata separately so evaluation does not need to reopen
and rewrite the autodev report contract.

**Pros:**
- Cleaner separation between controller state and derived status
- Easier to reason about stale-vs-live semantics

**Cons:**
- More moving pieces and file formats

**Effort:** 4-6 hours

**Risk:** Medium

## Recommended Action

Implement Option 1 with explicit tests: controller writes the snapshot, plain `raspberry status`
reads do not mutate timestamps, and `evaluate_program()` performs only one evaluation pass.

## Technical Details

**Affected files:**
- [evaluate.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/evaluate.rs)
- [autodev.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/autodev.rs)

## Acceptance Criteria

- [ ] Status evaluation no longer calls back into `evaluate_program()` through report loading
- [ ] Autodev reports still surface `max_parallel` and current counts correctly
- [ ] Tests cover non-mutating status reads and snapshot freshness

## Work Log

### 2026-03-25 - Review Discovery

**By:** Codex

**Actions:**
- Traced report loading from evaluation
- Verified the current snapshot path re-enters program evaluation
- Confirmed the report sync function is intentionally stubbed today

**Learnings:**
- The recursion guard prevents an infinite loop, but not the redundant second evaluation
- This is a hot-path cost multiplier rather than a crash bug

