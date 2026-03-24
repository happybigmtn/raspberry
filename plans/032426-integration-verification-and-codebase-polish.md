# Integration Verification and Codebase Polish

Autodev produces individually correct modules (74/221 lanes complete, 579 tests,
0 stubs) but has no mechanism to verify they work together. The synthesis pipeline
decomposes into isolated slices without generating cross-module verification.

## Problem Statement

Three classes of bug escape the current harness:

1. **Cross-module integration failures** — each lane's `cargo test -p {crate}` passes,
   but `cargo test --workspace` may fail due to API mismatches, stale imports, or
   type incompatibilities between crates.

2. **Protocol contract violations** — a `GameVariant` implementor (e.g., blackjack)
   and its consumer (house-agent) compile independently but the wire protocol
   doesn't match. No lane tests the full flow.

3. **Type/convention drift** — foundational types like `Chips = i16` were inherited
   and never reviewed. Each game reimplements patterns (phase enums, settlement
   logic) without shared conventions. No lane enforces consistency.

## Approach

Three system-level changes to fabro's synthesis pipeline, plus a codebase consistency
gate. All changes are in the fabro repo (raspberry-supervisor, fabro-synthesis) so
they apply automatically to any project via `synth create` / `synth evolve`.

---

## Phase 1: Workspace Health Lanes (auto-generated)

**Goal:** After all child implementation lanes of a unit complete, automatically
verify the full workspace compiles and tests pass.

### Changes

**`render.rs` — `augment_with_implementation_follow_on_units()`**

Add logic: when a unit has child implementation lanes, generate a workspace-verify
lane with:
- `id: "{unit}-workspace-verify"`
- `template: Implementation`
- `proof_profile: "integration"`
- `goal`: "Verify full workspace compiles and all tests pass. Run `cargo check
  --workspace && cargo test --workspace`. Report any cross-crate compilation
  errors or test failures. Do NOT modify source code — only report findings
  in spec.md and review.md."
- `verify_command`: `"cargo check --workspace && cargo test --workspace"`
- `dependencies`: all child lane IDs of the unit
- `owned_surfaces`: empty (read-only lane)

**`render.rs` — `profile_max_visits()` / `profile_extra_graph_elements()`**

Add `"integration"` proof profile:
- `max_visits`: 6 (cross-crate issues need more fixup cycles)
- Verify command: workspace-wide, not per-crate
- Challenge prompt: specifically check cross-module imports and shared trait
  implementations

**`blueprint.rs` — `BlueprintLane`**

No schema changes needed — the workspace-verify lane uses existing fields.
The `verify_command` override replaces the default per-crate test.

### Validation

- `synth create` on a blueprint with child lanes generates the workspace-verify lane
- The lane sits at `blocked` until children complete
- When dispatched, it runs `cargo test --workspace` and reports results
- If tests fail, the remediation loop captures the failures

---

## Phase 2: Protocol Contract Lanes (auto-generated from trait boundaries)

**Goal:** When the blueprint declares crates that implement/consume a shared
interface (trait), automatically generate a contract verification lane.

### Changes

**`blueprint.rs` — new `BlueprintProtocol` struct**

```rust
pub struct BlueprintProtocol {
    pub id: String,
    pub trait_name: String,
    pub implementor_units: Vec<String>,  // unit IDs that implement the trait
    pub consumer_units: Vec<String>,     // unit IDs that consume the trait
    pub verification_command: String,    // e.g., "cargo test -p house -- integration"
}
```

Add `protocols: Vec<BlueprintProtocol>` field to `ProgramBlueprint`.

**`render.rs` — protocol verification lane generation**

For each protocol declared in the blueprint:
- Generate a lane: `"{protocol.id}-contract-verify"`
- Dependencies: all implementor + consumer lanes must be complete
- Goal: "Verify that all `{trait_name}` implementors satisfy the contract
  expected by consumers. Run {verification_command}. Check that types,
  method signatures, and wire formats match."
- Template: Implementation
- Proof profile: integration

**Source blueprint changes (per-project)**

Projects declare protocols in their blueprint YAML:
```yaml
protocols:
  - id: game-variant-contract
    trait_name: GameVariant
    implementor_units: [blackjack, baccarat, caribbean-stud, crash, dice, ...]
    consumer_units: [house-agent]
    verification_command: "cargo test -p house -- integration"
```

### Validation

- Blueprint with `protocols` section generates contract-verify lanes
- Lanes depend on both implementor and consumer units
- Verification command exercises the actual interface boundary

---

## Phase 3: Codebase Consistency Gate

**Goal:** Detect type drift, convention violations, and foundational issues
that no individual lane is scoped to catch.

### Changes

**`render.rs` — consistency checks in quality gate**

Extend `implementation_quality_command()` to include workspace-level checks
when `proof_profile = "integration"`:

1. **Type consistency**: Scan for `as i16`, `as f64 * 0.95`, and other
   narrowing casts in game logic. Flag Chips arithmetic that could overflow.

2. **Convention audit**: Check that all `GameVariant` implementors follow
   the same pattern:
   - Phase enum named `{Game}Phase`
   - Settlement via `Settlement::new()` (not ad-hoc arithmetic)
   - Error types via `GameError` (not custom error types)

3. **Workspace compilation**: `cargo check --workspace` as a hard gate

**`render.rs` — consistency-specific challenge prompt**

For integration-profile lanes, the challenge prompt should ask:
- "Are there `as i16` or `as i32` casts in settlement/payout code that
  could overflow for large bets?"
- "Do all GameVariant implementations use the same state machine pattern?"
- "Are there cross-crate import inconsistencies?"

### Implementation in stage prompts (`cli.rs`)

Add a new stage prompt for integration-profile implement stages:
```
"You are running a WORKSPACE INTEGRATION CHECK. Do NOT write new code.
Read the codebase and verify: (1) cargo check --workspace passes,
(2) cargo test --workspace passes, (3) no narrowing casts (as i16, as i32)
in settlement/payout arithmetic that could overflow, (4) all GameVariant
implementations follow consistent patterns. Report all findings in spec.md."
```

### Validation

- Integration-profile lanes run the extended quality gate
- Overflow patterns are flagged in quality.md
- Convention drift is reported in verification.md

---

## Execution Order

| Step | Files | Effort | Depends On |
|------|-------|--------|------------|
| 1a. Add `integration` proof profile | render.rs | Small | None |
| 1b. Generate workspace-verify lanes | render.rs | Medium | 1a |
| 1c. Integration stage prompts | cli.rs | Small | 1a |
| 2a. Add `BlueprintProtocol` struct | blueprint.rs | Small | None |
| 2b. Protocol lane generation | render.rs | Medium | 2a, 1a |
| 3a. Extended quality gate | render.rs | Medium | 1a |
| 3b. Consistency challenge prompt | render.rs, cli.rs | Small | 3a |

Steps 1a-1c and 2a can run in parallel. 2b depends on both 2a and 1a.
3a-3b can start after 1a.

---

## Success Criteria

- [ ] `synth create` on a blueprint with >3 child lanes generates a workspace-verify lane
- [ ] The workspace-verify lane runs `cargo test --workspace` in its verify gate
- [ ] `synth create` on a blueprint with `protocols` section generates contract-verify lanes
- [ ] Integration-profile challenge prompts check for overflow casts and convention drift
- [ ] Extended quality gate scans for `as i16` / narrowing patterns
- [ ] All changes are backward-compatible (blueprints without protocols/children work as before)
