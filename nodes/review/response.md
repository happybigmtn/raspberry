Based on my review of the spec, review documents, and source code verification, here is my assessment:

## Review Summary

### Current State
The `greenfield-bootstrap-reliability` lane is in **specification-complete, implementation-pending** state. The spec correctly identifies four gaps in the current codebase:

1. **Scaffold-first ordering test** — Code exists at `planning.rs:752-843` but has no unit test
2. **Bootstrap verification gate** — No language-specific health checks in Bootstrap workflows  
3. **Runtime-stable asset refs** — `prompt_path()` uses relative `@../../prompts/` paths
4. **TypeScript quality checks** — `implementation_quality_command()` lacks TypeScript-specific validation

---

## Nemesis-Style Security Review

### Pass 1 — First-Principles Challenge

**Trust Boundaries & Authority**
- The scaffold-first logic trusts plan metadata from `PlanRegistry` without additional validation
- Plan IDs flow directly into dependency injection logic — assumes filesystem-backed registry is authoritative
- No capability checks before injecting implicit dependencies

**Dangerous Actions**
- Shell command generation in `implementation_quality_command()` (line 2232+) embeds user-controlled plan content (goal text, artifact paths, prompt_context) into shell scripts
- **Deviation from codebase convention**: `shell_single_quote()` (line 2451) uses manual `replace('\'', "'\\''")` instead of `shlex::try_quote()` as mandated by CLAUDE.md

**Secret Handling**
- No secrets in this code path — correct separation of concerns

### Pass 2 — Coupled-State Review

**Paired State / Protocol Surfaces**
- Scaffold dependency injection creates implicit coupling between `PlanRegistry::plans` and `LaneIntent::dependencies`
- **Asymmetry not explained**: Composite scaffold resolution (line ~790) maps to last child, but no reverse mapping exists for verification
- Multiple iterations over plans could cause duplicate dependency injection if not idempotent (the code checks `!dependencies.iter().any(|d| d.unit == *scaffold_id)` — correctly idempotent)

**External-Process Control**
- The `${FABRO_PROJECT_ROOT}` substitution proposed in spec does not exist in codebase
- If `FABRO_PROJECT_ROOT` is unset, `@${FABRO_PROJECT_ROOT}/malinka/prompts/...` expands to `@/malinka/prompts/...` — will fail or resolve to root filesystem
- **Missing**: No validation that the environment variable is set before executing workflows

**Failure Modes**
- Bootstrap workflow missing health gate means feature lanes may execute before `package.json`/`Cargo.toml` exists — agents will fail with confusing errors
- Relative path `@../../prompts/` in detached runs resolves relative to `~/.fabro/runs/<id>/`, potentially reading arbitrary files via directory traversal

**Idempotent Retries**
- Scaffold dependency injection correctly checks for existing dependencies before adding
- No mechanism to detect when scaffold plans have actually completed vs. just been scheduled

---

## Blockers for Implementation

| Blocker | Severity | Description |
|---------|----------|-------------|
| `FABRO_PROJECT_ROOT` env var | **High** | Spec requires variable that doesn't exist; `fabro-workflows` and `fabro-cli` need to set it |
| `shell_single_quote` deviation | Medium | Should use `shlex::try_quote()` per CLAUDE.md guidelines |
| No runtime asset validation | Medium | Workflows can be generated with broken prompt references |

---

## Recommendation

**Status**: Proceed with implementation **with modification**

The spec correctly identifies the gaps. However, the following should be addressed:

1. **Add `FABRO_PROJECT_ROOT` to execution context** BEFORE changing `prompt_path()` — this is a cross-crate dependency requiring changes in:
   - `fabro-cli/src/commands/run.rs` (detached run setup)
   - `fabro-workflows/src/handler/agent.rs` (workflow execution)

2. **Replace `shell_single_quote`** with `shlex::try_quote()` or use the existing `shell_quote()` helper from `fabro-workflows` to follow codebase conventions

3. **Consider fallback for missing env var**: If `FABRO_PROJECT_ROOT` is not set, workflows should fail fast with a clear error rather than attempting path resolution

4. **Milestone ordering**: The review's recommended order (test → asset resolution → bootstrap gate → TypeScript checks) is correct for risk mitigation

---

## Verification Commands

```bash
# Verify scaffold-first logic exists
cargo nextest run -p fabro-synthesis -- scaffold_first  # Expected: test not found

# Check current prompt_path output
grep -A5 'let prompt_path' lib/crates/fabro-synthesis/src/render.rs

# Verify no FABRO_PROJECT_ROOT exists
grep -r "FABRO_PROJECT_ROOT" lib/crates/  # Expected: no results
```