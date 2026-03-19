# Raspberry Lane Authoring Examples

These examples are derived from the current Myosu-shaped fixture manifest in
`test/fixtures/raspberry-supervisor/myosu-program.yaml`.

Use them to answer the repo-facing question:

What does this lane need to expose so the supervisory plane can reason about
it?

## 1. Platform Lane: Chain Runtime

Manifest shape:

```yaml
- id: runtime
  kind: platform
  run_config: run-configs/chain-runtime.toml
  managed_milestone: reviewed
  proof_profile: cargo_workspace
  produces: [runtime_spec, runtime_review]
```

Repo obligations:

- check in a run config for the lane
- produce `runtime_spec.md` and `runtime_review.md`
- define a `reviewed` milestone that requires both artifacts

Suggested topology:

- plan
- implement
- verify
- review

Why: this lane is artifact-owned. The supervisory plane mainly needs durable
spec and review outputs tied to the `reviewed` milestone.

## 2. Service Lane: Validator or Miner

Manifest shape:

```yaml
- id: service
  kind: service
  run_config: run-configs/miner-service.toml
  managed_milestone: service_ready
  proof_profile: miner_tests
  service_state_path: myosu/state/miner-service-state.json
  depends_on:
    - unit: chain
      milestone: reviewed
  checks:
    - label: abstractions_ready
      kind: precondition
      type: file_exists
      path: myosu/preconditions/abstractions-ready.flag
```

Repo obligations:

- define what upstream milestone must exist first
- expose precondition flags or JSON facts for readiness
- expose service health as a durable machine-readable file or running checks
- still produce durable artifacts such as `miner_spec.md` or
  `miner_service.md`

Suggested topology:

- plan
- implement or launch
- smoke test
- write service artifact

Why: a service lane is not only about artifacts. The supervisor also needs a
truthful health surface after the graph runs.

## 3. Orchestration Lane: Scorecard or Launch

Manifest shape:

```yaml
- id: scorecard
  kind: orchestration
  run_config: run-configs/operations-scorecard.toml
  managed_milestone: publish_ready
  proof_profile: ops_smoke
  proof_state_path: myosu/state/validator-proof-state.json
  checks:
    - label: validator_proof_passed
      kind: proof
      type: command_stdout_contains
      command: cat myosu/preconditions/validator-proof.json
      contains: '"status": "passed"'
```

Repo obligations:

- define upstream dependencies clearly
- expose proof or orchestration facts as machine-readable state
- write the durable report artifact such as `scorecard.md` or `launch_report.md`

Suggested topology:

- gather upstream facts
- evaluate state
- run coordination or launch commands
- verify proof or readiness
- write report

Why: orchestration lanes exist to turn many upstream facts into one durable
decision artifact the operator can trust.

## 4. Interface Lane: Gameplay TUI

Manifest shape:

```yaml
- id: tui
  kind: interface
  run_config: run-configs/play-tui.toml
  managed_milestone: specified
  proof_profile: tui_snapshots
  depends_on:
    - unit: validator
      lane: oracle
      milestone: specified
  produces: [tui_spec]
```

Repo obligations:

- define the upstream interface dependency precisely
- produce the lane-owned artifact, here `tui_spec.md`
- if the proof profile implies snapshots or smoke checks, design the graph to
  generate or verify them

Suggested topology:

- design or spec
- implement
- snapshot or smoke verify
- write spec or review artifact

Why: interface lanes are often artifact-driven, but their proof profile usually
depends on visual or interaction checks.

## Recommended Answer Shape

When a user gives you a repo and asks what must change, structure the answer as:

1. Manifest contract
   - unit
   - lane
   - managed milestone
   - produced artifacts
   - dependencies
   - checks and state surfaces

2. Workflow package
   - `workflow.fabro`
   - `workflow.toml`
   - prompt files if needed

3. Observability contract
   - proof surface
   - health surface
   - run directory or state paths

4. Suggested topology
   - the smallest Fabro pattern that satisfies the lane contract
