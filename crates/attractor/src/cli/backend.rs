use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use agent::{
    AnthropicProfile, DockerConfig, DockerExecutionEnvironment, EventData, EventKind,
    ExecutionEnvironment, GeminiProfile, LocalExecutionEnvironment, OpenAiProfile, ProviderProfile,
    Session, SessionConfig, Turn,
};
use agent::tool_registry::RegisteredTool;
use llm::client::Client;
use llm::types::ToolDefinition;
use terminal::Styles;

use crate::context::Context;
use crate::error::AttractorError;
use crate::graph::Node;
use crate::handler::codergen::{CodergenBackend, CodergenResult};
use crate::outcome::{Outcome, StageStatus, StageUsage};

/// LLM backend that delegates to an `agent` Session per invocation.
pub struct AgentBackend {
    model: String,
    provider: Option<String>,
    verbose: u8,
    styles: &'static Styles,
    docker: bool,
}

impl AgentBackend {
    #[must_use]
    pub const fn new(
        model: String,
        provider: Option<String>,
        verbose: u8,
        styles: &'static Styles,
        docker: bool,
    ) -> Self {
        Self {
            model,
            provider,
            verbose,
            styles,
            docker,
        }
    }

    fn build_profile(&self) -> Box<dyn ProviderProfile> {
        let provider = self.provider.as_deref().unwrap_or("anthropic");
        match provider {
            "openai" => Box::new(OpenAiProfile::new(&self.model)),
            "gemini" => Box::new(GeminiProfile::new(&self.model)),
            _ => Box::new(AnthropicProfile::new(&self.model)),
        }
    }
}

