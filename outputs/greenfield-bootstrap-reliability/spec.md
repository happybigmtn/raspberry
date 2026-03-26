# Greenfield Bootstrap and Runtime Asset Reliability — Spec

## Purpose

This spec defines the durable behavior required for `fabro synth create` to produce packages where:

1. Scaffold/infrastructure lanes complete **before** any feature lanes dispatch
2. Generated runtime assets (workflows, prompts) resolve correctly when copied into `~/.fabro/runs/`
3. Agents never write language-specific code into repos missing their language infrastructure
4. Quality gates are type-aware and catch language-specific issues

## Scope

This spec applies to the greenfield bootstrap flow: `fabro synth create` → `raspberry autodev` on repos with no pre-existing `malinka/` package.

## Genesis Context

**Genesis plan**: `genesis/plans/004-greenfield-bootstrap-reliability.md`

This plan replaces `plans/032426-greenfield-bootstrapping-and-code-quality.md` with a structured ExecPlan. It incorporates scaffold-first ordering (commit `6d0853f4`) and the bootstrap guard for fresh projects (commit `cb0c016e`). The root cause was identified on the **tonofcrap** project (TypeScript/React/Convex on TON): `project-scaffold` ran in parallel with feature lanes instead of before them, causing agents to write `.ts` files into a repo with no `package.json` or `tsconfig.json`.

## Current State

### What Already Exists

**Scaffold-first ordering** (`lib/crates/fabro-synthesis/src/planning.rs`, lines 752–843):
- `derive_registry_plan_intents()` identifies scaffold plans via `PlanCategory::Infrastructure` or plan ID matching `project-scaffold`/`workspace-setup`
- Only applies to plans from YAML plan-mappings (`PlanMappingSource::Contract`) to avoid false positives on legacy master-plan-based projects
- Injects scaffold plans as implicit dependencies for all non-infrastructure parent intents
- Resolves composite scaffold plans to their last child
- **Gap**: No `#[test]` validates this behavior; proof command `cargo nextest run -p fabro-synthesis -- scaffold_first` does not exist

**Bootstrap guard** (`fabro-workflows/src/`, commit `cb0c016e`):
- Partial guard for fresh projects exists
- **Gap**: No language-specific project health markers exist in the scaffold workflow graph

**Quality gate** (`lib/crates/fabro-synthesis/src/render.rs`, `implementation_quality_command()`, lines 2232–2440):
- Checks for placeholders, warnings, semantic risk, lane sizing, test debt, manual-followup markers
- Covers Rust (`*.rs`), Python (`*.py`), JavaScript (`*.js`), TypeScript (`*.ts`/`*.tsx`)
- **Gap**: No TypeScript-specific checks (no `any` detection in exported signatures, no missing import validation)

**Prompt/workflow asset references** (`lib/crates/fabro-synthesis/src/render.rs`, line 1913):
```rust
let prompt_path = |name: &str| -> String {
    format!(
        "@../../prompts/{}/{}/{}.md",
        lane.workflow_family(),
        lane.slug(),
        name
    )
};
```
- **Gap**: `../../prompts/` is relative to the workflow file location. When Fabro copies `graph.fabro` into `~/.fabro/runs/<run-id>/`, this path resolves relative to the run directory, not the project root. Every `fabro run` from a detached run directory fails prompt loading.

### What Is Missing

| Milestone | Status | Gap |
|-----------|--------|-----|
| 1: Scaffold-first ordering test | ❌ Not done | No `#[test] scaffold_first` |
| 2: Bootstrap verification gate | ❌ Not done | No language-specific health checks in scaffold workflow |
| 3: Runtime-stable asset refs | ❌ Not done | `@../../prompts/...` breaks in detached runs |
| 4: TypeScript quality checks | ❌ Not done | No `any` detection, no import validation |
| 5: tonofcrap 30-cycle validation | ⏸ Deferred | Requires Milestones 1–4 |
| 6: Fresh Rust project validation | ⏸ Deferred | Requires Milestones 1–4 |

## Architecture

### Target Greenfield Flow

```
scaffold ──> [bootstrap verify] ──> feature-1 ──┐
                                 ──> feature-2 ──┘
```

