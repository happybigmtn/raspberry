# Plan-First Autodev Redesign — Engineering Review Brief

This document summarizes the plan-first autodev redesign for senior engineering review before the first production `synth create` run against rXMRbro.

## What changed

The synthesis engine (`fabro synth create`) previously generated one flat bootstrap workflow per numbered plan file. A repo with 27 plans got 28 workflows (27 bootstraps + 1 workspace foundation). Plan files like `plans/005-craps-game.md` were treated as passive evidence — the system knew they existed but couldn't execute them.

Now, `synth create` decomposes each composite plan into milestone-level child workflows. The same 27 plans produce ~150-200 workflow units, each with typed proof commands, owned surfaces, and dependency chains. The decomposition is performed by Claude Opus 4.6 via `claude -p --dangerously-skip-permissions`, with a deterministic heuristic fallback (`--no-decompose`).

## Architecture

```
plans/*.md
    |
    v
plan_registry (raspberry-supervisor) — parses plans, reads mapping contracts
    |
    v
Opus decomposition (single claude -p call, parallel agent team)
    |
    v
malinka/plan-mappings/*.yaml — enriched mapping contracts per plan
    |
    v
author_blueprint_for_create (fabro-synthesis) — emits parent + child LaneIntents
    |
    v
render_blueprint — generates .fabro workflows, prompts, run-configs, manifest
    |
    v
malinka/ output directory (was fabro/)
```

## Schema: 4 archetypes × 4 review profiles

**Archetypes** determine workflow graph shape:

| Archetype | Graph | When |
|---|---|---|
| `implement` | Full implementation loop (preflight→implement→verify→quality→challenge→review→audit) | All code work — modules, services, clients, verification, testing, migration |
| `integration` | Supervisor-only dispatch | E2E tests, cross-crate wiring, system tests |
| `orchestration` | Supervisor-only dispatch | Meta-work spawning child programs |
| `report` | RecurringReport (specify→review→polish→verify) | Non-code artifacts — audits, status reports, scorecards |

**Review profiles** determine rigor level within the implementation graph:

| Profile | max_visits | Extra node | When |
|---|---|---|---|
| `standard` | 3 | None | Default for ordinary code |
| `foundation` | 4 | **Opus escalation** — checks downstream compatibility of shared contract changes | Shared types, traits, SDK, framework code |
| `hardened` | 5 | **Deep review + recheck** — adversarial review + independent proof re-run | Security, crypto, financial logic, correctness-critical invariants |
| `ux` | 3 | **Acceptance gate** — checks for acceptance-evidence.md | User-facing surfaces (TUI, web, mobile, CLI) |

## Key design decisions

1. **Opus-first decomposition**: `synth create` sends each composite plan to Opus by default. Heuristic fallback via `--no-decompose`. Opus produces cleaner child decompositions (5-7 children per plan vs 10-15 from heuristics).

2. **Full plan context in worker prompts**: Every MiniMax worker and Opus reviewer receives the complete plan markdown in their prompt context. Workers have the domain knowledge (design decisions, specifications, payout formulas) needed to implement correctly.

3. **Non-destructive writes**: `synth create` no longer wipes the output directory. Existing Opus-authored mapping contracts are preserved. Only heuristic-generated mappings are overwritten.

4. **Proof-contract precedence**: Child workflows use proof commands from the mapping contract (e.g., `cargo test -p casino-core --features craps`) instead of generic `test -f` artifact checks.

5. **Model routing**: MiniMax M2.7 Highspeed for write/challenge stages. Claude Opus 4.6 for final review. Fallback chain: opus → gpt-4.6 (codex) → kimi 2.5. All via local `claude --dangerously-skip-permissions` — no API keys.

6. **Output directory renamed**: `fabro/` → `malinka/` via `DEFAULT_PACKAGE_DIR` constant.

## New commands

| Command | What it does |
|---|---|
| `fabro synth create --target-repo ... --program ...` | Decompose plans with Opus, generate full workflow package |
| `fabro synth create --no-decompose ...` | Heuristic-only fallback (offline/CI) |
| `fabro synth review --target-repo ...` | Adversarial eng-review of mapping contracts via Opus |
| `fabro synth genesis --target-repo ...` | For unfamiliar codebases: Opus explores as interim CEO, writes SPEC.md + PLANS.md + numbered plans, then runs synth create |

## What's NOT in this release

- Paperclip plan-root-keyed sync (still lane-centric)
- Shadow-mode cutover from lane-centric to plan-centric dispatch
- Portfolio scheduler with global surface-lock enforcement (module exists but not wired to dispatch yet)

## Risk assessment

| Risk | Mitigation |
|---|---|
| Opus decomposition produces bad children | `synth review` adversarial pass catches misassignments; mapping contracts are checked in and human-reviewable |
| Non-destructive writes leave stale files | Render pipeline handles file updates atomically; obsolete files are removed by `cleanup_obsolete_package_files` |
| 200+ workflow units overwhelm the autodev scheduler | Existing `max_parallel` budget limits concurrent work; children depend on parent bootstrap |
| Profile-specific graph nodes change behavior | Only 3 non-standard profiles have extra nodes; all are between challenge→review, not in the hot path |

## Test results

- `plan_registry`: 5/5 pass (enriched child records, mapping contract parsing)
- `portfolio_scheduler`: 3/3 pass (surface locks, dependency filtering, foundation priority)
- `maintenance`: 3/3 pass (read-only, dispatch refusal, autodev stop)
- `fabro-synthesis`: 59/59 lib tests pass
- Full compilation: zero warnings, zero errors across raspberry-supervisor, fabro-synthesis, fabro-cli, raspberry-cli

## How to run

```bash
# Delete old output directory
rm -rf /home/r/coding/rXMRbro/fabro

# Generate new plan-first package with Opus decomposition
cd /home/r/coding/fabro && cargo run -p fabro-cli -- synth create \
  --target-repo /home/r/coding/rXMRbro \
  --program rxmragent

# Review the mapping contracts
cd /home/r/coding/fabro && cargo run -p fabro-cli -- synth review \
  --target-repo /home/r/coding/rXMRbro

# Inspect the plan matrix
cd /home/r/coding/fabro && cargo run -p raspberry-cli -- plan-matrix \
  --manifest /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml
```

## Questions for review

1. Is the 4×4 schema (archetypes × profiles) the right level of granularity?
2. Should `synth genesis` require human approval before running `synth create` on the generated plans?
3. Is the fallback chain (opus → gpt-4.6 → kimi 2.5) correct for your model access?
4. Should the Paperclip sync refactor block this release, or can it ship separately?
