---
status: complete
priority: p1
issue_id: "001"
tags: [plan-first, raspberry-supervisor, fabro-synthesis, paperclip]
dependencies: []
---

# Execute Plan-First Registry And Status Slice

Implement the first execution slice of the expanded `032126` ExecPlan so the
repo has an honest shared plan registry and a richer plan-status surface to
build the rest of the plan-first control plane on.

## Problem Statement

The current plan has been expanded substantially, but the repository does not
yet have file-based task tracking for that expanded work and the first
implementation slice is still missing. The plan also assumed the new registry
would live in `fabro-synthesis`, but that creates a crate-cycle issue because
`fabro-synthesis` already depends on `raspberry-supervisor`.

## Findings

- `raspberry-supervisor` already implements maintenance mode in
  `lib/crates/raspberry-supervisor/src/maintenance.rs`, so the old progress
  checklist was stale.
- `raspberry-supervisor` already has a basic `plan_status.rs`, but it is still
  lane-centric and infers plan identity from filenames and current blueprint
  lanes rather than from a first-class registry.
- `fabro-cli` Paperclip sync remains frontier-centric via `FrontierSyncModel`
  and `FrontierSyncEntry`.
- `fabro-synthesis` depends on `raspberry-supervisor`, so a shared registry must
  start in `raspberry-supervisor` or a dedicated shared crate.

## Proposed Solutions

### Option 1: Shared Registry In `raspberry-supervisor`

**Approach:** Add `plan_registry.rs` to `raspberry-supervisor`, re-export it,
update `plan_status.rs` to consume it, and let synthesis consume it later.

**Pros:**
- Avoids a crate cycle immediately
- Gives plan status and future scheduler one deterministic source of truth
- Keeps the first execution batch small enough to verify cleanly

**Cons:**
- Does not yet wire synthesis or Paperclip to the registry
- May later be extracted to a dedicated shared crate

**Effort:** 2-4 hours

**Risk:** Low

---

### Option 2: New Shared Crate First

**Approach:** Introduce a brand-new shared crate for plan registry and mapping
logic before touching supervisor status code.

**Pros:**
- Architecturally cleaner end state
- Makes the shared boundary explicit from day one

**Cons:**
- Larger first slice
- More churn before any user-visible plan-first status improvement lands

**Effort:** 4-8 hours

**Risk:** Medium

## Recommended Action

Execute Option 1 now. Update the ExecPlan to reflect the crate-cycle discovery,
add a shared plan registry in `raspberry-supervisor`, upgrade `plan_status.rs`
to consume it, and run focused verification. Leave synthesis and Paperclip
integration as the next slices.

## Technical Details

**Affected files:**
- `plans/032126-plan-first-autodev-redesign.md`
- `todos/001-ready-p1-plan-first-registry-and-status.md`
- `lib/crates/raspberry-supervisor/src/lib.rs`
- `lib/crates/raspberry-supervisor/src/plan_registry.rs`
- `lib/crates/raspberry-supervisor/src/plan_status.rs`
- `lib/crates/raspberry-cli/tests/cli.rs`

**Related components:**
- `lib/crates/fabro-synthesis/src/planning.rs`
- `lib/crates/fabro-cli/src/commands/paperclip.rs`

**Database changes (if any):**
- No

## Resources

- `plans/032126-plan-first-autodev-redesign.md`
- `lib/crates/raspberry-supervisor/src/maintenance.rs`
- `lib/crates/raspberry-supervisor/src/plan_status.rs`
- `lib/crates/fabro-synthesis/src/planning.rs`

## Acceptance Criteria

- [x] ExecPlan progress and concrete steps reflect the shared-registry first slice
- [x] File-based todo tracking exists for this expanded execution work
- [x] `raspberry-supervisor` exposes a deterministic plan registry with tests
- [x] `plan_status.rs` consumes the registry and reports richer plan-first rows
- [x] Focused tests pass for the touched crates or any unrelated failure is documented

## Work Log

### 2026-03-21 - Kickoff And First Slice Setup

**By:** Codex

**Actions:**
- Re-read the ExecPlan and compared it against current repository reality
- Confirmed maintenance mode already exists and identified the stale checklist
- Identified the crate-cycle constraint blocking a synthesis-only registry
- Created this todo to track the first implementation slice

**Learnings:**
- The honest first execution batch is shared registry plus richer plan status,
  not maintenance rework
- The plan document needed to reflect real code constraints before safe
  implementation could begin

### 2026-03-21 - Shared Registry And Status Slice Landed

**By:** Codex

**Actions:**
- Updated `plans/032126-plan-first-autodev-redesign.md` to reflect the
  shared-registry-first implementation path and the crate-cycle discovery
- Added `lib/crates/raspberry-supervisor/src/plan_registry.rs`
- Re-exported the registry types from
  `lib/crates/raspberry-supervisor/src/lib.rs`
- Reworked `lib/crates/raspberry-supervisor/src/plan_status.rs` to consume the
  registry and report mapping status, child counts, and next operator moves
- Updated `lib/crates/raspberry-cli/tests/cli.rs` for the richer matrix output
- Ran:
  `cargo test -p raspberry-supervisor plan_registry -- --nocapture`
  `cargo test -p raspberry-supervisor plan_status -- --nocapture`
  `cargo test -p raspberry-cli plan_matrix -- --nocapture`
  `cargo check -p raspberry-supervisor -p raspberry-cli`

**Learnings:**
- The shared-registry approach works cleanly with the current crate graph
- The first useful user-visible plan-first win is a richer local matrix, even
  before synthesis and Paperclip are rewired

## Notes

- If the shared registry proves broadly useful quickly, consider extracting it
  into a dedicated crate in a follow-up slice.
