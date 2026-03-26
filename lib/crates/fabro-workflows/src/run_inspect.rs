use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::conclusion::Conclusion;
use crate::live_state::RunLiveState;
use crate::manifest::Manifest;
use crate::run_lookup::{resolve_run, RunInfo};
use crate::run_status::{RunStatus, RunStatusRecord};
use crate::sandbox_record::SandboxRecord;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct ProgressSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_event: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_completed_stage_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_stage_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_stage_duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_files_read: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_files_written: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_usage: Option<UsageSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsageSummary {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunInspection {
    pub run_id: String,
    pub run_dir: PathBuf,
    pub status: RunStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_record: Option<RunStatusRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<Manifest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conclusion: Option<Conclusion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<RunLiveState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<SandboxRecord>,
    #[serde(default, skip_serializing_if = "ProgressSummary::is_empty")]
    pub progress: ProgressSummary,
}

impl ProgressSummary {
    fn is_empty(&self) -> bool {
        self.latest_event.is_none()
            && self.last_completed_stage_label.is_none()
            && self.current_stage_label.is_none()
            && self.last_stage_duration_ms.is_none()
            && self.last_files_read.is_empty()
            && self.last_files_written.is_empty()
            && self.last_usage.is_none()
    }
}

pub fn inspect_run(base: &Path, identifier: &str) -> Result<RunInspection> {
    let run = resolve_run(base, identifier)
        .with_context(|| format!("failed to resolve run `{identifier}` in {}", base.display()))?;
    inspect_run_info(&run)
}

pub fn inspect_run_info(run: &RunInfo) -> Result<RunInspection> {
    inspect_run_dir(&run.run_id, &run.path, run.status)
}

pub fn inspect_run_dir(run_id: &str, run_dir: &Path, status: RunStatus) -> Result<RunInspection> {
    let manifest = Manifest::load(&run_dir.join("manifest.json")).ok();
    let status_record = RunStatusRecord::load(&run_dir.join("status.json")).ok();
    let conclusion = Conclusion::load(&run_dir.join("conclusion.json")).ok();
    let state = RunLiveState::load(&run_dir.join("state.json")).ok();
    let live = std::fs::read_to_string(run_dir.join("live.json"))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok());
    let sandbox = SandboxRecord::load(&run_dir.join("sandbox.json")).ok();
    let progress = load_progress_summary(run_dir, state.as_ref())?;

    Ok(RunInspection {
        run_id: run_id.to_string(),
        run_dir: run_dir.to_path_buf(),
        status,
        status_record,
        manifest,
        conclusion,
        state,
        live,
        sandbox,
        progress,
    })
}

fn load_progress_summary(run_dir: &Path, state: Option<&RunLiveState>) -> Result<ProgressSummary> {
    let progress_path = run_dir.join("progress.jsonl");
    let contents = match std::fs::read_to_string(&progress_path) {
        Ok(contents) => contents,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(source) => {
            return Err(source)
                .with_context(|| format!("failed to read {}", progress_path.display()));
        }
    };

    let mut summary = ProgressSummary {
        latest_event: state.and_then(|state| state.last_event.clone()),
        last_completed_stage_label: state
            .and_then(|state| state.last_completed_stage_label.clone()),
        current_stage_label: state.and_then(|state| state.current_stage_label.clone()),
        last_files_read: state
            .map(|state| state.last_files_read.clone())
            .unwrap_or_default(),
        last_files_written: state
            .map(|state| state.last_files_written.clone())
            .unwrap_or_default(),
        ..ProgressSummary::default()
    };

    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let event = value
            .get("event")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        summary.latest_event = Some(event.clone());
        match event.as_str() {
            "StageStarted" => {
                summary.current_stage_label = value
                    .get("node_label")
                    .or_else(|| value.get("name"))
                    .and_then(|value| value.as_str())
                    .map(ToOwned::to_owned);
            }
            "StageCompleted" => {
                summary.last_completed_stage_label = value
                    .get("node_label")
                    .or_else(|| value.get("name"))
                    .and_then(|value| value.as_str())
                    .map(ToOwned::to_owned);
                summary.last_stage_duration_ms =
                    value.get("duration_ms").and_then(|value| value.as_u64());
                summary.last_files_read = parse_string_array(&value, "files_read");
                summary.last_files_written = parse_string_array(&value, "files_written");
                summary.last_usage = parse_usage(&value);
                summary.current_stage_label = None;
            }
            "WorkflowRunCompleted" | "WorkflowRunFailed" => {
                summary.last_usage = parse_usage(&value).or(summary.last_usage);
                summary.current_stage_label = None;
            }
            _ => {}
        }
    }

    Ok(summary)
}

