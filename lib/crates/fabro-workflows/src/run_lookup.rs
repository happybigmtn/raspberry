use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::run_status::{RunStatus, RunStatusRecord, StatusReason};

#[derive(Debug, Clone, Serialize)]
pub struct RunInfo {
    pub run_id: String,
    pub dir_name: String,
    pub workflow_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_slug: Option<String>,
    pub status: RunStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_reason: Option<StatusReason>,
    pub start_time: String,
    pub labels: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_cost: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_repo_path: Option<String>,
    pub goal: String,
    #[serde(skip)]
    pub start_time_dt: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub end_time: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub path: PathBuf,
    #[serde(skip)]
    pub is_orphan: bool,
}

pub fn default_data_dir() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join(".fabro")
}

pub fn default_logs_base() -> PathBuf {
    default_data_dir().join("logs")
}

pub fn default_runs_base() -> PathBuf {
    default_data_dir().join("runs")
}

pub fn scan_runs(base: &Path) -> Result<Vec<RunInfo>> {
    let entries = match std::fs::read_dir(base) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };

    let mut runs = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();
        let manifest_path = path.join("manifest.json");

        if let Ok(manifest) = crate::manifest::Manifest::load(&manifest_path) {
            let run_id = manifest.run_id;
            let workflow_name = manifest.workflow_name;
            let workflow_slug = manifest.workflow_slug;
            let host_repo_path = manifest.host_repo_path;
            let goal = manifest.goal;
            let start_time_dt = manifest.start_time;
            let start_time = start_time_dt.to_rfc3339();
            let labels = manifest.labels;
            let status_info = read_status(&path);

            runs.push(RunInfo {
                run_id,
                dir_name,
                workflow_name,
                workflow_slug,
                status: status_info.status,
                status_reason: status_info.reason,
                start_time,
                labels,
                duration_ms: status_info.duration_ms,
                total_cost: status_info.total_cost,
                host_repo_path,
                start_time_dt: Some(start_time_dt),
                end_time: status_info.end_time,
                path,
                goal,
                is_orphan: false,
            });
        } else {
            let mtime_dt = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|time| -> DateTime<Utc> { time.into() });
            let mtime = mtime_dt.map(|dt| dt.to_rfc3339()).unwrap_or_default();

            let run_id = std::fs::read_to_string(path.join("id.txt"))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| dir_name.clone());

            let status_info = read_status(&path);
            let is_orphan = matches!(status_info.status, RunStatus::Dead);
            runs.push(RunInfo {
                run_id,
                dir_name,
                workflow_name: if is_orphan {
                    "[no manifest]"
                } else {
                    "[starting]"
                }
                .to_string(),
                workflow_slug: None,
                status: status_info.status,
                status_reason: status_info.reason,
                start_time: mtime,
                labels: HashMap::new(),
                duration_ms: status_info.duration_ms,
                total_cost: status_info.total_cost,
                host_repo_path: None,
                start_time_dt: mtime_dt,
                end_time: status_info.end_time,
                path,
                goal: String::new(),
                is_orphan,
            });
        }
    }

    runs.sort_by(|a, b| b.start_time.cmp(&a.start_time));
    Ok(runs)
}

struct StatusInfo {
    status: RunStatus,
    reason: Option<StatusReason>,
    end_time: Option<DateTime<Utc>>,
    duration_ms: Option<u64>,
    total_cost: Option<f64>,
}

impl StatusInfo {
    fn simple(status: RunStatus) -> Self {
        Self {
            status,
            reason: None,
            end_time: None,
            duration_ms: None,
            total_cost: None,
        }
    }
}

fn read_status(run_dir: &Path) -> StatusInfo {
    if let Ok(record) = RunStatusRecord::load(&run_dir.join("status.json")) {
        if record.status.is_terminal() {
            if let Ok(conclusion) =
                crate::conclusion::Conclusion::load(&run_dir.join("conclusion.json"))
            {
                return StatusInfo {
                    status: record.status,
                    reason: record.reason,
                    end_time: Some(conclusion.timestamp),
                    duration_ms: Some(conclusion.duration_ms),
                    total_cost: conclusion.total_cost,
                };
            }
        }
        return StatusInfo {
            status: record.status,
            reason: record.reason,
            end_time: None,
            duration_ms: None,
            total_cost: None,
        };
    }

    StatusInfo::simple(RunStatus::Dead)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    RunningOnly,
    All,
}

pub fn filter_runs(
    runs: &[RunInfo],
    before: Option<&str>,
    workflow: Option<&str>,
    labels: &[(String, String)],
    include_orphans: bool,
    status_filter: StatusFilter,
) -> Vec<RunInfo> {
    runs.iter()
        .filter(|run| {
            if status_filter == StatusFilter::RunningOnly && !run.status.is_active() {
                return false;
            }
            if run.is_orphan && !include_orphans {
                return false;
            }
            if let Some(before) = before {
                if !run.start_time.is_empty() && run.start_time.as_str() >= before {
                    return false;
                }
            }
            if let Some(pattern) = workflow {
                if !run.workflow_name.contains(pattern) {
                    return false;
                }
            }
            for (key, value) in labels {
                match run.labels.get(key) {
                    Some(current) if current == value => {}
                    _ => return false,
                }
            }
            true
        })
        .cloned()
        .collect()
}