#[async_trait]
impl CodergenBackend for AgentBackend {
    async fn run(
        &self,
        node: &Node,
        prompt: &str,
        _context: &Context,
        _thread_id: Option<&str>,
    ) -> Result<CodergenResult, AttractorError> {
        let client = Client::from_env()
            .await
            .map_err(|e| AttractorError::Handler(format!("Failed to create LLM client: {e}")))?;

        let mut profile = self.build_profile();
        let (tool, outcome_cell) = make_report_outcome_tool();
        profile.tool_registry_mut().register(tool);
        let profile: Arc<dyn ProviderProfile> = Arc::from(profile);

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let exec_env: Arc<dyn ExecutionEnvironment> = if self.docker {
            let config = DockerConfig {
                host_working_directory: cwd.to_string_lossy().to_string(),
                ..DockerConfig::default()
            };
            Arc::new(
                DockerExecutionEnvironment::new(config)
                    .map_err(|e| AttractorError::Handler(format!("Failed to create Docker environment: {e}")))?,
            )
        } else {
            Arc::new(LocalExecutionEnvironment::new(cwd))
        };

        let config = SessionConfig {
            reasoning_effort: Some(node.reasoning_effort().to_string()),
            ..SessionConfig::default()
        };

        let mut session = Session::new(client, profile, exec_env, config);

        // Subscribe to session events for real-time tool status on stderr.
        let verbose = self.verbose;
        if verbose >= 1 {
            let node_id = node.id.clone();
            let styles = self.styles;
            let mut rx = session.subscribe();
            tokio::spawn(async move {
                while let Ok(event) = rx.recv().await {
                    match (&event.kind, &event.data) {
                        (
                            EventKind::ToolCallStart,
                            EventData::ToolCall {
                                tool_name,
                                arguments,
                                ..
                            },
                        ) => {
                            eprintln!(
                                "{dim}[{node_id}]{reset}   {dim}\u{25cf}{reset} {bold}{cyan}{tool_name}{reset}{dim}({args}){reset}",
                                dim = styles.dim,
                                reset = styles.reset,
                                bold = styles.bold,
                                cyan = styles.cyan,
                                args = format_tool_args(arguments),
                            );
                        }
                        (
                            EventKind::ToolCallEnd,
                            EventData::ToolCallEnd {
                                tool_name,
                                output,
                                is_error,
                                ..
                            },
                        ) if verbose >= 2 => {
                            let label = if *is_error { "error" } else { "result" };
                            eprintln!(
                                "{dim}[{node_id}]   [{label}] {tool_name}:{reset}\n{}",
                                serde_json::to_string_pretty(output)
                                    .unwrap_or_else(|_| output.to_string()),
                                dim = styles.dim,
                                reset = styles.reset,
                            );
                        }
                        (EventKind::Error, EventData::Error { error }) => {
                            eprintln!(
                                "{dim}[{node_id}]{reset}   {red}\u{2717} {error}{reset}",
                                dim = styles.dim,
                                red = styles.red,
                                reset = styles.reset,
                            );
                        }
                        _ => {}
                    }
                }
            });
        }

        session.initialize().await;
        session.process_input(prompt).await.map_err(|e| {
            AttractorError::Handler(format!("Agent session failed: {e}"))
        })?;

        // Aggregate token usage from all assistant turns.
        let (mut turn_count, mut tool_call_count, mut input_tokens, mut output_tokens) =
            (0usize, 0usize, 0i64, 0i64);
        for turn in session.history().turns() {
            if let Turn::Assistant {
                tool_calls, usage, ..
            } = turn
            {
                turn_count += 1;
                tool_call_count += tool_calls.len();
                input_tokens += usage.input_tokens;
                output_tokens += usage.output_tokens;
            }
        }

        let stage_usage = StageUsage {
            model: self.model.clone(),
            input_tokens,
            output_tokens,
        };

        // Print session summary to stderr.
        if self.verbose >= 1 {
            let total_tokens = input_tokens + output_tokens;
            let token_str = if total_tokens >= 1000 {
                format!("{}k tokens", total_tokens / 1000)
            } else {
                format!("{total_tokens} tokens")
            };
            eprintln!(
                "{dim}[{node_id}] Done ({turn_count} turns, {tool_call_count} tool calls, {token_str}){reset}",
                node_id = node.id,
                dim = self.styles.dim,
                reset = self.styles.reset,
            );
        }

        // If the LLM called report_outcome, return a Full outcome.
        let tool_outcome = outcome_cell.lock().unwrap().take();
        if let Some(mut outcome) = tool_outcome {
            outcome.usage = Some(stage_usage);
            return Ok(CodergenResult::Full(outcome));
        }

        // Extract last assistant response from the session history.
        let response = session
            .history()
            .turns()
            .iter()
            .rev()
            .find_map(|turn| {
                if let Turn::Assistant { content, .. } = turn {
                    if !content.is_empty() {
                        return Some(content.clone());
                    }
                }
                None
            })
            .unwrap_or_default();

        Ok(CodergenResult::Text { text: response, usage: Some(stage_usage) })
    }
}

/// Creates a `report_outcome` tool that the LLM calls to declare routing decisions.
///
/// Returns the `RegisteredTool` and a shared cell where the tool stores the latest `Outcome`.
///
/// # Panics
///
/// The tool executor panics if the internal mutex is poisoned.
#[must_use]
pub fn make_report_outcome_tool() -> (RegisteredTool, Arc<Mutex<Option<Outcome>>>) {
    let outcome_cell: Arc<Mutex<Option<Outcome>>> = Arc::new(Mutex::new(None));
    let cell = outcome_cell.clone();

    let definition = ToolDefinition {
        name: "report_outcome".to_string(),
        description: "Report the outcome of this task, including routing preference and context updates. \
            Call this tool when you have completed your work to declare the result status \
            and optionally indicate which next step should be taken."
            .to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["status"],
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["success", "fail", "partial_success", "retry", "skipped"],
                    "description": "The result status of this task."
                },
                "preferred_next_label": {
                    "type": "string",
                    "description": "The label of the preferred next edge to follow."
                },
                "context_updates": {
                    "type": "object",
                    "description": "Key-value pairs to merge into the pipeline context."
                },
                "notes": {
                    "type": "string",
                    "description": "Optional notes about the outcome."
                },
                "failure_reason": {
                    "type": "string",
                    "description": "Reason for failure (when status is 'fail')."
                }
            }
        }),
    };

    let executor: agent::tool_registry::ToolExecutor = Arc::new(move |args, _env, _cancel| {
        let cell = cell.clone();
        Box::pin(async move {
            let status_str = args
                .get("status")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "missing required field: status".to_string())?;

            let status: StageStatus = status_str
                .parse()
                .map_err(|e: String| e)?;

            let mut outcome = Outcome {
                status,
                preferred_label: args.get("preferred_next_label").and_then(|v| v.as_str()).map(String::from),
                suggested_next_ids: Vec::new(),
                context_updates: std::collections::HashMap::new(),
                notes: args.get("notes").and_then(|v| v.as_str()).map(String::from),
                failure_reason: args.get("failure_reason").and_then(|v| v.as_str()).map(String::from),
                usage: None,
            };

            if let Some(updates) = args.get("context_updates").and_then(|v| v.as_object()) {
                for (key, val) in updates {
                    outcome.context_updates.insert(key.clone(), val.clone());
                }
            }

            *cell.lock().unwrap() = Some(outcome);
            Ok("Outcome recorded.".to_string())
        })
    });

    let tool = RegisteredTool {
        definition,
        executor,
    };

    (tool, outcome_cell)
}

