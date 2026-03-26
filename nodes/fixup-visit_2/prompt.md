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
  - Model: MiniMax-M2.7-highspeed, 22.7k tokens in / 362 out
- **implement**: success
  - Model: MiniMax-M2.7-highspeed, 21.7k tokens in / 609 out
  - Files: test/fixtures/raspberry-supervisor/.raspberry/myosu-program-state.json, test/fixtures/raspberry-supervisor/.raspberry/program-state.json
- **verify**: success
  - Script: `true`
  - Stdout: (empty)
  - Stderr: (empty)
- **quality**: fail
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
  - Model: MiniMax-M2.7-highspeed, 50.7k tokens in / 539 out
  - Files: lib/crates/fabro-cli/src/commands/synth.rs, lib/crates/fabro-synthesis/src/planning.rs, lib/crates/fabro-synthesis/src/render.rs, lib/crates/raspberry-supervisor/src/evaluate.rs, test/fixtures/raspberry-supervisor/.raspberry/myosu-program-state.json, test/fixtures/raspberry-supervisor/.raspberry/program-state.json
- **verify**: success
  - Script: `true`
  - Stdout: (empty)
  - Stderr: (empty)
- **quality**: fail
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

## Context
- failure_class: deterministic
- failure_signature: quality|deterministic|script failed with exit code: <n>


# Ci Preservation And Hardening Lane — Fixup

Fix only the current slice for `test-coverage-critical-paths-ci-preservation-and-hardening`.


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