pub fn find_run_by_prefix(base: &Path, prefix: &str) -> Result<PathBuf> {
    let runs = scan_runs(base).context("Failed to scan runs")?;
    let matches: Vec<_> = runs
        .iter()
        .filter(|run| run_matches_prefix(run, prefix))
        .collect();

    match matches.len() {
        0 => bail!("No run found matching prefix '{prefix}'"),
        1 => Ok(matches[0].path.clone()),
        count => {
            let ids: Vec<String> = matches
                .iter()
                .map(|run| describe_run_identity(run))
                .collect();
            bail!(
                "Ambiguous prefix '{prefix}': {count} runs match: {}",
                ids.join(", ")
            )
        }
    }
}

pub fn resolve_run(base: &Path, identifier: &str) -> Result<RunInfo> {
    let runs = scan_runs(base).context("Failed to scan runs")?;

    let id_matches: Vec<_> = runs
        .iter()
        .filter(|run| run_matches_prefix(run, identifier))
        .collect();

    match id_matches.len() {
        1 => return Ok(id_matches[0].clone()),
        count if count > 1 => {
            let ids: Vec<String> = id_matches
                .iter()
                .map(|run| describe_run_identity(run))
                .collect();
            bail!(
                "Ambiguous prefix '{identifier}': {count} runs match: {}",
                ids.join(", ")
            )
        }
        _ => {}
    }

    let id_lower = identifier.to_lowercase();
    let id_collapsed = collapse_separators(&id_lower);
    let workflow_match = runs.iter().filter(|run| !run.is_orphan).find(|run| {
        if let Some(slug) = &run.workflow_slug {
            if slug.to_lowercase() == id_lower {
                return true;
            }
        }
        let name_lower = run.workflow_name.to_lowercase();
        name_lower.contains(&id_lower) || collapse_separators(&name_lower).contains(&id_collapsed)
    });

    match workflow_match {
        Some(run) => Ok(run.clone()),
        None => {
            bail!("No run found matching '{identifier}' (tried run ID prefix and workflow name)")
        }
    }
}

fn collapse_separators(s: &str) -> String {
    s.chars().filter(|c| *c != '-' && *c != '_').collect()
}

fn run_matches_prefix(run: &RunInfo, prefix: &str) -> bool {
    run.run_id.starts_with(prefix) || run.dir_name.starts_with(prefix)
}

fn describe_run_identity(run: &RunInfo) -> String {
    if run.dir_name == run.run_id {
        run.run_id.clone()
    } else {
        format!("{} ({})", run.dir_name, run.run_id)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;

    use super::*;
    use crate::manifest::Manifest;

    fn write_manifest_run(base: &Path, dir_name: &str, run_id: &str) -> PathBuf {
        let run_dir = base.join(dir_name);
        std::fs::create_dir_all(&run_dir).expect("create run dir");
        Manifest {
            run_id: run_id.to_string(),
            workflow_name: "cash-court".to_string(),
            goal: "prove a lane".to_string(),
            start_time: Utc::now(),
            node_count: 1,
            edge_count: 0,
            run_branch: None,
            base_sha: None,
            labels: HashMap::new(),
            base_branch: None,
            workflow_slug: Some("cash-court".to_string()),
            host_repo_path: None,
        }
        .save(&run_dir.join("manifest.json"))
        .expect("save manifest");
        run_dir
    }

    #[test]
    fn find_run_by_prefix_matches_directory_prefix_when_manifest_run_id_is_unprefixed() {
        let temp = tempfile::tempdir().expect("tempdir");
        let run_dir = write_manifest_run(
            temp.path(),
            "20260322-01KMC50SPP97BFFJKFNMNYW07G",
            "01KMC50SPP97BFFJKFNMNYW07G",
        );

        let resolved = find_run_by_prefix(temp.path(), "20260322-01KMC50SPP97")
            .expect("resolve by directory prefix");

        assert_eq!(resolved, run_dir);
    }

    #[test]
    fn resolve_run_matches_prefixed_directory_name() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_manifest_run(
            temp.path(),
            "20260322-01KMC50SPP97BFFJKFNMNYW07G",
            "01KMC50SPP97BFFJKFNMNYW07G",
        );

        let run = resolve_run(temp.path(), "20260322-01KMC50SPP97BFFJKFNMNYW07G")
            .expect("resolve by full directory name");

        assert_eq!(run.run_id, "01KMC50SPP97BFFJKFNMNYW07G");
        assert_eq!(run.dir_name, "20260322-01KMC50SPP97BFFJKFNMNYW07G");
    }
}
