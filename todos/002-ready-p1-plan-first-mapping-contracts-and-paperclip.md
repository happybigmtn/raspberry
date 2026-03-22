---
status: ready
priority: p1
issue_id: "002"
tags: [plan-first, paperclip, mapping-contracts, fabro-cli, fabro-synthesis]
dependencies: ["001"]
---

# Execute Mapping Contracts And Paperclip Plan-First Slice

Implement the next plan-first execution batch after the shared registry and
status/synthesis wiring: parse real mapping contracts, improve synthesized
execution shapes, and reorganize Paperclip sync around plan roots instead of
frontier lanes.

## Problem Statement

The redesign now has a shared plan registry, richer local plan status, and
registry-backed synthesis previews. The next major trust gaps are:

- the registry only detects mapping-contract files; it does not parse them yet
- many plans still fall back to conservative synthesized mappings that should be
  improved by checked-in contracts
- Paperclip remains frontier-centric even though local status and synthesis are
  now plan-first

Without this slice, operators can see the plan-first truth locally but the web
dashboard and synchronized issue hierarchy still lag behind.

## Findings

- `fabro-synthesis` now emits deterministic units for numbered plans in the
  live `rXMRbro` preview.
- `raspberry plan-matrix` now reports mapping status and next operator move for
  each numbered plan.
- `fabro-cli/src/commands/paperclip.rs` still revolves around
  `FrontierSyncModel` and `FrontierSyncEntry`.
- `plan_registry.rs` currently treats mapping contracts as file presence only.

## Proposed Solutions

### Option 1: Parse Mapping Contracts First, Then Rebuild Paperclip On Top

**Approach:** Extend the shared registry to read structured mapping-contract
data, use that to reduce false ambiguity, then switch Paperclip sync to plan
roots and plan children.

**Pros:**
- Better plan truth before dashboard changes
- Reduces churn in Paperclip sync semantics
- Aligns dashboard work to deterministic registry data

**Cons:**
- Slightly delays visible Paperclip changes

**Effort:** 4-8 hours

**Risk:** Medium

---

### Option 2: Rebuild Paperclip First With Current Registry

**Approach:** Make the dashboard plan-first immediately, even before real
mapping-contract parsing lands.

**Pros:**
- Faster visible UI/control-plane change

**Cons:**
- Risks encoding conservative or misleading ambiguity into the dashboard
- More rework once mapping-contract parsing lands

**Effort:** 3-6 hours

**Risk:** Medium

## Recommended Action

Execute Option 1. Use the shared registry as the contract boundary, teach it to
parse checked-in mapping data, then move Paperclip sync and company markdown to
plan-root-first status using that richer registry.

## Technical Details

**Likely files:**
- `lib/crates/raspberry-supervisor/src/plan_registry.rs`
- `lib/crates/fabro-synthesis/src/planning.rs`
- `lib/crates/fabro-cli/src/commands/paperclip.rs`
- `plans/032126-plan-first-autodev-redesign.md`

## Acceptance Criteria

- [ ] Mapping contracts are parsed, not just detected by filename
- [ ] Synthesized execution shapes improve where checked-in contracts exist
- [ ] Paperclip sync exposes one top-level object per plan root
- [ ] Company markdown becomes plan-first instead of lane-first
- [ ] Focused verification passes for the touched crates

## Work Log

### 2026-03-21 - Follow-On Slice Created

**By:** Codex

**Actions:**
- Created the next ready todo after completing the shared registry/status
  execution slice

**Learnings:**
- The next bottleneck is not local status anymore; it is mapping-contract depth
  and Paperclip parity

### 2026-03-21 - Parser And Plan-First Paperclip Pass

**By:** Codex

**Actions:**
- Extended `lib/crates/raspberry-supervisor/src/plan_registry.rs` to parse
  checked-in YAML/JSON mapping contracts and apply overrides for title,
  category, dependencies, child ids, and execution-shape metadata
- Updated `lib/crates/fabro-cli/src/commands/paperclip.rs` so company markdown
  and root plan documents lead with plan status summary, plans needing
  attention, and the plan matrix when available
- Added the canonical proving-ground contract
  `rXMRbro/fabro/plan-mappings/005-craps-game.yaml`
- Verified with:
  `cargo test -p raspberry-supervisor plan_registry -- --nocapture`
  `cargo test -p fabro-cli paperclip -- --nocapture`
  `cargo check -p raspberry-supervisor -p fabro-cli`
  plus live `raspberry plan-matrix` and `fabro synth evolve` reruns against
  `rXMRbro`

**Learnings:**
- The shared registry is now a real contract boundary, not just a file detector
- One checked-in contract is enough to prove the end-to-end path from contract
  parsing to live operator output
- Paperclip is still structurally frontier-centric beneath the presentation
  layer, so the next step is deeper issue/work-product rekeying by plan root

### 2026-03-21 - Always-Mapped Simplification And Scratch Regeneration

**By:** Codex

**Actions:**
- Removed `needs_mapping_review` as a first-class control-plane state across
  registry, status, synthesis prompt context, Paperclip presentation, and the
  ExecPlan
- Verified the simplified model with:
  `cargo test -p raspberry-supervisor plan_registry -- --nocapture`
  `cargo test -p raspberry-supervisor plan_status -- --nocapture`
  `cargo test -p fabro-synthesis create_authoring -- --nocapture`
  `cargo test -p fabro-cli paperclip -- --nocapture`
  `cargo check -p raspberry-supervisor -p fabro-synthesis -p fabro-cli`
- Rebuilt `/home/r/coding/rXMRbro/fabro` from scratch with
  `fabro synth create --target-repo /home/r/coding/rXMRbro --program rxmragent`
- Verified the regenerated repo with live
  `raspberry plan-matrix --manifest /home/r/coding/rXMRbro/fabro/programs/rxmragent.yaml`

**Learnings:**
- The simplified user model works better: every numbered plan is always mapped,
  and the operator only reasons about current status/risk/next move
- The wiped `rXMRbro` control plane now regenerates cleanly under the new
  plan-first assumptions

### 2026-03-21 - Synth Create Now Writes Mapping Snapshots

**By:** Codex

**Actions:**
- Extended `lib/crates/fabro-cli/src/commands/synth.rs` so `synth create`
  writes `fabro/plan-mappings/*.yaml` automatically as part of create
- Added focused coverage in `lib/crates/fabro-cli/tests/synth.rs`
- Verified with:
  `cargo test -p fabro-cli --test synth synth_create_writes_plan_mapping_snapshots -- --nocapture`
  and a live rerun of
  `fabro synth create --target-repo /home/r/coding/rXMRbro --program rxmragent`

**Learnings:**
- The generated package now includes both the executable Fabro package and the
  mapping snapshot it was derived from, which makes the create step
  self-describing

## Notes

- This todo intentionally depends on issue `001` because it builds directly on
  the shared registry introduced there.
