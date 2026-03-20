# Harden Autonomy and Direct Trunk Integration

This ExecPlan is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

`PLANS.md` is checked into the repository root and this document must be
maintained in accordance with it. This plan depends on
`plans/031926-build-raspberry-autodev-orchestrator.md`,
`plans/031926-build-skill-guided-program-synthesis.md`,
`plans/031926-extend-fabro-create-workflow-for-raspberry.md`, and the active
Myosu proving-ground plan at
`/home/r/coding/myosu/plans/031926-iterative-execution-and-raspberry-hardening.md`.

## Purpose / Big Picture

After this slice lands, Fabro and Raspberry should feel much closer to a truly
autonomous factory instead of a promising operator-assisted system. A
settled implementation lane should be able to land directly on trunk without a
pull request and without a fresh human decision, provided the lane's own
workflow contract says that direct integration is allowed. A new repository
such as Zend should be able to move from `README.md`, `SPEC.md`, `specs/`, and
`plans/` to a checked-in `fabro/` package and a truthful `raspberry plan`
surface through a guided create-mode bootstrap instead of manual control-plane
assembly.

The user-visible outcome is twofold. First, once an implementation lane reaches
an honest `merge_ready` state, the system should automatically land it on trunk
when the lane's run config opts into direct integration. Second, a repo that
has only planning doctrine should be able to bootstrap into a supervised
program with far less ceremony. The proof is not a new diagram; the proof is
that Myosu can settle and land real work with less operator glue, and Zend can
generate its first truthful control plane from planning docs alone.

## Progress

- [x] (2026-03-19 20:50Z) Re-read `PLANS.md`, the active Raspberry/Fabro plans,
  and the Myosu proving-ground plan so this roadmap starts from repository
  reality instead of a fresh abstraction.
- [x] (2026-03-19 20:55Z) Reviewed the current Fabro and Raspberry control
  plane after the latest hardening work and confirmed five remaining leverage
  points: branch-backed settlement, create-mode bootstrap, late-stage pipeline
  slimming, single-source runtime truth, and automatic recovery.
- [x] (2026-03-19 21:05Z) Captured the user direction that direct-to-trunk
  integration must be the default aspiration, with pull requests treated as an
  optional escape hatch rather than the mainline autonomy model.
- [x] (2026-03-19 21:20Z) Proved a stopgap supervisor-side `integration` lane
  that can squash-merge a settled run branch directly into trunk and write an
  `integration.md` artifact, then refreshed the live Myosu implementation
  packages so `play` and `sdk` now expose `*-integrate` lanes.
- [x] (2026-03-20 02:02Z) Added first-class `[integration]` support to the run
  config model, loader, and docs, including `target_branch` and
  `artifact_path`, and added a fast-fail guard that rejects configs which
  enable both `[pull_request]` and `[integration]`.
- [x] (2026-03-20 02:02Z) Moved direct trunk integration into the successful
  Fabro run path via a shared `fabro-workflows` integration module, while
  keeping Raspberry's explicit `integration` lane as a temporary compatibility
  bridge that now calls the same shared merge logic.
- [x] (2026-03-20 02:02Z) Updated synthesized implementation run configs so
  they emit `[integration] enabled = true` plus an `integration.md` artifact
  path, which means new implementation-family packages can land directly on
  trunk and satisfy the old bridge artifact contract automatically.
- [x] (2026-03-20 02:02Z) Tightened branch-backed execution so
  integration-enabled local runs force worktree creation even when the repo is
  clean, and worktree setup failure now aborts the run instead of silently
  falling back to branchless settlement.
- [x] (2026-03-20 02:02Z) Removed visible `*-integrate` follow-on lanes from
  newly synthesized implementation-family child programs. The implementation
  lane now produces `integration.md` itself, so the control plane can observe
  landed state without a second merge-layer frontier.
- [x] (2026-03-20 02:02Z) Slimmed the implementation-family graph from
  `review -> promote -> promotion_check` down to a single `settle` stage plus
  a deterministic final `audit` gate, while keeping `quality.md` and
  `promotion.md` as the machine-readable evidence boundary.
- [x] (2026-03-20 02:02Z) Repaired the built-in blueprint compiler enough to
  stop collapsing Zend into one generic lane. The current create path now
  decomposes the repo into repo-specific fronts such as
  `private-control-plane`, `home-miner-service`, `command-center-client`,
  `hermes-adapter`, and `proof-and-validation`.
- [x] (2026-03-20 02:02Z) Re-ran the Zend pilot after fixing the compiler.
  `raspberry plan` and `raspberry status` now show an honest multi-lane
  frontier aligned to the repo plan instead of a one-lane placeholder package.
- [x] (2026-03-20 02:02Z) Unified the supervisor's runtime-truth refresh path
  enough that child evaluation now propagates parent state and autodev updates,
  and stale parent snapshots no longer wait on a manual top-level status call
  to converge.
- [x] (2026-03-20 02:02Z) Added the first automatic recovery path in `autodev`
  for replay-worthy branch or integration failures, so recoverable failed lanes
  can be explicitly resubmitted without operator archaeology.
- [x] (2026-03-20 04:18Z) Added a repo-local, headless `paperclip bootstrap`
  path that layers a Paperclip company and tailored agent roster on top of
  Raspberry supervision. The Zend pilot now proves dry-run bundle generation,
  live import/update, company goal creation, repo workspace project creation,
  and isolated local-cli env export generation under `.paperclip/`.