fn parse_string_array(value: &serde_json::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
        .collect()
}

fn parse_usage(value: &serde_json::Value) -> Option<UsageSummary> {
    let usage = value.get("usage")?;
    let model = usage
        .get("model")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown")
        .to_string();
    let input_tokens = usage
        .get("input_tokens")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let output_tokens = usage
        .get("output_tokens")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    Some(UsageSummary {
        model,
        input_tokens,
        output_tokens,
    })
}

pub fn summarize_usage(usage: Option<&UsageSummary>) -> Option<String> {
    usage.map(|usage| {
        format!(
            "{}: {} in / {} out",
            usage.model, usage.input_tokens, usage.output_tokens
        )
    })
}

pub fn finished_at(inspection: &RunInspection) -> Option<DateTime<Utc>> {
    inspection
        .conclusion
        .as_ref()
        .map(|conclusion| conclusion.timestamp)
        .or_else(|| {
            inspection.state.as_ref().and_then(|state| {
                if state.status.is_terminal() {
                    Some(state.updated_at)
                } else {
                    None
                }
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_status::RunStatusRecord;

    #[test]
    fn inspect_reads_progress_summary() {
        let dir = tempfile::tempdir().unwrap();
        let run_dir = dir.path();

        Manifest {
            run_id: "run-1".to_string(),
            workflow_name: "demo".to_string(),
            goal: "do work".to_string(),
            start_time: Utc::now(),
            node_count: 2,
            edge_count: 1,
            run_branch: None,
            base_sha: None,
            labels: std::collections::HashMap::new(),
            base_branch: None,
            workflow_slug: None,
            host_repo_path: None,
        }
        .save(&run_dir.join("manifest.json"))
        .unwrap();

        RunStatusRecord::new(RunStatus::Running, None)
            .save(&run_dir.join("status.json"))
            .unwrap();

        RunLiveState::new("run-1")
            .save(&run_dir.join("state.json"))
            .unwrap();

        std::fs::write(
            run_dir.join("progress.jsonl"),
            concat!(
                "{\"ts\":\"2026-03-19T01:00:00Z\",\"run_id\":\"run-1\",\"event\":\"StageStarted\",\"node_label\":\"Review\"}\n",
                "{\"ts\":\"2026-03-19T01:01:00Z\",\"run_id\":\"run-1\",\"event\":\"StageCompleted\",\"node_label\":\"Draft\",\"duration_ms\":1200,\"files_read\":[\"input.md\"],\"files_written\":[\"draft.md\"],\"usage\":{\"model\":\"gpt-5.4\",\"input_tokens\":100,\"output_tokens\":80}}\n"
            ),
        )
        .unwrap();

        let inspection = inspect_run_dir("run-1", run_dir, RunStatus::Running).unwrap();
        assert_eq!(
            inspection.progress.last_completed_stage_label.as_deref(),
            Some("Draft")
        );
        assert_eq!(inspection.progress.last_stage_duration_ms, Some(1200));
        assert_eq!(inspection.progress.last_files_written, vec!["draft.md"]);
        assert_eq!(
            summarize_usage(inspection.progress.last_usage.as_ref()).as_deref(),
            Some("gpt-5.4: 100 in / 80 out")
        );
    }

    #[test]
    fn finished_at_prefers_conclusion_timestamp() {
        let now = Utc::now();
        let inspection = RunInspection {
            run_id: "run-1".to_string(),
            run_dir: PathBuf::from("/tmp/run-1"),
            status: RunStatus::Succeeded,
            status_record: None,
            manifest: None,
            conclusion: Some(Conclusion {
                timestamp: now,
                status: crate::outcome::StageStatus::Success,
                duration_ms: 1,
                failure_reason: None,
                final_git_commit_sha: None,
                stages: Vec::new(),
                total_cost: None,
                total_retries: 0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_cache_read_tokens: 0,
                total_cache_write_tokens: 0,
                total_reasoning_tokens: 0,
                has_pricing: false,
            }),
            state: Some(RunLiveState::new("run-1")),
            live: None,
            sandbox: None,
            progress: ProgressSummary::default(),
        };

        assert_eq!(finished_at(&inspection), Some(now));
    }
}
