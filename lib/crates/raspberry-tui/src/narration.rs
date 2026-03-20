use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use chrono::{DateTime, Utc};
use fabro_config::project::discover_project_config;
use fabro_llm::generate::{generate, GenerateParams};
use fabro_llm::types::Message;
use raspberry_supervisor::ProgramManifest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorNarration {
    pub headline: String,
    pub running_now: Vec<String>,
    pub recent_changes: Vec<String>,
    pub blocked_or_risky: Vec<String>,
    pub next_expected: Vec<String>,
    pub confidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorNarrationRecord {
    pub generated_at: DateTime<Utc>,
    pub model: String,
    pub provider: String,
    pub snapshot: serde_json::Value,
    pub summary: OperatorNarration,
}

#[derive(Debug)]
pub struct NarrationRefreshHandle {
    receiver: Receiver<NarrationRefreshResult>,
}

#[derive(Debug)]
pub enum NarrationRefreshResult {
    Updated(OperatorNarrationRecord),
    Failed(String),
}

pub fn load_cached_operator_narration(
    manifest_path: &Path,
    manifest: &ProgramManifest,
) -> Option<OperatorNarrationRecord> {
    let path = operator_narration_path(manifest_path, manifest);
    load_operator_narration(&path)
}

pub fn narration_refresh_enabled() -> bool {
    if !narration_refresh_env_enabled() {
        return false;
    }
    true
}

pub fn start_operator_narration_refresh(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    snapshot: &serde_json::Value,
    existing: Option<&OperatorNarrationRecord>,
) -> Option<NarrationRefreshHandle> {
    if !narration_refresh_enabled() {
        return None;
    }
    if existing.is_some_and(|record| record.snapshot == *snapshot) {
        return None;
    }

    let manifest_path = manifest_path.to_path_buf();
    let manifest = manifest.clone();
    let snapshot = snapshot.clone();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let path = operator_narration_path(&manifest_path, &manifest);
        let result = match generate_operator_narration(&manifest_path, &manifest, &snapshot) {
            Ok(record) => {
                let _ = save_operator_narration(&path, &record);
                NarrationRefreshResult::Updated(record)
            }
            Err(error) => NarrationRefreshResult::Failed(error.to_string()),
        };
        let _ = sender.send(result);
    });

    Some(NarrationRefreshHandle { receiver })
}

impl NarrationRefreshHandle {
    pub fn try_complete(&self) -> Option<NarrationRefreshResult> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(NarrationRefreshResult::Failed(
                "narration refresh worker disconnected".to_string(),
            )),
        }
    }
}

fn generate_operator_narration(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    snapshot: &serde_json::Value,
) -> anyhow::Result<OperatorNarrationRecord> {
    let target_repo = manifest.resolved_target_repo(manifest_path);
    let (model, provider) = narration_model_provider(&target_repo);
    let prompt = format!(
        "Summarize this Raspberry operator snapshot in plain English.\n\n\
Rules:\n\
- Use only information present in the snapshot.\n\
- Be concrete about lane names and stages.\n\
- If nothing changed, say that.\n\
- Do not suggest code changes.\n\
- Output valid JSON only with keys headline, running_now, recent_changes, blocked_or_risky, next_expected, confidence.\n\
- Keep each list item to one sentence max.\n\n\
Snapshot:\n{}\n",
        serde_json::to_string_pretty(snapshot)?
    );

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(generate(
        GenerateParams::new(model.clone())
            .provider(provider.clone())
            .messages(vec![
                Message::system(
                    "You are an operator narrator. Summarize only what is explicitly \
                     present in the snapshot. Prefer concise, factual plain English. \
                     Output strict JSON only.",
                ),
                Message::user(prompt),
            ])
            .temperature(0.1)
            .max_tokens(600),
    ))?;

    let summary = serde_json::from_str::<OperatorNarration>(&result.response.text())?;
    Ok(OperatorNarrationRecord {
        generated_at: Utc::now(),
        model,
        provider,
        snapshot: snapshot.clone(),
        summary,
    })
}

fn narration_model_provider(target_repo: &Path) -> (String, String) {
    if let Ok(Some((_path, config))) = discover_project_config(target_repo) {
        if let Some(llm) = config.llm {
            if let (Some(model), Some(provider)) = (llm.model, llm.provider) {
                return (model, provider);
            }
        }
    }
    (
        "MiniMax-M2.7-highspeed".to_string(),
        "anthropic".to_string(),
    )
}

fn narration_refresh_env_enabled() -> bool {
    let Ok(value) = env::var("RASPBERRY_TUI_ENABLE_NARRATION") else {
        return false;
    };
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn operator_narration_path(manifest_path: &Path, manifest: &ProgramManifest) -> PathBuf {
    manifest
        .resolved_target_repo(manifest_path)
        .join(".raspberry")
        .join(format!("{}-operator-summary.json", manifest.program))
}

fn load_operator_narration(path: &Path) -> Option<OperatorNarrationRecord> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_operator_narration(path: &Path, record: &OperatorNarrationRecord) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(record)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn build_operator_snapshot(
    program_name: &str,
    counts: &BTreeMap<&'static str, usize>,
    selected_lane: &serde_json::Value,
    child_digest: Option<&serde_json::Value>,
    autodev: Option<&serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "program": program_name,
        "counts": counts,
        "selected_lane": selected_lane,
        "child_digest": child_digest,
        "autodev": autodev,
    })
}
