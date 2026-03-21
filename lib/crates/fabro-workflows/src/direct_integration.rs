use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;
use fabro_config::run::MergeStrategy;
use serde::Serialize;
use thiserror::Error;

use crate::manifest::Manifest as RunManifest;

const DIRECT_INTEGRATION_LOCK_WAIT_SECS: u64 = 30;
const REMOTE_NAME: &str = "origin";
const DEFAULT_TARGET_BRANCH: &str = "origin/HEAD";

#[derive(Debug, Clone)]
pub struct DirectIntegrationRequest {
    pub source_lane: String,
    pub manifest: RunManifest,
    pub repo_fallback: PathBuf,
    pub target_branch: String,
    pub strategy: MergeStrategy,
    pub artifact_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DirectIntegrationRecord {
    pub integrated: bool,
    pub mode: String,
    pub source_lane: String,
    pub source_run_id: String,
    pub source_run_branch: String,
    pub target_branch: String,
    pub commit_sha: String,
    pub already_integrated: bool,
    pub pushed: bool,
    pub integrated_at: String,
}

#[derive(Debug, Error)]
pub enum DirectIntegrationError {
    #[error("run `{run_id}` is not branch-backed; rerun it in a branch-backed worktree")]
    MissingRunBranch { run_id: String },
    #[error("integration strategy `{strategy:?}` is not supported yet")]
    UnsupportedStrategy { strategy: MergeStrategy },
    #[error("failed to acquire direct integration lock at {path}")]
    LockTimeout { path: PathBuf },
    #[error("git {step} failed in {repo}: {message}")]
    Git {
        step: String,
        repo: PathBuf,
        message: String,
    },
    #[error("failed to create direct integration tempdir: {message}")]
    Tempdir { message: String },
    #[error("failed to write integration artifact {path}: {message}")]
    WriteArtifact { path: PathBuf, message: String },
}

pub fn integrate_run(
    request: &DirectIntegrationRequest,
) -> Result<DirectIntegrationRecord, DirectIntegrationError> {
    if request.strategy != MergeStrategy::Squash {
        return Err(DirectIntegrationError::UnsupportedStrategy {
            strategy: request.strategy,
        });
    }

    let run_branch = request.manifest.run_branch.as_ref().ok_or_else(|| {
        DirectIntegrationError::MissingRunBranch {
            run_id: request.manifest.run_id.clone(),
        }
    })?;
    let repo = request
        .manifest
        .host_repo_path
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| request.repo_fallback.clone());

    let _lock = acquire_integration_lock(&repo)?;
    let target_branch = detect_target_branch(&repo, &request.target_branch, &request.manifest)?;
    let source_ref = ensure_source_ref(&repo, run_branch)?;
    let base_ref = ensure_target_branch_ref(&repo, &target_branch)?;
    let tempdir = create_temp_worktree_dir()?;
    let worktree_path = tempdir.path.clone();
    let worktree_str = worktree_path.to_string_lossy().into_owned();

    run_git(
        &repo,
        "worktree add",
        [
            "worktree",
            "add",
            "--detach",
            worktree_str.as_str(),
            base_ref.as_str(),
        ],
    )?;

    let result = run_integration_worktree(
        request,
        &worktree_path,
        &base_ref,
        &target_branch,
        run_branch,
        &source_ref,
    );
    let _ = run_git(
        &repo,
        "worktree remove",
        ["worktree", "remove", "--force", worktree_str.as_str()],
    );
    result
}

