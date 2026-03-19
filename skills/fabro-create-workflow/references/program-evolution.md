# Program Evolution

Use this reference when the user wants to update an existing `fabro/` tree.

The evolve loop is:

1. import the current `fabro/` package into a blueprint-like model
2. read doctrine files such as `OS.md`
3. read run evidence such as `outputs/`, `.raspberry/`, and `~/.fabro/runs/`
4. describe the drift
5. revise the blueprint
6. evolve the existing package deterministically

## Doctrine inputs

Doctrine files are repo-level sources that describe how the repo wants to
operate. Examples:

- `OS.md`
- `AGENTS.md`
- project plans or specs

Doctrine should influence:

- lane ordering
- lane kind
- milestone naming
- proof posture
- whether work should stay bootstrap-only or become implementation-shaped

## Evidence inputs

Run evidence should influence:

- missing dependencies
- stale or over-broad milestones
- lanes that complete without useful artifacts
- repeated failure causes
- services or orchestration lanes that need stronger health surfaces

## What evolve should change

Typical evolve changes:

- reorder dependencies
- split one lane into bootstrap + implementation
- tighten `produces` or milestone requirements
- add proof or health state paths
- add a new downstream lane required by doctrine
- remove or demote dead lanes that no longer fit the doctrine

The important rule is that evolve should explain each structural change in
terms of doctrine or evidence, not in terms of stylistic preference.
