Goal: Synthesis Runtime Regression Tests

Child work item of plan: Test Coverage for Critical Paths

Proof commands:
- `cargo nextest run -p fabro-synthesis -- render`

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
  cargo nextest run -p fabro-synthesis -- render
else
  cargo test -p fabro-synthesis -- render
fi
true`
  - Stdout: (empty)
  - Stderr:
    ```
    (104 lines omitted)
    
        failures:
            render::tests::inject_workspace_verify_lanes_document_release_is_hard_gate_and_retro_is_tail
    
        test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 89 filtered out; finished in 0.00s
    
      stderr ───
    
        thread 'render::tests::inject_workspace_verify_lanes_document_release_is_hard_gate_and_retro_is_tail' (3409883) panicked at lib/crates/fabro-synthesis/src/render.rs:7260:14:
        document release unit exists
        note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    
            PASS [   0.006s] (50/68) fabro-synthesis render::tests::parent_holistic_deep_and_adjudication_use_expected_provider_failover
            PASS [   0.026s] (51/68) fabro-synthesis render::tests::bootstrap_run_config_omits_direct_integration_for_non_git_repo
            PASS [   0.005s] (52/68) fabro-synthesis render::tests::proof_commands_from_markdown_collects_commands_from_fenced_block
            PASS [   0.029s] (53/68) fabro-synthesis render::tests::bootstrap_run_config_uses_minimax_defaults_and_direct_integration
            PASS [   0.029s] (54/68) fabro-synthesis render::tests::bootstrap_run_config_targets_local_branch_when_origin_is_missing
            PASS [   0.019s] (55/68) fabro-synthesis render::tests::implementation_run_config_enables_direct_integration
    ────────────
         Summary [   0.031s] 55/68 tests run: 52 passed, 3 failed, 27 skipped
            FAIL [   0.006s] (36/68) fabro-synthesis render::tests::inject_workspace_verify_lanes_adds_conditional_parent_lanes_for_sensitive_plan
            FAIL [   0.008s] (37/68) fabro-synthesis render::tests::inject_workspace_verify_lanes_adds_parent_holistic_gauntlet
            FAIL [   0.009s] (49/68) fabro-synthesis render::tests::inject_workspace_verify_lanes_document_release_is_hard_gate_and_retro_is_tail
    warning: 13/68 tests were not run due to test failure (run with --no-fail-fast to run all tests, or run with --max-fail)
    error: test run failed
    ```
- **contract**: success
  - Model: MiniMax-M2.7-highspeed, 19.4k tokens in / 451 out
- **implement**: success
  - Model: MiniMax-M2.7-highspeed, 29.0k tokens in / 295 out
  - Files: lib/crates/fabro-synthesis/src/render.rs
- **verify**: success
  - Script: `if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p fabro-synthesis -- render
else
  cargo test -p fabro-synthesis -- render
fi`
  - Stdout: (empty)
  - Stderr:
    ```
    (52 lines omitted)
            PASS [   0.011s] (46/68) fabro-synthesis render::tests::raw_lane_refs_finds_lane_like_tokens
            PASS [   0.007s] (47/68) fabro-synthesis render::tests::service_bootstrap_workflow_retries_verify_outputs_via_polish
            PASS [   0.020s] (48/68) fabro-synthesis render::tests::normalize_lane_verify_command_wraps_explicit_nextest_command
            PASS [   0.022s] (49/68) fabro-synthesis render::tests::normalize_blueprint_lane_kinds_downgrades_invalid_implementation_integration_kind
            PASS [   0.059s] (50/68) fabro-synthesis render::tests::augment_with_implementation_follow_on_units_adds_child_program_lane
            PASS [   0.017s] (51/68) fabro-synthesis render::tests::promotion_contract_requires_security_fields_for_sensitive_lane
            PASS [   0.017s] (52/68) fabro-synthesis render::tests::proof_commands_from_markdown_collects_commands_from_fenced_block
            PASS [   0.006s] (53/68) fabro-synthesis render::tests::setup_notes_from_markdown_extracts_slice_one_setup_steps
            PASS [   0.019s] (54/68) fabro-synthesis render::tests::parent_holistic_minimax_workflow_uses_minimax_first_pass
            PASS [   0.015s] (55/68) fabro-synthesis render::tests::review_stage_requirements_extracts_blocker_stage_requirements
            PASS [   0.007s] (56/68) fabro-synthesis render::tests::slice_notes_from_markdown_keeps_ordering_constraints
            PASS [   0.008s] (57/68) fabro-synthesis render::tests::service_review_prompt_includes_observability_sections
            PASS [   0.007s] (58/68) fabro-synthesis render::tests::smoke_commands_from_markdown_extracts_inline_smoke_gate
            PASS [   0.027s] (59/68) fabro-synthesis render::tests::observability_notes_from_markdown_keeps_structured_log_signals
            PASS [   0.017s] (60/68) fabro-synthesis render::tests::service_implementation_workflow_includes_health_gate
            PASS [   0.010s] (61/68) fabro-synthesis render::tests::trim_list_prefix_removes_leading_numeric_marker
            PASS [   0.017s] (62/68) fabro-synthesis render::tests::service_review_prompt_includes_health_sections
            PASS [   0.077s] (63/68) fabro-synthesis render::tests::bootstrap_run_config_omits_direct_integration_for_non_git_repo
            PASS [   0.081s] (64/68) fabro-synthesis::synthesis render_blueprint_writes_expected_package
            PASS [   0.136s] (65/68) fabro-synthesis render::tests::implementation_run_config_enables_direct_integration
            PASS [   0.158s] (66/68) fabro-synthesis render::tests::bootstrap_run_config_targets_local_branch_when_origin_is_missing
            PASS [   0.118s] (67/68) fabro-synthesis render::tests::service_bootstrap_run_config_enables_direct_integration
            PASS [   0.196s] (68/68) fabro-synthesis render::tests::bootstrap_run_config_uses_minimax_defaults_and_direct_integration
    ────────────
         Summary [   0.199s] 68 tests run: 68 passed, 27 skipped
    ```
- **quality**: fail
  - Script: `set -e
QUALITY_PATH='outputs/test-coverage-critical-paths-synthesis-runtime-regression-tests/quality.md'
IMPLEMENTATION_PATH='outputs/test-coverage-critical-paths-synthesis-runtime-regression-tests/implementation.md'
VERIFICATION_PATH='outputs/test-coverage-critical-paths-synthesis-runtime-regression-tests/verification.md'
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


# Synthesis Runtime Regression Tests Lane — Fixup

Fix only the current slice for `test-coverage-critical-paths-synthesis-runtime-regression-tests`.


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