fn run_integration_worktree(
    request: &DirectIntegrationRequest,
    worktree_path: &Path,
    base_ref: &str,
    target_branch: &str,
    run_branch: &str,
    source_ref: &str,
) -> Result<DirectIntegrationRecord, DirectIntegrationError> {
    let merge_output = run_git(
        worktree_path,
        "merge --squash",
        ["merge", "--squash", "--no-commit", source_ref],
    )?;
    let already_integrated = git_exit_status(
        worktree_path,
        ["diff", "--cached", "--quiet"],
        "diff --cached --quiet",
    )? == 0;

    let commit_sha = if already_integrated {
        head_sha(worktree_path)?
    } else {
        let subject = format!(
            "integrate({}): settle {}",
            sanitize_commit_component(&request.source_lane),
            request.manifest.run_id
        );
        run_git(
            worktree_path,
            "commit",
            [
                "-c",
                "user.name=Fabro",
                "-c",
                "user.email=noreply@fabro.sh",
                "commit",
                "-m",
                subject.as_str(),
            ],
        )?;
        head_sha(worktree_path)?
    };

    let (mode, pushed) = if base_ref.starts_with(&format!("{REMOTE_NAME}/")) {
        let push_refspec = format!("HEAD:refs/heads/{target_branch}");
        run_git(
            worktree_path,
            "push",
            ["push", REMOTE_NAME, push_refspec.as_str()],
        )?;
        ("direct_trunk_squash".to_string(), true)
    } else {
        let local_mode = match run_git(
            worktree_path,
            "branch -f target branch",
            ["branch", "-f", target_branch, "HEAD"],
        ) {
            Ok(_) => "direct_trunk_squash".to_string(),
            Err(DirectIntegrationError::Git { message, .. })
                if is_checked_out_branch_update_error(&message) =>
            {
                "direct_trunk_squash_local_branch_pending".to_string()
            }
            Err(error) => return Err(error),
        };
        (local_mode, false)
    };

    let record = DirectIntegrationRecord {
        integrated: true,
        mode,
        source_lane: request.source_lane.clone(),
        source_run_id: request.manifest.run_id.clone(),
        source_run_branch: run_branch.to_string(),
        target_branch: target_branch.to_string(),
        commit_sha,
        already_integrated,
        pushed,
        integrated_at: Utc::now().to_rfc3339(),
    };
    if let Some(path) = &request.artifact_path {
        write_integration_artifact(path, &record)?;
    }
    if !merge_output.stdout.trim().is_empty() {
        tracing::debug!(
            target_branch,
            source_ref,
            merge_output = %merge_output.stdout,
            "Direct integration merged source branch"
        );
    }
    Ok(record)
}

fn acquire_integration_lock(repo: &Path) -> Result<IntegrationLock, DirectIntegrationError> {
    let lock_path = repo.join(".raspberry").join("integration.lock");
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent).map_err(|error| DirectIntegrationError::WriteArtifact {
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }

    let started = Instant::now();
    loop {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(_) => return Ok(IntegrationLock { path: lock_path }),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                if started.elapsed() >= Duration::from_secs(DIRECT_INTEGRATION_LOCK_WAIT_SECS) {
                    return Err(DirectIntegrationError::LockTimeout { path: lock_path });
                }
                thread::sleep(Duration::from_millis(250));
            }
            Err(error) => {
                return Err(DirectIntegrationError::WriteArtifact {
                    path: lock_path,
                    message: error.to_string(),
                });
            }
        }
    }
}

