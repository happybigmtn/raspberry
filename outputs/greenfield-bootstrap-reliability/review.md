# Greenfield Bootstrap and Runtime Asset Reliability — Review

## Assessment Date
2026-03-26

## Current Implementation Status

### ✅ Scaffold-First Ordering (Planning)

**Location**: `lib/crates/fabro-synthesis/src/planning.rs`, lines 752–843

**Code Review**:
```rust
// Scaffold-first ordering: identify infrastructure/scaffold plans and
// inject them as implicit dependencies for all non-infrastructure plans.
let scaffold_plan_ids: Vec<String> = registry
    .plans
    .iter()
    .filter(|p| {
        p.mapping_source == PlanMappingSource::Contract
            && (p.category == PlanCategory::Infrastructure
                || p.plan_id == "project-scaffold"
                || p.plan_id == "workspace-setup")
    })
    .map(|p| p.plan_id.clone())
    .collect();
```

**Strengths**:
- Explicit check for `PlanMappingSource::Contract` prevents false positives on legacy master-plan projects
- Handles composite scaffold plans by resolving to last child
- Properly excludes infrastructure plans themselves from dependency injection
- Comprehensive filtering by category AND ID

**Concerns**:
- **No unit test**: The logic exists but has no `#[test]` coverage
- Debug output via `eprintln!` instead of structured logging (minor)

**Test Gap**: `cargo nextest run -p fabro-synthesis -- scaffold_first` does not exist

---

### ❌ Bootstrap Verification Gate (Render)

**Location**: `lib/crates/fabro-synthesis/src/render.rs`

**Current State**: The `render_workflow_graph()` function generates workflows for `WorkflowTemplate::Bootstrap` but does NOT include language-specific health checks.

**Code Gap** (in `render_workflow_graph()` Bootstrap branch):
```rust
// Current Bootstrap workflow (simplified):
verify  [label="Verify", shape=parallelogram, script="...", goal_gate=true]
// No language-specific health check before verify
```

**Required Addition**:
```rust
// Insert between implement and verify stages:
bootstrap_health [label="Bootstrap Health", shape=parallelogram, script="{bootstrap_health_command}", goal_gate=true]
// Then: implement -> bootstrap_health -> verify
```

**Language Detection**: Need to detect project type from `target_repo` before rendering.

---

### ❌ Runtime-Stable Asset Resolution (Render)

**Location**: `lib/crates/fabro-synthesis/src/render.rs`

**Current Code**:
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

**Problem**: `../../prompts/` is relative to the workflow file location, which breaks when workflows are copied to `~/.fabro/runs/<run-id>/`.

**Impact**: Every `fabro run` from a detached run directory will fail prompt loading.

**Required Change**: Use `${FABRO_PROJECT_ROOT}` substitution:
```rust
let prompt_path = |name: &str| -> String {
    format!(
        "@${FABRO_PROJECT_ROOT}/malinka/prompts/{}/{}/{}.md",
        lane.workflow_family(),
        lane.slug(),
        name
    )
};
```

**Consumer Impact**: Requires `fabro-workflows` to set `FABRO_PROJECT_ROOT` environment variable when executing in detached runs. Check `lib/crates/fabro-workflows/src/handler/agent.rs` and `lib/crates/fabro-cli/src/commands/run.rs`.

---

### ❌ TypeScript Quality Checks (Render)

**Location**: `lib/crates/fabro-synthesis/src/render.rs`, `implementation_quality_command()`

**Current Coverage**:
- ✅ Rust: overflow detection, f64 truncation warnings
- ✅ Python: syntax validation, duplicate function detection
- ❌ TypeScript: No checks

**Required TypeScript Checks**:

1. **`any` usage in exported signatures**:
   ```bash
   # Pattern: export function foo(): any OR export const bar: any
   rg -n 'export (function|const|class|interface|type) \w+.*: any' -g '*.ts' -g '*.tsx'
   ```

2. **Missing imports in test files**:
   - Heuristic: if test file is `foo.test.ts`, it should import from `foo.ts`
   - Check for mismatched module paths

3. **Schema file existence** (if declared in plan):
   - Extract schema path from `prompt_context`
   - `test -f "$schema_path"`

---

## Test Coverage Assessment

| Test | Status | Location |
|------|--------|----------|
| `scaffold_first` | ❌ Missing | Need to add to `src/planning.rs` |
| `bootstrap_verify` | ❌ Missing | Need to add to `src/render.rs` |
| `quality_typescript` | ❌ Missing | Need to add to `src/render.rs` |
| Existing synthesis tests | ✅ 5 passing | `tests/synthesis.rs` |
| Existing render tests | ✅ 86 passing | `src/render.rs` |

---

## Dependencies and Risks

### Risks

1. **False positive infrastructure detection**: A plan named "infrastructure" that is actually a feature plan gets treated as a scaffold dependency.
   - **Status**: Mitigated by requiring `PlanMappingSource::Contract`

2. **Bootstrap verification passes in target repo but fails in detached runs**: `@../../prompts/...` resolves under `~/.fabro/` instead of project root.
   - **Status**: Not mitigated — requires `${FABRO_PROJECT_ROOT}` fix

3. **TypeScript quality gate passes with `any[]` parameters**: No schema to check against when scaffold hasn't completed.
   - **Status**: Mitigated by scaffold-first ordering ensuring scaffold completes first

---

## Implementation Checklist

### Milestone 1: Scaffold-First Ordering Test
- [ ] Add `#[test] scaffold_first` to `src/planning.rs`
- [ ] Create test fixture with Infrastructure plan and Feature plan
- [ ] Verify feature plan dependencies include scaffold plan

### Milestone 2: Bootstrap Verification Gate
- [ ] Add `bootstrap_health_command()` function to `src/render.rs`
- [ ] Implement language detection from `target_repo`
- [ ] Add health node to Bootstrap workflow graph
- [ ] Add `#[test] bootstrap_verify` test

### Milestone 3: Runtime-Stable Asset Resolution
- [ ] Update `prompt_path()` to use `${FABRO_PROJECT_ROOT}`
- [ ] Verify `fabro-workflows` sets `FABRO_PROJECT_ROOT` in run context
- [ ] Add integration test for detached run validation

### Milestone 4: TypeScript Quality Checks
- [ ] Add TypeScript detection to `implementation_quality_command()`
- [ ] Implement `any` usage detection
- [ ] Implement missing import validation
- [ ] Add `#[test] quality_typescript` test

### Milestones 5-6: Live Validations
- [ ] Run tonofcrap 30-cycle autodev
- [ ] Create fresh Rust project fixture
- [ ] Run scaffold-first validation

---

## Effort Estimate

| Milestone | Estimated Effort | Complexity |
|-----------|-----------------|------------|
| 1: Scaffold-First Test | 2-4 hours | Low |
| 2: Bootstrap Verification | 4-8 hours | Medium |
| 3: Runtime-Stable Assets | 2-4 hours | Medium |
| 4: TypeScript Quality | 4-6 hours | Medium |
| 5-6: Live Validations | 8-16 hours | High |

**Total**: ~20-38 hours

---

## Recommendation

Proceed with implementation in order:

1. **First**: Add scaffold-first test (validates existing code works)
2. **Second**: Runtime-stable asset resolution (small change, high impact)
3. **Third**: Bootstrap verification gate (requires language detection design)
4. **Fourth**: TypeScript quality checks (follows Python pattern in codebase)
5. **Fifth**: Live validations (end-to-end proof)

The scaffold-first ordering logic is already correct; adding a test first validates the existing implementation before building new features on top of it.
