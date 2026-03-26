# Quality: Fresh Rust Project Validation

## Code quality

### Test file: `lib/crates/fabro-synthesis/tests/greenfield_rust.rs`

- **No compiler warnings** — test compiles clean
- **Clear test names** — each test name describes exactly what it validates
- **Helper functions** — bootstrap and verification logic extracted into reusable helpers
- **Descriptive assertions** — each assertion has a message explaining what failed

### Design quality

- **Isolation** — each test uses an independent temporary directory
- **Cleanup** — `tempfile::TempDir` ensures automatic cleanup
- **No hardcoded paths** — all paths derived from temp directory
- **Graceful error handling** — uses `Result` types and `expect` with messages

## Test quality

### Coverage

| Behavior | Test |
|----------|------|
| Minimal binary project bootstrap | `greenfield_rust_minimal_binary` |
| Workspace project bootstrap | `greenfield_rust_workspace` |
| Synthesis pipeline end-to-end | `greenfield_rust_synthesis_pipeline` |
| Bootstrap verification | `greenfield_rust_bootstrap_verify` |
| Invalid project rejection | `greenfield_rust_invalid_project_rejected` |
| Full lifecycle | `greenfield_rust_full_lifecycle` |
| Scaffold-first ordering | `greenfield_rust_scaffold_first_ordering` |
| Health markers | `greenfield_rust_health_markers` |

### Full lifecycle test

`greenfield_rust_full_lifecycle` drives the complete pipeline:
1. Bootstrap workspace
2. Verify tests pass
3. Add SPEC.md planning corpus
4. Author blueprint
5. Verify blueprint structure
6. Render blueprint
7. Verify files written
8. Verify project still compiles after rendering

### Behavioral assertions

Each test makes specific behavioral assertions, not just compilation checks:
- File existence assertions
- `cargo metadata` success/failure assertions  
- Blueprint structure assertions
- Written files verification

## Acceptance criteria

| Criterion | Evidence |
|-----------|----------|
| Test exists and runs | `cargo nextest run -p fabro-synthesis -- greenfield_rust` |
| Rust project bootstrapped | `bootstrap_minimal_rust_project` and `bootstrap_workspace_rust_project` |
| Project compiles | `cargo check` succeeds in all compilation tests |
| Basic structure verified | Assertions on Cargo.toml, src/main.rs, src/lib.rs |
| No panics | Uses `Result` error handling with `expect` messages |
| Cleanup | `tempfile::tempdir()` automatic cleanup |