fn detect_target_branch(
    repo: &Path,
    target_branch: &str,
    manifest: &RunManifest,
) -> Result<String, DirectIntegrationError> {
    if let Ok(branch) = std::env::var("FABRO_TRUNK_BRANCH") {
        let trimmed = branch.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    if target_branch != DEFAULT_TARGET_BRANCH {
        return Ok(target_branch.to_string());
    }

    let output = run_git(
        repo,
        "symbolic-ref refs/remotes/origin/HEAD",
        ["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"],
    );
    if let Ok(output) = output {
        if let Some(branch) = output.stdout.trim().strip_prefix("refs/remotes/origin/") {
            if !branch.is_empty() {
                return Ok(branch.to_string());
            }
        }
    }

    if let Some(branch) = manifest.base_branch.as_deref() {
        if !branch.trim().is_empty() {
            return Ok(branch.to_string());
        }
    }

    Err(DirectIntegrationError::Git {
        step: "detect trunk branch".to_string(),
        repo: repo.to_path_buf(),
        message:
            "set FABRO_TRUNK_BRANCH or configure origin/HEAD so direct integration can target trunk"
                .to_string(),
    })
}

fn ensure_target_branch_ref(
    repo: &Path,
    target_branch: &str,
) -> Result<String, DirectIntegrationError> {
    let fetch_output = run_git(
        repo,
        "fetch target branch",
        ["fetch", REMOTE_NAME, target_branch],
    );
    if let Err(error) = fetch_output {
        if !ref_exists(repo, &format!("refs/heads/{target_branch}"))? {
            return Err(error);
        }
    }

    let remote_ref = format!("refs/remotes/{REMOTE_NAME}/{target_branch}");
    let local_ref = format!("refs/heads/{target_branch}");
    let remote_exists = ref_exists(repo, &remote_ref)?;
    let local_exists = ref_exists(repo, &local_ref)?;

    if remote_exists && local_exists {
        return preferred_target_branch_ref(repo, target_branch, &local_ref, &remote_ref);
    }
    if remote_exists {
        return Ok(format!("{REMOTE_NAME}/{target_branch}"));
    }
    if local_exists {
        return Ok(target_branch.to_string());
    }

    Err(DirectIntegrationError::Git {
        step: "resolve target branch".to_string(),
        repo: repo.to_path_buf(),
        message: format!("branch `{target_branch}` is missing locally and on `{REMOTE_NAME}`"),
    })
}

fn preferred_target_branch_ref(
    repo: &Path,
    target_branch: &str,
    local_ref: &str,
    remote_ref: &str,
) -> Result<String, DirectIntegrationError> {
    let local_oid = ref_oid(repo, local_ref)?;
    let remote_oid = ref_oid(repo, remote_ref)?;
    if local_oid == remote_oid {
        return Ok(format!("{REMOTE_NAME}/{target_branch}"));
    }
    if is_ancestor(repo, &remote_oid, &local_oid)? {
        return Ok(target_branch.to_string());
    }
    if is_ancestor(repo, &local_oid, &remote_oid)? {
        return Ok(format!("{REMOTE_NAME}/{target_branch}"));
    }
    Ok(target_branch.to_string())
}

fn ensure_source_ref(repo: &Path, run_branch: &str) -> Result<String, DirectIntegrationError> {
    let local_ref = format!("refs/heads/{run_branch}");
    if ref_exists(repo, &local_ref)? {
        return Ok(run_branch.to_string());
    }

    let remote_ref = format!("refs/remotes/{REMOTE_NAME}/{run_branch}");
    if ref_exists(repo, &remote_ref)? {
        return Ok(format!("{REMOTE_NAME}/{run_branch}"));
    }

    run_git(
        repo,
        "fetch source branch",
        ["fetch", REMOTE_NAME, run_branch],
    )?;
    if ref_exists(repo, &remote_ref)? {
        return Ok(format!("{REMOTE_NAME}/{run_branch}"));
    }

    Err(DirectIntegrationError::Git {
        step: "resolve source branch".to_string(),
        repo: repo.to_path_buf(),
        message: format!("source branch `{run_branch}` is missing locally and on `{REMOTE_NAME}`"),
    })
}

fn ref_exists(repo: &Path, reference: &str) -> Result<bool, DirectIntegrationError> {
    let status = git_exit_status(
        repo,
        ["show-ref", "--verify", "--quiet", reference],
        "show-ref --verify --quiet",
    )?;
    Ok(status == 0)
}

fn ref_oid(repo: &Path, reference: &str) -> Result<String, DirectIntegrationError> {
    let output = run_git(repo, "rev-parse ref", ["rev-parse", reference])?;
    Ok(output.stdout.trim().to_string())
}

fn is_ancestor(
    repo: &Path,
    ancestor: &str,
    descendant: &str,
) -> Result<bool, DirectIntegrationError> {
    let status = git_exit_status(
        repo,
        ["merge-base", "--is-ancestor", ancestor, descendant],
        "merge-base --is-ancestor",
    )?;
    Ok(status == 0)
}

fn create_temp_worktree_dir() -> Result<TempWorktreeDir, DirectIntegrationError> {
    let path =
        std::env::temp_dir().join(format!("fabro-direct-integration-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&path).map_err(|error| DirectIntegrationError::Tempdir {
        message: error.to_string(),
    })?;
    Ok(TempWorktreeDir { path })
}

fn git_exit_status(
    repo: &Path,
    args: impl IntoIterator<Item = impl AsRef<str>>,
    step: &str,
) -> Result<i32, DirectIntegrationError> {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args.into_iter().map(|value| value.as_ref().to_string()))
        .output()
        .map_err(|error| DirectIntegrationError::Git {
            step: step.to_string(),
            repo: repo.to_path_buf(),
            message: error.to_string(),
        })?;
    Ok(output.status.code().unwrap_or(1))
}

fn run_git(
    repo: &Path,
    step: &str,
    args: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<GitOutput, DirectIntegrationError> {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args.into_iter().map(|value| value.as_ref().to_string()))
        .output()
        .map_err(|error| DirectIntegrationError::Git {
            step: step.to_string(),
            repo: repo.to_path_buf(),
            message: error.to_string(),
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DirectIntegrationError::Git {
            step: step.to_string(),
            repo: repo.to_path_buf(),
            message: stderr.trim().to_string(),
        });
    }

    Ok(GitOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
    })
}

