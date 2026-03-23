# Centralize Provider Policy And Recover Live Autodev

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds.
Maintain this file in accordance with `/home/r/coding/fabro/PLANS.md`.

## Purpose / Big Picture

Fabro currently says provider and model selection should come from central configuration, but in
practice several code paths still hardcode Anthropic, OpenAI, and MiniMax choices directly. That
leak causes live runs to render impossible combinations like Claude Code with a MiniMax model and
causes synth work to fail instead of falling through to the next provider. After this change, all
normal write work should default to MiniMax through the `pi` harness, synth work should try Opus
4.6 then GPT-5.4 then MiniMax, and review work should try GPT-5.4 then Opus 4.6 then MiniMax. The
live proof of success is that newly rendered runs and live autodev attempts stop producing
Claude-plus-MiniMax mismatches and synth evolve starts moving again.

## Progress

- [x] (2026-03-23 02:05Z) Confirmed the existing live failures are caused by provider policy
      leaking into multiple layers: `fabro-cli/src/commands/synth.rs`,
      `fabro-synthesis/src/render.rs`, and `fabro-workflows/src/backend/cli.rs`.
- [x] (2026-03-23 02:12Z) Added `/home/r/coding/fabro/lib/crates/fabro-model/src/policy.rs`
      with shared write, review, and synth fallback chains and exported it from
      `/home/r/coding/fabro/lib/crates/fabro-model/src/lib.rs`.
- [x] (2026-03-23 02:17Z) Started moving normal write defaults in
      `/home/r/coding/fabro/lib/crates/fabro-cli/src/commands/run.rs` and
      `/home/r/coding/fabro/lib/crates/fabro-agent/src/cli.rs` to the shared write policy.
- [x] (2026-03-23 05:39Z) Switched
      `/home/r/coding/fabro/lib/crates/fabro-cli/src/commands/synth.rs` to profile-based
      automation chains for synth and review work, and removed the normal
      deterministic-steering fallback from `synth evolve`.
- [x] (2026-03-23 05:39Z) Made
      `/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs` execute the declared
      fallback chain on retryable failures with provider-specific environment setup per attempt.
- [x] (2026-03-23 05:39Z) Updated
      `/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs` and related tests so review
      nodes render the review profile primary target and write configs stay MiniMax-only.
- [x] (2026-03-23 05:39Z) Verified targeted tests for `fabro-model`, `fabro-workflows`,
      `fabro-synthesis`, and `fabro-cli` after the policy rollout.
- [x] (2026-03-23 05:42Z) Rebuilt `fabro` and `raspberry`, restarted the `rxmragent` watchdog,
      and confirmed a fresh controller lock on PID `3152914`.
- [x] (2026-03-23 05:56Z) Isolated the remaining stall to `run_synth_evolve()` inside the
      supervisor, added a timeout-and-skip guard, redeployed again, and observed the controller
      dispatch 5 lanes instead of hanging before cycle 1 completed.
- [x] (2026-03-23 05:59Z) Fixed a replay race in
      `/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/dispatch.rs` so explicitly
      replayed lanes that flip to `running` no longer crash the controller with `LaneNotReady`.
- [x] (2026-03-23 06:00Z) Re-rendered the full `rxmragent` package from
      `/home/r/coding/rXMRbro/malinka/blueprints/rxmragent.yaml`, which rewrote the checked-in
      `malinka/workflows` and `malinka/run-configs` to the new provider policy.
- [x] (2026-03-23 06:07Z) Taught
      `/home/r/coding/fabro/lib/crates/raspberry-supervisor/src/program_state.rs` to demote
      service lanes that still point at supervisor-only orchestration stubs back to `blocked`,
      which removed those fake failures from the live frontier.
- [ ] Keep observing whether the remaining failed set shrinks further as newly rendered workflows
      and runtime refreshes replace older stale failures.

## Surprises & Discoveries

- Observation: The shared policy module already existed locally, but the live system still failed
  because the backend loop never executed the policy fallbacks.
  Evidence: `/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs` had
  `central_policy_fallback_targets(...)` implemented but unused in `AgentCliBackend::run`.

