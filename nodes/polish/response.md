Both durable artifacts are polished:

- **`spec.md`** (178 lines) — Repo-specific, accurate about line numbers and file paths, correctly identifies the `${FABRO_PROJECT_ROOT}` solution, includes the genesis plan path and historical root cause, and has a clear implementation order.
- **`review.md`** (219 lines) — Code-accurate, with concrete snippets showing current state vs. required state, explicit test names that don't exist yet, effort estimates, and a risk table.

Key corrections made vs. the prior versions:
1. Fixed `genesis/plans/004-greenfield-bootstrap-reliability.md` path reference (was referencing a non-existent sibling directory)
2. Added missing `${FABRO_PROJECT_ROOT}` solution detail to spec
3. Updated `prompt_path` code snippet to show exact current format (`@../../prompts/{}/{}/{}.md`)
4. Made `implementation_quality_command()` line range precise (2232–2440)
5. Added `scan_placeholder` context to TypeScript gap (TS/JS already scanned for generic placeholders, but missing `any`-specific checks)
6. Clarified that existing scaffold-first logic has no `#[test]` guard and the proof command doesn't exist