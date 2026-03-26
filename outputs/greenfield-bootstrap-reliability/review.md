# Greenfield Bootstrap and Runtime Asset Reliability — Lane Review

**Review Date**: 2026-03-26  
**Reviewer**: Genesis / Nemesis Security Review  
**Plan**: `004-greenfield-bootstrap-reliability`  
**Stage**: Polish (Durable Artifact)

---

## Verdict

**Phase 0 gate: NOT READY.**  
The scaffold-first ordering is correctly implemented. The bootstrap verification gate and TypeScript quality checks are not implemented. Runtime-stable asset resolution is convention-only with no explicit validation.

---

## Pass 1: First-Principles Challenge

### Trust Boundaries

| Boundary | Trust Level | Finding |
|----------|-------------|---------|
| `PlanCategory::Infrastructure` metadata | Medium | Comes from YAML contract; mislabeling bypasses scaffold ordering |
| `"project-scaffold"` / `"workspace-setup"` plan ID strings | Low risk | String match only; affects dependency injection, not code execution |
| `@malinka/prompts/` path convention | Risk | No runtime validation that the path resolves or assets are copied |

**Finding**: `scaffold_plan_ids` filter (planning.rs:761–770) trusts both `category` and `plan_id` without secondary validation. A mislabeled plan bypasses scaffold-first ordering.

### Authority Assumptions

- Only `PlanMappingSource::Contract` plans participate in scaffold detection — legacy master-plan projects are intentionally excluded. This is documented and acceptable.

### Dangerous Action Surface

| Action | Trigger | Safeguard | Assessment |
|--------|---------|-----------|------------|
| Scaffold dependency injection | Any YAML-mapped plan | Category metadata check | ✅ Low risk |
| Preflight command execution | Workflow engine | `set +e` + `true` suffix | ✅ Non-failing by design |
| Quality command execution | Workflow engine | Sandboxed shell | ✅ Low risk |

### What Can Go Wrong

1. **Wrong**: A feature plan with `category: Infrastructure` gets treated as a scaffold, blocking all downstream lanes.
2. **Wrong**: A composite scaffold with one child marked as feature gets last-child resolution wrong.
3. **Wrong**: `@../../../etc/passwd` prompt path traversal — no validation at render time or runtime.
4. **Wrong**: Bootstrap verify command is a no-op — `preflight_command` wraps `true`, scaffold lane completes, feature lanes dispatch before `package.json` exists.

---

## Pass 2: Coupled-State Review

### State Pair Consistency

| State A | State B | Consistency | Notes |
|---------|---------|-------------|-------|
| `plan.category == Infrastructure` | Scaffold deps injected | ✅ Consistent | Checked in both detection and injection sites |
| `scaffold_plan_ids` | Lane dependencies | ✅ Consistent | Each non-scaffold plan gets all scaffold deps |
| Composite scaffold | Resolved to `children.last()` | ⚠️ Fragile | Assumes last child = last to complete; no ordering metadata |
| `@malinka/prompts/` path | File existence at runtime | ❌ **Gap** | No validation; path may not survive runtime handoff |
| `preflight_command` output | Actual bootstrap state | ❌ **Gap** | `set +e` + `true` always exits 0 |

### Capability Scoping

| Capability | Scope | Assessment |
|------------|-------|------------|
| Scaffold injection | Planning phase only | ✅ Contained |
| Bootstrap verification | **Not implemented** | ❌ Gap |
| Quality enforcement | Per-lane shell, read-only | ✅ Contained |

### Pairing / Idempotence

- Scaffold injection is idempotent (may add duplicate deps; manifest loader deduplicates).
- `preflight_command` is idempotent — always returns 0.
- Quality command generates deterministic `quality.md` report.

### Privilege Escalation Paths

| Path | Assessment |
|------|------------|
| Malicious `category: Infrastructure` label | Bypasses scaffold ordering; no privilege escalation |
| Malicious `plan_id == "project-scaffold"` | Dependency injection only; no code execution |
| `@../../../../../etc/passwd` in prompt ref | ❌ **Not validated** — path could escape worktree |

