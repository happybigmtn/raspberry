I've created both required durable artifacts for the `autodev-efficiency-and-dispatch` lane:

## Artifacts Created

### `outputs/autodev-efficiency-and-dispatch/spec.md` (179 lines, ~8KB)

A capability specification that defines:

1. **Purpose**: Make `raspberry autodev` run generated packages without local-only rescue steps

2. **Command Surface Invariant**: `fabro` binary must expose `run`, `validate`, `synth evolve`, `synth create`, `synth import`

3. **Prompt Resolution Contract**: Generated workflows must NOT resolve prompts from `~/.fabro/prompts/`

4. **Lane State Truth Contract**: Stale `running` lanes must be reclassified as `failed` within 30 seconds

5. **Dispatch Telemetry Contract**: Six required telemetry fields (`dispatch_rate`, `idle_cycles`, `ready_but_undispatched`, `failed_bootstrap_count`, `runtime_path_errors`, `stale_running_reclaimed`)

6. **Key Files**: Maps each contract to the source files (`autodev.rs`, `dispatch.rs`, `program_state.rs`, `evaluate.rs`, `render.rs`, `main.rs`)

7. **Acceptance Criteria**: Six verifiable outcomes for live validation

### `outputs/autodev-efficiency-and-dispatch/review.md` (223 lines, ~11KB)

A review document that:

1. **Summarizes what is working**: Lane state management, evaluation logic, dispatch, and orchestration all have solid foundations

2. **Identifies four issues**:
   - Issue 1: Command surface exposure (Medium) — `fabro synth` may not be reliably exposed
   - Issue 2: Prompt resolution (High) — `@../../prompts/...` paths resolve incorrectly
   - Issue 3: Dispatch telemetry gaps (Medium) — missing 6 telemetry fields
   - Issue 4: Blocking evolve (Low-Medium) — `synth evolve` delays dispatch

3. **Provides recommendations**: Code snippets for adding startup validation and telemetry structs

4. **Includes verification checklist**: 9 items to verify before live validation on rXMRbro

5. **Lists risks and mitigations**: Build configuration drift, prompt resolution, evolve race conditions