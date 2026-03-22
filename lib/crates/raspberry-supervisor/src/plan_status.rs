use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use fabro_config::run::{load_run_config, resolve_graph_path};
use serde::Deserialize;
use thiserror::Error;

use crate::evaluate::{
    evaluate_program_local, EvaluateError, EvaluatedLane, EvaluatedProgram, LaneExecutionStatus,
};
use crate::manifest::{ManifestError, ProgramManifest};
use crate::plan_registry::{load_plan_registry, PlanCategory, PlanRecord, PlanRegistryError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanMatrix {
    pub program: String,
    pub rows: Vec<PlanStatusRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanStatusRow {
    pub plan_id: String,
    pub plan_file: String,
    pub title: String,
    pub category: String,
    pub composite: bool,
    pub mapping_status: String,
    pub child_count: usize,
    pub represented_in_blueprint: bool,
    pub has_bootstrap_lane: bool,
    pub has_implementation_lane: bool,
    pub has_real_verify_gate: bool,
    pub current_status: String,
    pub current_risk: String,
    pub next_operator_move: String,
}

#[derive(Debug, Error)]
pub enum PlanStatusError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error(transparent)]
    Evaluate(#[from] EvaluateError),
    #[error(transparent)]
    Registry(#[from] PlanRegistryError),
    #[error("failed to read blueprint {path}: {source}")]
    ReadBlueprint {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse blueprint {path}: {source}")]
    ParseBlueprint {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
}

#[derive(Debug, Deserialize)]
struct StoredBlueprint {
    #[serde(default)]
    units: Vec<StoredBlueprintUnit>,
}

#[derive(Debug, Deserialize)]
struct StoredBlueprintUnit {
    id: String,
}

pub fn load_plan_matrix(manifest_path: &Path) -> Result<PlanMatrix, PlanStatusError> {
    let manifest = ProgramManifest::load(manifest_path)?;
    let program = evaluate_program_local(manifest_path)?;
    let target_repo = manifest.resolved_target_repo(manifest_path);
    let represented_units = load_blueprint_unit_ids(manifest_path, &manifest)?;
    let registry = load_plan_registry(&target_repo)?;

    let rows = registry
        .plans
        .into_iter()
        .map(|plan| build_status_row(&program, &represented_units, plan))
        .collect();

    Ok(PlanMatrix {
        program: program.program,
        rows,
    })
}

pub fn render_plan_matrix(matrix: &PlanMatrix) -> String {
    let mut lines = vec![
        format!("Program: {}", matrix.program),
        "plan file | plan id | mapping | children | represented? | bootstrap? | implementation? | real proof? | status | risk | next move".to_string(),
    ];
    for row in &matrix.rows {
        lines.push(format!(
            "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {}",
            row.plan_file,
            row.plan_id,
            row.mapping_status,
            row.child_count,
            yes_no(row.represented_in_blueprint),
            yes_no(row.has_bootstrap_lane),
            yes_no(row.has_implementation_lane),
            yes_no(row.has_real_verify_gate),
            row.current_status,
            row.current_risk,
            row.next_operator_move
        ));
    }
    lines.join("\n")
}

fn build_status_row(
    program: &EvaluatedProgram,
    represented_units: &BTreeMap<String, ()>,
    plan: PlanRecord,
) -> PlanStatusRow {
    let mapping_status = mapping_status(&plan).to_string();
    let child_count = child_count(&plan);
    let represented = represented_units.contains_key(&plan.plan_id)
        || represented_units.contains_key(&format!("{}-implementation", plan.plan_id));
    let bootstrap_lane = find_bootstrap_lane(program, &plan.plan_id);
    let implementation_lane = find_implementation_lane(program, &plan.plan_id);
    let has_real_verify_gate = implementation_lane
        .and_then(lane_verify_gate_is_real)
        .unwrap_or(false);
    let (current_status, current_risk, next_operator_move) = summarize_plan_status(
        &plan,
        represented,
        bootstrap_lane,
        implementation_lane,
        has_real_verify_gate,
    );

    PlanStatusRow {
        plan_id: plan.plan_id,
        plan_file: plan.path.display().to_string(),
        title: plan.title,
        category: plan.category.as_str().to_string(),
        composite: plan.composite,
        mapping_status,
        child_count,
        represented_in_blueprint: represented,
        has_bootstrap_lane: bootstrap_lane.is_some(),
        has_implementation_lane: implementation_lane.is_some(),
        has_real_verify_gate,
        current_status,
        current_risk,
        next_operator_move,
    }
}

fn mapping_status(plan: &PlanRecord) -> &'static str {
    let _ = plan;
    "mapped"
}

fn child_count(plan: &PlanRecord) -> usize {
    if !plan.declared_child_ids.is_empty() {
        return plan.declared_child_ids.len();
    }
    usize::from(plan.implementation_required)
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn load_blueprint_unit_ids(
    manifest_path: &Path,
    manifest: &ProgramManifest,
) -> Result<BTreeMap<String, ()>, PlanStatusError> {
    let blueprint_path = manifest
        .resolved_target_repo(manifest_path)
        .join("fabro")
        .join("blueprints")
        .join(format!("{}.yaml", manifest.program));
    if !blueprint_path.exists() {
        return Ok(BTreeMap::new());
    }
    let raw = std::fs::read_to_string(&blueprint_path).map_err(|source| {
        PlanStatusError::ReadBlueprint {
            path: blueprint_path.clone(),
            source,
        }
    })?;
    let blueprint: StoredBlueprint =
        serde_yaml::from_str(&raw).map_err(|source| PlanStatusError::ParseBlueprint {
            path: blueprint_path,
            source,
        })?;
    Ok(blueprint
        .units
        .into_iter()
        .map(|unit| (unit.id, ()))
        .collect())
}

fn find_bootstrap_lane<'a>(
    program: &'a EvaluatedProgram,
    plan_id: &str,
) -> Option<&'a EvaluatedLane> {
    program
        .lanes
        .iter()
        .find(|lane| lane.unit_id == plan_id && lane.lane_id == plan_id)
}

fn find_implementation_lane<'a>(
    program: &'a EvaluatedProgram,
    plan_id: &str,
) -> Option<&'a EvaluatedLane> {
    program
        .lanes
        .iter()
        .find(|lane| lane.unit_id == format!("{plan_id}-implementation"))
}

