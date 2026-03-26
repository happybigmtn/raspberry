# Integration: CI Preservation And Hardening

## Crate Integration Points

### fabro-db Integration

The fabro-db crate is used by other crates for SQLite persistence. The new tests verify:

1. **Connection patterns** - Both `connect()` for file-based and `connect_memory()` for in-memory work correctly
2. **WAL mode** - WAL journal mode is enabled and persists across connections
3. **Migration system** - `initialize_db()` correctly applies migrations and tracks version via `PRAGMA user_version`
4. **Schema integrity** - All tables (workflow_runs) are created with correct columns (run_dir, not logs_dir)

### raspberry-supervisor Integration

The raspberry-supervisor crate orchestrates program execution and depends on:

1. **Program evaluation** - `evaluate_program()` uses manifest parsing and state tracking
2. **Dispatch** - `execute_selected_lanes()` coordinates lane execution
3. **Autodev cycles** - `orchestrate_program()` runs iterative evaluation-dispatch cycles
4. **State persistence** - `ProgramRuntimeState` tracks lane statuses across cycles

New tests verify:
- Evaluation produces correct lane statuses from manifests
- Dispatch updates program state correctly
- Autodev report is saved after orchestration
- Cycle limits are respected
- Stale running states are detected

### fabro-synthesis Integration

The fabro-synthesis crate generates workflow packages from blueprints:

1. **Blueprint loading** - `load_blueprint()` parses YAML into structured types
2. **Rendering** - `render_blueprint()` generates program manifests and workflows
3. **Reconciliation** - `reconcile_blueprint()` updates existing packages

New tests verify:
- Render produces valid run config paths
- Workflow files are generated
- Directories are created correctly
- Special characters in program names are handled

### fabro-cli Integration

The fabro-cli crate provides the `fabro synth` command surface:

1. **create** - Creates checked-in workflow packages from blueprints
2. **evolve** - Steers active programs from genesis + evidence
3. **import** - Imports existing packages into blueprints

New tests verify:
- Command help is accessible
- Required arguments are validated
- Invalid blueprints produce graceful errors
- Directory structures are created correctly
- Path handling works with spaces

## Cross-Crate Dependencies

```
fabro-cli
  └── fabro-synthesis (for synth commands)

raspberry-supervisor
  └── fabro-db (for workflow run persistence, if used)
  └── fabro-workflows (for run inspection)
```

## Test Fixtures

### raspberry-supervisor fixtures
- `test/fixtures/raspberry-supervisor/program.yaml` - Multi-unit program with consensus/runtime/p2p
- `test/fixtures/raspberry-supervisor/portfolio-program.yaml` - Portfolio with child programs
- `test/fixtures/raspberry-supervisor/run-configs/` - Sample run configs

### fabro-synthesis fixtures
- `test/fixtures/program-synthesis/craps/blueprint.yaml` - Bootstrap blueprint
- `test/fixtures/program-synthesis/update-myosu/` - Evolve test fixtures

### fabro-cli fixtures
- Uses same fabro-synthesis fixtures for integration testing

## CI Pipeline Integration

The CI workflow `.github/workflows/rust.yml` runs:

1. **Format check** - `cargo fmt --check --all`
2. **Lint** - `cargo clippy --workspace -- -D warnings`
3. **Test** - `cargo nextest run --workspace` (or `cargo test --workspace`)

All new tests are automatically included via path filter `lib/crates/**`.

## Verification Checklist

- [x] fabro-db tests pass (28 tests)
- [x] raspberry-supervisor integration tests pass (6 tests)
- [x] fabro-synthesis tests pass (12 tests)
- [x] fabro-cli tests pass (18 tests)
- [x] Format check passes
- [x] Clippy passes (pre-existing warning in evaluate.rs)
- [x] CI path filters cover all new test files