- [x] (2026-03-20 02:02Z) Added a first-class orchestration synthesis
  template so child-program lanes now infer and serialize as
  `WorkflowTemplate::Orchestration` instead of being smuggled through the
  bootstrap family.
- [x] (2026-03-19 22:10Z) Re-reviewed the Fabro workflow docs to align the
  five family definitions with idiomatic node, transition, goal-gate, and
  parallelization patterns instead of only with Myosu-local habits.
- [x] (2026-03-20 02:02Z) Proved that the built-in Zend `synth create` path,
  `raspberry plan`, `raspberry status`, and bounded `raspberry execute` all
  work mechanically end-to-end.
- [x] (2026-03-20 02:02Z) Replaced the first Zend output after the user flagged
  it as unacceptable. The current generated package is no longer a one-lane
  placeholder and now carries explicit lane decomposition and dependencies.

## Surprises & Discoveries

- Observation: the current proving-ground pain is no longer "can Raspberry
  dispatch work?" but "does settled work become trunk truth automatically?"
  Evidence: Myosu can now drive implementation-family children through
  `merge_ready`, but trunk integration still requires extra control-plane
  ceremony or rerun knowledge.

- Observation: the newly added `integration` lane proves the git mechanics are
  feasible, but it is still one layer too visible in the user model.
  Evidence: the refreshed Myosu child manifests now show `tui-integrate` and
  `core-integrate` as ready lanes, which proves the system can land work, but
  the user correctly wants direct integration to be a property of the lane's
  run config rather than another explicit frontier to supervise.

- Observation: branch-backed settlement is still inconsistent because local
  implementation runs in `autodev-live` may begin from a dirty worktree.
  Evidence: older successful Myosu implementation runs do not carry
  `run_branch` in their stored manifests, which means they are settled in an
  artifact sense but not yet mergeable in a trunk-integration sense.

- Observation: late-stage workflow prompts are now stronger, but the
  node-level process still has more ceremony than the user wants.
  Evidence: the current implementation-family template still uses
  `review -> promote -> promotion_check -> audit`, even though deterministic
  `quality.md` already carries much of the anti-optimism burden.

- Observation: Zend is close to the bootstrap threshold but not yet at the
  autonomy threshold.
  Evidence: `/home/r/coding/zend` now contains `README.md`, `SPEC.md`,
  `specs/2026-03-19-zend-product-spec.md`, and
  `plans/2026-03-19-build-zend-home-command-center.md`, but it still has no
  `fabro/` package and no `.raspberry/` state surfaces.

- Observation: the current CLI boundary is the wrong place for the user to
  stop.
  Evidence: `fabro synth create` and `fabro synth evolve` already handle
  deterministic rendering and reconciliation, but they still require an
  explicit blueprint file, which leaves the most important planning-to-blueprint
  step outside the product.

- Observation: the first built-in Zend bootstrap output is not good enough to
  count as a successful compiler result.
  Evidence: the generated `zend` package collapsed the repo into one generic
  bootstrap lane with only `spec.md` and `review.md`, and its prompts or goals
  were visibly generic rather than reflecting the real plan structure. This is
  a command-path proof, not an acceptable blueprint compiler output.

- Observation: Paperclip is now mature enough to serve as a repo-local
  company/agent layer on top of Raspberry rather than as a competing factory
  abstraction.
  Evidence: as of March 20, 2026, the official Paperclip quickstart documents
  `npx paperclipai onboard --yes`, `pnpm paperclipai run`, embedded PostgreSQL
  local mode, `--data-dir` instance isolation, company import or export, and
  built-in `codex_local`, `claude_local`, `gemini_local`, `hermes_local`,
  `process`, and `http` adapters.

- Observation: the best headless Paperclip integration path is an importable
  company package emitted by the compiler, not imperative post-bootstrap API
  mutation.
  Evidence: the CLI docs expose `company import --target new --include company,agents`
  and `company export`, but the quickstart still assumes initial company and CEO
  setup in the web UI. That implies our deterministic bootstrap should emit a
  portable company package and then import it into a repo-local instance.

## Decision Log

- Decision: direct trunk integration is the target operator model, and pull
  requests are not the default autonomy boundary.
  Rationale: the user explicitly wants settled work to go straight to trunk
  with no manual merge bureaucracy once the workflow has already paid for the
  real review and proof work.
  Date/Author: 2026-03-19 / User + Codex

- Decision: the long-term configuration surface for direct integration must
  live in run-config TOML rather than only in a Raspberry manifest lane.
  Rationale: the integration policy belongs beside the workflow's execution
  contract in the same way `[pull_request]` already does, and the user wants
  the behavior to be literally part of the workflow TOML.
  Date/Author: 2026-03-19 / User + Codex

- Decision: the current explicit `integration` lane is a useful proving
  mechanism, but it is not the desired final user-facing abstraction.
  Rationale: it lets us validate direct trunk merge mechanics now, while the
  next platform slice can fold that behavior into a single settled-run policy.
  Date/Author: 2026-03-19 / Codex

- Decision: a lane must never be considered honestly settled unless it has a
  durable, mergeable source artifact for integration.
  Rationale: `merge_ready` without a usable run branch or equivalent source
  commit still leaves a hidden operator task, which breaks the autonomy model.
  Date/Author: 2026-03-19 / Codex

