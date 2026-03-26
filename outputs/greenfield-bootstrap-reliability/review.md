# Greenfield Bootstrap and Runtime Asset Reliability — Review

## Assessment Date
2026-03-26

## Genesis Plan
`genesis/plans/004-greenfield-bootstrap-reliability.md`

## Current Implementation Status

### ✅ Scaffold-First Ordering (Planning)

**Location**: `lib/crates/fabro-synthesis/src/planning.rs`, lines 752–843

**Code**:
```rust
// Scaffold-first ordering: identify infrastructure/scaffold plans and
// inject them as implicit dependencies for all non-infrastructure plans.
// This ensures project scaffolding (package.json, tsconfig, schema) completes
// before feature lanes dispatch (agents need package.json, tsconfig, schema before writing code).
// Only applies to plans from YAML plan-mappings (Opus-decomposed) to avoid
// false positives on legacy master-plan-based projects.
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
- Explicit `PlanMappingSource::Contract` check prevents false positives on legacy master-plan projects
- Handles composite scaffold plans by resolving to the last child
- Properly excludes infrastructure plans themselves from dependency injection
- Comprehensive filtering by category AND ID

**Concerns**:
- **No unit test**: Logic exists but no `#[test]` validates it
- Debug output uses `eprintln!` instead of structured logging (minor)

**Missing proof**: `cargo nextest run -p fabro-synthesis -- scaffold_first` does not exist

---

### ❌ Bootstrap Verification Gate (Render)

**Location**: `lib/crates/fabro-synthesis/src/render.rs`, `render_workflow_graph()`

**Current State**: `WorkflowTemplate::Bootstrap` generates a workflow graph with `specify → review → polish → verify` stages. No language-specific health check exists between scaffold completion and feature lane dispatch.

**Current Bootstrap workflow structure**:
```
start → specify → review → polish → verify → exit
                                    ↘ polish (retry)
```

**Gap**: The scaffold lane outputs no signal that `package.json`/`tsconfig.json`/etc. are present before feature lanes begin. A feature lane that runs `npx tsc --noEmit` or `cargo check` immediately after scaffold dispatch will fail if scaffold hasn't finished.

**Required addition** — insert `bootstrap_health` node:
```
start → specify → review → polish → bootstrap_health → verify → exit
                                                   ↘ polish (retry)
```

**Language detection**: Must derive project type from `target_repo` before rendering. Check for `package.json` (Node/TS), `Cargo.toml` (Rust), `pyproject.toml`/`requirements.txt` (Python).

---

### ❌ Runtime-Stable Asset Resolution (Render)

**Location**: `lib/crates/fabro-synthesis/src/render.rs`, line 1913

**Current code**:
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

**Problem**: `../../prompts/` resolves relative to the workflow file location. When `fabro run` copies `graph.fabro` into `~/.fabro/runs/<run-id>/`, the path resolves under `~/.fabro/` instead of the project root. Every detached run fails prompt loading.

**Impact**: This breaks `fabro validate` on any run directory, not just greenfield projects.

**Fix**: Replace with `${FABRO_PROJECT_ROOT}` substitution:
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

**Dependency**: Requires `fabro-workflows` to set `FABRO_PROJECT_ROOT` in the run context. Check:
- `lib/crates/fabro-workflows/src/handler/agent.rs`
- `lib/crates/fabro-cli/src/commands/run.rs`

---

### ❌ TypeScript Quality Checks (Render)

**Location**: `lib/crates/fabro-synthesis/src/render.rs`, `implementation_quality_command()`, lines 2232–2440

**Current coverage**:
- ✅ Rust: overflow detection, f64 truncation warnings, semantic risk scanning
- ✅ Python: syntax validation, duplicate function detection
- ✅ JS/TS files: placeholder scanning via `scan_placeholder`
- ❌ TypeScript-specific: no `any` detection, no import validation

**Required TypeScript checks**:

1. **`any` in exported signatures**:
   ```bash
   rg -n 'export (function|const|class|interface|type) \w+.*: any' -g '*.ts' -g '*.tsx'
   ```

2. **Missing imports in test files**:
   - Heuristic: `foo.test.ts` should import from `foo.ts`
   - Check for path mismatches between test file name and imported module

