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
const STALE_LOCK_SECS: u64 = 300;
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
    /// Shell command to run after merge+commit but before push.
    /// If it exits non-zero, the integration is rejected and the lane
    /// will be re-dispatched against a fresh trunk checkout.
    pub post_merge_check: Option<String>,
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
    let merge_output = match run_git(
        worktree_path,
        "merge --squash",
        ["merge", "--squash", "--no-commit", source_ref],
    ) {
        Ok(output) => output,
        Err(DirectIntegrationError::Git { message, .. }) if is_merge_conflict_message(&message) => {
            resolve_lane_owned_output_conflicts(request, worktree_path, source_ref, base_ref)?
        }
        Err(error) => return Err(error),
    };
    // Strip agent debris from staging area before committing.
    // Agents sometimes write junk files (heredoc artifacts, stray .md) to root.
    strip_agent_debris_from_staging(worktree_path);
    // Strip unrelated generated package and evidence churn from ordinary
    // product settlement commits so integrate(<lane>) diffs stay reviewable.
    strip_structural_noise_from_staging(request, worktree_path);

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

    // Post-merge compilation guard: catch broken code before it reaches trunk.
    // Each lane validates in isolation, but the squash-merged result may fail
    // when combined with other concurrent integrations.
    if !already_integrated {
        if let Some(check_cmd) = &request.post_merge_check {
            let check_output = Command::new("sh")
                .arg("-c")
                .arg(check_cmd)
                .current_dir(worktree_path)
                .output();
            match check_output {
                Ok(output) if !output.status.success() => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(DirectIntegrationError::Git {
                        step: "post-merge check".to_string(),
                        repo: worktree_path.to_path_buf(),
                        message: format!(
                            "post-merge check failed (exit {}): {}",
                            output.status.code().unwrap_or(-1),
                            stderr.chars().take(500).collect::<String>()
                        ),
                    });
                }
                Err(io_err) => {
                    tracing::warn!(
                        check_cmd,
                        error = %io_err,
                        "post-merge check command could not be spawned, skipping"
                    );
                }
                Ok(_) => {}
            }
        }
    }

    let (mode, pushed) = if base_ref.starts_with(&format!("{REMOTE_NAME}/")) {
        let push_refspec = format!("HEAD:refs/heads/{target_branch}");
        let push_url =
            crate::git::resolve_ssh_push_url(worktree_path, REMOTE_NAME).map_err(|error| {
                DirectIntegrationError::Git {
                    step: "resolve ssh push url".to_string(),
                    repo: worktree_path.to_path_buf(),
                    message: error.to_string(),
                }
            })?;
        run_git(
            worktree_path,
            "push",
            ["push", push_url.as_str(), push_refspec.as_str()],
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

fn resolve_lane_owned_output_conflicts(
    request: &DirectIntegrationRequest,
    worktree_path: &Path,
    source_ref: &str,
    base_ref: &str,
) -> Result<GitOutput, DirectIntegrationError> {
    let conflicted = conflicted_paths(worktree_path)?;
    if conflicted.is_empty() {
        return Err(DirectIntegrationError::Git {
            step: "merge --squash".to_string(),
            repo: worktree_path.to_path_buf(),
            message: "merge reported conflict but no conflicted paths were found".to_string(),
        });
    }

    let owned = lane_owned_output_paths(request);

    // Resolve each conflict: owned paths take the source (run's version),
    // non-owned paths take the target (trunk version).  Non-owned conflicts
    // arise because the run's worktree was forked from an older trunk — the
    // stale copies of files modified by other lanes cause merge conflicts
    // that are safe to resolve by keeping the current trunk version.
    let mut source_resolved = Vec::new();
    let mut target_resolved = Vec::new();
    for path in &conflicted {
        if owned.contains(path) {
            run_git(
                worktree_path,
                "checkout source artifact",
                [
                    "checkout",
                    source_ref,
                    "--",
                    path.to_string_lossy().as_ref(),
                ],
            )?;
            source_resolved.push(path.display().to_string());
        } else {
            run_git(
                worktree_path,
                "checkout target for non-owned",
                ["checkout", base_ref, "--", path.to_string_lossy().as_ref()],
            )?;
            target_resolved.push(path.display().to_string());
        }
        run_git(
            worktree_path,
            "add resolved",
            ["add", path.to_string_lossy().as_ref()],
        )?;
    }

    let mut summary = String::new();
    if !source_resolved.is_empty() {
        summary.push_str(&format!(
            "auto-resolved owned conflicts (source): {}\n",
            source_resolved.join(", ")
        ));
    }
    if !target_resolved.is_empty() {
        summary.push_str(&format!(
            "auto-resolved non-owned conflicts (target): {}\n",
            target_resolved.join(", ")
        ));
    }

    Ok(GitOutput { stdout: summary })
}

/// Remove known agent debris from the git staging area so junk never lands on trunk.
/// Patterns: heredoc artifacts (EOF, ENDFILE, etc.), stray root-level .md that aren't
/// project docs, single-character files, leaked lane directories, and .fabro-work/.
fn strip_agent_debris_from_staging(worktree: &Path) {
    let staged = Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(worktree)
        .output();
    let Ok(output) = staged else { return };
    if !output.status.success() {
        return;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let junk: Vec<&str> = stdout
        .lines()
        .filter(|path| {
            let path = path.trim();
            if path.is_empty() {
                return false;
            }
            // .fabro-work/ is ephemeral, never commit
            if path.starts_with(".fabro-work/") {
                return true;
            }
            // Only check root-level files (no '/' in path)
            if path.contains('/') {
                return false;
            }
            // Known heredoc/shell artifacts
            let known_junk = ["EOF", "ENDFILE", "ENDOFFILE", "REVIEW_EOF", "ENDOFPROMPT"];
            if known_junk.contains(&path) {
                return true;
            }
            // Single-character files at root (e.g., "1", "0")
            if path.len() == 1 && path.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return true;
            }
            // Root-level ephemeral workflow .md files that should be in outputs/ or .fabro-work/
            let ephemeral_md = [
                "quality.md",
                "promotion.md",
                "verification.md",
                "deep-review-findings.md",
                "escalation-verdict.md",
                "integration.md",
                "task_plan.md",
            ];
            if ephemeral_md.contains(&path) {
                return true;
            }
            false
        })
        .collect();

    if junk.is_empty() {
        return;
    }
    tracing::info!(count = junk.len(), "stripping agent debris from staging");
    let mut args = vec!["rm", "--cached", "--ignore-unmatch", "--force", "--"];
    args.extend(junk);
    let _ = Command::new("git")
        .args(&args)
        .current_dir(worktree)
        .output();
}

fn strip_structural_noise_from_staging(request: &DirectIntegrationRequest, worktree: &Path) {
    let staged = Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(worktree)
        .output();
    let Ok(output) = staged else { return };
    if !output.status.success() {
        return;
    }

    let allow_generated_package = lane_allows_generated_package_paths(&request.source_lane);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stripped: Vec<&str> = stdout
        .lines()
        .filter(|path| {
            let path = path.trim();
            if path.is_empty() {
                return false;
            }
            if is_evidence_artifact_path(path) {
                return true;
            }
            if !allow_generated_package && is_generated_package_path(path) {
                return true;
            }
            false
        })
        .collect();

    if stripped.is_empty() {
        return;
    }

    tracing::info!(
        source_lane = request.source_lane,
        count = stripped.len(),
        "stripping structural noise from settlement staging"
    );
    let mut args = vec!["rm", "--cached", "--ignore-unmatch", "--force", "--"];
    args.extend(stripped.iter().copied());
    let _ = Command::new("git")
        .args(&args)
        .current_dir(worktree)
        .output();
}

fn lane_allows_generated_package_paths(source_lane: &str) -> bool {
    [
        "blueprint-pipeline",
        "plan-level",
        "program-synthesis",
        "package",
        "workflow",
        "prompt",
        "run-config",
    ]
    .iter()
    .any(|needle| source_lane.contains(needle))
}

fn is_generated_package_path(path: &str) -> bool {
    [
        ".raspberry/",
        "malinka/programs/",
        "malinka/workflows/",
        "malinka/run-configs/",
        "malinka/prompts/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
        || path == "integration.md"
}

fn is_evidence_artifact_path(path: &str) -> bool {
    if !path.starts_with("outputs/") {
        return false;
    }
    let Some(file_name) = Path::new(path).file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    matches!(
        file_name,
        "spec.md" | "review.md" | "verification.md" | "quality.md" | "promotion.md"
    )
}

fn conflicted_paths(repo: &Path) -> Result<Vec<PathBuf>, DirectIntegrationError> {
    let output = run_git(
        repo,
        "diff --name-only --diff-filter=U",
        ["diff", "--name-only", "--diff-filter=U"],
    )?;
    let mut paths = output
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn lane_owned_output_paths(request: &DirectIntegrationRequest) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = &request.artifact_path {
        paths.push(path.clone());
        if let Some(parent) = path.parent() {
            paths.push(parent.join("spec.md"));
            paths.push(parent.join("review.md"));
        }
    }
    let lane = request.source_lane.replace(':', "-");
    let output_root = PathBuf::from("outputs").join(&lane);
    paths.push(output_root.join("spec.md"));
    paths.push(output_root.join("review.md"));
    paths.sort();
    paths.dedup();
    paths
}

fn is_merge_conflict_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("merge conflict")
        || lower.contains("automatic merge failed")
        || lower.contains("recorded preimage for")
        || lower.contains("could not apply")
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
                // Remove stale locks left by crashed processes
                if let Ok(meta) = fs::metadata(&lock_path) {
                    let is_stale = meta
                        .modified()
                        .ok()
                        .and_then(|m| m.elapsed().ok())
                        .is_some_and(|age| age >= Duration::from_secs(STALE_LOCK_SECS));
                    if is_stale {
                        let _ = fs::remove_file(&lock_path);
                        continue;
                    }
                }
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
            post_merge_check: None,
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
            post_merge_check: None,
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
            post_merge_check: None,
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
            post_merge_check: None,
        })
        .expect("direct integration succeeds");

        assert!(record.integrated);
        assert_eq!(record.mode, "direct_trunk_squash_local_branch_pending");
        assert!(!record.pushed);
    }

    #[test]
    fn direct_integration_auto_resolves_lane_owned_output_conflicts() {
        let temp = tempfile::tempdir().expect("tempdir");
        let remote = temp.path().join("remote.git");
        git(
            temp.path(),
            &["init", "--bare", remote.to_str().expect("remote path")],
        );

        let repo = temp.path().join("repo");
        fs::create_dir_all(repo.join("outputs/build-fix-ci")).expect("repo dir");
        git(&repo, &["init", "-b", "main"]);
        git(&repo, &["config", "user.name", "Test"]);
        git(&repo, &["config", "user.email", "test@example.com"]);
        fs::write(repo.join("outputs/build-fix-ci/spec.md"), "trunk spec\n").expect("spec");
        fs::write(
            repo.join("outputs/build-fix-ci/review.md"),
            "trunk review\n",
        )
        .expect("review");
        git(
            &repo,
            &[
                "add",
                "outputs/build-fix-ci/spec.md",
                "outputs/build-fix-ci/review.md",
            ],
        );
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

        fs::write(
            repo.join("outputs/build-fix-ci/spec.md"),
            "new trunk spec\n",
        )
        .expect("update trunk spec");
        fs::write(
            repo.join("outputs/build-fix-ci/review.md"),
            "new trunk review\n",
        )
        .expect("update trunk review");
        git(
            &repo,
            &[
                "add",
                "outputs/build-fix-ci/spec.md",
                "outputs/build-fix-ci/review.md",
            ],
        );
        git(&repo, &["commit", "-m", "trunk update"]);
        git(&repo, &["push", "origin", "main"]);

        git(&repo, &["branch", "fabro/run/run-1", "HEAD~1"]);
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
        fs::write(worktree.join("outputs/build-fix-ci/spec.md"), "run spec\n").expect("run spec");
        fs::write(
            worktree.join("outputs/build-fix-ci/review.md"),
            "run review\n",
        )
        .expect("run review");
        git(
            &worktree,
            &[
                "add",
                "outputs/build-fix-ci/spec.md",
                "outputs/build-fix-ci/review.md",
            ],
        );
        git(&worktree, &["commit", "-m", "run output"]);
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

        let record = integrate_run(&DirectIntegrationRequest {
            source_lane: "build-fix-ci".to_string(),
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
                workflow_slug: Some("build-fix-ci".to_string()),
                host_repo_path: Some(repo.display().to_string()),
            },
            repo_fallback: repo.clone(),
            target_branch: DEFAULT_TARGET_BRANCH.to_string(),
            strategy: MergeStrategy::Squash,
            artifact_path: None,
            post_merge_check: None,
        })
        .expect("owned output conflict should auto-resolve");

        assert!(record.integrated);
        assert_eq!(
            git_output(&repo, &["show", "origin/main:outputs/build-fix-ci/spec.md"]),
            "run spec"
        );
        assert_eq!(
            git_output(
                &repo,
                &["show", "origin/main:outputs/build-fix-ci/review.md"]
            ),
            "run review"
        );
    }

    #[test]
    fn direct_integration_strips_generated_package_and_evidence_noise() {
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
        fs::write(repo.join("LOCAL_ONLY.md"), "local branch tip\n").expect("local only");
        git(&repo, &["add", "LOCAL_ONLY.md"]);
        git(&repo, &["commit", "-m", "local branch ahead"]);
        git(&repo, &["branch", "trunk"]);

        git(&repo, &["branch", "fabro/run/run-2", "HEAD"]);
        let worktree = temp.path().join("run-worktree");
        git(
            &repo,
            &[
                "worktree",
                "add",
                worktree.to_str().expect("worktree path"),
                "fabro/run/run-2",
            ],
        );
        fs::write(worktree.join("feature.txt"), "integrated\n").expect("feature");
        fs::create_dir_all(worktree.join("malinka/prompts/demo")).expect("prompt dir");
        fs::write(
            worktree.join("malinka/prompts/demo/plan.md"),
            "generated prompt\n",
        )
        .expect("prompt");
        fs::create_dir_all(worktree.join("outputs/demo")).expect("outputs dir");
        fs::write(worktree.join("outputs/demo/review.md"), "evidence\n").expect("review");
        fs::write(worktree.join("integration.md"), "root artifact\n").expect("integration");
        git(&worktree, &["add", "."]);
        git(&worktree, &["commit", "-m", "feature with noise"]);
        git(
            &repo,
            &[
                "push",
                "origin",
                "fabro/run/run-2:refs/heads/fabro/run/run-2",
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

        let record = integrate_run(&DirectIntegrationRequest {
            source_lane: "demo:implement".to_string(),
            manifest: RunManifest {
                run_id: "run-2".to_string(),
                workflow_name: "Demo".to_string(),
                goal: "Demo".to_string(),
                start_time: Utc::now(),
                node_count: 1,
                edge_count: 0,
                run_branch: Some("fabro/run/run-2".to_string()),
                base_sha: None,
                labels: std::collections::HashMap::new(),
                base_branch: Some("trunk".to_string()),
                workflow_slug: Some("demo".to_string()),
                host_repo_path: Some(repo.display().to_string()),
            },
            repo_fallback: repo.clone(),
            target_branch: "trunk".to_string(),
            strategy: MergeStrategy::Squash,
            artifact_path: None,
            post_merge_check: None,
        })
        .expect("integration succeeds");

        assert!(record.integrated);
        assert_eq!(
            git_output(&repo, &["show", "trunk:feature.txt"]),
            "integrated"
        );
        let ls_tree = git_output(&repo, &["ls-tree", "-r", "--name-only", "trunk"]);
        assert!(!ls_tree.contains("malinka/prompts/demo/plan.md"));
        assert!(!ls_tree.contains("outputs/demo/review.md"));
        assert!(!ls_tree.lines().any(|line| line == "integration.md"));
    }
}
