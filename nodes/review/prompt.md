Goal: Ci Preservation And Hardening

Child work item of plan: Test Coverage for Critical Paths

Required durable artifacts:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`


## Completed stages
- **preflight**: success
  - Script: `set +e
true
true`
  - Stdout: (empty)
  - Stderr: (empty)
- **contract**: success
  - Model: MiniMax-M2.7-highspeed, 14.7k tokens in / 249 out
- **implement**: success
  - Model: MiniMax-M2.7-highspeed, 142.4k tokens in / 622 out
  - Files: Cargo.lock, lib/crates/fabro-cli/tests/synth_regression.rs, lib/crates/fabro-db/Cargo.toml, lib/crates/fabro-db/src/lib.rs, lib/crates/fabro-db/tests/db_tests.rs, lib/crates/fabro-synthesis/tests/render_regression.rs, lib/crates/raspberry-supervisor/src/autodev.rs, lib/crates/raspberry-supervisor/src/dispatch.rs, 2 more repo file(s)
- **verify**: success
  - Script: `true`
  - Stdout: (empty)
  - Stderr: (empty)
- **quality**: success
  - Script: `set -e
QUALITY_PATH='outputs/test-coverage-critical-paths-ci-preservation-and-hardening/quality.md'
IMPLEMENTATION_PATH='outputs/test-coverage-critical-paths-ci-preservation-and-hardening/implementation.md'
VERIFICATION_PATH='outputs/test-coverage-critical-paths-ci-preservation-and-hardening/verification.md'
placeholder_hits=""
scan_placeholder() {
  surface="$1"
  if [ ! -e "$surface" ]; then
    return 0
  fi
  if [ -f "$surface" ]; then
    surface="$(dirname "$surface")"
  fi
  hits="$(rg -n -i -g '*.rs' -g '*.py' -g '*.js' -g '*.ts' -g '*.tsx' -g '*.md' -g 'Cargo.toml' -g '*.toml' 'TODO|stub|placeholder|not yet implemented|compile-only|for now|will implement|todo!|unimplemented!' "$surface" || true)"
  if [ -n "$hits" ]; then
    if [ -n "$placeholder_hits" ]; then
      placeholder_hits="$(printf '%s\n%s' "$placeholder_hits" "$hits")"
    else
      placeholder_hits="$hits"
    fi
  fi
}
true
external_blocker_only=no
root_artifact_hits=""
for shadow in spec.md review.md implementation.md verification.md quality.md promotion.md integration.md; do
  if [ -f "$shadow" ]; then
    root_artifact_hits="$root_artifact_hits\n$shadow"
  fi