fn head_sha(repo: &Path) -> Result<String, DirectIntegrationError> {
    let output = run_git(repo, "rev-parse HEAD", ["rev-parse", "HEAD"])?;
    Ok(output.stdout.trim().to_string())
}

fn write_integration_artifact(
    path: &Path,
    record: &DirectIntegrationRecord,
) -> Result<(), DirectIntegrationError> {
    let text = format!(
        "integrated: {integrated}\nmode: {mode}\nsource_lane: {source_lane}\nsource_run_id: {source_run_id}\nsource_run_branch: {source_run_branch}\ntarget_branch: {target_branch}\ncommit_sha: {commit_sha}\nalready_integrated: {already_integrated}\npushed: {pushed}\nintegrated_at: {integrated_at}\n",
        integrated = yes_no(record.integrated),
        mode = record.mode,
        source_lane = record.source_lane,
        source_run_id = record.source_run_id,
        source_run_branch = record.source_run_branch,
        target_branch = record.target_branch,
        commit_sha = record.commit_sha,
        already_integrated = yes_no(record.already_integrated),
        pushed = yes_no(record.pushed),
        integrated_at = record.integrated_at,
    );
    crate::write_text_atomic(path, &text, "integration artifact").map_err(|error| {
        DirectIntegrationError::WriteArtifact {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    })
}

fn sanitize_commit_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            ':' | '/' => '-',
            _ => ch,
        })
        .collect()
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn is_checked_out_branch_update_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("cannot force update the branch") && lower.contains("used by worktree")
}

struct IntegrationLock {
    path: PathBuf,
}

impl Drop for IntegrationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

struct TempWorktreeDir {
    path: PathBuf,
}

impl Drop for TempWorktreeDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct GitOutput {
    stdout: String,
}

#[cfg(test)]
mod tests {
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
    fn direct_integration_squash_merges_run_branch_into_trunk() {
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

        let artifact = temp.path().join("integration.md");
        let record = integrate_run(&DirectIntegrationRequest {
            source_lane: "demo:implement".to_string(),
            manifest: RunManifest {
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
            },
            repo_fallback: repo.clone(),
            target_branch: DEFAULT_TARGET_BRANCH.to_string(),
            strategy: MergeStrategy::Squash,
            artifact_path: Some(artifact.clone()),
        })
        .expect("direct integration succeeds");

        assert!(record.integrated);
        assert!(artifact.exists(), "integration artifact should be written");
        assert_eq!(
            git_output(&repo, &["show", "origin/main:feature.txt"]),
            "integrated"
        );
    }

