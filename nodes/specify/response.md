Both durable artifacts have been created:

**`outputs/autodev-efficiency-and-dispatch/spec.md`** (258 lines) — Defines the specification for:
1. **Runtime path self-consistency**: `fabro synth` availability in debug/release, prompt reference resolution from target repo, run config validation before dispatch, no local-only shims
2. **Stale lane state reconciliation**: Stale `running` lanes detected before dispatch (root cause: `evaluate_lane` re-classifies via stale `run_snapshot` after `refresh_program_state` correctly fixes the record), stale `failed` re-evaluation after render changes, stale lanes not consuming dispatch slots
3. **Dispatch telemetry**: `AutodevCycleReport` fields (`dispatch_skip_reason`, `ready_undispatched`, `runtime_path_errors`), `AutodevCurrentSnapshot` `dispatch_summary` fields (`idle_cycles`, `failed_bootstrap`, `stale_running_reclaimed`), `raspberry status` display
4. **Decouple evolve from dispatch**: Thread-based decoupling of `run_synth_evolve`
5. **Greedy dispatch**: Full `max_parallel` budget consumption per cycle

**`outputs/autodev-efficiency-and-dispatch/review.md`** (266 lines) — The review:
- Documents 5 specific gaps with severity ratings
- **Gap 1** (HIGH): Stale lane reclassification doesn't persist through evaluation — `is_active` in `evaluate.rs` falls through to stale `runtime_record.status == Running` after `refresh_program_state` corrects the record
- **Gap 2** (HIGH): Missing runtime path validation for graph/prompt resolution in `dispatch.rs`
- **Gap 3** (MEDIUM): `synth evolve` blocks dispatch synchronously
- **Gap 4** (MEDIUM): Incomplete dispatch telemetry fields
- **Gap 5** (MEDIUM): `../` prompt path escape resolution unverified
- Includes specific code locations requiring changes, test plan (unit + integration + live), risk matrix, and recommended fix ordering