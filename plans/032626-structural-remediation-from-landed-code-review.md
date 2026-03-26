# Structural Remediation From Landed Code Review

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.

This document must be maintained in accordance with [PLANS.md](/home/r/coding/fabro/PLANS.md).

This plan builds on the repository state after [032526-parent-holistic-review-shipping-gauntlet.md](/home/r/coding/fabro/plans/032526-parent-holistic-review-shipping-gauntlet.md), but it repeats the relevant context here so a novice can execute it without reading that earlier plan first.

## Purpose / Big Picture

After this change, Fabro will do a better job preventing low-signal landed diffs and structurally weak slices from reaching `main`. The immediate user-visible improvement is that `integrate(...)` commits become easier to trust and easier to review: they stop carrying unrelated generated-package churn, they include stronger machine-generated invariants for risky UI and game-layout slices, and they keep evidence artifacts separate from product settlement work. You can see it working by generating a package, landing a feature lane, and verifying that the resulting integration commit contains only the owned product files plus any explicitly approved evidence path.

## Progress

- [x] (2026-03-26 14:05Z) Reviewed the latest landed `rXMRbro` integration commits to identify recurring structural failures rather than one-off product bugs.
- [x] (2026-03-26 14:08Z) Distilled the review into five Fabro-level themes: settlement commit hygiene, invariant-driven test synthesis, lane sizing pressure, evidence publication separation, and semantic-risk checks beyond core crates.
- [x] (2026-03-26 14:47Z) Implemented settlement-commit hygiene in `lib/crates/fabro-workflows/src/direct_integration.rs` so ordinary product settlements strip generated package churn under `malinka/**`, root `integration.md`, and evidence artifacts under `outputs/**` from the staged commit.
- [x] (2026-03-26 14:51Z) Added invariant-driven synthesis pressure in `lib/crates/fabro-synthesis/src/render.rs` for layout-heavy roulette-style slices, including required uniqueness/completeness tests and explicit review verdict fields.
- [x] (2026-03-26 14:54Z) Added lane-sizing pressure in `lib/crates/fabro-synthesis/src/render.rs` through prompt guidance and quality-gate structural debt detection for oversized mixed-responsibility UI files.
- [x] (2026-03-26 14:47Z) Separated evidence publication from product settlement for ordinary integrations by stripping `outputs/**/{spec,review,verification,quality,promotion}.md` from staged settlement commits in `direct_integration.rs`.
- [x] (2026-03-26 14:51Z) Extended semantic-risk coverage for UI/client slices by making roulette/layout screens inherit stronger invariant and decomposition checks rather than implementation-only pressure.
- [x] (2026-03-26 15:00Z) Rebuilt release binaries and regenerated a fresh `rXMRbro` clone to verify the new roulette prompt contracts and direct-integration hygiene changes from built binaries.

## Surprises & Discoveries

- Observation: the largest structural failure in the recent landed code was not a single bug but an `integrate(...)` commit that carried enormous generated package churn unrelated to the nominal settled slice.
  Evidence: commit `fc9412733` touched `malinka/blueprints/rxmragent.yaml`, `malinka/programs/rxmragent.yaml`, many `malinka/prompts/**`, `malinka/workflows/**`, and removed root `integration.md`, even though it was labeled `integrate(red-dog)`.

- Observation: the current review stack can still land a giant single-file UI slice that mixes state, input handling, rendering, and domain modeling without pressure to decompose.
  Evidence: commit `a244ed68a` landed [roulette.rs](/home/r/coding/rXMRbro/crates/tui/src/screens/roulette.rs) as a 952-line file with state mutation, input handling, animation, rendering, and tests in one unit.

- Observation: the current generated tests did not enforce domain invariants for rendered game layouts.
  Evidence: the landed roulette screen in [roulette.rs](/home/r/coding/rXMRbro/crates/tui/src/screens/roulette.rs) duplicates numbers in the board layout and omits others, but the tests only verify basic interactions and state transitions.

- Observation: evidence artifacts are still landing alongside product code in ordinary integration commits, which makes landed diffs noisier and harder to trust.
  Evidence: commits like `a244ed68a` and `8acd801af` changed both source files and `outputs/<lane>/spec.md` / `outputs/<lane>/review.md`.