- Observation: `synth evolve` had been patched earlier to fall back to a deterministic report
  instead of failing through the declared provider chain.
  Evidence: `/home/r/coding/fabro/lib/crates/fabro-cli/src/commands/synth.rs` printed
  `Mode: evolve (deterministic steering fallback)` after a failed Opus attempt.

- Observation: Rendered workflow graphs were already close to central policy, but tests and run
  config fallback expectations still assumed the old provider-per-provider fallback map.
  Evidence: `/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs` now reads review and
  write targets from the shared policy, while nearby assertions still expect legacy fallback text.

- Observation: The old backend integration tests were also encoding the retired repo-local
  `.fabro_cli` scratch directory behavior.
  Evidence: `/home/r/coding/fabro/lib/crates/fabro-workflows/tests/integration.rs` only passed
  once the mock sandbox learned the new `$HOME/.fabro/cli/run.*` scratch path and `rm -rf`
  cleanup command.

- Observation: After redeploy, the live controller process and lock updated immediately, but the
  frontier stayed at `ready=14`, `running=0`, `failed=17`, and no new
  `/home/r/.fabro/runs/**/provider_used.json` artifacts appeared within the first observation
  window.
  Evidence: `/home/r/coding/rXMRbro/.raspberry/rxmragent-autodev.lock` shows PID `3152914`
  acquired at `2026-03-23T05:41:28Z`, while
  `/home/r/coding/rXMRbro/.raspberry/rxmragent-autodev.json` still reports zero running lanes at
  `2026-03-23T05:42:28Z`.

- Observation: The root controller was not dead; it was hanging before cycle 1 completed inside
  `run_synth_evolve()`.
  Evidence: with `FABRO_AUTODEV_DEBUG_STEPS=1`, the watchdog log showed
  `program-before-evaluated`, `frontier ...`, and then `running-synth-evolve` with no later
  dispatch log until a timeout guard was added.

- Observation: A short synth-evolve timeout restores scheduler forward progress.
  Evidence: after lowering the timeout to 15 seconds, the watchdog log showed
  `synth-evolve-timeout-skipping-cycle-evolve`, then
  `dispatch-plan available_slots=5 ... dispatching=5`, and the live report moved to
  `running=5`, `failed=12`.

- Observation: Fresh running lanes now prove the centralized provider policy is live on actual
  work.
  Evidence: new `provider_used.json` files under `/home/r/.fabro/runs/20260323-01KMCM.../nodes/`
  show `implement` using MiniMax via `pi`, and review-related Codex runs using a rotator slot.

- Observation: The stale checked-in workflow package was still lagging behind source policy even
  after the runtime fixes.
  Evidence: before re-render, checked-in files like
  `/home/r/coding/rXMRbro/malinka/run-configs/implementation/blueprint-pipeline-blueprint-format.toml`
  still said `provider = "anthropic"` with `model = "MiniMax-M2.7-highspeed"`.

- Observation: Re-rendering from the checked-in blueprint flushes those stale graph/render choices.
  Evidence: after re-render, the same implementation workflow file now renders
  `#challenge` and `#review` as `gpt-5.4` on `openai`, and the same run config now has
  `[llm] provider = "minimax"` with no cross-provider mismatch.

- Observation: Demoting supervisor-only orchestration stubs out of `failed` materially reduces the
  failed set without affecting active work.
  Evidence: after the `program_state.rs` refresh fix, the live root snapshot moved from
  `failed=12` to `failed=8`, while
  `chain-operations-bring-miners-online`,
  `chain-operations-deploy-port-rename`, and
  `chain-operations-recover-wallet-passwords`
  now appear under `blocked`.

- Observation: The first timeout-based recovery exposed a second scheduler bug: replayed lanes
  could become `running` between autodev selection and dispatch re-evaluation, which crashed the
  controller with `lane ... is not ready to execute`.
  Evidence: the watchdog log showed the sequence
  `dispatch-plan available_slots=1 replayed=1 ...` followed by
  `Error: lane 'build-fix-ci-fix-street-import:build-fix-ci-fix-street-import' is not ready to execute`.

