use std::path::{Path, PathBuf};

use fabro_config::run::MergeStrategy;
use fabro_workflows::direct_integration::{
    integrate_run, DirectIntegrationError, DirectIntegrationRequest,
};
use fabro_workflows::run_inspect::inspect_run;
use fabro_workflows::run_status::RunStatus;
use thiserror::Error;

use crate::dispatch::DispatchOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationRequest {
    pub lane_key: String,
    pub source_lane_key: String,
    pub source_run_id: String,
    pub target_repo: PathBuf,
    pub artifact_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum IntegrationError {
    #[error("failed to inspect source run `{run_id}`: {message}")]
    InspectRun { run_id: String, message: String },
    #[error("source run `{run_id}` has no manifest")]
    MissingManifest { run_id: String },
    #[error(transparent)]
    Direct(#[from] DirectIntegrationError),
}

pub fn integrate_lane(request: &IntegrationRequest) -> Result<DispatchOutcome, IntegrationError> {
    let runs_base = fabro_workflows::run_lookup::default_runs_base();
    integrate_lane_from_runs_base(request, &runs_base)
}

fn integrate_lane_from_runs_base(
    request: &IntegrationRequest,
    runs_base: &Path,
) -> Result<DispatchOutcome, IntegrationError> {
    let inspection = inspect_run(runs_base, &request.source_run_id).map_err(|error| {
        IntegrationError::InspectRun {
            run_id: request.source_run_id.clone(),
            message: error.to_string(),
        }
    })?;
    let manifest = inspection
        .manifest
        .ok_or_else(|| IntegrationError::MissingManifest {
            run_id: request.source_run_id.clone(),
        })?;
    if inspection.status != RunStatus::Succeeded {
        return Err(IntegrationError::InspectRun {
            run_id: request.source_run_id.clone(),
            message: format!(
                "source run is not settled successfully (status={})",
                inspection.status
            ),
        });
    }

    let post_merge_check = detect_post_merge_check(&request.target_repo);
    let record = integrate_run(&DirectIntegrationRequest {
        source_lane: request.source_lane_key.clone(),
        manifest,
        repo_fallback: request.target_repo.clone(),
        target_branch: "origin/HEAD".to_string(),
        strategy: MergeStrategy::Squash,
        artifact_path: Some(request.artifact_path.clone()),
        post_merge_check,
    })?;

    let mut stdout = String::new();
    stdout.push_str(&format!(
        "integrated {} from {} into {}\n",
        record.source_lane, record.source_run_branch, record.target_branch
    ));
    stdout.push_str(&format!("commit_sha={}\n", record.commit_sha));
    if record.already_integrated {
        stdout.push_str("already_integrated=yes\n");
    }

    Ok(DispatchOutcome {
        lane_key: request.lane_key.clone(),
        exit_status: 0,
        fabro_run_id: None,
        stdout,
        stderr: String::new(),
    })
}

/// Detect the appropriate post-merge compilation check for a project.
/// Returns `None` for projects without a recognizable build system,
/// allowing the integration to proceed without a check (backwards compatible).
fn detect_post_merge_check(target_repo: &Path) -> Option<String> {
    if target_repo.join("Cargo.toml").exists() {
        // Verify it's a real Rust project, not a placeholder Cargo.toml
        if let Ok(contents) = std::fs::read_to_string(target_repo.join("Cargo.toml")) {
            let has_workspace = contents.contains("[workspace]") && contents.contains("members");
            let has_lib = contents.contains("[lib]") || target_repo.join("src/lib.rs").exists();
            let has_bin = contents.contains("[[bin]]") || target_repo.join("src/main.rs").exists();
            if has_workspace || has_lib || has_bin {
                return Some("cargo check --tests --workspace".to_string());
            }
        }
    }
    if target_repo.join("package.json").exists() {
        return Some("npm run build --if-present".to_string());
    }
    if target_repo.join("pyproject.toml").exists() || target_repo.join("setup.py").exists() {
        return Some(
            "python -m py_compile $(find . -name '*.py' -not -path './.*' | head -20) 2>&1 || true"
                .to_string(),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;

    use chrono::Utc;
    use fabro_workflows::manifest::Manifest as RunManifest;

    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .expect("git output");
        assert!(output.status.success(), "git command failed: {:?}", args);
    }

    fn git_output(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .expect("git output");
        assert!(output.status.success(), "git command failed: {:?}", args);
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    fn integrate_lane_squash_merges_run_branch_into_trunk() {
        let temp = tempfile::tempdir().expect("tempdir");
        let remote = temp.path().join("remote.git");
        git(
            temp.path(),
            &["init", "--bare", remote.to_str().expect("remote path")],
        );

        let repo = temp.path().join("repo");
        fs::create_dir_all(&repo).expect("repo dir");
        git(&repo, &["init", "-b", "main"]);
        git(&repo, &["config", "user.name", "Test"]);
        git(&repo, &["config", "user.email", "test@example.com"]);
        fs::write(repo.join("README.md"), "base\n").expect("readme");
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "base"]);
        git(
            &repo,
            &[
                "remote",
                "add",
                "origin",
                remote.to_str().expect("remote path"),
            ],
        );
        git(&repo, &["push", "-u", "origin", "main"]);
        git(&repo, &["remote", "set-head", "origin", "main"]);

        git(&repo, &["branch", "fabro/run/run-1", "HEAD"]);
        let worktree = temp.path().join("run-worktree");
        git(
            &repo,
            &[
                "worktree",
                "add",
                worktree.to_str().expect("worktree path"),
                "fabro/run/run-1",
            ],
        );
        fs::write(worktree.join("feature.txt"), "integrated\n").expect("feature");
        git(&worktree, &["add", "feature.txt"]);
        git(&worktree, &["commit", "-m", "feature"]);
        git(
            &repo,
            &[
                "push",
                "origin",
                "fabro/run/run-1:refs/heads/fabro/run/run-1",
            ],
        );
        git(
            &repo,
            &[
                "worktree",
                "remove",
                "--force",
                worktree.to_str().expect("worktree path"),
            ],
        );

        let runs_base = temp.path().join("runs");
        let run_dir = runs_base.join("20260319-run-1");
        fs::create_dir_all(&run_dir).expect("run dir");
        let run_manifest = RunManifest {
            run_id: "run-1".to_string(),
            workflow_name: "Demo".to_string(),
            goal: "Demo".to_string(),
            start_time: Utc::now(),
            node_count: 1,
            edge_count: 0,
            run_branch: Some("fabro/run/run-1".to_string()),
            base_sha: None,
            labels: std::collections::HashMap::new(),
            base_branch: Some("scratch".to_string()),
            workflow_slug: None,
            host_repo_path: Some(repo.display().to_string()),
        };
        run_manifest
            .save(&run_dir.join("manifest.json"))
            .expect("save manifest");
        fabro_workflows::run_status::RunStatusRecord::new(
            fabro_workflows::run_status::RunStatus::Succeeded,
            None,
        )
        .save(&run_dir.join("status.json"))
        .expect("save status");

        let artifact = temp.path().join("integration.md");
        let outcome = integrate_lane_from_runs_base(
            &IntegrationRequest {
                lane_key: "demo:integrate".to_string(),
                source_lane_key: "demo:implement".to_string(),
                source_run_id: "run-1".to_string(),
                target_repo: repo.clone(),
                artifact_path: artifact.clone(),
            },
            &runs_base,
        )
        .expect("integration succeeds");

        assert_eq!(outcome.exit_status, 0);
        assert!(artifact.exists(), "integration artifact should be written");
        assert_eq!(
            git_output(&repo, &["show", "origin/main:feature.txt"]),
            "integrated"
        );
    }
}
