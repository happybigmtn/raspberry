# Parent Holistic Review Shipping Gauntlet

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [PLANS.md](/home/r/coding/fabro/PLANS.md).

## Purpose / Big Picture

After this change, Fabro and Raspberry will no longer treat parent-level review as a single bug-review pass followed by an advisory Codex note. Instead, once all child lanes for a plan are integrated, the parent plan will go through a systematic end-to-end shipping gauntlet: structural review, Codex adjudication, root-cause investigation when needed, UI review when relevant, end-to-end QA, security review for trust-sensitive plans, benchmark checks for performance-sensitive plans, explicit ship readiness, optional deploy verification, documentation sync, and a retrospective tail.

The user-visible effect is that a plan can only be considered fully shipped after the whole integrated parent implementation has survived the same kinds of checks that `gstack` applies manually in a high-discipline release flow. A reviewer can see it working by inspecting the synthesized parent lanes in `malinka/programs/*.yaml`, the generated workflows and prompts under `malinka/workflows/` and `malinka/prompts/`, and by watching autodev schedule the new parent-only stages after all child units reach `integrated`.

## Progress

- [x] (2026-03-25 22:35Z) Reviewed the current parent `plan-review`, `codex-review`, and `codex-unblock` generation in `lib/crates/fabro-synthesis/src/render.rs` and the current replay/recovery logic in `lib/crates/raspberry-supervisor/src/autodev.rs`.
- [x] (2026-03-25 22:42Z) Reviewed the relevant `gstack` flows from `/review`, `/investigate`, `/design-review`, `/qa`, `/cso`, `/ship`, `/land-and-deploy`, `/benchmark`, `/document-release`, `/retro`, and `/codex`.
- [x] (2026-03-25 22:48Z) Collected parallel review input on current workflow weaknesses, relevant `gstack` stages to adopt, and a candidate parent-level staged pipeline.
- [x] (2026-03-25 23:18Z) Spawned a second focused agent pass to review the individual `gstack` skill files by group and extracted non-interactive adaptation guidance for each one.
- [x] (2026-03-26 03:42Z) Replaced the old synthesized `plan-review -> codex-review` pair with an explicit parent gauntlet in `lib/crates/fabro-synthesis/src/render.rs`: `holistic-preflight`, `holistic-review-minimax`, `holistic-review-deep`, `holistic-review-adjudication`, conditional `investigate` / `design-review` / `qa` / `cso` / `benchmark` / `land-and-deploy`, then `ship-readiness`, `document-release`, and `retro`.
- [x] (2026-03-26 03:42Z) Implemented explicit parent provider assignment and failover: Minimax first pass, Opus-preferred deep pass with Codex fallback, Codex-preferred adjudication with Opus fallback.
- [x] (2026-03-26 03:43Z) Added focused synthesis tests for the new parent gauntlet shape, conditional lane creation, hard-gated `document-release`, `retro` tail behavior, and provider/fallback selection.
- [x] (2026-03-26 03:46Z) Regenerated a throwaway local clone of `rXMRbro` from the updated synthesis and verified the generated package contains the new parent lanes and milestone chain.
- [ ] Decide whether the current recurring-report template is enough for the first rollout or whether a first-class parent workflow template should replace it in a follow-on patch.
- [ ] Run a live autodev validation and confirm that parent review no longer stops at the current bug-review-only model.

## Surprises & Discoveries

- Observation: the current parent `plan-review` lane already assumes child code is on trunk before it runs, which means the strongest parent review arrives too late to block bad child integrations.
  Evidence: `render.rs` currently synthesizes `plan-review` with goal text stating “All implementation lanes for this plan have completed and their code is on trunk.”

- Observation: the current `codex-review` lane is intentionally light-weight and report-first, with only `spec.md` and `review.md` artifacts, so it is not acting as a full shipping gate.
  Evidence: `render.rs` currently synthesizes `*-codex-review` units with only two artifacts and a recurring-report workflow.

- Observation: the best reusable `gstack` value is not the exact shell wrapper around each skill, but the staged discipline: structural review, root-cause investigation, QA, security, release readiness, deploy verification, docs sync, and retro.
  Evidence: the `gstack` skill docs consistently describe these as separate workflows with separate acceptance and outputs rather than one merged “review” concept.

- Observation: a direct lift of `gstack` is not appropriate because many upstream skills assume an interactive operator, AskUserQuestion loops, telemetry, contributor mode, and browser-side tools. Raspberry needs the operational substance of those skills, but expressed as non-interactive lanes with deterministic artifacts and conditional triggers.
  Evidence: the local skill files under `/home/r/.claude/skills/gstack/` repeatedly include AskUserQuestion scaffolding, telemetry hooks, contributor-mode logging, and “manual trigger only” semantics that do not map directly into autodev.

- Observation: the existing supervisor completion model already blocks program settlement on any synthesized unit that remains incomplete, so the first parent-gauntlet rollout did not require extra autodev milestone code to make the new hard-gated parent stages matter.
  Evidence: the generated parent stages are ordinary units with explicit milestone dependencies, and the regenerated `rxmragent.yaml` showed the ship/docs/retro chain as normal unit dependencies rather than supervisor-only metadata.