- Observation: After teaching dispatch to skip replay targets that are already `running`, the
  controller stays up and holds the worker pool.
  Evidence: the live report at `2026-03-23T05:58:51Z` shows `running=5`, `failed=12`, and the
  controller lease remains held by PID `3219946`.

## Decision Log

- Decision: Create a dedicated ExecPlan in `plans/` instead of editing the repository-root
  `PLANS.md`.
  Rationale: The root file is the repository-wide ExecPlan specification. Overwriting it would
  destroy instructions other contributors need.
  Date/Author: 2026-03-23 / Codex

- Decision: Keep MiniMax on the `pi` harness and OpenAI on the Codex harness by centralizing policy
  above command construction, not by duplicating per-command overrides.
  Rationale: The user wants one source of truth. Harness choice should follow provider choice, and
  provider choice should follow the shared automation profile.
  Date/Author: 2026-03-23 / Codex

## Outcomes & Retrospective

This section will be updated after the next code and runtime milestone. The current state is that
the shared policy is implemented, tested, rebuilt, deployed, and exercised by fresh live lanes.
The remaining gaps are operational polish: synth-evolve currently needs a timeout guard to avoid
wedging the controller, and some already-rendered workflow graphs still carry older provider choices
until regeneration rewrites them.

## Context and Orientation

The provider policy lives in `/home/r/coding/fabro/lib/crates/fabro-model/src/policy.rs`. That
crate is the best place for a central source of truth because both the CLI layer and the workflow
engine already depend on `fabro-model`.

Normal workflow execution enters through
`/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs`. That backend chooses a
harness (`claude`, `codex`, `pi`, or `gemini`), launches the external CLI inside the sandbox, and
parses the response back into Fabro. If fallback order is only stored in a helper and not executed
here, live runs still stop after the first provider failure.

Synth work enters through
`/home/r/coding/fabro/lib/crates/fabro-cli/src/commands/synth.rs`. That file contains the `synth
create`, `synth evolve`, `synth review`, `synth genesis`, and decomposition paths. It still has
direct `claude` shell commands and an earlier deterministic steering fallback. Those direct calls
must be replaced or wrapped so synth uses the shared provider chain instead of assuming Anthropic.

Rendered run configs and workflow graphs come from
`/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs`. That renderer writes the
`.fabro` workflow graph and the `.toml` run config that autodev later executes. If the renderer
stamps stale provider/model combinations into the graph, even perfect source policy changes will
not repair already-rendered work.

## Plan of Work

First, finish centralizing the model policy. Keep `/home/r/coding/fabro/lib/crates/fabro-model`
as the only place that defines automation profiles. The write profile remains MiniMax-only. The
review profile becomes GPT-5.4 first, then Opus 4.6, then MiniMax. The synth profile becomes Opus
4.6 first, then GPT-5.4, then MiniMax.

Next, update the runtime backend in
`/home/r/coding/fabro/lib/crates/fabro-workflows/src/backend/cli.rs` so a node whose requested
provider/model pair matches one of the central profiles walks the declared fallback chain on
retryable failures. When the actual provider is OpenAI, it must select a dedicated `CODEX_HOME`
from the codex rotator before launch. When the actual provider is MiniMax, it must stay on the
`pi` harness. The fallback loop should use the same error classification path the backend already
uses for provider-auth and model-access failures.

Then, remove direct provider assumptions from
`/home/r/coding/fabro/lib/crates/fabro-cli/src/commands/synth.rs`. Synth review, genesis,
decomposition, and evolve should use the synth or review profile as appropriate instead of direct
`claude` commands. Normal `synth evolve` should stop silently writing deterministic reports when
the first provider fails; it should either succeed through the fallback chain or fail honestly.
The explicit `--no-review` path can still generate a deterministic report because that is a
deliberate operator choice.

Finally, update rendering and tests in
`/home/r/coding/fabro/lib/crates/fabro-synthesis/src/render.rs` so new workflow graphs and run
configs line up with the shared provider policy, rebuild the local `target-local` binaries,
restart the live controller, and inspect the new frontier for evidence that the provider mismatch
class is gone.