### Bootstrap Verification Gate

The scaffold lane workflow must include a verification node that checks language-specific project health markers **before** feature lanes dispatch.

#### Language-Specific Health Markers

| Language | Required Markers | Verification |
|----------|-----------------|--------------|
| Node.js/TypeScript | `package.json` exists, `node_modules/` populated, `tsconfig.json` present | `test -f package.json && test -d node_modules && test -f tsconfig.json` |
| Rust | `Cargo.toml` valid | `cargo check --quiet` |
| Python | `pyproject.toml` or `requirements.txt` exists | `test -f pyproject.toml -o -f requirements.txt` |

#### Implementation Location

- **File**: `lib/crates/fabro-synthesis/src/render.rs`
- **Function**: `render_workflow_graph()`
- **Target**: `WorkflowTemplate::Bootstrap` and `WorkflowTemplate::ServiceBootstrap` branches
- **Change**: Insert a `bootstrap_health` node between `implement` and `verify` stages

### Runtime-Stable Asset Resolution

#### Problem

Generated workflows contain prompt references like:
```
@../../prompts/bootstrap/rules/plan.md
```

When Fabro copies `graph.fabro` into `~/.fabro/runs/<run-id>/`, this path resolves relative to the run directory, not the project root.

#### Solution

Use `${FABRO_PROJECT_ROOT}` environment variable substitution:
```rust
let prompt_path = |name: &str| -> String {
    format!(
        "@${{FABRO_PROJECT_ROOT}}/malinka/prompts/{}/{}/{}.md",
        lane.workflow_family(),
        lane.slug(),
        name
    )
};
```

Requires `fabro-workflows` to set `FABRO_PROJECT_ROOT` in the run context environment (check `lib/crates/fabro-workflows/src/handler/agent.rs` and `lib/crates/fabro-cli/src/commands/run.rs`).

#### Implementation Location

- **File**: `lib/crates/fabro-synthesis/src/render.rs`, line 1913
- **Function**: `render_workflow_graph()` → `prompt_path` closure

### TypeScript Quality Checks

#### Required Checks

1. **`any` usage in exported signatures**:
   ```bash
   rg -n 'export (function|const|class|interface|type) \w+.*: any' -g '*.ts' -g '*.tsx'
   ```

2. **Missing imports in test files**:
   - Heuristic: if test file is `foo.test.ts`, it should import from `foo.ts`
   - Check for mismatched module paths

3. **Schema file existence** (if declared in plan context):
   - Extract schema path from `prompt_context`
   - `test -f "$schema_path"`

#### Implementation Location

- **File**: `lib/crates/fabro-synthesis/src/render.rs`
- **Function**: `implementation_quality_command()`, lines 2232–2440
- **Add**: TypeScript-specific quality script block following the existing Rust/Python pattern

## Validation Criteria

The implementation is complete when:

1. ✅ `cargo nextest run -p fabro-synthesis -- scaffold_first` passes
2. ✅ `cargo nextest run -p fabro-synthesis -- bootstrap_verify` passes
3. ✅ `cargo nextest run -p fabro-synthesis -- quality_typescript` passes
4. ✅ `fabro validate` works on detached run directories with `${FABRO_PROJECT_ROOT}` resolution
5. ✅ tonofcrap 30-cycle autodev completes without infrastructure-caused failures
6. ✅ Fresh Rust project scaffold-first validation passes

## Non-Goals

- This spec does not address multi-repo workflows
- This spec does not address Windows path resolution
- This spec does not address remote/Docker sandbox environments (separate concern)

## Implementation Order

1. **First**: Add scaffold-first test (validates existing code works)
2. **Second**: Runtime-stable asset resolution (small change, high impact)
3. **Third**: Bootstrap verification gate (requires language detection design)
4. **Fourth**: TypeScript quality checks (follows Python pattern in codebase)
5. **Fifth**: Live validations (end-to-end proof)

## Related Documents

- `genesis/plans/004-greenfield-bootstrap-reliability.md` — ExecPlan with Milestones 1–6
- `genesis/plans/001-master-plan.md` — Phase 0 dependency ordering
- `plans/032426-greenfield-bootstrapping-and-code-quality.md` — Historical root cause analysis
