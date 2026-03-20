# Build skill-guided program synthesis and evolution above `fabro-create-workflow`

This ExecPlan is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

`PLANS.md` is checked into the repository root and this document must be
maintained in accordance with it. This plan depends on
`specs/031926-workflow-authoring-knowledge-boundary.md`,
`specs/031926-skill-guided-program-synthesis.md`, and
`specs/031826-raspberry-malinka-control-plane-port.md`.

## Purpose / Big Picture

After this slice lands, a contributor should be able to do two high-value
things without hand-authoring the whole `fabro/` tree:

1. start from a broad requirement or spec corpus and get a proposed
   supervised-repo package
2. point Fabro at an existing `fabro/` directory plus doctrine and run evidence
   and get a principled update proposal for the existing package

The system should inspect the repo, ask only the missing questions, write a
structured blueprint, and compile that blueprint into program manifests,
run configs, workflow graphs, and prompt/check scaffolding. In update mode, it
should first import the existing workflow tree, compare it to doctrine and run
evidence, and then render a deterministic patch.

The user-visible proof is a two-step flow for both create and evolve:

    fabro skill install --for project --dir claude --force

Then, using the installed `fabro-create-workflow` skill:

- on a broad request such as "build a craps game", the assistant writes a
  blueprint file in the target repo
- or on an update request such as "evolve `myosu/fabro` against `OS.md`,
  plans, and run evidence", the assistant writes an imported-and-revised
  blueprint for the existing package

After that:

    fabro synth create --blueprint fabro/blueprints/craps.yaml --target-repo .

or:

    fabro synth evolve \
      --blueprint fabro/blueprints/myosu.yaml \
      --target-repo /home/r/coding/myosu

The command writes a deterministic `fabro/` package whose generated manifests
and run configs pass Fabro/Raspberry validation, or a deterministic patch that
updates the existing package.

This first build is intentionally split in two. It does not require a brand-new
end-to-end "one command talks to the model and mutates the repo" feature on day
one. It ships a skill-guided blueprint path and a deterministic renderer first,
with evolve support built on the same blueprint model.

## Progress

- [x] (2026-03-19 06:48Z) Reviewed Fabro's current built-in authoring surface in
  `skills/fabro-create-workflow/` and the installer embedding in
  `lib/crates/fabro-cli/src/skill.rs`.
- [x] (2026-03-19 06:49Z) Reviewed the existing workflow-authoring boundary
  decision and confirmed that new synthesis work must extend, not replace,
  `fabro-create-workflow`.
- [x] (2026-03-19 06:51Z) Re-read Raspberry's current manifest contract in
  `lib/crates/raspberry-supervisor/src/manifest.rs` through the shipped skill
  references and nearby specs to keep the new build aligned with the real
  supervisory consumer.
- [x] (2026-03-19 06:53Z) Wrote the decision spec
  `specs/031926-skill-guided-program-synthesis.md`.
- [x] (2026-03-19 06:58Z) Updated the decision spec and this plan so the target
  product includes both greenfield synthesis and evidence-driven evolution
  of existing `fabro/` trees.
- [x] (2026-03-19 07:15Z) Added the new `fabro-synthesis` crate with a versioned
  blueprint schema, validation, importer, deterministic renderer, and a
  evolve report surface.
- [x] (2026-03-19 07:18Z) Added `fabro synth create` and
  `fabro synth evolve` to `fabro-cli`.
- [x] (2026-03-19 07:20Z) Extended `fabro-create-workflow` with blueprint-first
  create/evolve guidance and added new synthesis/evolution reference
  files to the built-in skill installer.
- [x] (2026-03-19 07:23Z) Added a toy `craps` synthesis fixture, an
  existing-package update fixture, library tests, CLI tests, and manual smoke
  runs for render and reconcile.
- [x] (2026-03-19 07:24Z) Updated the CLI docs to describe `fabro synth create`
  and `fabro synth evolve`.
- [x] (2026-03-19 07:36Z) Added `fabro synth import` so an existing program
  manifest can be exported into a blueprint file.
- [x] (2026-03-19 07:39Z) Added `--preview-root` to `fabro synth evolve` so the
  review-first loop can write an evolved package into a separate tree without
  mutating the live repo.
- [x] (2026-03-19 07:47Z) Extended evolve findings so doctrine files, evidence
  files, runtime state, and artifact presence are surfaced in the report.
- [x] (2026-03-19 07:52Z) Normalized imported repo-root paths so imported
  blueprints and preview manifests use clean paths instead of
  `fabro/programs/../../...` path fragments.
- [x] (2026-03-19 07:57Z) Taught evolve to preserve unchanged existing lane
  packages instead of regenerating generic workflow content for them.
- [x] (2026-03-19 08:02Z) Ran the first real review-first evolve pass against
  `/home/r/coding/myosu` on the `myosu-platform` program and inspected the
  generated blueprint, findings, and preview package.
- [x] (2026-03-19 08:11Z) Extended evolve findings with runtime-state-missing
  detection, artifact presence/missing reporting, and lightweight
  execution-candidate detection.
- [x] (2026-03-19 08:14Z) Ran a second real review-first evolve pass against
  `/home/r/coding/myosu` on the `myosu-product` program and confirmed that the
  report now identifies `agent:experience` as a likely next execution lane.
- [x] (2026-03-19 08:21Z) Added stale runtime-state detection when a lane is
  still marked active but all of its produced artifacts already exist.
- [x] (2026-03-19 08:22Z) Re-ran real review-first evolve passes against
  `myosu-product` and `myosu-platform`; product now surfaces a likely next
  execution candidate and platform now surfaces a stale `running` record.
- [x] (2026-03-19 08:29Z) Added an explicit recommendation layer to evolve
  output so the tool now says what to do next, not only what it observed.
- [x] (2026-03-19 08:31Z) Re-ran the real review-first Myosu passes and
  confirmed operator-quality recommendations: execute `agent:experience` next
  for product and clear the stale `games:multi-game` runtime record for
  platform.
- [x] (2026-03-19 08:38Z) Added `leave the package unchanged` recommendations
  when the workflow structure is already sound and only execution or runtime
  truth needs attention.
- [x] (2026-03-19 08:46Z) Refined the recommendation layer so it now distinguishes
  structural package changes from operator actions and can explicitly advise
  when to leave the workflow package unchanged.
- [x] (2026-03-19 08:49Z) Re-ran the real review-first passes and confirmed the
  tool now emits package-stability guidance plus next actions on Myosu:
  execute `agent:experience` next for product, and repair stale runtime truth
  for `games:multi-game` on platform.
- [x] (2026-03-19 08:57Z) Added review-artifact interpretation so evolve can
  recommend when a bootstrap lane should gain an implementation-family
  follow-on.
- [x] (2026-03-19 08:59Z) Re-ran the real review-first passes and confirmed
  evolve now recommends implementation-family follow-ons for `play:tui`,
  `games:multi-game`, `games:poker-engine`, and `sdk:core` when their review
  artifacts explicitly say they are ready.
- [x] (2026-03-19 09:07Z) Deepened the structural recommendation layer so
  evolve now identifies exact missing implementation-package paths under
  `fabro/run-configs/implement/` and `fabro/workflows/implement/`.
- [x] (2026-03-19 09:09Z) Re-ran the real review-first passes and confirmed the
  product now recommends concrete implementation-package additions for
  `play:tui`, `games:multi-game`, `games:poker-engine`, and `sdk:core`.