done
semantic_risk_hits="$(rg -n -i -g '*.rs' 'payout_multiplier\(\)\s+as\s+i16|numerator\s+as\s+i16|deterministic placeholder|spin made without seed being set|house doesn.t play - the player spins|Generate seed \(in real impl, comes from house via action_seed\)' . 2>/dev/null || true)"
lane_sizing_hits=""
for surface in .; do
  if [ -d "$surface" ]; then
    while IFS= read -r file; do
      lines=$(wc -l < "$file" 2>/dev/null || echo 0)
      if [ "$lines" -lt 400 ]; then
        continue
      fi
      if rg -q 'handle_input' "$file" 2>/dev/null && rg -q 'render_' "$file" 2>/dev/null && rg -q 'tick\(|ui_state|session_pnl' "$file" 2>/dev/null; then
        lane_sizing_hits="$lane_sizing_hits\n$file:$lines"
      fi
    done < <(find "$surface" -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' \) 2>/dev/null)
  fi
done
artifact_hits="$(rg -n -i 'manual proof still required|placeholder|stub implementation|not yet fully implemented|todo!|unimplemented!' "$IMPLEMENTATION_PATH" "$VERIFICATION_PATH" 2>/dev/null || true)"
test_quality_debt=no
for surface in .; do
  if [ -d "$surface" ]; then
    total_tests=$(rg -c '#\[test\]' -g '*.rs' "$surface" 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
    derive_tests=$(rg -c 'assert.*\.to_string\(\).*contains\|assert_eq!.*\.to_string\(\)\|assert_eq!.*format!.*Display' -g '*.rs' "$surface" 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
    if [ "$total_tests" -gt 5 ] && [ "$derive_tests" -gt 0 ]; then
      ratio=$((derive_tests * 100 / total_tests))
      if [ "$ratio" -gt 50 ]; then
        test_quality_debt=yes
      fi
    fi
  fi
done
warning_hits="$(rg -n 'warning:' "$IMPLEMENTATION_PATH" "$VERIFICATION_PATH" 2>/dev/null || true)"
manual_hits="$(rg -n -i 'manual proof still required|manual;' "$VERIFICATION_PATH" 2>/dev/null || true)"
placeholder_debt=no
warning_debt=no
artifact_mismatch_risk=no
manual_followup_required=no
semantic_risk_debt=no
lane_sizing_debt=no
[ -n "$placeholder_hits" ] && placeholder_debt=yes
if [ "$external_blocker_only" = no ] && [ -n "$warning_hits" ]; then warning_debt=yes; fi
if [ -n "$artifact_hits" ] || [ -n "$root_artifact_hits" ]; then artifact_mismatch_risk=yes; fi
if [ "$external_blocker_only" = no ] && [ -n "$manual_hits" ]; then manual_followup_required=yes; fi
[ -n "$semantic_risk_hits" ] && semantic_risk_debt=yes
[ -n "$lane_sizing_hits" ] && lane_sizing_debt=yes
quality_ready=yes
if [ "$placeholder_debt" = yes ] || [ "$warning_debt" = yes ] || [ "$artifact_mismatch_risk" = yes ] || [ "$manual_followup_required" = yes ] || [ "$semantic_risk_debt" = yes ] || [ "$lane_sizing_debt" = yes ] || [ "$test_quality_debt" = yes ]; then
  quality_ready=no
fi
mkdir -p "$(dirname "$QUALITY_PATH")"
cat > "$QUALITY_PATH" <<EOF
quality_ready: $quality_ready
placeholder_debt: $placeholder_debt
warning_debt: $warning_debt
test_quality_debt: $test_quality_debt
artifact_mismatch_risk: $artifact_mismatch_risk
manual_followup_required: $manual_followup_required
semantic_risk_debt: $semantic_risk_debt
lane_sizing_debt: $lane_sizing_debt
external_blocker_only: $external_blocker_only

## Touched Surfaces
- (none declared)

## Placeholder Hits
$placeholder_hits

## Artifact Consistency Hits
$artifact_hits

## Root Artifact Shadow Hits
$root_artifact_hits

## Semantic Risk Hits
$semantic_risk_hits

## Lane Sizing Hits
$lane_sizing_hits

## Warning Hits
$warning_hits

## Manual Followup Hits
$manual_hits
EOF
test "$quality_ready" = yes

if [ -f .fabro-work/contract.md ]; then
  rm -f .fabro-work/.contract-missing
  sed -n '/^## Deliverables/,/^## /p' .fabro-work/contract.md | grep '^- ' | while IFS= read -r line; do
    cfile=$(echo "$line" | sed 's/^- //' | sed 's/`//g' | tr -d ' ')
    if [ -n "$cfile" ] && echo "$cfile" | grep -qE '\.(rs|ts|tsx|js|py|go|sol|rb|json|toml|yaml|yml)$'; then
      if [ ! -e "$cfile" ]; then
        echo "$cfile" >> .fabro-work/.contract-missing
      fi
    fi
  done
  if [ -f .fabro-work/.contract-missing ]; then
    echo '## Contract Deliverables Missing' >> "$QUALITY_PATH"
    cat .fabro-work/.contract-missing >> "$QUALITY_PATH"
    rm -f .fabro-work/.contract-missing
    quality_ready=no
  fi
fi`
  - Stdout: (empty)
  - Stderr: (empty)
- **fixup**: success
  - Model: MiniMax-M2.7-highspeed, 45.7k tokens in / 717 out
  - Files: lib/crates/fabro-synthesis/src/render.rs, outputs/test-coverage-critical-paths-ci-preservation-and-hardening/quality.md
- **verify**: success
  - Script: `true`
  - Stdout: (empty)
  - Stderr: (empty)
- **quality**: success
  - Script: `set -e
QUALITY_PATH='outputs/test-coverage-critical-paths-ci-preservation-and-hardening/quality.md'
IMPLEMENTATION_PATH='outputs/test-coverage-critical-paths-ci-preservation-and-hardening/implementation.md'
VERIFICATION_PATH='outputs/test-coverage-critical-paths-ci-preservation-and-hardening/verification.md'
placeholder_hits=""
scan_placeholder() {
  surface="$1"
  if [ ! -e "$surface" ]; then
    return 0
  fi
  if [ -f "$surface" ]; then
    surface="$(dirname "$surface")"
  fi
  hits="$(rg -n -i -g '*.rs' -g '*.py' -g '*.js' -g '*.ts' -g '*.tsx' -g '*.md' -g 'Cargo.toml' -g '*.toml' 'TODO|stub|placeholder|not yet implemented|compile-only|for now|will implement|todo!|unimplemented!' "$surface" || true)"
  if [ -n "$hits" ]; then
    if [ -n "$placeholder_hits" ]; then
      placeholder_hits="$(printf '%s\n%s' "$placeholder_hits" "$hits")"
    else
      placeholder_hits="$hits"
    fi
  fi
}
true
external_blocker_only=no
root_artifact_hits=""
for shadow in spec.md review.md implementation.md verification.md quality.md promotion.md integration.md; do
  if [ -f "$shadow" ]; then
    root_artifact_hits="$root_artifact_hits\n$shadow"
  fi
done
semantic_risk_hits="$(rg -n -i -g '*.rs' 'payout_multiplier\(\)\s+as\s+i16|numerator\s+as\s+i16|deterministic placeholder|spin made without seed being set|house doesn.t play - the player spins|Generate seed \(in real impl, comes from house via action_seed\)' . 2>/dev/null || true)"
lane_sizing_hits=""
for surface in .; do
  if [ -d "$surface" ]; then
    while IFS= read -r file; do
      lines=$(wc -l < "$file" 2>/dev/null || echo 0)
      if [ "$lines" -lt 400 ]; then
        continue
      fi
      if rg -q 'handle_input' "$file" 2>/dev/null && rg -q 'render_' "$file" 2>/dev/null && rg -q 'tick\(|ui_state|session_pnl' "$file" 2>/dev/null; then
        lane_sizing_hits="$lane_sizing_hits\n$file:$lines"
      fi
    done < <(find "$surface" -type f \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' \) 2>/dev/null)
  fi
done
artifact_hits="$(rg -n -i 'manual proof still required|placeholder|stub implementation|not yet fully implemented|todo!|unimplemented!' "$IMPLEMENTATION_PATH" "$VERIFICATION_PATH" 2>/dev/null || true)"
test_quality_debt=no
for surface in .; do
  if [ -d "$surface" ]; then
    total_tests=$(rg -c '#\[test\]' -g '*.rs' "$surface" 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
    derive_tests=$(rg -c 'assert.*\.to_string\(\).*contains\|assert_eq!.*\.to_string\(\)\|assert_eq!.*format!.*Display' -g '*.rs' "$surface" 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
    if [ "$total_tests" -gt 5 ] && [ "$derive_tests" -gt 0 ]; then
      ratio=$((derive_tests * 100 / total_tests))
      if [ "$ratio" -gt 50 ]; then
        test_quality_debt=yes
      fi
    fi
  fi
done
warning_hits="$(rg -n 'warning:' "$IMPLEMENTATION_PATH" "$VERIFICATION_PATH" 2>/dev/null || true)"
manual_hits="$(rg -n -i 'manual proof still required|manual;' "$VERIFICATION_PATH" 2>/dev/null || true)"
placeholder_debt=no
warning_debt=no
artifact_mismatch_risk=no
manual_followup_required=no
semantic_risk_debt=no
lane_sizing_debt=no
[ -n "$placeholder_hits" ] && placeholder_debt=yes
if [ "$external_blocker_only" = no ] && [ -n "$warning_hits" ]; then warning_debt=yes; fi
if [ -n "$artifact_hits" ] || [ -n "$root_artifact_hits" ]; then artifact_mismatch_risk=yes; fi
if [ "$external_blocker_only" = no ] && [ -n "$manual_hits" ]; then manual_followup_required=yes; fi
[ -n "$semantic_risk_hits" ] && semantic_risk_debt=yes
[ -n "$lane_sizing_hits" ] && lane_sizing_debt=yes
quality_ready=yes
if [ "$placeholder_debt" = yes ] || [ "$warning_debt" = yes ] || [ "$artifact_mismatch_risk" = yes ] || [ "$manual_followup_required" = yes ] || [ "$semantic_risk_debt" = yes ] || [ "$lane_sizing_debt" = yes ] || [ "$test_quality_debt" = yes ]; then
  quality_ready=no
fi
mkdir -p "$(dirname "$QUALITY_PATH")"
cat > "$QUALITY_PATH" <<EOF
quality_ready: $quality_ready
placeholder_debt: $placeholder_debt
warning_debt: $warning_debt
test_quality_debt: $test_quality_debt
artifact_mismatch_risk: $artifact_mismatch_risk
manual_followup_required: $manual_followup_required
semantic_risk_debt: $semantic_risk_debt
lane_sizing_debt: $lane_sizing_debt
external_blocker_only: $external_blocker_only

## Touched Surfaces
- (none declared)

## Placeholder Hits
$placeholder_hits

## Artifact Consistency Hits
$artifact_hits

## Root Artifact Shadow Hits
$root_artifact_hits

## Semantic Risk Hits
$semantic_risk_hits

## Lane Sizing Hits
$lane_sizing_hits

## Warning Hits
$warning_hits

## Manual Followup Hits
$manual_hits
EOF
test "$quality_ready" = yes

if [ -f .fabro-work/contract.md ]; then
  rm -f .fabro-work/.contract-missing
  sed -n '/^## Deliverables/,/^## /p' .fabro-work/contract.md | grep '^- ' | while IFS= read -r line; do
    cfile=$(echo "$line" | sed 's/^- //' | sed 's/`//g' | tr -d ' ')
    if [ -n "$cfile" ] && echo "$cfile" | grep -qE '\.(rs|ts|tsx|js|py|go|sol|rb|json|toml|yaml|yml)$'; then
      if [ ! -e "$cfile" ]; then
        echo "$cfile" >> .fabro-work/.contract-missing
      fi
    fi
  done
  if [ -f .fabro-work/.contract-missing ]; then
    echo '## Contract Deliverables Missing' >> "$QUALITY_PATH"
    cat .fabro-work/.contract-missing >> "$QUALITY_PATH"
    rm -f .fabro-work/.contract-missing
    quality_ready=no
  fi
fi`
  - Stdout: (empty)
  - Stderr: (empty)
- **challenge**: success
  - Model: MiniMax-M2.7-highspeed, 56.6k tokens in / 941 out


# Ci Preservation And Hardening Lane — Review

Review only the current slice for `test-coverage-critical-paths-ci-preservation-and-hardening`.

Current Slice Contract:
Plan file:
- `genesis/plans/005-test-coverage-critical-paths.md`

Child work item: `test-coverage-critical-paths-ci-preservation-and-hardening`

Full plan context (read this for domain knowledge, design decisions, and specifications):

# Test Coverage for Critical Paths

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [genesis/PLANS.md](/home/r/coding/fabro/genesis/PLANS.md).

## Purpose / Big Picture

After this change, every crate in the autodev critical path has meaningful test coverage for the failure modes we now know are real: generated package/runtime mismatches, detached-run validation failures, stale frontier truth, and workspace-level regressions. The two crates with zero tests (`fabro-db`, `fabro-types`) get baseline coverage, and synthesis/autodev regressions are pinned down with targeted tests before they reappear overnight.

The proof is: introduce a deliberate regression (e.g., break a SQL migration in fabro-db), push to a branch, and watch CI fail with a specific test name.

## Progress

- [ ] Add tests to fabro-db (schema migration, WAL mode, basic CRUD)
- [ ] Add edge case tests to raspberry-supervisor (stale state, race conditions)
- [ ] Add integration tests for autodev dispatch cycle and detached-run validation
- [ ] Preserve and extend CI coverage for synthesis/autodev regressions
- [ ] Add fabro-mcp and fabro-github minimal test coverage

## Surprises & Discoveries

(To be updated)

## Decision Log

- Decision: Focus test additions on failure modes, not happy paths.
  Rationale: The highest-value new tests are for the failures that actually stopped proving-ground runs: stale state, malformed runtime paths, detached-run validation failures, and command-surface mismatches. Those are more urgent than broad happy-path expansion.
  Date/Author: 2026-03-26 / Genesis

- Decision: Do not pursue code coverage percentages as a target.
  Rationale: 80% coverage on fabro-workflows (52K LOC) would require thousands of tests for marginal benefit. Instead, target specific failure modes identified in the assessment.
  Date/Author: 2026-03-26 / Genesis

- Failure scenario: New tests may be flaky if they depend on file I/O timing or network state. All new tests must be deterministic — use in-memory SQLite for db tests, fixture files for state tests, no network calls.
  Date/Author: 2026-03-26 / Genesis

## Outcomes & Retrospective

(To be filled)

## Context and Orientation

Current test landscape:

| Crate | Tests | LOC | Gap |
|-------|-------|-----|-----|
| `fabro-db` | 0 | ~1,500 | **Zero coverage** — SQLite, WAL, migrations |
| `fabro-types` | 0 | ~5,000 | Auto-generated from OpenAPI, low risk |
| `fabro-mcp` | 7 | ~1,800 | MCP protocol client barely tested |
| `fabro-github` | 4 | ~1,200 | JWT signing, installation tokens barely tested |
| `raspberry-supervisor` | 114 | 15,049 | Missing: stale running state, dispatch races |
| `fabro-synthesis` | 88 | 15,472 | Missing: edge cases in render.rs |

CI config lives in `.github/workflows/rust.yml`. The repo already has fmt, clippy, and nextest checks. The real gap is that critical synthesis/autodev regressions are not yet captured by focused tests, so CI can stay green while proving-ground runs still fail.

## Milestones

### Milestone 1: fabro-db baseline tests

Add tests for:
- Database creation with WAL mode
- Schema migration (apply all migrations, verify tables exist)
- Basic CRUD operations (insert, query, update, delete)
- Concurrent read during write (WAL mode correctness)
- Corrupt/missing database file handling

All tests must use in-memory SQLite (`:memory:`) or temp files.

Key file: `lib/crates/fabro-db/src/lib.rs` (or `lib/crates/fabro-db/src/`)

Proof command:

    cargo nextest run -p fabro-db

Expected: 5+ new tests, all passing.

### Milestone 2: raspberry-supervisor edge case tests

Add tests for:
- Stale `running` lane detection and reconciliation
- Dispatch with max_parallel budget exhaustion
- Recovery action authority (persisted vs recomputed)
- Cycle limit termination behavior
- Frontier budget accounting after failures
- Program state with malformed JSON files

Key files:
- `lib/crates/raspberry-supervisor/src/program_state.rs`
- `lib/crates/raspberry-supervisor/src/dispatch.rs`
- `lib/crates/raspberry-supervisor/src/autodev.rs`

Proof command:

    cargo nextest run -p raspberry-supervisor -- stale dispatch recovery cycle frontier malformed

### Milestone 3: Autodev integration test

Add a fixture-based integration test that simulates a complete autodev cycle: load a fixture manifest, evaluate, dispatch (mocked), observe state change, and verify detached-run bootstrap diagnostics surface the real cause when validation fails.

Key file: `lib/crates/raspberry-supervisor/tests/` (new integration test file)

Proof command:

    cargo nextest run -p raspberry-supervisor -- integration autodev_cycle

### Milestone 4: Synthesis/runtime regression tests

Add targeted regression tests for the failures observed during live restart work:
- generated workflows depending on a `fabro` binary that does not expose required subcommands
- copied run graphs failing validation because prompt refs resolve under the wrong root
- detached runs collapsing to generic `Validation failed` without actionable diagnostics

Key files:
- `lib/crates/fabro-cli/src/main.rs`
- `lib/crates/fabro-cli/src/commands/synth.rs`
- `lib/crates/fabro-synthesis/src/render.rs`
- `lib/crates/fabro-workflows/src/`

Proof command:

    cargo nextest run -p fabro-cli -- synth
    cargo nextest run -p fabro-synthesis -- render

### Milestone 5: CI preservation and hardening

Update `.github/workflows/rust.yml` only where needed to make sure the new synthesis/autodev regression tests run in CI and fail loudly. Preserve the existing fmt/clippy/nextest checks rather than "adding clippy" from scratch.

Proof command:

    cargo clippy --workspace -- -D warnings && \
    cargo fmt --check --all && \
    cargo nextest run --workspace

### Milestone 6: Minimal coverage for fabro-mcp and fabro-github

Add 3-5 tests each for:
- `fabro-mcp`: message serialization, tool call parsing, protocol handshake
- `fabro-github`: JWT generation, installation token request structure, PR creation payload

Proof command:

    cargo nextest run -p fabro-mcp && cargo nextest run -p fabro-github

## Validation and Acceptance

The plan is done when:
- `fabro-db` has >5 tests covering schema and CRUD
- `raspberry-supervisor` has edge case tests for stale state and dispatch races
- An autodev integration test exists and passes
- synthesis/runtime regressions are covered by tests that fail before proving-ground autodev does
- A deliberate regression in fabro-db is caught by CI


Workflow archetype: implement

Review profile: standard

Active plan:
- `genesis/plans/001-master-plan.md`

Active spec:
- `genesis/SPEC.md`

Artifacts to write:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Nemesis-style security review
- Pass 1 — first-principles challenge: question trust boundaries, authority assumptions, and who can trigger the slice's dangerous actions
- Pass 2 — coupled-state review: identify paired state or protocol surfaces and check that every mutation path keeps them consistent or explains the asymmetry
- check state transitions that affect balances, commitments, randomness, payout safety, or replayability
- check secret handling, capability scoping, pairing/idempotence behavior, and privilege escalation paths

Focus on:
- slice scope discipline
- proof-gate coverage for the active slice
- touched-surface containment
- implementation and verification artifact quality
- remaining blockers before the next slice


Structural discipline
- if a new source file would exceed roughly 400 lines, split it before landing
- do not mix state transitions, input handling, rendering, and animation in one new file unless the prompt explicitly justifies that coupling
- if the slice cannot stay small, stop and update the artifacts to explain the next decomposition boundary instead of silently landing a monolith
Deterministic evidence:
- treat `.fabro-work/quality.md` as machine-generated truth about placeholder debt, warning debt, manual follow-up, and artifact mismatch risk
- if `.fabro-work/quality.md` says `quality_ready: no`, do not bless the slice as merge-ready


Score each dimension 0-10 and write `.fabro-work/promotion.md` in this exact form:

merge_ready: yes|no
manual_proof_pending: yes|no
completeness: <0-10>
correctness: <0-10>
convention: <0-10>
test_quality: <0-10>
reason: <one sentence>
next_action: <one sentence>

Scoring guide:
- completeness: 10=all deliverables present + all acceptance criteria met, 7=core present + 1-2 gaps, 4=missing deliverables, 0=skeleton
- correctness: 10=compiles + tests pass + edges handled, 7=tests pass + minor gaps, 4=some failures, 0=broken
- convention: 10=matches all project patterns, 7=minor deviations, 4=multiple violations, 0=ignores conventions
- test_quality: 10=tests import subject + verify all criteria, 7=most criteria tested, 4=structural only, 0=no tests

If `.fabro-work/contract.md` exists, verify EVERY acceptance criterion from it.
Any dimension below 6 = merge_ready: no.
If `.fabro-work/quality.md` says quality_ready: no = merge_ready: no.

For security-sensitive slices, append these mandatory fields exactly:
- layout_invariants_complete: yes|no
- slice_decomposition_respected: yes|no
If any mandatory security field is `no`, set `merge_ready: no`.

Review stage ownership:
- you may write or replace `.fabro-work/promotion.md` in this stage
- read `.fabro-work/quality.md` before deciding `merge_ready`
- when the slice is security-sensitive, perform a Nemesis-style pass: first-principles assumption challenge plus coupled-state consistency review
- include security findings in the review verdict when the slice touches trust boundaries, keys, funds, auth, control-plane behavior, or external process control
- prefer not to modify source code here unless a tiny correction is required to make the review judgment truthful
