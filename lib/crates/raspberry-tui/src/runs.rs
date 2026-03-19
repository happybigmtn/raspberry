use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Utc};
use fabro_workflows::run_inspect::{finished_at, inspect_run_info, summarize_usage};
use fabro_workflows::run_lookup::{default_runs_base, scan_runs, RunInfo};
use fabro_workflows::run_status::RunStatus;
use raspberry_supervisor::{EvaluatedProgram, ProgramManifest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentLaneRun {
    pub run_id: String,
    pub workflow_name: String,
    pub status: RunStatus,
    pub finished_at: Option<DateTime<Utc>>,
    pub last_completed_stage_label: Option<String>,
    pub usage_summary: Option<String>,
    pub matched_files: Vec<String>,
}

pub fn build_recent_run_index(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    program: &EvaluatedProgram,
) -> BTreeMap<String, Vec<RecentLaneRun>> {
    build_recent_run_index_in_base(manifest_path, manifest, program, &default_runs_base())
}

pub fn build_recent_run_index_in_base(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    program: &EvaluatedProgram,
    runs_base: &Path,
) -> BTreeMap<String, Vec<RecentLaneRun>> {
    let targets = build_targets(manifest_path, manifest, program);
    if targets.is_empty() {
        return BTreeMap::new();
    }

    let runs = match scan_runs(runs_base) {
        Ok(runs) => runs,
        Err(_) => return BTreeMap::new(),
    };

    let mut matches = BTreeMap::new();
    for run in runs {
        if run.status != RunStatus::Succeeded {
            continue;
        }

        let written_paths = collect_written_paths(&run);
        if written_paths.is_empty() {
            continue;
        }

        let mut matched_lane_keys = Vec::new();
        for (lane_key, target) in &targets {
            let matched_files = target
                .artifacts
                .iter()
                .filter(|(_, path)| written_paths.contains(path))
                .map(|(artifact_id, _)| artifact_id.clone())
                .collect::<Vec<_>>();
            if matched_files.is_empty() {
                continue;
            }
            matched_lane_keys.push((lane_key.clone(), matched_files));
        }
        if matched_lane_keys.is_empty() {
            continue;
        }

        let Ok(inspection) = inspect_run_info(&run) else {
            continue;
        };
        let finished = finished_at(&inspection);
        let usage_summary = summarize_usage(inspection.progress.last_usage.as_ref());

        for (lane_key, matched_files) in matched_lane_keys {
            matches
                .entry(lane_key)
                .or_insert_with(Vec::new)
                .push(RecentLaneRun {
                    run_id: run.run_id.clone(),
                    workflow_name: run.workflow_name.clone(),
                    status: run.status,
                    finished_at: finished,
                    last_completed_stage_label: inspection
                        .progress
                        .last_completed_stage_label
                        .clone(),
                    usage_summary: usage_summary.clone(),
                    matched_files,
                });
        }
    }

    matches
}

fn build_targets(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    program: &EvaluatedProgram,
) -> BTreeMap<String, LaneTarget> {
    let mut targets = BTreeMap::new();
    for lane in &program.lanes {
        let artifacts = manifest
            .resolve_lane_artifacts(manifest_path, &lane.unit_id, &lane.lane_id)
            .into_iter()
            .map(|artifact| (artifact.id, normalize_path(&artifact.path)))
            .collect::<Vec<_>>();
        if artifacts.is_empty() {
            continue;
        }
        targets.insert(lane.lane_key.clone(), LaneTarget { artifacts });
    }
    targets
}

fn collect_written_paths(run: &RunInfo) -> BTreeSet<PathBuf> {
    let Some(host_repo_path) = run.host_repo_path.as_deref() else {
        return BTreeSet::new();
    };

    let progress_path = run.path.join("progress.jsonl");
    let Ok(contents) = std::fs::read_to_string(progress_path) else {
        return BTreeSet::new();
    };

    let mut written_paths = BTreeSet::new();
    let repo_root = Path::new(host_repo_path);
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        extend_logged_paths(&mut written_paths, repo_root, &value, "files_written");
        extend_logged_paths(&mut written_paths, repo_root, &value, "files_touched");
    }
    written_paths
}

