# Program Synthesis

Use this reference when the user is asking for a repo-level `fabro/` package,
not only for one workflow graph.

The correct sequence is:

1. inspect the repo and requirement corpus
2. write a blueprint
3. render the package from the blueprint

Do not start by free-writing many `.fabro` and `.toml` files.

## Create Mode

Use create mode when the repo does not already have a suitable `fabro/` tree or
when the user is bootstrapping a new supervised repo.

Inputs:

- broad requirement such as "build a craps game"
- checked-in specs or plans
- existing crates, services, and outputs

Output:

- one blueprint describing the program, units, lanes, milestones, artifacts,
  dependencies, and workflow family choices

Then the renderer turns that blueprint into the checked-in package.

## Minimal-first rule

The first generated package should be the smallest honest executable shape.

Do not generate every future lane just because the specs mention them. Prefer:

- the first bootstrap or restart lanes
- the first implementation lane only when the repo already has enough trust and
  proof surface
- stable shared workflow families over many lane-specific one-offs

The package can grow later through evolve mode.