- Observation: a throwaway regenerated `rXMRbro` clone now contains concrete parent gauntlet units such as `roulette-holistic-preflight`, `roulette-holistic-review-minimax`, `roulette-holistic-review-deep`, `roulette-holistic-review-adjudication`, `roulette-design-review`, `roulette-qa`, `roulette-cso`, `roulette-ship-readiness`, `roulette-document-release`, and `roulette-retro`.
  Evidence: regeneration on 2026-03-26 into `/home/r/.cache/rust-tmp/tmp.9ia83vP5dy/rXMRbro` produced those exact unit IDs and corresponding workflow/run-config files.

## Decision Log

- Decision: model the future parent workflow as a sequence of explicit parent-only lanes rather than one giant “super review” lane.
  Rationale: explicit lanes make provider assignment, conditional triggers, artifacts, and operator visibility much clearer. They also make it easier to skip irrelevant stages like design review or benchmark without weakening the rest of the flow.
  Date/Author: 2026-03-25 / Codex

- Decision: use a three-tier parent review stack: Minimax first, then a deep-review tier that prefers Opus 4.6 with Codex fallback, then an adjudication tier that prefers Codex with Opus fallback.
  Rationale: this preserves Minimax as the breadth-first normalizer, gives the middle tier to the strongest synthesis-oriented reviewer available, and keeps a distinct final engineering adjudication pass. Mutual Opus/Codex failover keeps the gauntlet alive during provider or token pressure without collapsing the role separation entirely.
  Date/Author: 2026-03-25 / Codex

- Decision: keep Kimi valuable at the child in-band review level, not as the primary parent holistic reviewer.
  Rationale: the recent discussion and live evidence suggest the current weakness is from review onward at the parent/integrated level, not from the absence of another child-local reviewer.
  Date/Author: 2026-03-25 / Codex

- Decision: make `/document-release` a hard parent gate and `/retro` a reporting-only tail.
  Rationale: documentation drift is a concrete shipping failure mode, while retrospective value is real but should not block release.
  Date/Author: 2026-03-25 / Codex

- Decision: the implementation must encode the substantive behavior of the selected `gstack` skills into synthesized lanes, prompts, artifacts, gates, and dependencies, rather than treating those skills as reference material or naming inspiration only.
  Rationale: the user explicitly wants the parent workflow to inherit the real end-to-end discipline from `gstack`. A plan that merely borrows skill names without operationalizing their checks would preserve the exact weakness this redesign is meant to fix.
  Date/Author: 2026-03-25 / Codex

- Decision: include upstream `gstack` source links and a detailed non-verbatim appendix in this plan instead of copying large verbatim skill files.
  Rationale: the implementation needs the latest upstream substance, but a self-contained plan can satisfy that by embedding the operative checks, artifacts, and gate semantics in our own words while linking the canonical upstream files for reference.
  Date/Author: 2026-03-25 / Codex

## Outcomes & Retrospective

The first implementation slice is now in place. Fabro synthesis no longer emits only the old parent `plan-review` plus advisory `codex-review` pair. It now emits a concrete parent gauntlet with explicit preflight, Minimax first pass, Opus/Codex deep tier, final adjudication, conditional parent review stages, ship readiness, docs release, and retro. The remaining work is mostly rollout hardening: deciding whether to keep using the recurring-report template for the first live rollout or move to a dedicated parent workflow template, then validating the new chain under live autodev.

## Context and Orientation

The current parent-level synthesis lives in `lib/crates/fabro-synthesis/src/render.rs`. That file currently creates three relevant synthesized lane families:

- `*-plan-review`: a parent-level implementation-template lane that runs after child units complete. It currently performs a 5-step adversarial bug review, writes full implementation-style artifacts, and can directly modify product code.
- `*-codex-review`: a lighter recurring-report lane that runs after `plan-review`. It is post-completion, report-first, and currently non-blocking from the perspective of child integration.
- `*-codex-unblock`: a dedicated recovery lane for a single stuck implementation lane. It is dispatched by the supervisor only for specific failure kinds.

The current supervisor orchestration for failed lanes lives in `lib/crates/raspberry-supervisor/src/autodev.rs`. That file decides when to replay a failed lane, when to trigger `codex-unblock`, and when to regenerate or back off. It does not currently know about any richer parent-level gauntlet beyond the lanes that synthesis creates.

The `gstack` workflows being borrowed as concepts live under `/home/r/.claude/skills/gstack/`. The important meaning of those names in this plan is:

- `review`: a pre-landing structural review of the diff against base branch, especially trust boundaries and risky conditionals.
- `investigate`: a root-cause-first debugging flow that insists on investigation before fixes.
- `design-review`: a visual and interaction quality pass for UI-heavy work.
- `qa`: an end-to-end user-flow testing pass with severity-tagged failures.
- `cso`: a security-and-infrastructure review, including secrets, dependencies, CI/CD, and trust boundaries.
- `ship`: a release-preparation checklist and final readiness pass.
- `land-and-deploy`: merge, deploy, and verify health in a deploy-aware system.
- `benchmark`: performance regression detection against a baseline.
- `document-release`: documentation synchronization after the code is considered shipped.
- `retro`: a reporting-only retrospective over the delivered work.
- `codex`: an independent second-opinion review or challenge pass.