- [x] (2026-03-19 09:16Z) Added dependency-ordered implementation-package
  recommendations so evolve now suggests the sequence in which follow-on
  implementation packages should be created.
- [x] (2026-03-19 09:18Z) Re-ran the real review-first passes and confirmed the
  ordered structural advice on Myosu: `games:poker-engine -> games:multi-game -> sdk:core`
  for platform, and `play:tui` as the single implementation-package follow-on
  for product.
- [x] (2026-03-19 09:25Z) Extended the structural recommendation layer so
  evolve now recommends exact implementation program-manifest paths as well as
  run-config/workflow package paths.
- [x] (2026-03-19 09:27Z) Re-ran the real review-first passes and confirmed
  concrete implementation program recommendations on Myosu, including
  `myosu-play-tui-implementation.yaml`,
  `myosu-games-poker-engine-implementation.yaml`, and
  `myosu-sdk-core-implementation.yaml`.
- [x] (2026-03-19 09:35Z) Added blocker-aware review parsing so implementation
  follow-on recommendations are suppressed when the review artifact still marks
  the lane as blocked.
- [x] (2026-03-19 09:37Z) Re-ran the real review-first `myosu-services` pass
  and confirmed the tool now defers `validator:oracle` implementation until the
  named upstream blockers (`chain:pallet`, `games:poker-engine`, `miner:service`)
  clear instead of proposing a contradictory implementation package immediately.
- [x] (2026-03-19 09:45Z) Tightened blocker extraction to only keep real lane
  references from the current repo, removing noisy false blockers from review
  prose.
- [x] (2026-03-19 09:46Z) Re-ran the real review-first `myosu-services` pass
  and confirmed the blocker recommendation is now clean and repo-shaped:
  `chain:pallet`, `games:poker-engine`, `miner:service`.
- [x] (2026-03-19 09:55Z) Added first contract-tightening recommendations so
  evolve can propose stronger dependency/check gates when blocked reviews name
  upstream lanes that are not encoded strongly enough in the current package.
- [x] (2026-03-19 09:57Z) Re-ran the real review-first `myosu-services` pass
  and confirmed the tool now recommends tightening `validator:oracle` with a
  dependency on `miner:service` reviewed and a precondition check on
  `outputs/games/poker-engine/review.md`.
- [x] (2026-03-19 10:08Z) Added blocked-review stage parsing so evolve can
  extract specific prerequisite language like `Slice 5` and `Phase 2` from
  review artifacts instead of only generic blocker names.
- [x] (2026-03-19 10:11Z) Re-ran the real review-first `myosu-services`,
  `myosu-product`, and `myosu-platform` passes and confirmed the services
  advice now includes milestone-refinement recommendations for
  `games:poker-engine`, `chain:pallet`, and `miner:service`.
- [x] (2026-03-19 10:24Z) Taught evolve to materialize the safest blocked-review
  contract tightening directly into the preview package by adding missing
  same-program dependencies and cross-program review-file checks.
- [x] (2026-03-19 10:27Z) Re-ran the real review-first `myosu-services` pass
  and confirmed the preview manifest now picks up the `miner:service`
  dependency and `games:poker-engine` review gate without introducing a bogus
  self-dependency.
- [x] (2026-03-19 10:38Z) Taught evolve to render review-approved
  implementation follow-on packages directly into the preview tree as
  one-lane implementation programs with manifests, run-configs, workflows, and
  prompts.
- [x] (2026-03-19 10:42Z) Re-ran the real review-first `myosu-product` and
  `myosu-platform` passes and confirmed the preview tree now includes
  `myosu-play-tui-implementation.yaml`,
  `myosu-games-poker-engine-implementation.yaml`,
  `myosu-games-multi-game-implementation.yaml`, and
  `myosu-sdk-core-implementation.yaml` plus their `implement/` workflow
  packages.
- [x] (2026-03-19 10:53Z) Taught generated implementation follow-ons to inherit
  source-lane preconditions, synthesize dependency artifact checks, and mine
  proof commands from reviewed markdown into the generated verify script and
  proof checks.
- [x] (2026-03-19 10:56Z) Re-ran the real review-first `myosu-product` and
  `myosu-platform` passes and confirmed the generated implementation manifests
  now carry inherited preconditions and command-backed proof checks, while the
  generated `implement/` workflows use the mined proof commands as their verify
  node script.
- [x] (2026-03-19 11:07Z) Upgraded generated implementation workflows from a
  straight-line shape to a richer execution loop with preflight, verify, fixup,
  review, and artifact-audit stages.
- [x] (2026-03-19 11:10Z) Re-ran the real review-first `myosu-product` and
  `myosu-platform` passes and confirmed the generated implementation prompts
  now carry slice/manual-proof context, while the generated `implement/`
  workflows use the richer preflight/fixup/audit shape.
- [x] (2026-03-19 11:19Z) Tightened slice-note extraction so generated
  implementation prompts keep the strongest ordering constraints without
  dragging in unrelated table rows or broad prose.
- [x] (2026-03-19 11:22Z) Added a service-oriented implementation workflow path
  that can insert a dedicated health gate when reviewed evidence yields a real
  health command such as `curl .../health`.
- [x] (2026-03-19 11:31Z) Reworked implementation prompt slice-note rendering
  into structured execution guidance (`Start`, `Order`, `Parallel`, `Scope`)
  instead of raw review excerpts.
- [x] (2026-03-19 11:33Z) Re-ran the real review-first `myosu-product` pass and
  confirmed the `play:tui` implementation prompt now keeps the manual proof
  note while dropping the noisier slice-order clutter. The new service health
  gate path is covered by unit tests and ready for the first eligible live
  service candidate.
- [x] (2026-03-19 11:41Z) Added first-slice extraction from reviewed/spec
  markdown so generated implementation prompts now include an explicit
  `Implement now` instruction for the first concrete slice.
- [x] (2026-03-19 11:43Z) Re-ran the real review-first `myosu-product` and
  `myosu-platform` passes and confirmed the generated prompts now surface
  concrete first-slice starts such as `Slice 1: myosu-play binary skeleton`
  and `Slice 1: crate skeleton`.
- [x] (2026-03-19 11:52Z) Improved first-slice extraction to prefer more
  specific spec slice headers over generic review prose and added a `First proof
  gate` prompt section sourced from the earliest proof command in the lane
  artifacts.
- [x] (2026-03-19 11:54Z) Re-ran the real review-first `myosu-product` and
  `myosu-platform` passes and confirmed the prompts now say
  `Slice 1: myosu-play Binary Skeleton + Shell Wiring` and
  `Slice 1: Create myosu-games-poker Crate Skeleton`, plus a concrete first
  proof gate for each lane.
- [x] (2026-03-19 12:02Z) Added first code-surface extraction from Slice 1 spec
  sections so generated implementation prompts now tell the operator which
  crate/file to touch first.
- [x] (2026-03-19 12:04Z) Re-ran the real review-first `myosu-product` and
  `myosu-platform` passes and confirmed the prompts now include `Touch first`
  guidance such as `crates/myosu-play/` and
  `crates/myosu-games-poker/Cargo.toml`.
- [x] (2026-03-19 12:14Z) Added extraction of slice-1 setup work from spec
  sections so generated implementation prompts can surface workspace,
  dependency, and feature-flag setup before coding begins.
- [x] (2026-03-19 12:16Z) Re-ran the real review-first `myosu-product` and
  `myosu-platform` passes and confirmed the `games:poker-engine` prompt now
  includes a `Set up first` block covering workspace membership, robopoker git
  rev pinning, serde feature enablement, and crate-type setup.
