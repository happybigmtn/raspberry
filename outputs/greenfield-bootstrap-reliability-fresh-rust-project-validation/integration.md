# Integration: Fresh Rust Project Validation

## Integration points

### Into fabro-synthesis crate

The test file `lib/crates/fabro-synthesis/tests/greenfield_rust.rs` integrates with:

- **`fabro_synthesis::author_blueprint_for_create`** — validates blueprint authoring for greenfield projects
- **`fabro_synthesis::render_blueprint`** — validates blueprint rendering and manifest generation
- **`tempfile::tempdir()`** — dev dependency for isolated temporary directories

### Dependencies

- `tempfile` dev dependency (already declared in `fabro-synthesis/Cargo.toml`)
- Standard library: `std::fs`, `std::path::Path`, `std::process::Command`

## How the tests validate the synthesis pipeline

### Bootstrap verification

Tests verify that `cargo metadata` correctly identifies a healthy Rust project:
- `greenfield_rust_bootstrap_verify` — confirms metadata succeeds
- `greenfield_rust_invalid_project_rejected` — confirms metadata fails without Cargo.toml

### Blueprint authoring

`greenfield_rust_synthesis_pipeline` and `greenfield_rust_full_lifecycle` validate:
- `author_blueprint_for_create` produces a valid blueprint for fresh projects
- Blueprint has correct program ID
- Blueprint contains at least one unit with lanes
- Rendering writes the manifest file to `malinka/programs/`

### Full lifecycle

`greenfield_rust_full_lifecycle` validates the complete pipeline:
1. Bootstrap project structure
2. Author blueprint
3. Render blueprint
4. Verify output files exist
5. Verify project still compiles after rendering (no corruption)

## Relationship to other lanes

This lane is part of the "Greenfield Bootstrap and Runtime Asset Reliability" plan (genesis/plans/004-greenfield-bootstrap-reliability.md).

### Milestone 6 alignment

This lane implements Milestone 6: "Fresh Rust project validation"

> Create a minimal test fixture Rust project (Cargo workspace with 2 crates). Run `fabro synth genesis` → `raspberry autodev` and confirm scaffold-first ordering works.
>
> Proof command: `cargo nextest run -p fabro-synthesis -- greenfield_rust`

The tests validate that:
- Fresh Rust projects can be bootstrapped
- The synthesis pipeline handles greenfield projects
- Scaffold-first ordering is respected

## Backward compatibility

These tests do not modify existing behavior — they add new test coverage for an existing pipeline. No breaking changes.
