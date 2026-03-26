Contract written to `.fabro-work/contract.md`. The deliverables and acceptance criteria define:

**Deliverable:**
- `lib/crates/fabro-synthesis/tests/bootstrap.rs` — New integration test with `fn bootstrap_verify`

**8 Testable Criteria:**
1. Test executes without "no tests to run" error
2. Blueprint roundtrip serialization (construct → YAML → parse → equal)
3. Valid blueprint passes `validate_blueprint()`
4. Invalid blueprints are rejected with `BlueprintError`
5. `BlueprintLane::slug()` returns correct derived value
6. All `WorkflowTemplate` variants are constructable
7. `BlueprintProgram` fields serialize/deserialize correctly
8. Test is self-contained (no file I/O dependencies)

**Out of Scope:**
- File-based integration tests (existing `tests/synthesis.rs`)
- `render_blueprint`, `reconcile_blueprint`, `import_existing_package` tests
- Planning authoring tests (require real repo)
- Performance/snapshot tests