- [x] (2026-03-19 12:26Z) Added extraction of Slice 1 `What` content from spec
  sections and threaded it into the generated implementation prompts as a
  `Build in this slice` block.
- [x] (2026-03-19 12:28Z) Re-ran the real review-first `myosu-product` and
  `myosu-platform` passes and confirmed the `play:tui` prompt now includes the
  concrete Slice 1 work description, while `games:poker-engine` keeps the
  stronger setup-first guidance.
- [x] (2026-03-19 12:39Z) Reworked generated implementation `review` and
  `fixup` prompts so they now surface the active slice, touched surfaces, setup
  checks, first proof gate, and execution guidance as structured sections
  instead of repeating a raw lane-wide context block.
- [x] (2026-03-19 12:41Z) Re-ran the real review-first `play:tui` and
  `games:poker-engine` previews and confirmed the generated review/fixup prompts
  now stay anchored to the current slice without duplicating the full contract
  block.
- [x] (2026-03-19 12:50Z) Added slice-specific implementation and verification
  artifact expectation sections to generated implementation prompts.
- [x] (2026-03-19 12:52Z) Re-ran the real review-first `play:tui` preview and
  confirmed the generated implementation plan prompt now says what
  `implementation.md` and `verification.md` should cover for the active slice,
  not just what the lane should do.
- [x] (2026-03-19 13:02Z) Reworked generated implementation `review` and
  `fixup` prompts so they now carry the same slice-specific artifact
  expectations as the plan prompt, without repeating the full raw contract
  block.
- [x] (2026-03-19 13:04Z) Re-ran the real review-first `play:tui` and
  `games:poker-engine` previews and confirmed the generated review/fixup prompts
  now stay anchored to the active slice, touched surfaces, setup checks, proof
  gate, and artifact quality bar.
- [x] (2026-03-19 13:12Z) Extended the generated implementation prompt contract
  so service lanes can surface `First health gate` and service-health artifact
  expectations once the reviewed evidence exposes them.
- [x] (2026-03-19 13:14Z) Added service-follow-on synth fixtures and end-to-end
  library/CLI assertions so the service health-gate path is now proven through
  a real rendered preview package, not only unit tests.
- [x] (2026-03-19 13:23Z) Extended the rendered service-follow-on prompts and
  artifact expectations to carry observability surfaces in addition to
  health-first bringup guidance.
- [x] (2026-03-19 13:25Z) Re-ran the synth library/CLI suites and confirmed the
  dedicated service-follow-on fixture now proves a rendered service package with
  health-gate workflow, health-aware prompts, and observability-aware review
  sections.
- [x] (2026-03-19 13:34Z) Extended the service-follow-on fixture and prompt
  extraction so rendered service packages now also carry service-shaped start
  and parallel execution guidance from the reviewed artifact.
- [x] (2026-03-19 13:36Z) Re-ran the synth library/CLI suites and confirmed the
  dedicated service-follow-on fixture now proves a rendered service package with
  health-gate workflow, health and observability sections, and service-shaped
  execution guidance in the generated prompts.
- [x] (2026-03-19 10:11Z) Deepened structural recommendations beyond first
  dependency tightening into initial milestone-refinement suggestions driven by
  blocked review artifacts.
- [ ] Deepen doctrine and run-evidence reasoning from file presence and text
  support into stronger structural recommendations such as stale runtime-state
  correction, dependency tightening, and milestone refinement.
- [ ] Turn milestone-refinement findings into concrete package-evolution edits,
  not only recommendations, while preserving review-first preview safety.
- [ ] Validate the full evolve path on more than one real Myosu program,
  including a program that is not already structurally complete.

## Surprises & Discoveries

- Observation: the current built-in skill already reaches all the way to
  Raspberry lane contracts, but it stops short of repo-level package synthesis.
  Evidence: `skills/fabro-create-workflow/SKILL.md` now explicitly covers
  "map repo work into units and lanes" and Raspberry lane contracts, but the
  shipped references still culminate in authoring advice rather than a
  structured intermediate file or renderer.

- Observation: the hardest work in Myosu was decomposition, not syntax.
  Evidence: the broad `fabro/` tree in Myosu was authored through repeated
  interactive review of units, lanes, milestones, proof surfaces, and
  dependencies; the raw `.fabro` and `.toml` syntax was the easy part once that
  decomposition existed.

- Observation: the more valuable long-term product is likely workflow
  evolution, not greenfield generation by itself.
  Evidence: once Myosu had a real `fabro/` tree, the next work shifted from
  inventing files to revising lane order, dependencies, milestones, and proof
  posture in response to doctrine and actual run outcomes.

- Observation: reviewed bootstrap artifacts are now strong enough to drive
  structural follow-on advice.
  Evidence: the real Myosu passes now recommend implementation-family
  follow-ons for `play:tui`, `games:multi-game`, `games:poker-engine`, and
  `sdk:core` based directly on the checked-in `review.md` text.

- Observation: concrete package-path advice is a real quality jump over generic
  follow-on advice.
  Evidence: the latest Myosu passes now recommend exact missing paths like
  `fabro/run-configs/implement/play-tui.toml` and
  `fabro/workflows/implement/play-tui.fabro`, which is directly actionable for
  a repo maintainer.

- Observation: implementation program-manifest advice is another quality jump,
  because it describes the whole next control-plane package rather than just
  two files.
  Evidence: the latest Myosu passes now recommend concrete manifests like
  `myosu-play-tui-implementation.yaml`,
  `myosu-games-poker-engine-implementation.yaml`, and
  `myosu-sdk-core-implementation.yaml`.

- Observation: contract-tightening advice becomes more trustworthy once blocker
  extraction is filtered to real lane names.
  Evidence: the latest `myosu-services` pass now recommends a dependency on
  `miner:service` reviewed and a precondition check on
  `outputs/games/poker-engine/review.md`, which are both concrete and
  repo-shaped, instead of noisy pseudo-blockers from prose.

- Observation: blocked review artifacts already contain milestone-shaping
  evidence, not just blocker names.
  Evidence: `outputs/validator/oracle/review.md` explicitly says
  `games:poker-engine` must reach Slice 5, `chain:pallet` must reach at least
  Phase 2, and `miner:service` must reach Slice 3 before honest implementation
  can start. Parsing those lines lets evolve recommend upstream milestone
  refinement instead of only saying "wait on blockers".

- Observation: the first safe auto-apply path is downstream contract tightening,
  not upstream milestone synthesis.
  Evidence: after materializing the `miner:service` dependency and
  `games:poker-engine` review-file check into the `myosu-services` preview,
  the evolved manifest changed in a useful, reviewable way while the upstream
  milestone-refinement work remained recommendation-only.

- Observation: rendering the next implementation packages into preview makes the
  product feel materially more real than recommendation-only follow-on advice.
  Evidence: the latest live `myosu-product` and `myosu-platform` passes now
  emit concrete preview packages for `play:tui`, `games:poker-engine`,
  `games:multi-game`, and `sdk:core`, not just console text about what files
  should exist.

- Observation: reviewed markdown is already rich enough to drive proof surfaces
  for implementation follow-ons.
  Evidence: `play:tui` and `games:poker-engine` reviews include explicit proof
  command blocks, and the latest preview implementation manifests now carry
  those commands as proof checks while the generated workflows use them for the
  verify step.