- Decision: Zend should be used as the first create-mode bootstrap pilot only
  after the bootstrap command and runtime-truth cleanup are in place.
  Rationale: the current system is strong enough to generate a package, but not
  yet strong enough to call the whole flow "hands-off" on a brand-new repo.
  Date/Author: 2026-03-19 / User + Codex

- Decision: do not introduce a separate bootstrap command if `synth create`
  and `synth evolve` can absorb blueprint authoring directly.
  Rationale: the user wants the blueprint-creation step to be part of the
  primary synthesis workflow, not a separate preparatory command.
  Date/Author: 2026-03-19 / User + Codex

- Decision: command-path success is not enough for synthesis. The compiler is
  not "working" until the generated package is repo-faithful and qualitatively
  obvious to a human reviewer.
  Rationale: the first Zend output proved that the CLI can author and render a
  blueprint, but it did not prove that the authored blueprint was good. The
  quality bar is now explicit and blocks downstream roadmap claims.
  Date/Author: 2026-03-20 / User + Codex

- Decision: Paperclip integration is downstream of fixing blueprint compiler
  quality.
  Rationale: a headless company or agent layer built on top of bad lane
  decomposition will just amplify the wrong plan. The compiler must become
  trustworthy before we automate a Paperclip layer above it.
  Date/Author: 2026-03-20 / User + Codex

- Decision: Paperclip should be integrated as a higher-level company and
  heartbeat layer above Raspberry rather than as a replacement for Fabro or
  Raspberry orchestration.
  Rationale: Raspberry already owns lane truth, execution, and repo-local
  control-plane semantics. Paperclip is strongest when it adds company goal,
  org chart, issue hierarchy, and heartbeat-driven agent invocation on top of
  those primitives.
  Date/Author: 2026-03-20 / User + Codex

- Decision: the Paperclip bootstrap path should default to a repo-local,
  embedded, headless instance rooted inside the target repository.
  Rationale: the official setup docs explicitly support `--data-dir` instance
  isolation and embedded PostgreSQL, which makes a per-repo control plane both
  cheap and reproducible.
  Date/Author: 2026-03-20 / Codex

- Decision: compiler-driven Paperclip bootstrap should emit a company package
  and import it, not create agents imperatively one call at a time.
  Rationale: import or export is the clearest deterministic boundary exposed by
  the current Paperclip CLI, and it lets the synthesis front half keep one
  reviewable artifact contract for company goal, org chart, adapters, and seed
  tasks.
  Date/Author: 2026-03-20 / Codex

## Outcomes & Retrospective

This plan begins at a moment when Fabro and Raspberry are clearly useful but
not yet truly self-driving. The system can synthesize child programs, run
deterministic quality gates, and drive live Myosu work under real credentials.
The remaining gap is not general capability. The remaining gap is friction at
the edges: settlement that does not land, bootstrap that still requires too
many manual steps, runtime truth that can drift, and recovery that still needs
an operator's judgment.

The immediate positive outcome before this plan even starts is that we now have
evidence for the direction. Myosu proved that slot-aware Codex review works,
that implementation-family quality contracts can block optimistic promotion,
and that a direct-to-trunk supervisor merge is technically viable. The main
lesson is that the next slice should cut operator ceremony rather than adding
more proof theater. We already know how to ask the models to review work; the
next system improvement is to stop asking twice after the evidence is already
good enough.

## What Already Exists

Several pieces of the target architecture already exist and should be reused
rather than rebuilt.

- `fabro synth import` already converts a checked-in package into a blueprint.
  That logic lives in
  `lib/crates/fabro-synthesis/src/blueprint.rs` via
  `import_existing_package(...)`.
- `fabro synth create` already performs deterministic blueprint rendering into
  `fabro/programs/`, `fabro/run-configs/`, `fabro/workflows/`, and
  `fabro/prompts/`. That logic lives in
  `lib/crates/fabro-synthesis/src/render.rs` via `render_blueprint(...)`.
- `fabro synth evolve` already performs deterministic reconciliation against an
  existing package and run evidence through `reconcile_blueprint(...)`.
- The current implementation-family renderer already contains the strongest
  synthesis contract in the codebase. It writes deterministic proof commands,
  `quality.md`, and `promotion.md`.
- The stopgap direct-to-trunk merge mechanics already exist in
  `lib/crates/raspberry-supervisor/src/integration.rs`. That code should be
  migrated into first-class run-config behavior rather than discarded.
- Fabro already has idiomatic sub-workflow support through `shape=house`, as
  documented in `docs/tutorials/sub-workflow.mdx`. That means orchestration
  does not need to invent a new graph concept; it needs to decide when
  repo-level child-program supervision should use Raspberry semantics and when a
  nested Fabro sub-workflow is sufficient.

## NOT in Scope

This plan intentionally does not include:

- Replacing the existing `[pull_request]` path. It can remain as an optional
  integration mode for teams that still want PR-backed review.
- Retrofitting every historical Myosu run to become mergeable. Old branchless
  settled runs may need replay; the goal is to prevent new branchless
  settlement, not to rewrite history.
- Designing an open-ended family catalog beyond the agreed five families. This
  slice is about making those five real and deterministic, not adding more.
- Full unattended Zend autodev on the first pilot. The Zend proof target is an
  honest package plus bounded execution, not a full hands-off autonomous loop.