In this repository, “parent-level” means a synthesized lane that looks across the integrated output of all child units belonging to one plan prefix. It does not mean a child lane’s own review stages. “Hard gate” means autodev cannot mark the parent plan fully shipped while that lane is failing or incomplete. “Conditional gate” means the lane exists only when the plan’s touched surfaces or risk profile justify it.

This plan is intentionally stronger than “borrow the idea of `gstack`.” The implementation is not complete unless the selected `gstack` skills are translated into real synthesized behavior in Fabro. That means one or more of the following must exist for each adopted stage:

- a dedicated synthesized parent lane or template,
- prompt contracts that force the same category of checks,
- machine-checkable audit or quality rules,
- explicit artifacts that capture the stage’s result,
- milestone dependencies that make the stage blocking when it is supposed to be a gate,
- conditional triggers that create or skip the stage based on the parent plan’s risk profile.

Merely referencing a skill name in prose, or telling an agent to “be inspired by `/qa`,” does not satisfy this plan.

The implementation must also be explicitly non-interactive. Any upstream `gstack` behavior that currently relies on AskUserQuestion, a human choosing an option, a browser tool being opened manually, or telemetry/config prompts must be translated into deterministic Raspberry behavior. In practice this means:

- use precomputed conditional triggers instead of interactive branching,
- use hard-coded artifact contracts instead of “ask the user whether to continue,”
- use machine-readable pass/fail fields instead of “recommendation” prose where a hard gate is required,
- omit telemetry, contributor-mode logging, and human-facing preambles entirely,
- treat browser-dependent checks as conditional parent lanes only when the target repo supports those checks in a non-interactive way.

## Plan of Work

The implementation should begin in `lib/crates/fabro-synthesis/src/render.rs`, because that file already owns the creation of `plan-review`, `codex-review`, and `codex-unblock`. The first change is to replace the current simple parent pair of `plan-review` then `codex-review` with a richer family of synthesized parent-only units. Do not overload the existing generic implementation template further than necessary. If the current implementation graph becomes too awkward, introduce a first-class parent review template or a small set of parent-specific proof profiles with distinct graph shapes.

For every adopted `gstack` stage, preserve the stage’s substance, not just its name. The expected programmatic mapping is:

- `/review` becomes a hard-gated parent holistic review lane with structured findings, diff-aware checks, and a normalized issue index.
- `/codex` becomes part of the deep-review tier and final adjudication tier, depending on provider availability and role assignment for the current run.
- `/investigate` becomes a root-cause artifact and conditional hard gate, not a generic fixup prompt.
- `/design-review` becomes a conditional UI/TUI parent lane with visual or interaction findings.
- `/qa` becomes a conditional integrated-flow test lane with severity and repro artifacts.
- `/cso` becomes a conditional security/infrastructure lane with explicit residual-risk output.
- `/benchmark` becomes a conditional performance lane with baseline/comparison output.
- `/ship` becomes an explicit parent release-readiness gate.
- `/land-and-deploy` becomes a conditional deploy verification gate when deployment is actually in scope.
- `/document-release` becomes a hard-gated docs-sync lane.
- `/retro` becomes a reporting-only retrospective tail.

The first new synthesized unit should be a deterministic parent preflight lane, tentatively named `*-contract-verify` or `*-holistic-preflight`, that checks all expected child integration artifacts, child review artifacts, and runnable proof commands exist before any expensive parent LLM work begins. This stage should be command-driven, produce a concise verification artifact, and fail fast when the parent is not yet ready for holistic review.

Then define the new parent sequence. The first parent LLM lane should be `*-holistic-review-minimax`. It should use Minimax on the first pass and synthesize a normalized issue index, a parent remediation plan, and a structured verdict over correctness, trust boundaries, UX, deployability, performance, and documentation. This is where the `/review` concepts belong. It should be diff-aware and integrated-state-aware rather than purely artifact-aware, which means the prompt and supporting checks should tell the reviewer to compare the landed code against the parent plan’s expected changes, not just re-summarize artifacts.

The second parent LLM lane should be `*-holistic-review-deep`. It should prefer Opus 4.6, fall back to Codex when Opus is unavailable or token-constrained, and act as the deep synthesis pass over the Minimax findings. It should collapse duplicates, challenge weak evidence, identify systemic edge cases, and refine the remediation plan rather than merely restating the first pass.

The third parent LLM lane should be `*-holistic-review-adjudication`. It should prefer Codex, fall back to Opus 4.6 when Codex is unavailable or token-constrained, and write separate confirmed and rejected finding sets plus a final merge-or-no-ship verdict. The key point is that parent completion must no longer mean “Kimi/Minimax saw it”; it must mean “Minimax first pass, deep synthesis pass, and final adjudication pass all completed.”

Next, add conditional parent lanes:

