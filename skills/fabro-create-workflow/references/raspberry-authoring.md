# Raspberry Supervisory Plane Authoring

Use this reference when the user is asking from the perspective of a supervised
repo such as `coding/myosu`, or when the request mentions units, lanes,
milestones, proof profiles, health checks, or the supervisory plane.

The core question is:

What does this repo have to provide so the supervisory plane can decide what
work exists, what is blocked, what is healthy, and what counts as done?

## What The Supervisory Plane Needs

A supervised repo is ready only when checked-in files answer these questions:

1. What work exists?
   Answer with a program manifest containing units and lanes.

2. What does each lane own?
   Answer with `managed_milestone` and the artifacts that satisfy it.

3. What blocks readiness?
   Answer with dependencies and precondition checks.

4. What proves correctness?
   Answer with `proof_profile`, proof checks, or `proof_state_path`.

5. What shows runtime health or orchestration state?
   Answer with `service_state_path`, `orchestration_state_path`, running
   checks, and optional `run_dir`.

If the repo cannot answer those questions, the graph is not enough yet.

## Required Manifest Contract

The current Raspberry manifest model expects these ideas:

### Program

- `program`
- `target_repo`
- `state_path`
- `max_parallel`
- optional `run_dir`

### Unit

- `id`
- `title`
- optional `output_root`
- `artifacts`
- `milestones`
- `lanes`

### Lane

Every lane needs:

- `id`
- `kind`
- `title`
- `run_config`
- `managed_milestone`

Most useful lanes also need:

- `produces`
- `depends_on` / `dependencies`
- optional `proof_profile`
- optional `checks`
- optional `run_dir`

Service, orchestration, and proof-sensitive lanes may also need:

- `service_state_path`
- `orchestration_state_path`
- `proof_state_path`

## Repo-Readiness Checklist

When a user asks what a repo must do, answer in this order.

### 1. Define durable outputs first

For each lane, decide which files prove useful work happened.

Examples:

- `runtime_spec.md`
- `runtime_review.md`
- `scorecard.md`
- `launch_report.md`
- `tui_spec.md`

Those artifact paths belong to the unit. The lane's `produces` list chooses
which artifacts it is responsible for.

### 2. Define the milestone contract

Pick the milestone the lane owns, then make the milestone require the produced
artifacts.

Examples:

- `reviewed` requires `runtime_spec` and `runtime_review`
- `publish_ready` requires `scorecard`
- `ready` requires `launch_report`

The supervisory plane reasons about completion from this contract. If the
milestone is vague, the plane cannot tell when the lane is done.

### 3. Define dependencies and readiness checks

Dependencies tell the plane what must already be true before the lane is ready.

Use:

- lane or unit milestone dependencies
- precondition checks for repo-local flags or machine-readable facts

Examples:

- chain must be reviewed before validator starts
- a `chain-ready.flag` file must exist
- a JSON probe must say `"status": "ok"`

### 4. Define proof and observability surfaces

The graph may do the work, but the supervisory plane needs durable, observable
facts after the run.

Use the smallest surface that tells the truth:

- `proof_profile` when the lane has a clear evidence theme
- `proof_state_path` when proof status is written as machine-readable state
- `service_state_path` for long-running health
- `orchestration_state_path` for launch or coordination state
- running checks for live health facts

### 5. Define the workflow package path

The manifest points at `run_config`, not directly at the DOT file. That means
the repo should check in a stable workflow package for each lane:

```text
some-repo/
  fabro/
    programs/
      myosu.yaml
    workflows/
      chain-runtime/
        workflow.fabro
        workflow.toml
        prompts/
          plan.md
          implement.md
```

The exact directories can vary. The important rule is that the manifest path,
the run config path, and the graph path remain repo-local and predictable.

## How The Fabro Graph Must Line Up

The supervisory plane does not inspect the graph to infer success. It inspects
the manifest contract and the durable outputs around the run.

So the graph should be designed to satisfy the lane contract:

- write the artifacts that the milestone requires
- run the verification or proof steps that justify `proof_profile`
- update or emit the state files that observers and running checks will read
- avoid burying success conditions only inside transient assistant text

If a lane owns `runtime_spec.md` and `runtime_review.md`, the graph should
write those files or drive commands that produce them.

If a lane owns service health, the graph should leave behind the service state
or smoke-test outputs that the manifest points to.

## Lane-Kind Guidance

### `platform` or `artifact`

Bias toward:

- plan
- implement
- verify
- review or summarize

Best when the lane's milestone is about checked-in artifacts.

### `service`

Bias toward:

- plan or configure
- implement or launch
- smoke test
- write a service report or spec artifact

Also define service health outside the graph with `service_state_path` or
running checks.

### `orchestration`

Bias toward:

- gather upstream state
- evaluate readiness
- run coordination commands
- verify launch or proof facts
- write a report artifact

Use `orchestration_state_path` when the lane has durable waiting or blocked
state that is not just "failed".

### `interface`

Bias toward:

- plan or spec
- implement
- snapshot, smoke, or screenshot verification
- write a spec or review artifact

### `recurring`

Bias toward:

- collect facts
- synthesize
- verify a policy or threshold
- write a durable report

These lanes are good fits for scorecards, compiler-like normalization, and
periodic control-plane summaries.

## Common Mistakes

- Treating the graph as the only source of truth.
  The supervisory plane needs manifest fields, artifacts, and durable state.

- Making the lane milestone vague.
  If success is not tied to artifact requirements, the lane is hard to observe.

- Omitting `produces`.
  Then the observer falls back to the whole unit artifact set, which is less
  precise than a lane-owned contract.

- Hiding proof semantics only inside prompts.
  Put proof posture in `proof_profile`, proof checks, or `proof_state_path`.

- Using placeholder run-config paths without deciding the checked-in package
  layout.
  The manifest should reference real files the repo intends to keep stable.

## What To Return

When the user asks "what do we have to do to meet supervisory-plane
requirements?", return:

- a repo-readiness checklist
- the manifest fields to add or tighten
- the artifact and milestone contract
- the check and state surfaces to expose
- the Fabro topology and workflow package you recommend for the lane