    #[test]
    fn direct_integration_updates_local_branch_when_origin_is_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        fs::create_dir_all(&repo).expect("repo dir");
        git(&repo, &["init", "-b", "main"]);
        git(&repo, &["config", "user.name", "Test"]);
        git(&repo, &["config", "user.email", "test@example.com"]);
        fs::write(repo.join("README.md"), "base\n").expect("readme");
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "base"]);

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
                "worktree",
                "remove",
                "--force",
                worktree.to_str().expect("worktree path"),
            ],
        );
        git(&repo, &["checkout", "--detach"]);

        let record = integrate_run(&DirectIntegrationRequest {
            source_lane: "demo:implement".to_string(),
            manifest: RunManifest {
                run_id: "run-1".to_string(),
                workflow_name: "Demo".to_string(),
                goal: "Demo".to_string(),
                start_time: Utc::now(),
                node_count: 1,
                edge_count: 0,
                run_branch: Some("fabro/run/run-1".to_string()),
                base_sha: None,
                labels: std::collections::HashMap::new(),
                base_branch: Some("main".to_string()),
                workflow_slug: None,
                host_repo_path: Some(repo.display().to_string()),
            },
            repo_fallback: repo.clone(),
            target_branch: "main".to_string(),
            strategy: MergeStrategy::Squash,
            artifact_path: None,
        })
        .expect("direct integration succeeds");

        assert!(record.integrated);
        assert!(!record.pushed);
        assert_eq!(
            git_output(&repo, &["show", "main:feature.txt"]),
            "integrated"
        );
    }

    #[test]
    fn direct_integration_prefers_local_branch_when_remote_has_diverged() {
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
        git(&remote, &["symbolic-ref", "HEAD", "refs/heads/main"]);

        let other = temp.path().join("other");
        git(
            temp.path(),
            &[
                "clone",
                remote.to_str().expect("remote path"),
                other.to_str().expect("other path"),
            ],
        );
        git(&other, &["config", "user.name", "Test"]);
        git(&other, &["config", "user.email", "test@example.com"]);
        fs::write(other.join("remote.txt"), "remote\n").expect("remote file");
        git(&other, &["add", "remote.txt"]);
        git(&other, &["commit", "-m", "remote change"]);
        git(&other, &["push", "origin", "main"]);

        fs::write(repo.join("local.txt"), "local\n").expect("local file");
        git(&repo, &["add", "local.txt"]);
        git(&repo, &["commit", "-m", "local change"]);
        git(&repo, &["checkout", "--detach"]);

        git(&repo, &["branch", "fabro/run/run-1", "main"]);
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
                "worktree",
                "remove",
                "--force",
                worktree.to_str().expect("worktree path"),
            ],
        );

        let record = integrate_run(&DirectIntegrationRequest {
            source_lane: "demo:implement".to_string(),
            manifest: RunManifest {
                run_id: "run-1".to_string(),
                workflow_name: "Demo".to_string(),
                goal: "Demo".to_string(),
                start_time: Utc::now(),
                node_count: 1,
                edge_count: 0,
                run_branch: Some("fabro/run/run-1".to_string()),
                base_sha: None,
                labels: std::collections::HashMap::new(),
                base_branch: Some("main".to_string()),
                workflow_slug: None,
                host_repo_path: Some(repo.display().to_string()),
            },
            repo_fallback: repo.clone(),
            target_branch: DEFAULT_TARGET_BRANCH.to_string(),
            strategy: MergeStrategy::Squash,
            artifact_path: None,
        })
        .expect("direct integration succeeds");

        assert!(record.integrated);
        assert!(!record.pushed);
        assert_eq!(record.target_branch, "main");
        assert_eq!(
            git_output(&repo, &["show", "main:feature.txt"]),
            "integrated"
        );
        assert_eq!(git_output(&repo, &["show", "main:local.txt"]), "local");
        assert!(
            Command::new("git")
                .current_dir(&repo)
                .args(["show", "origin/main:feature.txt"])
                .output()
                .expect("git output")
                .status
                .code()
                .unwrap_or(1)
                != 0
        );
    }

    #[test]
    fn direct_integration_allows_local_branch_ref_pending_for_linked_worktree() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        fs::create_dir_all(&repo).expect("repo dir");
        git(&repo, &["init", "-b", "main"]);
        git(&repo, &["config", "user.name", "Test"]);
        git(&repo, &["config", "user.email", "test@example.com"]);
        fs::write(repo.join("README.md"), "base\n").expect("readme");
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "base"]);
        git(&repo, &["checkout", "--detach"]);

        let target = temp.path().join("target");
        git(
            &repo,
            &[
                "worktree",
                "add",
                target.to_str().expect("target path"),
                "main",
            ],
        );

        git(&repo, &["branch", "fabro/run/run-1", "HEAD"]);
        let run_worktree = temp.path().join("run-worktree");
        git(
            &repo,
            &[
                "worktree",
                "add",
                run_worktree.to_str().expect("worktree path"),
                "fabro/run/run-1",
            ],
        );
        fs::write(run_worktree.join("feature.txt"), "integrated\n").expect("feature");
        git(&run_worktree, &["add", "feature.txt"]);
        git(&run_worktree, &["commit", "-m", "feature"]);
        git(
            &repo,
            &[
                "worktree",
                "remove",
                "--force",
                run_worktree.to_str().expect("worktree path"),
            ],
        );

        let record = integrate_run(&DirectIntegrationRequest {
            source_lane: "demo:implement".to_string(),
            manifest: RunManifest {
                run_id: "run-1".to_string(),
                workflow_name: "Demo".to_string(),
                goal: "Demo".to_string(),
                start_time: Utc::now(),
                node_count: 1,
                edge_count: 0,
                run_branch: Some("fabro/run/run-1".to_string()),
                base_sha: None,
                labels: std::collections::HashMap::new(),
                base_branch: Some("main".to_string()),
                workflow_slug: None,
                host_repo_path: Some(target.display().to_string()),
            },
            repo_fallback: target.clone(),
            target_branch: "main".to_string(),
            strategy: MergeStrategy::Squash,
            artifact_path: None,
        })
        .expect("direct integration succeeds");

        assert!(record.integrated);
        assert_eq!(record.mode, "direct_trunk_squash_local_branch_pending");
        assert!(!record.pushed);
    }
}