- Rewriting Fabro's core graph engine or replacing Graphviz DOT. The goal is to
  use the existing engine more deterministically, not to replace it.

## Context and Orientation

Fabro is the workflow runner. It executes a graph, records run metadata under
`~/.fabro/runs/<run-id>/`, and already understands project-level run config
TOML. The existing trunk-adjacent post-run feature lives in the `[pull_request]`
section documented in
`/home/r/coding/fabro/docs/execution/run-configuration.mdx` and implemented in
`/home/r/coding/fabro/lib/crates/fabro-cli/src/commands/run.rs`. That path can
open a GitHub pull request and optionally enable GitHub auto-merge, but it is
not the direct trunk path the user wants.

Raspberry is the supervisory plane. It evaluates program manifests, tracks lane
state in `.raspberry/*-state.json`, renders operator views through
`raspberry plan/status/watch/tui`, and dispatches lanes with
`raspberry execute` or bounded autonomous cycles with `raspberry autodev`.
The key supervisor modules are:

- `lib/crates/raspberry-supervisor/src/manifest.rs`
- `lib/crates/raspberry-supervisor/src/evaluate.rs`
- `lib/crates/raspberry-supervisor/src/program_state.rs`
- `lib/crates/raspberry-supervisor/src/dispatch.rs`
- `lib/crates/raspberry-supervisor/src/autodev.rs`

Synthesis is the layer that turns blueprints into checked-in packages. Its key
files are:

- `lib/crates/fabro-synthesis/src/blueprint.rs`
- `lib/crates/fabro-synthesis/src/render.rs`

Implementation-family child programs are the most important proving surface
today. They currently synthesize units that end in milestones such as
`reviewed`, `implemented`, `verified`, and `merge_ready`, with artifacts such
as `implementation.md`, `verification.md`, `quality.md`, and `promotion.md`.
The live Myosu worktree under
`/home/r/coding/myosu/.worktrees/autodev-live/` is the active proving ground.
It now also contains a stopgap `integration.md` artifact and `*-integrate`
lanes, which prove direct trunk merge mechanics but are not yet the desired
final abstraction.

Zend is the new-repo bootstrap target. Its current durable planning inputs are:

- `/home/r/coding/zend/README.md`
- `/home/r/coding/zend/SPEC.md`
- `/home/r/coding/zend/specs/2026-03-19-zend-product-spec.md`
- `/home/r/coding/zend/plans/2026-03-19-build-zend-home-command-center.md`

Zend has no `fabro/` package and no `.raspberry/` state yet. That means it is
ready for create-mode bootstrap work, not yet for unattended autonomous
execution.

The key architecture for this plan is:

    planning corpus
        |
        |  README.md / SPEC.md / specs/ / plans/ / doctrine
        v
    lane intents
        |
        |  deterministic family selector
        v
    blueprint
        |
        |  deterministic renderer
        v
    checked-in fabro/ package
        |
        |  run-config policy + supervisor truth
        v
    execution, settlement, and direct trunk integration

The evolve path should mirror the same architecture:

    existing fabro/ package
        |
        |  import
        v
    current blueprint
        |
        |  doctrine + evidence refinement
        v
    revised blueprint
        |
        |  deterministic reconcile
        v
    updated fabro/ package

The critical architectural insight is that blueprint creation is not a separate
world from rendering. It is the missing deterministic front half of synthesis.

## Plan of Work

The immediate blocker is blueprint compiler quality. Before any more priority
is given to higher-level autonomy layers, the built-in synthesis path must stop
producing thin placeholder packages for new repos. The Zend output failure
showed that "command exists" is not the right success bar. The success bar is:

- the generated lanes are obviously repo-specific
- the package reflects the real frontier decomposition from the active plan
- artifact contracts are strong enough to justify the chosen family
- a human reviewer can tell why each lane exists and why the chosen family fits

That changes the priority order for the remaining work:

1. repair the blueprint compiler
2. re-run the Zend pilot until the generated package is good
3. finish runtime-truth and automatic recovery
4. add Paperclip on top of the corrected compiler and control plane

The first milestone is to make trunk landing a built-in execution policy rather
than a separate operator chore. The end state for this milestone is that an
implementation-family run config can say, in TOML, that a successful settled
run should integrate directly into trunk. The integration policy must behave
like a post-run execution hook with deterministic guards, not like a new LLM
conversation. The intended config shape should mirror the existing
`[pull_request]` section, but target direct integration instead, for example:

    [integration]
    enabled = true
    mode = "direct_trunk"
    strategy = "squash"
    target_branch = "origin/HEAD"
    require_managed_milestone = "merge_ready"

The implementation should live in the Fabro run path rather than only in
Raspberry dispatch. That means `fabro-cli` should read the integration policy
after a successful run, inspect the run manifest and checkpoint metadata, and
perform the integration automatically. Raspberry should still understand the
result, but it should not need a visible extra lane just to land a settled run.
As part of this milestone, the temporary explicit `integration` lane should be
treated as a compatibility bridge. The final generated implementation-family
package should not require an operator to notice and execute `*-integrate`.

