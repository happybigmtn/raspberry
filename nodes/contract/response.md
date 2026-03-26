Contract written to `.fabro-work/contract.md`. The contract defines:

**Deliverables:**
- CI workflow verification and updates if needed
- Test crates verified running: fabro-db, raspberry-supervisor, fabro-mcp, fabro-github

**Acceptance Criteria (8 verifiable conditions):**
1. `cargo fmt --check --all` passes
2. `cargo clippy --workspace -- -D warnings` passes
3. `cargo nextest run -p fabro-db` shows ≥5 tests passing
4. `cargo nextest run -p raspberry-supervisor` passes with edge case tests
5. `cargo nextest run -p fabro-mcp` shows ≥10 tests passing
6. `cargo nextest run -p fabro-github` shows ≥7 tests passing
7. CI workflow runs `cargo nextest run --workspace` covering all tests
8. No `--exclude` flags in CI test job

**Out of Scope:**
- Adding new tests (other lanes)
- Test infrastructure changes
- CI matrix expansion
- Workspace configuration changes