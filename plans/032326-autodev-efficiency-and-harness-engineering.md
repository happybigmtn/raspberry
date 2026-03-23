# Autodev Efficiency: Next Steps

Post-mortem from overnight rxmragent runs (2026-03-22/23) and forward-looking
improvements. All review and execution models now route through MiniMax via pi;
gpt-5.4 removed from automation policy due to quota exhaustion.

## Current State (2026-03-23)

| Metric | Value |
|--------|-------|
| Total lanes | 221 |
| Complete | 7 (2 with real work) |
| Failed | 26 |
| Blocked | 188 |
| Cycles run | 580 |
| Dispatch rate | 16.2% (94/580 cycles dispatched) |
| Idle with ready work | 83.3% of cycles |
| Code landed to main | 0 lanes |

**Two lanes produced real work** (`mining-operations`, `wallet-rpc-reference`).
The remaining 5 "complete" lanes are orchestration parents with no run IDs.
Zero integration lanes were dispatched, so nothing merged to trunk.

---

## 1. Model Routing (DONE)

**Change:** Removed gpt-5.4 from all automation profiles in `policy.rs`.

| Profile | Before | After |
|---------|--------|-------|
| Write | MiniMax only | MiniMax only (unchanged) |
| Review | gpt-5.4 → Opus → MiniMax | MiniMax → Opus |
| Synth | Opus → gpt-5.4 → MiniMax | Opus → MiniMax |

**Why:** OpenAI Codex quota exhausted mid-run. 8 lanes hit `surface_blocked` at
the Quality Gate stage with zero tokens consumed. Error: "You've hit your usage
limit... try again at Mar 25th." Quota consumption at $2.50/$15.00 per megatoken
is unsustainable for continuous overnight runs.

**Where:** `lib/crates/fabro-model/src/policy.rs` (control plane only). All
downstream consumers (render.rs, synth.rs, cli.rs) read from the policy and will
automatically pick up the change on next build/re-render.

---

## 2. Overnight Failure Taxonomy

### 2a. Deterministic Verify Cycle (11 lanes)

Recovery: `regenerate_lane` (requires evolve, which never fires — see §3a).

Root cause: MiniMax implementations fail the same verify script 3x. In 8/11
cases, MiniMax produced 0 tokens (provider refused or prompt was too large).
The fixup stage (also MiniMax) cannot correct what was never produced.

**Highest-impact failure:** `provably-fair` blocks `casino-core`, which
transitively blocks all 28 game-type lanes and ~147 children. This single lane
is the critical path bottleneck for the entire program.

Affected: `provably-fair`, `test-coverage`, 4× `wallet-integration-tui-*`,
3× `blueprint-pipeline-*`, `chain-operations-genesis-workaround`,
`monero-infrastructure-subaddress-generation`.

### 2b. Provider Access Limited (8 lanes)

Recovery: `surface_blocked` (will not auto-retry).

Root cause: All 8 lanes reached the review/quality gate stages, which were
routed to gpt-5.4 via Codex. Codex quota hit at ~07:53 UTC. Lanes completed
implementation (MiniMax) successfully but could not pass the OpenAI-gated
quality gates.

**Now resolved by routing review to MiniMax (§1).**

### 2c. Proof Script Failure (7 lanes)

Recovery: `replay_lane`.

Two sub-causes:
- 5 lanes failed at Start with "goal gate unsatisfied for node verify and no
  retry target" — documentation/analysis lanes with unmet verify preconditions.
- `build-fix-ci-add-ci-pipeline` replayed **37 times** (36% of all dispatches)
  because `cargo clippy --workspace -- -D warnings` fails on pre-existing
  warnings in code outside the lane's owned surfaces. Every replay is futile.

### 2d. Stale Milestone / Bootstrap Gap (3 lanes)

`chain-operations` is marked complete with no run_id (pre-bootstrapped), but 3
children remain permanently blocked because the `reviewed` milestone was never
formally registered.

---

## 3. Velocity Improvements (TODO)

### 3a. Fix Frontier Budget Gate for Evolve (CRITICAL)

**File:** `lib/crates/raspberry-supervisor/src/autodev.rs:837`

`should_trigger_evolve` suppresses evolve when `frontier.total_work() >= frontier_budget`,
where `frontier_budget = max_parallel + 2 = 7`. With 15 ready + 5 running = 20, this
gate is permanently closed.