- `*-investigate`: synthesize this only when the parent plan is marked trust-sensitive or when either holistic review lane finds unresolved high-severity issues or “root cause unclear” findings. This lane should be root-cause-first, not a generic fixup. It must produce an `investigation.md` artifact with hypothesis, evidence, and remediation path.
- `*-design-review`: synthesize only when the parent touched UI-heavy surfaces such as TUI screens, frontend surfaces, visual assets, or design-system files. This lane should record concrete visual/interaction issues and can remain report-first unless the plan explicitly includes parent-level UI fixes.
- `*-qa`: synthesize when the plan exposes integrated user flows that can be exercised. This lane should write a severity-tagged QA report with repro steps and a ship score.
- `*-cso`: synthesize when the plan touches wallets, balances, payouts, seeds, auth, external process control, deployment infrastructure, or control-plane behavior. This lane should produce a structured security review and residual-risk summary.
- `*-benchmark`: synthesize when performance-sensitive surfaces changed or when a benchmark baseline already exists. This can be hard-gated for known performance-sensitive plans and report-only otherwise.
- `*-ship-readiness`: always synthesize after the review gates. This lane owns release readiness, version/changelog status, and explicit ship/no-ship judgment from a release perspective.
- `*-land-and-deploy`: synthesize only when the repo includes deploy configuration and the plan is meant to deploy. This lane must wait for CI/deploy and record health/canary evidence.
- `*-document-release`: always synthesize as the last hard gate. It should ensure README, architecture notes, and other release docs match what shipped.
- `*-retro`: synthesize as a reporting-only final lane, not a blocker.

Provider assignment must be explicit. The minimum required split is:

- Minimax: `parent-holistic-review-minimax` and optionally first-pass `investigate` or `document-release`
- Anthropic/Opus 4.6 preferred: `parent-holistic-review-deep`, with Codex/OpenAI fallback
- Codex/OpenAI preferred: `parent-holistic-review-adjudication`, with Opus 4.6 fallback
- Kimi: keep as child in-band review authority where already useful, but do not make it the main parent holistic reviewer

The synthesized artifacts also need to change. The current parent `codex-review` lane only requires `spec.md` and `review.md`, which is too light for a shipping gauntlet. Each new parent lane must define only the artifacts it really owns. For example:

- `*-holistic-review-minimax`: `holistic-review.md`, `finding-index.json`, `remediation-plan.md`, `promotion.md`
- `*-holistic-review-deep`: `deep-review.md`, `finding-deltas.json`, `remediation-plan.md`
- `*-holistic-review-adjudication`: `adjudication-verdict.md`, `confirmed-findings.json`, `rejected-findings.json`, `promotion.md`
- `*-investigate`: `investigation.md`
- `*-qa`: `qa-report.md`
- `*-cso`: `security-review.md`
- `*-benchmark`: `benchmark.md`
- `*-ship-readiness`: `ship-checklist.md`
- `*-land-and-deploy`: `deploy-verification.md`
- `*-document-release`: `docs-release.md`
- `*-retro`: `retro.md`

Do not force every parent lane through the full implementation artifact set unless that lane can actually modify code. Otherwise the same artifact drift problems we just fixed in unblock lanes will recur at the parent level.

After synthesis is updated, the supervisor in `lib/crates/raspberry-supervisor/src/autodev.rs` must understand the new parent milestones. It does not need to know `gstack` by name, but it must treat the parent plan as incomplete until all hard-gated parent milestones complete. If a conditional lane is not synthesized, that should count as “not applicable,” not “missing.” If a conditional lane is synthesized and fails, it must block the final parent-shipped milestone.

The current `codex-unblock` routing should also be revisited. Today it only triggers for a narrow set of failure kinds. The new parent flow should allow unresolved parent findings from holistic review, QA, security, or benchmark passes to trigger a parent investigation or parent Codex remediation lane, not just a per-child unblock lane.

Finally, regenerate a real target repo, ideally `rXMRbro`, from the new synthesis and confirm that the parent plan sequence appears only after child units reach `integrated`. The live autodev proof should show that parent-only stages are generated and queued in the order described here, with provider assignments matching the intended split.

## Concrete Steps

Work from `/home/r/coding/fabro`.

1. Read the current parent-lane synthesis and supporting helpers.

       cd /home/r/coding/fabro
       sed -n '540,860p' lib/crates/fabro-synthesis/src/render.rs
       sed -n '1360,1705p' lib/crates/fabro-synthesis/src/render.rs

   Expect to see the current `*-plan-review`, `*-codex-review`, and `*-codex-unblock` generation.

2. Implement the new parent lane synthesis in `lib/crates/fabro-synthesis/src/render.rs`.

   Define explicit parent-only lane families and milestones. Keep all path generation lane-scoped. Add helper functions if needed for:

   - deciding whether a plan is UI-sensitive, security-sensitive, performance-sensitive, or deployable
   - deciding which conditional parent lanes to synthesize
   - assigning providers to each parent stage
   - generating parent-specific prompts and artifact contracts

3. If the generic implementation graph cannot describe the new parent flow cleanly, add a first-class parent workflow template or a dedicated parent proof profile in `lib/crates/fabro-synthesis/src/render.rs`.

   The graph should support command-only gates and report-first stages without pretending every lane is an implementation/fixup cycle.

4. Update any parent-level prompt renderers and artifact/audit contracts.

       cargo test -p fabro-synthesis --lib --no-run

   Fix compile errors before continuing.

