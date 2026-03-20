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

## Deterministic workflow-family selection

When create mode starts from an accepted plan, do not browse a wide design
space unless the plan forces you to.

Map each lane to one of these workflow families first:

1. `orchestration`
   The lane supervises a child program.

2. `recurring_report`
   The lane is recurring oversight, scorecard, planning, or audit work.

3. `service_bootstrap`
   The first proof bar is a service health surface.

4. `implementation`
   The repo already has a reviewed slice and a deterministic proof command.

5. `bootstrap`
   Everything else.

This is the preferred decision order:

- child program? -> `orchestration`
- recurring governance? -> `recurring_report`
- service health first? -> `service_bootstrap`
- reviewed slice plus real proof command? -> `implementation`
- otherwise -> `bootstrap`

Only choose a different topology when the plan contains a concrete reason that
the catalog cannot represent honestly.

## Preferred implementation-family pattern

When create mode decides the repo is ready for an implementation-family lane,
be prescriptive. The default pattern should be:

1. `preflight` — tolerant proof probe
2. `implement` — bounded slice implementation
3. `verify` — deterministic must-pass proof
4. `quality` — deterministic evidence pack written to `quality.md`
5. `settle` — one strong-model settlement judgment that writes `promotion.md`
6. `audit` — final deterministic artifact and merge-readiness check

The implementation-family artifact set should normally be:

- `implementation.md`
- `verification.md`
- `quality.md`
- `promotion.md`

Do not create implementation lanes that rely only on `implementation.md` and
`verification.md` if the lane is expected to make merge-worthiness claims.

Also do not choose the implementation family too early. If the plan is still
creating the first proof surface, use `bootstrap` or `service_bootstrap` first
and let evolve promote the lane later.
