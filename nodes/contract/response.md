## Contract Written

Created `.fabro-work/contract.md` defining the DONE criteria for the Autodev Integration Test lane.

### Key Points

**Preflight Note:** The preflight failed because `cargo-nextest` is not installed. The contract uses `cargo test` instead of `cargo nextest` (they use the same test filter syntax).

**Deliverables:**
- 9 files to create/modify tests in
- `fabro-db` gets its first tests (zero currently)
- `raspberry-supervisor` gets `integration_tests.rs` plus module-level tests
- `fabro-mcp`, `fabro-github`, `fabro-synthesis`, `fabro-cli` each get minimal test coverage

**Acceptance Criteria (13 total):**
1. All `fabro-db` tests pass
2. `autodev_cycle` integration test passes
3-8. Six filter-based test groups for edge cases: `stale`, `dispatch`, `recovery`, `cycle`, `frontier`, `malformed`
9-10. MCP and GitHub tests pass
11-12. Synthesis render and CLI synth tests pass
13. Full workspace test pass

**Out of Scope:**
- Code coverage percentages
- Benchmarks
- Network tests
- CI config changes
- `fabro-types` tests (auto-generated)