5. Update the supervisor if needed so final parent completion depends on the new parent hard gates.

   Read:

       sed -n '330,760p' lib/crates/raspberry-supervisor/src/autodev.rs

   Then adjust milestone/failure handling so the new parent stages can block completion correctly.

6. Add focused tests in `lib/crates/fabro-synthesis/src/render.rs` and `lib/crates/raspberry-supervisor/src/autodev.rs`.

   The minimum test set should prove:

   - parent holistic review lanes synthesize after all child `integrated` milestones
   - Minimax owns the first holistic parent pass
   - Codex owns the immediate second parent pass
   - conditional lanes are present for representative UI/security/performance/deploy cases
   - parent `document-release` is a hard gate
   - parent `retro` is reporting-only

7. Rebuild the release binaries and regenerate a real target repo from source blueprint.

       cargo build --release -p fabro-cli -p raspberry-cli --target-dir target-local
       /home/r/coding/fabro/target-local/release/fabro --no-upgrade-check synth create \
         --target-repo /home/r/coding/rXMRbro \
         --program rxmragent \
         --blueprint /home/r/coding/rXMRbro/malinka/blueprints/rxmragent.yaml \
         --no-decompose --no-review

8. Inspect the generated parent-level lanes on disk.

       rg -n "holistic|codex-review|investigate|design-review|qa|cso|ship|benchmark|document-release|retro" \
         /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml \
         /home/r/coding/rXMRbro/malinka/workflows \
         /home/r/coding/rXMRbro/malinka/run-configs

   Expect to see the new parent lane family with the intended provider split.

9. Restart autodev and inspect the live report.

       /home/r/coding/fabro/target-local/release/raspberry autodev \
         --manifest /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml \
         --fabro-bin /home/r/coding/fabro/target-local/release/fabro \
         --max-parallel 10 \
         --max-cycles 1000000 \
         --poll-interval-ms 10000 \
         --evolve-every-seconds 1800

   Then inspect:

       sed -n '1,220p' /home/r/coding/rXMRbro/.raspberry/rxmragent-autodev.json
       jq -r '.lanes | to_entries[] | [.key, .value.status, (.value.current_stage_label // .value.last_completed_stage_label // "-")] | @tsv' \
         /home/r/coding/rXMRbro/.raspberry/rxmragent-state.json

   Expect the parent lane family to remain dormant until all child integrations complete, then appear in the ready/running set with the proper order.

10. For each adopted `gstack` stage, verify that the generated Fabro lane actually encodes the intended behavior rather than merely referencing the stage name.

        rg -n "holistic-review|investigate|design-review|qa|cso|benchmark|ship|land-and-deploy|document-release|retro" \
          /home/r/coding/rXMRbro/malinka/workflows \
          /home/r/coding/rXMRbro/malinka/prompts \
          /home/r/coding/rXMRbro/malinka/run-configs

   Inspect the generated prompts and workflows and confirm:

   - `review` lanes contain structured review/adjudication contracts,
   - `investigate` lanes require root-cause outputs,
   - `qa` lanes produce repro/severity artifacts,
   - `cso` lanes produce security findings,
   - `ship` lanes require release-readiness outputs,
   - `document-release` lanes require docs-sync outputs,
   - `retro` is explicitly reporting-only.

## Validation and Acceptance

This plan is accepted only when all of the following are true:

- A regenerated target repo contains an explicit parent lane family broader than the current `plan-review` plus `codex-review` pair.
- The first parent holistic review lane is Minimax-backed.
- The second parent review lane prefers Opus 4.6 and falls back to Codex when needed.
- The final parent adjudication lane prefers Codex and falls back to Opus 4.6 when needed.
- Conditional parent lanes exist only when their trigger conditions apply.
- The parent final completion milestone is blocked by the new hard-gated parent stages.
- Each adopted `gstack` stage is implemented programmatically as a lane contract, gate, artifact, or conditional trigger; none of them exist only as named reference material.
- A target repo with UI-heavy and trust-sensitive child work can synthesize at least these stages:
  - parent contract/preflight verification
  - parent holistic Minimax review
  - parent deep review
  - parent adjudication review
  - parent QA
  - parent CSO
  - parent ship readiness
  - parent document release
  - parent retro
- Live autodev can run on the regenerated package without treating the new parent stages as invalid or supervisor-only failures.

Concrete proof should include:

- passing synthesis tests
- passing supervisor tests
- a regenerated `malinka/programs/*.yaml` showing the new milestones and lane dependencies
- a live autodev report showing the package is valid and schedulable

## Idempotence and Recovery

All synthesis work should be idempotent. Re-running `fabro synth create` on the same blueprint should either rewrite only the affected generated files or no-op when nothing changed. If a generated parent lane shape is wrong, update synthesis and regenerate; do not hand-edit generated workflow or run-config files in the target repo as a permanent fix.

If a live autodev restart produces invalid lane behavior, stop the controller, inspect the generated `malinka/programs/*.yaml`, `malinka/workflows/*.fabro`, and `malinka/run-configs/*.toml`, fix synthesis, regenerate, and restart again. Do not patch the target repo’s generated package by hand unless capturing a temporary debugging experiment, and if you do, record it in `Surprises & Discoveries` before discarding it.

## Artifacts and Notes

