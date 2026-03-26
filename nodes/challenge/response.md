Challenge note appended to `outputs/greenfield-bootstrap-reliability-fresh-rust-project-validation/verification.md`. Here's the summary:

---

## Adversarial Challenge Findings

**Proof gate**: 8 tests pass. First proof gate is satisfied.

**G1 — `project_compiles` is a stub** (Medium severity)
- `lib/crates/fabro-synthesis/tests/greenfield_rust.rs:165–190` — the function only checks directory layout (Cargo.toml exists, src/ is a dir). It never invokes `cargo check`.
- The docstrings on `project_passes_cargo_check` and `project_passes_cargo_test` claim cargo invocations but both delegate to `project_compiles`.
- `verification.md` states "Project compiles" with "Structure verification confirms compilability" — this is misleading. Tests pass because the created temp dirs have the right shape, not because cargo successfully compiles anything.
- **Fix**: swap in `Command::new("cargo").args(["check", ...])` or rename and update the verification table.

**G2 — `scaffold_first_ordering` assertions are trivial** (Low severity)
- The test only checks `!units.is_empty()` and `!first_unit.lanes.is_empty()`. Any non-empty blueprint satisfies these. The test exercises real `author_blueprint_for_create` code paths, but the ordering claim is not verified.

**G3 — 585-line test file exceeds 400-line guideline** (Low severity)
- Structural discipline calls for splitting files approaching ~400 lines. This test file is at 585 lines with helper functions and test cases mixed together.

**G4 — No layout invariant checklist in verification.md** (Low — not applicable)
- The challenge requested a board/grid invariant checklist proving no duplicate domain values. This is a synthesis pipeline, not a board game. The checklist is absent because it doesn't apply.

**G5 — No AGENTS.md design pattern violations**
- The settlement arithmetic, GameError/VerifyError, state machine conventions from the challenge prompt are not present in AGENTS.md and do not apply to `fabro-synthesis`.

**Next fixup target**: G1 — `project_compiles` should invoke `cargo check` or the helper names/docstrings/verification table must be corrected to reflect that they only validate structure.