---
name: fabro-create-workflow
description: Use when the user wants to create or evolve a Fabro workflow package for a repo. Triggers on requests to write `.fabro` or `.toml` files, map repo work into units and lanes, synthesize a full `fabro/` package from broad requirements, or evolve an existing `fabro/` tree against doctrine and run evidence.
---

# Fabro Create Workflow

Turn requirements into a Fabro workflow package: a `.fabro` graph, a `.toml`
run config, and, when Raspberry supervision is involved, the repo-level
contract that lets the supervisory plane understand what the workflow owns.

When the request is broad or repo-shaped, do not jump straight to isolated
files. Use a blueprint first, then create or evolve the full checked-in
`fabro/` package from that blueprint.

If the request is Raspberry-shaped, answer the repo question first:

- what unit and lane exist?
- what milestone does the lane own?
- which artifacts prove that milestone?
- which checks or state files will the supervisor read?
- which run config path should the manifest point at?

Do not jump straight to graph syntax if those answers are missing.

## Start

1. Run `fabro model list` before naming models or providers.
2. Classify the request:
   - Plain Fabro workflow authoring: read `references/dot-language.md`,
     `references/run-configuration.md`, and
     `references/example-workflows.md`.
   - Raspberry-supervised repo or lane authoring: also read
     `references/raspberry-authoring.md` and
     `references/raspberry-examples.md`.
   - Broad repo bootstrap or repo update request: also read
     `references/program-synthesis.md`,
     `references/program-interview.md`,
     `references/program-blueprint-schema.md`, and
     `references/program-evolution.md`.
3. Prefer the simplest topology that satisfies the goal.
4. Produce both the graph and the run config whenever the workflow will be
   checked into a repo or referenced from a Raspberry manifest.

## Program Synthesis Mode

Use this mode when the user is not asking for one lane file, but for a repo
package or a repo update.

Two sub-modes exist:

- **Create**: broad requirement or spec corpus -> blueprint -> full executable
  `fabro/` package
- **Evolve**: existing `fabro/` tree + doctrine + run evidence ->
  imported blueprint -> revised blueprint -> deterministic package patch

In both modes, the blueprint is the primary design artifact. Do not free-write
the final `fabro/` tree first.

## Plain Fabro Workflow Authoring

Use the generic Fabro references when the user is building a standalone
workflow or asking about DOT or TOML syntax.

Core rules:

- Every workflow is a `digraph` with `graph [goal="..."]`.
- Exactly one start node and one exit node are required.
- `box` nodes are tool-using agents, `tab` nodes are single prompt calls,
  `parallelogram` nodes run commands, `diamond` nodes route, `hexagon` nodes
  ask humans, `component` fans out, and `tripleoctagon` merges.
- Validate with `fabro run --preflight run.toml` or
  `fabro validate workflow.fabro`.

## Raspberry-Supervised Repo Authoring

Use this mode when the user is asking from the perspective of a supervised repo
such as `coding/myosu`, or when they mention lanes, milestones, program
manifests, proof profiles, health checks, or the supervisory plane.

Your job is not only to produce a Fabro graph. Your job is to make the repo
legible to the supervisory plane.

That means the answer should usually include:

- the manifest fields the repo must define
- the artifacts and milestone contract for the lane
- the recommended Fabro topology for the lane
- the run-config path and workflow package layout
- any proof, health, or orchestration state surfaces the repo must expose

When the user asks "what does this repo need to do?", answer with a
repo-readiness checklist first and a workflow proposal second.

## Output Shape

For a normal workflow request, return:

- `workflow.fabro`
- `workflow.toml` if needed
- optional prompt files under `prompts/`

For a Raspberry request, return:

- manifest edits or a manifest-ready checklist
- `workflow.fabro`
- `workflow.toml`
- the artifact, milestone, and check contract the repo must satisfy

For a broad repo create request, return:

- a blueprint draft
- the full package generation plan or generated package files

For an evolve request, return:

- imported current-state findings
- doctrine/evidence drift findings
- a revised blueprint
- the deterministic patch plan for the existing `fabro/` tree

## Guardrails

- Never put a `prompt` on a `diamond` node.
- Every `box` and `tab` node needs a `prompt`.
- No edges may enter the start node or leave the exit node.
- Prefer `model_stylesheet` over per-node model assignment.
- Keep retry loops bounded with `max_visits` or retry policy.
- For Raspberry lanes, do not invent hidden control-plane semantics. If the
  manifest does not say what proves success, surface that gap explicitly.
- For Raspberry lanes, the graph should write or update the durable artifacts
  that the lane's milestone requires. A lane is not ready just because the
  graph "did work".

## References

- `references/dot-language.md` for Fabro graph syntax and validator rules
- `references/run-configuration.md` for TOML run config behavior
- `references/example-workflows.md` for generic topology patterns
- `references/raspberry-authoring.md` for the repo contract required by the
  supervisory plane
- `references/raspberry-examples.md` for lane-oriented examples grounded in the
  current Raspberry fixture model
- `references/program-synthesis.md` for blueprint-first repo synthesis
- `references/program-interview.md` for targeted clarification rules
- `references/program-blueprint-schema.md` for the blueprint fields
- `references/program-evolution.md` for updating an existing `fabro/` tree
  from doctrine and evidence

{{user_input}}