Expected examples after implementation:

    rg -n "parent-holistic-review|minimax|codex-review|document-release|retro" \
      /home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml

Should show the parent lanes and milestones for at least one plan prefix.

    jq -r '.lanes | to_entries[] | select(.key | test("holistic|codex-review|document-release|retro")) | [.key, .value.status] | @tsv' \
      /home/r/coding/rXMRbro/.raspberry/rxmragent-state.json

Should show that the parent lane family is recognized by the supervisor as ordinary lanes with normal statuses.

## Interfaces and Dependencies

The synthesis changes should remain centered in `lib/crates/fabro-synthesis/src/render.rs`. If a new workflow template is required, define it there and thread it through existing helpers that render graph, run config, and lane artifacts.

The supervisor changes should stay in `lib/crates/raspberry-supervisor/src/autodev.rs` unless a cleaner milestone helper belongs elsewhere. Avoid scattering the parent gauntlet logic across many crates unless a type truly belongs in a shared manifest or model definition.

Provider selection should continue to use the existing `Provider` and `AutomationProfile` machinery already present in synthesis and workflows. Do not invent a second provider policy system for the parent lanes.

The final implementation must preserve the current child review contract:

- children still complete their own execution/review/integration first
- parent holistic stages begin only after that
- Codex remains a deep-tier reviewer and preferred final adjudicator, not the first parent pass

## Appendix A — Upstream gstack Skill Sources

Canonical upstream repository:

- `https://github.com/garrytan/gstack`

Relevant upstream skill paths at the time this plan was written:

- `https://github.com/garrytan/gstack/tree/main/review`
- `https://github.com/garrytan/gstack/tree/main/investigate`
- `https://github.com/garrytan/gstack/tree/main/design-review`
- `https://github.com/garrytan/gstack/tree/main/qa`
- `https://github.com/garrytan/gstack/tree/main/cso`
- `https://github.com/garrytan/gstack/tree/main/ship`
- `https://github.com/garrytan/gstack/tree/main/land-and-deploy`
- `https://github.com/garrytan/gstack/tree/main/benchmark`
- `https://github.com/garrytan/gstack/tree/main/document-release`
- `https://github.com/garrytan/gstack/tree/main/retro`
- `https://github.com/garrytan/gstack/tree/main/codex`

The local mirror used during planning was:

- `/home/r/.claude/skills/gstack/review/SKILL.md`
- `/home/r/.claude/skills/gstack/investigate/SKILL.md`
- `/home/r/.claude/skills/gstack/design-review/SKILL.md`
- `/home/r/.claude/skills/gstack/qa/SKILL.md`
- `/home/r/.claude/skills/gstack/cso/SKILL.md`
- `/home/r/.claude/skills/gstack/ship/SKILL.md`
- `/home/r/.claude/skills/gstack/land-and-deploy/SKILL.md`
- `/home/r/.claude/skills/gstack/benchmark/SKILL.md`
- `/home/r/.claude/skills/gstack/document-release/SKILL.md`
- `/home/r/.claude/skills/gstack/retro/SKILL.md`
- `/home/r/.claude/skills/gstack/codex/SKILL.md`

## Appendix B — Operational Extraction From Upstream Skills

This appendix is intentionally non-verbatim. It captures the latest operative checks that this implementation must preserve from the upstream `gstack` skills.

### `/review`

The substantive behavior to preserve is pre-landing structural review against the diff or integrated codebase, with explicit focus on trust boundaries, risky conditionals, data safety, and structural side effects. The upstream skill also assumes an explicit pass/fail decision rather than “some notes.”

The Raspberry adaptation must therefore do all of the following:

- compare integrated parent code against the base state, not just summarize artifacts,
- emit a structured findings artifact rather than a prose-only paragraph,
- classify findings by severity and category,
- produce a hard-gated verdict,
- preserve the notion that review is about “what changed and what risk that creates,” not “please restate the implementation.”

Required parent artifacts:

- `holistic-review.md`
- `finding-index.json`
- `promotion.md`

Required hard-gate behavior:

- if the structured review cannot prove correctness/trust-boundary/deployability confidence, parent progression stops,
- findings must be tied to real code or integrated behavior, not only to artifact wording.

Non-interactive adaptation note: omit all AskUserQuestion and telemetry behavior from the upstream skill. Keep only the review semantics and the requirement for a concrete verdict.

### `/investigate`

The substantive behavior to preserve is root-cause-first debugging. The upstream skill is built around a four-phase loop: investigate, analyze, hypothesize, implement, with an iron law that no fix should proceed without root cause.

The Raspberry adaptation must therefore:

- synthesize `investigate` only when the parent flow has unresolved or systemic findings,
- force an explicit root-cause artifact before remediation,
- separate observed evidence from hypotheses,
- distinguish reproduction, narrowing, and final cause,
- feed the chosen remediation target back into the next parent lane.

Required parent artifacts:

- `investigation.md`

Required conditional hard-gate behavior:

- if either holistic review lane says “root cause unclear,” “flaky,” “state divergence,” or equivalent, `parent-investigate` must exist,
- downstream parent remediation or ship gates must wait for it.

Non-interactive adaptation note: the upstream interactive debugging questions become synthesized trigger rules and mandatory artifact fields.