The second milestone is to make branch-backed settlement mandatory. Hidden
internal branches are acceptable; user-visible branch ceremony is not. The
system should stop reporting `merge_ready` when the settled run cannot be
integrated deterministically. This work belongs in the Fabro run setup and
checkpoint path. `lib/crates/fabro-cli/src/commands/run.rs`,
`lib/crates/fabro-workflows/src/manifest.rs`, and the git helpers in
`lib/crates/fabro-workflows/src/git.rs` must ensure that implementation-family
lanes always get a mergeable source. Dirty autodev worktrees should no longer
silently degrade into branchless settlement. If a true branch cannot be
created, the run must either synthesize an equivalent durable source commit or
fail before settlement.

The third milestone is to slim the implementation-family workflow tail. The
target behavior is that deterministic proof and deterministic quality do the
bulk of the anti-optimism work, and the remaining model judgment happens once.
The likely end state is to reduce the tail to something like
`implement -> verify -> quality -> settle -> audit`, where the settle step owns
the final truth artifact and direct integration can happen immediately after a
successful audit. This work belongs in `lib/crates/fabro-synthesis/src/render.rs`
and the skill/reference guidance under
`skills/fabro-create-workflow/`. The point is not to remove review quality; the
point is to stop paying for redundant review surfaces after the system already
has strong evidence.

The fourth milestone is to unify runtime truth and automatic recovery. Today
the same repo can report different realities in a child program state file, a
top-level state file, and an autodev report. The system should converge on one
canonical model that all observer surfaces read from. `evaluate.rs`,
`program_state.rs`, `autodev.rs`, and the TUI observer code must agree on lane
status, child-program status, and recent completion. At the same time, the
supervisor should gain automatic repair paths for common control-plane drift:
stale runtime records, branchless settled runs after a config change,
schema-evolved manifests, and lanes whose artifacts already prove settlement.

The fifth milestone is create-mode bootstrap folded into synthesis itself, but
with an explicit quality reset. The built-in path now exists mechanically, but
the current compiler still over-collapses repo plans into generic bootstrap
output. The immediate work here is not just "support omitted `--blueprint`."
The immediate work is:

- better lane-intent extraction from active plans
- multi-lane decomposition when a repo plan obviously contains multiple fronts
- family selection that does not overfit on stray plan words
- goals and prompt contexts that are specific enough to the repo to be useful
- a Zend bootstrap result that is good enough to review without embarrassment

Only after that should this milestone be considered complete.

The sixth milestone is first-class family encoding. Today the docs now describe
five canonical families, but the code does not yet treat them as a complete
standard library. The plan-to-blueprint compiler must make them first-class in
both structure and optimization rules:

- `bootstrap`
- `service_bootstrap`
- `implementation`
- `recurring_report`
- `orchestration`

That means every generated lane should be explainable as:

    accepted plan section
        -> lane intent
        -> chosen family
        -> family-specific blueprint shape
        -> rendered workflow/run-config/prompts

No hidden freehand topology choices should survive this step.

The seventh milestone is Paperclip bootstrap layered on top of the compiler.
Once synthesis can derive a first honest blueprint and package without manual
authoring, the same compiler should be able to emit a deterministic Paperclip
company package and install it into a repo-local instance. The target operator
experience is:

    prompt/spec/plan -> synth create/evolve
                      -> fabro/ package
                      -> paperclip company package
                      -> repo-local Paperclip instance bootstrap
                      -> Raspberry-supervised execution via tailored agents

This milestone should not bypass the compiler. The Paperclip company package
must be derived from the same family selector and lane intents as the Fabro
package so the two control planes agree on mission, units, and role ownership.

## First-Class Workflow Families

The synthesis system should stop treating the family catalog as a loose naming
convention and start treating it as a real standard library. Each family below
should have one first-class workflow definition, one run-config policy shape,
and one clear set of artifact and proof expectations.

The idiomatic rules from the current Fabro docs apply across every family:

- start and exit remain explicit `Mdiamond` and `Msquare` nodes
- deterministic shell proof should live in `parallelogram` command nodes
- loops should be bounded and route through explicit fixup or retry nodes
- `goal_gate=true` belongs on proof-enforcing command nodes, not on prose-only
  LLM stages
- human gates should be exceptional and explicit, not the default bootstrap
  mechanism
- parallel fan-out and merge are optional ingredients for broad lanes, not the
  default topology for normal bootstrap or implementation work

### Family 1: `bootstrap`

This family exists for the first honest frontier in a repo or subsystem when
the system still needs to create stable surfaces, reviewed artifacts, or
restart contracts before implementation-family delivery is justified.

The idiomatic topology should be linear and evidence-light:

    start -> specify -> review -> polish -> verify_outputs -> exit

`specify` can be a prompt or agent stage, `review` should stay lightweight but
critical, `polish` should normalize durable artifacts, and `verify_outputs`
should be a deterministic command node that proves the expected artifacts were
written. This family should not include a human gate by default, and it should
not include merge-worthiness claims. Its job is to establish truthful reviewed
or restart-ready surfaces.

### Family 2: `service_bootstrap`

This family exists when the planned slice is a service or daemon and the first
proof bar is a health surface such as `/health`, a stable log line, or a basic
RPC method.

The idiomatic topology should be:

    start -> inventory -> review -> polish -> verify_outputs -> health -> exit

`inventory` should capture the service boundary, `verify_outputs` should prove
the durable artifacts exist, and `health` should be a deterministic command
goal gate that exercises the first service signal. This family should preserve
health and observability contracts explicitly in its prompts and artifacts. It
should not jump to full implementation-family quality gates until the service
has a real reviewed slice and a deterministic proof command.

### Family 3: `implementation`

