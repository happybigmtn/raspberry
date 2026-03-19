# Program Interview

Use this reference when the repo does not answer every synthesis question.

Ask only targeted questions. The repo should carry most of the burden.

## Ask only when the repo cannot infer safely

Good questions:

- is the first slice bootstrap, restart, or implementation?
- should the first user-visible proof be reviewed artifacts, tests, service
  health, or launch orchestration?
- when doctrine and the current package disagree, which should win first?

Bad questions:

- broad open-ended "how do you want this designed?"
- questions the specs or existing outputs already answer
- asking the user to re-list lanes that are already obvious from the repo

## Reconcile-mode questions

When updating an existing `fabro/` tree, ask only if doctrine and evidence
still leave ambiguity after inspection.

Examples:

- the run logs suggest splitting one lane in two, but the doctrine file is
  silent; should the split happen now or later?
- the current package says a lane is `platform`, but the doctrine reads more
  like a `service`; should the kind change now?

If a run log clearly shows a missing dependency or artifact contract, prefer
stating that finding over asking a vague design question.
