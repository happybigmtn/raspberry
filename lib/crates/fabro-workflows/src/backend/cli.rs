use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use fabro_agent::sandbox::ExecResult;
use fabro_agent::Sandbox;
use fabro_model::{automation_fallback_targets, automation_profile_for_target, Provider};
use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::cost::compute_stage_cost;
use crate::error::FabroError;
use crate::event::{EventEmitter, WorkflowRunEvent};
use crate::handler::agent::{CodergenBackend, CodergenResult};
use crate::outcome::StageUsage;
use fabro_graphviz::graph::Node;

/// Maps a provider to its corresponding CLI tool metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentCli {
    Claude,
    Codex,
    Gemini,
    Pi,
}

impl AgentCli {
    pub fn for_provider(provider: Provider) -> Self {
        match provider {
            Provider::Anthropic => Self::Claude,
            Provider::Gemini => Self::Gemini,
            Provider::Minimax | Provider::Kimi => Self::Pi,
            Provider::OpenAi | Provider::Zai | Provider::Inception => Self::Codex,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::Pi => "pi",
        }
    }

    pub fn npm_package(self) -> &'static str {
        match self {
            Self::Claude => "@anthropic-ai/claude-code",
            Self::Codex => "@openai/codex",
            Self::Gemini => "@anthropic-ai/gemini-cli",
            Self::Pi => "@mariozechner/pi-coding-agent",
        }
    }
}

fn inherit_host_provider_credentials(cli: AgentCli) -> bool {
    !matches!(cli, AgentCli::Codex | AgentCli::Claude)
}

/// Ensure the CLI tool for the given provider is installed in the sandbox.
///
/// Checks if the CLI binary exists; if not, installs Node.js (if missing) and
/// the CLI via npm. Emits `CliEnsure*` events for observability.
async fn ensure_cli(
    cli: AgentCli,
    provider: Provider,
    sandbox: &Arc<dyn Sandbox>,
    emitter: &Arc<EventEmitter>,
) -> Result<(), FabroError> {
    let start = std::time::Instant::now();
    let cli_name = cli.name();
    let provider_str = provider.as_str();

    emitter.emit(&WorkflowRunEvent::CliEnsureStarted {
        cli_name: cli_name.to_string(),
        provider: provider_str.to_string(),
    });

    // Check if the CLI is already installed (include ~/.local/bin for npm-installed CLIs)
    let version_check = sandbox
        .exec_command(
            &format!("PATH=\"$HOME/.local/bin:$PATH\" {cli_name} --version"),
            30_000,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| FabroError::handler(format!("Failed to check {cli_name} version: {e}")))?;

    if version_check.exit_code == 0 {
        let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        emitter.emit(&WorkflowRunEvent::CliEnsureCompleted {
            cli_name: cli_name.to_string(),
            provider: provider_str.to_string(),
            already_installed: true,
            node_installed: false,
            duration_ms,
        });
        return Ok(());
    }

    // Install Node.js (if needed) and the CLI in a single shell so PATH persists
    let install_cmd = format!(
        "export PATH=\"$HOME/.local/bin:$PATH\" && \
         (node --version >/dev/null 2>&1 || \
          (mkdir -p ~/.local && curl -fsSL https://nodejs.org/dist/v22.14.0/node-v22.14.0-linux-x64.tar.gz | tar -xz --strip-components=1 -C ~/.local)) && \
         npm install -g {}",
        cli.npm_package()
    );
    let install_result = sandbox
        .exec_command(&install_cmd, 180_000, None, None, None)
        .await
        .map_err(|e| FabroError::handler(format!("Failed to install {cli_name}: {e}")))?;

    let node_installed = true;
    if install_result.exit_code != 0 {
        let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let output = if install_result.stderr.is_empty() {
            &install_result.stdout
        } else {
            &install_result.stderr
        };
        let detail: String = output
            .chars()
            .rev()
            .take(500)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let error_msg = format!(
            "{cli_name} install exited with code {}: {detail}",
            install_result.exit_code
        );
        emitter.emit(&WorkflowRunEvent::CliEnsureFailed {
            cli_name: cli_name.to_string(),
            provider: provider_str.to_string(),
            error: error_msg.clone(),
            duration_ms,
        });
        return Err(FabroError::handler(error_msg));
    }

    let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    emitter.emit(&WorkflowRunEvent::CliEnsureCompleted {
        cli_name: cli_name.to_string(),
        provider: provider_str.to_string(),
        already_installed: false,
        node_installed,
        duration_ms,
    });

    Ok(())
}

/// Models that are only available through CLI tools (not via API).
const CLI_ONLY_MODELS: &[&str] = &[];

/// Returns true if the given model is only available through a CLI tool.
#[must_use]
pub fn is_cli_only_model(model: &str) -> bool {
    CLI_ONLY_MODELS.contains(&model)
}

const PI_TOOLS_FULL: &str = "read,bash,edit,write,grep,find,ls";
const PI_TOOLS_READONLY: &str = "read,bash,grep,find,ls";

fn is_readonly_stage(node_id: &str) -> bool {
    matches!(
        node_id,
        "challenge" | "review" | "deep_review" | "escalation"
    )
}

fn stage_system_prompt(node_id: &str) -> Option<&'static str> {
    match node_id {
        "implement" | "specify" => Some(
            "You are executing the IMPLEMENT stage of an automated pipeline. RULES: (1) Read existing code before implementing. (2) Implement functionality COMPLETELY. Code that compiles but does nothing is worse than code that fails to compile — the review will catch hollow implementations. (3) Every public function must have real logic, not a stub that returns a default or immediately transitions state without doing work. (4) Write tests that exercise BEHAVIORAL outcomes: given specific input, assert specific output. Tests for Display, Clone, PartialEq derive macros do not count. (5) Include at least one full lifecycle test that drives the system from initial state to terminal state through multiple actions. (6) Run the proof commands from your goal to verify changes work. (7) Write required durable artifacts (spec.md, review.md) BEFORE finishing — the audit gate will reject if these are missing. Write spec.md with your implementation plan and review.md with your self-assessment. Do this in this stage, not later. ANTI-PATTERNS TO AVOID: returning hardcoded values, marking state as complete without performing the action (e.g. Hit without drawing a card), writing tests that only cover the happy path. The challenge stage will specifically look for functions that compile but do nothing meaningful. SURFACE OWNERSHIP: you may ONLY modify files listed under Owned surfaces. The audit gate rejects changes outside your scope."
        ),
        "fixup" | "polish" => Some(
            "You are executing the FIXUP stage after the verify gate failed. Read the failure output from prior stages to understand what went wrong. Your #1 priority: make the proof commands pass. Your #2 priority: ensure required durable artifacts (spec.md, review.md) exist in the outputs directory — the audit gate will reject if they are missing. SURFACE OWNERSHIP: you may ONLY modify files listed under Owned surfaces. If failures are from code outside your surfaces, IGNORE them and focus on your owned files only. Do not delete, modify, or rewrite files outside your scope."
        ),
        "challenge" => Some(
            "You are executing an ADVERSARIAL CHALLENGE. SPECIFICALLY CHECK: (1) Functions that compile but do nothing meaningful — e.g. a Hit action that does not draw a card, a settle function that returns hardcoded values. Read every match arm and verify it performs real work. (2) Tests that only verify derive macros (Display, Clone, PartialEq) — count how many tests exercise actual business logic vs formatting. (3) State machines that never reach their terminal state through normal gameplay. Run through the lifecycle mentally: can a user actually play the game from start to finish? (4) Duplicate tests that inflate coverage without testing new behavior. (5) DESIGN PATTERN CONFORMANCE: Read AGENTS.md and verify the code follows the mandatory conventions — especially settlement arithmetic (must widen to i32, no bare f64-to-i16 casts), error types (must use shared GameError/VerifyError), and state machine patterns ({Game}Phase enum, is_terminal()). Flag every issue with file:line. Write findings to verification.md. Do NOT approve."
        ),
        "review" => Some(
            "You are executing the REVIEW stage with merge authority. Read quality.md (machine-generated). Read the implementation and verification artifacts. Read AGENTS.md for the mandatory design conventions. Write promotion.md with merge_ready: yes|no. Only approve when proof gates pass, tests verify real behavior, no stubs or placeholders remain, AND the code follows the project's design conventions (settlement arithmetic uses i32 widening, error types from shared error.rs, state machine patterns). CRITICAL: run `git diff --stat HEAD` to verify actual code changes exist. If no files were changed, set merge_ready: no with reason explaining what implementation is missing."
        ),
        "deep_review" => Some(
            "You are executing an ADVERSARIAL DEEP REVIEW in an automated pipeline. Challenge every trust boundary, input validation, and error path. Verify that tests exercise real behavioral outcomes, not trivial assertions. Check for placeholder debt (todo!, unimplemented!, stub comments). Write your findings to deep-review-findings.md with specific file paths and line numbers."
        ),
        "escalation" => Some(
            "You are executing an ESCALATION REVIEW for code that modifies shared foundation infrastructure. Verify backward compatibility: all downstream consumers must continue to compile and pass tests. Check that public API changes are additive, not breaking. Approve only if all existing tests pass and new interfaces are documented. Write escalation-verdict.md."
        ),
        _ => None,
    }
}