fn extend_logged_paths(
    output: &mut BTreeSet<PathBuf>,
    repo_root: &Path,
    value: &serde_json::Value,
    key: &str,
) {
    let Some(paths) = value.get(key).and_then(|value| value.as_array()) else {
        return;
    };
    for path in paths {
        let Some(path) = path.as_str() else {
            continue;
        };
        output.insert(normalize_path(&repo_root.join(path)));
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

#[derive(Debug, Clone)]
struct LaneTarget {
    artifacts: Vec<(String, PathBuf)>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use fabro_workflows::manifest::Manifest as RunManifest;
    use fabro_workflows::run_status::RunStatusRecord;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn build_recent_run_index_matches_succeeded_runs_by_written_artifacts() {
        let fixture = TempFixture::create();

        let manifest = ProgramManifest::load(&fixture.manifest_path).expect("manifest loads");
        let program = raspberry_supervisor::evaluate_program(&fixture.manifest_path)
            .expect("program evaluates");

        let matches = build_recent_run_index_in_base(
            &fixture.manifest_path,
            &manifest,
            &program,
            &fixture.runs_base,
        );

        let oracle_runs = matches
            .get("validator:oracle")
            .expect("validator lane should have a recent run");
        assert_eq!(oracle_runs[0].run_id, "01TESTRECENTRUNMATCH0000000000");
        assert_eq!(oracle_runs[0].workflow_name, "BootstrapValidatorOracle");
        assert_eq!(
            oracle_runs[0].matched_files,
            vec!["spec".to_string(), "review".to_string()]
        );
    }

    struct TempFixture {
        _temp: TempDir,
        manifest_path: PathBuf,
        runs_base: PathBuf,
    }

    impl TempFixture {
        fn create() -> Self {
            let temp = tempfile::tempdir().expect("tempdir");
            let repo_root = temp.path().join("repo");
            let runs_base = temp.path().join("runs");
            std::fs::create_dir_all(repo_root.join("outputs/validator/oracle"))
                .expect("outputs dir");
            std::fs::create_dir_all(repo_root.join("run-configs")).expect("config dir");
            std::fs::write(
                repo_root.join("outputs/validator/oracle/spec.md"),
                "validator spec",
            )
            .expect("spec");
            std::fs::write(
                repo_root.join("outputs/validator/oracle/review.md"),
                "validator review",
            )
            .expect("review");

            let manifest_path = repo_root.join("program.yaml");
            std::fs::write(
                &manifest_path,
                indoc::indoc! {r#"
                    version: 1
                    program: demo
                    target_repo: .
                    state_path: .raspberry/program-state.json
                    units:
                      - id: validator
                        title: Validator Oracle
                        output_root: outputs/validator/oracle
                        artifacts:
                          - id: spec
                            path: spec.md
                          - id: review
                            path: review.md
                        milestones:
                          - id: reviewed
                            requires: [spec, review]
                        lanes:
                          - id: oracle
                            kind: service
                            title: Oracle
                            run_config: run-configs/oracle.toml
                            managed_milestone: reviewed
                            produces: [spec, review]
                "#},
            )
            .expect("manifest");

            let run_dir = runs_base.join("20260319-01TESTRECENTRUNMATCH0000000000");
            std::fs::create_dir_all(&run_dir).expect("run dir");
            RunManifest {
                run_id: "01TESTRECENTRUNMATCH0000000000".to_string(),
                workflow_name: "BootstrapValidatorOracle".to_string(),
                goal: "Bootstrap the Myosu `validator:oracle` lane.".to_string(),
                start_time: Utc::now(),
                node_count: 3,
                edge_count: 2,
                run_branch: None,
                base_sha: None,
                labels: HashMap::new(),
                base_branch: None,
                workflow_slug: Some("services".to_string()),
                host_repo_path: Some(repo_root.display().to_string()),
            }
            .save(&run_dir.join("manifest.json"))
            .expect("run manifest");
            RunStatusRecord::new(RunStatus::Succeeded, None)
                .save(&run_dir.join("status.json"))
                .expect("status");
            std::fs::write(
                run_dir.join("progress.jsonl"),
                indoc::indoc! {r#"
                    {"ts":"2026-03-19T06:39:36Z","run_id":"01TESTRECENTRUNMATCH0000000000","event":"StageCompleted","node_label":"Inventory","duration_ms":1000,"files_written":["outputs/validator/oracle/spec.md","outputs/validator/oracle/review.md"],"usage":{"model":"gpt-5.4","input_tokens":100,"output_tokens":80}}
                    {"ts":"2026-03-19T06:39:37Z","run_id":"01TESTRECENTRUNMATCH0000000000","event":"WorkflowRunCompleted","duration_ms":1200,"status":"success","usage":{"model":"gpt-5.4","input_tokens":100,"output_tokens":80}}
                "#},
            )
            .expect("progress");

            Self {
                _temp: temp,
                manifest_path,
                runs_base,
            }
        }
    }
}