3. **Schema file existence** (if declared in plan context):
   - Extract schema path from `prompt_context`
   - `test -f "$schema_path"`

**Note**: `scan_placeholder` already scans `*.ts`/`*.tsx` for generic placeholders (`TODO`, `stub`, `placeholder`, etc.), but misses type-level issues like `any`.

---

## Test Coverage Assessment

| Test | Status | Location |
|------|--------|----------|
| `scaffold_first` | ❌ Missing | Need `#[test]` in `src/planning.rs` |
| `bootstrap_verify` | ❌ Missing | Need `#[test]` in `src/render.rs` |
| `quality_typescript` | ❌ Missing | Need `#[test]` in `src/render.rs` |
| Existing synthesis tests | ✅ ~5 passing | `tests/synthesis.rs` |
| Existing render tests | ✅ ~86 passing | `src/render.rs` (inline `#[test]` blocks) |

---

## Risks and Mitigations

| Risk | Status | Mitigation |
|------|--------|------------|
| False-positive infrastructure detection | Mitigated | Requires `PlanMappingSource::Contract` |
| Bootstrap passes in-repo but fails in detached runs | Not mitigated | Requires `${FABRO_PROJECT_ROOT}` fix |
| TypeScript quality gate passes with `any[]` parameters | Mitigated | Scaffold-first ordering ensures scaffold completes first |

---

## Implementation Checklist

### Milestone 1: Scaffold-First Ordering Test
- [ ] Add `#[test] scaffold_first` to `lib/crates/fabro-synthesis/src/planning.rs`
- [ ] Create test fixture with `PlanCategory::Infrastructure` plan and feature plan
- [ ] Verify feature plan intent dependencies include scaffold plan ID
- [ ] Verify infrastructure plans are NOT injected with themselves as dependencies

### Milestone 2: Bootstrap Verification Gate
- [ ] Add `bootstrap_health_command(language: &str) -> String` to `lib/crates/fabro-synthesis/src/render.rs`
- [ ] Implement language detection from `target_repo` (check file existence heuristics)
- [ ] Insert `bootstrap_health` node into `WorkflowTemplate::Bootstrap` graph
- [ ] Add `#[test] bootstrap_verify` test

### Milestone 3: Runtime-Stable Asset Resolution
- [ ] Update `prompt_path` closure in `render_workflow_graph()` to use `${FABRO_PROJECT_ROOT}`
- [ ] Verify `fabro-workflows` sets `FABRO_PROJECT_ROOT` in run context
- [ ] Add integration test for detached run directory validation

### Milestone 4: TypeScript Quality Checks
- [ ] Add TypeScript detection to `implementation_quality_command()`
- [ ] Implement `any` usage detection in exported signatures
- [ ] Implement missing import validation for test files
- [ ] Add `#[test] quality_typescript` test

### Milestones 5–6: Live Validations
- [ ] Run tonofcrap 30-cycle autodev
- [ ] Create fresh Rust project fixture
- [ ] Run scaffold-first validation

---

## Effort Estimate

| Milestone | Estimated Effort | Complexity |
|-----------|-----------------|------------|
| 1: Scaffold-First Test | 2–4 hours | Low |
| 2: Bootstrap Verification | 4–8 hours | Medium |
| 3: Runtime-Stable Assets | 2–4 hours | Medium |
| 4: TypeScript Quality | 4–6 hours | Medium |
| 5–6: Live Validations | 8–16 hours | High |

**Total**: ~20–38 hours

---

## Recommendation

Proceed with implementation in genesis order:

1. **First** (Milestone 1): Add `scaffold_first` test — validates existing code works before building on top of it
2. **Second** (Milestone 3): Runtime-stable asset resolution — small change, high impact, unblocks detached-run validation
3. **Third** (Milestone 2): Bootstrap verification gate — requires language detection design, builds on scaffold-first
4. **Fourth** (Milestone 4): TypeScript quality checks — follows the existing Python/Rust pattern in `implementation_quality_command()`
5. **Fifth** (Milestones 5–6): Live validations — end-to-end proof on tonofcrap and a fresh Rust project

The scaffold-first ordering logic is already correct; adding a test first provides a regression guard for subsequent changes.