fn format_tool_args(args: &serde_json::Value) -> String {
    let Some(obj) = args.as_object() else {
        return args.to_string();
    };
    obj.iter()
        .map(|(k, v)| match v {
            serde_json::Value::String(s) => {
                let display = if s.len() > 80 {
                    format!("{}...", &s[..77])
                } else {
                    s.clone()
                };
                format!("{k}={display:?}")
            }
            other => format!("{k}={other}"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::sync::CancellationToken;

    fn dummy_env() -> Arc<dyn ExecutionEnvironment> {
        Arc::new(LocalExecutionEnvironment::new(std::path::PathBuf::from(".")))
    }

    #[tokio::test]
    async fn report_outcome_tool_captures_outcome() {
        let (tool, cell) = make_report_outcome_tool();
        let args = serde_json::json!({
            "status": "success",
            "preferred_next_label": "Fix"
        });
        let result = (tool.executor)(args, dummy_env(), CancellationToken::new()).await;
        assert!(result.is_ok());

        let outcome = cell.lock().unwrap().clone().unwrap();
        assert_eq!(outcome.status, StageStatus::Success);
        assert_eq!(outcome.preferred_label.as_deref(), Some("Fix"));
    }

    #[tokio::test]
    async fn report_outcome_tool_last_call_wins() {
        let (tool, cell) = make_report_outcome_tool();

        let _ = (tool.executor)(
            serde_json::json!({"status": "success", "notes": "first"}),
            dummy_env(),
            CancellationToken::new(),
        )
        .await;

        let _ = (tool.executor)(
            serde_json::json!({"status": "fail", "failure_reason": "oops"}),
            dummy_env(),
            CancellationToken::new(),
        )
        .await;

        let outcome = cell.lock().unwrap().clone().unwrap();
        assert_eq!(outcome.status, StageStatus::Fail);
        assert_eq!(outcome.failure_reason.as_deref(), Some("oops"));
    }

    #[tokio::test]
    async fn report_outcome_tool_parses_context_updates() {
        let (tool, cell) = make_report_outcome_tool();
        let args = serde_json::json!({
            "status": "success",
            "context_updates": {"k": "v"}
        });
        let result = (tool.executor)(args, dummy_env(), CancellationToken::new()).await;
        assert!(result.is_ok());

        let outcome = cell.lock().unwrap().clone().unwrap();
        assert_eq!(
            outcome.context_updates.get("k"),
            Some(&serde_json::json!("v"))
        );
    }

    #[tokio::test]
    async fn report_outcome_tool_invalid_status_errors() {
        let (tool, _cell) = make_report_outcome_tool();
        let args = serde_json::json!({"status": "bogus"});
        let result = (tool.executor)(args, dummy_env(), CancellationToken::new()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown stage status"));
    }
}
