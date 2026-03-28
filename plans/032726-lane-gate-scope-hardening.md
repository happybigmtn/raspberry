# Lane Gate Scope Hardening Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Stop synthesized implementation lanes from failing quality and audit on repo-wide noise instead of lane-owned work.

**Architecture:** The fix stays inside `fabro-synthesis` render-time command generation. We will narrow generated `quality` scripts to owned surfaces and lane-local artifacts, and we will adjust generated `audit` scripts so root artifact markdown does not count as a surface violation while unrelated code still does.

**Tech Stack:** Rust, `fabro-synthesis`, generated shell gates, Rust unit tests.

---

### Task 1: Scope `quality` to owned surfaces

**Files:**
- Modify: `lib/crates/fabro-synthesis/src/render.rs`
- Test: `lib/crates/fabro-synthesis/src/render.rs`

**Steps:**
1. Change placeholder scanning so file surfaces are scanned as files, not widened to their parent directories.
2. Change layout sizing and test-quality scans so file-owned lanes inspect only declared files, while directory-owned lanes still recurse within declared directories.
3. Remove repo-root shadow artifact scanning from the generated `quality` command.
4. Narrow artifact text scanning so ordinary mentions of “placeholder” in lane docs do not trigger `artifact_mismatch_risk`.

### Task 2: Keep `audit` strict on code, lenient on root markdown artifacts

**Files:**
- Modify: `lib/crates/fabro-synthesis/src/render.rs`
- Test: `lib/crates/fabro-synthesis/src/render.rs`

**Steps:**
1. Update the generated audit surface guard to continue rejecting unrelated code and config files.
2. Allow known root artifact markdown filenames (`spec.md`, `review.md`, `implementation.md`, `verification.md`, `quality.md`, `promotion.md`, `integration.md`) so artifact spillover does not force a fixup cycle by itself.
3. Preserve lane-local artifact checks and `quality_ready: yes` gating.

### Task 3: Add regression coverage and verify

**Files:**
- Modify: `lib/crates/fabro-synthesis/src/render.rs`

**Steps:**
1. Add or update renderer unit tests that prove the new `quality` command does not widen file-owned surfaces to sibling directories.
2. Add or update renderer unit tests that prove root shadow artifact scanning is gone.
3. Add or update renderer unit tests that prove the audit command tolerates root artifact markdown but still enforces owned code surfaces.
4. Run targeted `cargo test` coverage for `fabro-synthesis`.