### External Process Control

| Process | Control | Safety |
|---------|---------|--------|
| `cargo check` | Quality gate | Sandboxed |
| `rg` (ripgrep) | Quality scan | Read-only, sandboxed |
| `python3 -m py_compile` | Python syntax | Sandboxed |
| `npx` / `npm` | ❌ Not called — bootstrap verification missing | N/A |

### Failure Mode Summary

| Failure | Detected? | Recovery | Status |
|---------|-----------|----------|--------|
| Scaffold plan missing | Warning logged only | Lane proceeds anyway | ⚠️ Should fail fast |
| Prompt path not found at runtime | ❌ No | Runtime agent error | ❌ Gap |
| Bootstrap verify fail | ❌ No (not implemented) | N/A | ❌ Gap |
| Quality gate fail | ✅ Yes | `quality_ready: no` → fixup loop | ✅ Handled |

---

## Remaining Blockers for Phase 0 Gate

All four are required before Phase 0 can be considered complete:

| Blocker | Severity | Status | Owner |
|---------|----------|--------|-------|
| Bootstrap verification gate (language-specific) | **High** | Not implemented | Required |
| `bootstrap_verify` test | **High** | Test does not exist | Required |
| `scaffold_first` test | **High** | Test does not exist | Required |
| `quality_typescript` test | Medium | Test does not exist | Required |
| `greenfield_rust` test | **High** | Test does not exist | Required |
| tonofcrap 30-cycle validation | **High** | Not run | Required |
| Prompt path traversal validation | Low | Not implemented | Hardening |
| Runtime-stable asset copy/validation | Medium | Convention only | Required |

---

## Recommendations

### Immediate (Before Phase 0 Gate)

1. **Implement bootstrap verification gate**: Extend `preflight_command()` or add `bootstrap_verify_command()` that checks:
   - TypeScript: `test -f package.json && test -d node_modules && test -f tsconfig.json`
   - Rust: `test -f Cargo.toml && cargo check --workspace --quiet`
   - Python: `test -f pyproject.toml || test -f requirements.txt`

2. **Write `scaffold_first` test**: Create a test blueprint with infrastructure and feature plans. Assert that infrastructure plan IDs appear as dependencies of all feature plans.

3. **Write `bootstrap_verify` test**: Create TypeScript and Rust fixtures. Assert bootstrap gate correctly passes/fails based on fixture state.

4. **Validate `@malinka/prompts/` path scope**: Ensure paths prefixed with `@` cannot escape the worktree. Validate in `render_prompt()` or `agent.rs` before using.

### Near-term (Phase 0–1)

5. **Add TypeScript quality checks** to `implementation_quality_command()`:
   ```bash
   rg -n 'export.*:\s*any(\[\])?' -g '*.ts' -g '*.tsx'
   rg -n 'from.*\.\./' -g '*.test.ts' -g '*.spec.ts'
   ```

6. **Fix composite scaffold assumption**: Add explicit ordering metadata to composite scaffold children, or validate `children.last()` semantics with a test.

7. **Implement explicit asset copying**: When dispatching a run, copy `@malinka/prompts/` assets into the run context, or validate all referenced paths exist before dispatch.

---

## Conclusion

The scaffold-first ordering implementation is **structurally correct** — the detection logic, dependency injection, and composite resolution are all consistent and well-commented. The decision to scope to `PlanMappingSource::Contract` is appropriately conservative.

The **bootstrap verification gate is not implemented**. The current `preflight_command` is a pass-through that never fails. This is the primary blocker.

The **runtime asset resolution is convention-only**. The `@malinka/prompts/` prefix is a reasonable approach, but there is no validation that those paths survive the runtime handoff or stay within the worktree.

**Security posture**: Acceptable for development use. No privilege escalation paths. Prompt path traversal is the one unmitigated risk that should be addressed before production use.

**Next action**: Implement language-specific bootstrap verification in `render.rs` and write the four required tests (`scaffold_first`, `bootstrap_verify`, `quality_typescript`, `greenfield_rust`) before re-reviewing for Phase 0 gate.