### `/codex`

The substantive behavior to preserve is an independent second opinion. Upstream `gstack` treats Codex as a distinct review, challenge, or consult pass rather than as a stylistic variant of the first review.

The Raspberry adaptation must therefore:

- run Codex immediately after the first holistic Minimax pass,
- feed Codex the integrated diff and the first-pass findings,
- require Codex to re-adjudicate those findings instead of merely summarizing them,
- preserve disagreement explicitly when Codex and Minimax disagree,
- produce a final second-pass ship or no-ship verdict.

Required parent artifacts:

- `codex-verdict.md`
- `confirmed-findings.json`
- `rejected-findings.json`
- `promotion.md`

Hard-gate behavior:

- always a hard gate once the parent holistic path starts,
- if Codex finds unresolved critical issues, parent completion stops,
- if Codex disagrees with Minimax, the disagreement must be recorded and adjudicated rather than flattened into one paragraph.

Non-interactive adaptation note: the upstream “review/challenge/consult” modes become explicit lane intent and artifact contracts rather than interactive mode selection.

### `/design-review`

The substantive behavior to preserve is a visual and interaction quality pass that looks for inconsistency, spacing problems, hierarchy problems, low-quality AI-generated UI patterns, and sluggish interaction behavior. Upstream `gstack` expects iterative visual QA with screenshots and a fix-and-verify loop.

The Raspberry adaptation must therefore:

- synthesize `design-review` only when the parent touches UI, TUI, frontend, design-token, or visual interaction surfaces,
- produce a structured list of visual and interaction findings,
- classify them by severity,
- separate “visual polish” from “functional QA,”
- avoid pretending to run a browser lane when the target repo cannot support one non-interactively.

Required parent artifacts:

- `design-review.md`

Conditional hard-gate behavior:

- hard gate only for UI-bearing parent plans,
- absent entirely for plans with no UI surfaces.

Non-interactive adaptation note: upstream before/after screenshots are only required if the target repo has a real non-interactive way to generate them. Otherwise preserve the same review intent through artifact-backed UI findings.

### `/qa`

The substantive behavior to preserve is a real end-to-end usage pass that classifies bugs by severity, records repro steps, and produces a ship-readiness judgment. Upstream `gstack` treats QA as a separate discipline from source review and expects health scores plus repro evidence.

The Raspberry adaptation must therefore:

- synthesize `qa` when the parent exposes runnable integrated flows,
- run scenario-based proofs rather than source-only review,
- produce a severity-tagged report,
- record repro steps and final ship score,
- keep QA as its own gate, not a subsection of review.

Required parent artifacts:

- `qa-report.md`

Conditional hard-gate behavior:

- parent progression stops on unresolved critical or high issues,
- medium issues can be allowed only if the gate contract explicitly says so,
- if no integrated user-facing flow exists, the lane is omitted.

Non-interactive adaptation note: upstream manual QA exploration must become scripted or contract-driven checks plus structured findings.

The important `gstack` discipline to preserve is that QA is not a prose review of code. It is an end-to-end behavior sweep with severity, repro, evidence, and post-fix re-verification. The Fabro implementation should mirror that discipline directly in lane artifacts and gating rules.

### `/cso`

The substantive behavior to preserve is a broad security and infrastructure review, not just a source-level style check. Upstream `gstack` treats this as a real security workflow covering secrets, dependencies, CI/CD, LLM and infrastructure trust boundaries, and confidence thresholds.

The Raspberry adaptation must therefore:

- synthesize `cso` only for trust-sensitive parent plans,
- include supply-chain/dependency posture, secrets handling, CI/CD/deploy posture, and trust-boundary review,
- produce a residual-risk artifact,
- clearly separate confirmed findings from lower-confidence concerns.

Required parent artifacts:

- `security-review.md`

Conditional hard-gate behavior:

- required for wallets, balances, seeds, payouts, auth, deploy infrastructure, control plane, external process control, and similar sensitive work,
- if synthesized, it is a hard gate for parent completion.

Non-interactive adaptation note: omit telemetry, contributor mode, and interactive questioning. Keep the structured security output and explicit residual-risk judgment.

### `/ship`

The substantive behavior to preserve is explicit release preparation. Upstream `gstack` does not treat “ship” as merely “tests are green”; it includes branch hygiene, test confirmation, diff review, versioning, release-note or changelog preparation, and final readiness.

The Raspberry adaptation must therefore:

- synthesize an explicit `ship-readiness` parent lane,
- require a release-readiness artifact rather than assuming the review verdict is enough,
- include branch/base hygiene, test confirmation, release notes or changelog state, and packaging/readiness status.

Required parent artifacts:

- `ship-checklist.md`

Hard-gate behavior:

- this is always a hard gate once a parent plan reaches the end of its review stack.

The important upstream discipline here is that “ship” is not equal to “tests passed.” It is the release-prep moment where branch hygiene, merged-base verification, test pass on merged code, release-note state, changelog state, coverage/readiness, and final verification all become explicit.

### `/land-and-deploy`

The substantive behavior to preserve is merge/deploy verification and health confirmation. Upstream `gstack` treats deployment as a workflow with merge, CI wait, deploy completion, and health/canary verification.

