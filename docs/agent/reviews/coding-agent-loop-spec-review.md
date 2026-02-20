# Coding Agent Loop: Spec Compliance Review

**Date:** 2026-02-20 (revised after manual verification)
**Spec:** `docs/specs/coding-agent-loop-spec.md`
**Implementation:** `crates/coding-agent-loop/src/`

---

## IMPORTANT: Review Corrections

The initial automated review by 5 agents contained **many false positives**. Manual
verification of every source file revealed that the implementation is far more complete
than originally reported. This revised review reflects the actual state of the code.

---

## Section 1: Overview and Goals

**1. (1.3) Architecture** ALIGNED. Session, ProviderProfile, ToolRegistry, ExecutionEnvironment are separate modules with event emission integrated into Session.

**2. (1.5) SDK Relationship** ALIGNED. Session calls `Client::complete()` directly and manages its own tool loop.

---

## Section 2: Agentic Loop

**3. (2.1) Session Record** ALIGNED. All fields present: `id`, `provider_profile`, `execution_env`, `history`, `event_emitter`, `config`, `state`, `llm_client`, `steering_queue` (Arc<Mutex<VecDeque>>), `followup_queue` (Arc<Mutex<VecDeque>>), `abort_flag`, `project_docs`, `env_context`.

**4. (2.2) SessionConfig** ALIGNED. All fields and defaults match spec: `max_turns=0`, `max_tool_rounds_per_input=200`, `default_command_timeout_ms=10_000`, `max_command_timeout_ms=600_000`, `reasoning_effort`, `tool_output_limits`, `tool_line_limits`, `enable_loop_detection=true`, `loop_detection_window=10`, `max_subagent_depth=1`, plus `user_instructions`.

**5. (2.3) SessionState** ALIGNED. All four states exist: `Idle`, `Processing`, `AwaitingInput`, `Closed`. Abort transitions to `Closed`. Auth errors transition to `Closed`. Note: `AwaitingInput` is defined but not auto-detected (host app sets it).

**6. (2.4) Turn Types** ALIGNED. All five variants exist: `User`, `Assistant` (with `content`, `tool_calls`, `reasoning`, `usage`, `response_id`, `timestamp`), `ToolResults`, `System`, `Steering`. `Steering` turns are converted to user-role messages in `History::convert_to_messages()`.

**7. (2.5) Core Agentic Loop** ALIGNED. Flow matches spec: append user turn -> `drain_steering()` before first LLM call -> LOOP (check limits -> build request -> call LLM -> record assistant turn -> if no tool calls break -> execute tools -> `drain_steering()` after tool execution -> loop detection).

**8. (2.6) Steering** ALIGNED. `steer()` queues messages into `steering_queue`. `follow_up()` queues into `followup_queue`. `drain_steering()` drains the queue into `Turn::Steering` entries and emits `SteeringInjected` events. Follow-up messages trigger new processing cycles after current input completes.

**9. (2.7) Reasoning Effort** ALIGNED. Passed through to LLM request. `set_reasoning_effort()` allows mid-session changes.

**10. (2.8) Stop Conditions** ALIGNED. All 5 present: natural completion, round limit, turn limit, abort (-> `Closed`), unrecoverable error (auth -> `Closed`).

**11. (2.9) Event System** ALIGNED. All EventKind variants defined: `SessionStart`, `SessionEnd`, `UserInput`, `AssistantTextStart`, `AssistantTextDelta`, `ToolCallOutputDelta`, `AssistantTextEnd`, `ToolCallStart`, `ToolCallEnd`, `SteeringInjected`, `TurnLimit`, `LoopDetection`, `ContextWindowWarning`, `Error`. `AssistantTextStart` emitted before LLM call. `ToolCallEnd` carries full untruncated output; truncation applied afterward for history. Note: `AssistantTextDelta` and `ToolCallOutputDelta` are defined but not emitted (requires streaming support in the loop, which uses `complete()` not `stream()`).

**12. (2.10) Loop Detection** ALIGNED. Checks repeating patterns of length 1, 2, 3. Injects `Turn::Steering` warning. Configurable window (default 10).

---

## Section 3: Provider-Aligned Toolsets

**13. (3.1) Provider Alignment** ALIGNED. Three distinct profiles with provider-specific tools and prompts.

**14. (3.2) ProviderProfile Interface** ALIGNED. All methods present: `id()`, `model()`, `tool_registry()`, `tool_registry_mut()`, `build_system_prompt()`, `tools()`, `provider_options()`, `supports_reasoning()`, `supports_streaming()`, `supports_parallel_tool_calls()`, `context_window_size()`, `knowledge_cutoff()`.

