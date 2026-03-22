# Fabro Architecture

This document is the current handoff map for Fabro, Raspberry, and Paperclip.
It explains what the system does today, what is already implemented, what is
still missing, and where to look in code.

## System Shape

The stack currently has three main layers:

- `fabro-synthesis`: turns repo plans/specs into checked-in control-plane
  artifacts
- `raspberry-supervisor` + `raspberry-cli`: evaluates and supervises the
  generated program state
- `fabro-cli paperclip`: synchronizes the local control plane into the
  Paperclip collaboration surface

The intended end-to-end flow is:

`plans/*.md` -> mapping contracts -> blueprint/program/workflows/prompts ->
Raspberry plan status + scheduling -> Paperclip dashboard/issues/documents

## Current Truth

Today the system can do all of the following:

- read numbered plans through a shared registry in
  `lib/crates/raspberry-supervisor/src/plan_registry.rs`
- compute a plan-first matrix in
  `lib/crates/raspberry-supervisor/src/plan_status.rs`
- author blueprints from numbered plans in
  `lib/crates/fabro-synthesis/src/planning.rs`
- render program manifests, workflows, prompts, and run configs in
  `lib/crates/fabro-synthesis/src/render.rs`
- generate mapping snapshots automatically during
  `fabro synth create` in
  `lib/crates/fabro-cli/src/commands/synth.rs`
- present a first plan-first Paperclip summary in
  `lib/crates/fabro-cli/src/commands/paperclip.rs`

The proving ground is `/home/r/coding/rXMRbro`.

`fabro synth create --target-repo /home/r/coding/rXMRbro --program rxmragent`
now regenerates:

- `fabro/blueprints/`
- `fabro/programs/`
- `fabro/workflows/`
- `fabro/run-configs/`
- `fabro/prompts/`
- `fabro/plan-mappings/`

And `raspberry plan-matrix` now reports all 27 numbered `rXMRbro` plans as
`mapped`.

## What Is Proven

The system currently proves:

- automatic plan -> mapped per-plan workflow generation
- automatic plan -> mapping snapshot generation
- automatic regeneration of `target_repo/fabro` from scratch during
  `synth create`
- plan-first local status visibility

The system does **not** yet prove:

- automatic plan -> milestone-level child workflow generation

That is the main remaining architectural gap.

Example: for `plans/005-craps-game.md`, the current system generates one
`craps` bootstrap workflow and one `005-craps-game.yaml` mapping snapshot, but
it does not yet generate separate workflows like:

- `craps-casino-core`
- `craps-provably-fair`
- `craps-house-agent`
- `craps-tui-shell`
- `craps-e2e-verification`
- `craps-acceptance-balance`

## Model Policy

The intended model split is:

- synth-side decomposition:
  Claude Opus 4.6
- synth-side plan-eng review of decomposition:
  Claude Opus 4.6
- generated execution work:
  MiniMax M2.7 Highspeed
- final promotion review:
  `gpt-5.4`

Important: the current code still generates ordinary execution workflows with
MiniMax defaults in `render.rs`, which is correct for runtime work. The missing
piece is wiring real model intelligence into the synth-side decomposition step.

## Remaining Core Work

The highest-priority remaining work is:

1. Add a two-pass LLM decomposition layer inside `synth create/evolve`
   - draft milestone/workflow mapping
   - plan-eng review/signoff of that mapping
2. Convert synthesized child mappings into actual child workflows, run configs,
   prompts, and program entries
3. Replace bootstrap-only proof of automation with full
   plan -> all workflows proof
4. Deepen Paperclip so top-level issue/work-product identity is keyed by plan
   root and child workflow, not only legacy frontier lanes
5. Tighten proof-contract generation so milestone-level proof commands are
   explicit and primary

## Important Files

### Registry and Status

- `lib/crates/raspberry-supervisor/src/plan_registry.rs`
- `lib/crates/raspberry-supervisor/src/plan_status.rs`

### Synthesis

- `lib/crates/fabro-synthesis/src/planning.rs`
- `lib/crates/fabro-synthesis/src/render.rs`
- `lib/crates/fabro-synthesis/src/blueprint.rs`

### CLI Entry Points

- `lib/crates/fabro-cli/src/commands/synth.rs`
- `lib/crates/fabro-cli/src/commands/paperclip.rs`
- `lib/crates/raspberry-cli/src/main.rs`

### Proving Ground

- `/home/r/coding/rXMRbro/plans/`
- `/home/r/coding/rXMRbro/fabro/`

## Operational Notes

- `synth create` now wipes `target_repo/fabro` before regeneration
- `synth create` also writes `fabro/plan-mappings/*.yaml`
- `raspberry plan-matrix --manifest <program.yaml>` is the best quick check for
  current plan-first status

## Bottom Line

Fabro is no longer only lane-first. It is now plan-first at the registry,
status, synthesis, and initial Paperclip summary layers.

But the architecture is only halfway to the real goal.

The remaining proof we need is:

`plan -> correct milestone-level workflow graph automatically`

That is the next major engineering milestone.