**Impact:** 11 lanes with `regenerate_lane` recovery are permanently stuck. They
need evolve to re-synthesize their run config before they can retry. This is the
single biggest structural blocker.

**Fix:** Scale frontier_budget proportionally to total lane count, or bypass the
budget gate entirely when regeneration targets exist. The existing
`should_fast_track_regenerate_evolve` has a 15s retry but is gated by
`frontier_progressed`, which stalls once failures stabilize.

### 3b. Add Replay Attempt Limits (HIGH)

**Files:** `lib/crates/raspberry-supervisor/src/program_state.rs`, `failure.rs`

No replay ceiling exists. `build-fix-ci-add-ci-pipeline` replayed 37 times,
consuming 36% of all dispatches with zero progress. After N consecutive replays
(suggest 5) with the same failure signature, escalate to `SurfaceBlocked`.

### 3c. Skip Evaluation When No Slots Available (MEDIUM)

**File:** `lib/crates/raspberry-supervisor/src/autodev.rs`

When `available_slots == 0`, the controller still runs full `evaluate_program`
(reads 221 lane progress files, run directories, check probes). A lightweight
"has any running lane finished?" pre-check would eliminate 83% of evaluation
overhead.

### 3d. Critical Path Surface (MEDIUM)

Add a critical-path analysis to the evaluation output: which failed lanes
transitively block the most downstream work. This enables targeted operator
intervention on the lanes that matter most (`provably-fair` → 147 blocked).

### 3e. Dynamic Stall Timeouts (LOW)

`ACTIVE_STALL_TIMEOUT_SECS = 1800` (30 min). One stalled lane reduces
throughput by 20% for 30 minutes in a 5-slot pool. Early stages (sandbox init,
file reads) should timeout faster than late stages (compilation, verification).

### 3f. Verify Integration Lane Wiring (LOW)

The 2 lanes that completed with real work (`mining-operations`,
`wallet-rpc-reference`) did not trigger integration lanes. Verify that
integration lanes exist in the manifest for these lanes and that their
dependency milestones are satisfiable.

---

## 4. Pi Harness Engineering (TODO)

### 4a. Pass `--thinking` to Pi (HIGH)

**File:** `lib/crates/fabro-workflows/src/backend/cli.rs:193`

The workflow graph sets `reasoning_effort` per node (high for implement/review,
medium for polish/challenge) but the CLI backend drops it. Pi supports
`--thinking <level>` (off, minimal, low, medium, high, xhigh).

**Fix:** `cli_command_for_provider` should accept the node's `reasoning_effort`
and map it to `--thinking`. This is the highest-ROI harness change — deeper
reasoning for implementation, lighter reasoning for fixup passes.

### 4b. Restrict Tools for Review/Challenge Stages (MEDIUM)

Current: all 7 tools enabled for every stage (`read,bash,edit,write,grep,find,ls`).

Review and challenge nodes should NOT have write access. The review prompt
already says "prefer not to modify source code here." Restricting to
`read,bash,grep,find,ls` for review stages:
- Shrinks the system prompt (fewer tool definitions = more context for reasoning)
- Prevents accidental writes during adversarial review
- Clearer separation of concerns between implementation and review

### 4c. Add `--no-extensions --no-skills --no-prompt-templates --no-themes` (MEDIUM)

Pi auto-discovers extensions and skills on startup. Disabling these eliminates
startup overhead and prevents external plugins from interfering with the
controlled automation pipeline.

### 4d. Reduce Review Prompt Boilerplate (MEDIUM)

The review prompt duplicates all implementation context (~40 lines of
`promotion.md` format spec, stage ownership rules, deterministic evidence
rules). These are static across all lanes. Options:
- Extract static review instructions into a single doctrine file
- Reference via `--append-system-prompt` (pi supports this) to improve cache
  hit rates
- Compress format specs into terser versions

### 4e. Model-Aware Prompt Budget (LOW)

`PROMPT_BUDGET_BYTES = 900_000` (~225K tokens). MiniMax-M2.7's effective context
is 196K tokens. With 16K max output reserved, the input budget should cap at
~180K tokens (~720KB). The current 900KB budget risks sending prompts that
overflow MiniMax's context.