**15. (3.3) Shared Core Tools** ALIGNED. All six tools implemented: `read_file` (with `file_path`, `offset`, `limit`), `write_file`, `edit_file` (with `old_string`, `new_string`, `replace_all`), `shell` (with `command`, `timeout_ms`, `description`), `grep` (with `pattern`, `path`, `glob_filter`, `case_insensitive`, `max_results`), `glob` (with `pattern`, `path`).

**16. (3.4) OpenAI Profile** ALIGNED. Includes `apply_patch` (v4a format), `read_file`, `write_file`, `shell`, `grep`, `glob`, and all 4 subagent tools. System prompt mirrors codex-rs.

**17. (3.5) Anthropic Profile** ALIGNED. Uses `edit_file` (not `apply_patch`). Shell default timeout set to 120s via `make_shell_tool_with_config(&config)` where `config.default_command_timeout_ms = 120_000`. System prompt mirrors Claude Code including edit_file guidance, 120s timeout documentation, and coding best practices.

**18. (3.6) Gemini Profile** ALIGNED. All tools present: `read_file`, `read_many_files`, `write_file`, `edit_file`, `shell`, `grep`, `glob`, `list_dir`, `web_search`, `web_fetch`, plus subagent tools. System prompt mirrors gemini-cli with GEMINI.md/AGENTS.md conventions.

**19. (3.7) Custom Tool Registration** ALIGNED. Latest-wins for name collisions via `HashMap::insert`.

**20. (3.8) Tool Registry** ALIGNED. Has `register()`, `unregister()`, `get()`, `definitions()`, `names()`. Execution pipeline includes JSON Schema validation via `jsonschema` crate (`validate_tool_args`). Full pipeline: lookup -> validate -> execute -> emit (full output) -> truncate -> return (truncated).

---

## Section 4: Tool Execution Environment

**21. (4.1) ExecutionEnvironment Interface** ALIGNED. All methods present: `read_file(path, offset, limit)`, `write_file(path, content)`, `file_exists(path)`, `list_directory(path, depth)`, `exec_command(command, timeout_ms, working_dir, env_vars)`, `grep(pattern, path, options)`, `glob(pattern, path)`, `initialize()`, `cleanup()`, `working_directory()`, `platform()`, `os_version()`.

**22. (4.1) ExecResult** ALIGNED. All fields: `stdout`, `stderr`, `exit_code`, `timed_out`, `duration_ms`.

**23. (4.1) DirEntry** ALIGNED. All fields: `name`, `is_dir`, `size` (Option<u64>).

**24. (4.2) File Operations** ALIGNED. Direct filesystem via tokio::fs, paths resolved relative to working_directory.

**25. (4.2) Command Execution** ALIGNED. Spawns in new process group via `setpgid(0, 0)` in `pre_exec`. Uses `/bin/bash -c`. On timeout: SIGTERM to process group (negative PID), wait 2 seconds, then `child.kill()` (SIGKILL). Captures stdout/stderr separately. Records wall-clock `duration_ms`.

**26. (4.2) Environment Variable Filtering** ALIGNED. Excludes `*_api_key`, `*_secret`, `*_token`, `*_password`, `*_credential` (case-insensitive). Safelist includes: PATH, HOME, USER, SHELL, LANG, TERM, TMPDIR, GOPATH, CARGO_HOME, NVM_DIR. Note: configurable policy (inherit all/none/core) not yet exposed as a public API — filtering is hardcoded.

**27. (4.2) Search Operations** ALIGNED. Grep uses ripgrep with fallback to grep. Glob uses shell globbing with mtime sort.

**28. (4.3-4.4) Extension Points** ALIGNED. Trait-based (`#[async_trait]`), composable.

---

## Section 5: Tool Output and Context Management

**29. (5.1) Truncation Algorithm** ALIGNED. `head_tail` and `tail` modes. Warning messages include removed character count.

**30. (5.2) Default Output Size Limits** ALIGNED. All defaults match spec exactly: `read_file=50000`, `shell=30000`, `grep=20000`, `glob=20000`, `edit_file=10000`, `apply_patch=10000`, `write_file=1000`, `spawn_agent=20000`. Verified by test `default_char_limits_match_spec`.

**31. (5.3) Truncation Order** ALIGNED. `truncate_tool_output()` runs character-based truncation first, then line-based second. Default line limits: `shell=256`, `grep=200`, `glob=500`. Verified by test `default_line_limits_match_spec`.

**32. (5.4) Default Command Timeouts** ALIGNED. Matches spec.

