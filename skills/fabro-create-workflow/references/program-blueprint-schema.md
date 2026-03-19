# Program Blueprint Schema

The blueprint is the reviewable intermediate file between a broad repo request
and the final checked-in `fabro/` package.

At minimum the blueprint should contain:

- program id
- max parallelism
- optional state path and run dir
- optional doctrine and evidence inputs
- units
- unit artifact roots and artifact ids
- milestones and required artifacts
- lanes
- lane kind
- workflow family
- lane goal
- managed milestone
- dependencies
- produced artifacts
- optional proof, service, and orchestration state paths
- optional checks

The blueprint should be specific enough that a deterministic renderer can write
the package without guessing paths or semantics.

If something remains unresolved, record that fact explicitly instead of hiding
it in free-form prose.
