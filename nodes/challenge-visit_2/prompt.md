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
    Compiling raspberry-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/raspberry-cli)
       Compiling fabro-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-cli)
        Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.78s
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
- **review**: success
  - Model: gpt-5.4, 8.9m tokens in / 26.7k out
  - Files: .config/nextest.toml, lib/crates/fabro-cli/src/commands/run.rs, lib/crates/fabro-cli/tests/cmd/model/bare.trycmd, lib/crates/fabro-cli/tests/cmd/model/list.trycmd
- **audit**: fail
  - Script: `capture_remediation() {
  mkdir -p '.raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock'
  {
    echo '# Remediation Notes (auto-captured from failed audit)'
    echo ''
    echo '## Quality Gate'
    cat .fabro-work/quality.md 2>/dev/null || echo '(not found)'
    echo ''
    echo '## Verification Findings'
    cat .fabro-work/verification.md 2>/dev/null || echo '(not found)'
    echo ''
    echo '## Deep Review Findings'
    cat .fabro-work/deep-review-findings.md 2>/dev/null || echo '(not found)'
    echo ''
    echo '## Promotion Decision'
    cat .fabro-work/promotion.md 2>/dev/null || echo '(not found)'
  } > '.raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/remediation.md'
}
if ! ( test -f '.raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/spec.md' && test -f '.raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/review.md' && test -f '.raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/verification.md' && test -f '.raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/quality.md' && test -f '.raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/promotion.md' && grep -Eq '^merge_ready: yes$' .raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/promotion.md && grep -Eq '^manual_proof_pending: no$' .raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/promotion.md && grep -Eq '^reason: .+$' .raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/promotion.md && grep -Eq '^next_action: .+$' .raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/promotion.md && grep -Eq '^completeness: ([6-9]|10)$' .raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/promotion.md && grep -Eq '^correctness: ([6-9]|10)$' .raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/promotion.md && grep -Eq '^convention: ([6-9]|10)$' .raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/promotion.md && grep -Eq '^test_quality: ([6-9]|10)$' .raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/promotion.md && ( _mb=$(git merge-base HEAD origin/main 2>/dev/null || echo origin/main); changed_count=$(git diff --name-only "$_mb"..HEAD -- '*.rs' '*.toml' '*.py' '*.js' '*.ts' '*.tsx' '*.go' '*.java' '*.rb' '*.yaml' '*.yml' '*.json' '*.sol' '*.sh' | wc -l); test "$changed_count" -gt 0 || rg -q -i 'no code changes were needed|outside lane-owned surface: yes|outside the lane-owned surface: yes|owned proof gate is already green|proof gate was already green' '.raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/review.md' '.raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/verification.md' .fabro-work/deep-review-findings.md 2>/dev/null ) && grep -Eq '^quality_ready: yes$' '.raspberry/portfolio/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/quality.md' ); then
  capture_remediation
  exit 1
fi`
  - Stdout: (empty)
  - Stderr: (empty)
- **fixup**: success
  - Model: gpt-5.4, 2.3m tokens in / 22.7k out
  - Files: malinka/prompts/implementation/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock/review.md, malinka/workflows/implementation/test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock.fabro
- **verify**: success
  - Script: `cargo check --workspace`
  - Stdout: (empty)
  - Stderr:
    ```
    Compiling raspberry-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/raspberry-cli)
       Compiling fabro-cli v0.176.2 (/home/r/.fabro/runs/20260326-01KMNT43GY5T3AG67J2TSNF09H/worktree/lib/crates/fabro-cli)
        Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.78s
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


# Ci Preservation And Hardening Lane Codex Unblock — Challenge

Perform a cheap adversarial review of the current slice for `test-coverage-critical-paths-ci-preservation-and-hardening-codex-unblock` before the expensive final review runs.

Your job is to challenge assumptions, find obvious scope drift, identify weak proof, and catch mismatches between code and artifacts. Do not bless the slice as merge-ready; that belongs to the final review gate.


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Challenge checklist:
- Is the slice smaller than the plan says, or larger?
- Did the implementation actually satisfy the first proof gate?
- Are any touched surfaces outside the named slice?
- Are the artifacts overstating completion?
- Are the tests actually verifying behavioral outcomes, or are they trivial stubs that pass without real logic?
- Is there an obvious bug, trust-boundary issue, or missing test the final reviewer should not have to rediscover?

Write a short challenge note in `.fabro-work/verification.md` or amend it if needed, focusing on concrete gaps and the next fixup target. Do not write `promotion.md` here.