This family exists only after the repo has a reviewed slice and a real
deterministic proof command. Based on Myosu, this is the strongest existing
family, but it is still carrying too much late-stage bureaucracy.

The target topology should be:

    start -> preflight -> implement -> verify -> quality -> settle -> audit -> exit

with:

    verify -> fixup
    quality -> fixup
    settle -> fixup
    audit -> fixup
    fixup -> verify

`settle` is the single strong-model judgment step. It replaces redundant
`review` and `promote` duplication while keeping the deterministic `quality.md`
gate intact. For service implementations, a `health` goal gate may sit between
`verify` and `quality` when the health surface is real and distinct. Direct
trunk integration should not be another visible lane in the final model. It
should be a post-success run-config policy attached to this family.

Parallel review remains optional inside this family. Use it only when the lane
is broad enough to justify independent security, architecture, and quality
perspectives.

### Family 4: `recurring_report`

This family exists for recurring oversight such as scorecards, retros,
strategy-planning, or operational audit loops that do not own product code
delivery directly.

The idiomatic topology should be:

    start -> collect -> synthesize -> verify_report -> exit

`collect` should be a deterministic command stage or small evidence-gathering
step, `synthesize` can be a prompt or agent stage, and `verify_report` should
prove the expected durable report artifact exists and is non-empty. This family
should default to linear execution. It should not borrow implementation-family
gates unless the recurring lane is explicitly converting into a deliverable
code lane.

### Family 5: `orchestration`

This family exists when a lane supervises a child program rather than running
one raw workflow directly. It must become first-class because child-program
coordination is no longer an edge case in the Myosu proving ground.

The idiomatic topology should be command-driven and non-LLM by default:

    start -> refresh_child -> dispatch_child -> wait_or_settle -> summarize -> exit

where:

- `refresh_child` is a deterministic state refresh
- `dispatch_child` is a deterministic child-program execution step
- `wait_or_settle` is a deterministic condition or bounded wait loop
- `summarize` is optional and should remain lightweight

This family should not pretend that orchestration is just a normal bootstrap
lane with a `program_manifest` attached. It needs its own synthesis family,
own run-config contract, and own runtime-truth semantics because it supervises
other lanes instead of owning product artifacts directly.

There are two orchestration cases and the template must choose deliberately:

1. **Repo-level child program supervision**
   Use Raspberry orchestration semantics when the parent lane is supervising a
   checked-in child program manifest over time.

2. **Nested workflow delegation**
   Use Fabro's idiomatic `shape=house` sub-workflow pattern when the parent
   workflow is still one run and just wants to delegate a reusable child graph.

The family must not conflate those two. Repo-level orchestration is the
default for Raspberry packages. `house` sub-workflows are an internal Fabro
ingredient, not the default replacement for child-program supervision.

## Failure Modes

The review surfaced a small set of realistic failure modes that the plan should
treat as first-class test targets.

- **Direct integration silently targets the wrong branch**
  The system should prove trunk selection explicitly, preferably from
  `origin/HEAD` or an explicit TOML override, and fail loudly if the target is
  ambiguous.

- **A lane reaches `merge_ready` but still has no mergeable source**
  The run must fail before settlement or be marked for replay automatically.
  Silent branchless settlement is no longer acceptable.

- **The plan-to-blueprint compiler chooses the wrong family**
  The compiler should emit an inspectable family-selection explanation and stop
  for only narrow unresolved ambiguity, not produce a plausible but wrong
  package.

- **The evolve path drifts from the create path**
  If create and evolve use different family rules, the same repo will mutate
  unexpectedly. The selector and optimizer must be shared logic.

- **Parent state and child state disagree after landing**
  Integration and child completion must update the same canonical runtime truth
  model. Otherwise the UI will keep showing stale ready/running lanes after
  work already landed.

- **Orchestration picks the wrong execution mode**
  A child-program supervisory lane and a nested sub-workflow are not the same
  thing. Using a `house` node where a long-lived child program is needed will
  lose the correct operator semantics.

## Concrete Steps

Work in `/home/r/coding/fabro` unless a step explicitly names the Myosu or Zend
repositories.

1. Add a first-class `[integration]` configuration block to the run-config data
   model and CLI load path. Update the TOML parsing surface and the docs in
   `docs/execution/run-configuration.mdx`. Make the config describe direct
   trunk integration, not pull requests.

2. Move the current direct trunk merge mechanics out of the stopgap supervisor
   lane and into the successful-run path in
   `lib/crates/fabro-cli/src/commands/run.rs`. Reuse the existing run manifest,
   host repo path, and run branch metadata rather than inventing a new source
   of truth.

3. Tighten the run setup path so implementation-family workflows always create
   a mergeable source branch or fail before settlement. The relevant files are
   `lib/crates/fabro-cli/src/commands/run.rs`,
   `lib/crates/fabro-workflows/src/manifest.rs`, and
   `lib/crates/fabro-workflows/src/git.rs`.

4. Simplify the implementation-family synthesis template in
   `lib/crates/fabro-synthesis/src/render.rs`. Remove redundant late-stage
   bureaucracy while preserving deterministic `quality.md` and final truth
   artifacts. Update the create/evolve guidance in the Fabro workflow skill so
   future synthesized packages follow the slimmer pattern.

5. Remove the need for explicit `*-integrate` lanes from generated
   implementation-family programs once run-config integration is working. Until
   then, keep the current stopgap lane only as a migration bridge and document
   it as transitional rather than final.

