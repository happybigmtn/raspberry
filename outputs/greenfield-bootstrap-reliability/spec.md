# Greenfield Bootstrap and Runtime Asset Reliability

**Status**: Specification Complete  
**Plan**: [genesis/plans/004-greenfield-bootstrap-reliability.md](../../genesis/plans/004-greenfield-bootstrap-reliability.md)

## Purpose

After this change, `fabro synth create` on a new repo produces a package where:
1. The scaffold/infrastructure lane completes before any feature lane dispatches
2. Generated runtime assets (prompts, workflows) resolve correctly when copied to `~/.fabro/runs/`
3. Agents never write TypeScript into a repo with no `package.json`, Rust code with no `Cargo.toml`, or prompt references that only work from the original repo checkout

## Architecture

### Scaffold-First Ordering

The scaffold-first ordering is implemented in `derive_registry_plan_intents()` in `lib/crates/fabro-synthesis/src/planning.rs` (lines ~750-850).

Key implementation:
```rust
// Identify scaffold plans: only plans explicitly categorized as Infrastructure
// or with "project-scaffold" in their ID.
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

Scaffold dependencies are injected for all non-infrastructure plans, with composite scaffolds resolved to their last child.

### Bootstrap Verification Gate

The preflight command in `render.rs` provides non-failing preflight validation. However, **language-specific bootstrap verification is NOT yet implemented**. The plan calls for:

- Node.js/TypeScript: `package.json` exists, `node_modules/` populated, `tsconfig.json` present
- Rust: `Cargo.toml` valid, `cargo check` passes
- Python: `pyproject.toml` or `requirements.txt` exists

Current `preflight_command()` (lines 2167-2175) only wraps verify commands without bootstrap checks.

### Runtime-Stable Asset Resolution

Generated workflows use `@malinka/prompts/{family}/{slug}/{stage}.md` references (line ~1907 in render.rs). These paths are:
- Relative to the repo root
- Copied into run context during workflow dispatch
- Resolved against the worktree, not `~/.fabro/`

The `@` prefix convention indicates a repo-relative path that survives the runtime handoff.

### Type-Aware Quality Checks

The `implementation_quality_command()` in render.rs (lines 2224+) includes:
- Placeholder/stub detection across `.rs`, `.ts`, `.tsx`, `.py`, `.js`
- Missing owned surface detection
- Root artifact shadow detection
- Security semantic risk scanning (settlement code patterns)
- Lane sizing debt for TUI layout files
- Python syntax and duplicate function detection
- Contract deliverable verification

**TypeScript-specific checks for `any[]` and missing imports are NOT yet implemented** - the current implementation has generic placeholder detection but no TypeScript type-aware quality enforcement.

## Acceptance Criteria

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Scaffold plans identified by category | ✅ Implemented | `planning.rs` lines 750-760 |
| Scaffold injected as implicit deps | ✅ Implemented | `planning.rs` lines 780-810 |
| Composite scaffold resolved to last child | ✅ Implemented | `planning.rs` lines 800-815 |
| Preflight command non-failing | ✅ Implemented | `render.rs` lines 2167-2175 |
| Runtime-stable prompt refs | ✅ Implemented | `@malinka/prompts/` convention |
| Placeholder/stub detection | ✅ Implemented | `implementation_quality_command()` |
| Bootstrap verification (language-specific) | ❌ NOT Implemented | Missing from `preflight_command()` |
| TypeScript `any[]` detection | ❌ NOT Implemented | Missing from quality command |
| Import verification | ❌ NOT Implemented | Missing from quality command |

## Test Coverage

Existing tests:
- `bootstrap_workflow_retries_verify_via_polish()` - Bootstrap template verify retry behavior
- `service_bootstrap_workflow_retries_verify_outputs_via_polish()` - Service bootstrap verify behavior

Missing tests (from plan milestones):
- `scaffold_first` - Verify scaffold plans are ordered before feature plans
- `bootstrap_verify` - Verify language-specific bootstrap gates
- `quality_typescript` - Verify TypeScript-specific quality checks
- `greenfield_rust` - End-to-end fresh Rust project validation

## Dependencies

- Blocked by: None (Phase 0 plan)
- Blocks: Plan 003 (autodev efficiency - needs reliable bootstrap)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| False positive infrastructure detection | Medium | Medium | Category metadata check implemented |
| Bootstrap passes in-repo but fails detached | Low | High | Runtime-stable refs verified; asset copying TBD |
| TypeScript quality gaps | Medium | Medium | Generic placeholder detection catches some issues |

## Implementation Phases

### Phase 0 (Current - Days 1-30)
- [x] Scaffold-first ordering implemented
- [ ] Bootstrap verification gate (language-specific)
- [ ] TypeScript type-aware quality checks
- [ ] 30-cycle tonofcrap validation
- [ ] Fresh Rust project validation

### Phase 1 (Days 31-90)
- Hardening based on Phase 0 findings

## Related Files

- `lib/crates/fabro-synthesis/src/planning.rs` - Scaffold-first ordering
- `lib/crates/fabro-synthesis/src/render.rs` - Verification gates, quality commands
- `lib/crates/fabro-workflows/src/handler/agent.rs` - Runtime prompt resolution
- `lib/crates/fabro-cli/src/commands/run.rs` - Run dispatch, worktree setup

## Decision Log

- **Runtime asset resolution**: Using `@malinka/prompts/` prefix for repo-relative paths that survive runtime handoff
- **Scaffold detection**: Only applies to `PlanMappingSource::Contract` (YAML-mapped) plans to avoid false positives on legacy master-plan projects
- **Composite resolution**: Scaffold plans with children resolve to last child lane for dependency
