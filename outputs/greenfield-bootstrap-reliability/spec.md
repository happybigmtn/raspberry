# Greenfield Bootstrap and Runtime Asset Reliability — Spec

## Purpose

This spec defines the durable behavior required for `fabro synth create` to produce packages where:

1. Scaffold/infrastructure lanes complete **before** any feature lanes dispatch
2. Generated runtime assets (workflows, prompts) resolve correctly when copied into `~/.fabro/runs/`
3. Agents never write language-specific code into repos missing their language infrastructure
4. Quality gates are type-aware and catch language-specific issues

## Scope

This spec applies to the greenfield bootstrap flow: `fabro synth create` → `raspberry autodev` on repos with no pre-existing `malinka/` package.

## Current State

### What Already Exists

**Scaffold-first ordering** (`lib/crates/fabro-synthesis/src/planning.rs`):
- Lines 752–843 implement scaffold dependency injection
- Identifies scaffold plans via `PlanCategory::Infrastructure` or plan ID matching `project-scaffold`/`workspace-setup`
- Only applies to plans from YAML plan-mappings (Opus-decomposed) to avoid false positives
- **Gap**: No unit test exists to verify this behavior

**Bootstrap guard** (partial, `fabro-workflows/src/`):
- Commit `cb0c016e` added a bootstrap guard for fresh projects
- **Gap**: No language-specific project health markers exist

**Quality gate** (`lib/crates/fabro-synthesis/src/render.rs`):
- `implementation_quality_command()` exists (lines ~2300–2500)
- Checks for placeholders, warnings, semantic risk, lane sizing, etc.
- **Gap**: No TypeScript-specific checks (no `any` detection, no missing import checks)

**Prompt/workflow asset references** (`lib/crates/fabro-synthesis/src/render.rs`):
- `prompt_path()` generates: `@../../prompts/{family}/{slug}/{name}.md`
- These are **relative** paths that may break in detached run directories

### What Is Missing

| Milestone | Status | Gap |
|-----------|--------|-----|
| Scaffold-first ordering test | ❌ Not done | No unit test `scaffold_first` |
| Bootstrap verification gate | ❌ Not done | No language-specific health checks in scaffold workflow |
| Runtime-stable asset refs | ❌ Not done | `@../../prompts/...` breaks in detached runs |
| TypeScript quality checks | ❌ Not done | No `any` detection, no import validation |
| tonofcrap 30-cycle validation | ⏸ Deferred | Requires above milestones |
| Fresh Rust project validation | ⏸ Deferred | Requires above milestones |

## Architecture

### Target Greenfield Flow

```
scaffold ──> [bootstrap verify] ──> feature-1 ──┐
                                 ──> feature-2 ──┘
```

### Bootstrap Verification Gate

The scaffold lane workflow must include a verification node that checks language-specific project health markers **before** feature lanes dispatch. This gate is inserted into the scaffold workflow's `verify` stage.

#### Language-Specific Health Markers

| Language | Required Markers | Verification Command |
|----------|-----------------|---------------------|
| Node.js/TypeScript | `package.json` exists, `node_modules/` populated, `tsconfig.json` present | `test -f package.json && test -d node_modules && test -f tsconfig.json` |
| Rust | `Cargo.toml` valid, `cargo check` passes | `cargo check --quiet` |
| Python | `pyproject.toml` or `requirements.txt` exists | `test -f pyproject.toml -o -f requirements.txt` |

#### Implementation Location

- **File**: `lib/crates/fabro-synthesis/src/render.rs`
- **Function**: `render_workflow_graph()`
- **Target**: `WorkflowTemplate::Bootstrap` and `WorkflowTemplate::ServiceBootstrap` branches

### Runtime-Stable Asset Resolution

#### Problem

Generated workflows contain prompt references like:
```
@../../prompts/bootstrap/rules/plan.md
```

When Fabro copies `graph.fabro` into `~/.fabro/runs/<run-id>/`, this path resolves relative to the run directory, not the project root.

#### Solution Options

1. **Inline prompts**: Copy prompt content directly into the workflow graph
2. **Absolute paths with env var substitution**: `@${FABRO_PROJECT_ROOT}/malinka/prompts/...`
3. **Copy prompts to run context**: Bundle prompts alongside the workflow in the run dir

**Decision**: Option 2 — use `${FABRO_PROJECT_ROOT}` environment variable substitution. This preserves the current workflow format while making resolution runtime-stable.

#### Implementation Location

- **File**: `lib/crates/fabro-synthesis/src/render.rs`
- **Function**: `render_workflow_graph()` → `prompt_path()` lambda
- **Change**: Replace `@../../prompts/` with `@${FABRO_PROJECT_ROOT}/malinka/prompts/`

### TypeScript Quality Checks

#### Required Checks

1. **`any` usage detection**: Exported function signatures should not use `any`
   ```bash
   rg -n 'export (function|const|class) \w+.*: any' -g '*.ts'
   ```

2. **Missing import validation**: Test files should import from the module they claim to test
   - This is heuristic-based; check that test file paths match imported module paths

3. **Schema file existence**: If plan context declares a schema, verify it exists
   - Extract schema path from plan context (look for `schema`, `types`, `api` references)
   - `test -f` the extracted path

#### Implementation Location

- **File**: `lib/crates/fabro-synthesis/src/render.rs`
- **Function**: `implementation_quality_command()`
- **Add**: TypeScript-specific quality script block after existing Rust/Python checks

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

## Related Documents

- `genesis/plans/004-greenfield-bootstrap-reliability.md` — ExecPlan with Milestones 1-6
- `genesis/plans/001-master-plan.md` — Phase 0 dependency ordering
- `plans/032426-greenfield-bootstrapping-and-code-quality.md` — Historical root cause analysis
