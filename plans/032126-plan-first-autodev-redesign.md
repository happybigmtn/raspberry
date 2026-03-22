# Plan-First Autodev Redesign

This ExecPlan is a living document. The sections `Progress`,
`Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must
be kept up to date as work proceeds.

`PLANS.md` is checked into the repository root and this document must be
maintained in accordance with it.

This plan depends on the current Fabro, Raspberry, and Paperclip control-plane
implementation in this repository, plus the checked-in proving-ground package in
`/home/r/coding/rXMRbro`. The purpose of this plan is to make numbered plan
files such as `plans/005-craps-game.md` become the primary supervised work
objects instead of passive evidence.

## Purpose / Big Picture

After this slice lands, a repository with a numbered planning corpus should be
supervised by plan, not by whatever lane names synthesis guessed from prose.
A human or agent should be able to point at one plan file and answer, with no
interpretation layer in between, whether that plan is modeled, which executable
subwork exists beneath it, what proof contract guards it, what risk remains,
and what the next operator move is.

The user-visible proof is concrete. In `/home/r/coding/rXMRbro`, a plan such as
`plans/005-craps-game.md` should stop being listed only under `evidence_paths`
and instead appear as one deterministic plan record with a stable status,
drill-down work items for its major milestones, explicit proof commands, and a
Paperclip category that mirrors the same truth. The system should not rely on
`synth create` or `synth evolve` making a lucky judgment call from prose. The
mapping must become deterministic, testable, and reviewable.

## Progress

- [x] (2026-03-21 17:05Z) Re-read `PLANS.md`, the current plan-first redesign
  draft, the Fabro synthesis code, and the current `rXMRbro` generated package
  to confirm the real trust gaps before revising this ExecPlan.
- [x] (2026-03-21 17:12Z) Confirmed that `rXMRbro` still treats
  `plans/005-craps-game.md` and the rest of the numbered game plans as evidence
  rather than as executable supervised objects.
- [x] (2026-03-21 17:20Z) Identified the missing design pieces in the earlier
  draft: a checked-in mapping contract, milestone-child decomposition for
  multi-surface plans, proof-contract precedence rules, ambiguity handling, and
  fixture-driven validation using a real composite plan.
- [x] (2026-03-21 17:33Z) Rewrote this document into `PLANS.md`-conformant
  ExecPlan form and incorporated the deterministic mapping recommendations.
- [x] (2026-03-21 17:46Z) Expanded the redesign into an end-to-end execution
  framework with explicit workflow archetypes, archetype-selection rules,
  integration rules, and all-27-plan validation requirements.
- [x] (2026-03-21 18:02Z) Added a Paperclip-specific pass so the web dashboard,
  synchronized issue hierarchy, work products, and generated agents are all
  tailored to live plan execution instead of to lane-centric frontier state.
- [x] (2026-03-21 18:19Z) Completed an engineering-review pass and identified
  the remaining trust gaps: no explicit portfolio scheduler policy, no
  cross-plan shared-surface lock model, and no shadow-mode cutover path from
  lane-centric truth to plan-centric truth.
- [x] (2026-03-21 18:31Z) Reviewed `coding/rsocietyv2/ralph/SPEC.md` and
  extracted the strongest deterministic contract patterns: AC-level `Where`,
  `How`, `State`, `Pass/fail`, and `Rollback` fields plus scoped backpressure
  gates and explicit rejection behavior.
- [x] (2026-03-21 19:12Z) Reconciled the plan against current repository
  reality and confirmed that maintenance mode already exists in
  `raspberry-supervisor`, so execution should start from shared plan-registry
  and plan-status work rather than redoing the maintenance slice.
- [x] (2026-03-21 19:18Z) Found a crate-cycle constraint in the original plan:
  `fabro-synthesis` already depends on `raspberry-supervisor`, so the first
  shared plan-registry implementation must start in `raspberry-supervisor` or
  a future shared crate rather than only in `fabro-synthesis`.
- [x] (2026-03-21 19:41Z) Added file-based todo tracking for execution of the
  expanded plan so the first implementation slice is tracked outside the
  ExecPlan itself.
- [x] (2026-03-21 19:54Z) Implemented a shared deterministic
  `raspberry-supervisor::plan_registry` module with tests for numbered-plan
  ingestion, dependency extraction, composite-plan detection, and mapping
  contract discovery.
- [x] (2026-03-21 20:03Z) Reworked `plan_status.rs` and the CLI plan-matrix
  test expectations to consume the shared registry and report richer
  plan-first rows including mapping status, child count, and next operator
  move.
- [x] (2026-03-21 20:08Z) Verified the first slice with focused tests and type
  checks: `cargo test -p raspberry-supervisor plan_registry`,
  `cargo test -p raspberry-supervisor plan_status`,
  `cargo test -p raspberry-cli plan_matrix`, and
  `cargo check -p raspberry-supervisor -p raspberry-cli`.
- [x] (2026-03-21 20:27Z) Wired `fabro-synthesis` authoring through the shared
  plan registry so `author_blueprint_for_create/evolve` now emits deterministic
  registry-backed units for numbered plans, including meta-plan and
  contract-aware prompt context for composite plans.
- [x] (2026-03-21 20:31Z) Verified the synthesis slice with
  `cargo test -p fabro-synthesis create_authoring -- --nocapture`,
  `cargo test -p fabro-synthesis rxmragent -- --nocapture`, and
  `cargo check -p fabro-synthesis`.
- [x] (2026-03-21 20:35Z) Ran live preview evolve against
  `/home/r/coding/rXMRbro` and confirmed that the generated preview now adds
  doctrine-derived units such as `master`, `craps`, `sic-bo`, `video-poker`,
  `roulette`, `dice`, and `faucet` instead of leaving those plans only in
  `evidence_paths`.
- [x] (2026-03-21 20:39Z) Ran live `raspberry plan-matrix` against the checked
  in `rXMRbro` manifest and confirmed that the local operator view now exposes
  mapping status and next operator move per numbered plan.
- [x] (2026-03-21 20:58Z) Extended the shared plan registry to parse checked-in
  mapping contract data rather than only detecting contract filenames, with
  override support for title, category, child ids, dependencies, and execution
  shape metadata.
- [x] (2026-03-21 21:04Z) Added a first Paperclip plan-first presentation pass:
  company markdown and root plan documents now lead with plan status summary,
  plans needing attention, and the plan matrix when local plan status is
  available.
- [x] (2026-03-21 21:10Z) Added the canonical proving-ground mapping contract at
  `/home/r/coding/rXMRbro/malinka/plan-mappings/005-craps-game.yaml` and
  confirmed that `plans/005-craps-game.md` now renders as `mapped` in the live
  `rXMRbro` plan matrix instead of remaining on the generic synthesized
  fallback path.
- [x] (2026-03-21 21:12Z) Verified the mapping-contract and Paperclip pass with
  `cargo test -p raspberry-supervisor plan_registry -- --nocapture`,
  `cargo test -p fabro-cli paperclip -- --nocapture`,
  `cargo check -p raspberry-supervisor -p fabro-cli`, plus live reruns of
  `raspberry plan-matrix` and `fabro synth evolve` against `rXMRbro`.
- [x] (2026-03-21 21:31Z) Removed mapping review as a first-class status from
  the plan-first model. The registry now always emits a mapping, plan status
  always reports `mapped`, and Paperclip no longer treats missing checked-in
  contracts as a blocking state.
- [x] (2026-03-21 21:38Z) Regenerated `/home/r/coding/rXMRbro/fabro` from
  scratch with `fabro synth create --target-repo /home/r/coding/rXMRbro
  --program rxmragent` after the full directory wipe, restoring a fresh
  plan-first control plane.
- [x] (2026-03-21 21:40Z) Verified the regenerated baseline with live
  `raspberry plan-matrix` output showing all numbered `rXMRbro` plans as
  `mapped` and dispatched from a best-effort execution shape rather than a
  mapping-review queue.
- [x] (2026-03-21 21:53Z) Extended `fabro synth create` so it now writes
  synthesized mapping snapshots under `target_repo/malinka/plan-mappings/` as
  part of create, using the same shared registry that blueprint authoring
  consumes.
- [x] (2026-03-21 21:56Z) Verified the new create behavior with the focused
  CLI test `cargo test -p fabro-cli --test synth
  synth_create_writes_plan_mapping_snapshots -- --nocapture` and a live rerun
  of `fabro synth create` against `/home/r/coding/rXMRbro`, confirming that
  all numbered plan mapping files are generated automatically.
- [x] (2026-03-21 22:03Z) Confirmed the remaining core gap for handoff: the
  regenerated package now proves automatic plan-to-bootstrap workflow mapping,
  but it still does not emit milestone-level child workflows such as
  `craps-casino-core` or `craps-provably-fair`. That remaining step must be
  treated as an explicit deliverable in the next batch.
### Completed deliverables

- [x] (2026-03-21 ~23:00Z) Milestone-level child workflow generation. Composite
  plans emit children with typed archetypes, review profiles, proof commands,
  owned surfaces, and AC contracts. `synth create --no-decompose` produces ~200
  child workflows from 27 plans using deterministic heuristics.
- [x] (2026-03-21 ~23:30Z) Opus decomposition as default `synth create` path.
  Each composite plan's markdown is sent to
  `claude -p --dangerously-skip-permissions --model claude-opus-4-6` which
  returns a YAML mapping contract with clean milestone-level children.
  `--no-decompose` falls back to heuristics for offline/CI.
- [x] (2026-03-21 ~23:15Z) Model routing: review-class nodes are Opus-first
  with ordered fallback through gpt-5.4, gemini-3.1-pro-preview, kimi, and
  MiniMax. Auth uses local CLI backends and session credentials, no API key
  needed for the default path.
- [x] (2026-03-21 ~23:20Z) Maintenance mode validated: 3 tests pass covering
  read-only access, dispatch refusal, and autodev stop reason.
- [x] (2026-03-21 ~22:45Z) Proof-contract verify gates wired: child workflows
  use proof commands from mapping contracts (e.g.,
  `cargo test -p casino-core --features craps`) instead of `test -f`.
- [x] (2026-03-21 ~22:50Z) Golden assertion proven: regenerated rXMRbro
  package goes from 27 plan files to milestone-level workflow units with
  proper archetypes, proof contracts, and dependency chains.
- [x] (2026-03-22 ~01:00Z) Full plan content injected into worker prompts.
  Both bootstrap and implementation workers now receive the complete plan
  markdown in their `prompt_context`, not just a file path reference. Workers
  have domain knowledge (design decisions, specifications, architectural
  context) without needing to discover and read the plan file themselves.

### Remaining deliverables

- [x] (2026-03-22 ~01:30Z) Parallel Opus decomposition. Single `claude -p`
  call with agent team manifest replaces sequential per-plan calls.

- [x] (2026-03-22 ~02:00Z) `synth review` subcommand. Adversarial eng-review
  of mapping contracts via Opus. Read-only — produces report, no mutations.

- [x] (2026-03-22 ~02:00Z) Non-destructive `synth create/evolve`. Removed
  `wipe_package_root()`. Existing Opus/manual mapping contracts are preserved,
  heuristic snapshots are treated as cache-only, and stale heuristic contracts
  are refreshed from current plan text instead of pinning old child IDs.

- [x] (2026-03-22 ~02:00Z) Renamed output directory from `fabro/` to
  `malinka/`. Configurable via `DEFAULT_PACKAGE_DIR` constant.

- [x] (2026-03-22 ~02:00Z) `synth evolve` child workflow support confirmed
  already working via `merge_missing_doctrine_units`.

- [x] (2026-03-22 ~03:00Z) Final schema tightening. Consolidated to 4
  archetypes (`implement`, `integration`, `orchestration`, `report`) and 4
  review profiles (`standard`, `foundation`, `hardened`, `ux`) plus explicit
  child `lane_kind` (`platform`, `service`, `interface`, `artifact`,
  `integration`, `orchestration`). All backward-compat aliases preserved in
  `from_str()`. Previous 11 archetypes and 8 profiles folded — every removed
  variant maps to `implement` + appropriate profile, while `lane_kind`
  preserves service/interface execution semantics. Profile-specific workflow
  graphs: `hardened` adds deep review + recheck nodes, `foundation` adds Opus
  escalation, `ux` adds acceptance gate. Profile-specific convergence:
  `hardened` 5 visits, `foundation` 4, others 3.

- [x] (2026-03-22 ~02:30Z) Portfolio scheduler module. Surface locks,
  dependency filtering, shared-foundation priority boost. 3/3 tests pass.

- [x] (2026-03-22 ~02:00Z) `synth genesis` command. Opus as interim CEO,
  180-day turnaround plan, and direct `synth create` from the generated
  `genesis/` planning root without copying numbered plans into the repo root.
  The command now refuses to reuse a non-empty `genesis/` directory.

- [x] (2026-03-22 ~02:15Z) Reviewer fallback chain: Opus 4.6 →
  gpt-5.4 → gemini-3.1-pro-preview → kimi → MiniMax via shared
  `[llm.fallbacks]` in generated run configs. Review-class nodes
  (`review`, `challenge`, `deep_review`, `escalation`) are now Opus-first.

- [x] (2026-03-22 ~04:00Z) Paperclip plan-root-keyed sync. Layered
  plan-first sync on top of existing lane sync. `PlanDashboardModel`
  joins `PlanRegistry` with `PlanMatrix` for sync-ready plan data.
  Plan root + child issues, documents, comments, work products, and
  plan-root agents. Company markdown puts plans first, frontier second.
  Bootstrap state persists `planSync` alongside `frontierSync`.
  6 new tests. Zero errors in paperclip.rs.
  Where: `fabro-cli/src/commands/paperclip.rs`
  Proof: `cargo test -p fabro-cli paperclip_plan_sync -- --nocapture`

- [x] (2026-03-22 ~04:30Z) Shadow-mode cutover. Three-phase migration
  with `CutoverPhase` enum (Shadow/ParityReview/PlanFirst), pure
  `compare_legacy_and_plan_truth()` parity comparison, and
  `render_parity_report()` for human-readable output. Cutover safe
  when: no unmodeled plans, no failures, all lanes map to plans.
  Includes rollback instructions. 4/4 tests pass.
  Where: `raspberry-supervisor/src/plan_cutover.rs` (new module)
  Proof: `cargo test -p raspberry-supervisor plan_cutover -- --nocapture`

## Surprises & Discoveries

- Observation: deterministic heuristics actively hurt Opus decomposition quality.
  Evidence: when `--decompose` ran against heuristic-seeded mapping contracts,
  Opus tended to echo back the noisy baseline (e.g., mines had 11 children with
  duplicate pairs) instead of restructuring from the plan milestones. Switching
  to Opus-from-scratch produced cleaner 5-7 child decompositions per plan.
  Decision: Opus is now the default path; heuristics are `--no-decompose` fallback.

- Observation: no API key is needed for Opus in fabro.
  Evidence: `fabro-workflows/src/backend/cli.rs` line 201 shells out to
  `claude -p --dangerously-skip-permissions --model claude-opus-4-6`, which
  inherits the user's Claude Code session auth. The `ANTHROPIC_API_KEY` env var
  path is only used by the unused `api` backend.

- Observation: the `security_sensitive` review profile was over-assigned by both
  heuristic and early Opus runs.
  Evidence: 59 of 211 children got `security_sensitive` because plan bodies
  mention “verification” extensively (it's a provably-fair casino). The
  tightened Opus prompt with explicit archetype→profile guidance table reduced
  this, but ongoing prompt refinement is needed.

- Observation: the plan registry and synthesis correctly share data through
  `raspberry-supervisor::plan_registry` without crate cycles.
  Evidence: `fabro-synthesis` depends on `raspberry-supervisor` and imports
  `load_plan_registry`, `PlanRecord`, `PlanChildRecord`, `WorkflowArchetype`,
  `ReviewProfile`. The reverse dependency does not exist.

- Observation: mapping contracts written by `write_plan_mapping_snapshots()`
  run AFTER blueprint authoring in the `synth create` flow, creating a
  chicken-and-egg problem for child workflow generation.
  Evidence: the first run against rXMRbro produced 28 flat lanes because
  `plan.children` was empty during `derive_registry_plan_intents()`. Fixed by
  adding `infer_child_records_from_ids()` in `planning.rs` as a fallback when
  no enriched mapping contract exists yet.

- Observation: the existing `WorkflowTemplate` enum is sufficient for all
  archetype workflow shapes.
  Evidence: `Implementation` handles implement_module, service_surface,
  tui_surface, verification_only, acceptance_and_balance, and migration.
  `Bootstrap` handles bootstrap_contract. `Integration` handles
  integration_only. `Orchestration` handles orchestration_program.
  `RecurringReport` handles review_or_report_only. No new template variants
  were needed — the differentiation comes from verify_command and prompt_context.

## Decision Log

- Decision: this redesign will no longer treat “one plan file becomes one
  implementation lane” as sufficient.
  Rationale: many real plans, especially game plans like craps, are composite
  delivery contracts that cross crate, binary, interface, and verification
  boundaries. A single implementation lane would blur ownership and make proof
  contracts dishonest.
  Date/Author: 2026-03-21 / Codex

- Decision: plan mapping must become a checked-in contract with heuristics as a
  fallback, not the other way around.
  Rationale: synthesis can assist with inference, but operator trust requires a
  stable, reviewable record of how a plan maps to executable work and proof.
  Date/Author: 2026-03-21 / Codex

- Decision: proof commands declared in a plan mapping contract take precedence
  over category-level defaults and over repo-level guesses.
  Rationale: a plan such as `plans/005-craps-game.md` already contains more
  precise proof commands than the current synthesis heuristics can infer.
  Date/Author: 2026-03-21 / Codex

- Decision: ambiguous plans must surface an explicit review-needed status
  rather than silently synthesizing a low-confidence workflow.
  Rationale: the current failure mode is false certainty. It is better to stop
  with a deterministic “mapping incomplete” state than to route expensive or
  misleading implementation work from a guessed interpretation.
  Date/Author: 2026-03-21 / Codex

- Decision: the worked `rXMRbro` example in this plan will use
  `plans/005-craps-game.md` as the canonical composite-plan fixture.
  Rationale: it is broad enough to force milestone decomposition and specific
  proof selection, which makes it the best trust-surface test for this redesign.
  Date/Author: 2026-03-21 / Codex

- Decision: the model-routing task will be reframed to preserve and formalize
  the current MiniMax-first execution policy rather than pretending the code
  still defaults all write stages to `claude-opus-4-6`.
  Rationale: the current render code already uses MiniMax for write and
  challenge stages. The plan should reflect repository reality, not a stale
  diagnosis.
  Date/Author: 2026-03-21 / Codex

- Decision: the final design must use a small explicit workflow archetype
  catalog rather than treating “implementation” as the universal answer.
  Rationale: the current broad families are too coarse for multi-surface plans,
  migration plans, verification-only work, orchestration work, and acceptance
  work. A bounded archetype catalog is a better compromise than either one
  generic workflow or an unbounded custom workflow per plan.
  Date/Author: 2026-03-21 / Codex

- Decision: child workflow definitions must be the product of
  `workflow archetype × review profile`, not of archetype alone.
  Rationale: with effectively abundant Minimax and Kimi capacity, the harness
  should spend inference on repeated cheap review and adversarial cycles while
  reserving Opus 4.6 for selective adjudication and high-blast-radius signoff.
  Date/Author: 2026-03-21 / Codex

- Decision: every executable child must carry an AC-style contract modeled on
  Ralph's deterministic task schema.
  Rationale: cheap models do exactly what the harness asks. To get correctness,
  each child needs explicit fields for where it changes code, how behavior
  changes, what state is persisted, what proves success, and what reopens the
  child.
  Date/Author: 2026-03-21 / Codex

- Decision: every one of `rXMRbro`'s 27 numbered plans must be covered by a
  deterministic mapping fixture or contract-backed assertion before this redesign can be
  called complete.
  Rationale: confidence in the framework cannot come from a single worked
  example, even a strong one like craps. The design must prove that all 27
  plans receive an explicit execution shape, whether synthesized or refined by
  a checked-in contract.
  Date/Author: 2026-03-21 / Codex

- Decision: Paperclip's web dashboard must become plan-root-first and must use
  lane detail only as drill-down context beneath plans.
  Rationale: the operator mental model after this redesign is “what is the
  status of plan 005?” rather than “what is the status of lane
  `casino-core:craps`?” The dashboard must match the plan-first control plane
  or it will keep reintroducing the old lane-centric worldview.
  Date/Author: 2026-03-21 / Codex

- Decision: generated Paperclip agents and synchronized issue documents must be
  aligned to plans and plan children, not just to blueprint units and lanes.
  Rationale: if the web UI is plan-first but the generated agents still think in
  lane-only terms, operators and agents will drift into different mental models
  of the same work.
  Date/Author: 2026-03-21 / Codex

- Decision: the plan-first framework must ship with a portfolio scheduler and a
  global surface-lock policy, not just with a better mapper and dashboard.
  Rationale: correctly mapping 27 plans is not enough if the executor can still
  pick the wrong child next or run conflicting children at the same time.
  Date/Author: 2026-03-21 / Codex

- Decision: cutover from lane-centric truth to plan-centric truth must happen in
  shadow mode first, with explicit parity checks before plan-first status
  becomes authoritative.
  Rationale: the current system already has live frontier refresh, Paperclip
  sync, and autodev hooks. Replacing those truth surfaces in one jump would
  create a high-blast-radius migration with weak rollback.
  Date/Author: 2026-03-21 / Codex

- Decision: the first implementation slice will place the plan registry in
  `raspberry-supervisor` instead of only in `fabro-synthesis`.
  Rationale: `fabro-synthesis` already depends on `raspberry-supervisor`, so a
  synthesis-only registry would prevent `plan_status.rs` and the future
  scheduler from consuming the same deterministic records without a crate
  cycle. Synthesis can consume the shared registry immediately, and the code
  can later move to a dedicated shared crate if that becomes worthwhile.
  Date/Author: 2026-03-21 / Codex

- Decision: keep MiniMax as the default write/polish model, but make every
  review-class node Opus-first with ordered fallback through gpt-5.4,
  gemini-3.1-pro-preview, kimi, and MiniMax.
  Rationale: the project wants the strongest available reviewer by default,
  while lanes must degrade instead of stopping when one provider or model is
  unavailable. The CLI backends already support local-auth execution.
  Date/Author: 2026-03-21 / User + Codex

- Decision: Opus decomposition is the default `synth create` path, not opt-in.
  Deterministic heuristics are the `--no-decompose` fallback for offline/CI.
  Rationale: heuristic-seeded mappings produce noisy duplicates that Opus then
  echoes back. Starting from plan text only produces cleaner 5-7 child
  decompositions that map 1:1 to plan milestones.
  Date/Author: 2026-03-21 / User + Claude

- Decision: the master plan (001) does not generate child workflows even though
  it is composite with many milestones.
  Rationale: meta-category plans are roadmap documents, not implementation
  plans. Their milestones reference other numbered plans that have their own
  child decompositions. Generating children for meta plans would create
  duplicate work items with broken dependency chains.
  Date/Author: 2026-03-21 / Claude

## Outcomes & Retrospective

The plan-first redesign is implemented and proven against `rXMRbro`. The
framework now decomposes 27 numbered plans into ~200 milestone-level child
workflows, each with typed archetypes, review profiles, proof commands, owned
surfaces, and AC contracts.

The most important architectural outcome is that decomposition authority moved
from deterministic Rust heuristics to Claude Opus 4.6. The heuristic path
remains as a `--no-decompose` fallback, but the default `synth create` flow
sends each composite plan's markdown to Opus, which returns a structured YAML
mapping contract with concise child IDs, correct archetypes, and proof commands
extracted from the plan text. The re-render then consumes those mapping
contracts to generate full workflow graphs, prompts, and run configs.

The model routing is now: MiniMax M2.7 Highspeed for write/polish stages, and
Claude Opus 4.6 for review-class work with fallback order gpt-5.4 →
gemini-3.1-pro-preview → kimi → MiniMax. The CLI backends inherit local auth,
so the default path does not require API-key management.

Key metrics from the proving-ground run:
- 27 plan files → 27 bootstrap + ~150 implementation + ~40 integration + 1
  recurring_report workflows
- Craps decomposes into 6 children: casino-core, provably-fair, house-handler,
  tui-screen, e2e-verify, acceptance
- Each implementation child gets a full workflow: preflight → implement →
  verify → quality → challenge → review → audit
- Verify gates use real proof commands (`cargo test -p casino-core --features
  craps`) instead of `test -f` artifact checks

Remaining work:
- Deeper Paperclip plan-root-keyed sync (issue hierarchy, work products,
  generated agents aligned to plan roots instead of lanes)
- Portfolio scheduler with global surface-lock policy
- Shadow-mode cutover from lane-centric to plan-centric truth
- Review-profile-specific workflow bundles (convergence gates, nemesis passes,
  escalation rules) — currently all archetypes use the same Implementation
  workflow graph shape

## Context and Orientation

Fabro is the workflow engine in this repository. Its synthesis layer lives
primarily in `lib/crates/fabro-synthesis/src/planning.rs`,
`lib/crates/fabro-synthesis/src/render.rs`, and
`lib/crates/fabro-synthesis/src/blueprint.rs`. The Opus decomposition pass
lives in `lib/crates/fabro-cli/src/commands/synth.rs`.

Raspberry is the repo-level supervisor. The supervisor logic lives in
`lib/crates/raspberry-supervisor/src/`, with the shared plan registry at
`plan_registry.rs`, plan status computation at `plan_status.rs`, and the
autodev orchestration loop at `autodev.rs`.

The `synth create` flow now works as follows:

1. `author_blueprint_for_create()` reads `plans/*.md`, loads the plan registry,
   and calls `derive_registry_plan_intents()` which emits a parent bootstrap
   `LaneIntent` per plan plus child `LaneIntent`s for composite plans.
2. For each composite plan, `run_opus_decomposition()` sends the plan markdown
   to `claude -p --model claude-opus-4-6` and writes the returned YAML to
   `malinka/plan-mappings/{plan-stem}.yaml`.
3. The blueprint is re-authored consuming the Opus-written mapping contracts,
   which now carry enriched `PlanChildRecord` entries with archetypes,
   `lane_kind`, review profiles, proof commands, owned surfaces, and AC
   contract fields.
4. `render_blueprint()` generates workflow graphs, prompts, run configs, and
   the program manifest.

The key data types in the plan registry are:
- `PlanRecord`: plan_id, path, title, category, composite, dependencies,
  declared_child_ids, children (Vec<PlanChildRecord>)
- `PlanChildRecord`: child_id, title, archetype (WorkflowArchetype),
  lane_kind (LaneKind), review_profile (ReviewProfile), proof_commands,
  owned_surfaces, AC fields
- `WorkflowArchetype`: 4 variants covering execution graph shape
- `ReviewProfile`: 4 variants covering rigor level

The archetype-to-template mapping reuses existing `WorkflowTemplate` variants:
- `ImplementModule/CrossSurface/ServiceSurface/TuiSurface/VerificationOnly/
  AcceptanceAndBalance/Migration` → `WorkflowTemplate::Implementation`
- `BootstrapContract` → `WorkflowTemplate::Bootstrap`
- `IntegrationOnly` → `WorkflowTemplate::Integration`
- `OrchestrationProgram` → `WorkflowTemplate::Orchestration`
- `ReviewOrReportOnly` → `WorkflowTemplate::RecurringReport`

In this plan, the phrase “plan mapping contract” means a checked-in,
deterministic record of how one numbered plan maps to executable work. That
contract must identify the plan, classify it, describe its dependencies, define
its owned surfaces, and state the proof commands required for each executable
child. The contract may be stored as frontmatter in the plan file or as a
sidecar file under the repository, but it must be versioned with the repo and
read by synthesis before heuristics are allowed to run.

In this plan, the phrase “plan root” means the master supervised object for one
plan file. A plan root can have zero or more executable child work items. A
simple plan might need only a bootstrap child or only an implementation child.
A composite plan such as `plans/005-craps-game.md` needs multiple children that
mirror its major milestones.

In this plan, the phrase “proof contract” means the commands and artifacts that
must succeed before Raspberry can advance the child work item to the next state.
The proof contract is not an informal note. It is a deterministic part of the
plan mapping and must be surfaced in status and Paperclip.

In this plan, the phrase “workflow archetype” means a named workflow shape that
defines how a child work item is executed. An archetype decides which artifacts
must be produced, what the proof gate looks like, whether integration is part
of the child, whether the child is allowed to own multiple surfaces, and what
kind of review is required before promotion. The redesign in this document no
longer assumes that every code-bearing child should use the same generic
implementation workflow.

In this plan, the phrase “review profile” means the rigor stack attached to a
child independently of the child's archetype. A review profile defines how many
cheap-model implementation or review cycles may run, which adversarial or
nemesis passes are required, which deterministic gates must be green, and what
conditions escalate the child to Opus 4.6.

In this plan, the phrase “convergence gate” means the rule that determines when
the cheap-model loop has done enough. Because Minimax and Kimi capacity are
effectively abundant for this system, the goal is not to minimize cycles. The
goal is to keep cycling until the deterministic gates are green, the required
cheap-model reviews have stabilized, and no open critical findings remain.

In this plan, the phrase “AC contract” means an acceptance-criteria-style child
contract modeled on the deterministic structure used in
`coding/rsocietyv2/ralph/SPEC.md`. Each executable child must say:

- `Where`: which repo-relative surfaces it owns
- `How`: what behavior or state transition changes
- `State`: what durable state, artifacts, or side effects must exist
- `Pass/fail`: which deterministic commands or checks prove success
- `Rollback`: what invalidates the child and reopens it

This is the minimum harness detail needed for cheap models to execute correctly.

In this plan, the phrase “portfolio scheduler” means the Raspberry logic that
chooses which plan child should run next across the entire repo. It is not
enough for the scheduler to know that a child is “ready”. It must also know
which plan it belongs to, which surfaces it owns, which other plans depend on
it, which mapping produced it, and whether running it would conflict with
another in-flight child.

In this plan, the phrase “surface lock” means a deterministic claim on one or
more owned surfaces such as `crates/casino-core`, `crates/provably-fair`,
`bin/house`, or `crates/tui`. The scheduler must not dispatch two children with
overlapping non-shareable surface locks at the same time unless the mapping
contract explicitly marks the overlap as safe.

## What Already Exists

Several important pieces already exist and should be extended rather than
replaced.

- `lib/crates/fabro-synthesis/src/planning.rs`,
  `lib/crates/fabro-synthesis/src/render.rs`, and
  `lib/crates/fabro-synthesis/src/blueprint.rs` already provide the seams where
  plan ingestion, workflow rendering, and package generation happen.
- `lib/crates/raspberry-supervisor/src/plan_status.rs` and
  `lib/crates/raspberry-cli/src/main.rs` already provide the first plan-matrix
  read path. This redesign should evolve them into the authoritative plan
  status layer rather than start over.
- `lib/crates/fabro-cli/src/commands/paperclip.rs` already knows how to build a
  local company bundle, refresh synchronized issues, attach artifacts, and
  include a rendered plan matrix in company markdown.
- `lib/crates/raspberry-supervisor/src/autodev.rs` already has a live hook that
  refreshes the Paperclip dashboard after frontier movement.
- `rXMRbro` already provides the strongest real fixture: 27 numbered plans with
  a mix of simple and composite work shapes, plus a checked-in generated
  package that still exposes the current lane-centric limitations.

## NOT in Scope

This plan intentionally does not include the following work, even though the
framework may eventually benefit from it.

- Replacing Fabro workflows with a totally new execution engine.
  Rationale: the goal is to deepen the current Fabro-Raspberry-Paperclip stack,
  not to restart the architecture.
- Solving every future repo’s plan taxonomy in this first slice.
  Rationale: `rXMRbro` is the proving ground and must be covered completely, but
  the framework should remain extensible rather than pretending to know every
  future repo shape up front.
- UI redesign of Paperclip itself beyond the data and synchronization surfaces
  this repo controls.
  Rationale: the deliverable here is plan-first dashboard truth via sync,
  issues, documents, work products, and generated company markdown, not a fork
  of the upstream web app.
- Review-profile-specific workflow graph shapes (e.g., extra nemesis passes for
  security_sensitive, Monte Carlo loops for economic_correctness).
  Rationale: all archetypes currently use the same Implementation workflow
  graph. The review profile is metadata carried through to prompts, but the
  graph shape does not yet vary per profile. This is a future refinement.

## Plan of Work

The end-to-end architecture after this redesign should look like this:

    plans/*.md + mapping contracts
                |
                v
  raspberry-supervisor::plan_registry
                |
                v
      plan roots + child records + archetypes
                |
                +----------------------+
                |                      |
                v                      v
      blueprint/render output      plan_status.rs
                |                      |
                v                      v
       portfolio scheduler -----> plan matrix / snapshots
                |                      |
                v                      v
         Fabro workflow runs ---> Paperclip dashboard sync

The scheduler path after cutover should look like this:

    all plan children
          |
          v
    dependency filter
          |
          v
    mapping-provenance filter
          |
          v
    surface-lock filter
          |
          v
    archetype dispatch policy
          |
          v
    Fabro run + proof + review
          |
          v
    status rollup + Paperclip refresh

The first change is to add a repository-wide maintenance lock so the supervisor
can be put into a read-only state while the control plane is being repaired or
re-synthesized. This work touches `lib/crates/raspberry-cli/src/main.rs`,
`lib/crates/raspberry-supervisor/src/autodev.rs`,
`lib/crates/raspberry-supervisor/src/dispatch.rs`, and
`lib/crates/fabro-cli/src/commands/paperclip.rs`. The maintenance contract is a
repo-local `.raspberry/maintenance.json` file with fields `enabled`, `reason`,
`set_at`, and `set_by`. Read-only commands such as status and plan-matrix must
continue to function, while dispatching commands must refuse to launch work and
must report the maintenance reason clearly.

The next change is the heart of this redesign: create a shared plan-ingestion
layer, starting with `lib/crates/raspberry-supervisor/src/plan_registry.rs`,
and make it the canonical plan registry surface that both supervisor and
synthesis consume. This module must parse `plans/*.md` into stable registry
records. A registry record must include at minimum the `plan_id`, `path`,
`title`, `category`, `composite` flag, deterministic dependency references,
whether bootstrap work is required, whether implementation work is required,
and the proof or review expectations. The registry must also carry the result
of reading the checked-in plan mapping contract when one exists.

The registry work must add a checked-in plan mapping contract with deterministic
precedence. The contract can be implemented either as structured frontmatter at
the top of a plan file or as a sidecar file under a directory such as
`malinka/plan-mappings/`. The exact storage shape is less important than the
behavior. The contract must support a simple plan and a composite plan. For a
simple plan it must be able to say, in checked-in data, that one plan maps to
one executable child with one owned-surface set and one proof contract. For a
composite plan it must support multiple child records, each with a stable child
id, title, lane kind, workflow family, owned surfaces, dependencies, and proof
commands.

The mapping contract must be the first source of truth. Synthesis may still
infer details from prose, but only when the mapping contract does not provide
them. If the plan is composite, or spans multiple major surfaces, and no
mapping contract exists yet, the registry must still emit the best synthesized
mapping it can and preserve enough metadata for humans to refine it later.

The redesign must also add a bounded workflow archetype catalog and a
deterministic child-to-archetype selection rule. “Implementation” remains one
important archetype, but it is no longer the universal default. The catalog
must be small enough to stay understandable and testable, but rich enough to
represent the real work shapes present in `rXMRbro` and similar repos. At
minimum the catalog must include:

- `bootstrap_contract`, for plan children whose job is to author or review the
  first honest slice contract for one owned surface.
- `implement_module`, for a code-bearing child that owns one primary surface or
  crate and proves it with direct build or test commands.
- `implement_cross_surface`, for a child that intentionally spans multiple
  tightly-coupled owned surfaces and cannot be split honestly into smaller
  children.
- `service_surface`, for service or daemon work that requires proof plus a
  deterministic health or protocol gate.
- `tui_surface`, for user-visible terminal or client work that requires build
  proof and artifact-backed acceptance evidence.
- `verification_only`, for verifier, audit, or replay logic whose main output
  is proof rather than product behavior.
- `integration_only`, for children whose job is to wire already-built surfaces
  together and prove the integration boundary.
- `acceptance_and_balance`, for Monte Carlo, ignored edge-case, or behavioral
  acceptance work that should not be disguised as module implementation.
- `migration`, for schema, protocol, or compatibility changes where the main
  risk is rollout and data correctness rather than ordinary module coding.
- `orchestration_program`, for plan children that coordinate child programs,
  external loops, or cross-program sequencing rather than writing product code.
- `review_or_report_only`, for meta or operator-facing work that produces a
  durable report or review artifact but no implementation child.

The redesign must add a bounded review-profile catalog separate from the
archetype catalog. At minimum the profile set must include:

- `standard`, for ordinary code-bearing children with normal deterministic proof
  plus one or more cheap-model review loops.
- `shared_foundation`, for children that modify shared contracts, shared types,
  or repo-wide interfaces used by many downstream plans.
- `security_sensitive`, for children that touch auth, trust boundaries,
  verification logic, external command execution, secret handling, or other
  security-critical behavior.
- `economic_correctness`, for children that affect settlement, payouts,
  balances, accounting, fairness proofs, or any invariant where small mistakes
  become high-cost product bugs.
- `user_visible`, for children where the main risk is UX or rendering
  correctness and artifact-backed acceptance evidence matters.
- `production_service`, for services, daemons, or agents where health,
  idempotency, and restart behavior matter.
- `migration_risky`, for children whose main risk is rollout, compatibility, or
  rollback correctness.

The workflow definition for a child must be the product of
`WorkflowArchetype × ReviewProfile`. For example:

- `implement_module × standard`
- `implement_module × economic_correctness`
- `service_surface × production_service`
- `tui_surface × user_visible`
- `implement_cross_surface × shared_foundation`
- `migration × migration_risky`

This is the level of harness specificity required for near-unlimited cheap-model
execution to be useful rather than noisy.

The selection rule must be deterministic. The mapping contract may specify the
archetype and review profile directly. If it does not, the registry may infer
them, but only from explicit signals such as milestone wording, owned surfaces,
proof commands, service-health requirements, migration language, security cues,
economic settlement cues, or the absence of any code surface. The registry must
not silently downgrade a multi-surface plan into a generic `implement_module`
child just because the plan mentions code, and it must not silently assign a
low-rigor review profile when the owned surfaces or proof contract indicate
shared-foundation, security, or economic risk.

The redesign must also define how child boundaries are chosen. A child work
item must be the smallest honest unit that satisfies all of the following
conditions: it has one clear user-visible purpose, one coherent proof contract,
one review boundary, and a stable owned-surface set. A child should be split if
combining it with another child would either blur proof, widen owned surfaces
without need, or force a generic workflow where a more precise archetype exists.
Conversely, a child should stay whole if splitting it would create fake
parallelism or force an integration boundary inside one inherently coupled
change.

Each child must also carry a deterministic AC contract, not just an id and a
proof command list. Borrowing the strongest pattern from Ralph, each child
record must explicitly declare:

- `Where`: repo-relative owned surfaces
- `How`: concise description of the intended behavior change
- `State`: expected durable artifacts, persisted state, or external side effects
- `Required tests`: concrete, scoped backpressure commands
- `Verification plan`: assertable pass or fail conditions
- `Rollback condition`: what reopens the child after it was considered complete

If any of these fields is missing for an executable child, the child must be
rejected as incomplete and sent back through contract repair or synthesized
fallback improvement rather than being dispatched.

The plan-status layer comes next. Create
`lib/crates/raspberry-supervisor/src/plan_status.rs` and make it responsible
for computing one master status row per plan root. The status row must answer:
is the plan represented in the blueprint, does it have executable child work,
does each child have a real proof contract, what is the current status of the
plan, what is the current risk, and what is the next operator move. The status
must roll up from actual child workflow state and durable artifacts, not from
optimistic assumptions about what synthesis probably emitted.

The redesign must then add a portfolio scheduler layer inside Raspberry. The
current lane scheduler already knows how to decide readiness for lanes. The new
work is to add plan-child-aware dispatch policy that chooses the next child from
the whole portfolio, not just from the old lane frontier. This scheduler must
use the deterministic child records, dependency edges, mapping provenance,
and surface locks to decide what can run now.

The portfolio scheduler must prefer the smallest unblocked child that:

- has all plan and child dependencies satisfied
- has a real proof contract or an explicitly approved report-only archetype
- does not conflict with any in-flight child on a non-shareable owned surface
- fits within the existing parallelism budget

When multiple children are eligible, the scheduler should prefer the one with
the smallest blast radius and the clearest path to unlocking downstream plans.
For `rXMRbro`, that means foundational or shared-surface children such as
`provably-fair`, `casino-core`, `house-agent`, `tui-shell`, and
`monero-infrastructure` can be chosen ahead of leaf game delivery work when the
dependency graph says they unblock many other plans.

The scheduler must also understand review profiles. Children with stronger
profiles should not necessarily run later; they should simply run with a richer
workflow bundle. The scheduler's job is to choose the right child. The workflow
bundle's job is to decide how much cheap review, adversarial review, and
escalation that child requires.

The current `raspberry` command surface must then gain a read-only plan matrix
command, implemented through `lib/crates/raspberry-cli/src/main.rs` and
`plan_status.rs`. The matrix must print one deterministic row per plan file and
must be safe to run even during maintenance mode. The command must make it easy
to see whether a plan is still evidence-only, is missing a mapping contract, is
blocked on an upstream plan, or is ready for implementation.

Paperclip synchronization must be reorganized around those plan roots. Instead
of exposing guessed lanes as the main top-level objects, sync must create one
root issue or category per plan and then attach child work state beneath it for
bootstrap, implementation, verification, review, and integration work. The
Paperclip surface must expose the same plan id, path, status, risk, and next
operator move that Raspberry reports locally.

This Paperclip pass must go further than simply changing the issue titles. The
live sync model in `lib/crates/fabro-cli/src/commands/paperclip.rs` must grow
from the current frontier-and-lane representation into a plan-dashboard
representation that can drive the web UI directly. The synchronized dashboard
must answer, at minimum:

- which plans are blocked, running, ready, or merge-ready right now
- which plan children are currently executing and what proof gate they are in
- which plans are running on synthesized mappings versus checked-in contracts
- which plans changed status since the last refresh
- which plan artifacts, proofs, and mapping contracts are available as work
  products or attachments
- which generated agents are responsible for a plan or plan child

The company dashboard markdown must become plan-first. The current “Lane Sets”
section in `build_company_markdown(...)` should be replaced or subordinated by
sections such as “Plan Status Summary”, “Plans Needing Attention”, “Plans In
Motion”, and “Contract Refinement Opportunities”. The plan matrix should not be an optional
appendix buried after lane details; it should be one of the primary sources of
operator truth in the company markdown and, where possible, in the synchronized
issue documents themselves.

The synchronized Paperclip issue hierarchy must become:

- one root coordination issue for the whole program, summarizing plan health
- one top-level issue per plan root
- optional child issues or documents for each plan child work item
- work products and attachments scoped to the plan root or child, not only to
  the old lane key

The synchronized issue descriptions and `plan` documents must also become
plan-first. The root plan issue should explain the plan purpose, current plan
status, current plan risk, next operator move, mapping provenance, and child
summary. Child documents may still mention owned lanes or blueprint surfaces,
but only as implementation detail under the plan.

The plan-first Paperclip design must remain live rather than static. The
existing autodev refresh hook in
`lib/crates/raspberry-supervisor/src/autodev.rs` already knows how to call
`fabro paperclip refresh`. This redesign must preserve that path and make the
resulting dashboard obviously reflect the latest plan state within one refresh
cycle. A stale dashboard indicator must appear when the last successful refresh
timestamp is older than the latest Raspberry state update or when maintenance
mode pauses active syncing.

Generated Paperclip agents must be tailored to the plan-first model. The bundle
generation logic in `build_company_bundle(...)` must stop implying that unit and
lane ownership are the only first-class identities. At minimum the bundle must
include:

- a mission-level CEO or reviewer that reasons about the plan portfolio
- an orchestrator that wakes Raspberry based on plan movement
- plan-root-aligned agents or metadata for plan oversight
- optional child-work agents when a plan has complex subwork worth delegating

If retaining unit or lane agents remains necessary for backward compatibility,
their markdown must still explain which plan roots and plan children they serve.

Paperclip work products and attachments must also become plan-first. For each
plan root, sync should expose:

- the plan file itself
- the current mapping contract or synthesized mapping snapshot
- the current plan status row or plan-status JSON
- relevant proof artifacts for children that reached verification or review

For the highest-risk composite plans such as `plans/005-craps-game.md`, the web
dashboard should make it possible for a human to click from the plan issue to
the active child work items and then to the concrete artifacts and proofs that
justify the current status.

The workflow engine itself must change accordingly. Cheap-model cycles are not a
scarce resource in this design, so workflow bundles should optimize for
correctness through repeated constrained review rather than for speed through
minimal loop counts. Each review profile must define:

- the default implementation model, usually Minimax M2.7 Highspeed
- the default independent reviewer model, Minimax or Kimi 2.5
- whether a nemesis or adversarial review pass is required
- the deterministic gates required before the next cycle may begin
- the escalation conditions that require Opus 4.6
- the convergence rule that declares the child done

At minimum the profile bundles should behave as follows:

- `standard`: Minimax implement -> Minimax self-review against AC contract ->
  Kimi or second Minimax adversarial review -> fix -> deterministic proof ->
  final cheap-model compliance check.
- `shared_foundation`: Minimax implement -> Kimi independent review ->
  deterministic proof -> second cheap-model review pass -> Opus 4.6 selective
  promotion review.
- `security_sensitive`: Minimax implement -> Minimax contract review -> Kimi
  adversarial review -> Minimax fix -> Minimax nemesis/security audit -> Kimi
  second-pass challenge -> deterministic proof -> Opus 4.6 selective signoff.
- `economic_correctness`: Minimax implement -> cheap-model invariant review ->
  property or simulation proof -> Kimi adversarial review -> fix -> second
  invariant check -> Opus 4.6 selective signoff when shared balances or payouts
  are affected.
- `user_visible`: Minimax implement -> cheap-model artifact or UX review ->
  deterministic build proof -> Opus 4.6 selective signoff -> acceptance artifact check.
- `production_service`: Minimax implement -> cheap-model service review ->
  deterministic health and restart proof -> optional Kimi challenge if the
  service touches critical control flow.
- `migration_risky`: Minimax implement -> deterministic forward and rollback
  proof -> cheap-model migration review -> Opus 4.6 escalation only when the
  rollback or compatibility story is still disputed.

The convergence rule must be explicit. Replace hardcoded small retry counts for
important children with bundle-specific completion criteria. A child is done
only when:

- all deterministic gates required by its profile are green
- all required cheap-model review bundles ran
- no open critical findings remain
- no new critical findings appeared in the most recent review cycle
- the AC contract fields are fully satisfied
- any required Opus 4.6 escalation or signoff completed

This convergence model matters because Minimax and Kimi will do exactly what the
harness says and no more. The harness must therefore encode what “enough
review” actually means.

Synthesis expansion then needs to move from “one plan record per plan” to
“one plan root plus deterministic executable children”. This is where the new
contract is exercised against a real composite plan. The renderer in
`lib/crates/fabro-synthesis/src/render.rs` and the blueprint model in
`lib/crates/fabro-synthesis/src/blueprint.rs` must support a plan root that
does not directly correspond to one current lane. Instead, a plan root may own
multiple child units or lanes, depending on which representation best preserves
the current package model without making status dishonest.

The crucial proof change is that verify contracts must come from the mapping
contract first. Category defaults remain useful, but only as lower-precedence
fallbacks. Repo-level guesses such as `cargo test`, `npm test`, or `pytest`
must be the last fallback, not the main mechanism. If no contract and no safe
fallback exist, the child must still emit the best available proof contract and
surface any weakness as risk metadata rather than as a missing mapping state. The renderer
must never emit `script="true"` for an executable child whose plan mapping
claims a real proof contract exists.

This redesign must use `plans/005-craps-game.md` as the worked composite-plan
fixture in tests. The registry and renderer must be able to recognize that the
craps plan is composite because it declares separate milestones across
`crates/casino-core`, `crates/provably-fair`, `bin/house`, `crates/tui`, end to
end verification, and acceptance work. The expected deterministic child work
for the fixture is:

- `craps-casino-core`, owning the game engine and variant work in
  `crates/casino-core/src/craps/` and `crates/casino-core/src/lib.rs`, with
  proof commands headed by `cargo test -p casino-core --features craps`.
- `craps-provably-fair`, owning the dice derivation and verification work in
  `crates/provably-fair/src/dice.rs`, `crates/provably-fair/src/verify.rs`, and
  `crates/provably-fair/src/lib.rs`, with proof commands headed by
  `cargo test -p provably-fair`.
- `craps-house`, owning `bin/house/src/games/craps.rs` and
  `bin/house/src/games/mod.rs`, with proof commands headed by
  `cargo build -p house` and `cargo test -p house`.
- `craps-tui`, owning `crates/tui/src/craps/`, `crates/tui/src/app.rs`, and
  any module exports required to enable the screen, with proof commands headed
  by `cargo build -p rxmr-play`.
- `craps-e2e`, owning the full integration and verifier wiring, with proof
  commands headed by `cargo build --workspace` plus a deterministic local verify
  transcript contract.
- `craps-acceptance`, owning Monte Carlo, ignored edge cases, and behavioral
  acceptance artifacts, with proof commands headed by
  `cargo test -p casino-core --features craps -- --include-ignored` and
  `cargo test -p provably-fair -- craps`.

The exact storage shape for those child work items may differ from the names
above, but the tests must prove that the mapping is explicit, stable, and not
collapsed into one generic “craps” implementation lane.

This plan must not stop at the craps example. The redesigned framework must
cover all 27 numbered `rXMRbro` plans with enough specificity that each plan is
either mapped to an explicit execution shape or blocked on explicit mapping
review. The expected top-level coverage is:

- `001-master-plan.md` as a composite roadmap plan whose children are
  `review_or_report_only` or `orchestration_program` work, not product
  implementation.
- `002-provably-fair-crate.md` as a proof-heavy platform plan with
  `bootstrap_contract`, `implement_module`, and `verification_only` children.
- `003-poker-game.md` and `004-blackjack-game.md` as composite game-delivery
  plans mixing game logic, TUI, and verification-oriented children.
- `005-craps-game.md` and `006-sic-bo-game.md` as composite dice-game plans
  spanning `casino-core`, `provably-fair`, `house`, `tui`, and acceptance work.
- `007-video-poker-game.md` through `012-plinko-game.md` as game-delivery plans
  whose exact child sets vary by scope but must still be classified explicitly
  rather than collapsed into one guessed implementation lane.
- `013-house-agent.md`, `014-tui-shell.md`, `015-monero-infrastructure.md`, and
  `016-casino-core-trait.md` as foundational multi-surface plans that require
  their own reviewed child structure rather than being treated as generic game
  specs.
- `017-roulette-game.md` through `026-dice-game.md` as additional game-delivery
  plans that must each receive at least a reviewed mapping with archetype
  assignments and proof expectations.
- `027-faucet.md` as an operator-facing product plan that may involve service,
  integration, and anti-abuse acceptance children rather than a single module
  implementation.

The implementation must provide a coverage harness for all 27 plans. The
minimum acceptable harness is a deterministic fixture or golden assertion for
each plan containing the plan id, whether the plan is simple or composite, the
expected child count or minimum child count, the expected archetype set, the
expected review-profile set, and the expected proof-contract status. For the
highest-risk plans such as
`005-craps-game.md`, `013-house-agent.md`, `014-tui-shell.md`,
`015-monero-infrastructure.md`, and `016-casino-core-trait.md`, the harness
must assert exact child ids, dependencies, owned surfaces, leading proof
commands, and leading review-profile assignments.

The redesign must also add integration rules so the child work items fit the
rest of the codebase instead of optimizing in isolation. Each child record must
be able to declare:

- the owned surfaces it may edit
- the upstream child or plan dependencies that must settle first
- the proof commands that prove the child locally
- the integration targets that must still consume the child’s result
- whether direct integration onto trunk is allowed after promotion

These rules matter because many `rXMRbro` plans share surfaces such as
`crates/casino-core`, `crates/provably-fair`, `bin/house`, and `crates/tui`.
The framework must prevent two children from claiming overlapping owned
surfaces in the same plan unless the mapping contract explicitly chooses the
`implement_cross_surface` archetype and documents why the broader ownership is
honest.

That rule must apply across the whole portfolio, not just inside one plan. For
example, `plans/003-poker-game.md` and `plans/004-blackjack-game.md` may both
want to touch `crates/casino-core/src/lib.rs` or the same TUI menu files. The
portfolio scheduler must therefore use a global surface-lock table, not a
per-plan-only overlap check.

The redesign must also define a safe migration path from the current
lane-centric truth model to the new plan-centric truth model. The cutover must
happen in three phases:

1. shadow generation: synthesize plan roots, child records, and dashboard data
   without making them authoritative for dispatch
2. parity review: compare plan-first status, matrix output, and Paperclip sync
   against the current lane-centric state until the diff is understood and
   acceptable
3. cutover: make plan-child scheduling and plan-root dashboard truth
   authoritative, while retaining the ability to render legacy lane detail as a
   fallback or debugging aid

The implementation must include explicit rollback instructions for the cutover.
If plan-first dispatch misbehaves, the operator must be able to re-enable the
legacy lane-centric scheduler and keep the dashboard in read-only shadow mode
while the mapping or scheduler policy is repaired.

The final technical change is to update the model-routing task so the plan
matches current code reality. `render.rs` already defaults write and challenge
stages to MiniMax and final review to `opus-4.6`. The implementation work here
should therefore codify and test the existing intent rather than rewriting the
system from a false premise. The result should be explicit tests and
documentation that preserve MiniMax-first execution, optional stronger
intermediate review, and `opus-4.6` only at the final promotion boundary.

The plan-to-workflow decomposition inside `synth create` now uses Opus 4.6 by
default. The implemented flow is:

- `run_opus_decomposition()` sends each composite plan's markdown to
  `claude -p --dangerously-skip-permissions --model claude-opus-4-6`
- Opus returns a YAML mapping contract with milestone-level children
- The mapping is written to `malinka/plan-mappings/{plan-stem}.yaml`
- The blueprint is re-authored consuming the enriched mapping contracts
- `--no-decompose` falls back to deterministic heuristics for offline/CI

The model routing for generated workflows is:
- MiniMax M2.7 Highspeed for write and challenge stages (the `[llm]` section
  in run configs and `#challenge` in model stylesheets)
- Claude Opus 4.6 for final review and promotion signoff (the `#review`
  node in Implementation workflow model stylesheets)
- No API key management needed — the cli backend inherits Claude Code auth

The remaining decomposition improvement is a second-pass eng review: after
Opus produces the draft mapping, send it back to Opus for adversarial review
before rendering. This is not yet implemented.

## Concrete Steps

All commands run from `/home/r/coding/fabro` unless stated otherwise.

### Implemented — verification commands

Plan registry (5 tests passing):

    cargo test -p raspberry-supervisor plan_registry -- --nocapture

Maintenance mode (3 tests passing):

    cargo test -p raspberry-supervisor maintenance -- --nocapture

Synthesis with child workflow generation (59 tests passing):

    cargo test -p fabro-synthesis

CLI synth tests (mapping snapshot format updated):

    cargo test -p fabro-cli --test synth synth_create -- --nocapture

### Live proving-ground commands

Generate the full plan-first package with Opus decomposition (default):

    cargo run -p fabro-cli -- synth create \
      --target-repo /home/r/coding/rXMRbro \
      --program rxmragent

Generate with deterministic heuristics only (offline/CI fallback):

    cargo run -p fabro-cli -- synth create \
      --target-repo /home/r/coding/rXMRbro \
      --program rxmragent \
      --no-decompose

Inspect the plan matrix:

    cargo run -p raspberry-cli -- \
      plan-matrix \
      --manifest /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml

### Not yet implemented — future verification commands

Portfolio scheduler (surface locks, dependency-aware dispatch):

    cargo test -p raspberry-supervisor portfolio_scheduler -- --nocapture

Shadow-mode cutover parity:

    cargo test -p raspberry-supervisor plan_cutover -- --nocapture

Review-profile-specific workflow bundles:

    cargo test -p fabro-synthesis review_profiles -- --nocapture
    cargo test -p fabro-synthesis convergence_gates -- --nocapture

Paperclip plan-root-keyed sync:

    cargo test -p fabro-cli paperclip_plan_sync -- --nocapture
    cargo test -p fabro-cli paperclip_dashboard -- --nocapture

Full suite verification:

    cargo test -p fabro-synthesis
    cargo test -p raspberry-supervisor
    cargo test -p raspberry-cli
    cargo test -p fabro-cli --test synth
    cargo check -p fabro-synthesis -p raspberry-supervisor -p raspberry-cli -p fabro-cli

## Validation and Acceptance

This redesign is complete when a novice can take a repo with numbered plans and
observe that plans, not guessed lanes, are the primary supervised objects.

The first acceptance behavior is local and structural. Running the plan matrix
must show one row per numbered plan with deterministic fields for representation,
status, risk, and proof readiness. If a plan still lacks a checked-in mapping
contract, synth must still emit a mapped execution shape instead of falling
back to an unmapped state.

The second acceptance behavior is synthesis accuracy. Running preview evolve on
`/home/r/coding/rXMRbro` must produce a package where `plans/005-craps-game.md`
is represented as a plan root with deterministic child work items matching the
major milestones in the plan. The child proof commands must reflect the plan’s
declared commands instead of collapsing to generic repo-level guesses.

The third acceptance behavior is workflow-shape accuracy across the whole plan
set. The redesigned system must expose a bounded workflow archetype catalog and
must assign an archetype to every executable child in the 27-plan `rXMRbro`
corpus. The system must not default the whole corpus to one generic
implementation lane type.

The fourth acceptance behavior is review-bundle accuracy. Every executable child
must have an explicit review profile, and the workflow run for that child must
follow the correct cheap-model, adversarial, deterministic, and escalation
stack for that profile.

The fifth acceptance behavior is proof honesty. No executable child generated
from a mapped plan may emit `script="true"` as its verify contract. If the
system cannot derive a real proof contract, that child must surface risk and
weak-proof metadata without losing its mapped execution shape.

The sixth acceptance behavior is convergence correctness. High-risk children
must not stop merely because one review step passed. They must continue until
their bundle-specific convergence gate is satisfied.

The seventh acceptance behavior is scheduler correctness. The portfolio scheduler
must choose only eligible children and must never run two children at once if they hold
conflicting non-shareable surface locks.

The eighth acceptance behavior is integration discipline. Children that edit
shared surfaces must declare those surfaces, must not overlap dishonestly with
siblings, and must encode the correct upstream dependencies so that work
integrates with the rest of the codebase instead of drifting into isolated
subgraphs.

The ninth acceptance behavior is safe cutover. During shadow mode, the system
must be able to render both legacy lane-centric truth and new plan-centric
truth and explain the difference. After cutover, the new truth must become
authoritative without losing a rollback path.

The tenth acceptance behavior is Paperclip fidelity. After refresh, the
Paperclip view must show one top-level object per plan and must make it obvious
which child work items exist beneath that plan. A human should not have to know
the synthesized internal lane names to understand plan progress.

The eleventh acceptance behavior is live dashboard truth. The Paperclip web
dashboard must show the same plan status, risk, mapping provenance, and next
operator move that Raspberry reports locally. When Raspberry state changes and a
refresh occurs, the affected plan card or issue must update within one sync
cycle, and the update trail must appear as deterministic transition comments or
snapshot changes rather than as ad hoc description drift.

The twelfth acceptance behavior is dashboard usability for composite plans. For
`plans/005-craps-game.md`, a human using only the Paperclip web UI must be able
to find the plan issue, see the child breakdown for `casino-core`,
`provably-fair`, `house`, `tui`, `e2e`, and `acceptance`, and click through to
the relevant proofs or artifacts without needing to infer those children from
lane names.

The thirteenth acceptance behavior is maintenance safety. If
`.raspberry/maintenance.json` is enabled, `raspberry execute`, `raspberry
autodev`, and Paperclip wake paths must refuse new work while read-only plan
inspection still succeeds.

## Idempotence and Recovery

All redesign steps in this plan are additive and should be safe to repeat. The
new plan registry and mapping contract must be deterministic, which means the
same repository state should always produce the same plan records and child work
shape.

The preview evolve command is intentionally non-destructive because it writes to
`/tmp/rxmragent-plan-first-preview` instead of mutating the target repository’s
checked-in package during validation. Use preview mode first whenever the
registry or renderer changes.

The maintenance lock is the recovery mechanism for this redesign. If synthesis
or supervisor changes put the repo in a confused state, enable
`.raspberry/maintenance.json`, verify status surfaces in read-only mode, repair
the mapping or generated package, and only then re-enable dispatch.

If a plan cannot be mapped honestly, do not workaround the ambiguity by adding
more heuristics until the system happens to pass. Instead, add or update the
checked-in mapping contract for that plan and rerun the registry and preview
tests.

If a child boundary looks wrong during coverage review, do not fix it by
editing only the rendered workflow. Fix the mapping contract or archetype
selection rule first, then regenerate. The framework should remain the source of
truth for child structure.

## Artifacts and Notes

The most important artifact for this redesign is the golden fixture that proves
how a composite plan maps into executable work. That fixture should live in the
Fabro synthesis test suite and should encode the expected mapping for
`/home/r/coding/rXMRbro/plans/005-craps-game.md`.

The second important artifact is the plan matrix output itself. Keep a concise
example in the tests or docs showing the expected fields:

    plan_id | path | represented | children | real_proof | status | risk | next_move

The third important artifact is a concise preview assertion for `rXMRbro`
showing that `plans/005-craps-game.md` is represented as a plan root and not
merely listed in `evidence_paths`.

The fourth important artifact is the all-27 coverage report. Keep a durable
fixture or generated report showing, for each numbered plan, its composite
status, child count, archetype set, proof readiness, and whether human mapping
review is still required.

The fifth important artifact is the workflow archetype catalog itself. Keep it
checked in as code-level documentation or tests so future contributors can tell
which child shapes are first-class and which are intentionally out of scope.

The sixth important artifact is the Paperclip dashboard fixture. Keep a test or
snapshot artifact showing the expected company markdown and synchronized issue
shape for a plan-first repo, including at least one composite plan and one
contract-backed composite plan.

The seventh important artifact is a scheduler simulation fixture. Keep a
deterministic portfolio input and expected dispatch order that proves the
scheduler respects dependencies, blast radius, and surface locks across more
than one plan.

The eighth important artifact is a cutover parity report. Keep a deterministic
fixture that compares legacy lane-centric truth to plan-centric truth during
shadow mode so the migration remains reviewable and reversible.

The ninth important artifact is a workflow-bundle fixture. Keep a deterministic
artifact showing, for representative children, the selected archetype, review
profile, cheap-review stack, deterministic gates, GPT escalation conditions, and
convergence rule.

## Interfaces and Dependencies

In the shared registry surface, starting in
`lib/crates/raspberry-supervisor/src/plan_registry.rs`, define a stable public
API. The exact field names may vary, but the resulting module must provide
types equivalent to:

    pub struct PlanRegistry {
        pub plans: Vec<PlanRecord>,
    }

    pub struct PlanRecord {
        pub plan_id: String,
        pub path: PathBuf,
        pub title: String,
        pub category: PlanCategory,
        pub composite: bool,
        pub bootstrap_required: bool,
        pub implementation_required: bool,
        pub mapping_source: PlanMappingSource,
        pub dependencies: Vec<PlanDependency>,
        pub children: Vec<PlanChildRecord>,
        pub review_expectations: Vec<String>,
    }

    pub struct PlanChildRecord {
        pub child_id: String,
        pub title: String,
        pub lane_kind: LaneKind,
        pub workflow_family: WorkflowTemplate,
        pub archetype: WorkflowArchetype,
        pub review_profile: ReviewProfile,
        pub owned_surfaces: Vec<PathBuf>,
        pub how: String,
        pub state_expectations: Vec<String>,
        pub proof_commands: Vec<String>,
        pub verification_plan: Vec<String>,
        pub rollback_conditions: Vec<String>,
        pub dependencies: Vec<PlanDependency>,
        pub integration_targets: Vec<String>,
        pub direct_integration_allowed: bool,
    }

    pub enum PlanCategory {
        Platform,
        Service,
        Interface,
        Proof,
        Meta,
        Composite,
    }

    pub enum WorkflowArchetype {
        BootstrapContract,
        ImplementModule,
        ImplementCrossSurface,
        ServiceSurface,
        TuiSurface,
        VerificationOnly,
        IntegrationOnly,
        AcceptanceAndBalance,
        Migration,
        OrchestrationProgram,
        ReviewOrReportOnly,
    }

    pub enum ReviewProfile {
        Standard,
        SharedFoundation,
        SecuritySensitive,
        EconomicCorrectness,
        UserVisible,
        ProductionService,
        MigrationRisky,
    }

The registry module must also expose the checked-in plan mapping contract reader.
If frontmatter is chosen, document the supported fields in the module and in
tests. If sidecar files are chosen, document the repo-relative lookup rules and
the precedence rules when both sidecar and prose are present.

The mapping contract reader must support explicit child-level archetype
selection, child-level review profile selection, and child-level owned surfaces.
The registry must expose pure
selection function equivalent to:

    pub fn select_child_archetype(
        contract: Option<&PlanChildContract>,
        inferred: &InferredChildShape,
    ) -> Result<WorkflowArchetype, MappingReviewNeeded>;

    pub fn select_child_review_profile(
        contract: Option<&PlanChildContract>,
        inferred: &InferredChildShape,
    ) -> Result<ReviewProfile, MappingReviewNeeded>;

This function must prefer the explicit contract and return a review-needed
result when inference is too ambiguous to choose an honest archetype.

In `lib/crates/raspberry-supervisor/src/plan_status.rs`, define the master
status model. The exact naming may vary, but the resulting interface must be
able to answer:

    pub struct PlanStatusRow {
        pub plan_id: String,
        pub path: PathBuf,
        pub represented_in_blueprint: bool,
        pub has_bootstrap_child: bool,
        pub has_implementation_child: bool,
        pub has_real_verify_gate: bool,
        pub status: PlanStatus,
        pub risk: PlanRisk,
        pub next_operator_move: String,
    }

    pub enum PlanStatus {
        Unmodeled,
        MappingReviewNeeded,
        Planned,
        BootstrapReady,
        Bootstrapping,
        BootstrapFailed,
        Reviewed,
        ImplementationReady,
        Implementing,
        ImplementationFailed,
        Verifying,
        VerifyFailed,
        ReviewPending,
        MergeReady,
        Integrated,
        Blocked,
    }

The status model must derive from real child workflow state and real artifacts.
It must not infer “merge ready” or “real verify gate” from plan intention alone.

In `lib/crates/raspberry-supervisor/src/portfolio_scheduler.rs`, or an
equivalent module, define the scheduler model for plan-first dispatch. The exact
names may vary, but the implementation should expose something equivalent to:

    pub struct PortfolioExecutionGraph {
        pub plans: Vec<PlanExecutionNode>,
        pub in_flight_children: Vec<String>,
        pub surface_locks: Vec<SurfaceLock>,
    }

    pub struct PlanExecutionNode {
        pub plan_id: String,
        pub child_id: String,
        pub archetype: WorkflowArchetype,
        pub review_profile: ReviewProfile,
        pub dependencies: Vec<String>,
        pub owned_surfaces: Vec<SurfaceClaim>,
        pub status: PlanStatus,
        pub risk: PlanRisk,
        pub next_operator_move: String,
    }

    pub struct SurfaceClaim {
        pub path: PathBuf,
        pub mode: SurfaceLockMode,
    }

    pub enum SurfaceLockMode {
        Exclusive,
        SharedRead,
    }

    pub struct SurfaceLock {
        pub child_id: String,
        pub claims: Vec<SurfaceClaim>,
    }

    pub fn select_next_plan_children(
        graph: &PortfolioExecutionGraph,
        max_parallel: usize,
    ) -> Vec<String>;

This module must be testable without running Fabro itself. It must deterministically
return the next eligible child ids or an empty set when all remaining work is
blocked.

In `lib/crates/raspberry-supervisor/src/evaluate.rs` or an equivalent cutover
module, define shadow-mode parity support. The implementation should expose a
pure comparison path equivalent to:

    pub struct PlanCutoverParity {
        pub legacy_summary: serde_json::Value,
        pub plan_summary: serde_json::Value,
        pub differences: Vec<String>,
        pub cutover_safe: bool,
    }

    pub fn compare_legacy_and_plan_truth(...) -> PlanCutoverParity;

The cutover logic must make it possible to keep legacy lane truth live while
plan-first truth is still in review.

In `lib/crates/fabro-cli/src/commands/paperclip.rs`, make the synchronized
top-level object correspond to the plan root. The synchronized fields must at
least include the plan id, path, status, risk, real-proof flag, and next
operator move. Child work items for bootstrap, implementation, verify, review,
and integration must appear as drill-down state beneath that plan root.

The Paperclip command layer should also introduce a dedicated plan-first
dashboard model, separate from or layered above the current
`FrontierSyncModel`. The exact type names may vary, but the resulting code
should expose something equivalent to:

    pub struct PlanDashboardModel {
        pub program: String,
        pub generated_at: String,
        pub refreshed_at: Option<String>,
        pub last_raspberry_state_at: Option<String>,
        pub stale: bool,
        pub summary: PlanDashboardSummary,
        pub plans: Vec<PlanDashboardEntry>,
    }

    pub struct PlanDashboardSummary {
        pub mapping_review_needed: usize,
        pub ready: usize,
        pub running: usize,
        pub blocked: usize,
        pub failed: usize,
        pub merge_ready: usize,
        pub integrated: usize,
    }

    pub struct PlanDashboardEntry {
        pub plan_id: String,
        pub path: PathBuf,
        pub title: String,
        pub status: PlanStatus,
        pub risk: PlanRisk,
        pub mapping_source: PlanMappingSource,
        pub next_operator_move: String,
        pub child_entries: Vec<PlanChildDashboardEntry>,
        pub work_products: Vec<DesiredWorkProduct>,
    }

    pub struct PlanChildDashboardEntry {
        pub child_id: String,
        pub title: String,
        pub archetype: WorkflowArchetype,
        pub review_profile: ReviewProfile,
        pub status: String,
        pub current_stage: Option<String>,
        pub proof_summary: Vec<String>,
        pub owned_surfaces: Vec<PathBuf>,
    }

The company markdown builder must consume this dashboard model and render plan
status as the primary dashboard surface. It may still include frontier or lane
detail sections, but those should be explicitly secondary.

The synchronized issue layer must switch sync keys and hierarchy to plan-root
identity. A lane key may still be stored in metadata for backward compatibility,
but the operator-facing identity must be the plan id and child id. Transition
comments, attachments, and work products must follow the same identity model.

The generated bundle and agent markdown must also consume the plan dashboard
model or the underlying plan matrix so that the web UI, synchronized issues,
and generated agents all speak the same plan-first vocabulary.

In `lib/crates/fabro-synthesis/src/render.rs`, preserve the current MiniMax
write and challenge defaults and `claude-opus-4-6` final review default for generated
execution workflows, but add tests
that make this policy explicit. Also enforce proof-contract precedence in this
order:

1. explicit plan child proof commands from the checked-in mapping contract
2. deterministic category defaults derived from the mapped child type
3. repo-level fallback guesses such as `cargo test`, `npm test`, or `pytest`

If all three levels are empty for an executable child, the renderer must not
pretend the child has a proof contract.

The renderer must also map `WorkflowArchetype × ReviewProfile` to a concrete
workflow bundle. The bundle must specify:

- implementation model
- one or more cheap-model review passes
- whether Kimi 2.5 is required as an independent reviewer
- whether a nemesis or adversarial cycle is required
- deterministic gates that must pass before re-entry or promotion
- Opus 4.6 escalation conditions
- convergence criteria

The implementation should expose a testable bundle-selection function
equivalent to:

    pub struct WorkflowBundle {
        pub archetype: WorkflowArchetype,
        pub review_profile: ReviewProfile,
        pub implementation_model: String,
        pub reviewer_models: Vec<String>,
        pub requires_nemesis: bool,
        pub deterministic_gates: Vec<String>,
        pub gpt_escalation_conditions: Vec<String>,
        pub convergence_rules: Vec<String>,
    }

    pub fn workflow_bundle_for(
        archetype: WorkflowArchetype,
        review_profile: ReviewProfile,
    ) -> WorkflowBundle;

The renderer and blueprint layer must also expose a deterministic mapping from
`WorkflowArchetype` to concrete workflow shape. The mapping must be testable
and should include, at minimum:

- required artifacts for the child
- expected review gate type
- whether the child owns integration work
- whether the child may own multiple surfaces
- whether service-health proof is required
- whether acceptance evidence is required in addition to build or test proof

The implementation must add a coverage fixture for all 27 `rXMRbro` plans. The
fixture should be easy for a novice to inspect and update. A sidecar fixture
shape such as `lib/crates/fabro-synthesis/tests/fixtures/rxmrbro-plan-coverage`
is acceptable if it records, for each numbered plan, the approved child ids,
archetypes, owned surfaces, dependency edges, and proof expectations.

## Revision Note

This revision replaced the earlier outline-style draft with a `PLANS.md`
conformant ExecPlan and incorporated the missing deterministic mapping
recommendations. The key change is that plan-first supervision now requires a
checked-in plan mapping contract, composite-plan child decomposition, explicit
proof-contract precedence, ambiguity handling, and a worked golden fixture based
on `rXMRbro` `plans/005-craps-game.md`, rather than relying mainly on synthesis
judgment.

This follow-up revision extends the document from a plan-mapping redesign into
an end-to-end execution framework. The key new requirement is an explicit
workflow archetype catalog, deterministic child-to-archetype selection,
integration-discipline rules for shared surfaces, and an all-27-plan coverage
harness so confidence comes from full-corpus review rather than from one example.

This revision also incorporates the strongest deterministic contract ideas from
`coding/rsocietyv2/ralph/SPEC.md`: every executable child now needs an
acceptance-style contract with explicit `Where`, `How`, `State`, `Pass/fail`,
and `Rollback` fields, plus scoped backpressure commands and rejection
behavior. It also upgrades the workflow model from archetype-only to
`archetype × review profile`, which better matches the assumption that Minimax
and Kimi cycles are cheap and Opus 4.6 should be spent only where intelligence
and adjudication are truly required.

The execution kickoff revision then reconciles the plan against current repo
reality. It records that maintenance mode already exists, and it corrects the
first implementation slice so the shared plan registry starts in
`raspberry-supervisor` instead of only in `fabro-synthesis`, avoiding a crate
cycle while still giving synthesis and supervisor one deterministic source of
truth.