/// Build the CLI command string for a given provider.
///
/// The `prompt_file` is the path to a file containing the prompt text, which
/// is piped into the command's stdin via `cat`.
#[must_use]
pub fn cli_command_for_provider(
    provider: Provider,
    model: &str,
    prompt_file: &str,
    reasoning_effort: Option<&str>,
    readonly: bool,
    system_prompt: Option<&str>,
) -> String {
    // Use `cat | command` instead of `command < file` because the background
    // launch wrapper (`setsid sh -c '...' </dev/null`) can clobber stdin
    // redirects in nested shells. A pipe creates an explicit new stdin.
    match provider {
        // --yolo: auto-approve all tool calls and bypass sandbox prompts
        Provider::OpenAi | Provider::Zai | Provider::Inception => {
            let model_flag = if model.is_empty() {
                String::new()
            } else {
                format!(" -m {model}")
            };
            format!("cat {prompt_file} | codex exec --json --yolo{model_flag}")
        }
        Provider::Minimax | Provider::Kimi => {
            let pi_provider = match provider {
                Provider::Kimi => "kimi-coding",
                _ => "minimax",
            };
            // Map fabro catalog model names to pi provider model IDs
            let pi_model = match provider {
                Provider::Kimi => "k2p5",
                _ => model,
            };
            let model_flag = if pi_model.is_empty() {
                String::new()
            } else {
                format!(" --model {pi_model}")
            };
            let thinking_flag = match reasoning_effort {
                Some(effort) if effort != "high" => format!(" --thinking {effort}"),
                _ => " --thinking high".to_string(),
            };
            let tools = if readonly {
                PI_TOOLS_READONLY
            } else {
                PI_TOOLS_FULL
            };
            let sys_prompt_flag = match system_prompt {
                Some(text) => format!(" --append-system-prompt '{}'", text.replace('\'', "'\\''")),
                None => String::new(),
            };
            format!(
                "prompt=\"$(cat {prompt_file})\" && pi --provider {pi_provider} --mode json -p --no-session --no-extensions --no-skills --no-prompt-templates --tools {tools}{model_flag}{thinking_flag}{sys_prompt_flag} \"$prompt\""
            )
        }
        // --yolo: auto-approve all tool calls
        Provider::Gemini => {
            let model_flag = if model.is_empty() {
                String::new()
            } else {
                format!(" -m {model}")
            };
            format!("cat {prompt_file} | gemini -o json --yolo{model_flag}")
        }
        // --dangerously-skip-permissions: bypass all permission checks (required for non-interactive use).
        // CLAUDECODE= unset to allow running inside a Claude Code session.
        Provider::Anthropic => {
            let model_flag = if model.is_empty() {
                String::new()
            } else {
                format!(" --model {model}")
            };
            format!("cat {prompt_file} | CLAUDECODE= claude -p --verbose --output-format stream-json --dangerously-skip-permissions{model_flag}")
        }
    }
}

/// Parsed response from a CLI tool invocation.
#[derive(Debug)]
pub struct CliResponse {
    pub text: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub served_model: Option<String>,
}

/// Parse NDJSON output from Claude CLI (`--output-format stream-json`).
///
/// Looks for the last `{"type":"result",...}` line, extracts `result` text and `usage`.
fn parse_claude_ndjson(output: &str) -> Option<CliResponse> {
    let mut last_result: Option<serde_json::Value> = None;
    let mut served_model: Option<String> = None;

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(model) = value.get("model").and_then(|v| v.as_str()) {
                served_model = Some(model.to_string());
            }
            if value.get("type").and_then(|t| t.as_str()) == Some("result") {
                last_result = Some(value);
            }
        }
    }

    let result = last_result?;
    let text = result
        .get("result")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let input_tokens = result
        .pointer("/usage/input_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let output_tokens = result
        .pointer("/usage/output_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    Some(CliResponse {
        text,
        input_tokens,
        output_tokens,
        served_model,
    })
}

