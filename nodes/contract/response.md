Contract written to `.fabro-work/contract.md`. Here's the summary:

## Contract Summary

**Goal:** Write `bootstrap_verify` — a nextest test that proves `render_blueprint` produces the correct package structure for a greenfield bootstrap lane, including all five durable artifacts.

**What changes:**

1. **`Cargo.toml`** — add `fabro-workflows` dev-dependency (needed for `write_text_atomic`)
2. **`blueprint.rs`** — add `BOOTSTRAP_REQUIRED_ARTIFACTS` constant, a unit accessor, and a lane helper (`is_bootstrap()`)
3. **`render.rs`** — extend lane rendering to write the five `.md` placeholder files
4. **`tests/bootstrap_verify.rs`** — new test that renders a minimal blueprint with one bootstrap lane and asserts all outputs exist
5. **`tests/fixtures/bootstrap-verify/blueprint.yaml`** — minimal fixture

**What does NOT change:** No workflow execution, no planning/authoring, no reconcile logic, no multi-unit complexity.