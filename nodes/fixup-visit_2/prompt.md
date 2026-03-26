Goal: Bootstrap Verification Gate

Child work item of plan: Greenfield Bootstrap and Runtime Asset Reliability

Proof commands:
- `cargo nextest run -p fabro-synthesis -- bootstrap_verify`

Required durable artifacts:
- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`
- `integration.md`


## Completed stages
- **preflight**: success
  - Script: `set +e
if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p fabro-synthesis -- bootstrap_verify
else
  cargo test -p fabro-synthesis -- bootstrap_verify
fi
true`
  - Stdout: (empty)
  - Stderr:
    ```
    (8 lines omitted)
       Compiling fabro-model v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-model)
       Compiling fabro-github v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-github)
       Compiling fabro-graphviz v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-graphviz)
       Compiling fabro-git-storage v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-git-storage)
       Compiling fabro-validate v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-validate)
       Compiling fabro-config v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-config)
       Compiling fabro-llm v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-llm)
       Compiling fabro-devcontainer v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-devcontainer)
       Compiling fabro-interview v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-interview)
       Compiling fabro-mcp v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-mcp)
       Compiling fabro-sandbox v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-sandbox)
       Compiling fabro-agent v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-agent)
       Compiling fabro-retro v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-retro)
       Compiling fabro-hooks v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-hooks)
       Compiling fabro-workflows v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-workflows)
       Compiling raspberry-supervisor v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/raspberry-supervisor)
       Compiling fabro-synthesis v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-synthesis)
        Finished `test` profile [unoptimized + debuginfo] target(s) in 52.96s
    ────────────
     Nextest run ID 3dc1a1de-0603-4259-8aee-0ce1ba9280d6 with nextest profile: default
        Starting 0 tests across 2 binaries (95 tests skipped)
    ────────────
         Summary [   0.000s] 0 tests run: 0 passed, 95 skipped
    error: no tests to run
    (hint: use `--no-tests` to customize)
    ```
- **contract**: success
  - Model: MiniMax-M2.7-highspeed, 40.1k tokens in / 237 out
- **implement**: success
  - Model: MiniMax-M2.7-highspeed, 36.4k tokens in / 518 out
  - Files: lib/crates/fabro-synthesis/tests/bootstrap.rs
- **verify**: success
  - Script: `if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p fabro-synthesis -- bootstrap_verify
else
  cargo test -p fabro-synthesis -- bootstrap_verify
fi`
  - Stdout: (empty)
  - Stderr:
    ```
    Compiling fabro-synthesis v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-synthesis)
        Finished `test` profile [unoptimized + debuginfo] target(s) in 1.77s
    ────────────
     Nextest run ID 67df466a-c0c7-4bb4-ba87-2b0411c8ec11 with nextest profile: default
        Starting 1 test across 3 binaries (95 tests skipped)
            PASS [   0.004s] (1/1) fabro-synthesis::bootstrap bootstrap_verify
    ────────────
         Summary [   0.004s] 1 test run: 1 passed, 95 skipped
    ```
- **quality**: fail
  - Script: `set -e
QUALITY_PATH='outputs/greenfield-bootstrap-reliability-bootstrap-verification-gate/quality.md'
IMPLEMENTATION_PATH='outputs/greenfield-bootstrap-reliability-bootstrap-verification-gate/implementation.md'
VERIFICATION_PATH='outputs/greenfield-bootstrap-reliability-bootstrap-verification-gate/verification.md'
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
  - Model: MiniMax-M2.7-highspeed, 50.0k tokens in / 707 out
  - Files: outputs/greenfield-bootstrap-reliability-bootstrap-verification-gate/implementation.md, outputs/greenfield-bootstrap-reliability-bootstrap-verification-gate/verification.md
- **verify**: success
  - Script: `if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p fabro-synthesis -- bootstrap_verify
else
  cargo test -p fabro-synthesis -- bootstrap_verify
fi`
  - Stdout: (empty)
  - Stderr:
    ```
    Compiling fabro-synthesis v0.176.2 (/home/r/.fabro/runs/20260326-01KMP4JS1ZEC6912S61SSB6804/worktree/lib/crates/fabro-synthesis)
        Finished `test` profile [unoptimized + debuginfo] target(s) in 1.77s
    ────────────
     Nextest run ID 67df466a-c0c7-4bb4-ba87-2b0411c8ec11 with nextest profile: default
        Starting 1 test across 3 binaries (95 tests skipped)
            PASS [   0.004s] (1/1) fabro-synthesis::bootstrap bootstrap_verify
    ────────────
         Summary [   0.004s] 1 test run: 1 passed, 95 skipped
    ```
- **quality**: fail
  - Script: `set -e
QUALITY_PATH='outputs/greenfield-bootstrap-reliability-bootstrap-verification-gate/quality.md'
IMPLEMENTATION_PATH='outputs/greenfield-bootstrap-reliability-bootstrap-verification-gate/implementation.md'
VERIFICATION_PATH='outputs/greenfield-bootstrap-reliability-bootstrap-verification-gate/verification.md'
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

## Context
- failure_class: deterministic
- failure_signature: quality|deterministic|script failed with exit code: <n>


# Bootstrap Verification Gate Lane — Fixup

Fix only the current slice for `greenfield-bootstrap-reliability-bootstrap-verification-gate`.


Verification artifact must cover
- summarize the automated proof commands that ran and their outcomes

Priorities:
- unblock the active slice's first proof gate — this is the #1 priority
- prefer staying within the named slice and touched surfaces
- if the proof gate fails on pre-existing issues OUTSIDE your surfaces (e.g., linter warnings in unrelated files, missing imports in dependencies), you MUST fix those issues minimally to unblock the gate — do not leave the lane stuck on problems you can solve
- preserve setup constraints before expanding implementation scope
- keep implementation and verification artifacts durable and specific
- do not create or rewrite `.fabro-work/promotion.md` during Fixup; that file is owned by the Review stage
- do not hand-author `.fabro-work/quality.md`; the Quality Gate rewrites it after verification
- ALL ephemeral files (quality.md, promotion.md, verification.md) go in `.fabro-work/`, never the repo root
