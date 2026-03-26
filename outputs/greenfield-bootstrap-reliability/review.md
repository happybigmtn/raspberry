# Greenfield Bootstrap and Runtime Asset Reliability — Lane Review

**Review Date**: 2026-03-26  
**Reviewer**: Genesis / Nemesis Security Review  
**Plan**: 004-greenfield-bootstrap-reliability  

## Executive Summary

This lane addresses critical bootstrap-time failures observed on the tonofcrap project where agents wrote TypeScript code into repos lacking `package.json` or `tsconfig.json`. The scaffold-first ordering is **correctly implemented**, but **bootstrap verification gates and TypeScript-specific quality checks remain incomplete**. The runtime asset resolution approach is sound but relies on convention-based paths rather than explicit validation.

**Verdict**: Partial implementation — ready for Phase 0 continuation but NOT ready for Phase 0 gate.

## Pass 1: First-Principles Challenge

### Trust Boundaries

| Boundary | Assessment | Finding |
|----------|------------|---------|
| Plan categorization | **Risk**: `PlanCategory::Infrastructure` is trusted without verification | Categories come from YAML contract; could be mislabeled |
| Scaffold detection | **Risk**: String matching on `"project-scaffold"` plan_id | Acceptable — only affects dependency injection, not security |
| Prompt path resolution | **Risk**: `@malinka/prompts/` paths assumed to exist | No runtime validation that prompts are copied to run context |

**Finding**: The `scaffold_plan_ids` filter at planning.rs:753-761 trusts `p.category` and `p.plan_id` without secondary validation. A mislabeled plan could skip scaffold ordering.

### Authority Assumptions

- **Assumption**: Only `PlanMappingSource::Contract` plans participate in scaffold ordering
- **Risk**: Legacy master-plan projects bypass scaffold-first logic entirely
- **Mitigation**: Documented in code comments; acceptable for Phase 0

### Dangerous Action Triggers

| Action | Who Can Trigger | Safeguard | Assessment |
|--------|-----------------|-----------|------------|
| Scaffold dependency injection | Any blueprint with YAML plan-mappings | Category metadata check | ✅ Acceptable |
| Preflight command execution | Workflow engine | `set +e` prefix prevents fail-stop | ⚠️ Preflight doesn't actually verify bootstrap state |
| Quality command execution | Workflow engine | Runs in sandbox | ✅ Acceptable |

## Pass 2: Coupled-State Review

### State Pairs and Consistency

| State A | State B | Consistency Check | Status |
|---------|---------|-------------------|--------|
| `plan.category` | Scaffold dependency injection | Category checked before injection | ✅ Consistent |
| `scaffold_plan_ids` | Lane dependencies | Each non-infra plan gets scaffold deps | ✅ Consistent |
| Composite scaffold | Resolved child lane | Last child selected via `children.last()` | ⚠️ **Fragile** — assumes child ordering |
| Prompt path `@malinka/...` | File existence at runtime | **NO VALIDATION** | ❌ **Gap** |
| Worktree root | Prompt resolution base | Implicit in direct_integration.rs | ⚠️ Convention, not contract |

### Secret Handling

- No secrets in scaffold detection logic
- Quality command may log file contents; uses shell-quoted paths via `shell_single_quote()`

### Capability Scoping

| Capability | Scope | Assessment |
|------------|-------|------------|
| Scaffold injection | Planning phase only | ✅ Scoped to intent derivation |
| Bootstrap verification | **NOT IMPLEMENTED** | ❌ Gap — preflight is no-op |
| Quality enforcement | Per-lane shell script | ✅ Scoped to lane-owned surfaces |

### Pairing/Idempotence

- Scaffold dependency injection is idempotent — same deps added on re-plan
- `dependencies.push()` may create duplicates if plan regenerated multiple times; acceptable as set semantics in manifest

### Privilege Escalation Paths