- Observation: semantic-risk checks already improved core game logic review, but UI/client slices can still model unsafe randomness and trust boundaries in ways that mirror earlier core bugs.
  Evidence: the landed roulette TUI in [roulette.rs](/home/r/coding/rXMRbro/crates/tui/src/screens/roulette.rs#L213) synthesizes local spin seed material in the UI layer.

- Observation: ordinary parent/report lanes were still inheriting implementation-stage system prompts until the workflow backend was corrected, which helped explain weird root-path and cross-lane artifact writes in live parent preflight runs.
  Evidence: live `baccarat-holistic-preflight` and `blueprint-pipeline-holistic-preflight` runs showed root `verification.md` and even cross-lane `outputs/blackjack-holistic-preflight/*` writes before the stage prompt routing was fixed.

## Decision Log

- Decision: treat settlement-commit hygiene as the highest-priority remediation in this plan.
  Rationale: when one landed commit contains thousands of unrelated generated files, review signal collapses and all other quality defenses become harder to trust.
  Date/Author: 2026-03-26 / Codex

- Decision: implement invariant synthesis and lane-sizing pressure as Fabro-level authoring and quality-gate logic, not as tribal review advice.
  Rationale: these problems are repeatable and machine-detectable enough that the harness should catch them before human review has to.
  Date/Author: 2026-03-26 / Codex

- Decision: separate evidence publication from product settlement instead of merely “warning” about evidence churn in landed diffs.
  Rationale: the current system already proved that warnings are not enough; the workflow must make the cleaner path the default.
  Date/Author: 2026-03-26 / Codex

- Decision: extend semantic-risk detection to UI/client slices that can fabricate randomness or mis-model authority boundaries, even if they do not directly mutate core game state.
  Rationale: trust-boundary regressions can leak into presentation/client code in ways that later normalize unsafe architecture.
  Date/Author: 2026-03-26 / Codex

## Outcomes & Retrospective

This plan is newly authored, so no remediation work is complete yet. The main lesson from the review is that Fabro’s recent improvements to parent review and live recovery were necessary, but they do not yet solve a different class of structural quality issues: noisy settlement commits, under-specified invariants, oversized slices, and evidence churn. This plan addresses that separate layer.

The first implementation slice is now complete. Direct integration has a real hygiene fence against generated-package and evidence noise, and synthesis now emits stronger structural pressure for roulette-style layout slices. The remaining gap is live rollout hardening and follow-through: these guards are implemented, tested, and regenerated in a fresh target clone, but they still need a clean live rollout cycle if we want all future `rXMRbro` controller activity to pick them up immediately.

## Context and Orientation

There are three relevant repositories or paths in this work.

The first is the Fabro synthesis layer in [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs). “Synthesis” means the code that turns a blueprint into generated Raspberry package files: `malinka/programs/*.yaml`, `malinka/workflows/*.fabro`, `malinka/run-configs/*.toml`, and prompt files under `malinka/prompts/`. If a change should alter what lanes exist, what they verify, or what prompts they receive, it probably belongs here.

The second is the Raspberry supervisor layer in [autodev.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/autodev.rs), [dispatch.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/dispatch.rs), and [evaluate.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/evaluate.rs). “Supervisor” means the runtime control plane that decides which lanes are ready, which should be replayed, and how they get dispatched.

The third is the generated target package in `rXMRbro`, especially [rxmragent.yaml](/home/r/coding/rXMRbro/malinka/programs/rxmragent.yaml). That file is the generated manifest that the supervisor executes. You should never hand-edit it as a permanent solution. Instead, change synthesis and regenerate.

Two terms matter here.

“Settlement commit hygiene” means the guarantee that an `integrate(<lane>)` commit only contains files that belong to the actual settled slice, plus any explicitly approved metadata path. It must not silently include package regeneration, prompt churn, or unrelated evidence files.

“Invariant-driven synthesis” means generating tests or quality checks that assert domain truths, not just “the code compiles” or “some interaction happened.” For a roulette layout, a useful invariant is “every number from 0 through 36 appears exactly once in the rendered board model.”

The review findings driving this plan came from recent landed commits in `rXMRbro/main`, especially:

- `a244ed68a` `integrate(roulette)`
- `fc9412733` `integrate(red-dog)`
- `8acd801af` `integrate(provably-fair-integration-tests)`

The most important structural issues identified were:

- an `integrate(...)` commit carrying large unrelated `malinka/**` churn,
- a monolithic 952-line landed roulette TUI file,
- missing invariant tests for roulette board correctness,
- evidence artifacts landing with product code,
- and UI-layer trust-boundary smells around locally fabricated randomness.

## Plan of Work

Start in [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs). Add a settlement-hygiene layer that distinguishes product settlement from package regeneration and evidence publication. The generated integration path must know which paths are owned by the settled lane and which paths are generated package churn. If a proposed integration would include generated package files under `malinka/**`, root `integration.md`, `.raspberry/**`, or large prompt/workflow churn unrelated to the settled slice, synthesis or integration policy must prevent that from being landed as a normal product commit. The simplest first implementation is to emit explicit allowed-path metadata or a commit-hygiene contract for integration lanes, then have the supervisor or integration helper reject commits that violate it.

Add invariant-driven synthesis for layout- and board-heavy slices. In [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs), extend the current semantic-risk and security-sensitive logic with a second family of domain-layout cues. When a lane clearly builds a game board, grid, wheel, table, or screen representing a domain value set, the generated contract and review prompts must require invariants such as uniqueness, completeness, and valid ordering where appropriate. Start with roulette because we have a concrete failure. The generated review or challenge prompt should force a check like “all roulette numbers 0..=36 appear exactly once,” and the quality gate should reject obvious duplicate/omission patterns when a board model is hard-coded.

Add lane-sizing pressure in synthesis. This does not mean “count lines and fail everything.” It means biasing the system away from landing slices that mix unrelated responsibilities. In [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs), add heuristics that recognize when a proposed slice simultaneously includes state management, input handling, rendering, animation, and domain data in one file. In those cases, the plan prompt should require decomposition into smaller slices, and the review/quality path should flag oversized mixed-responsibility additions as quality debt instead of silently accepting them.

Separate evidence publication from product settlement. In practice, the current landed diffs show that `outputs/<lane>/spec.md` and `outputs/<lane>/review.md` are still making it into product commits. Decide one explicit policy and encode it. The cleanest first policy is: normal `integrate(...)` commits may include product code and integration metadata only; evidence artifacts either remain in the run directory, go to a metadata branch, or land in a dedicated evidence-only path/commit. The implementation point may live in the supervisor integration path rather than synthesis, but the contract should be visible in synthesis too so prompts and audits align.

Extend semantic-risk checks to UI/client layers. In [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs), add UI/client-facing trust-boundary cues so generated prompts and quality gates look for phrases and code patterns that indicate local randomness fabrication, fake seed generation, or authority-bypass modeling in screens and client flows. The point is not to ban all local simulation. The point is to force the lane to explicitly justify it and to prevent unreviewed normalization of unsafe patterns.

After those synthesis and policy changes, update the relevant integration or supervisor layer if required so the new guards are actually enforced at landing time. Then regenerate a real target repo, ideally `rXMRbro`, and prove that:

- generated parent or child prompts now ask for domain invariants where relevant,
- integration commits no longer absorb giant unrelated package churn,
- and evidence artifacts do not piggyback on ordinary product settlement without an explicit policy allowing it.

## Concrete Steps

Work from `/home/r/coding/fabro`.

1. Inspect the current synthesis and integration surfaces.

       cd /home/r/coding/fabro
       sed -n '430,1160p' lib/crates/fabro-synthesis/src/render.rs
       sed -n '1,260p' lib/crates/raspberry-supervisor/src/integration.rs

   Look for where integration commits are assembled and where lane prompts / artifact contracts are emitted.

2. Add settlement-hygiene enforcement.

   Decide how an integration lane learns its allowed landed paths. A minimal acceptable version is:

       - synthesize an explicit allowed-path contract for integration,
       - enforce it in supervisor integration code,
       - reject or split commits that include unrelated `malinka/**`, `.raspberry/**`, root `integration.md`, or unrelated evidence churn.

   After editing, run:

       cargo test -p raspberry-supervisor --lib --no-run

3. Add invariant-driven synthesis for board/layout slices in [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs).

   The roulette case should generate checks that force a uniqueness/completeness invariant over board numbers.

   Run:

       cargo test -p fabro-synthesis --lib --no-run

4. Add lane-sizing pressure.

   Update prompts and quality logic so a single new file that combines too many responsibilities becomes review debt or decomposition pressure rather than silently passing.

   Re-run:

       cargo test -p fabro-synthesis --lib --no-run

5. Add evidence-publication separation.

   Update the integration or settlement logic so ordinary product commits do not silently include `outputs/**` review/spec churn unless explicitly allowed by policy.

   Re-run:

       cargo test -p raspberry-supervisor --lib --no-run

6. Extend semantic-risk checks to UI/client slices.

   Start by teaching synthesis to flag local randomness fabrication or fake authority modeling in TUI/frontend contexts.

   Re-run:

       cargo test -p fabro-synthesis --lib --no-run

7. Rebuild the release binaries.

       cargo build --release -p fabro-cli -p raspberry-cli --target-dir target-local

8. Regenerate a real target repo from the source blueprint.

       /home/r/coding/fabro/target-local/release/fabro --no-upgrade-check synth create \
         --target-repo /home/r/coding/rXMRbro \
         --program rxmragent \
         --blueprint /home/r/coding/rXMRbro/malinka/blueprints/rxmragent.yaml \
         --no-decompose --no-review

9. Inspect the generated package for the new contracts and guards.

       rg -n "invariant|unique|0..=36|allowed paths|evidence|outputs/" \
         /home/r/coding/rXMRbro/malinka/prompts \
         /home/r/coding/rXMRbro/malinka/workflows \
         /home/r/coding/rXMRbro/malinka/run-configs

10. Validate against a real integration or a synthetic integration test.

    The acceptance proof should show either:

       - a candidate integration commit is rejected because it contains unrelated generated package churn, or
       - a generated roulette-like slice now contains and enforces the board invariants that were previously missing.

## Validation and Acceptance

This plan is accepted only when all of the following are true:

- a regenerated target repo contains stronger prompt/audit pressure for structured board/layout invariants,
- integration/settlement logic no longer allows unrelated generated package churn to ride along in ordinary `integrate(...)` commits,
- evidence artifacts are no longer silently coupled to product settlement work,
- oversized mixed-responsibility slices produce decomposition or quality pressure instead of silently landing unchanged,
- and UI/client trust-boundary smells are covered by semantic-risk or review guidance.

Concrete proof must include:

- passing focused synthesis tests,
- passing focused supervisor/integration tests,
- a regenerated package showing the new contracts,
- and at least one real or synthetic demonstration that the new guard catches a previously observed failure mode.

## Idempotence and Recovery

All synthesis changes must remain idempotent. If a generated package looks wrong, fix the synthesis code and regenerate. Do not hand-edit `malinka/programs/*.yaml`, `malinka/workflows/*.fabro`, `malinka/run-configs/*.toml`, or prompt files in the target repo as a permanent solution.

For live controller validation, assume `rXMRbro/main` may move independently on `origin/main`. Before restarting a controller or manually dispatching a lane, verify local `main` is current. If the repo is behind and dirty only because of generated package churn, use the new auto-heal path or perform the same safe sequence manually: stop the controller, stash generated package changes, fast-forward `main`, regenerate from the blueprint, then restart. If the dirt includes user code, stop and preserve it instead of forcing the sync.

## Artifacts and Notes

The most important evidence from the review that motivated this plan:

    git -C /home/r/coding/rXMRbro show --name-only --format='' fc9412733 | sed -n '1,40p'

This should show that `integrate(red-dog)` touched many generated package files under `malinka/**`, which is exactly the hygiene failure this plan aims to stop.

    nl -ba /home/r/coding/rXMRbro/crates/tui/src/screens/roulette.rs | sed -n '549,570p'

This should show the hard-coded roulette board rows where duplicates and omissions can be inspected directly.

## Interfaces and Dependencies

Keep the synthesis-side work in [render.rs](/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs) unless a type truly belongs in another crate. Keep landing-policy and runtime enforcement in the Raspberry supervisor layer, most likely [integration.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/integration.rs), [dispatch.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/dispatch.rs), or [autodev.rs](/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/autodev.rs).

The finished system should expose, directly or implicitly, these interfaces:

- a way to determine whether a landed path is allowed for a given integration lane,
- a way to classify board/layout slices as requiring invariant-driven prompts or checks,
- a way to classify oversized mixed-responsibility slices as requiring decomposition pressure,
- and a way to distinguish evidence publication from ordinary product settlement.

Do not invent a parallel policy system if the existing lane goals, prompt context, produced artifact sets, or supervisor integration manifests can carry the new information cleanly.

Change note: added on 2026-03-26 to capture structural remediation work revealed by review of landed `rXMRbro` commits. This plan exists because the earlier parent-gauntlet plan improved parent review, but it did not directly address noisy settlement commits, invariant synthesis gaps, oversized slices, or evidence-publication hygiene.

Change note (2026-03-26 15:00Z): updated after implementation to record that settlement hygiene, evidence stripping, layout invariants, and lane-sizing pressure are now present in code and validated against tests plus a fresh regenerated `rXMRbro` clone.