- Observation: the generated implementation packages feel much less generic
  once the prompt context carries slice ordering and manual-proof notes from the
  reviewed artifacts.
  Evidence: the latest `play:tui` preview prompt now carries the manual
  showdown proof note and Phase B constraints, while the latest
  `games:poker-engine` preview prompt carries the Slice 1 → Slice 3 ordering
  constraint from the reviewed lane artifact.

- Observation: service-style implementation workflows need a different
  execution shape only when the evidence yields a concrete health command.
  Evidence: the latest generator now keeps the normal implementation loop by
  default, but it can insert a dedicated health gate for service lanes when the
  review contains a real command like `curl http://{ip}:{port}/health`.

- Observation: tighter extraction can improve prompt quality by removing
  low-signal guidance entirely rather than always forcing a slice-order block.
  Evidence: after the latest refinement, the live `play:tui` implementation
  prompt keeps the manual showdown proof note but drops the earlier noisy
  slice-order excerpts, which reads much more like a real lane instruction.

- Observation: the prompt becomes much more actionable once the first slice is
  named explicitly instead of only implied by proof commands or ordering notes.
  Evidence: the latest live `play:tui` and `games:poker-engine` previews now
  include `Implement now` blocks that point at `myosu-play binary skeleton` and
  `crate skeleton` respectively, which is much closer to an executable lane
  contract.

- Observation: preferring spec slice headers over review summaries produces more
  useful implementation starts.
  Evidence: the latest live previews now say
  `Slice 1: myosu-play Binary Skeleton + Shell Wiring` and
  `Slice 1: Create myosu-games-poker Crate Skeleton`, which are clearly better
  than the earlier generic `binary skeleton` / `crate skeleton` prompts.

- Observation: adding the first code surface makes the generated implementation
  prompt read much more like an actual starting ticket.
  Evidence: the latest live previews now pair `Implement now` with
  `Touch first`, e.g. `crates/myosu-play/` for `play:tui` and
  `crates/myosu-games-poker/Cargo.toml` for `games:poker-engine`.

- Observation: surfacing slice-1 setup work is especially valuable for
  library-style implementation lanes where workspace membership, dependencies,
  or feature flags are the real first move.
  Evidence: the latest `games:poker-engine` prompt now includes a `Set up
  first` block with workspace-membership, dependency-rev, serde-feature, and
  crate-type setup notes before the proof gate.

- Observation: surfacing the Slice 1 `What` description makes the prompt feel
  much closer to a concrete implementation ticket than file/proof guidance
  alone.
  Evidence: the latest `play:tui` prompt now says not just where to start, but
  also what to build there: a bare `main.rs` with `--train`, a hardcoded
  `NlheRenderer`, and Shell wiring.

- Observation: implementation review/fixup prompts read much better once the
  raw lane-wide contract block is replaced with structured slice-specific
  sections.
  Evidence: the latest live `play:tui` and `games:poker-engine` review/fixup
  prompts now show `Current slice`, `Touched surfaces`, `Setup checks`, and
  `First proof gate` directly, without duplicating the entire plan prompt
  context first.

- Observation: generated implementation prompts become more execution-ready once
  they define what the slice-complete artifacts should actually say.
  Evidence: the latest live `play:tui` plan prompt now includes
  `Implementation artifact must cover` and `Verification artifact must cover`
  sections tied to the active slice and first proof gate.

- Observation: implementation review/fixup prompts are materially better once
  they inherit the same artifact-quality expectations as the plan prompt.
  Evidence: the latest live `play:tui` and `games:poker-engine` review/fixup
  prompts now tell the agent not just what slice is active, but also what the
  resulting `implementation.md` and `verification.md` must say for that slice.

- Observation: service implementation candidates need their own artifact quality
  bar around health-first bringup, not just code-proof expectations.
  Evidence: the miner review explicitly names `/health` as the first operator
  surface and describes health checks that should appear in later bringup
  stages, and the new service-follow-on fixture now proves that the generator
  emits a rendered implementation workflow with a `Health` node plus service
  review prompts that carry the health sections.

- Observation: service implementation packages feel more honest once they carry
  observability surfaces as part of the bringup contract, not just health
  checks.
  Evidence: the updated service-follow-on fixture now proves review prompts that
  carry structured-log observability guidance alongside the health gate and
  health-surface expectations.

- Observation: service implementation prompts benefit from service-shaped
  execution guidance, not just generic slice-order rules.
  Evidence: the updated service-follow-on fixture now exercises prompt output
  that captures `Start slices 1 and 3 immediately` and `Parallelize...` style
  guidance from the miner review rather than only generic `must precede`
  ordering lines.

- Observation: blocked-review parsing must exclude the current lane key before
  preview mutation or evolve can invent bogus self-dependencies.
  Evidence: the first auto-tightening pass briefly added
  `validator:oracle -> validator@reviewed` because the review text mentioned the
  lane itself. Filtering the current lane key out of blocked-review refs fixed
  the preview manifest immediately.

- Observation: blocker extraction must be filtered against real repo lane names
  or the advice becomes noisy and untrustworthy.
  Evidence: an intermediate `myosu-services` pass treated tokens like
  `AC-OR-03` and `chain::tests::connect_and_query` as blockers. After filtering
  against known lane references in the repo, the blocker list reduced cleanly
  to `chain:pallet`, `games:poker-engine`, and `miner:service`.

- Observation: blocked-lane advice must override missing-package advice or the
  tool becomes self-contradictory.
  Evidence: an intermediate `myosu-services` pass simultaneously recommended
  adding `validator:oracle` implementation files and deferring that work due to
  upstream blockers. Filtering implementation-package creation behind the
  review-blocked signal fixed that contradiction.

- Observation: implementation-package advice becomes meaningfully stronger when
  it preserves dependency order instead of listing missing packages flatly.
  Evidence: the latest `myosu-platform` pass now recommends the sequence
  `games:poker-engine -> games:multi-game -> sdk:core`, which matches the
  existing reviewed milestone dependencies in the package.

- Observation: the existing skill installer and Raspberry manifest model give
  this build a strong starting boundary.
  Evidence: `lib/crates/fabro-cli/src/skill.rs` already ships the authoring
  corpus, and `skills/fabro-create-workflow/references/raspberry-authoring.md`
  already describes the manifest-first questions the blueprint must answer.

- Observation: doctrine and run truth need to become first-class synthesis
  inputs rather than only advisory context.
  Evidence: the desired update behavior is specifically "read `OS.md`, read the
  current `fabro/` tree, read run evidence, then revise the workflow package"
  rather than merely "author a new lane from prose."

- Observation: the first useful evolve implementation can ship before full
  doctrine or run-log interpretation exists.
  Evidence: the current importer + renderer already make it possible to import
  an existing `fabro/` tree, compare lane structure, and deterministically add
  or remove lanes through `fabro synth evolve`, as shown by the update
  fixture and CLI smoke run.

- Observation: isolate-and-copy fixtures are mandatory for evolve tests.
  Evidence: an early version of the library reconcile test mutated the checked-in
  update fixture in place, which made later manual reconcile runs falsely report
  "already matches blueprint structure". Copying the fixture into a temp repo
  fixed that and made the tests honest again.

- Observation: imported repo paths must be normalized immediately or the
  generated blueprint and preview manifest become hard to trust.
  Evidence: the first real `myosu-platform` import produced values like
  `fabro/programs/../../.raspberry/...`, which were structurally correct but
  obviously wrong-looking. Normalizing the imported relative paths fixed that.