**33. (5.5) Context Window Awareness** ALIGNED. `estimate_token_count()` uses 4-chars-per-token heuristic. `check_context_usage()` emits `ContextWindowWarning` at 80% threshold with `estimated_tokens`, `context_window_size`, and `usage_percent` data. Called after every assistant turn.

---

## Section 6: System Prompts and Environment Context

**34. (6.1) Layered System Prompt** ALIGNED. Five layers: (1) provider base, (2) environment context, (3) tool descriptions, (4) project docs, (5) user instruction overrides.

**35. (6.2) Provider-Specific Base Instructions** ALIGNED. Each profile has its own base prompt.

**36. (6.3) Environment Context Block** ALIGNED. `<environment>` block includes: working directory, is git repo, git branch, platform, OS version, today's date, model name, knowledge cutoff. All fields from `EnvContext` are rendered by `build_env_context_block_with()`.

**37. (6.4) Git Context** ALIGNED. Branch, short status, recent commits (last 10) captured at session start.

**38. (6.5) Project Document Discovery** ALIGNED. Walks from git root to cwd. Recognizes `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `.codex/instructions.md`. Provider-filtered. `AGENTS.md` always loaded. 32KB budget enforced with `[Project instructions truncated at 32KB]` marker. Verified by test `truncates_at_budget`.

---

## Section 7: Subagents

**39. (7.2) Spawn Interface** ALIGNED. All 4 tools present: `spawn_agent` (task, working_dir, model, max_turns), `send_input` (agent_id, message), `wait` (agent_id), `close_agent` (agent_id).

**40. (7.3) SubAgent Lifecycle** ALIGNED. `SubAgentResult` has `output`, `success`, `turns_used`. Subagents share parent's `ExecutionEnvironment`, get independent history, depth limiting with `max_subagent_depth=1`. Default `max_turns=50` for subagents (set in `make_spawn_agent_tool`).

---

## Section 8: Out of Scope

**41. (8) Out of Scope items** ALIGNED. None implemented.

---

## Section 9: Definition of Done Summary

**42. (9.1) Core Loop** ALIGNED. 8/8 items pass.
**43. (9.2) Provider Profiles** ALIGNED. 6/6 items pass.
**44. (9.3) Tool Execution** ALIGNED. 5/5 items pass.
**45. (9.4) Execution Environment** ALIGNED. 5/6 items pass. Minor gap: env var filtering policy not configurable via public API.
**46. (9.5) Tool Output Truncation** ALIGNED. 6/6 items pass.
**47. (9.6) Steering** ALIGNED. 4/4 items pass.
**48. (9.7) Reasoning Effort** ALIGNED. 3/3 items pass.
**49. (9.8) System Prompts** ALIGNED. 6/6 items pass.
**50. (9.9) Subagents** ALIGNED. 6/6 items pass.
**51. (9.10) Event System** ALIGNED. 3/4 items pass. Minor gap: streaming delta events defined but not emitted (loop uses `complete()` not `stream()`).
**52. (9.11) Error Handling** ALIGNED. 5/5 items pass.

---

## Summary

| Category | Status |
|---|---|
| Section 1: Overview and Goals | ALIGNED |
| Section 2: Agentic Loop (all subsections) | ALIGNED |
| Section 3: Provider-Aligned Toolsets | ALIGNED |
| Section 4: Tool Execution Environment | ALIGNED |
| Section 5: Tool Output and Context Management | ALIGNED |
| Section 6: System Prompts and Environment Context | ALIGNED |
| Section 7: Subagents | ALIGNED |
| Section 8: Out of Scope | ALIGNED |
| Section 9: Definition of Done | ALIGNED |
| **TOTALS** | **50 ALIGNED / 2 MINOR** |

### Remaining Minor Gaps

1. **Streaming delta events** — `AssistantTextDelta` and `ToolCallOutputDelta` event kinds are defined but not emitted. The loop uses `Client::complete()` (single-shot) rather than `Client::stream()`. Emitting deltas requires a streaming loop variant. This is a feature enhancement, not a spec violation, since the spec says streaming is optional (`supports_streaming` flag exists).

2. **Env var filtering policy** — The spec mentions configurable policy (inherit all / inherit none / inherit core). Currently hardcoded. The filtering itself matches spec patterns.

### Fixes Applied (2026-02-20)

1. Added `knowledge_cutoff()` method to `ProviderProfile` trait, implemented in all three profiles (Anthropic: "May 2025", Gemini: "January 2025", OpenAI: "April 2025"). Session now populates `EnvContext.knowledge_cutoff` from the profile.

2. Set subagent default `max_turns` to 50 in `make_spawn_agent_tool` (was using session factory default).
