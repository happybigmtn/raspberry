Goal: Use Codex to unblock implementation lane `test-coverage-critical-paths-ci-preservation-and-hardening:test-coverage-critical-paths-ci-preservation-and-hardening`.

Inspect the source lane's most recent failure/remediation context and apply the minimal code or harness changes needed so the source lane can pass its next replay.

Proof commands:
- `cargo check --workspace`


## Completed stages
- **preflight**: success
  - Script: `set +e
cargo check --workspace
true`
  - Stdout: (empty)
  - Stderr:
    ```
    (5 lines omitted)
        Checking fabro-llm v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-llm)
        Checking fabro-mcp v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-mcp)
        Checking fabro-sandbox v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-sandbox)
        Checking fabro-validate v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-validate)
        Checking fabro-devcontainer v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-devcontainer)
        Checking fabro-interview v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-interview)
        Checking fabro-git-storage v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-git-storage)
       Compiling fabro-types v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-types)
        Checking fabro-agent v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-agent)
       Compiling fabro-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-cli)
       Compiling raspberry-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/raspberry-cli)
        Checking fabro-telemetry v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-telemetry)
        Checking fabro-hooks v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-hooks)
        Checking fabro-retro v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-retro)
        Checking fabro-workflows v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-workflows)
        Checking fabro-db v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-db)
        Checking fabro-openai-oauth v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-openai-oauth)
        Checking fabro-tracker v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-tracker)
        Checking fabro-beastie v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-beastie)
        Checking raspberry-supervisor v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/raspberry-supervisor)
        Checking fabro-api v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-api)
        Checking fabro-slack v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-slack)
        Checking raspberry-tui v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/raspberry-tui)
        Checking fabro-synthesis v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-synthesis)
        Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.25s
    ```
- **contract**: success
  - Model: gpt-5.4, 897.7k tokens in / 9.4k out
- **implement**: success
  - Model: gpt-5.4, 5.0m tokens in / 15.6k out
  - Files: malinka/workflows/implementation/test-coverage-critical-paths-ci-preservation-and-hardening.fabro
- **verify**: success
  - Script: `cargo check --workspace`
  - Stdout: (empty)
  - Stderr:
    ```
    Blocking waiting for file lock on build directory
       Compiling fabro-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-cli)
       Compiling raspberry-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/raspberry-cli)
        Finished `dev` profile [unoptimized + debuginfo] target(s) in 12.46s
    ```
- **quality**: success
  - Script: `set -e
QUALITY_PATH='.raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/quality.md'
IMPLEMENTATION_PATH='.raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/implementation.md'
VERIFICATION_PATH='.raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/verification.md'
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
if rg -q -i 'inside lane-owned surface: no remaining blocker found|outside lane-owned surface: yes|outside the lane-owned surface: yes' .fabro-work/deep-review-findings.md 2>/dev/null; then
  external_blocker_only=yes
fi
root_artifact_hits=""
for shadow in spec.md review.md implementation.md verification.md quality.md promotion.md integration.md; do
  if [ -f "$shadow" ]; then
    root_artifact_hits="$root_artifact_hits\n$shadow"
  fi
done
semantic_risk_hits=""
lane_sizing_hits=""
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
  - Model: gpt-5.4, 11.5m tokens in / 24.3k out
  - Files: lib/crates/fabro-cli/src/commands/synth.rs, lib/crates/fabro-cli/tests/synth.rs, lib/crates/fabro-model/src/catalog.rs, lib/crates/fabro-synthesis/src/planning.rs, lib/crates/fabro-synthesis/src/render.rs, lib/crates/raspberry-supervisor/src/evaluate.rs, lib/crates/raspberry-tui/src/app.rs
- **deep_review**: success
  - Model: gpt-5.4, 592.1k tokens in / 5.8k out
- **recheck**: success
  - Script: `cargo check --workspace`
  - Stdout: (empty)
  - Stderr:
    ```
    Blocking waiting for file lock on build directory
       Compiling fabro-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-cli)
       Compiling raspberry-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/raspberry-cli)
        Finished `dev` profile [unoptimized + debuginfo] target(s) in 41.96s
    ```


# Ci Preservation And Hardening Lane Codex Unblock — Review

Review only the current slice for `test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock`.

Current Slice Contract:
Target blocked lane: `test-coverage-critical-paths-ci-preservation-and-hardening:test-coverage-critical-paths-ci-preservation-and-hardening`.
Recovery objective: unblock the source lane so it can be replayed successfully.
This lane is dispatched only after the source lane is marked `surface_blocked`.
Focus on minimal, high-confidence changes that remove the blocker.
Read the target lane's latest artifacts and remediation notes before editing.
If the owned proof gate is already green and the only remaining blocker is outside the owned surface, do not invent more code changes. Write the unblock artifacts truthfully, explain the external blocker, and stop.
Keep the scope narrow: fix the blocker, verify, integrate, and stop.
This lane is distinct from the parent holistic deep/adjudication review path.


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Focus on:
- slice scope discipline
- proof-gate coverage for the active slice
- touched-surface containment
- implementation and verification artifact quality
- remaining blockers before the next slice

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

Review stage ownership:
- you may write or replace `.fabro-work/promotion.md` in this stage
- read `.fabro-work/quality.md` before deciding `merge_ready`
- when the slice is security-sensitive, perform a Nemesis-style pass: first-principles assumption challenge plus coupled-state consistency review
- include security findings in the review verdict when the slice touches trust boundaries, keys, funds, auth, control-plane behavior, or external process control
- prefer not to modify source code here unless a tiny correction is required to make the review judgment truthful