## Concrete Steps

Work from `/home/r/coding/fabro`.

1. Read and patch the central policy, synth, backend, and renderer files.
2. Run formatting:

       cargo fmt --manifest-path /home/r/coding/fabro/Cargo.toml --all

3. Run targeted tests for the changed crates and behaviors.
4. Build fresh local binaries:

       CARGO_TARGET_DIR=/home/r/coding/fabro/target-local cargo build \
         --manifest-path /home/r/coding/fabro/Cargo.toml \
         -p fabro-cli --bin fabro \
         -p raspberry-cli --bin raspberry

5. Restart the autodev watchdog/controller from `/home/r/coding/rXMRbro` so it points at the
   freshly built binaries in `/home/r/coding/fabro/target-local/debug/`.
6. Inspect `/home/r/coding/rXMRbro/.raspberry/rxmragent-autodev.json`,
   `/home/r/coding/rXMRbro/.raspberry/rxmragent-watchdog.log`, and `raspberry status` for new
   frontier behavior and provider mismatch errors.

## Validation and Acceptance

Acceptance has three layers.

At the code layer, the relevant targeted tests must pass and the repo must build. At the rendered
config layer, new workflow graphs and run configs must request MiniMax for normal write work and
the review profile for challenge/review nodes. At the live runtime layer, new runs must stop
showing Anthropic with MiniMax-model mismatches, and `synth evolve` must no longer fail only
because Opus was the first provider in the chain.

## Idempotence and Recovery

All source edits are idempotent. Re-running `cargo fmt`, tests, and builds is safe. Redeploying
the controller is safe as long as only one `raspberry autodev` process owns the lock for the same
program. If a restart leaves the frontier worse, restore service by launching the last known good
`target-local` binaries and keeping the changed source tree intact for further patching.

## Artifacts and Notes

The live failure classes being addressed by this plan are currently:

- `proof_script_failure`
- `supervisor_only_lane`
- `transient_launch_failure`
- `landing=push_failed`

The provider-policy portion of this plan is specifically targeting the subset caused by stale or
impossible CLI routing, especially Anthropic-plus-MiniMax combinations and synth paths that assume
Opus is always available.

Recent validation transcripts:

- `cargo test -p fabro-model build_fallback_chain_accepts_explicit_model_ids -- --nocapture`
- `cargo test -p fabro-workflows cli_backend_run_writes_prompt_and_calls_exec -- --nocapture`
- `cargo test -p fabro-workflows cli_backend_run_fails_on_nonzero_exit -- --nocapture`
- `cargo test -p fabro-synthesis service_bootstrap_run_config_enables_direct_integration -- --nocapture`
- `cargo test -p fabro-cli synth -- --nocapture`
- `cargo build --manifest-path /home/r/coding/fabro/Cargo.toml -p fabro-cli --bin fabro -p raspberry-cli --bin raspberry`
- live watchdog observation in `/home/r/coding/rXMRbro/.raspberry/rxmragent-watchdog.log`

## Interfaces and Dependencies

The central policy API in `/home/r/coding/fabro/lib/crates/fabro-model/src/policy.rs` must remain
simple and reusable:

    pub enum AutomationProfile { Write, Review, Synth }
    pub struct ModelTarget { pub provider: Provider, pub model: &'static str }
    pub fn automation_chain(profile: AutomationProfile) -> &'static [ModelTarget]
    pub fn automation_primary_target(profile: AutomationProfile) -> ModelTarget
    pub fn automation_fallback_targets(profile: AutomationProfile) -> &'static [ModelTarget]

The CLI backend should consume those APIs rather than redefining fallback order locally. The synth
command layer should consume the same APIs and reuse backend parsing rules where practical.

Change note: created this ExecPlan to track the central provider-policy rollout, live redeploys,
and runtime observations because the current work spans source changes and live autodev recovery.

Change note: updated the plan after finishing the central policy rollout in the synth, renderer,
and CLI backend layers and after the targeted validation pass. The remaining work is live rebuild,
redeploy, and runtime observation.