fn lane_verify_gate_is_real(lane: &EvaluatedLane) -> Option<bool> {
    let run_config = load_run_config(&lane.run_config).ok()?;
    let workflow_path = resolve_graph_path(&lane.run_config, &run_config.graph);
    let workflow = std::fs::read_to_string(workflow_path).ok()?;
    let verify_pos = workflow.find("verify ")?;
    let snippet = &workflow[verify_pos..workflow.len().min(verify_pos + 1200)];
    let script_start = snippet.find("script=\"")? + "script=\"".len();
    let script_rest = &snippet[script_start..];
    let script_end = script_rest.find('"')?;
    let script = &script_rest[..script_end];
    let normalized = script.replace("\\n", "\n").trim().to_ascii_lowercase();
    if normalized == "true" {
        return Some(false);
    }
    let only_artifacts = normalized.lines().all(|line| {
        let trimmed = line.trim();
        trimmed.is_empty()
            || trimmed == "set -e"
            || trimmed == "set +e"
            || trimmed.starts_with("test -f ")
            || trimmed.starts_with("grep -eq ")
    });
    Some(!only_artifacts)
}

fn summarize_plan_status(
    plan: &PlanRecord,
    represented: bool,
    bootstrap_lane: Option<&EvaluatedLane>,
    implementation_lane: Option<&EvaluatedLane>,
    has_real_verify_gate: bool,
) -> (String, String, String) {
    if plan.category == PlanCategory::Meta {
        return (
            "meta_plan".to_string(),
            "portfolio coordination plan; inspect child plan rows".to_string(),
            "inspect downstream numbered plans".to_string(),
        );
    }

    if !represented {
        let risk = if plan.mapping_contract_path.is_some() {
            "mapping exists but synthesis has not rendered this plan yet".to_string()
        } else if plan.composite {
            "mapped from plan structure, but synthesis has not rendered this composite plan yet"
                .to_string()
        } else {
            "evidence only; no executable frontier synthesized".to_string()
        };
        return (
            "unmodeled".to_string(),
            risk,
            "extend synthesis coverage for this plan".to_string(),
        );
    }

    if let Some(lane) = bootstrap_lane {
        if lane.status != LaneExecutionStatus::Complete {
            return (
                format!("bootstrap_{}", lane.status),
                lane.detail.clone(),
                next_move_for_lane("bootstrap", lane.status, true).to_string(),
            );
        }
    }

    if let Some(lane) = implementation_lane {
        let risk = if has_real_verify_gate {
            lane.detail.clone()
        } else {
            "implementation lane exists but verify gate is not yet real".to_string()
        };
        let next_move = if has_real_verify_gate {
            next_move_for_lane("implementation", lane.status, false).to_string()
        } else {
            "replace placeholder verify gate with a real proof contract".to_string()
        };
        return (format!("implementation_{}", lane.status), risk, next_move);
    }

    if plan.implementation_required {
        return (
            "reviewed".to_string(),
            "plan is represented, but no implementation lane is synthesized".to_string(),
            "render implementation child from plan registry".to_string(),
        );
    }

    (
        "reviewed".to_string(),
        "plan is represented and requires no direct implementation lane".to_string(),
        "monitor child plans".to_string(),
    )
}