**Fix:** Query the model's context window from the catalog and compute the
budget dynamically: `(context_window - max_output) * 4` bytes.

---

## 5. End-to-End Workflow Improvements (TODO)

### 5a. Verify Script Scoping

`build-fix-ci-add-ci-pipeline` fails because `cargo clippy --workspace` catches
warnings in files outside the lane's owned surfaces. Verify scripts should be
scoped to the lane's touched surfaces, not the entire workspace. This prevents
pre-existing issues from blocking unrelated lanes.

### 5b. Concurrency Configuration

The program YAML sets `max_parallel=25` but the effective limit is 5. Diagnose
why the YAML setting is not being honored and increase to match available
provider throughput (MiniMax has no parallel request limit from our side).

### 5c. Evolve-on-Regeneration Fast Path

Instead of waiting for the normal evolve cadence, implement a fast-path evolve
that fires immediately when any lane transitions to `regenerate_lane` recovery.
This closes the feedback loop between failure detection and fix re-synthesis.

### 5d. Provider Fallback Telemetry

Log when fallback chains activate and which provider succeeds. This enables
tracking provider reliability over time and informing policy changes with data
rather than quota-exhaustion incidents.

---

## Priority Stack

| # | Item | Impact | Effort | Section | Status |
|---|------|--------|--------|---------|--------|
| 1 | Fix frontier budget gate | Critical | Small | §3a | DONE |
| 2 | Pass `--thinking` to pi | High | Small | §4a | DONE |
| 3 | Add replay attempt limits | High | Small | §3b | DONE |
| 4 | Restrict review tools | Medium | Small | §4b | DONE |
| 5 | Add pi startup flags | Medium | Trivial | §4c | DONE |
| 6 | Verify script scoping | Medium | Medium | §5a | DONE (via §3b) |
| 7 | Skip idle evaluation | Medium | Small | §3c | DONE |
| 8 | Model-aware prompt budget | Low | Small | §4e | DONE |
| 9 | Reduce review boilerplate | Medium | Medium | §4d | DONE |
| 10 | Critical path surface | Medium | Medium | §3d | DONE |
| 11 | Fix concurrency limit | Medium | Small | §5b | DONE |
| 12 | Evolve-on-regeneration | Medium | Medium | §5c | DONE (via §3a) |
| 13 | Dynamic stall timeouts | Low | Medium | §3e | |
| 14 | Provider fallback telemetry | Low | Small | §5d | DONE |
| 15 | Integration lane wiring | Low | Small | §3f | |

---

## Round 2: Quality and Harness Review (2026-03-23 afternoon)

Live run state: 25 complete, 4 running, 6 ready. MiniMax handling all stages
via pi. Three-agent review uncovered critical issues.

### Round 2 Current State

| Metric | Before Round 2 | After Round 2 Fixes |
|--------|---------------|---------------------|
| Complete | 25 | 25 (in-progress) |
| Running | 5 | 4 |
| Quality gate scan_placeholder | DEAD CODE | Active on owned surfaces |
| cargo run verify scripts | Hang forever (30min stall) | cargo build (compile-only) |
| System prompts | 4 stages | 6 stages (added deep_review, escalation) |
| Plan prompts | Miss full context | Always include Full Slice Contract |
| Catalog MiniMax-M2.7 (anthropic) | Dead entry with wrong provider | Removed |
| Pi launch quoting | Broken (pid="") | Fixed via heredoc script |
| MiniMax alias shadowing | minimax-m2.5 shadows m2.7 | Fixed |

### Round 2 Fixes (DONE)

| # | Fix | Section | Status |
|---|-----|---------|--------|
| R2.1 | Fix scan_placeholder() dead code in quality gates | §6a | DONE |
| R2.2 | Fix cargo run verify scripts → cargo build | §6b | DONE |
| R2.3 | Add deep_review/escalation system prompts | §6c | DONE |
| R2.4 | Plan prompts always include full slice context | §6d | DONE |
| R2.5 | Remove dead MiniMax-M2.7 catalog entry | §6e | DONE |
| R2.6 | Fix pi launch shell quoting (heredoc) | §6f | DONE |
| R2.7 | Fix MiniMax-M2.7-highspeed alias shadowing | §6g | DONE |
| R2.8 | Remove Kimi K2.5 from review (Codex incompatible) | §6h | DONE |

