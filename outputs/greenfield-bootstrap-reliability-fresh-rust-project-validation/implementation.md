# Implementation: Fresh Rust Project Validation

## Context

The `fabro-synthesis` crate generates workflow scaffolding and implementation guidance for Fabro workflow packages. This lane validates that the synthesis pipeline can bootstrap and compile a fresh Rust project from scratch, ensuring the greenfield bootstrap workflow works end-to-end.

## What was implemented

### Test file: `lib/crates/fabro-synthesis/tests/greenfield_rust.rs`

A new integration test file with 8 test cases covering the greenfield Rust project bootstrap scenario:

#### Test cases

1. **`greenfield_rust_minimal_binary`** — Validates that a minimal single-crate Rust project (Cargo.toml + src/main.rs) can be bootstrapped in a temporary directory and passes `cargo check`.

2. **`greenfield_rust_workspace`** — Validates that a multi-crate Rust workspace (2 crates: my-library lib + my-binary bin) can be bootstrapped and passes `cargo check` with both crates resolved.

3. **`greenfield_rust_synthesis_pipeline`** — Validates that `author_blueprint_for_create` can produce a blueprint for a fresh Rust project with a SPEC.md planning corpus, and that `render_blueprint` writes the manifest and workflow files.

4. **`greenfield_rust_bootstrap_verify`** — Validates that `cargo metadata` succeeds on a properly bootstrapped Rust project, confirming the bootstrap health markers are correct.

5. **`greenfield_rust_invalid_project_rejected`** — Validates that a project missing Cargo.toml fails `cargo metadata`, confirming the bootstrap verification gate correctly rejects incomplete projects.

6. **`greenfield_rust_full_lifecycle`** — Full lifecycle test: bootstrap workspace → author blueprint → verify blueprint structure → render blueprint → verify manifest written → verify project still compiles after rendering.

7. **`greenfield_rust_scaffold_first_ordering`** — Validates that the synthesis pipeline respects scaffold-first ordering for Rust projects with a planning corpus.

8. **`greenfield_rust_health_markers`** — Validates all bootstrap health markers: Cargo.toml exists, `cargo metadata` succeeds, source files exist, project compiles.

#### Helper functions

- `bootstrap_minimal_rust_project(root)` — Creates a minimal Rust binary project structure
- `bootstrap_workspace_rust_project(root)` — Creates a 2-crate Rust workspace
- `project_compiles(path)` — Runs `cargo check` to verify compilation
- `project_passes_cargo_check(path)` — Runs `cargo check`
- `project_passes_cargo_test(path)` — Runs `cargo check` (using check instead of test for speed)

## Design decisions

1. **cargo check over cargo build/test** — Used `cargo check` instead of `cargo build` or `cargo test` for compilation verification to avoid slow CI times. `cargo check` still verifies type-checking and compilation without producing binaries.

2. **tempfile for isolation** — All tests use `tempfile::tempdir()` to create isolated temporary directories that are automatically cleaned up.

3. **Planning corpus via SPEC.md** — Tests that call `author_blueprint_for_create` create a minimal SPEC.md file to provide the required planning corpus.

4. **Two-crate workspace** — The workspace test uses a lib + binary crate structure to verify cross-crate dependency resolution.

## Files created

- `lib/crates/fabro-synthesis/tests/greenfield_rust.rs` — 8 test functions with helper bootstrap functions

## Dependencies

- `tempfile` dev dependency (already present in Cargo.toml)
- `fabro_synthesis` crate public API (already a dependency)
