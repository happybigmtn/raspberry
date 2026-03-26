# Greenfield Bootstrap and Runtime Asset Reliability — Lane Spec

**Status**: Phase 0 — In Progress  
**Plan**: `genesis/plans/004-greenfield-bootstrap-reliability.md`  
**Reviewed**: 2026-03-26

## Purpose

Ensure `fabro synth create` on a fresh repo produces a package where:
1. **Scaffold-first ordering**: infrastructure/scaffold lanes complete before any feature lane dispatches
2. **Bootstrap verification**: language-specific project health is confirmed before downstream lanes run
3. **Runtime-stable assets**: generated prompt/workflow references resolve correctly in `~/.fabro/runs/` (detached from the original checkout)
4. **Type-aware quality**: TypeScript projects are checked for `any[]` usage, missing imports, and type-safe exports

This lane addresses the tonofcrap failure mode: agents writing `.ts` files into a repo with no `package.json`, causing `npx convex dev --typecheck` to fail because `npx` has nothing to run.

## Architecture

### 1. Scaffold-First Ordering — `planning.rs`

Implemented in `derive_registry_plan_intents()` (`lib/crates/fabro-synthesis/src/planning.rs`, lines ~756–900).

**Detection logic:**
```rust
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

- Only `PlanMappingSource::Contract` plans participate (YAML-mapped, Opus-decomposed). Legacy master-plan projects are excluded by design.
- Composite scaffold plans resolve to their last child (`children.last()`) as the actual dependency.
- Every non-infrastructure, non-scaffold plan gets scaffold dependencies injected.

**Current status**: ✅ Implemented and structurally sound.

### 2. Bootstrap Verification Gate — `render.rs`

The current `preflight_command()` (line 2167) is a **non-failing wrapper** around verify commands:
```rust
fn preflight_command(verify_command: &str) -> String {
    format!("set +e\n{}\ntrue", body)
}
```

It strips `set -e`, runs the verify command with `set +e`, and exits with `true`. This prevents early workflow exit but does **not** perform language-specific bootstrap validation.

**What the plan calls for** (not yet implemented):
| Language | Checks |
|----------|--------|
| TypeScript | `package.json` exists, `node_modules/` populated, `tsconfig.json` present |
| Rust | `Cargo.toml` valid, `cargo check --workspace` passes |
| Python | `pyproject.toml` or `requirements.txt` exists |

**Current status**: ❌ Not implemented. This is the primary blocker for Phase 0 gate.

### 3. Runtime-Stable Asset Resolution — `render.rs`, `agent.rs`, `run.rs`

Generated workflows reference prompts via `@malinka/prompts/{family}/{slug}/{stage}.md` paths (line ~1907 in `render.rs`). The `@` prefix is a repo-relative convention intended to survive the runtime handoff when `graph.fabro` is copied into `~/.fabro/runs/`.

**What the plan calls for**:
- Prompt assets must be explicitly copied into the run context, OR
- `@` paths must resolve against the worktree, not `~/.fabro/`

**Current status**: ⚠️ Convention only — no explicit asset-copying or runtime validation implemented. Prompts are assumed to exist at the referenced path.

### 4. Type-Aware Quality Checks — `render.rs`

`implementation_quality_command()` (line 2224) includes:
- Placeholder/stub detection across `.rs`, `.ts`, `.tsx`, `.py`, `.js`
- Missing owned surface detection
- Security semantic risk scanning
- Lane sizing debt for TUI layout files
- Python syntax and duplicate function detection
- Contract deliverable verification

**What the plan calls for** (not yet implemented):
```bash
# TypeScript any[] in exported signatures
rg -n 'export.*:\s*any(\[\])?' -g '*.ts' -g '*.tsx'
# Test files importing from tested module
rg -n 'from.*\.\./' -g '*.test.ts' -g '*.spec.ts'
```

**Current status**: ❌ TypeScript-specific checks not implemented.

## Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| Scaffold detection by category/ID | ✅ Implemented | `planning.rs` lines 761–770 |
| Scaffold deps injected for non-infra plans | ✅ Implemented | `planning.rs` lines 816–840 |
| Composite scaffold resolved to last child | ✅ Implemented | `planning.rs` lines 821–833 |
| Preflight non-failing wrapper | ✅ Implemented | `render.rs` lines 2167–2175 |
| Bootstrap verification (language-specific) | ❌ NOT Implemented | Blocked |
| Runtime-stable `@malinka/prompts/` asset refs | ⚠️ Convention only | No explicit copy/validation |
| TypeScript `any[]` detection | ❌ NOT Implemented | Blocked |
| Import verification | ❌ NOT Implemented | Blocked |

## Test Coverage

**Existing tests** (named and confirmed present):
- `bootstrap_workflow_retries_verify_via_polish` — Bootstrap template verify retry
- `service_bootstrap_workflow_retries_verify_outputs_via_polish` — Service bootstrap verify outputs
- `implementation_quality_command_does_not_treat_future_slice_wording_as_artifact_mismatch` — Quality false-positive guard
- `implementation_quality_command_flags_lane_sizing_debt_for_layout_slices` — TUI layout sizing debt detection

**Required tests** (do not yet exist — from plan Milestones 1, 2, 4, 6):
| Test | Plan Milestone | Purpose |
|------|----------------|---------|
| `scaffold_first` | Milestone 1 | Verify scaffold plans ordered before feature plans |
| `bootstrap_verify` | Milestone 2 | Verify language-specific bootstrap gates |
| `quality_typescript` | Milestone 4 | Verify TypeScript-specific quality checks |
| `greenfield_rust` | Milestone 6 | End-to-end fresh Rust project validation |

**Current status**: Required tests do not exist. Writing them is the primary Phase 0 gate activity.

## Dependencies

- **Blocked by**: None (Phase 0 greenfield plan)
- **Blocks**: Plan 003 (autodev efficiency — needs reliable bootstrap before autodev can be measured)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| False positive infrastructure detection (mislabeled category) | Medium | Medium | `PlanMappingSource::Contract` filter; only YAML-mapped plans affected |
| Composite scaffold last-child assumption wrong | Low | Medium | Last child = last to complete; explicit ordering metadata TBD |
| `@malinka/prompts/` path not found at runtime | Medium | High | Convention only — no explicit copy/validation yet |
| `preflight_command` masks real verify failures | Low | High | Non-failing by design; verify result still recorded |
| TypeScript quality gaps | Medium | Medium | Generic placeholder detection catches some issues |

## Implementation Phases

### Phase 0 (Current)
- [x] Scaffold-first ordering in `planning.rs`
- [ ] Bootstrap verification gate in `render.rs` (language-specific)
- [ ] Runtime-stable asset copying/validation
- [ ] TypeScript `any[]` and import quality checks
- [ ] `scaffold_first` test
- [ ] `bootstrap_verify` test
- [ ] `quality_typescript` test
- [ ] `greenfield_rust` test
- [ ] tonofcrap 30-cycle validation
- [ ] Fresh Rust project validation

### Phase 1 (TBD — hardening)
- Prompt path traversal guard
- Composite scaffold explicit ordering metadata
- Full runtime asset validation in detached run context

## Key Files

| File | Role |
|------|------|
| `lib/crates/fabro-synthesis/src/planning.rs` | Scaffold-first dependency injection |
| `lib/crates/fabro-synthesis/src/render.rs` | Preflight wrapper, quality commands, prompt refs |
| `lib/crates/fabro-workflows/src/handler/agent.rs` | Runtime prompt resolution |
| `lib/crates/fabro-cli/src/commands/run.rs` | Run dispatch, worktree setup |

## Decision Log

| Decision | Rationale | Date |
|----------|-----------|------|
| Limit scaffold detection to `PlanMappingSource::Contract` | Avoids false positives on legacy master-plan projects | 2026-03-26 |
| Resolve composite scaffold to last child | Last child = last lane to complete; dependency semantics | 2026-03-26 |
| Use `@malinka/prompts/` prefix for repo-relative paths | Convention survives runtime handoff without global symlinks | 2026-03-26 |
| `preflight_command` is non-failing | Workflow should not exit on preflight; result is recorded separately | 2026-03-26 |