The Raspberry adaptation must therefore:

- synthesize this lane only when the parent repo has deployment configuration and the plan is actually meant to deploy,
- wait for deployment completion or equivalent deploy signal,
- verify health or canary evidence,
- write a deploy verification artifact rather than trusting a single shell exit code.

Required parent artifacts:

- `deploy-verification.md`

Conditional hard-gate behavior:

- only present when deployment is in scope,
- hard gate when present.

Non-interactive adaptation note: deployment support is opt-in per target repo. If a repo lacks deploy configuration, the lane is omitted rather than stubbed.

### `/benchmark`

The substantive behavior to preserve is performance regression detection against a baseline. Upstream `gstack` expects before/after comparison rather than an isolated benchmark run.

The Raspberry adaptation must therefore:

- synthesize `benchmark` only for performance-sensitive parent plans or where a baseline already exists,
- record the baseline used,
- record the delta and whether it is acceptable,
- optionally hard-gate only when a meaningful perf contract exists.

Required parent artifacts:

- `benchmark.md`

Conditional gate behavior:

- hard gate for known performance-sensitive plans,
- report-only or omitted for the rest.

The important upstream discipline here is “baseline plus comparison,” not “run one performance command.” The Fabro parent lane must preserve that comparison model.

### `/document-release`

The substantive behavior to preserve is post-ship documentation synchronization. Upstream `gstack` expects release-facing docs such as README, architecture notes, contribution guidance, and changelog material to be aligned with what actually shipped.

The Raspberry adaptation must therefore:

- synthesize a parent `document-release` lane after ship/deploy readiness,
- require docs-sync output,
- treat documentation drift as a real shipping failure rather than an optional nicety.

Required parent artifacts:

- `docs-release.md`
- `docs-diff.json`

Hard-gate behavior:

- always a hard gate for parent completion.

The Raspberry adaptation must also preserve the upstream discipline of reading the canonical release-facing docs first and then cross-checking them against what actually shipped. That means the lane should enumerate the checked docs explicitly, record which docs changed, record which docs were checked and intentionally left alone, and require a machine-readable “docs current: yes|no” style verdict.

### `/retro`

The substantive behavior to preserve is structured reflection over what shipped, what failed, and what patterns repeated. Upstream `gstack` uses this for quality trends and future improvement, not for blocking release.

The Raspberry adaptation must therefore:

- synthesize a final reporting-only `retro` lane,
- summarize what shipped, what review stages found, what repeated, and what Fabro or product follow-ups deserve attention,
- ensure this output can feed future harness or prompt improvements.

Required parent artifacts:

- `retro.md`
- `retro-metrics.json` or `pattern-trends.json` when enough structured data exists

Gate behavior:

- reporting-only, never a blocker for parent completion.

The Raspberry adaptation must also preserve the upstream discipline of reviewing a concrete execution window rather than writing a generic retrospective. That means the lane should gather real commit history, completed child and parent lanes, retries, failure classes, and recovery patterns, then summarize recurring issues like stale artifact drift, provider/quota failures, replay loops, environment collisions, or review misses.

### `/codex`

The core adaptation for `/codex` is already described above. The key enforcement rule is that Codex must remain a separate second-pass lane with its own verdict and must not collapse into a short “Codex summary” appended to the first review.

## Appendix C — Per-Skill Programmatic Adoption Checklist

The final implementation is incomplete unless each adopted skill is converted into explicit non-interactive Fabro behavior. The minimum translation checklist per skill is:

- trigger condition,
- provider/model assignment,
- stage ordering,
- required command evidence,
- required artifacts,
- hard-gate vs conditional-gate vs reporting-only semantics,
- machine-readable pass/fail or verdict fields,
- parent milestone dependency.

That checklist should be documented directly in the implementation and in any new helper that synthesizes the parent lane family.

## Appendix D — Non-Interactive Translation Rules

The following upstream `gstack` features are deliberately not carried over literally and must instead be translated into deterministic Raspberry behavior:

- AskUserQuestion flows become conditional synthesis rules and hard-coded gate semantics.
- Telemetry, contributor mode, and user-facing preambles are omitted entirely.
- Browser-only affordances become conditional lanes only when the target repo can support them non-interactively.
- “Recommend option A/B/C” prose becomes machine-readable artifact fields or milestone dependencies.
- Human-trigger-only semantics become parent-lane trigger conditions based on child completion and risk profile.

For `document-release` specifically, replace “go read all the docs and decide what to update” with:

- a canonical doc set to inspect,
- a structured artifact listing docs reviewed and changed,
- a blocking verdict when release-facing docs are stale.

For `retro` specifically, replace “write a retrospective” with:

- a deterministic review window,
- a structured collection of commits, lane outcomes, failure classes, retries, and repeated patterns,
- a reporting-only artifact that can feed future Fabro or prompt improvements.

## Revision Note

Created on 2026-03-25 to capture the next-stage redesign of parent-level autodev review after observing that the current `plan-review` plus post-hoc `codex-review` model is too shallow compared with the `gstack` end-to-end shipping flow. This plan folds in current codebase context, local `gstack` skill review, and parallel agent recommendations so a future implementation can proceed from this file alone.
