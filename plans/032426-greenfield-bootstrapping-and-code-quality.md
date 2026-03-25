# Greenfield Bootstrapping and Code Quality Enforcement

Fabro's autodev pipeline falls down on greenfield projects. When genesis creates
a project from planning docs, the first implementation lanes run in parallel
without a scaffolded project — no package.json, no schema, no linter config,
no type system. Agents write code into a void, producing files that can't run,
reference nonexistent schemas, use wrong types, and test nothing.

## Problem Statement

Observed on tonofcrap (Social Craps on TON, TypeScript/React/Convex):

1. **No project scaffold dependency** — `game-engine-core`, `convex-game-server`,
   and 12 other plans run simultaneously with `project-scaffold`. Agents write
   `.ts` files into a repo with no `package.json`, no `tsconfig.json`, no
   `node_modules/`. The verify gate (`npx convex dev --typecheck`) fails because
   `npx` has nothing to run.

2. **Agents write code without type context** — `queries.ts` uses `any[]` for
   player data, `number` for TON token amounts (should be BigInt per eng spec),
   references `big6`/`big8` bet types not in the spec. Without a schema file or
   type definitions to constrain them, agents hallucinate API shapes.

3. **Tests don't test the code** — `subscriptions.test.ts` duplicates query
   logic inline and tests the duplicate. It never imports from `queries.ts`.
   No Convex test utilities. Dead branches (`[] : []`). This passes quality
   because `scan_placeholder` only looks for TODO/stub markers, not test quality.

4. **Composite dependency resolution was broken** — composite plans referencing
   other composite plans caused validation errors. Fixed (resolved to last child
   lane), but the original dependency_plan_ids were cleared as a workaround,
   removing the ordering constraint that would have blocked feature lanes until
   scaffold completed.

## Root Causes

### RC1: No enforced scaffold-before-features ordering
The plan-mapping `dependency_plan_ids` mechanism exists but genesis/decomposition
doesn't reliably wire infrastructure plans as prerequisites. When `project-scaffold`
is composite, its children run alongside feature lanes rather than before them.

### RC2: No schema/type-system prerequisite enforcement
The verify gate checks proof commands but can't validate "does the code reference
real types." Without a schema.ts, type definitions, or linter, agents invent
interfaces that don't match the spec.

### RC3: Test quality is unmeasured
The quality gate checks for placeholder markers and test-to-derive ratios but
doesn't verify that tests actually import and exercise the code they claim to test.

## Proposed Fixes

### Phase 1: Scaffold-first ordering (planning.rs)

**Change**: When the plan registry contains a plan with `category: infrastructure`
or `plan_id` matching `project-scaffold`/`workspace-setup`/etc., automatically
inject it as a dependency for all non-infrastructure plans.

This ensures the scaffold lane completes before feature lanes dispatch.

**Location**: `derive_registry_plan_intents()` in `planning.rs`
**Scope**: ~30 lines in the dependency wiring logic

### Phase 2: Bootstrap verification gate (render.rs)

**Change**: For the first implementation lane in a project (the scaffold lane),
add a bootstrap verification step that checks for project health markers:
- Node.js: `package.json` exists, `npm install` succeeds, `tsconfig.json` present
- Rust: `Cargo.toml` valid, `cargo check` passes
- Python: `requirements.txt` or `pyproject.toml` exists

Only after this gate passes should downstream lanes dispatch.

**Location**: `render_workflow_graph()` in `render.rs`, keyed off `bootstrap_required`
**Scope**: ~40 lines for the gate script generation

### Phase 3: Type-aware code quality (render.rs quality gate)

**Change**: Extend the per-lane quality gate for TypeScript projects:
- Check that `.ts` files don't contain `any` in function signatures
- Check that test files import from the module they claim to test
- If a schema file is declared in plan context, verify it exists

**Location**: Quality gate generation in `implementation_quality_command()`
**Scope**: ~30 lines, similar pattern to the existing f64 detection

### Phase 4: Restore composite dependency_plan_ids for tonofcrap

**Change**: Re-render tonofcrap with the original `dependency_plan_ids` restored
(the composite resolution bug is now fixed). This re-establishes the ordering:
scaffold → game-engine → convex-server → features.

**Location**: `malinka/plan-mappings/*.yaml` in tonofcrap repo
**Scope**: Restore original values and re-render

## Execution Order

Phase 1 → Phase 4 → Phase 2 → Phase 3

Phase 1 makes the ordering automatic. Phase 4 applies it to tonofcrap immediately.
Phase 2 adds runtime verification. Phase 3 catches type-level issues.

## Success Criteria

- Tonofcrap scaffold lanes complete before feature lanes dispatch
- Feature lanes receive a project with package.json, tsconfig.json, schema.ts
- Quality gate catches `any` in TS function signatures
- Test files that don't import their subject code fail quality