- Observation: evolve quality depends on preserving good current lane packages,
  not only on emitting valid new ones.
  Evidence: the first real `myosu-platform` preview regenerated generic lane
  files for already-good lanes. After teaching evolve to preserve unchanged lane
  packages, the preview kept the existing `poker-engine` workflow file exactly.

- Observation: the first real Myosu evolve pass is already useful as a
  supervisory diagnostic even before doctrine semantics are deeper.
  Evidence: the `myosu-platform` preview reported that doctrine and evidence
  files were found, that all platform artifacts were already present, and that
  `.raspberry/myosu-platform-state.json` still reports `games:multi-game` as
  `running`, which is now a concrete stale-state quality signal.

- Observation: imported check-path normalization is necessary before evolve can
  surface next-step execution candidates from real repos.
  Evidence: the first `myosu-product` pass only reported missing artifacts.
  After normalizing imported `checks` paths to repo-relative form, the same
  review-first evolve pass correctly reported `agent:experience` as ready for
  execution.

- Observation: review-first preview mode is already good enough to use as a
  live repo diagnostic loop even before auto-apply is trusted.
  Evidence: `fabro synth import` plus `fabro synth evolve --preview-root ...`
  against `/home/r/coding/myosu` now produces concrete operator-facing findings
  like `runtime state missing` and `lane agent:experience appears ready for
  execution` without mutating the live package.

- Observation: the real Myosu loop is now surfacing both forward-progress and
  cleanup recommendations.
  Evidence: `myosu-product` reports `agent:experience` as a likely next
  execution lane, while `myosu-platform` reports that
  `games:multi-game` is still marked `running` even though all managed
  artifacts already exist.

- Observation: the recommendation layer is where the product starts to feel
  operational rather than merely descriptive.
  Evidence: the same real Myosu evolve passes now emit specific next actions:
  `execute agent:experience next` for `myosu-product`, and
  `refresh or clear the stale runtime record for games:multi-game` for
  `myosu-platform`.

- Observation: some evolve passes should end with "do not change the package"
  rather than more edits.
  Evidence: `myosu-product` now recommends leaving the workflow package
  unchanged and executing `agent:experience`, while `myosu-platform` now
  recommends leaving the workflow package unchanged and repairing runtime truth
  for `games:multi-game`.

- Observation: the first strategic step is often deciding that the package is
  already structurally correct.
  Evidence: on both `myosu-product` and `myosu-platform`, the current best
  recommendation is not "rewrite the workflow tree". It is "leave the package
  unchanged" plus a specific operational next action.

## Decision Log

- Decision: keep the authoring intelligence in the existing
  `fabro-create-workflow` skill and add a new blueprint compiler beneath it.
  Rationale: the earlier boundary decision already chose the canonical prompt
  corpus. The missing piece is deterministic file emission, not a second skill.
  Date/Author: 2026-03-19 / Codex

- Decision: make the first shipped product a two-step flow: skill-authored
  blueprint first, deterministic renderer second.
  Rationale: this creates a reviewable artifact between the interview and the
  checked-in file tree, which is safer and easier to test than a single opaque
  end-to-end mutation path.
  Date/Author: 2026-03-19 / Codex

- Decision: ask only targeted clarifying questions and require repo inspection
  before asking them.
  Rationale: broad requirements are useful precisely because the repo often
  already answers most of the decomposition questions. The system should only
  spend user attention on the gaps.
  Date/Author: 2026-03-19 / Codex

- Decision: validate the first implementation on both a toy "broad requirement"
  fixture and a real Raspberry-shaped fixture.
  Rationale: a toy fixture proves the dialogue and renderer work from sparse
  input, while a Myosu-shaped fixture proves the emitted files line up with the
  current supervisory contract.
  Date/Author: 2026-03-19 / Codex

- Decision: build create and evolve on the same blueprint model rather than
  as separate feature tracks.
  Rationale: the same lane, milestone, dependency, and proof semantics are
  needed both to create a package from scratch and to revise an existing one.
  Date/Author: 2026-03-19 / Codex

- Decision: treat doctrine files and run evidence as first-class evolve
  inputs.
  Rationale: updating an existing workflow tree is only meaningful if the
  system can explain why doctrine or evidence implies a structural change.
  Date/Author: 2026-03-19 / Codex

- Decision: ship a first evolve slice that compares current package structure
  to the blueprint, even before full doctrine/evidence semantics are encoded.
  Rationale: the importer/renderer/reconcile loop is valuable and testable on
  its own, and it creates the deterministic substrate that richer doctrine and
  run-truth reasoning can build on next.
  Date/Author: 2026-03-19 / Codex

- Decision: add `synth import` and `evolve --preview-root` before attempting a
  real Myosu evolve pass.
  Rationale: review-first iteration on a live repo is much safer if the current
  package can be captured into a blueprint and the evolved package can be
  rendered into a separate preview tree.
  Date/Author: 2026-03-19 / Codex

- Decision: preserve unchanged current lane packages during evolve rather than
  regenerating them from generic templates.
  Rationale: the real Myosu pass showed that deterministic structural validity
  is not enough; the evolver must avoid degrading already-good human-authored
  workflow content when a lane is unchanged.
  Date/Author: 2026-03-19 / Codex

- Decision: treat `synth import` plus `synth evolve --preview-root` as the
  default proving loop for real repos until auto-apply quality is much higher.
  Rationale: the real Myosu passes are already yielding valuable diagnostic
  findings, and preview mode keeps the loop safe while the doctrine/evidence
  reasoning is still maturing.
  Date/Author: 2026-03-19 / Codex

- Decision: treat "required before implementation" lines inside blocked review
  artifacts as first-class structural evidence for milestone refinement.
  Rationale: those lines describe the real downstream gate much more precisely
  than a coarse `reviewed` milestone or a generic review-file precondition, so
  they are the right next bridge from prose evidence into contract advice.
  Date/Author: 2026-03-19 / Codex

- Decision: auto-apply only the safest evidence-driven contract tightening into
  the preview package for now: same-program milestone dependencies and
  cross-program review-file precondition checks.
  Rationale: those edits are deterministic, easy to explain, and already map to
  existing manifest primitives, while upstream milestone redesign still needs
  review-first human judgment.
  Date/Author: 2026-03-19 / Codex

- Decision: render review-approved implementation follow-ons as standalone
  preview programs rather than only as recommendations.
  Rationale: the user-facing promise is whole workflow-package synthesis, not
  prose advice. Implementation follow-ons already map cleanly onto one-lane
  implementation programs, so they are a good next structural edit to make
  real.
  Date/Author: 2026-03-19 / Codex

- Decision: implementation follow-ons should inherit source-lane preconditions
  and mined proof commands instead of starting as empty implementation shells.
  Rationale: a generated implementation package is much more useful when it
  already carries the upstream review gates and proof commands that the curated
  lane artifacts explicitly require.
  Date/Author: 2026-03-19 / Codex

- Decision: generated implementation workflows should use a richer default
  shape than a simple implement->review->verify chain.
  Rationale: implementation work benefits from preflight proof discovery,
  fixup loops after verification failure, and an explicit artifact audit before
  exit. That structure is closer to how real Myosu implementation lanes already
  behave.
  Date/Author: 2026-03-19 / Codex

- Decision: tighten slice-note extraction toward numbered ordering constraints
  and explicit phase notes, while excluding unrelated table rows and general
  discussion.
  Rationale: the goal is prompt signal, not prompt volume. The first pass was
  useful but still too noisy on `play:tui`.
  Date/Author: 2026-03-19 / Codex

