# Decision: Repo requirements and existing workflow trees evolve through a skill-guided blueprint layer

Status: Draft
Date: 2026-03-19
Type: Decision Spec
Depends-on: `specs/031926-workflow-authoring-knowledge-boundary.md`
Supersedes: the implicit assumption that the main built-in Fabro authoring
surface should stop at direct `.fabro` and `.toml` generation or that existing
`fabro/` trees should be updated only by ad hoc manual editing

## Decision

Fabro should gain a new authoring layer for supervised repositories:

- the built-in `fabro-create-workflow` skill remains the canonical home of
  authoring intelligence
- that skill grows two higher-level modes:
  - **create** mode for broad greenfield requirements
  - **evolve** mode for existing `fabro/` trees that must be revised against
    doctrine, specs, and run evidence
- both modes produce a structured **blueprint** file, not a pile of free-written
  checked-in files
- a deterministic compiler then renders that blueprint into the checked-in
  `fabro/` package for a repository: program manifests, run-config TOMLs,
  workflow graphs, prompts, checks, and layout scaffolding
- in evolve mode, the system first imports the existing `fabro/` tree into a
  blueprint-like model, compares it to doctrine and evidence, and then renders
  a deterministic patch back into the repo

In other words, `SKILL.md` and its references should drive the interview and
the decomposition, while a renderer owned by Fabro should own the final file
layout, path conventions, and update behavior.

The first-class target is not "write one graph." The first-class target is:

- start from a broad requirement such as "build a craps game" and synthesize
  the repo-level supervised program structure needed to execute that outcome
- or start from an existing workflow tree such as `myosu/fabro`, inspect
  doctrine like `OS.md`, inspect run evidence, and revise the existing package
  so execution can proceed more honestly and effectively

## Why Now

The Myosu migration exposed the gap clearly.

What was hard was not DOT syntax or TOML syntax. What was hard was the
decomposition work:

- deciding what units and lanes existed
- deciding which lanes were bootstrap, restart, implementation, or recurring
- deciding which artifacts proved milestones
- deciding which checks or state files the supervisor should read
- deciding which lane depended on which other lane
- deciding which run-config and workflow-package paths the manifest should
  stabilize around

That work was done through repeated interactive review and manual authoring of
the `fabro/` tree in Myosu. The resulting package is valuable, but the authoring
experience does not scale well. If the user begins with a broad goal and a
specification corpus, Fabro should be able to help derive the control-plane
shape rather than forcing the user to hand-author every file.

The same migration also exposed the second, more important gap: once a repo has
an existing `fabro/` tree, the hard problem becomes revision from evidence.
For Myosu, that means being able to inspect:

- doctrine documents such as `OS.md`
- checked-in plans and specs
- current `fabro/` manifests, workflows, and run configs
- produced artifacts under `outputs/`
- supervisor state under `.raspberry/`
- run truth under `~/.fabro/runs/`

and then change the workflow package accordingly: reordering dependencies,
splitting or merging lanes, refining milestones, tightening proof contracts, or
otherwise updating the control-plane structure in a principled way.

The repository already contains the right foundation for this next step:

- `skills/fabro-create-workflow/` already owns the authoring heuristics
- `lib/crates/fabro-cli/src/skill.rs` already ships that skill through
  `fabro skill install`
- `lib/crates/raspberry-supervisor/src/manifest.rs` already defines the
  supervisory contract that generated repos must satisfy

So the next useful layer is not a second unrelated skill and not a vague
"better prompt." It is a structured synthesis pipeline above the current
workflow-authoring skill.

## What the New Layer Must Do

### 1. Inspect before asking

Given a target repo, requirement document, spec tree, doctrine file, or
existing `fabro/` tree, the system should
first try to answer as much as it can from checked-in evidence:

- existing crates, services, scripts, and docs
- existing specs and plans
- existing `fabro/` or `outputs/` surfaces
- existing proof commands
- existing doctrine files
- existing run logs and supervisor state

Only after that should it ask the user questions.

### 2. Ask targeted questions only

The dialogue should only ask for information that the repo cannot infer safely.
Examples:

- is the first slice bootstrap analysis, restart, or real implementation?
- is the first deliverable local gameplay, chain integration, or service bringup?
- what proof bar matters first: tests, health checks, devnet launch, or reviewed artifacts?
- is the lane meant to end in checked-in artifacts, long-running service state,
  or orchestration reports?
- when doctrine and current workflow structure disagree, which source should
  win in the near term?

The dialogue should not be an open-ended design interview when the repo already
contains enough evidence to answer most of those questions.

### 3. Write a structured blueprint

The LLM-facing skill should not directly author the final `fabro/` tree as the
primary output. It should write a structured intermediate representation, called
here a **blueprint**.

That blueprint should capture:

- program identity
- target repo and output roots
- units and lanes
- lane kind and milestone ownership
- artifact contracts
- dependency and readiness contracts
- proof, health, and orchestration semantics
- recommended workflow family and prompt family
- current observed workflow shape, when importing an existing repo
- doctrine-derived requirements and priorities
- run-evidence-derived findings such as stale lanes, failed prerequisites,
  missing artifacts, or over-broad milestones
- explicit unresolved questions, if any remain

### 4. Compile the blueprint deterministically

The final checked-in files should be emitted by a deterministic renderer owned
by Fabro. That renderer should generate:

- `fabro/programs/*.yaml`
- `fabro/run-configs/**/*.toml`
- `fabro/workflows/**/*.fabro`
- prompt/check skeletons where needed
- stable directory layout

This keeps path conventions, naming, and file layout reproducible instead of
depending on the wording style of a single model run.

### 5. Evolve existing trees from evidence

For existing repos, the system must not only create new files. It must also be
able to revise an existing checked-in package.

That means:

- import the current `fabro/` tree into a blueprint-like model
- compare that model to doctrine and run evidence
- produce a human-reviewable explanation of the drift
- render a deterministic patch that updates the existing package rather than
  blindly overwriting it

This evolve mode is the real steering surface for long-lived supervised
repos.

## Boundary with the Existing Skill

`specs/031926-workflow-authoring-knowledge-boundary.md` already established
that the built-in `fabro-create-workflow` skill is the canonical home for
workflow-authoring intelligence. This decision keeps that rule.

The new synthesis layer does **not** replace the skill.

Instead:

- the skill remains the canonical heuristic source
- the skill gains higher-level create and evolve modes
- the blueprint compiler consumes the output of those modes

This avoids two bad outcomes:

1. duplicating heuristics in Rust without a canonical prompt/reference corpus
2. leaving final checked-in layout to unconstrained free-form model output

## Alternatives Considered

### 1. Keep manual `fabro/` authoring as the main path

Why not:

- it does not scale to broad repo-level requests
- it makes the cost of adopting Raspberry disproportionately high
- it repeats the same decomposition work repo by repo

### 2. Let the LLM directly free-write the final `fabro/` tree every time

Why not:

- path conventions drift easily
- file layout becomes prompt-shaped instead of product-shaped
- idempotent regeneration is much harder
- it is difficult to review or diff the intended program structure separately
  from the rendered files
- it is even harder to safely revise an existing workflow tree from doctrine
  and run evidence without a structured intermediate form

### 3. Build a deterministic generator with no interview layer

Why not:

- broad requirements such as "build a craps game" still need judgement
- the repo often lacks enough evidence to choose the first milestone or proof
  posture without clarification
- omitting the interview would force brittle assumptions into code too early

### 4. Treat update of existing workflow trees as a separate unrelated product

Why not:

- the same heuristics that create a supervised repo package are needed to revise
  it later
- splitting create and evolve into unrelated systems would duplicate the
  doctrine, manifest, and lane-shaping logic
- the blueprint layer is exactly what allows both operations to share one model

### 5. Create a new standalone synthesis skill unrelated to `fabro-create-workflow`

Why not:

- it would split heuristics across two built-in authoring surfaces
- it would duplicate Fabro syntax, run-config, and Raspberry-manifest knowledge
- it would conflict with the earlier boundary decision

## Consequences

Positive consequences:

- supervised repo bootstrapping becomes much more repeatable
- broad requirements can be turned into executable program structure with less
  manual hand-authoring
- existing supervised repos gain a principled update path driven by doctrine and
  run evidence instead of ad hoc edits
- the final checked-in `fabro/` tree becomes regenerable from a stable source
  file rather than only from chat history
- Fabro gains a cleaner story for "start from requirements, end with a runnable
  supervisory package" and "start from an existing package, end with a better
  one"

Tradeoffs:

- a new blueprint schema must be designed and versioned
- the first implementation should ship a review gate before auto-applying files
- the skill and the renderer must stay aligned as the Raspberry manifest model
  evolves
- doctrine ingestion and evidence-driven evolution rules must remain explainable,
  or the updater becomes hard to trust

## Initial Implementation Direction

The first implementation should be two-step, not one giant magical command.

Step 1:

- use the built-in skill in create or evolve mode to inspect the repo,
  interview the user when needed, and write a blueprint draft or imported
  blueprint revision

Step 2:

- use a deterministic Fabro renderer to compile that blueprint into the repo's
  `fabro/` tree or to patch the existing tree deterministically

This keeps the intelligent decomposition and the deterministic file emission
separate, which is easier to review, test, and evolve.

## What This Does Not Decide

This decision does not yet settle:

- the exact CLI command names
- whether the blueprint lives under `fabro/blueprints/`,
  `fabro/design/`, or another repo-local directory
- whether the interview helper should later be runnable through a first-class
  `fabro` command or stay skill-driven initially
- the exact doctrine file convention (for example, `OS.md`, `AGENTS.md`, or a
  dedicated doctrine path)
- the exact mechanism for importing an existing `fabro/` tree into a blueprint

Those decisions belong in the implementation plan.