/// Parse NDJSON output from Codex CLI (`codex exec --json`).
///
/// Codex emits NDJSON lines. Text comes from `item.completed` events where
/// `item.type == "agent_message"`. Usage comes from the `turn.completed` event.
fn parse_codex_ndjson(output: &str) -> Option<CliResponse> {
    let mut last_message_text = String::new();
    let mut input_tokens: i64 = 0;
    let mut output_tokens: i64 = 0;
    let mut found_anything = false;
    let mut served_model: Option<String> = None;

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let event_type = value.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if let Some(model) = value.get("model").and_then(|t| t.as_str()) {
            served_model = Some(model.to_string());
        }

        match event_type {
            "item.completed" => {
                let item_type = value
                    .pointer("/item/type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                if item_type == "agent_message" {
                    if let Some(text) = value.pointer("/item/text").and_then(|t| t.as_str()) {
                        last_message_text = text.to_string();
                        found_anything = true;
                    }
                }
            }
            "turn.completed" => {
                input_tokens = value
                    .pointer("/usage/input_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                output_tokens = value
                    .pointer("/usage/output_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                found_anything = true;
            }
            _ => {}
        }
    }

    if !found_anything {
        return None;
    }

    Some(CliResponse {
        text: last_message_text,
        input_tokens,
        output_tokens,
        served_model,
    })
}

/// Parse JSON output from Gemini CLI (`-o json`).
///
/// Gemini outputs a single JSON object with `response` for text and
/// `stats.models.<model>.tokens` for usage.
fn parse_gemini_json(output: &str) -> Option<CliResponse> {
    let value: serde_json::Value = serde_json::from_str(output.trim()).ok()?;
    let text = value
        .get("response")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Extract tokens from the first model in stats.models
    let (input_tokens, output_tokens) = value
        .pointer("/stats/models")
        .and_then(|m| m.as_object())
        .and_then(|models| models.values().next())
        .map(|model_stats| {
            let input = model_stats
                .pointer("/tokens/input")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let output = model_stats
                .pointer("/tokens/candidates")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            (input, output)
        })
        .unwrap_or((0, 0));

    Some(CliResponse {
        text,
        input_tokens,
        output_tokens,
        served_model: value
            .pointer("/stats/models")
            .and_then(|m| m.as_object())
            .and_then(|models| models.keys().next().cloned()),
    })
}

fn parse_pi_message(value: &serde_json::Value) -> Option<CliResponse> {
    let message = value.get("message")?;
    if message.get("role").and_then(|role| role.as_str()) != Some("assistant") {
        return None;
    }

    let content = message.get("content")?.as_array()?;
    let text = content
        .iter()
        .filter(|block| block.get("type").and_then(|kind| kind.as_str()) == Some("text"))
        .filter_map(|block| block.get("text").and_then(|text| text.as_str()))
        .collect::<String>();
    let input_tokens = message
        .pointer("/usage/input")
        .and_then(|value| value.as_i64())
        .unwrap_or(0);
    let output_tokens = message
        .pointer("/usage/output")
        .and_then(|value| value.as_i64())
        .unwrap_or(0);
    let served_model = message
        .get("model")
        .and_then(|model| model.as_str())
        .map(ToString::to_string);

    Some(CliResponse {
        text,
        input_tokens,
        output_tokens,
        served_model,
    })
}

fn parse_pi_json(output: &str) -> Option<CliResponse> {
    let mut last_assistant: Option<CliResponse> = None;

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let event_type = value.get("type").and_then(|value| value.as_str());

        match event_type {
            Some("message_end") => {
                if let Some(parsed) = parse_pi_message(&value) {
                    last_assistant = Some(parsed);
                }
            }
            Some("agent_end") => {
                let messages = value.get("messages").and_then(|value| value.as_array());
                if let Some(messages) = messages {
                    for message in messages {
                        let wrapped = serde_json::json!({ "message": message });
                        if let Some(parsed) = parse_pi_message(&wrapped) {
                            last_assistant = Some(parsed);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    last_assistant
}

/// Parse CLI output, choosing the right parser based on provider.
pub fn parse_cli_response(provider: Provider, output: &str) -> Option<CliResponse> {
    match provider {
        Provider::OpenAi | Provider::Zai | Provider::Inception => parse_codex_ndjson(output),
        Provider::Minimax | Provider::Kimi => parse_pi_json(output),
        Provider::Gemini => parse_gemini_json(output),
        Provider::Anthropic => parse_claude_ndjson(output),
    }
}

/// Escape a value for safe embedding inside single quotes in a shell command.
fn shell_escape(val: &str) -> String {
    val.replace('\'', "'\\''")
}

fn shell_quote(val: &str) -> String {
    format!("'{}'", shell_escape(val))
}

struct ProviderUsedJson<'a> {
    requested_provider: Provider,
    requested_model: &'a str,
    provider: Provider,
    model: &'a str,
    served_model: Option<&'a str>,
    fallback_reason: Option<&'a str>,
    codex_slot: Option<&'a CodexSlotSelection>,
    command: &'a str,
}

async fn write_provider_used_json(stage_dir: &Path, details: ProviderUsedJson<'_>) {
    let provider_used = serde_json::json!({
        "mode": "cli",
        "requested_provider": details.requested_provider.as_str(),
        "requested_model": details.requested_model,
        "provider": details.provider.as_str(),
        "model": details.model,
        "served_model": details.served_model,
        "fallback_reason": details.fallback_reason,
        "codex_slot": details.codex_slot.map(|slot| serde_json::json!({
            "slot_name": slot.slot_name,
            "codex_home": slot.codex_home,
            "config_path": slot.config_path,
            "state_path": slot.state_path,
        })),
        "command": details.command,
    });
    if let Ok(json) = serde_json::to_string_pretty(&provider_used) {
        let _ = tokio::fs::write(stage_dir.join("provider_used.json"), json).await;
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexRotatorConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    selection_policy: Option<String>,
    #[serde(default)]
    cooldown_seconds: Option<u64>,
    #[serde(default)]
    state_path: Option<PathBuf>,
    #[serde(default)]
    shared_state_path: Option<PathBuf>,
    #[serde(default)]
    slots: Vec<CodexRotatorSlot>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexRotatorSlot {
    name: String,
    codex_home: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexRotatorState {
    #[serde(default)]
    selected_slot: Option<String>,
    #[serde(default)]
    slots: HashMap<String, CodexRotatorSlotState>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CodexRotatorSlotState {
    #[serde(default)]
    last_selected_at_ms: u64,
    #[serde(default)]
    cooldown_until_ms: u64,
    #[serde(default)]
    last_failure_reason: Option<String>,
    #[serde(default)]
    last_failure_at_ms: Option<u64>,
}

#[derive(Debug, Clone)]
struct CodexRotator {
    config_path: PathBuf,
    state_path: PathBuf,
    config: CodexRotatorConfig,
    state: CodexRotatorState,
}

#[derive(Debug, Clone)]
struct CodexSlotSelection {
    slot_name: String,
    codex_home: PathBuf,
    config_path: PathBuf,
    state_path: PathBuf,
    cooldown_seconds: u64,
}

fn resolve_cli_target(
    requested_provider: Provider,
    requested_model: &str,
    _default_model: &str,
    _launch_env: &HashMap<String, String>,
    _codex_slots_available: bool,
) -> (Provider, String, Option<String>) {
    (requested_provider, requested_model.to_string(), None)
}

#[derive(Debug, Clone)]
struct CliAttemptTarget {
    provider: Provider,
    model: String,
    fallback_reason: Option<String>,
}

fn build_cli_attempt_targets(
    requested_provider: Provider,
    requested_model: &str,
    default_model: &str,
) -> Vec<CliAttemptTarget> {
    let initial = resolve_cli_target(
        requested_provider,
        requested_model,
        default_model,
        &HashMap::new(),
        load_codex_rotator().is_some() || fallback_codex_slot_selection().is_some(),
    );
    let mut targets = vec![CliAttemptTarget {
        provider: initial.0,
        model: initial.1.clone(),
        fallback_reason: initial.2,
    }];
    for (provider, model) in central_policy_fallback_targets(initial.0, &initial.1) {
        targets.push(CliAttemptTarget {
            provider,
            model,
            fallback_reason: Some(format!(
                "fallback from {}:{} via central automation policy",
                initial.0.as_str(),
                initial.1
            )),
        });
    }
    targets
}

fn central_policy_fallback_targets(provider: Provider, model: &str) -> Vec<(Provider, String)> {
    let Some(profile) = automation_profile_for_target(provider, model) else {
        return Vec::new();
    };
    automation_fallback_targets(profile)
        .iter()
        .map(|target| (target.provider, target.model.to_string()))
        .collect()
}

fn cli_failure_is_retryable_for_fallback(detail: &str) -> bool {
    let lower = detail.to_ascii_lowercase();
    lower.contains("not logged in")
        || lower.contains("/login")
        || lower.contains("selected model")
        || lower.contains("may not exist or you may not have access to it")
        || lower.contains("you've hit your limit")
        || lower.contains("usage limit has been reached")
        || lower.contains("rate_limit")
        || lower.contains("401 unauthorized")
}

fn build_launch_env_for_provider(
    provider: Provider,
    explicit_env: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut launch_env = HashMap::new();
    let cli = AgentCli::for_provider(provider);
    if inherit_host_provider_credentials(cli) {
        for name in provider.api_key_env_vars() {
            if let Ok(val) = std::env::var(name) {
                launch_env.insert((*name).to_string(), val);
            }
        }
    }
    for (name, val) in explicit_env {
        launch_env.insert(name.clone(), val.clone());
    }
    if provider == Provider::OpenAi {
        launch_env.remove("OPENAI_API_KEY");
    }
    // Kimi routes through pi CLI (kimi-coding provider) which reads
    // KIMI_API_KEY from the environment or ~/.pi/agent/auth.json.
    launch_env
}

fn tail_chars(value: &str, limit: usize) -> String {
    value
        .chars()
        .rev()
        .take(limit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn format_cli_failure_detail(
    provider: Provider,
    stdout: &str,
    stderr: &str,
    command: &str,
) -> String {
    let stderr_tail = tail_chars(stderr, 500);
    let stdout_tail = tail_chars(stdout, 500);
    let parsed_error = parse_cli_response(provider, stdout)
        .map(|response| response.text.trim().to_string())
        .filter(|text| !text.is_empty());
    match (stderr_tail.is_empty(), stdout_tail.is_empty(), parsed_error) {
        (false, false, Some(parsed_error)) => {
            format!("{parsed_error}\n{stderr_tail}\nstdout: {stdout_tail}")
        }
        (false, true, Some(parsed_error)) => format!("{parsed_error}\n{stderr_tail}"),
        (true, false, Some(parsed_error)) => format!("{parsed_error}\nstdout: {stdout_tail}"),
        (true, true, Some(parsed_error)) => parsed_error,
        (false, false, None) => format!("{stderr_tail}\nstdout: {stdout_tail}"),
        (false, true, None) => stderr_tail,
        (true, false, None) => format!("stdout: {stdout_tail}"),
        (true, true, None) => format!("command: {command}"),
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

async fn create_cli_scratch_dir(sandbox: &Arc<dyn Sandbox>) -> Result<String, FabroError> {
    let command =
        "umask 077 && base=\"$HOME/.fabro/cli\" && mkdir -p \"$base\" && mktemp -d \"$base/run.XXXXXX\"";
    let result = sandbox
        .exec_command(command, 30_000, None, None, None)
        .await
        .map_err(|e| FabroError::handler(format!("Failed to create CLI scratch dir: {e}")))?;
    if result.exit_code != 0 {
        return Err(FabroError::handler(format!(
            "failed to create CLI scratch dir: {}",
            result.stderr
        )));
    }

    let scratch_dir = result.stdout.trim();
    if scratch_dir.is_empty() {
        return Err(FabroError::handler(
            "CLI scratch dir command returned an empty path".to_string(),
        ));
    }

    Ok(scratch_dir.to_string())
}

fn codex_rotator_config_paths() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let home = PathBuf::from(home);
    vec![
        home.join(".config/autonomy/codex-rotator.json"),
        home.join(".config/rsociety/codex-rotator.json"),
    ]
}

fn load_codex_rotator() -> Option<CodexRotator> {
    for path in codex_rotator_config_paths() {
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(config) = serde_json::from_str::<CodexRotatorConfig>(&raw) else {
            continue;
        };
        if !config.enabled || config.slots.is_empty() {
            continue;
        }
        let state_path = config
            .state_path
            .clone()
            .or(config.shared_state_path.clone())?;
        let state = std::fs::read_to_string(&state_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<CodexRotatorState>(&raw).ok())
            .unwrap_or_default();
        return Some(CodexRotator {
            config_path: path,
            state_path,
            config,
            state,
        });
    }
    None
}

fn slot_key(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn automation_codex_home_allowed_with_home(path: &Path, home: Option<&Path>) -> bool {
    let Some(home) = home else {
        return true;
    };
    let primary_home = home.join(".codex");
    path != primary_home
}

fn automation_codex_home_allowed(path: &Path) -> bool {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    automation_codex_home_allowed_with_home(path, home.as_deref())
}

fn choose_codex_slot(rotator: &CodexRotator) -> Option<CodexSlotSelection> {
    let now = now_ms();
    let cooldown_seconds = rotator.config.cooldown_seconds.unwrap_or(900);
    let selection_policy = rotator
        .config
        .selection_policy
        .as_deref()
        .unwrap_or("sticky");
    let sticky_reuse_min_ms = 60_000u64;

    let mut healthy_slots = rotator
        .config
        .slots
        .iter()
        .filter(|slot| automation_codex_home_allowed(&slot.codex_home))
        .filter(|slot| slot.codex_home.join("auth.json").exists())
        .collect::<Vec<_>>();
    if healthy_slots.is_empty() {
        return None;
    }

    if selection_policy == "sticky" {
        if let Some(selected) = rotator.state.selected_slot.as_ref() {
            let selected_key = slot_key(selected);
            if let Some(slot) = healthy_slots
                .iter()
                .find(|slot| slot_key(&slot.name) == selected_key)
            {
                let state = rotator
                    .state
                    .slots
                    .iter()
                    .find(|(name, _)| slot_key(name) == selected_key)
                    .map(|(_, state)| state)
                    .cloned()
                    .unwrap_or_default();
                if state.cooldown_until_ms <= now
                    && now.saturating_sub(state.last_selected_at_ms) >= sticky_reuse_min_ms
                {
                    return Some(CodexSlotSelection {
                        slot_name: slot.name.clone(),
                        codex_home: slot.codex_home.clone(),
                        config_path: rotator.config_path.clone(),
                        state_path: rotator.state_path.clone(),
                        cooldown_seconds,
                    });
                }
            }
        }
    }

    healthy_slots.sort_by_key(|slot| {
        rotator
            .state
            .slots
            .iter()
            .find(|(name, _)| slot_key(name) == slot_key(&slot.name))
            .map(|(_, state)| {
                if state.cooldown_until_ms > now {
                    u64::MAX
                } else {
                    state.last_selected_at_ms
                }
            })
            .unwrap_or(0)
    });

    let slot = healthy_slots.into_iter().find(|slot| {
        rotator
            .state
            .slots
            .iter()
            .find(|(name, _)| slot_key(name) == slot_key(&slot.name))
            .map(|(_, state)| state.cooldown_until_ms <= now)
            .unwrap_or(true)
    })?;

    Some(CodexSlotSelection {
        slot_name: slot.name.clone(),
        codex_home: slot.codex_home.clone(),
        config_path: rotator.config_path.clone(),
        state_path: rotator.state_path.clone(),
        cooldown_seconds,
    })
}

fn fallback_codex_slot_selection_with_home(home: &Path) -> Option<CodexSlotSelection> {
    let candidates = [
        ("slot1", home.join(".codex-slot1/.codex")),
        ("slot2", home.join(".codex-slot2/.codex")),
        ("slot3", home.join(".codex-slot3/.codex")),
        ("slot4", home.join(".codex-slot4/.codex")),
        ("slot5", home.join(".codex-slot5/.codex")),
    ];

    candidates
        .into_iter()
        .find(|(_, codex_home)| codex_home.join("auth.json").exists())
        .map(|(slot_name, codex_home)| CodexSlotSelection {
            slot_name: slot_name.to_string(),
            codex_home,
            config_path: PathBuf::from("<fallback>"),
            state_path: PathBuf::from("<fallback>"),
            cooldown_seconds: 900,
        })
}

fn fallback_codex_slot_selection() -> Option<CodexSlotSelection> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    fallback_codex_slot_selection_with_home(&home)
}

pub fn select_automation_codex_home() -> Option<PathBuf> {
    let selection = load_codex_rotator()
        .and_then(|rotator| choose_codex_slot(&rotator))
        .or_else(fallback_codex_slot_selection)?;
    mark_codex_slot_selected(&selection);
    Some(selection.codex_home)
}

fn save_codex_rotator_state(path: &Path, state: &CodexRotatorState) {
    let Ok(json) = serde_json::to_string_pretty(state) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, json);
}

fn mark_codex_slot_selected(selection: &CodexSlotSelection) {
    if selection.config_path == Path::new("<fallback>") {
        return;
    }
    let mut state = std::fs::read_to_string(&selection.state_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<CodexRotatorState>(&raw).ok())
        .unwrap_or_default();
    let now = now_ms();
    state.selected_slot = Some(selection.slot_name.clone());
    let entry = state.slots.entry(selection.slot_name.clone()).or_default();
    entry.last_selected_at_ms = now;
    save_codex_rotator_state(&selection.state_path, &state);
}

fn classify_codex_slot_failure(stderr: &str, stdout: &str) -> Option<String> {
    let combined = format!("{stderr}\n{stdout}").to_ascii_lowercase();
    let patterns = [
        "429",
        "rate limit",
        "quota",
        "limit_reached",
        "usage limit",
        "try again at",
        "insufficient permissions",
        "api.responses.write",
        "401 unauthorized",
    ];
    patterns
        .iter()
        .find(|pattern| combined.contains(**pattern))
        .map(|pattern| format!("output matched `{pattern}`"))
}

fn should_rotate_away_from_failed_slot(reason: &str) -> bool {
    let reason = reason.to_ascii_lowercase();
    reason.contains("429")
        || reason.contains("rate limit")
        || reason.contains("quota")
        || reason.contains("limit_reached")
        || reason.contains("insufficient permissions")
        || reason.contains("api.responses.write")
        || reason.contains("401 unauthorized")
}

fn mark_codex_slot_failed(selection: &CodexSlotSelection, reason: &str) {
    if selection.config_path == Path::new("<fallback>") {
        return;
    }
    let mut state = std::fs::read_to_string(&selection.state_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<CodexRotatorState>(&raw).ok())
        .unwrap_or_default();
    let now = now_ms();
    let entry = state.slots.entry(selection.slot_name.clone()).or_default();
    entry.last_failure_reason = Some(reason.to_string());
    entry.last_failure_at_ms = Some(now);
    entry.cooldown_until_ms = now.saturating_add(selection.cooldown_seconds.saturating_mul(1000));
    if should_rotate_away_from_failed_slot(reason)
        && state.selected_slot.as_deref() == Some(selection.slot_name.as_str())
    {
        state.selected_slot = None;
    }
    save_codex_rotator_state(&selection.state_path, &state);
}

fn mark_codex_slot_succeeded(selection: &CodexSlotSelection) {
    if selection.config_path == Path::new("<fallback>") {
        return;
    }
    let mut state = std::fs::read_to_string(&selection.state_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<CodexRotatorState>(&raw).ok())
        .unwrap_or_default();
    let entry = state.slots.entry(selection.slot_name.clone()).or_default();
    entry.cooldown_until_ms = 0;
    entry.last_failure_reason = None;
    entry.last_failure_at_ms = None;
    save_codex_rotator_state(&selection.state_path, &state);
}

/// CLI backend that invokes external CLI tools (claude, codex, gemini) via `exec_command()`.
pub struct AgentCliBackend {
    model: String,
    provider: Provider,
    env: HashMap<String, String>,
    poll_interval: std::time::Duration,
}

impl AgentCliBackend {
    #[must_use]
    pub fn new(model: String, provider: Provider) -> Self {
        Self {
            model,
            provider,
            env: HashMap::new(),
            poll_interval: std::time::Duration::from_secs(5),
        }
    }

    #[must_use]
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    #[must_use]
    pub fn with_poll_interval(mut self, interval: std::time::Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Detect changed files by comparing git state before and after the CLI run.
    async fn detect_changed_files(&self, sandbox: &Arc<dyn Sandbox>) -> Vec<String> {
        // Get unstaged changes
        let diff_result = sandbox
            .exec_command("git diff --name-only", 30_000, None, None, None)
            .await;

        // Get untracked files
        let untracked_result = sandbox
            .exec_command(
                "git ls-files --others --exclude-standard",
                30_000,
                None,
                None,
                None,
            )
            .await;

        let mut files: Vec<String> = Vec::new();

        if let Ok(result) = diff_result {
            if result.exit_code == 0 {
                files.extend(
                    result
                        .stdout
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .map(String::from),
                );
            }
        }

        if let Ok(result) = untracked_result {
            if result.exit_code == 0 {
                files.extend(
                    result
                        .stdout
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .map(String::from),
                );
            }
        }

        files.sort();
        files.dedup();
        files
    }
}

#[async_trait]
impl CodergenBackend for AgentCliBackend {
    async fn run(
        &self,
        node: &Node,
        prompt: &str,
        _context: &Context,
        _thread_id: Option<&str>,
        emitter: &Arc<EventEmitter>,
        stage_dir: &Path,
        sandbox: &Arc<dyn Sandbox>,
        _tool_hooks: Option<Arc<dyn fabro_agent::ToolHookCallback>>,
    ) -> Result<CodergenResult, FabroError> {
        // 1. Snapshot git state before the CLI run
        let files_before = self.detect_changed_files(sandbox).await;

        // 2. Generate unique paths for this run
        let scratch_dir = create_cli_scratch_dir(sandbox).await?;
        let prompt_path = format!("{scratch_dir}/prompt.txt");
        let stdout_path = format!("{scratch_dir}/stdout.log");
        let stderr_path = format!("{scratch_dir}/stderr.log");
        let exit_code_path = format!("{scratch_dir}/exit_code");

        sandbox
            .write_file(&prompt_path, prompt)
            .await
            .map_err(|e| FabroError::handler(format!("Failed to write prompt file: {e}")))?;

        // 3. Build CLI command
        let requested_model = node.model().unwrap_or(&self.model).to_string();
        let requested_provider = node
            .provider()
            .and_then(|s| s.parse::<Provider>().ok())
            .unwrap_or(self.provider);

        let _ = tokio::fs::create_dir_all(stage_dir).await;
        // 3a. Disable auto-stop so the sandbox stays alive during long CLI runs
        if let Err(e) = sandbox.set_autostop_interval(0).await {
            tracing::warn!("Failed to disable sandbox auto-stop: {e}");
        }
        let attempt_targets =
            build_cli_attempt_targets(requested_provider, &requested_model, &self.model);
        let stdout_path_quoted = shell_quote(&stdout_path);
        let stderr_path_quoted = shell_quote(&stderr_path);
        let exit_code_path_quoted = shell_quote(&exit_code_path);
        let poll_command = format!(
            "[ -f {exit_code_path_quoted} ] && cat {exit_code_path_quoted} || echo running"
        );
        let poll_interval = self.poll_interval;
        let mut attempt_failures = Vec::new();
        let mut chosen_model = requested_model.clone();
        let mut parsed = None;

        for (index, attempt) in attempt_targets.iter().enumerate() {
            let provider = attempt.provider;
            let model = attempt.model.clone();
            let fallback_reason = attempt.fallback_reason.clone();
            let mut launch_env = build_launch_env_for_provider(provider, &self.env);
            let codex_slot = if provider == Provider::OpenAi {
                load_codex_rotator()
                    .and_then(|rotator| choose_codex_slot(&rotator))
                    .or_else(fallback_codex_slot_selection)
            } else {
                None
            };

            if provider == Provider::OpenAi && codex_slot.is_none() {
                let detail = "no dedicated OAuth-backed Codex slot is available";
                if index + 1 < attempt_targets.len() {
                    attempt_failures.push(format!(
                        "{}:{} failed: {detail}",
                        provider.as_str(),
                        model
                    ));
                    continue;
                }
                return Err(FabroError::handler(format!(
                    "requested provider=openai but {detail}; Fabro will not use API-key auth or the shared ~/.codex home"
                )));
            }

            if let Some(selection) = codex_slot.as_ref() {
                launch_env.insert(
                    "CODEX_HOME".to_string(),
                    selection.codex_home.display().to_string(),
                );
                mark_codex_slot_selected(selection);
            }

            let cli = AgentCli::for_provider(provider);
            ensure_cli(cli, provider, sandbox, emitter).await?;
            let reasoning = node.reasoning_effort();
            let readonly = is_readonly_stage(&node.id);
            let sys_prompt = stage_system_prompt(&node.id);
            let command = cli_command_for_provider(
                provider,
                &model,
                &shell_quote(&prompt_path),
                Some(reasoning),
                readonly,
                sys_prompt,
            );

            write_provider_used_json(
                stage_dir,
                ProviderUsedJson {
                    requested_provider,
                    requested_model: &requested_model,
                    provider,
                    model: &model,
                    served_model: None,
                    fallback_reason: fallback_reason.as_deref(),
                    codex_slot: codex_slot.as_ref(),
                    command: &command,
                },
            )
            .await;
            if let Some(reason) = &fallback_reason {
                tracing::warn!(
                    requested_provider = requested_provider.as_str(),
                    actual_provider = provider.as_str(),
                    requested_model = requested_model,
                    actual_model = model,
                    "{reason}"
                );
            }

            let inner_command = format!("export PATH=\"$HOME/.local/bin:$PATH\" && {command} > {stdout_path_quoted} 2>{stderr_path_quoted}; echo $? > {exit_code_path_quoted}");
            let script_path = format!("{exit_code_path}.sh");
            let bg_command = format!(
                "cat > {script_path} << 'FABRO_SCRIPT_EOF'\n{inner_command}\nFABRO_SCRIPT_EOF\nSID=$(command -v setsid || true)\n$SID sh {script_path} </dev/null >/dev/null 2>&1 &\necho $!"
            );
            let launch_start = std::time::Instant::now();
            let launch_env_ref = if launch_env.is_empty() {
                None
            } else {
                Some(&launch_env)
            };
            let launch_result = sandbox
                .exec_command(&bg_command, 30_000, None, launch_env_ref, None)
                .await
                .map_err(|e| FabroError::handler(format!("Failed to launch CLI command: {e}")))?;
            let pid = launch_result.stdout.trim();
            tracing::info!(
                pid,
                provider = provider.as_str(),
                model,
                "CLI process launched in background"
            );

            let exit_code: i32 = loop {
                tokio::time::sleep(poll_interval).await;
                emitter.touch();
                let poll_result = sandbox
                    .exec_command(&poll_command, 30_000, None, None, None)
                    .await
                    .map_err(|e| FabroError::handler(format!("Failed to poll CLI command: {e}")))?;
                let status = poll_result.stdout.trim();
                for (remote, local) in [
                    (&stdout_path, "cli_stdout.log"),
                    (&stderr_path, "cli_stderr.log"),
                ] {
                    if let Ok(result) = sandbox
                        .exec_command(
                            &format!("cat {}", shell_quote(remote)),
                            30_000,
                            None,
                            None,
                            None,
                        )
                        .await
                    {
                        if !result.stdout.is_empty() {
                            let _ = tokio::fs::write(stage_dir.join(local), &result.stdout).await;
                        }
                    }
                }
                if status != "running" {
                    break status.parse::<i32>().unwrap_or(-1);
                }
            };

            let duration_ms = u64::try_from(launch_start.elapsed().as_millis()).unwrap_or(u64::MAX);
            let stdout_result = sandbox
                .exec_command(
                    &format!("cat {}", shell_quote(&stdout_path)),
                    60_000,
                    None,
                    None,
                    None,
                )
                .await
                .map_err(|e| FabroError::handler(format!("Failed to read stdout: {e}")))?;
            let stderr_result = sandbox
                .exec_command(
                    &format!("cat {}", shell_quote(&stderr_path)),
                    60_000,
                    None,
                    None,
                    None,
                )
                .await
                .map_err(|e| FabroError::handler(format!("Failed to read stderr: {e}")))?;
            let result = ExecResult {
                stdout: stdout_result.stdout,
                stderr: stderr_result.stdout,
                exit_code,
                timed_out: false,
                duration_ms,
            };

            if let Some(selection) = codex_slot.as_ref() {
                if result.exit_code == 0 {
                    mark_codex_slot_succeeded(selection);
                } else if let Some(reason) =
                    classify_codex_slot_failure(&result.stderr, &result.stdout)
                {
                    mark_codex_slot_failed(selection, &reason);
                }
            }

            if let Ok(json) = serde_json::to_string_pretty(&serde_json::json!({
                "exit_code": result.exit_code,
                "stdout_len": result.stdout.len(),
                "stderr_len": result.stderr.len(),
                "duration_ms": result.duration_ms,
            })) {
                let _ = tokio::fs::write(stage_dir.join("cli_result_meta.json"), json).await;
            }

            if result.exit_code != 0 {
                let _ = tokio::fs::write(stage_dir.join("cli_stdout.log"), &result.stdout).await;
                let _ = tokio::fs::write(stage_dir.join("cli_stderr.log"), &result.stderr).await;
                let detail =
                    format_cli_failure_detail(provider, &result.stdout, &result.stderr, &command);
                let can_fallback = index + 1 < attempt_targets.len()
                    && cli_failure_is_retryable_for_fallback(&detail);
                if can_fallback {
                    let next = &attempt_targets[index + 1];
                    tracing::info!(
                        from_provider = provider.as_str(),
                        from_model = model.as_str(),
                        to_provider = next.provider.as_str(),
                        to_model = next.model.as_str(),
                        reason = detail.as_str(),
                        "provider fallback activated"
                    );
                    attempt_failures.push(format!(
                        "{}:{} failed: {}",
                        provider.as_str(),
                        model,
                        detail
                    ));
                    continue;
                }
                let _ = sandbox
                    .exec_command(
                        &format!("rm -rf {}", shell_quote(&scratch_dir)),
                        30_000,
                        None,
                        None,
                        None,
                    )
                    .await;
                if !attempt_failures.is_empty() {
                    return Err(FabroError::handler(format!(
                        "CLI command exited with code {}: {}\nPrevious attempts:\n{}",
                        result.exit_code,
                        detail,
                        attempt_failures.join("\n")
                    )));
                }
                return Err(FabroError::handler(format!(
                    "CLI command exited with code {}: {}",
                    result.exit_code, detail,
                )));
            }

            let parsed_response = parse_cli_response(provider, &result.stdout)
                .ok_or_else(|| FabroError::handler("Failed to parse CLI output".to_string()))?;
            // Detect silent provider failures: pi returns exit 0 with an
            // empty assistant response on auth errors (401).  Treat as a
            // retryable provider failure rather than a successful empty run.
            if parsed_response.text.trim().is_empty()
                && parsed_response.input_tokens == 0
                && parsed_response.output_tokens == 0
            {
                let detail =
                    "provider returned empty response with zero tokens (possible auth failure)";
                let can_fallback = index + 1 < attempt_targets.len()
                    && cli_failure_is_retryable_for_fallback(detail);
                if can_fallback {
                    let next = &attempt_targets[index + 1];
                    tracing::info!(
                        from_provider = provider.as_str(),
                        from_model = model.as_str(),
                        to_provider = next.provider.as_str(),
                        to_model = next.model.as_str(),
                        reason = detail,
                        "provider fallback activated (silent empty response)"
                    );
                    attempt_failures.push(format!(
                        "{}:{} failed: {detail}",
                        provider.as_str(),
                        model
                    ));
                    continue;
                }
                return Err(FabroError::handler(format!(
                    "CLI command produced empty response: {detail}"
                )));
            }
            write_provider_used_json(
                stage_dir,
                ProviderUsedJson {
                    requested_provider,
                    requested_model: &requested_model,
                    provider,
                    model: &model,
                    served_model: parsed_response.served_model.as_deref(),
                    fallback_reason: fallback_reason.as_deref(),
                    codex_slot: codex_slot.as_ref(),
                    command: &command,
                },
            )
            .await;
            if index > 0 {
                tracing::info!(
                    provider = provider.as_str(),
                    model = model.as_str(),
                    attempt = index + 1,
                    "provider fallback succeeded"
                );
            }
            chosen_model = model;
            parsed = Some(parsed_response);
            break;
        }

        let _ = sandbox
            .exec_command(
                &format!("rm -rf {}", shell_quote(&scratch_dir)),
                30_000,
                None,
                None,
                None,
            )
            .await;

        let Some(parsed) = parsed else {
            return Err(FabroError::handler(format!(
                "all CLI attempts failed:\n{}",
                attempt_failures.join("\n")
            )));
        };

        // 5. Detect changed files
        let files_after = self.detect_changed_files(sandbox).await;
        let files_touched: Vec<String> = files_after
            .into_iter()
            .filter(|f| !files_before.contains(f))
            .collect();

        // Find the most recently modified file by mtime
        let last_file_touched = if !files_touched.is_empty() {
            let quoted_files: Vec<String> = files_touched
                .iter()
                .filter_map(|f| shlex::try_quote(f).ok().map(|q| q.into_owned()))
                .collect();
            let cmd = format!("ls -t {} | head -1", quoted_files.join(" "));
            if let Ok(result) = sandbox.exec_command(&cmd, 5_000, None, None, None).await {
                let trimmed = result.stdout.trim().to_string();
                if result.exit_code == 0 && !trimmed.is_empty() {
                    Some(trimmed)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let mut stage_usage = StageUsage {
            model: parsed
                .served_model
                .clone()
                .unwrap_or_else(|| chosen_model.clone()),
            input_tokens: parsed.input_tokens,
            output_tokens: parsed.output_tokens,
            cache_read_tokens: None,
            cache_write_tokens: None,
            reasoning_tokens: None,
            speed: None,
            cost: None,
        };
        stage_usage.cost = compute_stage_cost(&stage_usage);

        Ok(CodergenResult::Text {
            text: parsed.text,
            usage: Some(stage_usage),
            files_read: Vec::new(),
            files_written: files_touched.clone(),
            files_touched,
            last_file_touched,
        })
    }
}

/// Routes codergen invocations to either the API backend or CLI backend
/// based on node attributes and model type.
pub struct BackendRouter {
    api_backend: Box<dyn CodergenBackend>,
    cli_backend: AgentCliBackend,
}

impl BackendRouter {
    #[must_use]
    pub fn new(api_backend: Box<dyn CodergenBackend>, cli_backend: AgentCliBackend) -> Self {
        Self {
            api_backend,
            cli_backend,
        }
    }

    fn should_use_cli(&self, node: &Node) -> bool {
        // Explicit backend="cli" attribute on the node
        if node.backend() == Some("cli") {
            return true;
        }

        // CLI-only model on the node
        if let Some(model) = node.model() {
            if is_cli_only_model(model) {
                return true;
            }
        }

        false
    }
}

#[async_trait]
impl CodergenBackend for BackendRouter {
    async fn run(
        &self,
        node: &Node,
        prompt: &str,
        context: &Context,
        thread_id: Option<&str>,
        emitter: &Arc<EventEmitter>,
        stage_dir: &Path,
        sandbox: &Arc<dyn Sandbox>,
        tool_hooks: Option<Arc<dyn fabro_agent::ToolHookCallback>>,
    ) -> Result<CodergenResult, FabroError> {
        if self.should_use_cli(node) {
            self.cli_backend
                .run(
                    node, prompt, context, thread_id, emitter, stage_dir, sandbox, tool_hooks,
                )
                .await
        } else {
            self.api_backend
                .run(
                    node, prompt, context, thread_id, emitter, stage_dir, sandbox, tool_hooks,
                )
                .await
        }
    }

    async fn one_shot(
        &self,
        node: &Node,
        prompt: &str,
        system_prompt: Option<&str>,
        stage_dir: &Path,
    ) -> Result<CodergenResult, FabroError> {
        // CLI backend doesn't support one_shot, always route to API
        self.api_backend
            .one_shot(node, prompt, system_prompt, stage_dir)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fabro_graphviz::graph::AttrValue;
    use std::sync::Mutex;

    // -- AgentCli --

    #[test]
    fn agent_cli_for_provider() {
        assert_eq!(
            AgentCli::for_provider(Provider::Anthropic),
            AgentCli::Claude
        );
        assert_eq!(AgentCli::for_provider(Provider::OpenAi), AgentCli::Codex);
        assert_eq!(AgentCli::for_provider(Provider::Gemini), AgentCli::Gemini);
        assert_eq!(AgentCli::for_provider(Provider::Kimi), AgentCli::Pi);
        assert_eq!(AgentCli::for_provider(Provider::Zai), AgentCli::Codex);
        assert_eq!(AgentCli::for_provider(Provider::Minimax), AgentCli::Pi);
        assert_eq!(AgentCli::for_provider(Provider::Inception), AgentCli::Codex);
    }

    #[test]
    fn agent_cli_name() {
        assert_eq!(AgentCli::Claude.name(), "claude");
        assert_eq!(AgentCli::Codex.name(), "codex");
        assert_eq!(AgentCli::Gemini.name(), "gemini");
        assert_eq!(AgentCli::Pi.name(), "pi");
    }

    #[test]
    fn host_provider_credentials_are_not_inherited_for_subscription_capable_clis() {
        assert!(!inherit_host_provider_credentials(AgentCli::Claude));
        assert!(!inherit_host_provider_credentials(AgentCli::Codex));
        assert!(inherit_host_provider_credentials(AgentCli::Gemini));
        assert!(inherit_host_provider_credentials(AgentCli::Pi));
    }

    #[test]
    fn agent_cli_npm_package() {
        assert_eq!(AgentCli::Claude.npm_package(), "@anthropic-ai/claude-code");
        assert_eq!(AgentCli::Codex.npm_package(), "@openai/codex");
        assert_eq!(AgentCli::Gemini.npm_package(), "@anthropic-ai/gemini-cli");
        assert_eq!(AgentCli::Pi.npm_package(), "@mariozechner/pi-coding-agent");
    }

    #[test]
    fn resolve_cli_target_prefers_anthropic_when_openai_key_missing() {
        let env = HashMap::from([
            ("ANTHROPIC_API_KEY".to_string(), "test-key".to_string()),
            (
                "ANTHROPIC_MODEL".to_string(),
                "MiniMax-M2.7-highspeed".to_string(),
            ),
        ]);

        let (provider, model, reason) = resolve_cli_target(
            Provider::OpenAi,
            "gpt-5.4",
            "MiniMax-M2.7-highspeed",
            &env,
            false,
        );

        assert_eq!(provider, Provider::Anthropic);
        assert_eq!(model, "MiniMax-M2.7-highspeed");
        assert!(reason.is_some());
    }

    #[test]
    fn resolve_cli_target_keeps_openai_when_key_present() {
        let env = HashMap::from([("OPENAI_API_KEY".to_string(), "test-openai-key".to_string())]);

        let (provider, model, reason) = resolve_cli_target(
            Provider::OpenAi,
            "gpt-5.4",
            "MiniMax-M2.7-highspeed",
            &env,
            false,
        );

        assert_eq!(provider, Provider::OpenAi);
        assert_eq!(model, "gpt-5.4");
        assert!(reason.is_none());
    }

    #[test]
    fn resolve_cli_target_keeps_openai_when_strict_provider_requested() {
        let env = HashMap::from([
            ("FABRO_STRICT_PROVIDER".to_string(), "1".to_string()),
            (
                "ANTHROPIC_API_KEY".to_string(),
                "test-anthropic-key".to_string(),
            ),
        ]);

        let (provider, model, reason) = resolve_cli_target(
            Provider::OpenAi,
            "gpt-5.4",
            "MiniMax-M2.7-highspeed",
            &env,
            false,
        );

        assert_eq!(provider, Provider::OpenAi);
        assert_eq!(model, "gpt-5.4");
        assert!(reason.is_none());
    }

    #[test]
    fn resolve_cli_target_keeps_openai_when_codex_slots_exist() {
        let env = HashMap::from([(
            "ANTHROPIC_API_KEY".to_string(),
            "test-anthropic-key".to_string(),
        )]);

        let (provider, model, reason) = resolve_cli_target(
            Provider::OpenAi,
            "gpt-5.4",
            "MiniMax-M2.7-highspeed",
            &env,
            true,
        );

        assert_eq!(provider, Provider::OpenAi);
        assert_eq!(model, "gpt-5.4");
        assert!(reason.is_none());
    }

    #[test]
    fn choose_codex_slot_prefers_sticky_selected_slot_when_healthy() {
        let temp = tempfile::tempdir().expect("tempdir");
        let home1 = temp.path().join("slot1");
        let home2 = temp.path().join("slot2");
        std::fs::create_dir_all(&home1).expect("slot1 dir");
        std::fs::create_dir_all(&home2).expect("slot2 dir");
        std::fs::write(home1.join("auth.json"), "{}").expect("slot1 auth");
        std::fs::write(home2.join("auth.json"), "{}").expect("slot2 auth");

        let rotator = CodexRotator {
            config_path: temp.path().join("config.json"),
            state_path: temp.path().join("state.json"),
            config: CodexRotatorConfig {
                enabled: true,
                selection_policy: Some("sticky".to_string()),
                cooldown_seconds: Some(900),
                state_path: Some(temp.path().join("state.json")),
                shared_state_path: None,
                slots: vec![
                    CodexRotatorSlot {
                        name: "slot1".to_string(),
                        codex_home: home1,
                    },
                    CodexRotatorSlot {
                        name: "slot2".to_string(),
                        codex_home: home2,
                    },
                ],
            },
            state: CodexRotatorState {
                selected_slot: Some("slot2".to_string()),
                slots: HashMap::new(),
            },
        };

        let selected = choose_codex_slot(&rotator).expect("slot selected");
        assert_eq!(selected.slot_name, "slot2");
    }

    #[test]
    fn choose_codex_slot_skips_slots_on_cooldown() {
        let temp = tempfile::tempdir().expect("tempdir");
        let home1 = temp.path().join("slot1");
        let home2 = temp.path().join("slot2");
        std::fs::create_dir_all(&home1).expect("slot1 dir");
        std::fs::create_dir_all(&home2).expect("slot2 dir");
        std::fs::write(home1.join("auth.json"), "{}").expect("slot1 auth");
        std::fs::write(home2.join("auth.json"), "{}").expect("slot2 auth");

        let mut slots = HashMap::new();
        slots.insert(
            "slot1".to_string(),
            CodexRotatorSlotState {
                last_selected_at_ms: 1,
                cooldown_until_ms: now_ms().saturating_add(60_000),
                last_failure_reason: Some("quota".to_string()),
                last_failure_at_ms: Some(now_ms()),
            },
        );

        let rotator = CodexRotator {
            config_path: temp.path().join("config.json"),
            state_path: temp.path().join("state.json"),
            config: CodexRotatorConfig {
                enabled: true,
                selection_policy: Some("sticky".to_string()),
                cooldown_seconds: Some(900),
                state_path: Some(temp.path().join("state.json")),
                shared_state_path: None,
                slots: vec![
                    CodexRotatorSlot {
                        name: "slot1".to_string(),
                        codex_home: home1,
                    },
                    CodexRotatorSlot {
                        name: "slot2".to_string(),
                        codex_home: home2,
                    },
                ],
            },
            state: CodexRotatorState {
                selected_slot: Some("slot1".to_string()),
                slots,
            },
        };

        let selected = choose_codex_slot(&rotator).expect("slot selected");
        assert_eq!(selected.slot_name, "slot2");
    }

    #[test]
    fn choose_codex_slot_rotates_away_from_recently_selected_sticky_slot() {
        let temp = tempfile::tempdir().expect("tempdir");
        let home1 = temp.path().join("slot1");
        let home2 = temp.path().join("slot2");
        std::fs::create_dir_all(&home1).expect("slot1 dir");
        std::fs::create_dir_all(&home2).expect("slot2 dir");
        std::fs::write(home1.join("auth.json"), "{}").expect("slot1 auth");
        std::fs::write(home2.join("auth.json"), "{}").expect("slot2 auth");

        let now = now_ms();
        let mut slots = HashMap::new();
        slots.insert(
            "slot1".to_string(),
            CodexRotatorSlotState {
                last_selected_at_ms: now,
                cooldown_until_ms: 0,
                last_failure_reason: None,
                last_failure_at_ms: None,
            },
        );

        let rotator = CodexRotator {
            config_path: temp.path().join("config.json"),
            state_path: temp.path().join("state.json"),
            config: CodexRotatorConfig {
                enabled: true,
                selection_policy: Some("sticky".to_string()),
                cooldown_seconds: Some(900),
                state_path: Some(temp.path().join("state.json")),
                shared_state_path: None,
                slots: vec![
                    CodexRotatorSlot {
                        name: "slot1".to_string(),
                        codex_home: home1,
                    },
                    CodexRotatorSlot {
                        name: "slot2".to_string(),
                        codex_home: home2,
                    },
                ],
            },
            state: CodexRotatorState {
                selected_slot: Some("slot1".to_string()),
                slots,
            },
        };

        let selected = choose_codex_slot(&rotator).expect("slot selected");
        assert_eq!(selected.slot_name, "slot2");
    }

    #[test]
    fn choose_codex_slot_skips_primary_shared_home() {
        let temp = tempfile::tempdir().expect("tempdir");
        let home = dirs::home_dir().expect("home dir");
        let primary = home.join(".codex");
        let slot1 = temp.path().join(".codex-slot1/.codex");
        std::fs::create_dir_all(&slot1).expect("slot1 dir");
        std::fs::write(slot1.join("auth.json"), "{}").expect("slot1 auth");

        let rotator = CodexRotator {
            config_path: temp.path().join("config.json"),
            state_path: temp.path().join("state.json"),
            config: CodexRotatorConfig {
                enabled: true,
                selection_policy: Some("sticky".to_string()),
                cooldown_seconds: Some(900),
                state_path: Some(temp.path().join("state.json")),
                shared_state_path: None,
                slots: vec![
                    CodexRotatorSlot {
                        name: "primary".to_string(),
                        codex_home: primary.clone(),
                    },
                    CodexRotatorSlot {
                        name: "slot1".to_string(),
                        codex_home: slot1.clone(),
                    },
                ],
            },
            state: CodexRotatorState::default(),
        };

        let selected = choose_codex_slot(&rotator).expect("slot selected");
        assert_eq!(selected.slot_name, "slot1");
        assert_eq!(selected.codex_home, slot1);
    }

    #[test]
    fn classify_codex_slot_failure_detects_scope_error() {
        let reason = classify_codex_slot_failure(
            "unexpected status 401 Unauthorized: Missing scopes: api.responses.write",
            "",
        )
        .expect("retryable reason");

        assert!(reason.contains("api.responses.write"));
    }

    #[test]
    fn classify_codex_slot_failure_detects_usage_limit_error() {
        let reason = classify_codex_slot_failure(
            "",
            "You've hit your usage limit. Visit https://chatgpt.com/codex/settings/usage to purchase more credits or try again at 7:31 PM.",
        )
        .expect("retryable reason");

        assert!(reason.contains("usage limit"));
    }

    #[test]
    fn fallback_codex_slot_selection_picks_first_available_auth_home() {
        let temp = tempfile::tempdir().expect("tempdir");
        let slot3 = temp.path().join(".codex-slot3/.codex");
        std::fs::create_dir_all(&slot3).expect("slot3 dir");
        std::fs::write(slot3.join("auth.json"), "{}").expect("slot3 auth");

        let selected = fallback_codex_slot_selection_with_home(temp.path()).expect("fallback slot");
        assert_eq!(selected.slot_name, "slot3");
        assert_eq!(selected.codex_home, slot3);
    }

    #[test]
    fn mark_codex_slot_failed_clears_selected_slot_for_quota_failures() {
        let temp = tempfile::tempdir().expect("tempdir");
        let state_path = temp.path().join("state.json");
        let initial_state = CodexRotatorState {
            selected_slot: Some("slot1".to_string()),
            slots: HashMap::new(),
        };
        std::fs::write(
            &state_path,
            serde_json::to_string_pretty(&initial_state).expect("state json"),
        )
        .expect("write state");

        let selection = CodexSlotSelection {
            slot_name: "slot1".to_string(),
            codex_home: temp.path().join("slot1"),
            config_path: temp.path().join("config.json"),
            state_path: state_path.clone(),
            cooldown_seconds: 900,
        };

        mark_codex_slot_failed(&selection, "output matched `quota`");

        let state = std::fs::read_to_string(state_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<CodexRotatorState>(&raw).ok())
            .expect("updated state");
        assert_eq!(state.selected_slot, None);
        assert!(state
            .slots
            .get("slot1")
            .is_some_and(|slot| slot.cooldown_until_ms > 0));
    }

    // -- ensure_cli --

    use fabro_agent::sandbox::{DirEntry, GrepOptions};
    use std::collections::VecDeque;

    /// Mock sandbox that returns pre-configured ExecResults in FIFO order.
    struct CliMockSandbox {
        results: Mutex<VecDeque<ExecResult>>,
        commands: Mutex<Vec<String>>,
        writes: Mutex<Vec<(String, String)>>,
    }

    impl CliMockSandbox {
        fn new(results: Vec<ExecResult>) -> Self {
            Self {
                results: Mutex::new(results.into()),
                commands: Mutex::new(Vec::new()),
                writes: Mutex::new(Vec::new()),
            }
        }

        fn commands(&self) -> Vec<String> {
            self.commands.lock().unwrap().clone()
        }

        fn writes(&self) -> Vec<(String, String)> {
            self.writes.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl Sandbox for CliMockSandbox {
        async fn read_file(
            &self,
            _path: &str,
            _offset: Option<usize>,
            _limit: Option<usize>,
        ) -> Result<String, String> {
            Ok(String::new())
        }
        async fn write_file(&self, path: &str, content: &str) -> Result<(), String> {
            self.writes
                .lock()
                .unwrap()
                .push((path.to_string(), content.to_string()));
            Ok(())
        }
        async fn delete_file(&self, _path: &str) -> Result<(), String> {
            Ok(())
        }
        async fn file_exists(&self, _path: &str) -> Result<bool, String> {
            Ok(false)
        }
        async fn list_directory(
            &self,
            _path: &str,
            _depth: Option<usize>,
        ) -> Result<Vec<DirEntry>, String> {
            Ok(vec![])
        }
        async fn exec_command(
            &self,
            command: &str,
            _timeout_ms: u64,
            _working_dir: Option<&str>,
            _env_vars: Option<&std::collections::HashMap<String, String>>,
            _cancel_token: Option<tokio_util::sync::CancellationToken>,
        ) -> Result<ExecResult, String> {
            self.commands.lock().unwrap().push(command.to_string());
            self.results
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| "no more mock results".to_string())
        }
        async fn grep(
            &self,
            _pattern: &str,
            _path: &str,
            _options: &GrepOptions,
        ) -> Result<Vec<String>, String> {
            Ok(vec![])
        }
        async fn glob(&self, _pattern: &str, _path: Option<&str>) -> Result<Vec<String>, String> {
            Ok(vec![])
        }
        async fn download_file_to_local(&self, _remote: &str, _local: &Path) -> Result<(), String> {
            Ok(())
        }
        async fn upload_file_from_local(&self, _local: &Path, _remote: &str) -> Result<(), String> {
            Ok(())
        }
        async fn initialize(&self) -> Result<(), String> {
            Ok(())
        }
        async fn cleanup(&self) -> Result<(), String> {
            Ok(())
        }
        fn working_directory(&self) -> &str {
            "/workspace"
        }
        fn platform(&self) -> &str {
            "linux"
        }
        fn os_version(&self) -> String {
            "Ubuntu 22.04".to_string()
        }
        async fn set_autostop_interval(&self, _minutes: i32) -> Result<(), String> {
            Ok(())
        }
    }

    fn ok_result() -> ExecResult {
        ExecResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            timed_out: false,
            duration_ms: 10,
        }
    }

    fn fail_result(code: i32) -> ExecResult {
        ExecResult {
            exit_code: code,
            stdout: String::new(),
            stderr: "error".to_string(),
            timed_out: false,
            duration_ms: 10,
        }
    }

    #[tokio::test]
    async fn ensure_cli_skips_install_when_present() {
        let sandbox: Arc<dyn Sandbox> = Arc::new(CliMockSandbox::new(vec![ok_result()]));
        let emitter = Arc::new(EventEmitter::new());

        let result = ensure_cli(AgentCli::Claude, Provider::Anthropic, &sandbox, &emitter).await;
        assert!(result.is_ok());

        let mock = sandbox.as_ref() as *const dyn Sandbox as *const CliMockSandbox;
        let commands = unsafe { &*mock }.commands();
        assert_eq!(commands.len(), 1);
        assert!(commands[0].contains("claude --version"));
    }

    #[tokio::test]
    async fn ensure_cli_installs_when_missing() {
        // version check fails, combined install succeeds
        let sandbox: Arc<dyn Sandbox> = Arc::new(CliMockSandbox::new(vec![
            fail_result(127), // claude --version
            ok_result(),      // combined node + npm install
        ]));
        let emitter = Arc::new(EventEmitter::new());

        let result = ensure_cli(AgentCli::Claude, Provider::Anthropic, &sandbox, &emitter).await;
        assert!(result.is_ok());

        let mock = sandbox.as_ref() as *const dyn Sandbox as *const CliMockSandbox;
        let commands = unsafe { &*mock }.commands();
        assert_eq!(commands.len(), 2);
        assert!(commands[1].contains("npm install -g @anthropic-ai/claude-code"));
    }

    #[tokio::test]
    async fn ensure_cli_fails_on_install_failure() {
        let sandbox: Arc<dyn Sandbox> = Arc::new(CliMockSandbox::new(vec![
            fail_result(127), // claude --version
            fail_result(1),   // combined install fails
        ]));
        let emitter = Arc::new(EventEmitter::new());

        let result = ensure_cli(AgentCli::Claude, Provider::Anthropic, &sandbox, &emitter).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("install exited with code"));
    }

    #[tokio::test]
    async fn run_uses_home_scratch_dir_and_never_writes_env_file() {
        let stdout =
            r#"{"type":"result","result":"done","usage":{"input_tokens":1,"output_tokens":1}}"#;
        let sandbox = Arc::new(CliMockSandbox::new(vec![
            ExecResult {
                stdout: String::new(),
                ..ok_result()
            }, // git diff before
            ExecResult {
                stdout: String::new(),
                ..ok_result()
            }, // git ls-files before
            ExecResult {
                stdout: "/home/test/.fabro/cli/run.abcd12\n".to_string(),
                ..ok_result()
            }, // scratch dir
            ok_result(), // claude --version
            ExecResult {
                stdout: "12345\n".to_string(),
                ..ok_result()
            }, // background launch
            ExecResult {
                stdout: "0\n".to_string(),
                ..ok_result()
            }, // poll
            ExecResult {
                stdout: stdout.to_string(),
                ..ok_result()
            }, // sync stdout while polling
            ExecResult {
                stdout: String::new(),
                ..ok_result()
            }, // sync stderr while polling
            ExecResult {
                stdout: stdout.to_string(),
                ..ok_result()
            }, // read stdout
            ExecResult {
                stdout: String::new(),
                ..ok_result()
            }, // read stderr
            ok_result(), // cleanup
            ExecResult {
                stdout: String::new(),
                ..ok_result()
            }, // git diff after
            ExecResult {
                stdout: String::new(),
                ..ok_result()
            }, // git ls-files after
        ]));
        let sandbox_dyn: Arc<dyn Sandbox> = sandbox.clone();
        let emitter = Arc::new(EventEmitter::new());
        let stage_dir =
            std::env::temp_dir().join(format!("fabro-cli-run-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&stage_dir).unwrap();

        let node = Node::new("test");
        let backend = AgentCliBackend::new("claude-opus-4-6".into(), Provider::Anthropic)
            .with_env(HashMap::from([
                ("MINIMAX_API_KEY".to_string(), "secret".to_string()),
                ("ANTHROPIC_AUTH_TOKEN".to_string(), "secret".to_string()),
            ]))
            .with_poll_interval(std::time::Duration::from_millis(1));

        let result = backend
            .run(
                &node,
                "hello from prompt",
                &Context::default(),
                None,
                &emitter,
                &stage_dir,
                &sandbox_dyn,
                None,
            )
            .await
            .unwrap();

        let CodergenResult::Text { text, .. } = result else {
            panic!("expected text result");
        };
        assert_eq!(text, "done");

        let writes = sandbox.writes();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, "/home/test/.fabro/cli/run.abcd12/prompt.txt");
        assert_eq!(writes[0].1, "hello from prompt");

        let commands = sandbox.commands();
        assert!(commands[2].contains("base=\"$HOME/.fabro/cli\""));
        assert!(commands
            .iter()
            .all(|command| !command.contains(".fabro_cli/")));
        assert!(commands.iter().all(|command| !command.contains("env.sh")));
        assert!(commands.iter().any(|command| {
            command.contains("/home/test/.fabro/cli/run.abcd12/prompt.txt")
                && command.contains("claude -p")
        }));
        assert!(commands
            .iter()
            .any(|command| command.contains("rm -rf '/home/test/.fabro/cli/run.abcd12'")));

        let _ = std::fs::remove_dir_all(&stage_dir);
    }

    // -- Cycle 1: cli_command_for_provider --

    #[test]
    fn cli_command_for_codex() {
        let cmd = cli_command_for_provider(
            Provider::OpenAi,
            "gpt-5.3-codex",
            "/tmp/prompt.txt",
            None,
            false,
            None,
        );
        assert!(cmd.starts_with("cat /tmp/prompt.txt | codex exec --json --yolo"));
        assert!(cmd.contains("-m gpt-5.3-codex"));
    }

    #[test]
    fn cli_command_for_claude() {
        let cmd = cli_command_for_provider(
            Provider::Anthropic,
            "claude-opus-4-6",
            "/tmp/prompt.txt",
            None,
            false,
            None,
        );
        assert!(cmd.starts_with("cat /tmp/prompt.txt |"));
        assert!(cmd.contains("claude -p"));
        assert!(cmd.contains("--dangerously-skip-permissions"));
        assert!(cmd.contains("--output-format stream-json"));
        assert!(cmd.contains("--model claude-opus-4-6"));
    }

    #[test]
    fn cli_command_for_gemini() {
        let cmd = cli_command_for_provider(
            Provider::Gemini,
            "gemini-3.1-pro",
            "/tmp/prompt.txt",
            None,
            false,
            None,
        );
        assert!(cmd.starts_with("cat /tmp/prompt.txt | gemini -o json --yolo"));
        assert!(cmd.contains("-m gemini-3.1-pro"));
    }

    #[test]
    fn cli_command_for_pi_minimax() {
        let cmd = cli_command_for_provider(
            Provider::Minimax,
            "MiniMax-M2.7-highspeed",
            "/tmp/prompt.txt",
            Some("high"),
            false,
            None,
        );
        assert!(cmd.starts_with("prompt=\"$(cat /tmp/prompt.txt)\" && pi --provider minimax"));
        assert!(cmd.contains("--mode json -p --no-session"));
        assert!(cmd.contains("--tools read,bash,edit,write,grep,find,ls"));
        assert!(cmd.contains("--model MiniMax-M2.7-highspeed"));
        assert!(cmd.contains("--thinking high"));
    }

    #[test]
    fn cli_command_pi_passes_reasoning_effort() {
        let cmd = cli_command_for_provider(
            Provider::Minimax,
            "MiniMax-M2.7-highspeed",
            "/tmp/prompt.txt",
            Some("medium"),
            false,
            None,
        );
        assert!(cmd.contains("--thinking medium"));

        let cmd = cli_command_for_provider(
            Provider::Minimax,
            "MiniMax-M2.7-highspeed",
            "/tmp/prompt.txt",
            None,
            false,
            None,
        );
        assert!(cmd.contains("--thinking high"));
    }

    #[test]
    fn cli_command_pi_readonly_restricts_tools() {
        let cmd = cli_command_for_provider(
            Provider::Minimax,
            "MiniMax-M2.7-highspeed",
            "/tmp/prompt.txt",
            Some("high"),
            true,
            None,
        );
        assert!(cmd.contains("--tools read,bash,grep,find,ls"));
        assert!(!cmd.contains("edit"));
        assert!(!cmd.contains("write"));
    }

    #[test]
    fn is_readonly_stage_identifies_review_nodes() {
        assert!(is_readonly_stage("challenge"));
        assert!(is_readonly_stage("review"));
        assert!(is_readonly_stage("deep_review"));
        assert!(is_readonly_stage("escalation"));
        assert!(!is_readonly_stage("implement"));
        assert!(!is_readonly_stage("fixup"));
        assert!(!is_readonly_stage("polish"));
    }

    #[test]
    fn cli_command_omits_model_when_empty() {
        let cmd =
            cli_command_for_provider(Provider::OpenAi, "", "/tmp/prompt.txt", None, false, None);
        assert!(cmd.contains("codex exec --json --yolo"));
        assert!(!cmd.contains("-m "));
        let cmd = cli_command_for_provider(
            Provider::Anthropic,
            "",
            "/tmp/prompt.txt",
            None,
            false,
            None,
        );
        assert!(cmd.contains("--dangerously-skip-permissions"));
        assert!(!cmd.contains("--model "));
        let cmd =
            cli_command_for_provider(Provider::Gemini, "", "/tmp/prompt.txt", None, false, None);
        assert!(cmd.contains("--yolo"));
        assert!(!cmd.contains("-m "));
        let cmd =
            cli_command_for_provider(Provider::Minimax, "", "/tmp/prompt.txt", None, false, None);
        assert!(cmd.contains("pi --provider minimax"));
        assert!(!cmd.contains("--model "));
    }

    // -- Cycle 2: is_cli_only_model --

    #[test]
    fn no_models_are_currently_cli_only() {
        assert!(!is_cli_only_model("gpt-5.3-codex"));
        assert!(!is_cli_only_model("claude-opus-4-6"));
        assert!(!is_cli_only_model("gemini-3.1-pro-preview"));
    }

    // -- Cycle 3: parse_cli_response — Claude/Gemini/Pi --

    #[test]
    fn parse_claude_ndjson_extracts_text_and_usage() {
        let output = r#"{"type":"system","message":"Claude CLI v1.0","model":"MiniMax-M2.7-highspeed"}
{"type":"assistant","message":{"content":"thinking..."}}
{"type":"result","result":"Here is the implementation.","usage":{"input_tokens":100,"output_tokens":50}}"#;
        let response = parse_cli_response(Provider::Anthropic, output).unwrap();
        assert_eq!(response.text, "Here is the implementation.");
        assert_eq!(response.input_tokens, 100);
        assert_eq!(response.output_tokens, 50);
        assert_eq!(
            response.served_model.as_deref(),
            Some("MiniMax-M2.7-highspeed")
        );
    }

    #[test]
    fn parse_claude_ndjson_uses_last_result() {
        let output = r#"{"type":"result","result":"first","usage":{"input_tokens":10,"output_tokens":5}}
{"type":"result","result":"second","usage":{"input_tokens":20,"output_tokens":10}}"#;
        let response = parse_cli_response(Provider::Anthropic, output).unwrap();
        assert_eq!(response.text, "second");
        assert_eq!(response.input_tokens, 20);
    }

    #[test]
    fn parse_claude_ndjson_returns_none_for_no_result() {
        let output = r#"{"type":"system","message":"hello"}
{"type":"assistant","message":{"content":"no result line"}}"#;
        assert!(parse_cli_response(Provider::Anthropic, output).is_none());
    }

    #[test]
    fn parse_gemini_json_extracts_text_and_usage() {
        let output = r#"{"session_id":"abc","response":"Gemini says hello","stats":{"models":{"gemini-2.5-flash":{"tokens":{"input":200,"candidates":80,"total":280}}}}}"#;
        let response = parse_cli_response(Provider::Gemini, output).unwrap();
        assert_eq!(response.text, "Gemini says hello");
        assert_eq!(response.input_tokens, 200);
        assert_eq!(response.output_tokens, 80);
    }

    #[test]
    fn parse_gemini_json_handles_missing_stats() {
        let output = r#"{"response":"hello"}"#;
        let response = parse_cli_response(Provider::Gemini, output).unwrap();
        assert_eq!(response.text, "hello");
        assert_eq!(response.input_tokens, 0);
        assert_eq!(response.output_tokens, 0);
    }

    #[test]
    fn parse_gemini_json_returns_none_for_invalid_json() {
        assert!(parse_cli_response(Provider::Gemini, "not json").is_none());
    }

    #[test]
    fn parse_pi_json_extracts_last_assistant_message_and_usage() {
        let output = r#"{"type":"session","version":3}
{"type":"message_end","message":{"role":"assistant","content":[{"type":"toolCall","name":"read","arguments":{"path":"/tmp/x"}}],"provider":"minimax","model":"MiniMax-M2.7-highspeed","usage":{"input":4,"output":52},"stopReason":"toolUse"}}
{"type":"message_end","message":{"role":"assistant","content":[{"type":"thinking","thinking":"done"},{"type":"text","text":"pi minimax ok"}],"provider":"minimax","model":"MiniMax-M2.7-highspeed","usage":{"input":90,"output":11},"stopReason":"stop"}}"#;
        let response = parse_cli_response(Provider::Minimax, output).unwrap();
        assert_eq!(response.text, "pi minimax ok");
        assert_eq!(response.input_tokens, 90);
        assert_eq!(response.output_tokens, 11);
        assert_eq!(
            response.served_model.as_deref(),
            Some("MiniMax-M2.7-highspeed")
        );
    }

    // -- Cycle 4: parse_cli_response — Codex NDJSON --

    #[test]
    fn parse_codex_ndjson_extracts_text_and_usage() {
        let output = r#"{"type":"thread.started","thread_id":"abc","model":"gpt-5.4"}
{"type":"turn.started"}
{"type":"item.completed","item":{"id":"item_0","type":"reasoning","text":"thinking..."}}
{"type":"item.completed","item":{"id":"item_1","type":"agent_message","text":"Fixed the bug."}}
{"type":"turn.completed","usage":{"input_tokens":300,"output_tokens":150}}"#;
        let response = parse_cli_response(Provider::OpenAi, output).unwrap();
        assert_eq!(response.text, "Fixed the bug.");
        assert_eq!(response.input_tokens, 300);
        assert_eq!(response.output_tokens, 150);
        assert_eq!(response.served_model.as_deref(), Some("gpt-5.4"));
    }

    #[test]
    fn parse_codex_ndjson_handles_no_message() {
        let output = r#"{"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":5}}"#;
        let response = parse_cli_response(Provider::OpenAi, output).unwrap();
        assert_eq!(response.text, "");
        assert_eq!(response.input_tokens, 10);
    }

    #[test]
    fn parse_codex_ndjson_returns_none_for_no_events() {
        assert!(parse_cli_response(Provider::OpenAi, "not json at all").is_none());
    }

    // -- Cycle 5: Node::backend() accessor (tested here since the accessor is simple) --

    #[test]
    fn node_backend_returns_none_by_default() {
        let node = Node::new("test");
        assert_eq!(node.backend(), None);
    }

    #[test]
    fn node_backend_returns_cli_when_set() {
        let mut node = Node::new("test");
        node.attrs
            .insert("backend".to_string(), AttrValue::String("cli".to_string()));
        assert_eq!(node.backend(), Some("cli"));
    }

    // -- Cycle 6: backend in stylesheet (tested in stylesheet.rs) --

    // -- Cycle 7: BackendRouter routing logic --

    #[test]
    fn router_uses_cli_for_backend_attr() {
        let mut node = Node::new("test");
        node.attrs
            .insert("backend".to_string(), AttrValue::String("cli".to_string()));

        let cli_backend = AgentCliBackend::new("model".into(), Provider::Anthropic);
        let router = BackendRouter::new(Box::new(StubBackend), cli_backend);
        assert!(router.should_use_cli(&node));
    }

    #[test]
    fn router_uses_api_by_default() {
        let node = Node::new("test");

        let cli_backend = AgentCliBackend::new("model".into(), Provider::Anthropic);
        let router = BackendRouter::new(Box::new(StubBackend), cli_backend);
        assert!(!router.should_use_cli(&node));
    }

    #[test]
    fn router_uses_api_for_non_cli_model() {
        let mut node = Node::new("test");
        node.attrs.insert(
            "model".to_string(),
            AttrValue::String("claude-opus-4-6".to_string()),
        );

        let cli_backend = AgentCliBackend::new("model".into(), Provider::Anthropic);
        let router = BackendRouter::new(Box::new(StubBackend), cli_backend);
        assert!(!router.should_use_cli(&node));
    }

    /// Minimal stub backend for testing routing logic.
    struct StubBackend;

    #[async_trait]
    impl CodergenBackend for StubBackend {
        async fn run(
            &self,
            _node: &Node,
            _prompt: &str,
            _context: &Context,
            _thread_id: Option<&str>,
            _emitter: &Arc<EventEmitter>,
            _stage_dir: &Path,
            _sandbox: &Arc<dyn Sandbox>,
            _tool_hooks: Option<Arc<dyn fabro_agent::ToolHookCallback>>,
        ) -> Result<CodergenResult, FabroError> {
            Ok(CodergenResult::Text {
                text: "stub".to_string(),
                usage: None,
                files_read: Vec::new(),
                files_written: Vec::new(),
                files_touched: Vec::new(),
                last_file_touched: None,
            })
        }
    }
}