- Decision: keep the new service health-gate path in the generator even before
  the first live Myosu service implementation candidate is ready.
  Rationale: the miner review already provides the right kind of health-first
  evidence, and the unit-tested path is cheaper to keep live than to rebuild
  later when the first service implementation follow-on becomes eligible.
  Date/Author: 2026-03-19 / Codex

- Decision: extract and surface the first concrete slice as an explicit prompt
  instruction whenever the reviewed or spec markdown provides enough evidence.
  Rationale: constraints and proof expectations tell the agent what must stay
  true, but the next productivity jump comes from telling it exactly what slice
  to start with.
  Date/Author: 2026-03-19 / Codex

- Decision: prefer the more specific spec slice header over review prose when
  both are available, and pair that with the earliest proof gate in the
  artifacts.
  Rationale: the spec usually names the slice more precisely, while the first
  proof gate turns that slice label into a concrete starting checkpoint.
  Date/Author: 2026-03-19 / Codex

- Decision: surface the first code surface to touch whenever the Slice 1 spec
  section exposes a `File` or `Files` field.
  Rationale: once the prompt names the first slice and first proof gate, the
  next productivity gain is telling the implementation lane where to start
  editing.
  Date/Author: 2026-03-19 / Codex

- Decision: surface slice-1 setup notes from spec sections when they describe
  prerequisite workspace or dependency work before the first code edit.
  Rationale: some lanes do not really start with a Rust module implementation;
  they start with workspace membership, dependency declarations, or feature
  flags, and the prompt should say so explicitly.
  Date/Author: 2026-03-19 / Codex

- Decision: surface the Slice 1 `What` description when the spec provides it,
  and prefer that for slice-specific implementation guidance over generic lane
  prose.
  Rationale: once the prompt names the slice, file, setup, and proof gate, the
  next productivity gain is describing the concrete unit of work the agent
  should build in that slice.
  Date/Author: 2026-03-19 / Codex

- Decision: generated implementation review/fixup prompts should summarize the
  current slice contract as structured sections rather than repeating the full
  raw prompt context block.
  Rationale: fixup loops need fast, high-signal scoping more than they need a
  verbatim copy of the plan prompt, and the structured sections are easier for
  an agent to honor during retries.
  Date/Author: 2026-03-19 / Codex

- Decision: generated implementation plan prompts should explicitly define the
  expected contents of the slice-complete `implementation.md` and
  `verification.md` artifacts.
  Rationale: a lane package is stronger when it constrains not just the coding
  work but also the durable artifact contract produced by that work.
  Date/Author: 2026-03-19 / Codex

- Decision: generated implementation review/fixup prompts should inherit the
  same slice-specific artifact expectations as the plan prompt while avoiding a
  duplicated raw contract block.
  Rationale: retry and review loops are most effective when they stay scoped to
  the active slice and its artifact-quality bar instead of reconstructing that
  contract from scratch on every pass.
  Date/Author: 2026-03-19 / Codex

- Decision: keep developing the service-specific implementation prompt path
  ahead of the first live eligible Myosu service candidate, and validate it via
  a dedicated rendered service fixture until a real Myosu preview case exists.
  Rationale: the miner/validator reviews already provide the right health-first
  evidence shape, and the dedicated fixture gives us an honest rendered-package
  proof surface even before the first live service implementation follow-on
  becomes available.
  Date/Author: 2026-03-19 / Codex

- Decision: treat observability surfaces as part of the service implementation
  artifact contract, not just optional commentary.
  Rationale: for long-running services, operator-facing logs and health signals
  are part of whether a slice is honestly shippable, so the generated prompts
  should ask for them explicitly whenever the source review provides that
  evidence.
  Date/Author: 2026-03-19 / Codex

- Decision: keep broadening the service fixture until it exercises the same
  kinds of service-shaped cues we expect from a real Myosu service follow-on,
  including start/parallel execution guidance.
  Rationale: until the first live Myosu service implementation candidate is
  eligible, the dedicated rendered fixture is the best place to harden these
  service-specific synthesis behaviors end to end.
  Date/Author: 2026-03-19 / Codex

## Outcomes & Retrospective

This plan started before any blueprint or renderer code existed. The first real
slice has now landed:

- `fabro-synthesis` provides a blueprint schema, validation, import, render,
  and evolve primitives
- `fabro-cli` exposes `fabro synth import`, `fabro synth create`, and
  `fabro synth evolve`
- the built-in skill now explains blueprint-first create and evolve modes
- fixtures and tests cover a toy create case and an existing-package update
  case
- real Myosu review-first passes now produce explicit operator recommendations
  rather than only descriptive findings

What remains is the richer half of evolve: importing doctrine and run
evidence strongly enough to justify lane reordering, milestone changes, proof
surface changes, and stale-state correction from real repo history rather than
only from blueprint structure.

The most important new outcome from this round is that the real Myosu loop is
no longer hypothetical. The current tool can:

- import a live program into a blueprint
- evolve it into a separate preview tree
- preserve unchanged lane packages
- surface missing runtime-state and artifact-state findings
- identify at least one likely next execution candidate on a real program
- emit an operator-quality next-step recommendation from those findings
- distinguish between "change the package" and "leave the package alone; act on
  execution/runtime truth instead"
- recommend exact missing implementation-package paths for bootstrap lanes whose
  review artifacts say implementation is ready
- recommend the order in which those implementation packages should be added
  when multiple follow-ons are possible
- recommend the exact implementation program-manifest file to introduce for each
  follow-on package
- suppress those implementation-package recommendations when the reviewed lane
  still declares upstream blockers, and explain which blockers must clear first
- extract specific stage requirements from blocked reviews and recommend when an
  upstream lane needs a stronger milestone than its current coarse
  `reviewed`-style contract
- materialize the safest blocked-review contract tightening directly into the
  preview package so review-first evolve is starting to emit real package edits,
  not only prose guidance
- render review-approved implementation follow-ons directly into the preview
  tree so evolve emits a fuller executable package for the next frontier, not
  only the current program plus recommendations
- carry proof and precondition surfaces from reviewed lane evidence into the
  generated implementation packages so the preview output is closer to something
  we could actually execute

That is enough to support a safe iterative refinement loop against Myosu while
the deeper structural reasoning is still being built.

The target outcome is not another advisory document. It is a demonstrable path:

- from "broad requirement plus repo context" to "reviewable blueprint plus
  generated `fabro/` package"
- and from "existing `fabro/` tree plus doctrine plus run evidence" to
  "reviewable blueprint revision plus deterministic workflow patch"

If that lands, Fabro gains a real onboarding story and a real steering story
for supervised repositories instead of asking every adopter to rediscover the
same decomposition work manually and then maintain it by hand forever.

## Context and Orientation

Fabro's current shipped authoring intelligence lives in:

- `skills/fabro-create-workflow/SKILL.md`
- `skills/fabro-create-workflow/references/dot-language.md`
- `skills/fabro-create-workflow/references/run-configuration.md`
- `skills/fabro-create-workflow/references/example-workflows.md`
- `skills/fabro-create-workflow/references/raspberry-authoring.md`
- `skills/fabro-create-workflow/references/raspberry-examples.md`

Those files are embedded into the CLI installer in:

- `lib/crates/fabro-cli/src/skill.rs`

Raspberry's current supervisory contract lives in:

- `lib/crates/raspberry-supervisor/src/manifest.rs`
- `lib/crates/raspberry-supervisor/src/evaluate.rs`