6. Unify runtime truth in the supervisor. Update
   `lib/crates/raspberry-supervisor/src/evaluate.rs`,
   `lib/crates/raspberry-supervisor/src/program_state.rs`,
   `lib/crates/raspberry-supervisor/src/autodev.rs`, and the TUI code so
   top-level state, child state, and autodev summaries agree automatically.

7. Add automatic recovery behavior for stale or branchless settled runs. The
   system should either rerun them under the new branch-backed rules or mark
   them as requiring replay, without a human having to reconstruct the reason.

8. Fold blueprint authoring into the synthesis CLI. Extend `fabro synth create`
   so a repo with only `README.md`, `SPEC.md`, `specs/`, and `plans/` can
   produce a first blueprint and first package without manual file
   choreography. Extend `fabro synth evolve` so it can
   import the current package and refine the blueprint from doctrine and
   evidence when an explicit blueprint is not supplied. Update the bootstrap
   guide in `docs/guides/from-specs-to-blueprint.mdx` to describe the final
   built-in path.

   The concrete architecture should be:

       repo docs -> corpus loader -> lane intents -> family selector
                 -> family optimizer -> blueprint -> render/reconcile

   The family selector should be shared by create and evolve. It should not
   live only in prompt guidance.

9. Add a first-class `orchestration` workflow template in synthesis so the
   five-family selector is real in code. Child-program lanes should render
   from a stable orchestration family instead of being inferred only through
   special-case blueprint handling.

10. Run the Zend pilot in `/home/r/coding/zend`. First generate a blueprint and
   checked-in package. Then run `raspberry plan` and `raspberry status`. Only
   after those are honest and stable should the pilot advance to a bounded
   `raspberry execute`.

11. Add `paperclip bootstrap` as a repo-local setup path. It should:
   - start or repair a repo-local Paperclip instance with `--data-dir`
   - default to an embedded local instance instead of shared global state
   - emit a deterministic Paperclip company package from the same planning
     corpus and family selector used by `synth create/evolve`
   - import that package into the local instance instead of requiring manual
     UI setup for the first company and agents
   - seed a tailored initial org chart:
     - one mission or CEO agent bound to the company goal
     - one Raspberry orchestration agent that invokes repo-local
       `raspberry plan/status/execute/autodev`
     - lane-family-specific workers chosen from built-in adapters such as
       `codex_local`, `claude_local`, `gemini_local`, `hermes_local`, `process`,
       or `http`
   - write repo-local launcher scripts so the instance can run headlessly even
     if the upstream quickstart assumes an interactive browser setup

12. Extend the Zend pilot to exercise the Paperclip layer after the initial
   `fabro/` package is honest. The acceptance bar is that Zend can:
   - bootstrap a repo-local Paperclip instance under a repo-owned data dir
   - import the generated company package
   - expose the synthesized mission and initial tailored agents
   - invoke at least one Raspberry-backed heartbeat path without manual agent
     creation in the UI

## Validation and Acceptance

Validation must prove behavior, not merely code structure.

For direct trunk integration, create or reuse a local git test fixture where a
successful implementation run has a real run branch and `origin/HEAD` points to
the target trunk branch. Run the relevant cargo tests from
`/home/r/coding/fabro` and expect them to prove that a run with
`[integration] enabled = true` lands a squash commit on trunk automatically,
without opening a pull request.

For branch-backed settlement, run implementation-family tests and a real Myosu
rerun from `/home/r/coding/myosu/.worktrees/autodev-live/`. The acceptance bar
is that a fresh successful implementation run records a durable mergeable
source, and the subsequent direct integration path can use it without a manual
rerun or branch surgery.

For pipeline slimming, verify that the generated implementation workflow still
rejects placeholder-heavy or warning-heavy slices, but no longer requires
redundant extra review bureaucracy after evidence is already sufficient.

For unified runtime truth, run `raspberry status`, inspect the corresponding
`.raspberry/*-state.json` and `*-autodev.json`, and open the TUI for the same
program. Acceptance means those three surfaces agree on whether a lane is
ready, running, complete, failed, or already landed.

For create-mode bootstrap, run the built-in synthesis path against
`/home/r/coding/zend`. Acceptance means Zend gains a checked-in `fabro/`
package from `fabro synth create` itself, `raspberry plan` returns an honest
grouped frontier, and `raspberry status` exposes meaningful lane truth without
manual package edits or a separately authored blueprint.

That is necessary but not sufficient. The generated package must also pass a
qualitative repo-faithfulness bar:

- it must decompose obvious multi-front plans into more than one lane when the
  plan clearly contains multiple workstreams
- it must not collapse a product plan into one generic bootstrap lane unless
  that collapse is clearly justified by the plan corpus
- its goals, artifacts, and prompt context must read as repo-specific rather
  than generic summaries of the planning files
- a human reviewer should be able to look at the generated blueprint and say
  "yes, this is recognizably the plan I wrote"

For family selection, acceptance means the compiler can explain, for each
generated lane, why that family was chosen and which plan evidence justified
it. If two different engineers run the same command on the same repo state,
they should get the same blueprint shape.

For orchestration, acceptance means the compiler can distinguish child-program
supervision from nested sub-workflow delegation and choose the correct
template deterministically.

