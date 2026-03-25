# Harness Redesign: Sprint Contracts and Scored Evaluation

Based on Anthropic's "Harness Design for Long-Running Apps" engineering post
and 12+ hours of observed autodev runs across rXMRbro (Rust/casino), zend
(Python/miner), and tonofcrap (TypeScript/Telegram Mini App).

## Problem Statement

Five classes of quality failure escape the current harness:

1. **Scope drift** — agents interpret plans freely. A lane for "Convex schema"
   produces queries.ts with bet types not in the spec, `any[]` parameters, and
   `number` for token amounts. No contract defines what "done" means before coding.

2. **Self-evaluation failure** — agents declare quality_ready:yes with missing
   files, broken imports, and tests that don't test the code. The review stage
   "confidently praises the work" (Anthropic's exact finding).

3. **Unstructured evaluation** — the review prompt says "focus on correctness"
   but provides no scored dimensions or thresholds. A reviewer can write
   "looks good, merge_ready: yes" without checking any specific criterion.

4. **No behavioral verification** — the verify gate runs `cargo test` or `npx
   vitest` but doesn't verify that the code actually implements the plan's
   acceptance criteria. Tests can pass while features are missing.

5. **Context anxiety** — agents in long implement stages prematurely wrap up,
   producing skeleton code with TODOs. The fixup cycle catches some but not all.

## Current Workflow DAG

```
start → preflight → implement → verify → quality → challenge → review → audit → exit
                                  ↑                                        |
                                  └──── fixup ←────────────────────────────┘
```

## Proposed Workflow DAG

```
start → preflight → contract → implement → verify → quality → challenge → review → audit → exit
                                              ↑                                      |
                                              └──── fixup ←──────────────────────────┘
```

**One new stage** (contract) and **modifications to two existing stages**
(review scoring, quality completeness check).

---

## Phase 1: Sprint Contracts (render.rs — new "contract" stage)

### What

Add a `contract` stage between `preflight` and `implement`. The contract agent:
1. Reads the plan prompt
2. Writes `.fabro-work/contract.md` with:
   - **Deliverables**: exact files to create/modify
   - **Acceptance criteria**: testable conditions ("X function returns Y for input Z")
   - **Out of scope**: what this lane will NOT do
3. The verify gate checks deliverables from the contract exist

### Where

`render.rs` `render_workflow_graph()` — add a `contract` node for Implementation
template lanes. The contract prompt is auto-generated from the plan prompt.

### Contract Prompt Template

```
Read the implementation plan carefully. Before writing any code, write
.fabro-work/contract.md with:

## Deliverables
List every file you will create or modify, one per line.

## Acceptance Criteria
List 3-8 testable conditions that prove the implementation works.
Each must be verifiable by running a command or checking a file.

## Out of Scope
List what you will NOT implement in this lane.

Do NOT write any code in this stage. Only write the contract.
```

### Verification

The verify gate gains a contract check: for each file listed in
`.fabro-work/contract.md` Deliverables section, verify it exists.
For each acceptance criterion that includes a command, run it.

### Implementation Detail

- New node in the DAG: `contract [label="Contract", prompt="...", reasoning_effort="medium"]`
- Edge: `preflight -> contract -> implement`
- The contract prompt is rendered inline (not a separate file) to keep it simple
- The implement prompt gains: "You MUST satisfy all criteria in .fabro-work/contract.md"
- Cost: ~$0.10-0.30 per lane (short prompt, medium reasoning)

---

## Phase 2: Scored Review Dimensions (render.rs — review prompt)

### What

Replace the unstructured review prompt with explicit scored dimensions and
hard thresholds. The reviewer MUST output scores and fail the lane if any
dimension is below threshold.

### Scoring Rubric

```
Score each dimension 0-10. Any dimension below 6 = merge_ready: no.

## Completeness (weight: 3x)
Does the code implement ALL items from the plan and contract?
- 10: All deliverables present, all acceptance criteria met
- 7: Core deliverables present, 1-2 criteria unmet
- 4: Missing deliverables or >2 criteria unmet
- 0: Skeleton/placeholder implementation

## Correctness (weight: 2x)
Does the code compile, pass tests, handle edge cases?
- 10: All tests pass, edge cases handled, no warnings
- 7: Tests pass, minor edge cases missing
- 4: Some tests fail or significant edge cases missing
- 0: Does not compile or has obvious runtime errors

## Convention (weight: 2x)
Does the code follow project patterns and constraints?
- 10: Matches all project conventions, uses shared types correctly
- 7: Minor deviations from conventions
- 4: Multiple convention violations (wrong types, missing error handling)
- 0: Ignores project conventions entirely

## Test Quality (weight: 1x)
Do tests verify behavioral outcomes against acceptance criteria?
- 10: Tests import subject code, verify all acceptance criteria
- 7: Tests verify most criteria, some structural tests
- 4: Tests exist but don't verify criteria or test duplicated logic
- 0: No tests or tests that pass without real logic
```

### Where

`render.rs` `render_implementation_review_prompt()` — replace the unstructured
review guidance with the scoring rubric. The review agent must output scores
in `.fabro-work/promotion.md`:

```
merge_ready: yes|no
completeness: 8
correctness: 9
convention: 7
test_quality: 6
reason: <one sentence>
next_action: <one sentence>
```

### Promotion Gate Change

`implementation_promotion_contract_command()` — add score parsing. Extract
dimension scores and fail if any is below 6:

```bash
grep -Eq '^completeness: [6-9]$|^completeness: 10$' .fabro-work/promotion.md && \
grep -Eq '^correctness: [6-9]$|^correctness: 10$' .fabro-work/promotion.md && \
grep -Eq '^convention: [6-9]$|^convention: 10$' .fabro-work/promotion.md && \
grep -Eq '^test_quality: [6-9]$|^test_quality: 10$' .fabro-work/promotion.md
```

---

## Phase 3: Contract-Aware Quality Gate (render.rs — quality command)

### What

Extend the quality gate to verify deliverables from `.fabro-work/contract.md`.
If a contract exists, parse its Deliverables section and verify each file exists.

### Where

`implementation_quality_command()` — add contract verification after the
existing placeholder scan:

```bash
if [ -f .fabro-work/contract.md ]; then
  contract_missing=""
  while IFS= read -r line; do
    file=$(echo "$line" | sed 's/^- //' | sed 's/`//g' | xargs)
    if [ -n "$file" ] && [ ! -e "$file" ]; then
      contract_missing="$contract_missing\n$file"
    fi
  done < <(sed -n '/^## Deliverables/,/^## /p' .fabro-work/contract.md | grep '^- ')
  if [ -n "$contract_missing" ]; then
    echo "## Contract Deliverables Missing" >> "$QUALITY_PATH"
    printf '%b\n' "$contract_missing" >> "$QUALITY_PATH"
    quality_ready=no
  fi
fi
```

---

## Phase 4: Harness Simplification Assessment

### What

With Opus 4.6, test whether the challenge stage adds value. Anthropic found
that Opus 4.6 "plans more carefully" and sprint contracts became optional.

### Approach

Run 10 lanes with challenge enabled and 10 without (same plans). Compare:
- Quality gate pass rate
- Review scores
- Code quality (manual assessment)

If challenge adds <10% improvement, remove it to save cost (~$0.50/lane).

### Where

`render.rs` `profile_extra_graph_elements()` — add a `"lean"` profile that
skips the challenge stage. Test on tonofcrap.

This is NOT implemented now — just assessed after Phase 1-3 run for a week.

---

## Execution Order

| Phase | What | Files | LOC | Risk |
|-------|------|-------|-----|------|
| 1 | Sprint contracts | render.rs | ~60 | Low — additive, no existing behavior changed |
| 2 | Scored review | render.rs | ~40 | Low — prompt change, gate addition |
| 3 | Contract-aware quality | render.rs | ~25 | Low — additive check in existing gate |
| 4 | Simplification | — | 0 | None — assessment only |

Total: ~125 LOC across render.rs. All changes are in the workflow DAG rendering —
no changes to the engine, dispatch, or integration pipeline.

## Success Criteria

- Sprint contracts written before every implementation stage
- Review scores present in promotion.md for every reviewed lane
- Quality gate fails when contract deliverables are missing
- Tonofcrap lanes produce TypeScript files that match their plan
- Review reject rate increases initially (catching real issues) then decreases
  (agents learn from contract constraints)
