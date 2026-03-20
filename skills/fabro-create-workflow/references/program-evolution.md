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

## Patterns evolve should actively enforce

When doctrine or run evidence shows that a lane is over-optimistic, evolve
should prefer tightening the contract rather than trusting the next agent run
to "do better".

Prescriptive upgrades evolve should be willing to apply:

- add or strengthen deterministic proof commands
- split broad lanes into bootstrap + implementation families
- add `quality.md` and a deterministic quality gate for implementation lanes
- tighten milestone requirements when artifacts are too weak
- clear stale promotion artifacts before review/promote
- add health gates for service lanes
- add explicit review preconditions from upstream reviewed artifacts

Patterns evolve should infer from repeated failures:

- if runs complete with placeholder-heavy artifacts, add a quality pack
- if promotion is too easy, add machine-readable `promotion_check` blockers
- if auth, money, or trust-boundary code is involved, add explicit security
  review surfaces or security-oriented checks
- if one review perspective is not enough, upgrade to a parallel
  security/architecture/quality review fan-out and merge