For Paperclip bootstrap, acceptance means a repo can stand up an isolated local
Paperclip instance under a repo-owned data directory, import a compiler-emitted
company package, and expose an initial tailored agent roster whose first useful
worker is the Raspberry orchestration agent. The first import must not require
manual company or CEO creation in the web UI.

## Idempotence and Recovery

The direct integration path must be safe to retry. If the target branch already
contains the source diff, a retry should write a truthful `integration.md`
artifact and exit cleanly instead of generating duplicate history. If trunk has
moved and the merge can no longer be applied cleanly, the run must fail with a
clear message that says the lane needs a replay from current trunk, not a human
guess.

The bootstrap path must also be safe to repeat. Re-running create-mode on Zend
should update the generated package deterministically rather than multiplying
duplicate manifests or prompts. Runtime-truth refresh steps should be safe to
run repeatedly and should converge, not oscillate.

The family selector must also be idempotent. Re-running it over the same plan
corpus should not oscillate between `bootstrap` and `implementation`, or
between `orchestration` and a plain bootstrap lane, without a real repo-state
change that explains the transition.

## Artifacts and Notes

The proving-ground evidence that motivated this plan is already present in the
live worktree:

    /home/r/coding/myosu/.worktrees/autodev-live/fabro/programs/myosu-play-tui-implementation.yaml
    /home/r/coding/myosu/.worktrees/autodev-live/fabro/programs/myosu-sdk-core-implementation.yaml

Both now show a stopgap `*-integrate` lane and an `integrated` milestone. That
state proves the mechanics are within reach, but the plan above deliberately
treats those lanes as an intermediate bridge. The target experience is that the
workflow TOML itself declares direct trunk integration and the settled run lands
without a visible second lane.

Zend's current bootstrap inputs are:

    /home/r/coding/zend/README.md
    /home/r/coding/zend/SPEC.md
    /home/r/coding/zend/specs/2026-03-19-zend-product-spec.md
    /home/r/coding/zend/plans/2026-03-19-build-zend-home-command-center.md

Those files are sufficient to start the create-mode pilot once the bootstrap
command exists.

The relevant Paperclip docs reviewed on March 20, 2026 are:

    https://docs.paperclip.ing/start/quickstart
    https://docs.paperclip.ing/start/core-concepts
    https://docs.paperclip.ing/start/architecture
    https://docs.paperclip.ing/cli/overview
    https://docs.paperclip.ing/cli/setup-commands
    https://docs.paperclip.ing/cli/control-plane-commands

The most important test artifact to add during implementation is a family
selection matrix with cases like:

- plan says "stand up a health endpoint first" -> `service_bootstrap`
- plan says "coordinate child program X" -> `orchestration`
- plan says "write the next reviewed slice and prove merge readiness" ->
  `implementation`
- plan says "weekly scorecard" -> `recurring_report`
- plan says "create initial reviewed artifacts for a new surface" -> `bootstrap`

## Interfaces and Dependencies

The new direct integration policy must have a stable, documented run-config
surface. The intended end-state is a new configuration section alongside the
existing `[pull_request]` support in the Fabro run config model. The config
must be available to both CLI execution and any future server execution path.

The plan-to-blueprint compiler should introduce explicit internal interfaces so
create and evolve share the same deterministic front half. The exact names can
change, but the plan should result in concepts equivalent to:

    struct PlanningCorpus { ... }
    struct LaneIntent { ... }
    enum FamilySelection { Bootstrap, ServiceBootstrap, Implementation, RecurringReport, Orchestration }

with functions equivalent to:

    load_planning_corpus(repo: &Path) -> PlanningCorpus
    derive_lane_intents(corpus: &PlanningCorpus) -> Vec<LaneIntent>
    select_family(intent: &LaneIntent, repo: &Path) -> FamilySelection
    materialize_blueprint(program: &str, intents: &[LaneIntent]) -> ProgramBlueprint

This interface boundary is important because the selector and materializer must
be identical between `synth create` and `synth evolve`.

The trunk-integration implementation must depend on existing Fabro run truth:

- `fabro_workflows::manifest::Manifest` for `run_branch`, `base_branch`, and
  `host_repo_path`
- the existing git helpers in `lib/crates/fabro-workflows/src/git.rs`
- the existing run inspection path in
  `lib/crates/fabro-workflows/src/run_inspect.rs`

The supervisor must keep using a simple public model:

- `LaneExecutionStatus` remains the top-level status vocabulary
- implementation-family programs should expose `merge_ready` and `integrated`
  as milestones
- runtime surfaces must be derived from one canonical state path instead of
  repeated ad hoc reconciliation

Zend bootstrap must keep using the existing blueprint-first architecture:

- blueprint generation remains the reviewable intermediate step
- `fabro synth create` must become both the blueprint author and the compiler
  when no explicit blueprint is supplied
- Raspberry begins only after the package exists

Revision note: expanded this plan on 2026-03-19 after a direct engineering
review to add first-class workflow family definitions, explicit architecture
for plan/spec -> blueprint compilation, the missing `What already exists` and
`NOT in scope` sections, and concrete failure modes drawn from Myosu and the
Fabro docs.

Revision note: created this plan on 2026-03-19 to consolidate the next
high-impact autonomy work after the first Myosu proving-ground cycle showed
that direct-to-trunk landing, bootstrap UX, runtime-truth unification, and
recovery are now the main remaining blockers to a truly autonomous workflow.