| Path | Assessment |
|------|------------|
| Malicious plan category | Could bypass scaffold ordering, not a security escalation |
| Malicious scaffold plan_id | Injection only adds dependencies; no code execution |
| Prompt path traversal | `@../../../etc/passwd` — **NOT VALIDATED** |

**Finding**: The `@malinka/prompts/` path convention is not validated. A malicious workflow could reference `@../../../sensitive/file`.

## External Process Control

| Process | Control | Safety |
|---------|---------|--------|
| `cargo check` | Quality gate | Sandboxed |
| `rg` (ripgrep) | Quality scan | Read-only, sandboxed |
| `python3 -m py_compile` | Python syntax check | Sandboxed |
| `npx` / `npm` | **NOT CALLED** — bootstrap verification missing | N/A |

### Operator Safety

- No interactive prompts in bootstrap logic
- `preflight_command()` uses `set +e` to prevent early exit; runs `true` at end

### Idempotent Retries

- Scaffold injection: idempotent (may add duplicate deps, manifest loading deduplicates)
- Preflight: non-failing by design
- Quality: generates deterministic `quality.md` report

### Failure Modes

| Failure | Detection | Recovery | Assessment |
|---------|-----------|----------|------------|
| Scaffold plan missing | Planning warning logged | Lane proceeds anyway | ⚠️ Should fail fast |
| Prompt path not found | **NOT DETECTED** | Runtime agent error | ❌ Gap |
| Bootstrap verify fail | **NOT IMPLEMENTED** | N/A | ❌ Gap |
| Quality gate fail | `quality_ready: no` | Fixup loop | ✅ Handled |

## Remaining Blockers for Phase 0 Gate

| Blocker | Severity | Owner |
|---------|----------|-------|
| TypeScript `any[]` detection missing | Medium | Implementation needed |
| Import verification missing | Medium | Implementation needed |
| Prompt path traversal validation | Low | Hardening recommended |
| Bootstrap verification gate (language-specific) | **High** | **Required for Phase 0** |
| tonofcrap 30-cycle validation | **High** | **Required for Phase 0** |
| Fresh Rust project validation | **High** | **Required for Phase 0** |

## Recommendations

### Immediate (Before Phase 0 Gate)

1. **Implement bootstrap verification gate**: Extend `preflight_command()` or add new `bootstrap_verify_command()` that checks:
   - TypeScript: `package.json`, `node_modules`, `tsconfig.json`
   - Rust: `Cargo.toml`, `cargo check --workspace` passes
   - Python: `pyproject.toml` or `requirements.txt`

2. **Add prompt path validation**: In `render_prompt()` or workflow loading, validate `@` paths stay within `malinka/prompts/` or declared surfaces.

3. **Complete tonofcrap validation**: Run 30-cycle autodev and measure scaffold-first effectiveness.

### Near-term (Phase 0-1)

4. **TypeScript-specific quality checks**: Extend `implementation_quality_command()` with:
   ```bash
   # Check for any in exported signatures
   rg -n 'export.*:\s*any(\[\])?' -g '*.ts' -g '*.tsx'
   # Check test files import from implemented module
   rg -n 'from.*\.\./' -g '*.test.ts' -g '*.spec.ts'
   ```

5. **Composite scaffold ordering**: Validate that `children.last()` is correct (last child = last to complete), or use explicit ordering metadata.

## Conclusion

The scaffold-first ordering implementation is **correct and well-structured**. The decision to limit scaffold detection to `PlanMappingSource::Contract` plans is appropriately conservative. However, **the bootstrap verification gate is not implemented** — the current `preflight_command()` is a pass-through that doesn't verify project health. This is the critical gap blocking Phase 0 gate.

**Security posture**: Acceptable for development use. No privilege escalation paths identified. Prompt path traversal should be validated before production deployment.

**Milestone fit**: Partial. Core ordering logic is complete. Verification and quality gates need completion before Phase 0 gate.

**Next action**: Implement language-specific bootstrap verification in `render.rs` and validate against tonofcrap with 30-cycle autodev.