In Fabro terms, a **workflow package** is the checked-in set of files a repo
needs to run one lane or unit of work: `.fabro` graph, run-config TOML, prompt
files, checks, and stable paths. In Raspberry terms, a **program manifest** is
the checked-in YAML file that names units, lanes, milestones, dependencies,
proof contracts, and state surfaces. In this plan, a **blueprint** is the
structured design file that sits between a broad requirement and the final
checked-in workflow package plus program manifest.

The blueprint is needed because the current skill knows how to help author
files, but there is no durable, reviewable intermediate file that captures the
repo-level decomposition decisions. Without that intermediate file, broad
requirements such as "build a craps game" still force the user or the assistant
to free-write the entire `fabro/` tree.

For existing repos, a second gap exists: there is no productized way to import
the current workflow package, compare it to doctrine and run evidence, and then
patch it coherently. A reconcile path needs:

- an importer for current `fabro/` files
- a doctrine reader for files such as `OS.md`
- a run-evidence reader for `outputs/`, `.raspberry/`, and `~/.fabro/runs/`
- a reconciler that can describe and render the needed structural changes

The implementation should therefore introduce a deterministic bridge between
two existing facts:

1. the skill already knows how to think about workflows and Raspberry lanes
2. the repo still needs stable, regenerable checked-in files and stable,
   explainable updates to those files over time

## Milestones

### Milestone 1: Define a stable blueprint format

At the end of this milestone, Fabro has a versioned blueprint schema and
validation logic. A novice can read one blueprint file and understand the
program, units, lanes, artifact contracts, milestones, dependencies, proof
surfaces, unresolved questions, and, in evolve mode, imported current-state
plus doctrine/evidence findings. The proof is a checked-in fixture blueprint
plus automated parsing/validation tests.

### Milestone 2: Teach the built-in skill to author and revise blueprints

At the end of this milestone, `fabro-create-workflow` can operate in a
program-synthesis mode. It should inspect a repo and requirement corpus or an
existing workflow tree plus doctrine/evidence corpus, identify what it can
infer, ask only the remaining questions, and write a blueprint draft instead
of immediately free-writing final files. The proof is a new skill reference set
plus a reproducible assistant session transcript or fixture showing the
interview questions and resulting blueprint.

### Milestone 3: Compile and reconcile blueprints into checked-in `fabro/` packages

At the end of this milestone, CLI commands can read a blueprint and either
render a deterministic `fabro/` tree for a repo or reconcile an existing one.
The proof is a generated fixture tree and an update fixture whose program
manifests, run configs, and workflow graphs pass validation and match expected
snapshots.

### Milestone 4: Validate broad-requirement, real-repo, and update paths

At the end of this milestone, the system is proven in three ways: a toy
"build a craps game" requirement produces a coherent generated package, a
Myosu-shaped fixture slice produces files that match Raspberry's current
supervisory contract, and an existing-workflow update fixture produces a
coherent structural revision after ingesting doctrine and run evidence. The
proof is test fixtures, golden outputs, and command transcripts.

## Plan of Work

The first usable substrate now exists. `lib/crates/fabro-synthesis/` owns the
typed blueprint schema, parsing, validation, import, rendering, and a first
evolve report. `fabro-cli` exposes that through `fabro synth create` and
`fabro synth evolve`.

The next work should build on this substrate rather than replacing it.

The blueprint should live in a repo-local path convention such as:

    fabro/blueprints/<program>.yaml

The schema is explicit enough for the current renderer, but the next revision
should deepen the non-structural inputs. It already contains:

- blueprint version
- program identity and target repo
- units
- lanes
- lane kind
- lane title and goal summary
- `managed_milestone`
- owned artifacts and `produces`
- dependency/precondition contract
- proof/health/orchestration contract
- workflow family and package layout hints
- imported current package facts, when the blueprint comes from evolve mode
- doctrine findings and evidence findings
- unresolved questions or deferred decisions

The built-in skill package now knows how to produce a blueprint in both create
and evolve modes.
Keep `SKILL.md` concise and add new reference files rather than dumping the
whole interview policy into the skill body. The new references should explain:

- when to enter program-synthesis mode
- how to inspect the repo before asking the user questions
- how to inspect an existing `fabro/` tree before proposing changes
- how to ingest doctrine files and run evidence
- how to decide whether a lane is bootstrap, restart, implementation, service,
  orchestration, interface, platform, or recurring
- how to turn findings into blueprint fields
- what kinds of questions are legitimate when the repo leaves something
  unresolved

The deterministic renderer in `fabro-synthesis` already writes:

- `fabro/programs/*.yaml`
- `fabro/run-configs/**/*.toml`
- `fabro/workflows/**/*.fabro`
- prompt/check skeletons under stable subdirectories

The renderer should be conservative. If a blueprint field is missing or
explicitly unresolved, it should fail with a clear error rather than guessing a
path or contract silently. The goal is stable regeneration, not "best effort"
magic.

The current importer reads an existing `fabro/` tree into a blueprint-like
model and the current reconciler compares that imported model against the
desired blueprint. The next iteration should add richer doctrine and run
evidence interpretation before final rendering.

The renderer and evolver are already exposed through `fabro-cli` as a new
command family:

    fabro synth create --blueprint fabro/blueprints/<program>.yaml --target-repo .

and:

    fabro synth evolve \
      --blueprint fabro/blueprints/<program>.yaml \
      --target-repo /path/to/repo

The initial implementation does not need a first-class Rust-hosted interview
command. The skill-guided interview is enough for the first shipped build. If
later evidence shows that users need a fully integrated CLI interview loop, that
can be added on top of the same blueprint schema rather than bypassing it.

The initial fixtures and docs now exist. The next fixture growth should include:

- a toy "craps" requirement corpus plus an expected generated package
- a Myosu-shaped blueprint slice plus an expected generated package
- an existing-workflow update fixture with:
  - an initial `fabro/` tree
  - a doctrine file
  - run evidence snapshots
  - an expected reconciled `fabro/` tree

Those fixtures should prove both sparse-input broad requirement handling and
alignment with the real Raspberry manifest model, plus evidence-driven
reconciliation of an existing package.

## Concrete Steps

Work from the repository root at `/home/r/coding/fabro`.

1. Deepen doctrine and evidence ingestion.

   Modify `lib/crates/fabro-synthesis/src/blueprint.rs` and
   `lib/crates/fabro-synthesis/src/render.rs` so doctrine files and evidence
   paths influence reconcile findings, not just appear in the schema. The next
   version should be able to explain changes such as:

   - a lane exists in doctrine but not in the current package
   - a run log shows repeated failure before an upstream milestone exists
   - an output artifact exists but is not tied to any lane milestone

2. Add a Myosu-shaped reconcile fixture.

   Create a fixture that mirrors the real structure more closely:
   - an existing `fabro/` package
   - doctrine from `OS.md`
   - run evidence snapshots
   - a desired revised blueprint

   The proof should be that reconcile produces findings closer to the real
   reasons we care about, not only "lane X exists in blueprint but not current".

3. Tighten the skill around evolve output quality.

   Update:
   - `skills/fabro-create-workflow/SKILL.md`
   - `skills/fabro-create-workflow/references/program-evolution.md`

   so the skill explains how doctrine and evidence should be turned into
   concrete blueprint changes rather than generic "update the repo" advice.

4. Add tests and snapshots.

   Extend `lib/crates/fabro-synthesis/tests/synthesis.rs` and
   `lib/crates/fabro-cli/tests/synth.rs` so they cover doctrine/evidence-aware
   findings as well as structural package changes.