### Round 2 Findings (TODO - Next Round)

| # | Finding | Impact | Effort | Status |
|---|---------|--------|--------|--------|
| R2.9 | Reject no-op lanes in audit gate | Critical | Small | DONE |
| R2.10 | Chain sibling lanes sharing surfaces | Critical | Medium | DONE (92 deps) |
| R2.11 | Merge game logic siblings (5→2 lanes/game) | High | Medium | Deferred (chaining solves) |
| R2.12 | Sequence test lanes after implementation | High | Medium | DONE (via R2.10) |
| R2.13 | Raise max_parallel to 5-6 after surface fixes | Medium | Small | |
| R2.14 | Add routing JSON instructions to system prompts | Medium | Small | N/A (exit codes) |
| R2.15 | Remove dead challenge→review edges in hardened graphs | Low | Small | |
| R3.1 | Enforce surface ownership in audit gate | Critical | Medium | DONE |
| R3.2 | Scope verify scripts (cargo test --workspace → -p crate) | Critical | Medium | DONE |
| R3.3 | Anti-no-op implement system prompt | High | Small | DONE |
| R3.4 | Surface ownership in fixup system prompt | High | Small | DONE |
| R3.5 | Review stage git diff check instruction | High | Small | DONE |
| R4.1 | Implement prompt: anti-behavioral-stub language | Critical | Small | DONE |
| R4.2 | Challenge prompt: hunt hollow implementations | Critical | Small | DONE |
| R4.3 | Plan prompt: require full lifecycle tests | High | Small | DONE |
| R4.4 | Quality gate: test quality ratio check | High | Medium | DONE |
| R4.5 | AGENTS.md: scoped commands, surface ownership | Medium | Small | DONE |
| R4.6 | Audit gate: quality.md hard gate | Medium | Small | DONE |

---

## §6: Round 2 Details

### 6a. Quality Gate scan_placeholder() (DONE)

**File:** `lib/crates/fabro-synthesis/src/render.rs`

The quality gate shell script defined `scan_placeholder()` but called `true`
instead of invoking it on owned surfaces. Fixed by parsing "Owned surfaces:"
from the lane goal text and generating `scan_placeholder 'path'` calls.

### 6b. Cargo Run Verify Scripts (DONE)

**File:** `lib/crates/fabro-synthesis/src/render.rs` (default_verify_command)

Proof commands starting with `cargo run` are replaced with `cargo build` during
extraction, preventing 30-minute hangs on server/TUI lanes. 10 legacy lanes
with `cargo run` baked into blueprint goals still need re-decomposition.

### 6c. Deep Review/Escalation System Prompts (DONE)

**File:** `lib/crates/fabro-workflows/src/backend/cli.rs` (stage_system_prompt)

Added match arms for `deep_review` (adversarial deep review with file/line
citations) and `escalation` (backward-compat review for foundation code).

### 6d. Plan Prompt Full Context (DONE)

**File:** `lib/crates/fabro-synthesis/src/render.rs` (render_implementation_plan_prompt)

Changed from only including context when no structured sections exist, to
always including the full slice contract. MiniMax writing code now sees the
same concrete expectations (test vectors, interface specs) that the reviewer
sees.

### 6e-6h. Catalog/Config Fixes (DONE)

- Removed dead `MiniMax-M2.7` entry under `provider: "anthropic"` from catalog.json
- Removed `MiniMax-M2.7-highspeed` alias from `minimax-m2.5` (was shadowing real entry)
- Removed Kimi K2.5 from review chain (Codex CLI uses /v1/responses, Kimi only supports /v1/chat/completions)
- Fixed shell quoting in pi launch: heredoc script instead of nested single quotes

### 6i. Reject No-Op Lanes in Audit Gate (DONE)

**File:** `lib/crates/fabro-synthesis/src/render.rs` (implementation_audit_command)

**Finding:** MiniMax occasionally declares "no-op slice" and writes promotion.md
with `merge_ready: yes` without implementing anything. The quality gate passes
(nothing to scan) and the audit passes (promotion.md is well-formed). The lane
completes with zero code changes.

**Fix:** Added a git-diff guard to the audit script that requires at least one
file to be changed. Lanes that genuinely need no code changes (rare) will fail
at audit and get fixup'd — better than silently approving empty work.
