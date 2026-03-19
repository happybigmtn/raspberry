use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Args;
use serde::Serialize;

#[derive(Args)]
pub struct InspectArgs {
    /// Run ID prefix or workflow name (most recent run)
    pub run: String,
}

#[derive(Debug, Serialize)]
pub struct InspectOutput {
    pub run_id: String,
    pub run_dir: PathBuf,
    pub status: fabro_workflows::run_status::RunStatus,
    pub status_record: Option<serde_json::Value>,
    pub manifest: Option<serde_json::Value>,
    pub conclusion: Option<serde_json::Value>,
    pub checkpoint: Option<serde_json::Value>,
    pub state: Option<serde_json::Value>,
    pub live: Option<serde_json::Value>,
    pub sandbox: Option<serde_json::Value>,
}

pub fn run(args: &InspectArgs) -> Result<()> {
    let base = fabro_workflows::run_lookup::default_runs_base();
    let run = fabro_workflows::run_lookup::resolve_run(&base, &args.run)?;
    let output = inspect_run_dir(&run.run_id, &run.path, run.status)?;
    let json = serde_json::to_string_pretty(&[output])?;
    println!("{json}");
    Ok(())
}

fn inspect_run_dir(
    run_id: &str,
    run_dir: &Path,
    status: fabro_workflows::run_status::RunStatus,
) -> Result<InspectOutput> {
    let manifest = fabro_workflows::manifest::Manifest::load(&run_dir.join("manifest.json"))
        .ok()
        .and_then(|v| serde_json::to_value(v).ok());
    let status_record =
        fabro_workflows::run_status::RunStatusRecord::load(&run_dir.join("status.json"))
            .ok()
            .and_then(|v| serde_json::to_value(v).ok());
    let conclusion =
        fabro_workflows::conclusion::Conclusion::load(&run_dir.join("conclusion.json"))
            .ok()
            .and_then(|v| serde_json::to_value(v).ok());
    let checkpoint =
        fabro_workflows::checkpoint::Checkpoint::load(&run_dir.join("checkpoint.json"))
            .ok()
            .and_then(|v| serde_json::to_value(v).ok());
    let state = fabro_workflows::live_state::RunLiveState::load(&run_dir.join("state.json"))
        .ok()
        .and_then(|v| serde_json::to_value(v).ok());
    let live = std::fs::read_to_string(run_dir.join("live.json"))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok());
    let sandbox =
        fabro_workflows::sandbox_record::SandboxRecord::load(&run_dir.join("sandbox.json"))
            .ok()
            .and_then(|v| serde_json::to_value(v).ok());

    Ok(InspectOutput {
        run_id: run_id.to_string(),
        run_dir: run_dir.to_path_buf(),
        status,
        status_record,
        manifest,
        conclusion,
        checkpoint,
        state,
        live,
        sandbox,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use fabro_workflows::run_status::{RunStatus, RunStatusRecord};

    #[test]
    fn inspect_reads_status_state_and_live_snapshots() {
        let dir = tempfile::tempdir().unwrap();
        let run_dir = dir.path();

        fabro_workflows::manifest::Manifest {
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

        fabro_workflows::live_state::RunLiveState::new("run-1")
            .save(&run_dir.join("state.json"))
            .unwrap();

        std::fs::write(
            run_dir.join("live.json"),
            r#"{"event":"StageStarted","node_id":"verify"}"#,
        )
        .unwrap();

        let output = inspect_run_dir("run-1", run_dir, RunStatus::Running).unwrap();

        assert!(output.status_record.is_some());
        assert!(output.state.is_some());
        assert_eq!(
            output
                .live
                .as_ref()
                .and_then(|v| v.get("event"))
                .and_then(|v| v.as_str()),
            Some("StageStarted")
        );
    }
}