5. Update docs once the richer evolve behavior lands.

   `docs/reference/cli.mdx` already describes the command surface. The next doc
   revision should show a real doctrine + run-evidence reconcile example after
   that behavior exists.

## Validation and Acceptance

Run all commands from `/home/r/coding/fabro`.

Blueprint crate tests:

    cargo test -p fabro-synthesis

CLI tests:

    cargo test -p fabro-cli synth

Renderer smoke tests:

    cargo run -p fabro-cli -- synth create \
      --blueprint test/fixtures/program-synthesis/craps/blueprint.yaml \
      --target-repo /tmp/fabro-craps-out

Evolve smoke test:

    cargo run -p fabro-cli -- synth evolve \
      --blueprint test/fixtures/program-synthesis/update-myosu/blueprint.yaml \
      --target-repo /tmp/fabro-update-out

Expected result:

- the command exits 0
- `/tmp/fabro-craps-out/fabro/programs/` exists
- `/tmp/fabro-craps-out/fabro/run-configs/` exists
- `/tmp/fabro-craps-out/fabro/workflows/` exists

Then validate the emitted workflow packages and manifests:

    cargo run -p fabro-cli -- run --preflight /tmp/fabro-craps-out/fabro/run-configs/<lane>.toml
    /home/r/.cache/cargo-target/debug/raspberry status --manifest /tmp/fabro-craps-out/fabro/programs/<program>.yaml

Acceptance for the toy fixture:

- the emitted TOML files preflight successfully
- the emitted Raspberry manifest loads successfully
- the generated files match the expected snapshot fixture

Acceptance for the Myosu-shaped fixture:

- the rendered manifest uses valid lane kinds, milestones, dependencies, and
  produced artifacts
- the rendered file layout is stable and predictable
- no hidden assumptions are required beyond what is stated in the blueprint

Acceptance for the update fixture:

- the imported current package is represented in the blueprint without losing
  unit/lane/milestone structure
- doctrine and evidence findings are visible in the evolve report
- the rendered patch changes the existing `fabro/` tree in the expected
  structural ways
- the updated manifest and run-config files still validate

Acceptance for the skill layer:

- after `fabro skill install --for project --dir claude --force`, the installed
  skill contains the new program-synthesis references
- the skill's documented interview flow asks only targeted missing questions in
  the provided test transcript
- the resulting blueprint validates without manual patching

## Idempotence and Recovery

The renderer must be safe to run repeatedly on the same blueprint and target
repo. Rendering the same blueprint twice should produce the same files and the
same content. The evolve path must also be safe to rerun against the same
current package and evidence set; it should produce the same patch each time.

If the render fails halfway:

- rerunning the same command after fixing the blueprint should overwrite the
  previously generated files deterministically
- the command should never silently merge incompatible layouts from two
  different blueprints

To keep generated files reviewable, the renderer should print the files it
created or updated. The evolve path should also print or write the doctrine and
evidence reasons for each structural change. A future implementation may add a
`--check` or `--diff` mode, but the first slice only needs deterministic
overwrite behavior plus a clear reconcile report.

## Artifacts and Notes

The broad-requirement fixture should look like this at the input level:

    test/fixtures/program-synthesis/craps/requirements.md

with content similar to:

    Build a craps game for Myosu. Start with a local playable slice, but leave
    room for later chain/miner/validator integration.

The important proof is not that the renderer invents a perfect casino-grade
design. The proof is that the system:

- inspects sparse input
- identifies missing choices
- represents those choices in a blueprint
- compiles that blueprint into a stable `fabro/` package

For the update fixture and the real Myosu proving pass, the important proof is
that the system:

- imports the current workflow tree
- explains how doctrine and run evidence imply structural changes
- preserves unchanged high-quality lane packages
- produces a deterministic updated package instead of a freehand rewrite

## Interfaces and Dependencies

In `lib/crates/fabro-synthesis/src/blueprint.rs`, define typed blueprint
structures with a versioned top-level schema. The exact names may evolve, but
the end state must provide a stable API equivalent to:

    pub struct ProgramBlueprint {
        pub version: String,
        pub program: BlueprintProgram,
        pub units: Vec<BlueprintUnit>,
    }

    pub fn load_blueprint(path: &Path) -> Result<ProgramBlueprint, BlueprintError>;
    pub fn validate_blueprint(blueprint: &ProgramBlueprint) -> Result<(), BlueprintError>;

In `lib/crates/fabro-synthesis/src/render.rs`, define a renderer interface
equivalent to:

    pub struct RenderRequest<'a> {
        pub blueprint: &'a ProgramBlueprint,
        pub target_repo: &'a Path,
    }

    pub struct RenderReport {
        pub written_files: Vec<PathBuf>,
    }

    pub fn render_blueprint(req: RenderRequest<'_>) -> Result<RenderReport, RenderError>;

In `lib/crates/fabro-synthesis/src/blueprint.rs` or a nearby module, also
define importer and reconcile interfaces equivalent to:

    pub struct ImportRequest<'a> {
        pub target_repo: &'a Path,
    }

    pub fn import_existing_package(req: ImportRequest<'_>) -> Result<ProgramBlueprint, BlueprintError>;

    pub struct ReconcileRequest<'a> {
        pub blueprint: &'a ProgramBlueprint,
        pub current_repo: &'a Path,
    }

    pub struct ReconcileReport {
        pub findings: Vec<String>,
        pub written_files: Vec<PathBuf>,
    }

    pub fn reconcile_blueprint(req: ReconcileRequest<'_>) -> Result<ReconcileReport, RenderError>;

In `lib/crates/fabro-cli/src/commands/synth.rs`, define CLI entry points
equivalent to:

    pub struct SynthRenderArgs {
        pub blueprint: PathBuf,
        pub target_repo: PathBuf,
    }

    pub struct SynthReconcileArgs {
        pub blueprint: PathBuf,
        pub target_repo: PathBuf,
    }

The renderer and reconciler should depend on the current Raspberry manifest model rather than
re-inventing it. That means generated YAML must line up with the concepts in
`lib/crates/raspberry-supervisor/src/manifest.rs`, and generated run-configs
must line up with the current run-config resolution rules in
`lib/crates/fabro-config/src/run.rs`.

Plan Change Note: Updated on 2026-03-19 after renaming the user-facing command
surface to `create` / `evolve` so it better matches the product goal of
producing or updating full executable workflow packages. The plan still shifts
from "build the substrate" to "deepen evolve with doctrine and run evidence so the eventual Myosu run is
meaningful."

Live Learning Note: the worktree-backed Myosu autodev run exposed that a
generated implementation program can be artifact-complete without being
trunk-worthy. The current synthesis follow-on hardening therefore now adds:

- a `promotion.md` artifact to generated implementation-family programs
- a `merge_ready` milestone above `verified`
- a generated `promote.md` prompt and promotion gate

This is the right direction for confidence: evolve should strengthen the
definition of done itself when live runs reveal a false-positive completion,
not rely on human memory of what “complete” ought to mean.

Current policy note for generated implementation-family workflows:

- `implement` / `fixup`: fast worker model is acceptable
- `verify` / `audit` / `promotion_check`: deterministic commands
- `review`: high-intelligence model (`gpt-5.4`)
- `promote`: high-intelligence model (`gpt-5.4`)

Rationale: deterministic verification can prove that named checks passed, but
promotion still has to judge whether the slice is honestly merge-worthy,
whether manual proof obligations were actually satisfied, and whether the
artifacts match the real code/result. That is still a strong-reviewer problem,
not just a MiniMax worker problem.
