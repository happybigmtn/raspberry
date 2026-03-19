use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::event::WorkflowRunEvent;
use crate::run_status::{RunStatus, StatusReason};

/// Stable current-state snapshot for an active or completed run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunLiveState {
    pub run_id: String,
    pub updated_at: DateTime<Utc>,
    pub status: RunStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<StatusReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_stage_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_stage_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_stage_attempt: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_stage_max_attempts: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_completed_stage_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_completed_stage_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_completed_stage_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_failure: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_event: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_event_seq: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checkpoint_sha: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checkpoint_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_files_read: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_files_written: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_files_touched: Vec<String>,
    #[serde(default)]
    pub completed_stage_count: usize,
}

impl RunLiveState {
    #[must_use]
    pub fn new(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            updated_at: Utc::now(),
            status: RunStatus::Starting,
            reason: Some(StatusReason::SandboxInitializing),
            current_stage_id: None,
            current_stage_label: None,
            current_stage_attempt: None,
            current_stage_max_attempts: None,
            last_completed_stage_id: None,
            last_completed_stage_label: None,
            last_completed_stage_status: None,
            last_failure: None,
            last_event: None,
            last_event_seq: None,
            last_checkpoint_sha: None,
            last_checkpoint_at: None,
            last_files_read: Vec::new(),
            last_files_written: Vec::new(),
            last_files_touched: Vec::new(),
            completed_stage_count: 0,
        }
    }

    pub fn observe(
        &mut self,
        event: &WorkflowRunEvent,
        event_name: Option<&str>,
        event_seq: Option<u64>,
        observed_at: DateTime<Utc>,
    ) {
        self.updated_at = observed_at;
        self.last_event = event_name.map(str::to_owned);
        self.last_event_seq = event_seq;

        match event {
            WorkflowRunEvent::WorkflowRunStarted { run_id, .. } => {
                self.run_id = run_id.clone();
                self.status = RunStatus::Running;
                self.reason = None;
                self.last_failure = None;
            }
            WorkflowRunEvent::WorkflowRunCompleted { status, .. } => {
                let (run_status, reason) = match status.as_str() {
                    "success" => (RunStatus::Succeeded, Some(StatusReason::Completed)),
                    "partial_success" => (RunStatus::Succeeded, Some(StatusReason::PartialSuccess)),
                    _ => (RunStatus::Failed, Some(StatusReason::WorkflowError)),
                };
                self.status = run_status;
                self.reason = reason;
                self.current_stage_id = None;
                self.current_stage_label = None;
                self.current_stage_attempt = None;
                self.current_stage_max_attempts = None;
            }
            WorkflowRunEvent::WorkflowRunFailed { error, .. } => {
                self.status = RunStatus::Failed;
                self.reason = Some(StatusReason::WorkflowError);
                self.current_stage_id = None;
                self.current_stage_label = None;
                self.current_stage_attempt = None;
                self.current_stage_max_attempts = None;
                self.last_failure = Some(error.to_string());
            }
            WorkflowRunEvent::StageStarted {
                node_id,
                name,
                attempt,
                max_attempts,
                ..
            } => {
                self.status = RunStatus::Running;
                self.reason = None;
                self.current_stage_id = Some(node_id.clone());
                self.current_stage_label = Some(name.clone());
                self.current_stage_attempt = Some(*attempt);
                self.current_stage_max_attempts = Some(*max_attempts);
            }
            WorkflowRunEvent::StageCompleted {
                node_id,
                name,
                status,
                failure,
                files_read,
                files_written,
                files_touched,
                ..
            } => {
                self.last_completed_stage_id = Some(node_id.clone());
                self.last_completed_stage_label = Some(name.clone());
                self.last_completed_stage_status = Some(status.clone());
                self.completed_stage_count = self.completed_stage_count.saturating_add(1);
                self.current_stage_id = None;
                self.current_stage_label = None;
                self.current_stage_attempt = None;
                self.current_stage_max_attempts = None;
                self.last_failure = failure.as_ref().map(|detail| detail.message.clone());
                self.last_files_read = files_read.clone();
                self.last_files_written = files_written.clone();
                self.last_files_touched = files_touched.clone();
            }
            WorkflowRunEvent::StageFailed {
                node_id,
                name,
                failure,
                ..
            } => {
                self.current_stage_id = Some(node_id.clone());
                self.current_stage_label = Some(name.clone());
                self.last_failure = Some(failure.message.clone());
            }
            WorkflowRunEvent::StageRetrying {
                node_id,
                name,
                attempt,
                max_attempts,
                ..
            } => {
                self.status = RunStatus::Running;
                self.reason = None;
                self.current_stage_id = Some(node_id.clone());
                self.current_stage_label = Some(name.clone());
                self.current_stage_attempt = Some(*attempt);
                self.current_stage_max_attempts = Some(*max_attempts);
            }
            WorkflowRunEvent::InterviewStarted { .. } => {
                self.status = RunStatus::Paused;
                self.reason = None;
            }
            WorkflowRunEvent::InterviewCompleted { .. }
            | WorkflowRunEvent::InterviewTimeout { .. } => {
                if !self.status.is_terminal() {
                    self.status = RunStatus::Running;
                    self.reason = None;
                }
            }
            WorkflowRunEvent::CheckpointCompleted { git_commit_sha, .. } => {
                self.last_checkpoint_sha = git_commit_sha.clone();
                self.last_checkpoint_at = Some(observed_at);
            }
            _ => {}
        }
    }

    pub fn save(&self, path: &Path) -> crate::error::Result<()> {
        crate::save_json(self, path, "live_state")
    }

    pub fn load(path: &Path) -> crate::error::Result<Self> {
        crate::load_json(path, "live_state")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FabroError;

    #[test]
    fn stage_lifecycle_updates_current_and_last_completed_stage() {
        let mut state = RunLiveState::new("run-1");

        state.observe(
            &WorkflowRunEvent::StageStarted {
                node_id: "verify".to_string(),
                name: "Verify".to_string(),
                index: 2,
                handler_type: Some("agent".to_string()),
                script: None,
                attempt: 1,
                max_attempts: 3,
            },
            Some("StageStarted"),
            Some(4),
            Utc::now(),
        );

        assert_eq!(state.current_stage_id.as_deref(), Some("verify"));
        assert_eq!(state.current_stage_attempt, Some(1));
        assert_eq!(state.status, RunStatus::Running);
        assert_eq!(state.last_event_seq, Some(4));

        state.observe(
            &WorkflowRunEvent::StageCompleted {
                node_id: "verify".to_string(),
                name: "Verify".to_string(),
                index: 2,
                duration_ms: 1200,
                status: "success".to_string(),
                preferred_label: None,
                suggested_next_ids: Vec::new(),
                usage: None,
                failure: None,
                notes: Some("looks good".to_string()),
                files_read: vec!["src/lib.rs".to_string()],
                files_written: vec!["README.md".to_string()],
                files_touched: vec!["README.md".to_string(), "src/lib.rs".to_string()],
                attempt: 1,
                max_attempts: 3,
            },
            Some("StageCompleted"),
            Some(5),
            Utc::now(),
        );

        assert!(state.current_stage_id.is_none());
        assert_eq!(state.last_completed_stage_id.as_deref(), Some("verify"));
        assert_eq!(
            state.last_completed_stage_status.as_deref(),
            Some("success")
        );
        assert_eq!(state.completed_stage_count, 1);
        assert_eq!(state.last_files_read, vec!["src/lib.rs".to_string()]);
        assert_eq!(state.last_files_written, vec!["README.md".to_string()]);
    }

    #[test]
    fn workflow_failure_sets_failed_status_and_reason() {
        let mut state = RunLiveState::new("run-1");

        state.observe(
            &WorkflowRunEvent::WorkflowRunFailed {
                error: FabroError::Validation("bad graph".to_string()),
                duration_ms: 50,
                git_commit_sha: None,
            },
            Some("WorkflowRunFailed"),
            Some(2),
            Utc::now(),
        );

        assert_eq!(state.status, RunStatus::Failed);
        assert_eq!(state.reason, Some(StatusReason::WorkflowError));
        assert_eq!(
            state.last_failure.as_deref(),
            Some("Validation error: bad graph")
        );
    }

    #[test]
    fn interview_events_toggle_paused_state() {
        let mut state = RunLiveState::new("run-1");
        state.status = RunStatus::Running;
        state.reason = None;

        state.observe(
            &WorkflowRunEvent::InterviewStarted {
                question: "Ship it?".to_string(),
                stage: "review".to_string(),
                question_type: "yes_no".to_string(),
            },
            Some("InterviewStarted"),
            Some(3),
            Utc::now(),
        );
        assert_eq!(state.status, RunStatus::Paused);

        state.observe(
            &WorkflowRunEvent::InterviewCompleted {
                question: "Ship it?".to_string(),
                answer: "yes".to_string(),
                duration_ms: 100,
            },
            Some("InterviewCompleted"),
            Some(4),
            Utc::now(),
        );
        assert_eq!(state.status, RunStatus::Running);
    }

    #[test]
    fn checkpoint_events_update_last_checkpoint_metadata() {
        let mut state = RunLiveState::new("run-1");
        let observed_at = Utc::now();

        state.observe(
            &WorkflowRunEvent::CheckpointCompleted {
                node_id: "review".to_string(),
                status: "success".to_string(),
                git_commit_sha: Some("abc123".to_string()),
            },
            Some("CheckpointCompleted"),
            Some(8),
            observed_at.clone(),
        );

        assert_eq!(state.last_checkpoint_sha.as_deref(), Some("abc123"));
        assert_eq!(state.last_checkpoint_at.as_ref(), Some(&observed_at));
        assert_eq!(state.last_event_seq, Some(8));
    }
}
