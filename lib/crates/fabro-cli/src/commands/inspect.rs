use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct InspectArgs {
    /// Run ID prefix or workflow name (most recent run)
    pub run: String,
}

pub fn run(args: &InspectArgs) -> Result<()> {
    let base = fabro_workflows::run_lookup::default_runs_base();
    let output = fabro_workflows::run_inspect::inspect_run(&base, &args.run)?;
    let json = serde_json::to_string_pretty(&[output])?;
    println!("{json}");
    Ok(())
}

#[cfg(test)]
mod tests {
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

        let output =
            fabro_workflows::run_inspect::inspect_run_dir("run-1", run_dir, RunStatus::Running)
                .unwrap();

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