fn next_move_for_lane(scope: &str, status: LaneExecutionStatus, bootstrap: bool) -> &'static str {
    match status {
        LaneExecutionStatus::Ready => {
            if bootstrap {
                "dispatch bootstrap lane"
            } else {
                "dispatch implementation lane"
            }
        }
        LaneExecutionStatus::Running => "wait for active run to finish",
        LaneExecutionStatus::Blocked => "resolve blocker and reevaluate readiness",
        LaneExecutionStatus::Failed => {
            if scope == "bootstrap" {
                "repair bootstrap artifacts and rerun"
            } else {
                "repair failed implementation run"
            }
        }
        LaneExecutionStatus::Complete => "inspect proof artifacts and advance status",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn render_plan_matrix_includes_mapping_and_next_move_columns() {
        let matrix = PlanMatrix {
            program: "demo".to_string(),
            rows: vec![PlanStatusRow {
                plan_id: "craps".to_string(),
                plan_file: "plans/005-craps-game.md".to_string(),
                title: "Craps Game".to_string(),
                category: "game".to_string(),
                composite: true,
                mapping_status: "mapped".to_string(),
                child_count: 4,
                represented_in_blueprint: false,
                has_bootstrap_lane: false,
                has_implementation_lane: false,
                has_real_verify_gate: false,
                current_status: "unmodeled".to_string(),
                current_risk:
                    "mapped from plan structure, but synthesis has not rendered this composite plan yet"
                        .to_string(),
                next_operator_move: "extend synthesis coverage for this plan".to_string(),
            }],
        };

        let rendered = render_plan_matrix(&matrix);

        assert!(rendered.contains("plan file | plan id | mapping | children"));
        assert!(rendered.contains("plans/005-craps-game.md | craps | mapped | 4"));
        assert!(rendered.contains("extend synthesis coverage for this plan"));
    }

    #[test]
    fn load_plan_matrix_marks_composite_plan_as_mapped_even_without_contract() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path();
        fs::create_dir_all(repo.join("plans")).expect("plans dir");
        fs::create_dir_all(repo.join("malinka/programs")).expect("program dir");
        fs::create_dir_all(repo.join("malinka/blueprints")).expect("blueprints dir");
        fs::create_dir_all(repo.join("malinka/run-configs")).expect("run configs dir");

        fs::write(repo.join("plans/001-master-plan.md"), "# Master Plan\n").expect("master");
        fs::write(
            repo.join("plans/005-craps-game.md"),
            concat!(
                "# Craps Game\n\n",
                "- [ ] Milestone 1: casino-core\n",
                "- [ ] Milestone 2: provably-fair\n",
                "- [ ] Milestone 3: house\n",
            ),
        )
        .expect("craps");
        fs::write(
            repo.join("malinka/blueprints/demo.yaml"),
            concat!(
                "version: 1\n",
                "program: demo\n",
                "units:\n",
                "  - id: seed\n",
                "    title: Seed Unit\n",
                "    output_root: outputs/seed\n",
                "    artifacts:\n",
                "      - id: review\n",
                "        path: review.md\n",
                "    milestones:\n",
                "      - id: reviewed\n",
                "        requires: [review]\n",
                "    lanes: []\n",
            ),
        )
        .expect("blueprint");
        fs::write(
            repo.join("malinka/run-configs/seed.toml"),
            "# seed fixture run config\n",
        )
        .expect("run config");
        fs::write(
            repo.join("malinka/programs/demo.yaml"),
            concat!(
                "version: 1\n",
                "program: demo\n",
                "target_repo: ../..\n",
                "state_path: ../../.raspberry/demo-state.json\n",
                "max_parallel: 1\n",
                "units:\n",
                "  - id: seed\n",
                "    title: Seed Unit\n",
                "    output_root: ../../outputs/seed\n",
                "    artifacts:\n",
                "      - id: review\n",
                "        path: review.md\n",
                "    milestones:\n",
                "      - id: reviewed\n",
                "        requires: [review]\n",
                "    run_config: ../run-configs/seed.toml\n",
                "    managed_milestone: reviewed\n",
                "    lanes: []\n",
            ),
        )
        .expect("manifest");

        let matrix = load_plan_matrix(&repo.join("malinka/programs/demo.yaml")).expect("matrix");
        let craps = matrix
            .rows
            .iter()
            .find(|row| row.plan_id == "craps")
            .expect("craps row");

        assert_eq!(craps.mapping_status, "mapped");
        assert_eq!(craps.current_status, "unmodeled");
        assert_eq!(craps.child_count, 3);
        assert_eq!(
            craps.next_operator_move,
            "extend synthesis coverage for this plan"
        );
    }
}
