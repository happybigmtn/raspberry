# Plan-Level Adversarial Review and Recursive Self-Improvement

When all lanes of a plan complete and their code lands on trunk, trigger a
3-step adversarial bug review on the aggregate diff. Confirmed bugs get
fixed in the target repo. Bug patterns get fed back into fabro as quality
gate rules, convention checks, or prompt improvements — making the system
recursively better with every plan completion.

## Architecture

### Trigger: Plan Completion Detection

In `autodev.rs` `orchestrate_program()`, after each dispatch cycle:
1. Load plan status matrix (already exists in `plan_status.rs`)
2. Compare against previous cycle's plan statuses
3. For any plan that transitions to `*_complete` status, trigger review

Track completed plans in a `BTreeSet<String>` across cycles to avoid
re-triggering reviews for plans that were already reviewed.

### Review Process: 3-Step Adversarial

When a plan completes, compute the aggregate diff of all its lanes:
```bash
# Get all commits from this plan's integration lanes
git log --oneline --grep="integrate(plan-child-" | head -N
# Diff from before first integration to current HEAD
git diff <first-integration-parent>..HEAD -- <owned-surfaces>
```

Then run the 3-step process:

**Step 1 — Bug Finder** (Codex or Claude Agent):
Aggressive bug search on the plan's aggregate code. Score: +1 low,
+5 medium, +10 critical. Maximize score.

**Step 2 — Bug Skeptic** (Codex or Claude Agent):
Challenge each bug. Disprove false positives. Score: +[bug pts] for
correct disproves, -2×[bug pts] for wrong dismissals.

**Step 3 — Arbiter** (Codex or Claude Agent):
Final verdict on each disputed bug. Outputs confirmed bugs with
severity and actionable fixes.

### Bug Fix: Target Repo

For each confirmed bug, the review agent:
1. Creates a fix branch from trunk
2. Applies the fix
3. Runs verify command
4. Integrates the fix to trunk

This is a new lane type: `plan-review` with template `Implementation`.
It runs AFTER all child implementation+integration lanes complete.

### Recursive Improvement: Fabro Meta-Review

After fixing bugs in the target repo, a SECOND review runs:
- Analyze the BUG PATTERNS found (not individual bugs)
- Propose fabro-level changes: quality gate rules, prompt improvements,
  convention checks
- Write proposals to `.fabro-work/meta-review-{plan_id}.md`
- These proposals are reviewed by the operator before merging to fabro

Example patterns → fabro changes:
- "3 games used `as f64` for settlement" → add f64 detection to quality gate
- "Test files don't import subject module" → add import check to quality gate
- "Agents create files at root" → add .fabro-work routing
- "Contract deliverables missing" → add contract-aware quality gate

### Implementation

**Phase 1: Plan completion detection** (`autodev.rs`)
- Track plan statuses across cycles
- Detect transitions to complete
- ~30 lines

**Phase 2: Auto-generate plan-review lanes** (`render.rs`)
- New lane type: `plan-review` with dependencies on all child lanes
- Template: Implementation with custom prompt (the 3-step adversarial prompt)
- Verify command: `cargo test --workspace` or project equivalent
- ~50 lines

**Phase 3: Meta-review prompt** (`render.rs`)
- After plan-review completes, a follow-on prompt asks:
  "What patterns did you find? Propose fabro-level changes."
- Writes to `.fabro-work/meta-review-{plan_id}.md`
- Operator reviews and applies to fabro manually (safety gate)
- ~30 lines

## Execution Order

Phase 1 first (detection), Phase 2 (auto-generation), Phase 3 (meta-review).
Total: ~110 lines across autodev.rs and render.rs.

## Success Criteria

- Plan completion triggers review automatically
- Confirmed bugs get fixed before next plan starts
- Meta-review proposals are actionable and specific
- At least 1 fabro improvement per 5 plan completions
