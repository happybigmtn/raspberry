use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use fabro_model::{automation_fallback_map, automation_primary_target, AutomationProfile};
use git2::Repository;
use raspberry_supervisor::manifest::{LaneCheck, LaneCheckProbe};
use serde::Serialize;
use serde_json::Value;

use crate::blueprint::{
    validate_blueprint, BlueprintLane, BlueprintUnit, ProgramBlueprint, WorkflowTemplate,
};
use crate::error::RenderError;

#[derive(Debug, Clone, Copy)]
pub struct RenderRequest<'a> {
    pub blueprint: &'a ProgramBlueprint,
    pub target_repo: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct ImportRequest<'a> {
    pub target_repo: &'a Path,
    pub program: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ReconcileRequest<'a> {
    pub blueprint: &'a ProgramBlueprint,
    pub current_repo: &'a Path,
    pub output_repo: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderReport {
    pub written_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileReport {
    pub findings: Vec<String>,
    pub recommendations: Vec<String>,
    pub written_files: Vec<PathBuf>,
}

pub fn render_blueprint(req: RenderRequest<'_>) -> Result<RenderReport, RenderError> {
    validate_blueprint(Path::new("<render>"), req.blueprint)?;
    let layout = PackageLayout::new(req.blueprint, req.target_repo);
    let mut written_files = Vec::new();
    if layout.manifest_path().exists() {
        if let Ok(current) = crate::blueprint::import_existing_package(ImportRequest {
            target_repo: req.target_repo,
            program: &req.blueprint.program.id,
        }) {
            remove_obsolete_lane_files(&current, req.blueprint, req.target_repo)?;
        }
    }

    for unit in &req.blueprint.units {
        for lane in &unit.lanes {
            let files = render_lane(req.blueprint, &layout, &unit.id, lane)?;
            written_files.extend(files);
        }
    }

    written_files.push(write_manifest(req.blueprint, &layout)?);
    Ok(RenderReport { written_files })
}

pub fn cleanup_obsolete_package_files(
    previous: &ProgramBlueprint,
    desired: &ProgramBlueprint,
    target_repo: &Path,
) -> Result<(), RenderError> {
    remove_obsolete_lane_files(previous, desired, target_repo)
}

pub fn reconcile_blueprint(req: ReconcileRequest<'_>) -> Result<ReconcileReport, RenderError> {
    let current = crate::blueprint::import_existing_package(ImportRequest {
        target_repo: req.current_repo,
        program: &req.blueprint.program.id,
    })?;
    let evolved = refine_blueprint_from_evidence(req.blueprint, req.current_repo);
    let evolved = augment_with_implementation_follow_on_units(evolved, req.current_repo)?;

    let mut findings = diff_blueprints(&current, &evolved);
    findings.extend(input_findings(&evolved, req.current_repo));
    findings.extend(doctrine_evidence_support_findings(
        &current,
        &evolved,
        req.current_repo,
    ));
    findings.extend(current_runtime_findings(&current, req.current_repo));
    findings.extend(desired_artifact_findings(&evolved, req.current_repo));
    findings.extend(desired_execution_findings(&evolved, req.current_repo));
    findings.extend(review_artifact_findings(&evolved, req.current_repo));
    findings.extend(blocked_review_requirement_findings(
        &evolved,
        req.current_repo,
    ));

    let recommendations = evolve_recommendations(&evolved, req.current_repo, &findings);
    let mut report = render_evolved_blueprint(&evolved, &current, req.output_repo)?;
    report
        .written_files
        .extend(render_implementation_follow_ons(
            &evolved,
            req.current_repo,
            req.output_repo,
        )?);
    Ok(ReconcileReport {
        findings,
        recommendations,
        written_files: report.written_files,
    })
}

fn augment_with_implementation_follow_on_units(
    mut blueprint: ProgramBlueprint,
    target_repo: &Path,
) -> Result<ProgramBlueprint, RenderError> {
    let candidates = implementation_candidates(&blueprint, target_repo);
    if candidates.is_empty() {
        return Ok(blueprint);
    }

    let known_unit_ids = blueprint
        .units
        .iter()
        .map(|unit| unit.id.clone())
        .collect::<BTreeSet<_>>();

    for candidate in candidates {
        let dependency_milestone =
            source_lane_managed_milestone(&blueprint, &candidate.unit_id, &candidate.lane_id);
        let manifest_relative = candidate
            .program_manifest
            .strip_prefix(target_repo)
            .map(PathBuf::from)
            .map_err(|_| {
                RenderError::Blueprint(crate::error::BlueprintError::Invalid {
                    path: candidate.program_manifest.clone(),
                    message: format!(
                        "implementation program manifest `{}` is not inside target repo `{}`",
                        candidate.program_manifest.display(),
                        target_repo.display()
                    ),
                })
            })?;
        let unit_id = implementation_follow_on_unit_id(&candidate);
        if known_unit_ids.contains(&unit_id)
            || blueprint.units.iter().any(|unit| unit.id == unit_id)
        {
            continue;
        }
        blueprint.units.push(BlueprintUnit {
            id: unit_id,
            title: implementation_follow_on_title(&candidate),
            output_root: PathBuf::from(format!(
                ".raspberry/portfolio/{}",
                implementation_follow_on_slug(&candidate)
            )),
            artifacts: Vec::new(),
            milestones: Vec::new(),
            lanes: vec![BlueprintLane {
                id: "program".to_string(),
                kind: raspberry_supervisor::manifest::LaneKind::Orchestration,
                title: format!("{} Program", implementation_follow_on_title(&candidate)),
                family: "program".to_string(),
                workflow_family: None,
                slug: Some(
                    manifest_relative
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("implementation")
                        .to_string(),
                ),
                template: WorkflowTemplate::Orchestration,
                goal: format!(
                    "Coordinate the implementation follow-on program for `{}`.",
                    candidate.lane_key
                ),
                managed_milestone: "coordinated".to_string(),
                dependencies: vec![raspberry_supervisor::manifest::LaneDependency {
                    unit: candidate.unit_id.clone(),
                    lane: None,
                    milestone: Some(dependency_milestone),
                }],
                produces: Vec::new(),
                proof_profile: None,
                proof_state_path: None,
                program_manifest: Some(manifest_relative),
                service_state_path: None,
                orchestration_state_path: None,
                checks: Vec::new(),
                run_dir: None,
                prompt_context: None,
                verify_command: None,
                health_command: None,
            }],
        });
    }

    Ok(blueprint)
}

fn refine_blueprint_from_evidence(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
) -> ProgramBlueprint {
    let mut evolved = blueprint.clone();
    let catalog = lane_catalog(target_repo);
    let known_refs = known_lane_refs(target_repo);

    for unit_index in 0..evolved.units.len() {
        let lane_count = evolved.units[unit_index].lanes.len();
        for lane_index in 0..lane_count {
            let lane_key_text = {
                let unit = &evolved.units[unit_index];
                let lane = &unit.lanes[lane_index];
                lane_key(&unit.id, &lane.id)
            };
            let review_path = {
                let unit = &evolved.units[unit_index];
                let lane = &unit.lanes[lane_index];
                lane_review_artifact_path(unit, lane, target_repo)
            };
            let Some(review_path) = review_path else {
                continue;
            };
            let Ok(contents) = std::fs::read_to_string(&review_path) else {
                continue;
            };
            let lower = contents.to_lowercase();
            if !review_says_implementation_blocked(&lower) {
                continue;
            }
            let blockers = blocked_review_refs(&lower, &known_refs, &lane_key_text);
            if blockers.is_empty() {
                continue;
            }

            let lane = &mut evolved.units[unit_index].lanes[lane_index];
            apply_blocker_contract_tightening(lane, blueprint, &catalog, &blockers);
        }
    }

    augment_with_discovered_program_manifests(&mut evolved, target_repo);

    evolved
}

fn augment_with_discovered_program_manifests(blueprint: &mut ProgramBlueprint, target_repo: &Path) {
    if !blueprint
        .units
        .iter()
        .flat_map(|unit| unit.lanes.iter())
        .any(|lane| lane.program_manifest.is_some())
    {
        return;
    }

    let known_program_manifests = blueprint
        .units
        .iter()
        .flat_map(|unit| unit.lanes.iter())
        .filter_map(|lane| lane.program_manifest.as_ref().cloned())
        .collect::<BTreeSet<_>>();
    let programs_dir = target_repo
        .join(crate::blueprint::DEFAULT_PACKAGE_DIR)
        .join("programs");
    let Ok(entries) = std::fs::read_dir(&programs_dir) else {
        return;
    };

    let mut additions = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let relative = PathBuf::from("fabro").join("programs").join(name);
        if known_program_manifests.contains(&relative) {
            continue;
        }
        let Ok(manifest) = raspberry_supervisor::ProgramManifest::load(&path) else {
            continue;
        };
        if manifest.program == blueprint.program.id {
            continue;
        }
        let unit_id = manifest
            .program
            .strip_prefix("myosu-")
            .unwrap_or(manifest.program.as_str())
            .to_string();
        if blueprint.units.iter().any(|unit| unit.id == unit_id)
            || additions
                .iter()
                .any(|unit: &BlueprintUnit| unit.id == unit_id)
        {
            continue;
        }
        additions.push(BlueprintUnit {
            id: unit_id.clone(),
            title: discovered_program_title(&manifest.program),
            output_root: PathBuf::from(format!(".raspberry/portfolio/{unit_id}")),
            artifacts: Vec::new(),
            milestones: Vec::new(),
            lanes: vec![BlueprintLane {
                id: "program".to_string(),
                kind: raspberry_supervisor::manifest::LaneKind::Orchestration,
                title: format!("{} Program", discovered_program_title(&manifest.program)),
                family: "program".to_string(),
                workflow_family: None,
                slug: Some(manifest.program.clone()),
                template: WorkflowTemplate::Orchestration,
                goal: format!("Coordinate the child program `{}`.", manifest.program),
                managed_milestone: "coordinated".to_string(),
                dependencies: Vec::new(),
                produces: Vec::new(),
                proof_profile: None,
                proof_state_path: None,
                program_manifest: Some(relative),
                service_state_path: None,
                orchestration_state_path: None,
                checks: Vec::new(),
                run_dir: None,
                prompt_context: None,
                verify_command: None,
                health_command: None,
            }],
        });
    }

    additions.sort_by(|left, right| left.id.cmp(&right.id));
    blueprint.units.extend(additions);
}

fn discovered_program_title(program: &str) -> String {
    let trimmed = program.strip_prefix("myosu-").unwrap_or(program);
    let words = trimmed
        .split('-')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!("{}{}", first.to_uppercase(), chars.as_str())
        })
        .collect::<Vec<_>>();
    if words.is_empty() {
        "Program".to_string()
    } else {
        words.join(" ")
    }
}

fn render_evolved_blueprint(
    desired: &ProgramBlueprint,
    current: &ProgramBlueprint,
    output_repo: &Path,
) -> Result<RenderReport, RenderError> {
    validate_blueprint(Path::new("<render>"), desired)?;
    let layout = PackageLayout::new(desired, output_repo);
    let mut written_files = Vec::new();
    let current_units = current
        .units
        .iter()
        .map(|unit| (&unit.id, unit))
        .collect::<BTreeMap<_, _>>();

    for unit in &desired.units {
        let current_unit = current_units.get(&unit.id);
        for lane in &unit.lanes {
            let current_lane = current_unit
                .and_then(|unit| unit.lanes.iter().find(|candidate| candidate.id == lane.id));
            if let Some(current_lane) = current_lane {
                if lane_equivalent(current_lane, lane) {
                    let files = render_lane(desired, &layout, &unit.id, lane)?;
                    written_files.extend(files);
                    continue;
                }
            }

            let files = render_lane(desired, &layout, &unit.id, lane)?;
            written_files.extend(files);
        }
    }

    written_files.push(write_manifest(desired, &layout)?);
    Ok(RenderReport { written_files })
}

fn remove_obsolete_lane_files(
    current: &ProgramBlueprint,
    desired: &ProgramBlueprint,
    target_repo: &Path,
) -> Result<(), RenderError> {
    let current_layout = PackageLayout::new(current, target_repo);
    let desired_units = desired
        .units
        .iter()
        .map(|unit| (&unit.id, unit))
        .collect::<BTreeMap<_, _>>();

    for current_unit in &current.units {
        let desired_unit = desired_units.get(&current_unit.id);
        for current_lane in &current_unit.lanes {
            let desired_lane = desired_unit.and_then(|unit| {
                unit.lanes
                    .iter()
                    .find(|candidate| candidate.id == current_lane.id)
            });
            let keep = desired_lane.is_some_and(|desired_lane| {
                current_lane.family == desired_lane.family
                    && current_lane.workflow_family() == desired_lane.workflow_family()
                    && current_lane.slug() == desired_lane.slug()
                    && current_lane.template == desired_lane.template
            });
            if keep || current_lane.program_manifest.is_some() {
                continue;
            }
            remove_file_if_exists(&current_layout.run_config_path(current_lane))?;
            remove_file_if_exists(&current_layout.workflow_path(current_lane))?;
            remove_dir_if_exists(&current_layout.prompt_dir(current_lane))?;
        }
    }

    Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<(), RenderError> {
    if !path.exists() {
        return Ok(());
    }
    std::fs::remove_file(path).map_err(|source| RenderError::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn remove_dir_if_exists(path: &Path) -> Result<(), RenderError> {
    if !path.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(path).map_err(|source| RenderError::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn render_implementation_follow_ons(
    blueprint: &ProgramBlueprint,
    current_repo: &Path,
    output_repo: &Path,
) -> Result<Vec<PathBuf>, RenderError> {
    let mut written_files = Vec::new();
    for candidate in implementation_candidates(blueprint, current_repo) {
        let implementation_blueprint =
            implementation_blueprint_for_candidate(blueprint, &candidate, current_repo)?;
        let report = render_blueprint(RenderRequest {
            blueprint: &implementation_blueprint,
            target_repo: output_repo,
        })?;
        written_files.extend(report.written_files);
    }
    Ok(written_files)
}

fn render_lane(
    blueprint: &ProgramBlueprint,
    layout: &PackageLayout<'_>,
    unit_id: &str,
    lane: &BlueprintLane,
) -> Result<Vec<PathBuf>, RenderError> {
    if lane.program_manifest.is_some() {
        return Ok(Vec::new());
    }
    let run_config_path = layout.run_config_path(lane);
    let workflow_path = layout.workflow_path(lane);
    let prompt_dir = layout.prompt_dir(lane);
    let verify_command = lane
        .verify_command
        .clone()
        .unwrap_or_else(|| default_verify_command(blueprint, unit_id, lane));
    let health_command = lane
        .health_command
        .clone()
        .unwrap_or_else(|| "true".to_string());
    let promotion_command = implementation_promotion_contract_command(blueprint, unit_id, lane);
    let audit_command = implementation_audit_command(blueprint, unit_id, lane, &promotion_command);
    let quality_command = implementation_quality_command(blueprint, unit_id, lane);

    let graph = render_workflow_graph(
        lane,
        &verify_command,
        &health_command,
        &audit_command,
        &quality_command,
    );
    let integration_artifact_path = if lane.template == WorkflowTemplate::Implementation {
        lane_named_artifact_path_relative(blueprint, unit_id, lane, "integration")
    } else {
        None
    };
    let run_config = render_run_config(
        lane,
        integration_artifact_path.as_deref(),
        layout.target_repo,
    );
    let mut written_files = Vec::new();
    write_file(&workflow_path, &graph, &mut written_files)?;
    write_file(&run_config_path, &run_config, &mut written_files)?;
    if lane.template != WorkflowTemplate::Integration {
        let plan_prompt = render_prompt("plan", lane);
        let review_prompt = render_prompt("review", lane);
        let challenge_prompt = render_prompt("challenge", lane);
        let polish_prompt = render_prompt("polish", lane);
        write_file(
            &prompt_dir.join("plan.md"),
            &plan_prompt,
            &mut written_files,
        )?;
        write_file(
            &prompt_dir.join("review.md"),
            &review_prompt,
            &mut written_files,
        )?;
        if lane.template == WorkflowTemplate::Implementation {
            write_file(
                &prompt_dir.join("challenge.md"),
                &challenge_prompt,
                &mut written_files,
            )?;
        }
        write_file(
            &prompt_dir.join("polish.md"),
            &polish_prompt,
            &mut written_files,
        )?;
        if lane.template == WorkflowTemplate::Implementation {
            remove_file_if_exists(&prompt_dir.join("promote.md"))?;
        }
    }
    Ok(written_files)
}

fn write_manifest(
    blueprint: &ProgramBlueprint,
    layout: &PackageLayout<'_>,
) -> Result<PathBuf, RenderError> {
    let manifest_path = layout.manifest_path();
    let manifest = ManifestOut::from_blueprint(blueprint);
    let yaml =
        serde_yaml::to_string(&manifest).map_err(|source| RenderError::ManifestSerialize {
            path: manifest_path.clone(),
            source,
        })?;
    let trimmed = yaml.trim_start_matches("---\n");
    ensure_parent(&manifest_path)?;
    fabro_workflows::write_text_atomic(&manifest_path, trimmed, "manifest").map_err(|source| {
        RenderError::Write {
            path: manifest_path.clone(),
            source: std::io::Error::other(source.to_string()),
        }
    })?;
    Ok(manifest_path)
}

fn render_workflow_graph(
    lane: &BlueprintLane,
    verify_command: &str,
    health_command: &str,
    audit_command: &str,
    quality_command: &str,
) -> String {
    let write_target = automation_primary_target(AutomationProfile::Write);
    let review_target = automation_primary_target(AutomationProfile::Review);
    let prompt_path = |name: &str| -> String {
        format!(
            "@../../prompts/{}/{}/{}.md",
            lane.workflow_family(),
            lane.slug(),
            name
        )
    };
    let goal = escape_graph_attr(&lane.goal);

    match lane.template {
        WorkflowTemplate::Bootstrap | WorkflowTemplate::RecurringReport => format!(
            "digraph {} {{\n    graph [\n        goal=\"{}\",\n        model_stylesheet=\"\n            *       {{ backend: cli; }}\n            #review {{ backend: cli; model: {}; provider: {}; }}\n            #polish {{ backend: cli; model: {}; provider: {}; }}\n        \"\n    ]\n    rankdir=LR\n\n    start [shape=Mdiamond, label=\"Start\"]\n    exit  [shape=Msquare, label=\"Exit\"]\n\n    specify [label=\"Specify\", prompt=\"{}\", reasoning_effort=\"high\"]\n    review  [label=\"Review\", prompt=\"{}\", reasoning_effort=\"high\"]\n    polish  [label=\"Polish\", prompt=\"{}\", reasoning_effort=\"medium\"]\n    verify  [label=\"Verify\", shape=parallelogram, script=\"{}\", goal_gate=true, retry_target=\"polish\", max_retries=0]\n\n    start -> specify -> review -> polish -> verify\n    verify -> exit [condition=\"outcome=success\"]\n    verify -> polish\n}}\n",
            graph_name(lane),
            goal,
            review_target.model,
            review_target.provider.as_str(),
            write_target.model,
            write_target.provider.as_str(),
            prompt_path("plan"),
            prompt_path("review"),
            prompt_path("polish"),
            escape_graph_attr(verify_command),
        ),
        WorkflowTemplate::ServiceBootstrap => format!(
            "digraph {} {{\n    graph [\n        goal=\"{}\",\n        model_stylesheet=\"\n            *       {{ backend: cli; }}\n            #review {{ backend: cli; model: {}; provider: {}; }}\n            #polish {{ backend: cli; model: {}; provider: {}; }}\n        \"\n    ]\n    rankdir=LR\n\n    start [shape=Mdiamond, label=\"Start\"]\n    exit  [shape=Msquare, label=\"Exit\"]\n\n    inventory [label=\"Inventory\", prompt=\"{}\", reasoning_effort=\"high\"]\n    review    [label=\"Review\", prompt=\"{}\", reasoning_effort=\"high\"]\n    polish    [label=\"Polish\", prompt=\"{}\", reasoning_effort=\"medium\"]\n    verify_outputs [label=\"Verify Outputs\", shape=parallelogram, script=\"{}\", goal_gate=true, retry_target=\"polish\", max_retries=0]\n\n    start -> inventory -> review -> polish -> verify_outputs\n    verify_outputs -> exit [condition=\"outcome=success\"]\n    verify_outputs -> polish\n}}\n",
            graph_name(lane),
            goal,
            review_target.model,
            review_target.provider.as_str(),
            write_target.model,
            write_target.provider.as_str(),
            prompt_path("plan"),
            prompt_path("review"),
            prompt_path("polish"),
            escape_graph_attr(verify_command),
        ),
        WorkflowTemplate::Implementation => {
            let profile = lane.proof_profile.as_deref().unwrap_or("standard");
            let max_visits = profile_max_visits(profile);
            let service_health = lane.kind == raspberry_supervisor::manifest::LaneKind::Service
                && health_command != "true";
            let health_node = if service_health {
                format!(
                    "    health [label=\"Health\", shape=parallelogram, script=\"{}\", goal_gate=true, retry_target=\"fixup\"]\n",
                    escape_graph_attr(health_command)
                )
            } else {
                String::new()
            };
            let success_edges = if service_health {
                "    verify -> health [condition=\"outcome=success\"]\n    health -> quality [condition=\"outcome=success\"]\n    health -> fixup\n"
            } else {
                "    verify -> quality [condition=\"outcome=success\"]\n"
            };

            // Profile-specific extra nodes and edges
            let (extra_nodes, extra_edges) = profile_extra_graph_elements(
                profile,
                &prompt_path,
                verify_command,
                health_command,
            );
            let fallback_attr = profile_fallback_retry_target(profile);

            format!(
            "digraph {} {{\n    graph [\n        goal=\"{}\",{}\n        model_stylesheet=\"\n            *            {{ backend: cli; }}\n            #challenge   {{ backend: cli; model: {}; provider: {}; }}\n            #review      {{ backend: cli; model: {}; provider: {}; }}\n            #deep_review {{ backend: cli; model: {}; provider: {}; }}\n            #escalation  {{ backend: cli; model: {}; provider: {}; }}\n        \"\n    ]\n    rankdir=LR\n\n    start [shape=Mdiamond, label=\"Start\"]\n    exit  [shape=Msquare, label=\"Exit\"]\n\n    preflight [label=\"Preflight\", shape=parallelogram, script=\"{}\", max_retries=0]\n    implement [label=\"Implement\", prompt=\"{}\", reasoning_effort=\"high\"]\n    verify [label=\"Verify\", shape=parallelogram, script=\"{}\", goal_gate=true, retry_target=\"fixup\"]\n{}    quality [label=\"Quality Gate\", shape=parallelogram, script=\"{}\", goal_gate=true, retry_target=\"fixup\"]\n    fixup [label=\"Fixup\", prompt=\"{}\", reasoning_effort=\"high\", max_visits={}]\n    challenge [label=\"Challenge\", prompt=\"{}\", reasoning_effort=\"medium\"]\n    review [label=\"Review\", prompt=\"{}\", reasoning_effort=\"high\"]\n    audit [label=\"Audit Artifacts\", shape=parallelogram, script=\"{}\", goal_gate=true, retry_target=\"fixup\", max_retries=0]\n{}\n    start -> preflight -> implement -> verify\n{}    verify -> fixup\n    quality -> challenge [condition=\"outcome=success\"]\n    quality -> fixup\n    challenge -> review [condition=\"outcome=success\"]\n    challenge -> fixup\n{}    review -> audit [condition=\"outcome=success\"]\n    review -> fixup\n    audit -> exit [condition=\"outcome=success\"]\n    audit -> fixup\n    fixup -> verify\n}}\n",
                graph_name(lane),
                goal,
                fallback_attr,
                review_target.model,
                review_target.provider.as_str(),
                review_target.model,
                review_target.provider.as_str(),
                review_target.model,
                review_target.provider.as_str(),
                review_target.model,
                review_target.provider.as_str(),
                escape_graph_attr(&preflight_command(verify_command)),
                prompt_path("plan"),
                escape_graph_attr(verify_command),
                health_node,
                escape_graph_attr(quality_command),
                prompt_path("polish"),
                max_visits,
                prompt_path("challenge"),
                prompt_path("review"),
                escape_graph_attr(audit_command),
                extra_nodes,
                success_edges,
                extra_edges,
            )
        }
        WorkflowTemplate::Integration => format!(
            "digraph {} {{\n    graph [goal=\"{}\"]\n    rankdir=LR\n\n    start [shape=Mdiamond, label=\"Start\"]\n    unsupported [label=\"Supervisor Only\", shape=parallelogram, script=\"printf 'integration lanes are executed directly by raspberry supervisor\\n' >&2; exit 1\", max_retries=0]\n    exit [shape=Msquare, label=\"Exit\"]\n\n    start -> unsupported -> exit\n}}\n",
            graph_name(lane),
            goal,
        ),
        WorkflowTemplate::Orchestration => format!(
            "digraph {} {{\n    graph [goal=\"{}\"]\n    rankdir=LR\n\n    start [shape=Mdiamond, label=\"Start\"]\n    unsupported [label=\"Supervisor Orchestration\", shape=parallelogram, script=\"printf 'repo-level orchestration lanes are executed directly by raspberry supervisor\\n' >&2; exit 1\", max_retries=0]\n    exit [shape=Msquare, label=\"Exit\"]\n\n    start -> unsupported -> exit\n}}\n",
            graph_name(lane),
            goal,
        ),
    }
}

fn profile_max_visits(profile: &str) -> u32 {
    match profile {
        "hardened" => 5,
        "unblock" => 5,
        "foundation" => 4,
        _ => 3,
    }
}

fn profile_fallback_retry_target(profile: &str) -> String {
    match profile {
        "unblock" | "hardened" => "\n        fallback_retry_target=\"deep_review\",".to_string(),
        _ => String::new(),
    }
}

fn profile_extra_graph_elements(
    profile: &str,
    _prompt_path: &dyn Fn(&str) -> String,
    verify_command: &str,
    _health_command: &str,
) -> (String, String) {
    match profile {
        "hardened" => {
            // Adversarial deep review for security + correctness critical work
            let nodes = format!(
                "    deep_review [label=\"Deep Review\", prompt=\"You are an adversarial reviewer. Challenge every trust boundary, invariant, edge case, and correctness assumption. Re-run proof commands independently. Write deep-review-findings.md.\", reasoning_effort=\"high\"]\n    recheck [label=\"Recheck\", shape=parallelogram, script=\"{verify}\", goal_gate=true, retry_target=\"fixup\"]\n",
                verify = escape_graph_attr(verify_command),
            );
            let edges = "    challenge -> deep_review [condition=\"outcome=success\"]\n    deep_review -> recheck [condition=\"outcome=success\"]\n    deep_review -> fixup\n    recheck -> review [condition=\"outcome=success\"]\n    recheck -> fixup\n".to_string();
            (nodes, edges)
        }
        "foundation" => {
            let nodes =
                "    escalation [label=\"Opus Signoff\", prompt=\"This child modifies shared foundation code. Review for downstream compatibility. Approve only if backward-compatible or all consumers updated. Write escalation-verdict.md.\", reasoning_effort=\"high\"]\n"
                .to_string();
            let edges = "    challenge -> escalation [condition=\"outcome=success\"]\n    escalation -> review [condition=\"outcome=success\"]\n    escalation -> fixup\n".to_string();
            (nodes, edges)
        }
        "ux" => {
            let nodes = "    acceptance_gate [label=\"Acceptance Gate\", shape=parallelogram, script=\"test -f acceptance-evidence.md && grep -q 'accepted: yes' acceptance-evidence.md\", goal_gate=true, retry_target=\"fixup\", max_retries=0]\n".to_string();
            let edges = "    review -> acceptance_gate [condition=\"outcome=success\"]\n    acceptance_gate -> audit [condition=\"outcome=success\"]\n    acceptance_gate -> fixup\n".to_string();
            (nodes, edges)
        }
        "unblock" => {
            // Extended retry budget for lanes known to hit pre-existing
            // blockers.  The fixup prompt already has authority to fix issues
            // outside the lane's surfaces.  This profile gives it more
            // attempts and adds a dedicated deep-review pass.
            let nodes = format!(
                "    deep_review [label=\"Deep Review\", prompt=\"The verify gate has failed repeatedly. Analyze the failure output, identify whether the root cause is inside or outside this lane's owned surfaces, and write a concrete fix plan to deep-review-findings.md. If the issue is pre-existing external code (linter warnings, dependency issues), explicitly instruct the fixup stage to fix it.\", reasoning_effort=\"high\"]\n    recheck [label=\"Recheck\", shape=parallelogram, script=\"{verify}\", goal_gate=true, retry_target=\"fixup\"]\n",
                verify = escape_graph_attr(verify_command),
            );
            let edges = "    challenge -> deep_review [condition=\"outcome=success\"]\n    deep_review -> recheck [condition=\"outcome=success\"]\n    deep_review -> fixup\n    recheck -> review [condition=\"outcome=success\"]\n    recheck -> fixup\n".to_string();
            (nodes, edges)
        }
        _ => (String::new(), String::new()),
    }
}

fn preflight_command(verify_command: &str) -> String {
    let trimmed = verify_command.trim_start();
    let body = if let Some(rest) = trimmed.strip_prefix("set -e\n") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("set -e\r\n") {
        rest
    } else {
        trimmed
    };
    format!("set +e\n{}\ntrue", body)
}

fn implementation_promotion_contract_command(
    blueprint: &ProgramBlueprint,
    unit_id: &str,
    lane: &BlueprintLane,
) -> String {
    let promotion_path = implementation_promotion_path(blueprint, unit_id, lane);
    format!(
        "grep -Eq '^merge_ready: yes$' {} && grep -Eq '^manual_proof_pending: no$' {} && grep -Eq '^reason: .+$' {} && grep -Eq '^next_action: .+$' {}",
        promotion_path.display(),
        promotion_path.display(),
        promotion_path.display(),
        promotion_path.display()
    )
}

fn implementation_quality_path(
    blueprint: &ProgramBlueprint,
    unit_id: &str,
    lane: &BlueprintLane,
) -> PathBuf {
    lane_artifact_paths_relative(blueprint, unit_id, lane)
        .into_iter()
        .find(|path| path.file_name().and_then(|name| name.to_str()) == Some("quality.md"))
        .unwrap_or_else(|| PathBuf::from("quality.md"))
}

fn implementation_quality_command(
    blueprint: &ProgramBlueprint,
    unit_id: &str,
    lane: &BlueprintLane,
) -> String {
    let Some(unit) = blueprint
        .units
        .iter()
        .find(|candidate| candidate.id == unit_id)
    else {
        return "true".to_string();
    };
    let quality_path = implementation_quality_path(blueprint, unit_id, lane);
    let implementation_path = join_relative(
        &unit.output_root,
        &lane_named_artifact_path(unit, lane, "implementation")
            .display()
            .to_string(),
    );
    let verification_path = join_relative(
        &unit.output_root,
        &lane_named_artifact_path(unit, lane, "verification")
            .display()
            .to_string(),
    );
    let touch_first = lane
        .prompt_context
        .as_deref()
        .map(|context| prompt_context_block(context, "Touch first:"))
        .unwrap_or_default();
    let touched_surfaces = touch_first
        .iter()
        .map(|line| normalize_prompt_path_item(line))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let touched_surface_section = if touched_surfaces.is_empty() {
        "- (none declared)\n".to_string()
    } else {
        touched_surfaces
            .iter()
            .map(|surface| format!("- {surface}\n"))
            .collect::<String>()
    };
    let mut surface_scan_lines = Vec::new();
    for surface in &touched_surfaces {
        surface_scan_lines.push(format!("scan_placeholder {}", shell_single_quote(surface)));
    }
    if surface_scan_lines.is_empty() {
        surface_scan_lines.push("true".to_string());
    }

    format!(
        "set -e\nQUALITY_PATH={quality_path}\nIMPLEMENTATION_PATH={implementation_path}\nVERIFICATION_PATH={verification_path}\nplaceholder_hits=\"\"\nscan_placeholder() {{\n  surface=\"$1\"\n  if [ ! -e \"$surface\" ]; then\n    return 0\n  fi\n  if [ -f \"$surface\" ]; then\n    surface=\"$(dirname \"$surface\")\"\n  fi\n  hits=\"$(rg -n -i -g '*.rs' -g '*.py' -g '*.js' -g '*.ts' -g '*.tsx' -g '*.md' -g 'Cargo.toml' -g '*.toml' 'TODO|stub|placeholder|not yet implemented|compile-only|for now|will implement|todo!|unimplemented!' \"$surface\" || true)\"\n  if [ -n \"$hits\" ]; then\n    if [ -n \"$placeholder_hits\" ]; then\n      placeholder_hits=\"$(printf '%s\\n%s' \"$placeholder_hits\" \"$hits\")\"\n    else\n      placeholder_hits=\"$hits\"\n    fi\n  fi\n}}\n{surface_scan}\nartifact_hits=\"$(rg -n -i 'manual proof still required|placeholder|stub implementation|not yet fully implemented|todo!|unimplemented!' \"$IMPLEMENTATION_PATH\" \"$VERIFICATION_PATH\" 2>/dev/null || true)\"\nwarning_hits=\"$(rg -n 'warning:' \"$IMPLEMENTATION_PATH\" \"$VERIFICATION_PATH\" 2>/dev/null || true)\"\nmanual_hits=\"$(rg -n -i 'manual proof still required|manual;' \"$VERIFICATION_PATH\" 2>/dev/null || true)\"\nplaceholder_debt=no\nwarning_debt=no\nartifact_mismatch_risk=no\nmanual_followup_required=no\n[ -n \"$placeholder_hits\" ] && placeholder_debt=yes\n[ -n \"$warning_hits\" ] && warning_debt=yes\n[ -n \"$artifact_hits\" ] && artifact_mismatch_risk=yes\n[ -n \"$manual_hits\" ] && manual_followup_required=yes\nquality_ready=yes\nif [ \"$placeholder_debt\" = yes ] || [ \"$warning_debt\" = yes ] || [ \"$artifact_mismatch_risk\" = yes ] || [ \"$manual_followup_required\" = yes ]; then\n  quality_ready=no\nfi\nmkdir -p \"$(dirname \"$QUALITY_PATH\")\"\ncat > \"$QUALITY_PATH\" <<EOF\nquality_ready: $quality_ready\nplaceholder_debt: $placeholder_debt\nwarning_debt: $warning_debt\nartifact_mismatch_risk: $artifact_mismatch_risk\nmanual_followup_required: $manual_followup_required\n\n## Touched Surfaces\n{touched_surface_section}\n## Placeholder Hits\n$placeholder_hits\n\n## Artifact Consistency Hits\n$artifact_hits\n\n## Warning Hits\n$warning_hits\n\n## Manual Followup Hits\n$manual_hits\nEOF\ntest \"$quality_ready\" = yes",
        quality_path = shell_single_quote(&quality_path.display().to_string()),
        implementation_path = shell_single_quote(&implementation_path.display().to_string()),
        verification_path = shell_single_quote(&verification_path.display().to_string()),
        surface_scan = surface_scan_lines.join("\n"),
        touched_surface_section = touched_surface_section,
    )
}

fn implementation_promotion_path(
    blueprint: &ProgramBlueprint,
    unit_id: &str,
    lane: &BlueprintLane,
) -> PathBuf {
    let promotion_path = lane_artifact_paths_relative(blueprint, unit_id, lane)
        .into_iter()
        .find(|path| path.file_name().and_then(|name| name.to_str()) == Some("promotion.md"))
        .unwrap_or_else(|| PathBuf::from("promotion.md"));
    promotion_path
}

fn normalize_prompt_path_item(line: &str) -> String {
    line.trim()
        .trim_start_matches("- ")
        .trim()
        .trim_matches('`')
        .trim()
        .to_string()
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

fn toml_multiline_literal(value: &str) -> String {
    format!("'''\n{}\n'''", value.trim_end())
}

fn template_supports_direct_integration(template: WorkflowTemplate) -> bool {
    matches!(
        template,
        WorkflowTemplate::Bootstrap
            | WorkflowTemplate::ServiceBootstrap
            | WorkflowTemplate::Implementation
    )
}

fn direct_integration_target_branch(target_repo: &Path) -> Option<String> {
    let Ok(repo) = Repository::discover(target_repo) else {
        return None;
    };
    if repo.find_remote("origin").is_ok() {
        return Some("origin/HEAD".to_string());
    }
    repo.head()
        .ok()
        .and_then(|head| head.shorthand().map(ToOwned::to_owned))
}

fn render_run_config(
    lane: &BlueprintLane,
    integration_artifact_path: Option<&Path>,
    target_repo: &Path,
) -> String {
    let graph_rel = format!(
        "../../workflows/{}/{}.fabro",
        lane.workflow_family(),
        lane.slug()
    );
    let worktree_mode = if lane.template == WorkflowTemplate::Implementation {
        "always"
    } else {
        "clean"
    };
    let llm_config = if matches!(
        lane.template,
        WorkflowTemplate::Bootstrap
            | WorkflowTemplate::RecurringReport
            | WorkflowTemplate::ServiceBootstrap
            | WorkflowTemplate::Implementation
    ) {
        let write_target = automation_primary_target(AutomationProfile::Write);
        let fallback_section = render_fallback_section(AutomationProfile::Write);
        format!(
            "[llm]\nprovider = \"{}\"\nmodel = \"{}\"\n{}\n",
            write_target.provider.as_str(),
            write_target.model,
            fallback_section
        )
    } else {
        String::new()
    };
    let sandbox_env = if matches!(
        lane.template,
        WorkflowTemplate::Bootstrap
            | WorkflowTemplate::RecurringReport
            | WorkflowTemplate::ServiceBootstrap
            | WorkflowTemplate::Implementation
    ) {
        "\n[sandbox.env]\nMINIMAX_API_KEY = \"${env.MINIMAX_API_KEY}\"\n".to_string()
    } else {
        String::new()
    };
    let mut config = format!(
        "version = 1\ngraph = \"{}\"\ngoal = {}\ndirectory = \"../../..\"\n\n{}[sandbox]\nprovider = \"local\"\n\n[sandbox.local]\nworktree_mode = \"{}\"\n{}",
        graph_rel,
        toml_multiline_literal(&lane.goal),
        llm_config,
        worktree_mode,
        sandbox_env,
    );
    if let Some(target_branch) = template_supports_direct_integration(lane.template)
        .then(|| direct_integration_target_branch(target_repo))
        .flatten()
    {
        config.push_str(&format!(
            "\n[integration]\nenabled = true\nstrategy = \"squash\"\ntarget_branch = \"{}\"\n",
            target_branch
        ));
        if let Some(path) = integration_artifact_path {
            config.push_str(&format!(
                "artifact_path = \"{}\"\n",
                run_config_relative_string(path)
            ));
        }
    }
    config
}

fn render_fallback_section(profile: AutomationProfile) -> String {
    let fallback_map = automation_fallback_map(profile);
    if fallback_map.is_empty() {
        return String::new();
    }
    let mut section = String::from("[llm.fallbacks]\n");
    for (provider, values) in fallback_map {
        let rendered = values
            .into_iter()
            .map(|value| format!("\"{value}\""))
            .collect::<Vec<_>>()
            .join(", ");
        section.push_str(&format!("{provider} = [{rendered}]\n"));
    }
    section
}

fn render_prompt(kind: &str, lane: &BlueprintLane) -> String {
    let context = lane.prompt_context.as_deref().unwrap_or(
        "Inspect the relevant repo surfaces, preserve existing doctrine, and produce the lane artifacts honestly.",
    );
    if lane.template == WorkflowTemplate::Implementation {
        let implement_now = prompt_context_block(context, "Implement now:");
        let touch_first = prompt_context_block(context, "Touch first:");
        let build_slice = prompt_context_block(context, "Build in this slice:");
        let setup_first = prompt_context_block(context, "Set up first:");
        let first_proof_gate = prompt_context_block(context, "First proof gate:");
        let first_health_gate = prompt_context_block(context, "First health gate:");
        let execution_guidance = prompt_context_block(context, "Execution guidance:");
        let manual_notes = prompt_context_block(
            context,
            "Manual proof still required after automated verification:",
        );
        let health_surfaces = prompt_context_block(context, "Service/health surfaces to preserve:");
        let observability_surfaces =
            prompt_context_block(context, "Observability surfaces to preserve:");
        let implementation_artifact_expectations = implementation_artifact_expectations(
            &implement_now,
            &touch_first,
            &setup_first,
            &execution_guidance,
            &health_surfaces,
            &observability_surfaces,
        );
        let verification_artifact_expectations = verification_artifact_expectations(
            &first_proof_gate,
            &first_health_gate,
            &manual_notes,
            &execution_guidance,
            &health_surfaces,
            &observability_surfaces,
        );

        return match kind {
            "plan" => render_implementation_plan_prompt(
                lane,
                context,
                &implement_now,
                &touch_first,
                &build_slice,
                &setup_first,
                &first_proof_gate,
                &first_health_gate,
                &execution_guidance,
                &manual_notes,
                &health_surfaces,
                &observability_surfaces,
                &implementation_artifact_expectations,
                &verification_artifact_expectations,
            ),
            "review" => render_implementation_review_prompt(
                lane,
                context,
                &implement_now,
                &touch_first,
                &build_slice,
                &setup_first,
                &first_proof_gate,
                &first_health_gate,
                &execution_guidance,
                &manual_notes,
                &health_surfaces,
                &observability_surfaces,
                &implementation_artifact_expectations,
                &verification_artifact_expectations,
            ),
            "challenge" => render_implementation_challenge_prompt(
                lane,
                context,
                &implement_now,
                &touch_first,
                &build_slice,
                &setup_first,
                &first_proof_gate,
                &first_health_gate,
                &execution_guidance,
                &manual_notes,
                &health_surfaces,
                &observability_surfaces,
                &implementation_artifact_expectations,
                &verification_artifact_expectations,
            ),
            "polish" => render_implementation_fixup_prompt(
                lane,
                context,
                &implement_now,
                &touch_first,
                &build_slice,
                &setup_first,
                &first_proof_gate,
                &first_health_gate,
                &execution_guidance,
                &manual_notes,
                &health_surfaces,
                &observability_surfaces,
                &implementation_artifact_expectations,
                &verification_artifact_expectations,
            ),
            _ => String::new(),
        };
    }
    match kind {
        "plan" => format!(
            "# {} — Plan\n\nLane: `{}`\n\nGoal:\n- {}\n\nContext:\n- {}\n",
            lane.title, lane.id, lane.goal, context
        ),
        "review" => render_general_review_prompt(lane, context).to_string(),
        "polish" => format!(
            "# {} — Polish\n\nPolish the durable artifacts for `{}` so they are clear, repo-specific, and ready for the supervisory plane.\n",
            lane.title, lane.id
        ),
        _ => String::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_implementation_plan_prompt(
    lane: &BlueprintLane,
    context: &str,
    implement_now: &[String],
    touch_first: &[String],
    build_slice: &[String],
    setup_first: &[String],
    first_proof_gate: &[String],
    first_health_gate: &[String],
    execution_guidance: &[String],
    manual_notes: &[String],
    health_surfaces: &[String],
    observability_surfaces: &[String],
    implementation_artifact_expectations: &[String],
    verification_artifact_expectations: &[String],
) -> String {
    let has_structured_sections = !implement_now.is_empty()
        || !touch_first.is_empty()
        || !build_slice.is_empty()
        || !setup_first.is_empty()
        || !first_proof_gate.is_empty()
        || !first_health_gate.is_empty()
        || !execution_guidance.is_empty()
        || !manual_notes.is_empty()
        || !health_surfaces.is_empty()
        || !observability_surfaces.is_empty()
        || !implementation_artifact_expectations.is_empty()
        || !verification_artifact_expectations.is_empty();
    let mut output = format!(
        "# {} — Plan\n\nLane: `{}`\n\nGoal:\n- {}\n",
        lane.title, lane.id, lane.goal
    );
    append_prompt_section(&mut output, "Current slice", implement_now, false);
    append_prompt_section(&mut output, "Touched surfaces", touch_first, true);
    append_prompt_section(&mut output, "Build in this slice", build_slice, false);
    append_prompt_section(&mut output, "Set up first", setup_first, false);
    append_prompt_section(&mut output, "First proof gate", first_proof_gate, true);
    append_prompt_section(&mut output, "First health gate", first_health_gate, true);
    append_prompt_section(&mut output, "Execution guidance", execution_guidance, false);
    append_prompt_section(
        &mut output,
        "Manual proof still required",
        manual_notes,
        false,
    );
    append_prompt_section(
        &mut output,
        "Service/health surfaces to preserve",
        health_surfaces,
        false,
    );
    append_prompt_section(
        &mut output,
        "Observability surfaces to preserve",
        observability_surfaces,
        false,
    );
    append_prompt_section(
        &mut output,
        "Implementation artifact must cover",
        implementation_artifact_expectations,
        false,
    );
    append_prompt_section(
        &mut output,
        "Verification artifact must cover",
        verification_artifact_expectations,
        false,
    );
    output.push_str(
        "\n\nStage ownership:\n- do not write `promotion.md` during Plan/Implement\n- do not hand-author `quality.md`; it is regenerated by the Quality Gate\n- `promotion.md` is owned by the Review stage only\n- keep source edits inside the named slice and touched surfaces\n",
    );
    if !has_structured_sections {
        output.push_str(&format!("\n\nCurrent Slice Contract:\n{}\n", context));
    }
    output
}

fn prompt_context_block(context: &str, heading: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut capture = false;

    for line in context.lines() {
        let trimmed = line.trim();
        if capture && trimmed.ends_with(':') && !trimmed.starts_with('-') {
            break;
        }
        if trimmed == heading {
            capture = true;
            continue;
        }
        if !capture {
            continue;
        }
        if trimmed.is_empty() {
            break;
        }
        lines.push(trimmed.to_string());
    }

    lines
}

fn append_prompt_section(output: &mut String, title: &str, lines: &[String], code: bool) {
    if lines.is_empty() {
        return;
    }
    output.push_str(&format!("\n\n{title}"));
    for line in lines {
        let content = line.trim_start_matches("- ").trim();
        if code {
            output.push_str(&format!("\n- `{content}`"));
        } else {
            output.push_str(&format!("\n- {content}"));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_implementation_review_prompt(
    lane: &BlueprintLane,
    context: &str,
    implement_now: &[String],
    touch_first: &[String],
    build_slice: &[String],
    setup_first: &[String],
    first_proof_gate: &[String],
    first_health_gate: &[String],
    execution_guidance: &[String],
    manual_notes: &[String],
    health_surfaces: &[String],
    observability_surfaces: &[String],
    implementation_artifact_expectations: &[String],
    verification_artifact_expectations: &[String],
) -> String {
    let has_structured_sections = !implement_now.is_empty()
        || !touch_first.is_empty()
        || !build_slice.is_empty()
        || !setup_first.is_empty()
        || !first_proof_gate.is_empty()
        || !first_health_gate.is_empty()
        || !execution_guidance.is_empty()
        || !manual_notes.is_empty();
    let mut output = format!(
        "# {} — Review\n\nReview only the current slice for `{}`.\n",
        lane.title, lane.id
    );
    if !has_structured_sections {
        output.push_str(&format!("\nCurrent Slice Contract:\n{}\n", context));
    }
    append_prompt_section(&mut output, "Current slice", implement_now, false);
    append_prompt_section(&mut output, "Touched surfaces", touch_first, true);
    append_prompt_section(&mut output, "Slice work", build_slice, false);
    append_prompt_section(&mut output, "Setup checks", setup_first, false);
    append_prompt_section(&mut output, "First proof gate", first_proof_gate, true);
    append_prompt_section(&mut output, "First health gate", first_health_gate, true);
    append_prompt_section(&mut output, "Execution guidance", execution_guidance, false);
    append_prompt_section(
        &mut output,
        "Manual proof still required",
        manual_notes,
        false,
    );
    append_prompt_section(
        &mut output,
        "Health surfaces to preserve",
        health_surfaces,
        false,
    );
    append_prompt_section(
        &mut output,
        "Observability surfaces to preserve",
        observability_surfaces,
        false,
    );
    append_prompt_section(
        &mut output,
        "Implementation artifact must cover",
        implementation_artifact_expectations,
        false,
    );
    append_prompt_section(
        &mut output,
        "Verification artifact must cover",
        verification_artifact_expectations,
        false,
    );
    let security_review_items = implementation_security_review_items(
        lane,
        context,
        touch_first,
        build_slice,
        execution_guidance,
        health_surfaces,
        observability_surfaces,
    );
    append_prompt_section(
        &mut output,
        "Nemesis-style security review",
        &security_review_items,
        false,
    );
    output.push_str(
        "\n\nFocus on:\n- slice scope discipline\n- proof-gate coverage for the active slice\n- touched-surface containment\n- implementation and verification artifact quality\n- remaining blockers before the next slice\n",
    );
    output.push_str(
        "\nDeterministic evidence:\n- treat `quality.md` as machine-generated truth about placeholder debt, warning debt, manual follow-up, and artifact mismatch risk\n- if `quality.md` says `quality_ready: no`, do not bless the slice as merge-ready\n",
    );
    output.push_str(
        "\n\nWrite `promotion.md` in this exact machine-readable form:\n\n\
merge_ready: yes|no\n\
manual_proof_pending: yes|no\n\
reason: <one sentence>\n\
next_action: <one sentence>\n\n\
Only set `merge_ready: yes` when:\n\
- `quality.md` says `quality_ready: yes`\n\
- automated proof is sufficient for this slice\n\
- any required manual proof has actually been performed\n\
- no unresolved warnings or stale failures undermine confidence\n\
- the implementation and verification artifacts match the real code.\n",
    );
    output.push_str(
        "\nReview stage ownership:\n- you may write or replace `promotion.md` in this stage\n- read `quality.md` before deciding `merge_ready`\n- when the slice is security-sensitive, perform a Nemesis-style pass: first-principles assumption challenge plus coupled-state consistency review\n- include security findings in the review verdict when the slice touches trust boundaries, keys, funds, auth, control-plane behavior, or external process control\n- prefer not to modify source code here unless a tiny correction is required to make the review judgment truthful\n",
    );
    output
}

#[allow(clippy::too_many_arguments)]
fn render_implementation_challenge_prompt(
    lane: &BlueprintLane,
    _context: &str,
    implement_now: &[String],
    touch_first: &[String],
    build_slice: &[String],
    _setup_first: &[String],
    first_proof_gate: &[String],
    _first_health_gate: &[String],
    _execution_guidance: &[String],
    _manual_notes: &[String],
    _health_surfaces: &[String],
    _observability_surfaces: &[String],
    implementation_artifact_expectations: &[String],
    verification_artifact_expectations: &[String],
) -> String {
    // Challenge is a cheap adversarial pre-review.  Only include the sections
    // relevant to scope-checking and proof verification — omit health, observability,
    // and the raw context dump (which duplicates the structured sections already
    // extracted from it).
    let mut output = format!(
        "# {} — Challenge\n\nPerform a cheap adversarial review of the current slice for `{}` before the expensive final review runs.\n",
        lane.title, lane.id
    );
    output.push_str(
        "\nYour job is to challenge assumptions, find obvious scope drift, identify weak proof, and catch mismatches between code and artifacts. Do not bless the slice as merge-ready; that belongs to the final review gate.\n",
    );
    append_prompt_section(&mut output, "Current slice", implement_now, false);
    append_prompt_section(&mut output, "Touched surfaces", touch_first, true);
    append_prompt_section(&mut output, "Slice work", build_slice, false);
    append_prompt_section(&mut output, "First proof gate", first_proof_gate, true);
    append_prompt_section(
        &mut output,
        "Implementation artifact must cover",
        implementation_artifact_expectations,
        false,
    );
    append_prompt_section(
        &mut output,
        "Verification artifact must cover",
        verification_artifact_expectations,
        false,
    );
    output.push_str(
        "\nChallenge checklist:\n- Is the slice smaller than the plan says, or larger?\n- Did the implementation actually satisfy the first proof gate?\n- Are any touched surfaces outside the named slice?\n- Are the artifacts overstating completion?\n- Is there an obvious bug, trust-boundary issue, or missing test the final reviewer should not have to rediscover?\n",
    );
    output.push_str(
        "\nWrite a short challenge note in `verification.md` or amend it if needed, focusing on concrete gaps and the next fixup target. Do not write `promotion.md` here.\n",
    );
    output
}

#[allow(clippy::too_many_arguments)]
fn render_implementation_fixup_prompt(
    lane: &BlueprintLane,
    _context: &str,
    implement_now: &[String],
    touch_first: &[String],
    build_slice: &[String],
    setup_first: &[String],
    first_proof_gate: &[String],
    _first_health_gate: &[String],
    execution_guidance: &[String],
    _manual_notes: &[String],
    _health_surfaces: &[String],
    _observability_surfaces: &[String],
    implementation_artifact_expectations: &[String],
    verification_artifact_expectations: &[String],
) -> String {
    // Fixup runs AFTER implement and verify, so the model already has conversation
    // context via the preamble.  Include only the sections needed to unblock the
    // proof gate — omit doctrine, health, observability, and manual notes.
    let mut output = format!(
        "# {} — Fixup\n\nFix only the current slice for `{}`.\n",
        lane.title, lane.id
    );
    append_prompt_section(&mut output, "Current slice", implement_now, false);
    append_prompt_section(&mut output, "Touched surfaces", touch_first, true);
    append_prompt_section(&mut output, "Slice work", build_slice, false);
    append_prompt_section(&mut output, "Setup checks", setup_first, false);
    append_prompt_section(&mut output, "First proof gate", first_proof_gate, true);
    append_prompt_section(&mut output, "Execution guidance", execution_guidance, false);
    append_prompt_section(
        &mut output,
        "Implementation artifact must cover",
        implementation_artifact_expectations,
        false,
    );
    append_prompt_section(
        &mut output,
        "Verification artifact must cover",
        verification_artifact_expectations,
        false,
    );
    output.push_str(
        "\n\nPriorities:\n- unblock the active slice's first proof gate — this is the #1 priority\n- prefer staying within the named slice and touched surfaces\n- if the proof gate fails on pre-existing issues OUTSIDE your surfaces (e.g., linter warnings in unrelated files, missing imports in dependencies), you MUST fix those issues minimally to unblock the gate — do not leave the lane stuck on problems you can solve\n- preserve setup constraints before expanding implementation scope\n- keep implementation and verification artifacts durable and specific\n- do not create or rewrite `promotion.md` during Fixup; that file is owned by the Review stage\n- do not hand-author `quality.md`; the Quality Gate rewrites it after verification\n",
    );
    output
}

fn implementation_security_review_items(
    lane: &BlueprintLane,
    context: &str,
    touch_first: &[String],
    build_slice: &[String],
    execution_guidance: &[String],
    health_surfaces: &[String],
    observability_surfaces: &[String],
) -> Vec<String> {
    let mut haystack = format!("{} {} {} {}", lane.id, lane.title, lane.goal, context);
    for lines in [
        touch_first,
        build_slice,
        execution_guidance,
        health_surfaces,
        observability_surfaces,
    ] {
        for line in lines {
            haystack.push('\n');
            haystack.push_str(line);
        }
    }
    let lower = haystack.to_lowercase();
    let is_security_sensitive = [
        "wallet",
        "rpc",
        "seed",
        "shuffle",
        "provably",
        "settlement",
        "payout",
        "balance",
        "auth",
        "token",
        "principal",
        "secret",
        "key",
        "daemon",
        "control plane",
        "control-plane",
        "pair",
        "capability",
        "mining",
        "node",
        "external process",
    ]
    .iter()
    .any(|needle| lower.contains(needle));
    if !is_security_sensitive {
        return Vec::new();
    }

    let mut items = vec![
        "Pass 1 — first-principles challenge: question trust boundaries, authority assumptions, and who can trigger the slice's dangerous actions".to_string(),
        "Pass 2 — coupled-state review: identify paired state or protocol surfaces and check that every mutation path keeps them consistent or explains the asymmetry".to_string(),
    ];
    if [
        "wallet",
        "rpc",
        "seed",
        "shuffle",
        "provably",
        "settlement",
        "payout",
        "balance",
        "token",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        items.push(
            "check state transitions that affect balances, commitments, randomness, payout safety, or replayability"
                .to_string(),
        );
    }
    if ["auth", "principal", "secret", "key", "pair", "capability"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        items.push(
            "check secret handling, capability scoping, pairing/idempotence behavior, and privilege escalation paths"
                .to_string(),
        );
    }
    if [
        "daemon",
        "control plane",
        "control-plane",
        "mining",
        "node",
        "external process",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        items.push(
            "check external-process control, operator safety, idempotent retries, and failure modes around service lifecycle"
                .to_string(),
        );
    }
    items
}

fn general_security_review_items(lane: &BlueprintLane, context: &str) -> Vec<String> {
    implementation_security_review_items(lane, context, &[], &[], &[], &[], &[])
}

fn render_general_review_prompt(lane: &BlueprintLane, context: &str) -> String {
    let mut output = format!(
        "# {} — Review\n\nReview the lane outcome for `{}`.\n\nFocus on:\n- correctness\n- milestone fit\n- remaining blockers\n",
        lane.title, lane.id
    );
    let security_items = general_security_review_items(lane, context);
    append_prompt_section(
        &mut output,
        "Nemesis-style security review",
        &security_items,
        false,
    );
    output
}

fn implementation_artifact_expectations(
    implement_now: &[String],
    touch_first: &[String],
    setup_first: &[String],
    execution_guidance: &[String],
    health_surfaces: &[String],
    observability_surfaces: &[String],
) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(slice) = implement_now.first() {
        lines.push(format!(
            "state whether {} was completed or partially completed",
            slice.trim_start_matches("- ").trim()
        ));
    }
    if !touch_first.is_empty() {
        lines.push("list the touched files/modules for this slice".to_string());
    }
    if !setup_first.is_empty() {
        lines.push(
            "note which setup steps were completed, deferred, or intentionally skipped".to_string(),
        );
    }
    if !execution_guidance.is_empty() {
        lines.push("call out anything that still blocks the next slice from starting".to_string());
    }
    if !health_surfaces.is_empty() {
        lines.push("describe which operator-facing health surfaces were introduced or left for a later slice".to_string());
    }
    if !observability_surfaces.is_empty() {
        lines.push(
            "describe which operator-facing logs or observability surfaces were introduced or deferred"
                .to_string(),
        );
    }
    lines
}

fn verification_artifact_expectations(
    first_proof_gate: &[String],
    first_health_gate: &[String],
    manual_notes: &[String],
    execution_guidance: &[String],
    health_surfaces: &[String],
    observability_surfaces: &[String],
) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(proof_gate) = first_proof_gate.first() {
        lines.push(format!(
            "record whether {} passed and what it proved",
            proof_gate.trim_start_matches("- ").trim()
        ));
    }
    lines.push("summarize the automated proof commands that ran and their outcomes".to_string());
    if let Some(health_gate) = first_health_gate.first() {
        lines.push(format!(
            "record whether {} passed and what service signal it verified",
            health_gate.trim_start_matches("- ").trim()
        ));
    }
    if !manual_notes.is_empty() {
        lines.push("state which manual proof steps remain and why they still matter".to_string());
    }
    if !execution_guidance.is_empty() {
        lines.push(
            "say whether the slice is complete enough to move to the next ordered slice"
                .to_string(),
        );
    }
    if !health_surfaces.is_empty() {
        lines.push(
            "summarize the health/observability surfaces that were verified or remain pending"
                .to_string(),
        );
    }
    if !observability_surfaces.is_empty() {
        lines.push(
            "record which observability/log surfaces were exercised or remain pending".to_string(),
        );
    }
    lines
}

fn lane_equivalent(current: &BlueprintLane, desired: &BlueprintLane) -> bool {
    current.kind == desired.kind
        && current.title == desired.title
        && current.family == desired.family
        && current.workflow_family() == desired.workflow_family()
        && current.slug() == desired.slug()
        && current.template == desired.template
        && current.goal == desired.goal
        && current.managed_milestone == desired.managed_milestone
        && current.dependencies == desired.dependencies
        && current.produces == desired.produces
        && current.proof_profile == desired.proof_profile
        && current.proof_state_path == desired.proof_state_path
        && current.program_manifest == desired.program_manifest
        && current.service_state_path == desired.service_state_path
        && current.orchestration_state_path == desired.orchestration_state_path
        && current.checks == desired.checks
        && current.run_dir == desired.run_dir
        && current.verify_command == desired.verify_command
        && current.health_command == desired.health_command
}

fn default_verify_command(
    blueprint: &ProgramBlueprint,
    unit_id: &str,
    lane: &BlueprintLane,
) -> String {
    let artifact_paths = lane_artifact_paths(blueprint, unit_id, lane, Path::new("."));
    if artifact_paths.is_empty() {
        return "true".to_string();
    }
    artifact_paths
        .iter()
        .map(|path| format!("test -f {}", path.display()))
        .collect::<Vec<_>>()
        .join(" && ")
}

fn diff_blueprints(current: &ProgramBlueprint, desired: &ProgramBlueprint) -> Vec<String> {
    let mut findings = Vec::new();
    let current_units = current
        .units
        .iter()
        .map(|unit| (&unit.id, unit))
        .collect::<BTreeMap<_, _>>();
    let desired_units = desired
        .units
        .iter()
        .map(|unit| (&unit.id, unit))
        .collect::<BTreeMap<_, _>>();

    report_set_drift(
        "unit",
        current_units.keys().copied(),
        desired_units.keys().copied(),
        &mut findings,
    );

    for (unit_id, desired_unit) in &desired_units {
        let Some(current_unit) = current_units.get(unit_id) else {
            continue;
        };
        let current_lanes = current_unit
            .lanes
            .iter()
            .map(|lane| (&lane.id, lane))
            .collect::<BTreeMap<_, _>>();
        let desired_lanes = desired_unit
            .lanes
            .iter()
            .map(|lane| (&lane.id, lane))
            .collect::<BTreeMap<_, _>>();

        report_set_drift(
            &format!("lane in unit `{unit_id}`"),
            current_lanes.keys().copied(),
            desired_lanes.keys().copied(),
            &mut findings,
        );

        for (lane_id, desired_lane) in desired_lanes {
            let Some(current_lane) = current_lanes.get(lane_id) else {
                continue;
            };
            if current_lane.kind != desired_lane.kind {
                findings.push(format!(
                    "lane `{unit_id}:{lane_id}` kind changes from `{}` to `{}`",
                    current_lane.kind, desired_lane.kind
                ));
            }
            if current_lane.managed_milestone != desired_lane.managed_milestone {
                findings.push(format!(
                    "lane `{unit_id}:{lane_id}` milestone changes from `{}` to `{}`",
                    current_lane.managed_milestone, desired_lane.managed_milestone
                ));
            }
            if current_lane.produces != desired_lane.produces {
                findings.push(format!(
                    "lane `{unit_id}:{lane_id}` produced artifacts change"
                ));
            }
            if current_lane.dependencies != desired_lane.dependencies {
                findings.push(format!("lane `{unit_id}:{lane_id}` dependencies change"));
            }
        }
    }

    if findings.is_empty() {
        findings.push("existing package already matches blueprint structure".to_string());
    }
    findings
}

fn input_findings(blueprint: &ProgramBlueprint, target_repo: &Path) -> Vec<String> {
    let mut findings = Vec::new();
    for doctrine in &blueprint.inputs.doctrine_files {
        let absolute = target_repo.join(doctrine);
        if absolute.exists() {
            findings.push(format!("doctrine input found: {}", doctrine.display()));
        } else {
            findings.push(format!("doctrine input missing: {}", doctrine.display()));
        }
    }
    for evidence in &blueprint.inputs.evidence_paths {
        let absolute = target_repo.join(evidence);
        if absolute.exists() {
            findings.push(format!("evidence input found: {}", evidence.display()));
        } else {
            findings.push(format!("evidence input missing: {}", evidence.display()));
        }
    }
    findings
}

fn doctrine_evidence_support_findings(
    current: &ProgramBlueprint,
    desired: &ProgramBlueprint,
    target_repo: &Path,
) -> Vec<String> {
    let mut findings = Vec::new();
    let texts = input_texts(desired, target_repo);
    let current_units = current
        .units
        .iter()
        .map(|unit| (&unit.id, unit))
        .collect::<BTreeMap<_, _>>();

    for desired_unit in &desired.units {
        let current_lanes = current_units
            .get(&desired_unit.id)
            .map(|unit| {
                unit.lanes
                    .iter()
                    .map(|lane| lane.id.as_str())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();

        for lane in &desired_unit.lanes {
            if current_lanes.contains(lane.id.as_str()) {
                continue;
            }
            let lane_terms = lane_terms(lane);
            for (source, text) in &texts {
                if lane_terms.iter().any(|term| text.contains(term)) {
                    findings.push(format!("lane `{}` is supported by {}", lane.id, source));
                    break;
                }
            }
        }
    }

    findings
}

fn current_runtime_findings(current: &ProgramBlueprint, target_repo: &Path) -> Vec<String> {
    let mut findings = Vec::new();
    let Some(state_path) = current.program.state_path.as_ref() else {
        return findings;
    };
    let absolute = target_repo.join(state_path);
    let Ok(raw) = std::fs::read_to_string(&absolute) else {
        findings.push(format!("runtime state missing: {}", state_path.display()));
        return findings;
    };
    findings.push(format!("runtime state found: {}", state_path.display()));

    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        findings.push(format!(
            "runtime state unreadable as json: {}",
            state_path.display()
        ));
        return findings;
    };
    let Some(lanes) = value.get("lanes").and_then(Value::as_object) else {
        return findings;
    };

    for (lane_key, lane_state) in lanes {
        let Some(status) = lane_state.get("status").and_then(Value::as_str) else {
            continue;
        };
        if matches!(status, "running" | "failed" | "blocked") {
            findings.push(format!(
                "runtime state reports lane `{lane_key}` as `{status}`"
            ));
        }
        if matches!(status, "running" | "failed" | "blocked")
            && lane_artifacts_satisfied(current, lane_key, target_repo)
        {
            findings.push(format!(
                "runtime state for lane `{lane_key}` may be stale because all produced artifacts already exist"
            ));
        }
    }

    findings
}

fn desired_artifact_findings(desired: &ProgramBlueprint, target_repo: &Path) -> Vec<String> {
    let mut findings = Vec::new();
    for unit in &desired.units {
        for artifact in &unit.artifacts {
            let path = target_repo.join(&unit.output_root).join(&artifact.path);
            let relative = path
                .strip_prefix(target_repo)
                .unwrap_or(path.as_path())
                .display()
                .to_string();
            if path.exists() {
                findings.push(format!("artifact already present: {relative}"));
            } else {
                findings.push(format!("artifact missing: {relative}"));
            }
        }
    }
    findings
}

fn desired_execution_findings(desired: &ProgramBlueprint, target_repo: &Path) -> Vec<String> {
    let mut findings = Vec::new();
    for unit in &desired.units {
        for lane in &unit.lanes {
            let artifact_paths = lane_artifact_paths(desired, &unit.id, lane, target_repo);
            let artifacts_ready =
                !artifact_paths.is_empty() && artifact_paths.iter().all(|path| path.exists());
            if artifacts_ready {
                findings.push(format!(
                    "lane `{}` already satisfies its produced artifacts",
                    lane_key(&unit.id, &lane.id)
                ));
                continue;
            }

            let dependency_ok = lane
                .dependencies
                .iter()
                .all(|dependency| dependency_satisfied(desired, dependency, target_repo));
            let checks_ok = lane
                .checks
                .iter()
                .all(|check| check_satisfied(check, target_repo));

            if dependency_ok && checks_ok {
                findings.push(format!(
                    "lane `{}` appears ready for execution",
                    lane_key(&unit.id, &lane.id)
                ));
            }
        }
    }
    findings
}

fn review_artifact_findings(desired: &ProgramBlueprint, target_repo: &Path) -> Vec<String> {
    let mut findings = Vec::new();
    let known_refs = known_lane_refs(target_repo);
    for unit in &desired.units {
        for lane in &unit.lanes {
            let Some(review_path) = lane_review_artifact_path(unit, lane, target_repo) else {
                continue;
            };
            let Ok(contents) = std::fs::read_to_string(&review_path) else {
                continue;
            };
            let text = contents.to_lowercase();
            let key = lane_key(&unit.id, &lane.id);
            if review_says_implementation_ready(&text) {
                findings.push(format!(
                    "review artifact for lane `{key}` says an implementation follow-on is ready"
                ));
                if let Some((run_config, workflow)) =
                    missing_implementation_package(target_repo, lane)
                {
                    findings.push(format!(
                        "implementation package missing for lane `{key}` at `{}` and `{}`",
                        run_config.display(),
                        workflow.display()
                    ));
                }
            }
            if review_says_implementation_blocked(&text) {
                findings.push(format!(
                    "review artifact for lane `{key}` says implementation is still blocked"
                ));
                let blockers = blocked_review_refs(&text, &known_refs, &key);
                if !blockers.is_empty() {
                    findings.push(format!(
                        "review artifact for lane `{key}` names upstream blockers: {}",
                        blockers.join(", ")
                    ));
                }
            }
            if text.contains("reset implementation scope") {
                findings.push(format!(
                    "review artifact for lane `{key}` resets implementation scope"
                ));
            }
        }
    }
    findings
}

fn evolve_recommendations(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
    findings: &[String],
) -> Vec<String> {
    let mut recommendations = Vec::new();
    let structure_stable = findings
        .iter()
        .any(|finding| finding == "existing package already matches blueprint structure");
    let mut ready_lanes = Vec::new();
    let mut stale_lanes = Vec::new();
    let mut blocked_lanes = Vec::new();
    let implementation_candidates = implementation_candidates(blueprint, target_repo);
    let lane_catalog = lane_catalog(target_repo);

    for finding in findings {
        if let Some(lane) = extract_between(finding, "lane `", "` appears ready for execution") {
            ready_lanes.push(lane.to_string());
        }
        if let Some(lane) = extract_between(
            finding,
            "runtime state for lane `",
            "` may be stale because all produced artifacts already exist",
        ) {
            stale_lanes.push(lane.to_string());
            recommendations.push(format!(
                "refresh or clear the stale runtime record for `{lane}` because the lane looks complete from its artifacts"
            ));
        }
        if let Some(path) = finding.strip_prefix("runtime state missing: ") {
            recommendations.push(format!(
                "create or refresh `{path}` by running the supervisory status/evolve loop so the program has durable runtime truth"
            ));
        }
        if let Some(lane) = extract_between(
            finding,
            "review artifact for lane `",
            "` says implementation is still blocked",
        ) {
            blocked_lanes.push(lane.to_string());
            recommendations.push(format!(
                "do not add an implementation-family follow-on lane for `{lane}` yet because the current review artifact still marks implementation as blocked"
            ));
        }
        if let Some(rest) = finding.strip_prefix("review artifact for lane `") {
            if let Some((lane, blockers)) = rest.split_once("` names upstream blockers: ") {
                recommendations.push(format!(
                    "defer the implementation program for `{lane}` until these upstream blockers clear: {blockers}"
                ));
                recommendations.push(format!(
                    "when the implementation program for `{lane}` is created, include dependency gates for: {blockers}"
                ));
                recommendations.extend(blocker_contract_recommendations(
                    blueprint,
                    lane,
                    blockers,
                    &lane_catalog,
                ));
            }
        }
        if let Some(lane) = extract_between(
            finding,
            "review artifact for lane `",
            "` resets implementation scope",
        ) {
            recommendations.push(format!(
                "when you add a follow-on lane for `{lane}`, treat it as a fresh implementation scope rather than a continuation of prior code"
            ));
        }
    }

    recommendations.extend(blocker_milestone_refinement_recommendations(
        findings,
        &lane_catalog,
    ));

    if !ready_lanes.is_empty() {
        let lanes = ready_lanes.join(", ");
        recommendations.push(format!(
            "execute the next ready bootstrap lane(s) first: {lanes}"
        ));
    }

    if !implementation_candidates.is_empty() {
        let ordered = implementation_candidates
            .iter()
            .map(|candidate| candidate.lane_key.as_str())
            .collect::<Vec<_>>()
            .join(" -> ");
        recommendations.push(format!(
            "after bootstrap work settles, add implementation-family packages in this order: {ordered}"
        ));

        for candidate in &implementation_candidates {
            if candidate.package_missing {
                recommendations.push(format!(
                    "add implementation program `{}` plus an implementation-family package for `{}` using `{}` and `{}`",
                    candidate.program_manifest.display(),
                    candidate.lane_key,
                    candidate.run_config.display(),
                    candidate.workflow.display()
                ));
            } else {
                recommendations.push(format!(
                    "refresh implementation program `{}` and its implementation-family package for `{}` using `{}` and `{}`",
                    candidate.program_manifest.display(),
                    candidate.lane_key,
                    candidate.run_config.display(),
                    candidate.workflow.display()
                ));
            }
        }
    }

    if structure_stable && !ready_lanes.is_empty() && implementation_candidates.is_empty() {
        let lanes = ready_lanes.join(", ");
        recommendations.push(format!(
            "leave the workflow package unchanged for now and execute the next ready lane(s): {lanes}"
        ));
    }

    if structure_stable
        && !stale_lanes.is_empty()
        && ready_lanes.is_empty()
        && implementation_candidates.is_empty()
    {
        let lanes = stale_lanes.join(", ");
        recommendations.push(format!(
            "leave the workflow package unchanged for now and repair stale runtime truth for: {lanes}"
        ));
    }

    if structure_stable
        && !blocked_lanes.is_empty()
        && ready_lanes.is_empty()
        && implementation_candidates.is_empty()
    {
        let lanes = blocked_lanes.join(", ");
        recommendations.push(format!(
            "leave the workflow package unchanged for now and wait on upstream blockers for: {lanes}"
        ));
    }

    recommendations.sort();
    recommendations.dedup();
    recommendations
}

fn extract_between<'a>(text: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(prefix)?;
    rest.strip_suffix(suffix)
}

#[derive(Debug, Clone)]
struct LaneCatalogEntry {
    program_id: String,
    unit_id: String,
    lane_id: String,
    managed_milestone: String,
    review_artifact: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct ImplementationCandidate {
    lane_key: String,
    unit_id: String,
    lane_id: String,
    program_manifest: PathBuf,
    run_config: PathBuf,
    workflow: PathBuf,
    package_missing: bool,
}

fn implementation_follow_on_unit_id(candidate: &ImplementationCandidate) -> String {
    if candidate.unit_id == candidate.lane_id {
        format!("{}-implementation", candidate.lane_id)
    } else {
        format!("{}-{}-implementation", candidate.unit_id, candidate.lane_id)
    }
}

fn implementation_follow_on_slug(candidate: &ImplementationCandidate) -> String {
    implementation_follow_on_unit_id(candidate)
}

fn implementation_follow_on_title(candidate: &ImplementationCandidate) -> String {
    implementation_follow_on_unit_id(candidate)
        .split('-')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!("{}{}", first.to_uppercase(), chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReviewStageRequirement {
    blocker: String,
    detail: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ImplementationEvidence {
    first_slice: Option<String>,
    first_code_surface: Option<String>,
    first_slice_work: Option<String>,
    first_proof_gate: Option<String>,
    first_smoke_gate: Option<String>,
    first_health_gate: Option<String>,
    observability_notes: Vec<String>,
    setup_notes: Vec<String>,
    proof_commands: Vec<String>,
    smoke_commands: Vec<String>,
    health_commands: Vec<String>,
    manual_notes: Vec<String>,
    slice_notes: Vec<String>,
    health_notes: Vec<String>,
}

fn implementation_candidates(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
) -> Vec<ImplementationCandidate> {
    let mut candidates = Vec::new();
    for unit in &blueprint.units {
        for lane in &unit.lanes {
            let Some(review_path) = lane_review_artifact_path(unit, lane, target_repo) else {
                continue;
            };
            let Ok(contents) = std::fs::read_to_string(&review_path) else {
                continue;
            };
            let text = contents.to_lowercase();
            let lane_key_text = lane_key(&unit.id, &lane.id);
            let review_ready = review_says_implementation_ready(&text);
            let settled_follow_on_ready = lane.template != WorkflowTemplate::Implementation
                && lane.program_manifest.is_none()
                && lane_artifacts_satisfied(blueprint, &lane_key_text, target_repo)
                && !lane
                    .produces
                    .iter()
                    .any(|artifact_id| artifact_id == "validation_plan");
            if review_says_implementation_blocked(&text)
                || (!review_ready && !settled_follow_on_ready)
            {
                continue;
            }
            let (run_config, workflow) = implementation_package_paths(target_repo, lane);
            candidates.push(ImplementationCandidate {
                lane_key: lane_key_text,
                unit_id: unit.id.clone(),
                lane_id: lane.id.clone(),
                program_manifest: implementation_program_manifest_path(
                    &blueprint.program.id,
                    &unit.id,
                    &lane.id,
                    target_repo,
                ),
                run_config,
                workflow,
                package_missing: missing_implementation_package(target_repo, lane).is_some(),
            });
        }
    }
    sort_implementation_candidates(candidates, blueprint)
}

fn implementation_blueprint_for_candidate(
    blueprint: &ProgramBlueprint,
    candidate: &ImplementationCandidate,
    target_repo: &Path,
) -> Result<ProgramBlueprint, RenderError> {
    let Some(unit) = blueprint
        .units
        .iter()
        .find(|unit| unit.id == candidate.unit_id)
    else {
        return Err(RenderError::Blueprint(
            crate::error::BlueprintError::Invalid {
                path: candidate.program_manifest.clone(),
                message: format!(
                    "implementation candidate references unknown unit `{}`",
                    candidate.unit_id
                ),
            },
        ));
    };
    let Some(lane) = unit.lanes.iter().find(|lane| lane.id == candidate.lane_id) else {
        return Err(RenderError::Blueprint(
            crate::error::BlueprintError::Invalid {
                path: candidate.program_manifest.clone(),
                message: format!(
                    "implementation candidate references unknown lane `{}` in unit `{}`",
                    candidate.lane_id, candidate.unit_id
                ),
            },
        ));
    };

    let spec_path = lane_named_artifact_path_for_follow_on(unit, lane, "spec");
    let review_path = lane_named_artifact_path(unit, lane, "review");
    let artifact_dir = lane_artifact_dir(unit, lane);
    let implementation_path = join_relative(&artifact_dir, "implementation.md");
    let verification_path = join_relative(&artifact_dir, "verification.md");
    let quality_path = join_relative(&artifact_dir, "quality.md");
    let promotion_path = join_relative(&artifact_dir, "promotion.md");
    let integration_path = join_relative(&artifact_dir, "integration.md");
    let evidence = implementation_evidence(unit, lane, target_repo);
    let verify_command = implementation_verify_command(lane, &evidence);
    let program_id = candidate
        .program_manifest
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("implementation")
        .to_string();
    let lane_title = lane
        .title
        .strip_suffix(" Lane")
        .unwrap_or(lane.title.as_str());
    let implementation_lane = BlueprintLane {
        id: format!("{}-implement", lane.id),
        kind: lane.kind,
        title: format!("{lane_title} Implementation Lane"),
        family: "implement".to_string(),
        workflow_family: Some("implement".to_string()),
        slug: Some(lane.slug().to_string()),
        template: WorkflowTemplate::Implementation,
        goal: implementation_goal(
            unit,
            lane,
            &spec_path,
            &review_path,
            &quality_path,
            &promotion_path,
            &integration_path,
        ),
        managed_milestone: "merge_ready".to_string(),
        dependencies: vec![raspberry_supervisor::manifest::LaneDependency {
            unit: unit.id.clone(),
            lane: None,
            milestone: Some("reviewed".to_string()),
        }],
        produces: vec![
            "implementation".to_string(),
            "verification".to_string(),
            "quality".to_string(),
            "promotion".to_string(),
            "integration".to_string(),
        ],
        proof_profile: Some(implementation_proof_profile(lane)),
        proof_state_path: None,
        program_manifest: None,
        service_state_path: None,
        orchestration_state_path: None,
        checks: implementation_checks(blueprint, unit, lane, &verify_command),
        run_dir: None,
        prompt_context: Some(implementation_prompt_context(
            &spec_path,
            &review_path,
            &implementation_path,
            &verification_path,
            &quality_path,
            &promotion_path,
            &integration_path,
            &evidence,
        )),
        verify_command: Some(verify_command),
        health_command: implementation_health_command(&evidence),
    };

    Ok(ProgramBlueprint {
        version: blueprint.version,
        program: crate::blueprint::BlueprintProgram {
            id: program_id.clone(),
            max_parallel: 1,
            state_path: Some(PathBuf::from(format!(".raspberry/{program_id}-state.json"))),
            run_dir: None,
        },
        inputs: crate::blueprint::BlueprintInputs::default(),
        package: blueprint.package.clone(),
        units: vec![BlueprintUnit {
            id: unit.id.clone(),
            title: format!("{lane_title} Delivery"),
            output_root: unit.output_root.clone(),
            artifacts: vec![
                crate::blueprint::BlueprintArtifact {
                    id: "spec".to_string(),
                    path: spec_path,
                },
                crate::blueprint::BlueprintArtifact {
                    id: "review".to_string(),
                    path: review_path,
                },
                crate::blueprint::BlueprintArtifact {
                    id: "implementation".to_string(),
                    path: implementation_path,
                },
                crate::blueprint::BlueprintArtifact {
                    id: "verification".to_string(),
                    path: verification_path,
                },
                crate::blueprint::BlueprintArtifact {
                    id: "quality".to_string(),
                    path: quality_path,
                },
                crate::blueprint::BlueprintArtifact {
                    id: "promotion".to_string(),
                    path: promotion_path,
                },
                crate::blueprint::BlueprintArtifact {
                    id: "integration".to_string(),
                    path: integration_path,
                },
            ],
            milestones: vec![
                raspberry_supervisor::manifest::MilestoneManifest {
                    id: "reviewed".to_string(),
                    requires: vec!["spec".to_string(), "review".to_string()],
                },
                raspberry_supervisor::manifest::MilestoneManifest {
                    id: "implemented".to_string(),
                    requires: vec![
                        "spec".to_string(),
                        "review".to_string(),
                        "implementation".to_string(),
                    ],
                },
                raspberry_supervisor::manifest::MilestoneManifest {
                    id: "verified".to_string(),
                    requires: vec![
                        "spec".to_string(),
                        "review".to_string(),
                        "implementation".to_string(),
                        "verification".to_string(),
                        "quality".to_string(),
                    ],
                },
                raspberry_supervisor::manifest::MilestoneManifest {
                    id: "merge_ready".to_string(),
                    requires: vec![
                        "spec".to_string(),
                        "review".to_string(),
                        "implementation".to_string(),
                        "verification".to_string(),
                        "quality".to_string(),
                        "promotion".to_string(),
                    ],
                },
                raspberry_supervisor::manifest::MilestoneManifest {
                    id: "integrated".to_string(),
                    requires: vec!["integration".to_string()],
                },
            ],
            lanes: vec![implementation_lane],
        }],
    })
}

fn lane_named_artifact_path_for_follow_on(
    unit: &BlueprintUnit,
    lane: &BlueprintLane,
    kind: &str,
) -> PathBuf {
    if kind == "spec" {
        let artifact_ids = if lane.produces.is_empty() {
            unit.artifacts
                .iter()
                .map(|artifact| artifact.id.clone())
                .collect::<Vec<_>>()
        } else {
            lane.produces.clone()
        };
        if let Some(path) = artifact_ids.iter().find_map(|artifact_id| {
            unit.artifacts
                .iter()
                .find(|artifact| artifact.id == *artifact_id)
                .filter(|artifact| {
                    artifact.id != "review"
                        && artifact.path.file_name().and_then(|name| name.to_str())
                            != Some("review.md")
                })
                .map(|artifact| artifact.path.clone())
        }) {
            return path;
        }
    }
    lane_named_artifact_path(unit, lane, kind)
}

fn blocked_review_requirement_findings(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
) -> Vec<String> {
    let mut findings = Vec::new();
    let known_refs = known_lane_refs(target_repo);

    for unit in &blueprint.units {
        for lane in &unit.lanes {
            let Some(review_path) = lane_review_artifact_path(unit, lane, target_repo) else {
                continue;
            };
            let Ok(contents) = std::fs::read_to_string(&review_path) else {
                continue;
            };
            let text = contents.to_lowercase();
            if !review_says_implementation_blocked(&text) {
                continue;
            }

            let lane_key_text = lane_key(&unit.id, &lane.id);
            for requirement in review_stage_requirements(&contents, &known_refs) {
                findings.push(format!(
                    "review artifact for lane `{lane_key_text}` requires blocker `{}` to {} before implementation",
                    requirement.blocker, requirement.detail
                ));
            }
        }
    }

    findings
}

fn sort_implementation_candidates(
    candidates: Vec<ImplementationCandidate>,
    blueprint: &ProgramBlueprint,
) -> Vec<ImplementationCandidate> {
    let candidate_map = candidates
        .iter()
        .map(|candidate| (candidate.lane_key.clone(), candidate.clone()))
        .collect::<BTreeMap<_, _>>();

    let mut result = Vec::new();
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();

    for key in candidate_map.keys() {
        visit_candidate(
            key,
            &candidate_map,
            blueprint,
            &mut visiting,
            &mut visited,
            &mut result,
        );
    }

    result
}

fn visit_candidate(
    key: &str,
    candidates: &BTreeMap<String, ImplementationCandidate>,
    blueprint: &ProgramBlueprint,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    result: &mut Vec<ImplementationCandidate>,
) {
    if visited.contains(key) || visiting.contains(key) {
        return;
    }
    visiting.insert(key.to_string());

    for dependency in candidate_dependencies(key, blueprint) {
        if candidates.contains_key(&dependency) {
            visit_candidate(
                &dependency,
                candidates,
                blueprint,
                visiting,
                visited,
                result,
            );
        }
    }

    visiting.remove(key);
    if let Some(candidate) = candidates.get(key) {
        visited.insert(key.to_string());
        result.push(candidate.clone());
    }
}

fn candidate_dependencies(key: &str, blueprint: &ProgramBlueprint) -> Vec<String> {
    let Some((unit_id, lane_id)) = key.split_once(':') else {
        return Vec::new();
    };
    let Some(unit) = blueprint.units.iter().find(|unit| unit.id == unit_id) else {
        return Vec::new();
    };
    let Some(lane) = unit.lanes.iter().find(|lane| lane.id == lane_id) else {
        return Vec::new();
    };

    let mut dependencies = Vec::new();
    for dependency in &lane.dependencies {
        let Some(dependency_unit) = blueprint
            .units
            .iter()
            .find(|candidate| candidate.id == dependency.unit)
        else {
            continue;
        };

        if let Some(dep_lane) = &dependency.lane {
            dependencies.push(lane_key(&dependency.unit, dep_lane));
            continue;
        }

        if let Some(dep_milestone) = &dependency.milestone {
            for candidate_lane in &dependency_unit.lanes {
                if candidate_lane.managed_milestone == *dep_milestone {
                    dependencies.push(lane_key(&dependency.unit, &candidate_lane.id));
                }
            }
        }
    }
    dependencies
}

fn blocker_contract_recommendations(
    blueprint: &ProgramBlueprint,
    lane_key_text: &str,
    blockers_text: &str,
    catalog: &BTreeMap<String, LaneCatalogEntry>,
) -> Vec<String> {
    let mut recommendations = Vec::new();
    let Some((unit_id, lane_id)) = lane_key_text.split_once(':') else {
        return recommendations;
    };
    let Some(unit) = blueprint
        .units
        .iter()
        .find(|candidate| candidate.id == unit_id)
    else {
        return recommendations;
    };
    let Some(lane) = unit.lanes.iter().find(|candidate| candidate.id == lane_id) else {
        return recommendations;
    };

    for blocker in blockers_text.split(", ").filter(|token| !token.is_empty()) {
        let Some(entry) = catalog.get(blocker) else {
            continue;
        };
        let same_program = entry.program_id == blueprint.program.id;
        if same_program {
            let has_dependency = lane.dependencies.iter().any(|dependency| {
                dependency.unit == entry.unit_id
                    && (dependency.lane.as_deref() == Some(entry.lane_id.as_str())
                        || dependency.milestone.as_deref()
                            == Some(entry.managed_milestone.as_str()))
            });
            if !has_dependency {
                recommendations.push(format!(
                    "tighten the contract for `{}` by adding a dependency on `{}` milestone `{}`",
                    lane_key_text, blocker, entry.managed_milestone
                ));
            }
        } else if let Some(review_artifact) = &entry.review_artifact {
            let has_check = lane_has_check_on_path(lane, review_artifact);
            if !has_check {
                recommendations.push(format!(
                    "tighten the contract for `{}` by adding a precondition check on `{}` from `{}`",
                    lane_key_text,
                    review_artifact.display(),
                    blocker
                ));
            }
        }
    }

    recommendations
}

fn apply_blocker_contract_tightening(
    lane: &mut BlueprintLane,
    blueprint: &ProgramBlueprint,
    catalog: &BTreeMap<String, LaneCatalogEntry>,
    blockers: &[String],
) {
    for blocker in blockers {
        let Some(entry) = catalog.get(blocker) else {
            continue;
        };
        let same_program = entry.program_id == blueprint.program.id;
        if same_program {
            let has_dependency = lane.dependencies.iter().any(|dependency| {
                dependency.unit == entry.unit_id
                    && (dependency.lane.as_deref() == Some(entry.lane_id.as_str())
                        || dependency.milestone.as_deref()
                            == Some(entry.managed_milestone.as_str()))
            });
            if !has_dependency {
                lane.dependencies
                    .push(raspberry_supervisor::manifest::LaneDependency {
                        unit: entry.unit_id.clone(),
                        lane: None,
                        milestone: Some(entry.managed_milestone.clone()),
                    });
            }
            continue;
        }

        let Some(review_artifact) = &entry.review_artifact else {
            continue;
        };
        if lane_has_check_on_path(lane, review_artifact) {
            continue;
        }
        lane.checks.push(raspberry_supervisor::manifest::LaneCheck {
            label: blocker_review_check_label(blocker),
            kind: raspberry_supervisor::manifest::LaneCheckKind::Precondition,
            scope: raspberry_supervisor::manifest::LaneCheckScope::Ready,
            probe: raspberry_supervisor::manifest::LaneCheckProbe::FileExists {
                path: review_artifact.clone(),
            },
        });
    }
}

fn blocker_review_check_label(blocker: &str) -> String {
    format!(
        "{}_review_ready",
        blocker
            .chars()
            .map(|ch| match ch {
                ':' | '-' => '_',
                _ => ch,
            })
            .collect::<String>()
    )
}

fn blocker_milestone_refinement_recommendations(
    findings: &[String],
    catalog: &BTreeMap<String, LaneCatalogEntry>,
) -> Vec<String> {
    let mut recommendations = Vec::new();

    for finding in findings {
        let Some(rest) = finding.strip_prefix("review artifact for lane `") else {
            continue;
        };
        let Some((_, rest)) = rest.split_once("` requires blocker `") else {
            continue;
        };
        let Some((blocker, detail)) = rest.split_once("` to ") else {
            continue;
        };
        let Some(detail) = detail.strip_suffix(" before implementation") else {
            continue;
        };
        let Some(entry) = catalog.get(blocker) else {
            continue;
        };

        recommendations.push(format!(
            "refine the upstream contract for `{blocker}` beyond milestone `{}` so downstream work can gate on {} instead of only `{}`",
            entry.managed_milestone, detail, entry.managed_milestone
        ));
    }

    recommendations
}

fn lane_has_check_on_path(lane: &BlueprintLane, expected: &Path) -> bool {
    use raspberry_supervisor::manifest::LaneCheckProbe;

    lane.checks.iter().any(|check| match &check.probe {
        LaneCheckProbe::FileExists { path } => path == expected,
        LaneCheckProbe::JsonFieldEquals { path, .. } => path == expected,
        LaneCheckProbe::CommandSucceeds { .. } => false,
        LaneCheckProbe::CommandStdoutContains { .. } => false,
    })
}

fn lane_catalog(target_repo: &Path) -> BTreeMap<String, LaneCatalogEntry> {
    let mut catalog = BTreeMap::new();
    let programs_dir = target_repo
        .join(crate::blueprint::DEFAULT_PACKAGE_DIR)
        .join("programs");
    let Ok(entries) = std::fs::read_dir(programs_dir) else {
        return catalog;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
            continue;
        }
        let Ok(manifest) = raspberry_supervisor::manifest::ProgramManifest::load(&path) else {
            continue;
        };
        for (unit_id, unit) in &manifest.units {
            for (lane_id, lane) in &unit.lanes {
                let key = lane_key(unit_id, lane_id);
                let review_artifact = lane_review_artifact_from_manifest(unit, lane);

                catalog.insert(
                    key,
                    LaneCatalogEntry {
                        program_id: manifest.program.clone(),
                        unit_id: unit_id.clone(),
                        lane_id: lane_id.clone(),
                        managed_milestone: lane.managed_milestone.clone(),
                        review_artifact,
                    },
                );
            }
        }
    }

    catalog
}

fn lane_review_artifact_from_manifest(
    unit: &raspberry_supervisor::manifest::UnitManifest,
    lane: &raspberry_supervisor::manifest::LaneManifest,
) -> Option<PathBuf> {
    let artifact_ids = if lane.produces.is_empty() {
        unit.artifacts.keys().cloned().collect::<Vec<_>>()
    } else {
        lane.produces.clone()
    };

    artifact_ids.iter().find_map(|artifact_id| {
        unit.artifacts
            .get(artifact_id)
            .filter(|artifact_path| {
                artifact_id.contains("review")
                    || artifact_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name == "review.md")
            })
            .map(|artifact_path| {
                normalize_relative_path(
                    &unit
                        .output_root
                        .clone()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join(artifact_path),
                )
            })
    })
}

fn normalize_relative_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(part) => normalized.push(part),
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => {}
        }
    }
    normalized
}

fn implementation_program_manifest_path(
    program_id: &str,
    unit_id: &str,
    lane_id: &str,
    target_repo: &Path,
) -> PathBuf {
    let repo_prefix = program_id.split('-').next().unwrap_or(program_id);
    target_repo
        .join(crate::blueprint::DEFAULT_PACKAGE_DIR)
        .join("programs")
        .join(format!(
            "{repo_prefix}-{unit_id}-{lane_id}-implementation.yaml"
        ))
}

fn dependency_satisfied(
    desired: &ProgramBlueprint,
    dependency: &raspberry_supervisor::manifest::LaneDependency,
    target_repo: &Path,
) -> bool {
    let Some(unit) = desired.units.iter().find(|unit| unit.id == dependency.unit) else {
        return false;
    };
    let Some(milestone_id) = dependency.milestone.as_ref() else {
        return true;
    };
    let Some(milestone) = unit
        .milestones
        .iter()
        .find(|milestone| &milestone.id == milestone_id)
    else {
        return false;
    };
    milestone.requires.iter().all(|artifact_id| {
        artifact_path_for(unit, artifact_id, target_repo).is_some_and(|path| path.exists())
    })
}

fn artifact_path_for(
    unit: &BlueprintUnit,
    artifact_id: &str,
    target_repo: &Path,
) -> Option<PathBuf> {
    unit.artifacts
        .iter()
        .find(|artifact| artifact.id == artifact_id)
        .map(|artifact| target_repo.join(&unit.output_root).join(&artifact.path))
}

fn lane_artifact_paths(
    blueprint: &ProgramBlueprint,
    unit_id: &str,
    lane: &BlueprintLane,
    target_repo: &Path,
) -> Vec<PathBuf> {
    let Some(unit) = blueprint.units.iter().find(|unit| unit.id == unit_id) else {
        return Vec::new();
    };
    let artifact_ids = if lane.produces.is_empty() {
        unit.artifacts
            .iter()
            .map(|artifact| artifact.id.clone())
            .collect::<Vec<_>>()
    } else {
        lane.produces.clone()
    };
    artifact_ids
        .iter()
        .filter_map(|artifact_id| artifact_path_for(unit, artifact_id, target_repo))
        .collect()
}

fn lane_artifact_paths_relative(
    blueprint: &ProgramBlueprint,
    unit_id: &str,
    lane: &BlueprintLane,
) -> Vec<PathBuf> {
    let Some(unit) = blueprint.units.iter().find(|unit| unit.id == unit_id) else {
        return Vec::new();
    };
    let artifact_ids = if lane.produces.is_empty() {
        unit.artifacts
            .iter()
            .map(|artifact| artifact.id.clone())
            .collect::<Vec<_>>()
    } else {
        lane.produces.clone()
    };
    artifact_ids
        .iter()
        .filter_map(|artifact_id| {
            unit.artifacts
                .iter()
                .find(|artifact| artifact.id == *artifact_id)
                .map(|artifact| {
                    join_relative(&unit.output_root, &artifact.path.display().to_string())
                })
        })
        .collect()
}

fn implementation_audit_command(
    blueprint: &ProgramBlueprint,
    unit_id: &str,
    lane: &BlueprintLane,
    promotion_command: &str,
) -> String {
    let paths = lane_artifact_paths_relative(blueprint, unit_id, lane);
    if paths.is_empty() {
        return "true".to_string();
    }

    let artifact_checks = paths
        .iter()
        .map(|path| format!("test -f {}", path.display()))
        .collect::<Vec<_>>()
        .join(" && ");

    format!("{artifact_checks} && {promotion_command}")
}

fn lane_review_artifact_path(
    unit: &BlueprintUnit,
    lane: &BlueprintLane,
    target_repo: &Path,
) -> Option<PathBuf> {
    let artifact_ids = if lane.produces.is_empty() {
        unit.artifacts
            .iter()
            .map(|artifact| artifact.id.clone())
            .collect::<Vec<_>>()
    } else {
        lane.produces.clone()
    };
    artifact_ids.iter().find_map(|artifact_id| {
        unit.artifacts
            .iter()
            .find(|artifact| artifact.id == *artifact_id)
            .filter(|artifact| {
                artifact.id.contains("review")
                    || artifact
                        .path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name == "review.md")
            })
            .map(|artifact| target_repo.join(&unit.output_root).join(&artifact.path))
    })
}

fn lane_named_artifact_path(unit: &BlueprintUnit, lane: &BlueprintLane, kind: &str) -> PathBuf {
    let artifact_ids = if lane.produces.is_empty() {
        unit.artifacts
            .iter()
            .map(|artifact| artifact.id.clone())
            .collect::<Vec<_>>()
    } else {
        lane.produces.clone()
    };

    artifact_ids
        .iter()
        .find_map(|artifact_id| {
            unit.artifacts
                .iter()
                .find(|artifact| artifact.id == *artifact_id)
                .and_then(|artifact| {
                    let file_name_matches = artifact
                        .path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name == format!("{kind}.md"));
                    if artifact.id.contains(kind) || file_name_matches {
                        Some(artifact.path.clone())
                    } else {
                        None
                    }
                })
        })
        .unwrap_or_else(|| join_relative(&lane_artifact_dir(unit, lane), &format!("{kind}.md")))
}

fn lane_named_artifact_path_relative(
    blueprint: &ProgramBlueprint,
    unit_id: &str,
    lane: &BlueprintLane,
    kind: &str,
) -> Option<PathBuf> {
    blueprint
        .units
        .iter()
        .find(|unit| unit.id == unit_id)
        .map(|unit| lane_named_artifact_path(unit, lane, kind))
}

fn lane_artifact_dir(unit: &BlueprintUnit, lane: &BlueprintLane) -> PathBuf {
    let artifact_ids = if lane.produces.is_empty() {
        unit.artifacts
            .iter()
            .map(|artifact| artifact.id.clone())
            .collect::<Vec<_>>()
    } else {
        lane.produces.clone()
    };

    artifact_ids
        .iter()
        .filter_map(|artifact_id| {
            unit.artifacts
                .iter()
                .find(|artifact| artifact.id == *artifact_id)
                .map(|artifact| artifact.path.clone())
        })
        .find_map(|path| {
            let parent = path.parent()?.to_path_buf();
            if parent.as_os_str().is_empty() {
                None
            } else {
                Some(parent)
            }
        })
        .unwrap_or_default()
}

fn join_relative(prefix: &Path, file_name: &str) -> PathBuf {
    if prefix.as_os_str().is_empty() {
        PathBuf::from(file_name)
    } else {
        prefix.join(file_name)
    }
}

fn run_config_relative_string(path: &Path) -> String {
    repo_relative_string(path, 3)
}

fn implementation_goal(
    unit: &BlueprintUnit,
    lane: &BlueprintLane,
    spec_path: &Path,
    review_path: &Path,
    quality_path: &Path,
    promotion_path: &Path,
    integration_path: &Path,
) -> String {
    format!(
        "Implement the next approved `{}` slice.\n\nInputs:\n- `{}`\n- `{}`\n\nScope:\n- work only inside the smallest next approved implementation slice\n- treat the reviewed lane artifacts as the source of truth\n- keep changes aligned with the owned surfaces for `{}`\n\nRequired curated artifacts:\n- `{}`\n- `{}`\n- `{}`\n- `{}`\n- `{}`",
        lane_key(&unit.id, &lane.id),
        spec_path.display(),
        review_path.display(),
        lane_key(&unit.id, &lane.id),
        join_relative(&lane_artifact_dir(unit, lane), "implementation.md").display(),
        join_relative(&lane_artifact_dir(unit, lane), "verification.md").display(),
        quality_path.display(),
        promotion_path.display(),
        integration_path.display(),
    )
}

#[allow(clippy::too_many_arguments)]
fn implementation_prompt_context(
    spec_path: &Path,
    review_path: &Path,
    implementation_path: &Path,
    verification_path: &Path,
    quality_path: &Path,
    promotion_path: &Path,
    integration_path: &Path,
    evidence: &ImplementationEvidence,
) -> String {
    let mut context = format!(
        "Use `{}` and `{}` as the approved contract. Implement only the smallest honest next slice, write what changed to `{}`, write proof results plus remaining risk to `{}`, rely on the machine-generated quality evidence in `{}`, write the merge/promotion verdict to `{}`, and ensure the required integration artifact exists at `{}` before the lane is considered complete.",
        spec_path.display(),
        review_path.display(),
        implementation_path.display(),
        verification_path.display(),
        quality_path.display(),
        promotion_path.display(),
        integration_path.display(),
    );

    if let Some(first_slice) = &evidence.first_slice {
        context.push_str("\n\nImplement now:");
        context.push_str(&format!("\n- {first_slice}"));
    }

    if let Some(first_code_surface) = &evidence.first_code_surface {
        context.push_str("\n\nTouch first:");
        context.push_str(&format!("\n- `{first_code_surface}`"));
    }

    if let Some(first_slice_work) = &evidence.first_slice_work {
        context.push_str("\n\nBuild in this slice:");
        context.push_str(&format!("\n- {first_slice_work}"));
    }

    if !evidence.setup_notes.is_empty() {
        context.push_str("\n\nSet up first:");
        for note in &evidence.setup_notes {
            context.push_str(&format!("\n- {note}"));
        }
    }

    if let Some(first_proof_gate) = &evidence.first_proof_gate {
        context.push_str("\n\nFirst proof gate:");
        context.push_str(&format!("\n- `{first_proof_gate}`"));
    }

    if let Some(first_smoke_gate) = &evidence.first_smoke_gate {
        context.push_str("\n\nFirst smoke gate:");
        context.push_str(&format!("\n- `{first_smoke_gate}`"));
    }

    let execution_guidance = execution_guidance_from_slice_notes(&evidence.slice_notes);
    if !execution_guidance.is_empty() {
        context.push_str("\n\nExecution guidance:");
        for note in &execution_guidance {
            context.push_str(&format!("\n- {note}"));
        }
    }

    if !evidence.manual_notes.is_empty() {
        context.push_str("\n\nManual proof still required after automated verification:");
        for note in &evidence.manual_notes {
            context.push_str(&format!("\n- {note}"));
        }
    }

    if let Some(first_health_gate) = &evidence.first_health_gate {
        context.push_str("\n\nFirst health gate:");
        context.push_str(&format!("\n- {first_health_gate}"));
    }

    if !evidence.health_notes.is_empty() {
        context.push_str("\n\nService/health surfaces to preserve:");
        for note in &evidence.health_notes {
            context.push_str(&format!("\n- {note}"));
        }
    }

    if !evidence.observability_notes.is_empty() {
        context.push_str("\n\nObservability surfaces to preserve:");
        for note in &evidence.observability_notes {
            context.push_str(&format!("\n- {note}"));
        }
    }

    context
}

fn execution_guidance_from_slice_notes(slice_notes: &[String]) -> Vec<String> {
    let mut guidance = Vec::new();

    for note in slice_notes {
        let trimmed = note.trim();
        let normalized = trim_list_prefix(trimmed);
        let lower = normalized.to_ascii_lowercase();
        let prefix = if lower.contains("can begin with slice") || lower.contains("start slices") {
            "Start"
        } else if lower.contains("parallelize") || lower.contains("in parallel") {
            "Parallel"
        } else if lower.contains("must succeed before") || lower.contains("must precede") {
            "Order"
        } else if lower.contains("independent") {
            "Parallel"
        } else if lower.contains("phase 0 only") {
            "Scope"
        } else {
            "Note"
        };
        guidance.push(format!("{prefix}: {normalized}"));
    }

    guidance
}

fn trim_list_prefix(value: &str) -> &str {
    let trimmed = value.trim();
    let chars = trimmed.chars();
    let mut consumed = 0usize;

    for ch in chars {
        if ch.is_ascii_digit() {
            consumed += ch.len_utf8();
            continue;
        }
        if ch == '.' || ch == ')' {
            consumed += ch.len_utf8();
            break;
        }
        return trimmed;
    }

    trimmed[consumed..].trim_start()
}

fn implementation_proof_profile(lane: &BlueprintLane) -> String {
    let slug = lane.slug().replace('-', "_");
    format!("{slug}_implement")
}

fn implementation_health_command(evidence: &ImplementationEvidence) -> Option<String> {
    if evidence.health_commands.is_empty() {
        None
    } else {
        Some(format!("set -e\n{}", evidence.health_commands.join("\n")))
    }
}

fn lane_is_user_visible(lane: &BlueprintLane) -> bool {
    matches!(
        lane.kind,
        raspberry_supervisor::manifest::LaneKind::Interface
            | raspberry_supervisor::manifest::LaneKind::Service
    )
}

fn implementation_checks(
    blueprint: &ProgramBlueprint,
    unit: &BlueprintUnit,
    lane: &BlueprintLane,
    verify_command: &str,
) -> Vec<raspberry_supervisor::manifest::LaneCheck> {
    let mut checks = lane.checks.clone();
    checks.extend(dependency_artifact_checks(blueprint, lane));

    if verify_command != "true" && !has_command_check(&checks, verify_command) {
        checks.push(raspberry_supervisor::manifest::LaneCheck {
            label: format!("{}_implementation_proof", lane.slug().replace('-', "_")),
            kind: raspberry_supervisor::manifest::LaneCheckKind::Proof,
            scope: raspberry_supervisor::manifest::LaneCheckScope::Running,
            probe: raspberry_supervisor::manifest::LaneCheckProbe::CommandSucceeds {
                command: verify_command.to_string(),
            },
        });
    }

    dedupe_checks(checks, unit, lane)
}

fn dependency_artifact_checks(
    blueprint: &ProgramBlueprint,
    lane: &BlueprintLane,
) -> Vec<raspberry_supervisor::manifest::LaneCheck> {
    let mut checks = Vec::new();

    for dependency in &lane.dependencies {
        let Some(unit) = blueprint
            .units
            .iter()
            .find(|candidate| candidate.id == dependency.unit)
        else {
            continue;
        };

        if let Some(dep_lane_id) = &dependency.lane {
            let Some(dep_lane) = unit
                .lanes
                .iter()
                .find(|candidate| &candidate.id == dep_lane_id)
            else {
                continue;
            };
            for path in lane_artifact_paths(blueprint, &unit.id, dep_lane, Path::new(".")) {
                checks.push(raspberry_supervisor::manifest::LaneCheck {
                    label: dependency_artifact_check_label(&dependency.unit, dep_lane_id, &path),
                    kind: raspberry_supervisor::manifest::LaneCheckKind::Precondition,
                    scope: raspberry_supervisor::manifest::LaneCheckScope::Ready,
                    probe: raspberry_supervisor::manifest::LaneCheckProbe::FileExists { path },
                });
            }
            continue;
        }

        let Some(milestone_id) = &dependency.milestone else {
            continue;
        };
        let Some(milestone) = unit
            .milestones
            .iter()
            .find(|candidate| &candidate.id == milestone_id)
        else {
            continue;
        };

        for artifact_id in &milestone.requires {
            let Some(path) = artifact_path_for(unit, artifact_id, Path::new(".")) else {
                continue;
            };
            checks.push(raspberry_supervisor::manifest::LaneCheck {
                label: dependency_artifact_check_label(&dependency.unit, milestone_id, &path),
                kind: raspberry_supervisor::manifest::LaneCheckKind::Precondition,
                scope: raspberry_supervisor::manifest::LaneCheckScope::Ready,
                probe: raspberry_supervisor::manifest::LaneCheckProbe::FileExists { path },
            });
        }
    }

    checks
}

fn dependency_artifact_check_label(owner: &str, scope: &str, path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("artifact");
    format!(
        "{}_{}_{}_ready",
        owner.replace([':', '-'], "_"),
        scope.replace([':', '-'], "_"),
        stem.replace('-', "_"),
    )
}

fn has_command_check(checks: &[raspberry_supervisor::manifest::LaneCheck], command: &str) -> bool {
    checks.iter().any(|check| match &check.probe {
        raspberry_supervisor::manifest::LaneCheckProbe::CommandSucceeds { command: existing } => {
            existing == command
        }
        raspberry_supervisor::manifest::LaneCheckProbe::CommandStdoutContains {
            command: existing,
            ..
        } => existing == command,
        raspberry_supervisor::manifest::LaneCheckProbe::FileExists { .. }
        | raspberry_supervisor::manifest::LaneCheckProbe::JsonFieldEquals { .. } => false,
    })
}

fn dedupe_checks(
    checks: Vec<raspberry_supervisor::manifest::LaneCheck>,
    _unit: &BlueprintUnit,
    _lane: &BlueprintLane,
) -> Vec<raspberry_supervisor::manifest::LaneCheck> {
    let mut deduped = Vec::new();
    let mut seen = BTreeSet::new();

    for check in checks {
        let key = match &check.probe {
            raspberry_supervisor::manifest::LaneCheckProbe::FileExists { path } => {
                format!("file:{}", path.display())
            }
            raspberry_supervisor::manifest::LaneCheckProbe::JsonFieldEquals {
                path, field, ..
            } => {
                format!("json:{}:{field}", path.display())
            }
            raspberry_supervisor::manifest::LaneCheckProbe::CommandSucceeds { command } => {
                format!("cmd:{command}")
            }
            raspberry_supervisor::manifest::LaneCheckProbe::CommandStdoutContains {
                command,
                contains,
            } => format!("stdout:{command}:{contains}"),
        };
        if seen.insert(key) {
            deduped.push(check);
        }
    }

    deduped
}

fn implementation_verify_command(
    lane: &BlueprintLane,
    evidence: &ImplementationEvidence,
) -> String {
    let mut commands = evidence.proof_commands.clone();
    if lane_is_user_visible(lane) {
        for command in &evidence.smoke_commands {
            if !commands.contains(command) {
                commands.push(command.clone());
            }
        }
    }
    if commands.is_empty() {
        return "true".to_string();
    }
    commands = normalize_verify_commands(lane, commands);

    format!("set -e\n{}", commands.join("\n"))
}

fn normalize_verify_commands(lane: &BlueprintLane, commands: Vec<String>) -> Vec<String> {
    if !lane_is_user_visible(lane) {
        return dedupe_commands(commands);
    }

    let mut normalized = commands;
    let pair_clients = normalized
        .iter()
        .enumerate()
        .filter_map(|(index, command)| pair_command_client(command).map(|client| (index, client)))
        .collect::<Vec<_>>();
    let control_clients = normalized
        .iter()
        .filter_map(|command| control_command_client(command))
        .collect::<BTreeSet<_>>();

    if pair_clients
        .iter()
        .any(|(_, client)| client == "alice-phone")
    {
        for command in &mut normalized {
            if is_default_bootstrap_home_miner_command(command) {
                *command = format!("DEVICE_NAME=bootstrap-phone {command}");
            }
        }
    }

    for client in control_clients {
        if let Some((index, _)) = pair_clients.iter().find(|(_, paired)| paired == &client) {
            normalized[*index] = ensure_pair_command_has_control(&normalized[*index]);
            continue;
        }
        if let Some(index) = normalized
            .iter()
            .position(|command| control_command_client(command).as_deref() == Some(client.as_str()))
        {
            normalized.insert(
                index,
                format!(
                    "./scripts/pair_gateway_client.sh --client {} --capabilities observe,control",
                    client
                ),
            );
        }
    }

    dedupe_commands(normalized)
}

fn dedupe_commands(commands: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for command in commands {
        if seen.insert(command.clone()) {
            deduped.push(command);
        }
    }
    deduped
}

fn is_default_bootstrap_home_miner_command(command: &str) -> bool {
    command.contains("./scripts/bootstrap_home_miner.sh") && !command.contains("DEVICE_NAME=")
}

fn pair_command_client(command: &str) -> Option<String> {
    if !command.contains("pair_gateway_client.sh") {
        return None;
    }
    command_flag_value(command, "--client")
}

fn control_command_client(command: &str) -> Option<String> {
    if command.contains("set_mining_mode.sh") {
        return command_flag_value(command, "--client");
    }
    if command.contains("cli.py control") {
        return command_flag_value(command, "--client");
    }
    None
}

fn ensure_pair_command_has_control(command: &str) -> String {
    if !command.contains("pair_gateway_client.sh") {
        return command.to_string();
    }
    if let Some(capabilities) = command_flag_value(command, "--capabilities") {
        if capabilities
            .split(',')
            .any(|capability| capability.trim() == "control")
        {
            return command.to_string();
        }
        return command.replacen(
            &format!("--capabilities {capabilities}"),
            "--capabilities observe,control",
            1,
        );
    }
    format!("{command} --capabilities observe,control")
}

fn command_flag_value(command: &str, flag: &str) -> Option<String> {
    let tokens = command.split_whitespace().collect::<Vec<_>>();
    tokens
        .windows(2)
        .find_map(|window| (window[0] == flag).then(|| window[1].to_string()))
}

fn implementation_evidence(
    unit: &BlueprintUnit,
    lane: &BlueprintLane,
    target_repo: &Path,
) -> ImplementationEvidence {
    let review_path = target_repo
        .join(&unit.output_root)
        .join(lane_named_artifact_path(unit, lane, "review"));
    let spec_path = target_repo
        .join(&unit.output_root)
        .join(lane_named_artifact_path(unit, lane, "spec"));
    let review = std::fs::read_to_string(&review_path).ok();
    let spec = std::fs::read_to_string(&spec_path).ok();

    let proof_commands = review
        .as_deref()
        .map(proof_commands_from_markdown)
        .filter(|commands| !commands.is_empty())
        .or_else(|| {
            spec.as_deref()
                .map(proof_commands_from_markdown)
                .filter(|commands| !commands.is_empty())
        })
        .unwrap_or_default();
    let smoke_commands = review
        .as_deref()
        .map(smoke_commands_from_markdown)
        .filter(|commands| !commands.is_empty())
        .or_else(|| {
            spec.as_deref()
                .map(smoke_commands_from_markdown)
                .filter(|commands| !commands.is_empty())
        })
        .unwrap_or_default();

    ImplementationEvidence {
        first_slice: spec
            .as_deref()
            .and_then(first_slice_from_markdown)
            .or_else(|| review.as_deref().and_then(first_slice_from_markdown)),
        first_code_surface: spec
            .as_deref()
            .and_then(first_code_surface_from_markdown)
            .or_else(|| review.as_deref().and_then(first_code_surface_from_markdown)),
        first_slice_work: spec
            .as_deref()
            .and_then(first_slice_work_from_markdown)
            .or_else(|| review.as_deref().and_then(first_slice_work_from_markdown)),
        first_proof_gate: spec
            .as_deref()
            .and_then(first_proof_gate_from_markdown)
            .or_else(|| review.as_deref().and_then(first_proof_gate_from_markdown)),
        first_smoke_gate: review
            .as_deref()
            .and_then(first_smoke_gate_from_markdown)
            .or_else(|| spec.as_deref().and_then(first_smoke_gate_from_markdown)),
        first_health_gate: review.as_deref().and_then(first_health_gate_from_markdown),
        setup_notes: spec
            .as_deref()
            .map(setup_notes_from_markdown)
            .unwrap_or_default(),
        proof_commands,
        smoke_commands,
        health_commands: review
            .as_deref()
            .map(health_commands_from_markdown)
            .unwrap_or_default(),
        manual_notes: review
            .as_deref()
            .map(manual_notes_from_markdown)
            .unwrap_or_default(),
        slice_notes: review
            .as_deref()
            .map(slice_notes_from_markdown)
            .unwrap_or_default(),
        health_notes: review
            .as_deref()
            .map(health_notes_from_markdown)
            .unwrap_or_default(),
        observability_notes: review
            .as_deref()
            .map(observability_notes_from_markdown)
            .unwrap_or_default(),
    }
}

fn first_health_gate_from_markdown(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.to_ascii_lowercase().contains("health check") {
            continue;
        }
        if let Some(surface) = backticked_segments(trimmed).into_iter().next() {
            return Some(surface);
        }
    }
    None
}

fn first_smoke_gate_from_markdown(text: &str) -> Option<String> {
    smoke_commands_from_markdown(text).into_iter().next()
}

fn proof_commands_from_markdown(text: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut in_fence = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if looks_like_shell_command(trimmed) {
            commands.push(trimmed.to_string());
        }
    }

    if !commands.is_empty() {
        return commands;
    }

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(command) = inline_proof_command(trimmed) {
            commands.push(command);
        }
    }

    commands
}

fn smoke_commands_from_markdown(text: &str) -> Vec<String> {
    let mut commands = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if !(lower.contains("smoke") || lower.contains("runtime check")) {
            continue;
        }
        if let Some(command) = extract_backticked_command(trimmed) {
            if !commands.contains(&command) {
                commands.push(command);
            }
            continue;
        }
        if let Some(command) = inline_smoke_command(trimmed) {
            if !commands.contains(&command) {
                commands.push(command);
            }
        }
    }

    commands
}

fn first_slice_from_markdown(text: &str) -> Option<String> {
    if let Some(slice) = first_slice_from_header(text) {
        return Some(slice);
    }
    if let Some(slice) = first_slice_from_table(text) {
        return Some(slice);
    }
    first_slice_from_start_line(text)
}

fn first_slice_from_table(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let columns = trimmed
            .split('|')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if columns.len() < 2 || columns[0] != "1" {
            continue;
        }
        let description = columns[1].replace('`', "");
        let description = description.trim();
        if !description.is_empty() {
            return Some(format!("Slice 1: {description}"));
        }
    }
    None
}

fn first_slice_from_start_line(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if !lower.contains("begin with slice 1") {
            continue;
        }
        if let Some(detail) = paren_detail_after_slice(trimmed) {
            return Some(format!("Slice 1: {detail}"));
        }
        return Some(trimmed.to_string());
    }
    None
}

fn first_slice_from_header(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.to_ascii_lowercase().contains("slice 1") {
            continue;
        }
        let Some((_, detail)) = trimmed
            .split_once("—")
            .or_else(|| trimmed.split_once(':'))
            .or_else(|| trimmed.split_once("-"))
        else {
            continue;
        };
        let detail = detail.replace('`', "");
        let detail = detail.trim();
        if !detail.is_empty() {
            return Some(format!("Slice 1: {detail}"));
        }
    }
    None
}

fn first_proof_gate_from_markdown(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("**Proof")
            || trimmed.starts_with("Proof:")
            || trimmed.starts_with("Proof gate:")
        {
            if let Some(command) = extract_backticked_command(trimmed) {
                return Some(command);
            }
            if let Some(command) = inline_proof_command(trimmed) {
                return Some(command);
            }
        }
    }

    proof_commands_from_markdown(text).into_iter().next()
}

fn first_code_surface_from_markdown(text: &str) -> Option<String> {
    let section = slice_one_section(text)?;
    for line in section.lines() {
        let trimmed = line.trim();
        if !(trimmed.starts_with("**Files**:") || trimmed.starts_with("**File**:")) {
            continue;
        }
        let surfaces = backticked_segments(trimmed);
        if let Some(surface) = surfaces.first() {
            return Some(surface.clone());
        }
        let plain = trimmed
            .split_once(':')
            .map(|(_, rest)| rest.trim())
            .filter(|rest| !rest.is_empty())?;
        return Some(plain.to_string());
    }
    None
}

fn first_slice_work_from_markdown(text: &str) -> Option<String> {
    let section = slice_one_section(text)?;
    for line in section.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("**What**:") {
            let detail = rest.trim().trim_matches('`').trim();
            if !detail.is_empty() {
                return Some(detail.to_string());
            }
        }
    }
    None
}

fn setup_notes_from_markdown(text: &str) -> Vec<String> {
    let Some(section) = slice_one_section(text) else {
        return Vec::new();
    };

    let mut notes = Vec::new();
    let mut capture_bullets = false;

    for line in section.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            capture_bullets = false;
            continue;
        }
        if trimmed.starts_with("**Proof") {
            break;
        }
        if trimmed.starts_with("### Slice 1") || trimmed.starts_with("# Slice 1") {
            continue;
        }
        if trimmed.starts_with("**Files**:")
            || trimmed.starts_with("**File**:")
            || trimmed.starts_with("**What**:")
        {
            capture_bullets = false;
            continue;
        }
        if trimmed.starts_with("Add ") {
            notes.push(trimmed.to_string());
            capture_bullets = trimmed.ends_with(':');
            continue;
        }
        if trimmed.ends_with("Cargo.toml:") {
            capture_bullets = true;
            continue;
        }
        if capture_bullets && trimmed.starts_with("- ") {
            notes.push(trimmed.trim_start_matches("- ").trim().to_string());
            continue;
        }
        if trimmed.starts_with("`lib.rs`") || trimmed.starts_with("`main.rs`") {
            notes.push(trimmed.replace('`', ""));
        }
    }

    if notes.len() > 5 {
        notes.truncate(5);
    }
    notes
}

fn slice_one_section(text: &str) -> Option<String> {
    let mut in_slice_one = false;
    let mut lines = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if !in_slice_one {
            if lower.starts_with("### slice 1") || lower.starts_with("# slice 1") {
                in_slice_one = true;
                lines.push(trimmed.to_string());
            }
            continue;
        }

        if (lower.starts_with("### slice ") || lower.starts_with("# slice "))
            && !lower.contains("slice 1")
        {
            break;
        }
        lines.push(trimmed.to_string());
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn backticked_segments(line: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut rest = line;

    loop {
        let Some(start) = rest.find('`') else {
            break;
        };
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        let candidate = after_start[..end].trim();
        if !candidate.is_empty() {
            segments.push(candidate.to_string());
        }
        rest = &after_start[end + 1..];
    }

    segments
}

fn paren_detail_after_slice(line: &str) -> Option<String> {
    let slice_index = line.to_ascii_lowercase().find("slice 1")?;
    let tail = &line[slice_index..];
    let open = tail.find('(')?;
    let after_open = &tail[open + 1..];
    let close = after_open.find(')')?;
    let detail = after_open[..close].trim().trim_matches('`');
    if detail.is_empty() {
        None
    } else {
        Some(detail.to_string())
    }
}

fn health_commands_from_markdown(text: &str) -> Vec<String> {
    let mut commands = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(command) = extract_backticked_command(trimmed) {
            if command.starts_with("curl ") {
                commands.push(command);
            }
        }
    }

    commands
}

fn extract_backticked_command(line: &str) -> Option<String> {
    let start = line.find('`')?;
    let rest = &line[start + 1..];
    let end = rest.find('`')?;
    let candidate = rest[..end].trim();
    if looks_like_shell_command(candidate) {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn manual_notes_from_markdown(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("manual;") || lower.contains("manual proof")
        })
        .map(|line| {
            line.trim_start_matches('#')
                .trim()
                .trim_matches('`')
                .trim()
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .collect()
}

fn slice_notes_from_markdown(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            if line.contains('|') {
                return false;
            }
            (lower.starts_with(|ch: char| ch.is_ascii_digit())
                && (lower.contains("must precede")
                    || lower.contains("must succeed before")
                    || lower.contains("independent")
                    || lower.contains("start slices")
                    || lower.contains("parallelize")
                    || lower.contains("in parallel")))
                || lower.contains("phase 0 only")
                || lower.contains("can begin with slice")
        })
        .map(str::to_string)
        .collect()
}

fn health_notes_from_markdown(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("/health")
                || lower.contains("health check")
                || lower.contains("health surface")
                || lower.contains("get /health")
        })
        .map(str::to_string)
        .collect()
}

fn observability_notes_from_markdown(text: &str) -> Vec<String> {
    let mut notes = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower.contains("structured log")
            || lower.contains("operator dashboards")
            || trimmed.starts_with("{\"level\":")
        {
            notes.push(trimmed.to_string());
        }
    }

    notes
}

fn inline_proof_command(line: &str) -> Option<String> {
    for marker in ["**Proof**:", "**Proof gate**:", "Proof:", "Proof gate:"] {
        let Some(rest) = line.strip_prefix(marker) else {
            continue;
        };
        let candidate = rest.trim().trim_matches('`').trim();
        if looks_like_shell_command(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

fn inline_smoke_command(line: &str) -> Option<String> {
    for marker in [
        "**Smoke**:",
        "**Smoke gate**:",
        "Smoke:",
        "Smoke gate:",
        "**Runtime check**:",
        "Runtime check:",
    ] {
        let Some(rest) = line.strip_prefix(marker) else {
            continue;
        };
        let candidate = rest.trim().trim_matches('`').trim();
        if looks_like_shell_command(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

fn looks_like_shell_command(line: &str) -> bool {
    ["cargo ", "git ", "./", "test ", "fabro ", "myosu-", "curl "]
        .iter()
        .any(|prefix| line.starts_with(prefix))
}

fn missing_implementation_package(
    target_repo: &Path,
    lane: &BlueprintLane,
) -> Option<(PathBuf, PathBuf)> {
    let (run_config, workflow) = implementation_package_paths(target_repo, lane);
    if run_config.exists() && workflow.exists() {
        None
    } else {
        Some((run_config, workflow))
    }
}

fn implementation_package_paths(target_repo: &Path, lane: &BlueprintLane) -> (PathBuf, PathBuf) {
    let run_config = target_repo
        .join(crate::blueprint::DEFAULT_PACKAGE_DIR)
        .join("run-configs")
        .join("implement")
        .join(format!("{}.toml", lane.slug()));
    let workflow = target_repo
        .join(crate::blueprint::DEFAULT_PACKAGE_DIR)
        .join("workflows")
        .join("implement")
        .join(format!("{}.fabro", lane.slug()));
    (run_config, workflow)
}

fn lane_artifacts_satisfied(
    current: &ProgramBlueprint,
    lane_key: &str,
    target_repo: &Path,
) -> bool {
    let Some((unit_id, lane_id)) = lane_key.split_once(':') else {
        return false;
    };
    let Some(unit) = current.units.iter().find(|unit| unit.id == unit_id) else {
        return false;
    };
    let Some(lane) = unit.lanes.iter().find(|lane| lane.id == lane_id) else {
        return false;
    };
    let paths = lane_artifact_paths(current, unit_id, lane, target_repo);
    !paths.is_empty() && paths.iter().all(|path| path.exists())
}

fn check_satisfied(check: &raspberry_supervisor::manifest::LaneCheck, target_repo: &Path) -> bool {
    use raspberry_supervisor::manifest::LaneCheckProbe;

    match &check.probe {
        LaneCheckProbe::FileExists { path } => target_repo.join(path).exists(),
        LaneCheckProbe::JsonFieldEquals {
            path,
            field,
            equals,
        } => {
            let absolute = target_repo.join(path);
            let Ok(raw) = std::fs::read_to_string(absolute) else {
                return false;
            };
            let Ok(value) = serde_json::from_str::<Value>(&raw) else {
                return false;
            };
            value.get(field).is_some_and(|actual| actual == equals)
        }
        LaneCheckProbe::CommandSucceeds { .. } => false,
        LaneCheckProbe::CommandStdoutContains { .. } => false,
    }
}

fn lane_key(unit_id: &str, lane_id: &str) -> String {
    format!("{unit_id}:{lane_id}")
}

fn source_lane_managed_milestone(
    blueprint: &ProgramBlueprint,
    unit_id: &str,
    lane_id: &str,
) -> String {
    blueprint
        .units
        .iter()
        .find(|unit| unit.id == unit_id)
        .and_then(|unit| unit.lanes.iter().find(|lane| lane.id == lane_id))
        .map(|lane| lane.managed_milestone.clone())
        .unwrap_or_else(|| "reviewed".to_string())
}

fn review_says_implementation_ready(text: &str) -> bool {
    text.contains("implementation lane can begin")
        || text.contains("implementation-family workflow immediately")
        || text.contains("unblocked for an implementation-family workflow")
        || text.contains("ready for an implementation-family workflow")
        || text.contains("can begin with slice 1 immediately")
}

fn review_says_implementation_blocked(text: &str) -> bool {
    text.contains("implementation blocked")
        || text.contains("not yet unblocked")
        || text.contains("cannot begin honest implementation until")
        || text.contains("implementation is still blocked")
}

fn review_blocker_lane_refs(text: &str, allowed: &BTreeSet<String>) -> Vec<String> {
    raw_lane_refs(text)
        .into_iter()
        .filter(|candidate| allowed.contains(candidate))
        .collect()
}

fn blocked_review_refs(text: &str, allowed: &BTreeSet<String>, lane_key_text: &str) -> Vec<String> {
    review_blocker_lane_refs(text, allowed)
        .into_iter()
        .filter(|candidate| candidate != lane_key_text)
        .collect()
}

fn raw_lane_refs(text: &str) -> Vec<String> {
    let mut refs = BTreeSet::new();
    for raw in text.split_whitespace() {
        let token = raw.trim_matches(|ch: char| {
            !ch.is_ascii_alphanumeric() && ch != ':' && ch != '-' && ch != '_'
        });
        if token.contains(':')
            && token
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == ':' || ch == '-' || ch == '_')
        {
            refs.insert(token.to_string());
        }
    }
    refs.into_iter().collect()
}

fn known_lane_refs(target_repo: &Path) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    let programs_dir = target_repo
        .join(crate::blueprint::DEFAULT_PACKAGE_DIR)
        .join("programs");
    let Ok(entries) = std::fs::read_dir(programs_dir) else {
        return refs;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
            continue;
        }
        let Ok(manifest) = raspberry_supervisor::manifest::ProgramManifest::load(&path) else {
            continue;
        };
        for (unit_id, unit) in manifest.units {
            for (lane_id, _) in unit.lanes {
                refs.insert(lane_key(&unit_id, &lane_id));
            }
        }
    }
    refs
}

fn review_stage_requirements(
    text: &str,
    allowed: &BTreeSet<String>,
) -> Vec<ReviewStageRequirement> {
    let mut requirements = Vec::new();
    let mut seen = BTreeSet::new();

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_lowercase();
        if !lower.contains("must ") {
            continue;
        }

        let refs = review_blocker_lane_refs(&lower, allowed);
        if refs.len() != 1 {
            continue;
        }
        let blocker = refs[0].clone();
        let Some(detail) = extract_requirement_detail(line, &blocker) else {
            continue;
        };
        let key = format!("{blocker}\x1f{detail}");
        if !seen.insert(key) {
            continue;
        }
        requirements.push(ReviewStageRequirement { blocker, detail });
    }

    requirements
}

fn extract_requirement_detail(line: &str, blocker: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let blocker_start = lower.find(blocker)?;
    let after_blocker_start = blocker_start + blocker.len();
    let after_blocker_lower = &lower[after_blocker_start..];
    let must_offset = after_blocker_lower.find("must ")?;
    let detail_start = after_blocker_start + must_offset + "must ".len();
    let detail = line[detail_start..].trim().trim_end_matches('.');
    if detail.is_empty() {
        return None;
    }
    Some(detail.to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    use git2::Repository;

    use crate::blueprint::{
        BlueprintArtifact, BlueprintInputs, BlueprintLane, BlueprintPackage, BlueprintProgram,
        BlueprintUnit, ProgramBlueprint, WorkflowTemplate,
    };

    use super::{
        apply_blocker_contract_tightening, augment_with_implementation_follow_on_units,
        backticked_segments, blocked_review_refs, blocker_milestone_refinement_recommendations,
        execution_guidance_from_slice_notes, extract_requirement_detail,
        first_code_surface_from_markdown, first_health_gate_from_markdown,
        first_proof_gate_from_markdown, first_slice_from_markdown, first_slice_work_from_markdown,
        first_smoke_gate_from_markdown, health_commands_from_markdown, health_notes_from_markdown,
        implementation_blueprint_for_candidate, implementation_candidates, implementation_goal,
        implementation_quality_command, implementation_verify_command, inline_proof_command,
        looks_like_shell_command, manual_notes_from_markdown, observability_notes_from_markdown,
        prompt_context_block, proof_commands_from_markdown, raw_lane_refs, render_prompt,
        render_run_config, render_workflow_graph, review_blocker_lane_refs,
        review_stage_requirements, setup_notes_from_markdown, slice_notes_from_markdown,
        smoke_commands_from_markdown, trim_list_prefix, ImplementationEvidence, LaneCatalogEntry,
        ReviewStageRequirement,
    };

    #[test]
    fn raw_lane_refs_finds_lane_like_tokens() {
        let text = "cannot begin honest implementation until `games:poker-engine` is complete and `chain:pallet` restart lands while `miner:service` stays unfinished";
        let refs = raw_lane_refs(text);
        assert!(refs.contains(&"games:poker-engine".to_string()));
        assert!(refs.contains(&"chain:pallet".to_string()));
        assert!(refs.contains(&"miner:service".to_string()));
    }

    #[test]
    fn review_blocker_lane_refs_filters_to_known_lanes() {
        let text = "blocked by `games:poker-engine`, `chain:pallet`, AC-OR-03, and chain::tests::connect_and_query";
        let allowed =
            BTreeSet::from(["games:poker-engine".to_string(), "chain:pallet".to_string()]);
        let refs = review_blocker_lane_refs(text, &allowed);
        assert_eq!(
            refs,
            vec!["chain:pallet".to_string(), "games:poker-engine".to_string()]
        );
    }

    #[test]
    fn blocked_review_refs_excludes_the_current_lane() {
        let text = "blocked by `validator:oracle`, `games:poker-engine`, and `miner:service`";
        let allowed = BTreeSet::from([
            "validator:oracle".to_string(),
            "games:poker-engine".to_string(),
            "miner:service".to_string(),
        ]);

        let refs = blocked_review_refs(text, &allowed, "validator:oracle");

        assert_eq!(
            refs,
            vec![
                "games:poker-engine".to_string(),
                "miner:service".to_string()
            ]
        );
    }

    #[test]
    fn extract_requirement_detail_keeps_specific_stage_text() {
        let detail = extract_requirement_detail(
            "2. `chain:pallet` restart must complete through at least Phase 2 (storage + extrinsics available)",
            "chain:pallet",
        )
        .expect("detail");

        assert_eq!(
            detail,
            "complete through at least Phase 2 (storage + extrinsics available)"
        );
    }

    #[test]
    fn review_stage_requirements_extracts_blocker_stage_requirements() {
        let allowed = BTreeSet::from([
            "games:poker-engine".to_string(),
            "chain:pallet".to_string(),
            "miner:service".to_string(),
        ]);
        let text = r#"
Required before validator:oracle implementation-family workflow:
1. `games:poker-engine` must complete through Slice 5 (exploitability)
2. `chain:pallet` restart must complete through at least Phase 2 (storage + extrinsics available)
3. `miner:service` must complete through Slice 3 (MN-03 stable)
"#;

        let requirements = review_stage_requirements(text, &allowed);

        assert_eq!(
            requirements,
            vec![
                ReviewStageRequirement {
                    blocker: "games:poker-engine".to_string(),
                    detail: "complete through Slice 5 (exploitability)".to_string(),
                },
                ReviewStageRequirement {
                    blocker: "chain:pallet".to_string(),
                    detail: "complete through at least Phase 2 (storage + extrinsics available)"
                        .to_string(),
                },
                ReviewStageRequirement {
                    blocker: "miner:service".to_string(),
                    detail: "complete through Slice 3 (MN-03 stable)".to_string(),
                },
            ]
        );
    }

    #[test]
    fn blocker_milestone_refinement_recommendations_surface_specific_stage_gates() {
        let findings = vec![
            "review artifact for lane `validator:oracle` requires blocker `games:poker-engine` to complete through Slice 5 (exploitability) before implementation".to_string(),
        ];
        let catalog = BTreeMap::from([(
            "games:poker-engine".to_string(),
            LaneCatalogEntry {
                program_id: "myosu-platform".to_string(),
                unit_id: "games".to_string(),
                lane_id: "poker-engine".to_string(),
                managed_milestone: "poker_engine_reviewed".to_string(),
                review_artifact: Some(PathBuf::from("outputs/games/poker-engine/review.md")),
            },
        )]);

        let recommendations = blocker_milestone_refinement_recommendations(&findings, &catalog);

        assert_eq!(recommendations.len(), 1);
        assert!(recommendations[0].contains("games:poker-engine"));
        assert!(recommendations[0].contains("poker_engine_reviewed"));
        assert!(recommendations[0].contains("Slice 5 (exploitability)"));
    }

    #[test]
    fn augment_with_implementation_follow_on_units_adds_child_program_lane() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("outputs/private-control-plane"))
            .expect("outputs dir");
        std::fs::write(
            temp.path().join("outputs/private-control-plane/review.md"),
            "implementation lane can begin\n",
        )
        .expect("review artifact");

        let blueprint = ProgramBlueprint {
            version: 1,
            program: BlueprintProgram {
                id: "zend".to_string(),
                max_parallel: 1,
                state_path: Some(PathBuf::from(".raspberry/zend-state.json")),
                run_dir: None,
            },
            inputs: BlueprintInputs::default(),
            package: BlueprintPackage::default(),
            units: vec![crate::blueprint::BlueprintUnit {
                id: "private-control-plane".to_string(),
                title: "Private Control Plane".to_string(),
                output_root: PathBuf::from("outputs/private-control-plane"),
                artifacts: vec![
                    crate::blueprint::BlueprintArtifact {
                        id: "control_plane_contract".to_string(),
                        path: PathBuf::from("control-plane-contract.md"),
                    },
                    crate::blueprint::BlueprintArtifact {
                        id: "review".to_string(),
                        path: PathBuf::from("review.md"),
                    },
                ],
                milestones: vec![raspberry_supervisor::manifest::MilestoneManifest {
                    id: "reviewed".to_string(),
                    requires: vec!["control_plane_contract".to_string(), "review".to_string()],
                }],
                lanes: vec![BlueprintLane {
                    id: "private-control-plane".to_string(),
                    kind: raspberry_supervisor::manifest::LaneKind::Platform,
                    title: "Private Control Plane Lane".to_string(),
                    family: "bootstrap".to_string(),
                    workflow_family: Some("bootstrap".to_string()),
                    slug: Some("private-control-plane".to_string()),
                    template: WorkflowTemplate::Bootstrap,
                    goal: "Bootstrap the private control plane.".to_string(),
                    managed_milestone: "reviewed".to_string(),
                    dependencies: Vec::new(),
                    produces: vec!["control_plane_contract".to_string(), "review".to_string()],
                    proof_profile: None,
                    proof_state_path: None,
                    program_manifest: None,
                    service_state_path: None,
                    orchestration_state_path: None,
                    checks: Vec::new(),
                    run_dir: None,
                    prompt_context: None,
                    verify_command: None,
                    health_command: None,
                }],
            }],
        };

        let evolved =
            augment_with_implementation_follow_on_units(blueprint, temp.path()).expect("augment");

        assert!(evolved
            .units
            .iter()
            .any(|unit| unit.id == "private-control-plane-implementation"));
        let program_unit = evolved
            .units
            .iter()
            .find(|unit| unit.id == "private-control-plane-implementation")
            .expect("program unit exists");
        let program_lane = program_unit.lanes.first().expect("program lane exists");
        assert_eq!(program_lane.template, WorkflowTemplate::Orchestration);
        assert_eq!(
            program_lane.program_manifest.as_deref(),
            Some(Path::new(
                "malinka/programs/zend-private-control-plane-private-control-plane-implementation.yaml"
            ))
        );

        let candidate = implementation_candidates(&evolved, temp.path())
            .into_iter()
            .find(|candidate| candidate.lane_key == "private-control-plane:private-control-plane")
            .expect("candidate exists");
        let implementation_blueprint =
            implementation_blueprint_for_candidate(&evolved, &candidate, temp.path())
                .expect("implementation blueprint");
        let unit = implementation_blueprint
            .units
            .first()
            .expect("implementation unit exists");
        let spec_artifact = unit
            .artifacts
            .iter()
            .find(|artifact| artifact.id == "spec")
            .expect("spec artifact exists");
        assert_eq!(
            spec_artifact.path,
            PathBuf::from("control-plane-contract.md")
        );
    }

    #[test]
    fn augment_with_implementation_follow_on_units_uses_settled_bootstrap_artifacts() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("outputs/private-control-plane"))
            .expect("outputs dir");
        std::fs::write(
            temp.path().join("outputs/private-control-plane/review.md"),
            "bootstrap slice reviewed\n",
        )
        .expect("review artifact");
        std::fs::write(
            temp.path()
                .join("outputs/private-control-plane/control-plane-contract.md"),
            "contract\n",
        )
        .expect("primary artifact");

        let blueprint = ProgramBlueprint {
            version: 1,
            program: BlueprintProgram {
                id: "zend".to_string(),
                max_parallel: 1,
                state_path: Some(PathBuf::from(".raspberry/zend-state.json")),
                run_dir: None,
            },
            inputs: BlueprintInputs::default(),
            package: BlueprintPackage::default(),
            units: vec![crate::blueprint::BlueprintUnit {
                id: "private-control-plane".to_string(),
                title: "Private Control Plane".to_string(),
                output_root: PathBuf::from("outputs/private-control-plane"),
                artifacts: vec![
                    crate::blueprint::BlueprintArtifact {
                        id: "control_plane_contract".to_string(),
                        path: PathBuf::from("control-plane-contract.md"),
                    },
                    crate::blueprint::BlueprintArtifact {
                        id: "review".to_string(),
                        path: PathBuf::from("review.md"),
                    },
                ],
                milestones: vec![raspberry_supervisor::manifest::MilestoneManifest {
                    id: "reviewed".to_string(),
                    requires: vec!["control_plane_contract".to_string(), "review".to_string()],
                }],
                lanes: vec![BlueprintLane {
                    id: "private-control-plane".to_string(),
                    kind: raspberry_supervisor::manifest::LaneKind::Platform,
                    title: "Private Control Plane Lane".to_string(),
                    family: "bootstrap".to_string(),
                    workflow_family: Some("bootstrap".to_string()),
                    slug: Some("private-control-plane".to_string()),
                    template: WorkflowTemplate::Bootstrap,
                    goal: "Bootstrap the private control plane.".to_string(),
                    managed_milestone: "reviewed".to_string(),
                    dependencies: Vec::new(),
                    produces: vec!["control_plane_contract".to_string(), "review".to_string()],
                    proof_profile: None,
                    proof_state_path: None,
                    program_manifest: None,
                    service_state_path: None,
                    orchestration_state_path: None,
                    checks: Vec::new(),
                    run_dir: None,
                    prompt_context: None,
                    verify_command: None,
                    health_command: None,
                }],
            }],
        };

        let evolved =
            augment_with_implementation_follow_on_units(blueprint, temp.path()).expect("augment");

        assert!(evolved
            .units
            .iter()
            .any(|unit| unit.id == "private-control-plane-implementation"));
    }

    #[test]
    fn apply_blocker_contract_tightening_updates_lane_contract() {
        let blueprint = ProgramBlueprint {
            version: 1,
            program: BlueprintProgram {
                id: "myosu-services".to_string(),
                max_parallel: 2,
                state_path: None,
                run_dir: None,
            },
            inputs: BlueprintInputs::default(),
            package: BlueprintPackage::default(),
            units: Vec::new(),
        };
        let mut lane = BlueprintLane {
            id: "oracle".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Service,
            title: "Validator Oracle Lane".to_string(),
            family: "services".to_string(),
            workflow_family: Some("services".to_string()),
            slug: Some("validator-oracle".to_string()),
            template: WorkflowTemplate::ServiceBootstrap,
            goal: "Bootstrap validator".to_string(),
            managed_milestone: "reviewed".to_string(),
            dependencies: Vec::new(),
            produces: vec!["spec".to_string(), "review".to_string()],
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        };
        let catalog = BTreeMap::from([
            (
                "miner:service".to_string(),
                LaneCatalogEntry {
                    program_id: "myosu-services".to_string(),
                    unit_id: "miner".to_string(),
                    lane_id: "service".to_string(),
                    managed_milestone: "reviewed".to_string(),
                    review_artifact: Some(PathBuf::from("outputs/miner/service/review.md")),
                },
            ),
            (
                "games:poker-engine".to_string(),
                LaneCatalogEntry {
                    program_id: "myosu-platform".to_string(),
                    unit_id: "games".to_string(),
                    lane_id: "poker-engine".to_string(),
                    managed_milestone: "poker_engine_reviewed".to_string(),
                    review_artifact: Some(PathBuf::from("outputs/games/poker-engine/review.md")),
                },
            ),
        ]);

        apply_blocker_contract_tightening(
            &mut lane,
            &blueprint,
            &catalog,
            &[
                "miner:service".to_string(),
                "games:poker-engine".to_string(),
            ],
        );

        assert_eq!(lane.dependencies.len(), 1);
        assert_eq!(lane.dependencies[0].unit, "miner");
        assert_eq!(lane.dependencies[0].milestone.as_deref(), Some("reviewed"));

        assert_eq!(lane.checks.len(), 1);
        assert_eq!(lane.checks[0].label, "games_poker_engine_review_ready");
        match &lane.checks[0].probe {
            raspberry_supervisor::manifest::LaneCheckProbe::FileExists { path } => {
                assert_eq!(path, &PathBuf::from("outputs/games/poker-engine/review.md"));
            }
            probe => panic!("unexpected probe: {probe:?}"),
        }
    }

    #[test]
    fn proof_commands_from_markdown_collects_commands_from_fenced_block() {
        let text = r#"
## Proof Expectations

```bash
# Proof 1
cargo build -p myosu-play
cargo test -p myosu-play training::tests::hand_completes_fold
```
"#;

        let commands = proof_commands_from_markdown(text);

        assert_eq!(
            commands,
            vec![
                "cargo build -p myosu-play".to_string(),
                "cargo test -p myosu-play training::tests::hand_completes_fold".to_string(),
            ]
        );
    }

    #[test]
    fn inline_proof_command_extracts_single_line_proof_gate() {
        let command = inline_proof_command(
            "**Proof gate**: `cargo test -p myosu-sdk scaffold::tests::generates_compilable_crate`",
        )
        .expect("command");

        assert_eq!(
            command,
            "cargo test -p myosu-sdk scaffold::tests::generates_compilable_crate"
        );
    }

    #[test]
    fn looks_like_shell_command_ignores_non_commands() {
        assert!(looks_like_shell_command("cargo test -p myosu-sdk"));
        assert!(looks_like_shell_command("git diff crates/myosu-games/src/"));
        assert!(!looks_like_shell_command(
            "The lane is ready for implementation."
        ));
    }

    #[test]
    fn manual_notes_from_markdown_keeps_manual_proof_lines() {
        let text = "# (manual; run myosu-play --train and play one hand to showdown)";
        let notes = manual_notes_from_markdown(text);
        assert_eq!(
            notes,
            vec!["(manual; run myosu-play --train and play one hand to showdown)".to_string()]
        );
    }

    #[test]
    fn slice_notes_from_markdown_keeps_ordering_constraints() {
        let text = r#"
1. Slice 2 must succeed before Slice 3 can run
2. Slice 4 is independent
3. Spectator relay is Phase 0 only
"#;
        let notes = slice_notes_from_markdown(text);
        assert_eq!(
            notes,
            vec![
                "1. Slice 2 must succeed before Slice 3 can run".to_string(),
                "2. Slice 4 is independent".to_string(),
                "3. Spectator relay is Phase 0 only".to_string(),
            ]
        );
    }

    #[test]
    fn health_notes_from_markdown_keeps_health_surface_requirements() {
        let text = r#"
**Health check**: `GET /health` must include `training_active: bool`.
The /health endpoint is the lane's primary health surface.
"#;
        let notes = health_notes_from_markdown(text);
        assert_eq!(notes.len(), 2);
        assert!(notes[0].contains("GET /health"));
        assert!(notes[1].contains("/health endpoint"));
    }

    #[test]
    fn observability_notes_from_markdown_keeps_structured_log_signals() {
        let text = r#"
### Observability surfaces for operator dashboards

The validator binary should emit structured log lines.

{"level":"info","service":"myosu-validator","event":"epoch_complete"}
"#;
        let notes = observability_notes_from_markdown(text);
        assert_eq!(notes.len(), 3);
        assert!(notes[0].contains("Observability surfaces"));
        assert!(notes[1].contains("structured log lines"));
        assert!(notes[2].contains("\"event\":\"epoch_complete\""));
    }

    #[test]
    fn health_commands_from_markdown_extracts_curl_command() {
        let text = "| Axon reachability | `curl http://{ip}:{port}/health` | HTTP 200 |";
        let commands = health_commands_from_markdown(text);
        assert_eq!(commands, vec!["curl http://{ip}:{port}/health".to_string()]);
    }

    #[test]
    fn trim_list_prefix_removes_leading_numeric_marker() {
        assert_eq!(
            trim_list_prefix("1. Slice 2 must succeed before Slice 3 can run"),
            "Slice 2 must succeed before Slice 3 can run"
        );
        assert_eq!(
            trim_list_prefix("3) Spectator relay is Phase 0 only"),
            "Spectator relay is Phase 0 only"
        );
    }

    #[test]
    fn execution_guidance_from_slice_notes_categorizes_notes() {
        let notes = vec![
            "1. Slice 2 must succeed before Slice 3 can run".to_string(),
            "2. Slice 4 is independent".to_string(),
            "The implementation lane can begin with Slice 1 immediately.".to_string(),
            "Spectator relay is Phase 0 only".to_string(),
        ];

        let guidance = execution_guidance_from_slice_notes(&notes);

        assert_eq!(
            guidance,
            vec![
                "Order: Slice 2 must succeed before Slice 3 can run".to_string(),
                "Parallel: Slice 4 is independent".to_string(),
                "Start: The implementation lane can begin with Slice 1 immediately.".to_string(),
                "Scope: Spectator relay is Phase 0 only".to_string(),
            ]
        );
    }

    #[test]
    fn first_slice_from_markdown_extracts_table_row() {
        let text =
            "| 1 | `myosu-play` binary skeleton | `tui:shell` | `cargo build -p myosu-play` |";
        let slice = first_slice_from_markdown(text).expect("slice");
        assert_eq!(slice, "Slice 1: myosu-play binary skeleton");
    }

    #[test]
    fn first_slice_from_markdown_extracts_begin_with_slice_line() {
        let text = "The specification is stable. The implementation lane can begin with Slice 1 (SDK crate skeleton) immediately.";
        let slice = first_slice_from_markdown(text).expect("slice");
        assert_eq!(slice, "Slice 1: SDK crate skeleton");
    }

    #[test]
    fn first_slice_from_markdown_prefers_specific_header() {
        let text = "### Slice 1: `myosu-play` Binary Skeleton + Shell Wiring";
        let slice = first_slice_from_markdown(text).expect("slice");
        assert_eq!(slice, "Slice 1: myosu-play Binary Skeleton + Shell Wiring");
    }

    #[test]
    fn first_proof_gate_from_markdown_extracts_inline_gate() {
        let text =
            "**Proof gate**: `cargo build -p myosu-play` exits 0; `myosu-play --train` renders.";
        let gate = first_proof_gate_from_markdown(text).expect("gate");
        assert_eq!(gate, "cargo build -p myosu-play");
    }

    #[test]
    fn first_health_gate_from_markdown_extracts_health_command() {
        let text = "**Health check**: `GET /health` must include `training_active: bool`.";
        let gate = first_health_gate_from_markdown(text).expect("gate");
        assert_eq!(gate, "GET /health");
    }

    #[test]
    fn smoke_commands_from_markdown_extracts_inline_smoke_gate() {
        let text = "**Smoke gate**: `myosu-play --train --rounds 1` renders one hand.";
        let commands = smoke_commands_from_markdown(text);
        assert_eq!(commands, vec!["myosu-play --train --rounds 1".to_string()]);
        let gate = first_smoke_gate_from_markdown(text).expect("smoke gate");
        assert_eq!(gate, "myosu-play --train --rounds 1");
    }

    #[test]
    fn implementation_verify_command_appends_smoke_for_user_visible_lane() {
        let lane = BlueprintLane {
            id: "play-implement".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Interface,
            title: "Play Implement".to_string(),
            family: "implement".to_string(),
            workflow_family: Some("implement".to_string()),
            slug: Some("play".to_string()),
            template: WorkflowTemplate::Implementation,
            goal: "implement".to_string(),
            managed_milestone: "merge_ready".to_string(),
            dependencies: Vec::new(),
            produces: Vec::new(),
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        };
        let evidence = ImplementationEvidence {
            proof_commands: vec!["cargo test -p myosu-play".to_string()],
            smoke_commands: vec!["myosu-play --train --rounds 1".to_string()],
            ..ImplementationEvidence::default()
        };

        let command = implementation_verify_command(&lane, &evidence);

        assert!(command.contains("cargo test -p myosu-play"));
        assert!(command.contains("myosu-play --train --rounds 1"));
    }

    #[test]
    fn implementation_verify_command_ignores_smoke_for_platform_lane() {
        let lane = BlueprintLane {
            id: "sdk-implement".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Platform,
            title: "SDK Implement".to_string(),
            family: "implement".to_string(),
            workflow_family: Some("implement".to_string()),
            slug: Some("sdk".to_string()),
            template: WorkflowTemplate::Implementation,
            goal: "implement".to_string(),
            managed_milestone: "merge_ready".to_string(),
            dependencies: Vec::new(),
            produces: Vec::new(),
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        };
        let evidence = ImplementationEvidence {
            proof_commands: vec!["cargo test -p myosu-sdk".to_string()],
            smoke_commands: vec!["myosu-play --train --rounds 1".to_string()],
            ..ImplementationEvidence::default()
        };

        let command = implementation_verify_command(&lane, &evidence);

        assert!(command.contains("cargo test -p myosu-sdk"));
        assert!(!command.contains("myosu-play --train --rounds 1"));
    }

    #[test]
    fn implementation_verify_command_normalizes_bootstrap_pair_and_control_flow() {
        let lane = BlueprintLane {
            id: "command-center-client-implement".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Interface,
            title: "Command Center Client Implementation Lane".to_string(),
            family: "implement".to_string(),
            workflow_family: Some("implement".to_string()),
            slug: Some("command-center-client".to_string()),
            template: WorkflowTemplate::Implementation,
            goal: "implement".to_string(),
            managed_milestone: "merge_ready".to_string(),
            dependencies: Vec::new(),
            produces: Vec::new(),
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        };
        let evidence = ImplementationEvidence {
            proof_commands: vec![
                "./scripts/bootstrap_home_miner.sh".to_string(),
                "./scripts/pair_gateway_client.sh --client alice-phone".to_string(),
                "./scripts/read_miner_status.sh --client alice-phone".to_string(),
                "./scripts/set_mining_mode.sh --client alice-phone --mode balanced".to_string(),
            ],
            ..ImplementationEvidence::default()
        };

        let command = implementation_verify_command(&lane, &evidence);

        assert!(command.contains("DEVICE_NAME=bootstrap-phone ./scripts/bootstrap_home_miner.sh"));
        assert!(command.contains(
            "./scripts/pair_gateway_client.sh --client alice-phone --capabilities observe,control"
        ));
        assert!(
            command.contains("./scripts/set_mining_mode.sh --client alice-phone --mode balanced")
        );
    }

    #[test]
    fn implementation_goal_lists_integration_artifact() {
        let unit = BlueprintUnit {
            id: "hermes-adapter".to_string(),
            title: "Hermes Adapter".to_string(),
            output_root: PathBuf::from("outputs/hermes-adapter"),
            artifacts: vec![
                BlueprintArtifact {
                    id: "implementation".to_string(),
                    path: PathBuf::from("implementation.md"),
                },
                BlueprintArtifact {
                    id: "verification".to_string(),
                    path: PathBuf::from("verification.md"),
                },
                BlueprintArtifact {
                    id: "quality".to_string(),
                    path: PathBuf::from("quality.md"),
                },
                BlueprintArtifact {
                    id: "promotion".to_string(),
                    path: PathBuf::from("promotion.md"),
                },
                BlueprintArtifact {
                    id: "integration".to_string(),
                    path: PathBuf::from("integration.md"),
                },
            ],
            milestones: Vec::new(),
            lanes: Vec::new(),
        };
        let lane = BlueprintLane {
            id: "hermes-adapter".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Artifact,
            title: "Hermes Adapter Lane".to_string(),
            family: "implement".to_string(),
            workflow_family: Some("implement".to_string()),
            slug: Some("hermes-adapter".to_string()),
            template: WorkflowTemplate::Implementation,
            goal: String::new(),
            managed_milestone: "merge_ready".to_string(),
            dependencies: Vec::new(),
            produces: vec![
                "implementation".to_string(),
                "verification".to_string(),
                "quality".to_string(),
                "promotion".to_string(),
                "integration".to_string(),
            ],
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        };

        let goal = implementation_goal(
            &unit,
            &lane,
            Path::new("agent-adapter.md"),
            Path::new("review.md"),
            Path::new("outputs/hermes-adapter/quality.md"),
            Path::new("outputs/hermes-adapter/promotion.md"),
            Path::new("outputs/hermes-adapter/integration.md"),
        );

        assert!(goal.contains("outputs/hermes-adapter/integration.md"));
    }

    #[test]
    fn implementation_quality_command_does_not_treat_future_slice_wording_as_artifact_mismatch() {
        let unit = BlueprintUnit {
            id: "home-miner-service".to_string(),
            title: "Home Miner Service".to_string(),
            output_root: PathBuf::from("outputs/home-miner-service"),
            artifacts: vec![
                BlueprintArtifact {
                    id: "implementation".to_string(),
                    path: PathBuf::from("implementation.md"),
                },
                BlueprintArtifact {
                    id: "verification".to_string(),
                    path: PathBuf::from("verification.md"),
                },
                BlueprintArtifact {
                    id: "quality".to_string(),
                    path: PathBuf::from("quality.md"),
                },
            ],
            milestones: Vec::new(),
            lanes: Vec::new(),
        };
        let lane = BlueprintLane {
            id: "home-miner-service".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Service,
            title: "Home Miner Service Lane".to_string(),
            family: "implement".to_string(),
            workflow_family: Some("implement".to_string()),
            slug: Some("home-miner-service".to_string()),
            template: WorkflowTemplate::Implementation,
            goal: String::new(),
            managed_milestone: "merge_ready".to_string(),
            dependencies: Vec::new(),
            produces: vec![
                "implementation".to_string(),
                "verification".to_string(),
                "quality".to_string(),
            ],
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        };
        let blueprint = ProgramBlueprint {
            version: 1,
            program: BlueprintProgram {
                id: "zend-home-miner-service-home-miner-service-implementation".to_string(),
                max_parallel: 1,
                state_path: None,
                run_dir: None,
            },
            inputs: BlueprintInputs::default(),
            package: BlueprintPackage::default(),
            units: vec![unit],
        };

        let command = implementation_quality_command(&blueprint, "home-miner-service", &lane);

        assert!(!command.contains("future slice"));
    }

    #[test]
    fn service_review_prompt_includes_health_sections() {
        let lane = BlueprintLane {
            id: "service-implement".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Service,
            title: "Miner Service Implementation Lane".to_string(),
            family: "implement".to_string(),
            workflow_family: Some("implement".to_string()),
            slug: Some("miner-service".to_string()),
            template: WorkflowTemplate::Implementation,
            goal: "Implement the next approved `miner:service` slice.".to_string(),
            managed_milestone: "verified".to_string(),
            dependencies: Vec::new(),
            produces: vec!["implementation".to_string(), "verification".to_string()],
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: Some(
                "Implement now:\n- Slice 1: myosu-miner CLI skeleton\n\nFirst proof gate:\n- cargo test -p myosu-miner -- --test-threads=1\n\nFirst health gate:\n- GET /health\n\nService/health surfaces to preserve:\n- The /health endpoint is the lane's primary health surface.".to_string(),
            ),
            verify_command: None,
            health_command: None,
        };

        let review = render_prompt("review", &lane);

        assert!(review.contains("First health gate"));
        assert!(review.contains("GET /health"));
        assert!(review.contains("Health surfaces to preserve"));
    }

    #[test]
    fn service_review_prompt_includes_observability_sections() {
        let lane = BlueprintLane {
            id: "service-implement".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Service,
            title: "Miner Service Implementation Lane".to_string(),
            family: "implement".to_string(),
            workflow_family: Some("implement".to_string()),
            slug: Some("miner-service".to_string()),
            template: WorkflowTemplate::Implementation,
            goal: "Implement the next approved `miner:service` slice.".to_string(),
            managed_milestone: "verified".to_string(),
            dependencies: Vec::new(),
            produces: vec!["implementation".to_string(), "verification".to_string()],
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: Some(
                "Implement now:\n- Slice 1: myosu-miner CLI skeleton\n\nFirst health gate:\n- GET /health\n\nService/health surfaces to preserve:\n- The /health endpoint is the lane's primary health surface.\n\nObservability surfaces to preserve:\n- structured log lines\n- {\"level\":\"info\",\"service\":\"myosu-miner\",\"event\":\"epoch_complete\"}".to_string(),
            ),
            verify_command: None,
            health_command: None,
        };

        let review = render_prompt("review", &lane);

        assert!(review.contains("Observability surfaces to preserve"));
        assert!(review.contains("structured log lines"));
        assert!(review.contains("epoch_complete"));
    }

    #[test]
    fn implementation_challenge_prompt_marks_non_final_gate() {
        let lane = BlueprintLane {
            id: "service-implement".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Service,
            title: "Miner Service Implementation Lane".to_string(),
            family: "implement".to_string(),
            workflow_family: Some("implement".to_string()),
            slug: Some("miner-service".to_string()),
            template: WorkflowTemplate::Implementation,
            goal: "Implement the next approved `miner:service` slice.".to_string(),
            managed_milestone: "verified".to_string(),
            dependencies: Vec::new(),
            produces: vec!["implementation".to_string(), "verification".to_string()],
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: Some("Implement now:\n- Slice 1: miner CLI skeleton".to_string()),
            verify_command: None,
            health_command: None,
        };

        let challenge = render_prompt("challenge", &lane);

        assert!(challenge.contains("cheap adversarial review"));
        assert!(challenge.contains("Do not bless the slice as merge-ready"));
        assert!(challenge.contains("Challenge checklist"));
        assert!(challenge.contains("Do not write `promotion.md` here"));
    }

    #[test]
    fn backticked_segments_extracts_multiple_paths() {
        let line = "**Files**: `crates/myosu-sdk/Cargo.toml`, `crates/myosu-sdk/src/lib.rs`";
        let segments = backticked_segments(line);
        assert_eq!(
            segments,
            vec![
                "crates/myosu-sdk/Cargo.toml".to_string(),
                "crates/myosu-sdk/src/lib.rs".to_string(),
            ]
        );
    }

    #[test]
    fn first_code_surface_from_markdown_extracts_slice_one_files() {
        let text = r#"
### Slice 1 — Create `myosu-sdk` Crate Skeleton (AC-SDK-01)

**Files**: `crates/myosu-sdk/Cargo.toml`, `crates/myosu-sdk/src/lib.rs`
"#;
        let surface = first_code_surface_from_markdown(text).expect("surface");
        assert_eq!(surface, "crates/myosu-sdk/Cargo.toml");
    }

    #[test]
    fn setup_notes_from_markdown_extracts_slice_one_setup_steps() {
        let text = r#"
### Slice 1 — Create `myosu-sdk` Crate Skeleton (AC-SDK-01)

**Files**: `crates/myosu-sdk/Cargo.toml`, `crates/myosu-sdk/src/lib.rs`

Add `crates/myosu-sdk/` to workspace members. `Cargo.toml`:
- Dependency on `myosu-games`
- `crate-type = ["lib"]`
- `features = { default = [], tui = ["myosu-tui"] }`

`lib.rs` re-exports all types from `myosu-games`.

**Proof**: `cargo build -p myosu-sdk` exits 0.
"#;
        let notes = setup_notes_from_markdown(text);
        assert_eq!(
            notes,
            vec![
                "Add `crates/myosu-sdk/` to workspace members. `Cargo.toml`:".to_string(),
                "Dependency on `myosu-games`".to_string(),
                "`crate-type = [\"lib\"]`".to_string(),
                "`features = { default = [], tui = [\"myosu-tui\"] }`".to_string(),
                "lib.rs re-exports all types from myosu-games.".to_string(),
            ]
        );
    }

    #[test]
    fn first_slice_work_from_markdown_extracts_what_line() {
        let text = r#"
### Slice 1: `myosu-play` Binary Skeleton + Shell Wiring
**Files**: `crates/myosu-play/`
**What**: Bare `main.rs` with `--train` flag; creates `NlheRenderer`; wires into `Shell`.
"#;
        let work = first_slice_work_from_markdown(text).expect("work");
        assert_eq!(
            work,
            "Bare `main.rs` with `--train` flag; creates `NlheRenderer`; wires into `Shell`."
        );
    }

    #[test]
    fn implementation_render_prompt_uses_current_slice_contract() {
        let lane = BlueprintLane {
            id: "tui-implement".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Interface,
            title: "Gameplay TUI Implementation Lane".to_string(),
            family: "implement".to_string(),
            workflow_family: Some("implement".to_string()),
            slug: Some("play-tui".to_string()),
            template: WorkflowTemplate::Implementation,
            goal: "Implement the next approved `play:tui` slice.".to_string(),
            managed_milestone: "verified".to_string(),
            dependencies: Vec::new(),
            produces: vec!["implementation".to_string(), "verification".to_string()],
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: Some(
                "Implement now:\n- Slice 1: myosu-play Binary Skeleton + Shell Wiring".to_string(),
            ),
            verify_command: None,
            health_command: None,
        };

        let review = render_prompt("review", &lane);
        let polish = render_prompt("polish", &lane);
        let plan = render_prompt("plan", &lane);

        assert!(plan.contains("Implementation artifact must cover"));
        assert!(plan.contains("Verification artifact must cover"));
        assert!(plan.contains("do not hand-author `quality.md`"));
        assert!(review.contains("Review only the current slice"));
        assert!(review.contains("treat `quality.md` as machine-generated truth"));
        assert!(review.contains("Write `promotion.md` in this exact machine-readable form"));
        assert!(review.contains("Review stage ownership"));
        assert!(review.contains("Current slice"));
        assert!(polish.contains("# Gameplay TUI Implementation Lane — Fixup"));
        assert!(polish.contains("do not hand-author `quality.md`"));
        assert!(polish.contains("prefer staying within the named slice and touched surfaces"));
    }

    #[test]
    fn implementation_review_prompt_adds_security_guidance_for_sensitive_slices() {
        let lane = BlueprintLane {
            id: "wallet-implement".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Platform,
            title: "Wallet Service Implementation Lane".to_string(),
            family: "implement".to_string(),
            workflow_family: Some("implement".to_string()),
            slug: Some("wallet-service".to_string()),
            template: WorkflowTemplate::Implementation,
            goal: "Implement the next approved wallet RPC slice.".to_string(),
            managed_milestone: "merge_ready".to_string(),
            dependencies: Vec::new(),
            produces: vec!["implementation".to_string(), "verification".to_string()],
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: Some(
                "Implement now:\n- Wallet RPC session validation\n\nTouch first:\n- `crates/rxmr-wallet/src/rpc.rs`\n".to_string(),
            ),
            verify_command: None,
            health_command: None,
        };

        let review = render_prompt("review", &lane);

        assert!(review.contains("Nemesis-style security review"));
        assert!(review.contains("trust boundaries"));
        assert!(review.contains("state transitions"));
    }

    #[test]
    fn bootstrap_review_prompt_adds_security_guidance_for_sensitive_lanes() {
        let lane = BlueprintLane {
            id: "monero-infrastructure".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Platform,
            title: "Monero Infrastructure".to_string(),
            family: "bootstrap".to_string(),
            workflow_family: Some("bootstrap".to_string()),
            slug: Some("monero-infrastructure".to_string()),
            template: WorkflowTemplate::Bootstrap,
            goal: "Bootstrap wallet RPC and node lifecycle safely.".to_string(),
            managed_milestone: "reviewed".to_string(),
            dependencies: Vec::new(),
            produces: vec!["foundation_plan".to_string(), "review".to_string()],
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: Some("Review wallet/node trust boundaries.".to_string()),
            verify_command: None,
            health_command: None,
        };

        let review = render_prompt("review", &lane);

        assert!(review.contains("Nemesis-style security review"));
        assert!(review.contains("first-principles challenge"));
        assert!(review.contains("coupled-state review"));
    }

    #[test]
    fn prompt_context_block_extracts_named_section() {
        let context = "Implement now:\n- Slice 1: myosu-play Binary Skeleton + Shell Wiring\n\nTouch first:\n- `crates/myosu-play/`\n";
        let implement_now = prompt_context_block(context, "Implement now:");
        let touch_first = prompt_context_block(context, "Touch first:");

        assert_eq!(
            implement_now,
            vec!["- Slice 1: myosu-play Binary Skeleton + Shell Wiring".to_string()]
        );
        assert_eq!(touch_first, vec!["- `crates/myosu-play/`".to_string()]);
    }

    #[test]
    fn service_implementation_workflow_includes_health_gate() {
        let lane = BlueprintLane {
            id: "service-implement".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Service,
            title: "Miner Service Implementation Lane".to_string(),
            family: "implement".to_string(),
            workflow_family: Some("implement".to_string()),
            slug: Some("miner-service".to_string()),
            template: WorkflowTemplate::Implementation,
            goal: "Implement the next approved `miner:service` slice.".to_string(),
            managed_milestone: "verified".to_string(),
            dependencies: Vec::new(),
            produces: vec!["implementation".to_string(), "verification".to_string()],
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: Some("set -e\ncurl http://{ip}:{port}/health".to_string()),
        };

        let graph = render_workflow_graph(
            &lane,
            "cargo test -p myosu-miner -- --test-threads=1",
            "set -e\ncurl http://{ip}:{port}/health",
            "test -f outputs/miner/service/implementation.md && test -f outputs/miner/service/verification.md && test -f outputs/miner/service/quality.md && grep -Eq '^merge_ready: yes$' outputs/miner/service/promotion.md",
            "test -f outputs/miner/service/quality.md",
        );

        assert!(graph.contains("label=\"Health\""));
        assert!(graph.contains("label=\"Quality Gate\""));
        assert!(graph.contains("label=\"Challenge\""));
        assert!(graph.contains("label=\"Review\""));
        assert!(graph.contains(
            "#challenge   { backend: cli; model: MiniMax-M2.7-highspeed; provider: minimax; }"
        ));
        assert!(graph.contains(
            "#review      { backend: cli; model: MiniMax-M2.7-highspeed; provider: minimax; }"
        ));
        assert!(graph.contains("verify -> health"));
        assert!(graph.contains("health -> quality"));
        assert!(graph.contains("quality -> challenge [condition=\"outcome=success\"]"));
        assert!(graph.contains("challenge -> review [condition=\"outcome=success\"]"));
        assert!(graph.contains("review -> audit [condition=\"outcome=success\"]"));
        assert!(graph.contains("challenge -> fixup"));
        assert!(graph.contains("review -> fixup"));
        assert!(graph.contains("audit -> fixup"));
        assert!(!graph.contains("label=\"Settle\""));
    }

    #[test]
    fn bootstrap_workflow_retries_verify_via_polish() {
        let lane = BlueprintLane {
            id: "private-control-plane".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Platform,
            title: "Private Control Plane".to_string(),
            family: "bootstrap".to_string(),
            workflow_family: Some("bootstrap".to_string()),
            slug: Some("private-control-plane".to_string()),
            template: WorkflowTemplate::Bootstrap,
            goal: "Bootstrap the private control plane.".to_string(),
            managed_milestone: "reviewed".to_string(),
            dependencies: Vec::new(),
            produces: Vec::new(),
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        };

        let graph = render_workflow_graph(
            &lane,
            "test -f spec.md && test -f review.md",
            "true",
            "true",
            "true",
        );

        assert!(graph.contains("retry_target=\"polish\""));
        assert!(graph.contains("verify -> exit [condition=\"outcome=success\"]"));
        assert!(graph.contains("verify -> polish"));
    }

    #[test]
    fn service_bootstrap_workflow_retries_verify_outputs_via_polish() {
        let lane = BlueprintLane {
            id: "home-miner-service".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Service,
            title: "Home Miner Service".to_string(),
            family: "service_bootstrap".to_string(),
            workflow_family: Some("service_bootstrap".to_string()),
            slug: Some("home-miner-service".to_string()),
            template: WorkflowTemplate::ServiceBootstrap,
            goal: "Bootstrap the home miner service.".to_string(),
            managed_milestone: "reviewed".to_string(),
            dependencies: Vec::new(),
            produces: Vec::new(),
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        };

        let graph = render_workflow_graph(
            &lane,
            "test -f inventory.md && test -f review.md",
            "true",
            "true",
            "true",
        );

        assert!(graph.contains("verify_outputs [label=\"Verify Outputs\""));
        assert!(graph.contains("retry_target=\"polish\""));
        assert!(graph.contains("verify_outputs -> exit [condition=\"outcome=success\"]"));
        assert!(graph.contains("verify_outputs -> polish"));
    }

    #[test]
    fn implementation_run_config_enables_direct_integration() {
        let lane = BlueprintLane {
            id: "tui-implement".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Interface,
            title: "Gameplay TUI Implementation Lane".to_string(),
            family: "implement".to_string(),
            workflow_family: Some("implement".to_string()),
            slug: Some("play-tui".to_string()),
            template: WorkflowTemplate::Implementation,
            goal: "Implement the next approved `play:tui` slice.".to_string(),
            managed_milestone: "merge_ready".to_string(),
            dependencies: Vec::new(),
            produces: vec![
                "implementation".to_string(),
                "verification".to_string(),
                "quality".to_string(),
                "promotion".to_string(),
            ],
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let repo = Repository::init(temp.path()).expect("git repo");
        repo.remote("origin", "https://example.com/repo.git")
            .expect("origin remote");
        let run_config = render_run_config(
            &lane,
            Some(Path::new("outputs/play/tui/integration.md")),
            temp.path(),
        );
        assert!(run_config.contains("worktree_mode = \"always\""));
        assert!(run_config.contains("[llm]"));
        assert!(run_config.contains("provider = \"minimax\""));
        assert!(run_config.contains("model = \"MiniMax-M2.7-highspeed\""));
        assert!(run_config.contains("[llm.fallbacks]"));
        assert!(run_config.contains("[sandbox.env]"));
        assert!(run_config.contains("MINIMAX_API_KEY = \"${env.MINIMAX_API_KEY}\""));
        assert!(!run_config.contains("OPENAI_API_KEY = \"${env.OPENAI_API_KEY}\""));
        assert!(run_config.contains("[integration]"));
        assert!(run_config.contains("enabled = true"));
        assert!(run_config.contains("target_branch = \"origin/HEAD\""));
        assert!(run_config.contains("artifact_path = \"../../../outputs/play/tui/integration.md\""));
    }

    #[test]
    fn bootstrap_run_config_uses_minimax_defaults_and_direct_integration() {
        let lane = BlueprintLane {
            id: "private-control-plane".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Platform,
            title: "Private Control Plane".to_string(),
            family: "bootstrap".to_string(),
            workflow_family: Some("bootstrap".to_string()),
            slug: Some("private-control-plane".to_string()),
            template: WorkflowTemplate::Bootstrap,
            goal: "Bootstrap the private control plane.".to_string(),
            managed_milestone: "reviewed".to_string(),
            dependencies: Vec::new(),
            produces: Vec::new(),
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let repo = Repository::init(temp.path()).expect("git repo");
        repo.remote("origin", "https://example.com/repo.git")
            .expect("origin remote");
        let run_config = render_run_config(&lane, None, temp.path());

        assert!(run_config.contains("[llm]"));
        assert!(run_config.contains("provider = \"minimax\""));
        assert!(run_config.contains("model = \"MiniMax-M2.7-highspeed\""));
        assert!(run_config.contains("[llm.fallbacks]"));
        assert!(run_config.contains("[sandbox.env]"));
        assert!(run_config.contains("MINIMAX_API_KEY = \"${env.MINIMAX_API_KEY}\""));
        assert!(!run_config.contains("OPENAI_API_KEY = \"${env.OPENAI_API_KEY}\""));
        assert!(run_config.contains("worktree_mode = \"clean\""));
        assert!(run_config.contains("[integration]"));
        assert!(run_config.contains("enabled = true"));
        assert!(run_config.contains("target_branch = \"origin/HEAD\""));
        assert!(!run_config.contains("artifact_path = "));
    }

    #[test]
    fn service_bootstrap_run_config_enables_direct_integration() {
        let lane = BlueprintLane {
            id: "home-miner-service".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Service,
            title: "Home Miner Service".to_string(),
            family: "service_bootstrap".to_string(),
            workflow_family: Some("service_bootstrap".to_string()),
            slug: Some("home-miner-service".to_string()),
            template: WorkflowTemplate::ServiceBootstrap,
            goal: "Bootstrap the home miner service.".to_string(),
            managed_milestone: "reviewed".to_string(),
            dependencies: Vec::new(),
            produces: Vec::new(),
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        };

        let temp = tempfile::tempdir().expect("tempdir");
        let repo = Repository::init(temp.path()).expect("git repo");
        repo.remote("origin", "https://example.com/repo.git")
            .expect("origin remote");
        let run_config = render_run_config(&lane, None, temp.path());

        assert!(run_config.contains("[llm]"));
        assert!(run_config.contains("provider = \"minimax\""));
        assert!(run_config.contains("model = \"MiniMax-M2.7-highspeed\""));
        assert!(run_config.contains("[llm.fallbacks]"));
        assert!(run_config.contains("MINIMAX_API_KEY = \"${env.MINIMAX_API_KEY}\""));
        assert!(run_config.contains("[integration]"));
        assert!(run_config.contains("enabled = true"));
        assert!(run_config.contains("target_branch = \"origin/HEAD\""));
        assert!(!run_config.contains("artifact_path = "));
    }

    #[test]
    fn bootstrap_run_config_omits_direct_integration_for_non_git_repo() {
        let lane = BlueprintLane {
            id: "poker".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Interface,
            title: "Poker".to_string(),
            family: "bootstrap".to_string(),
            workflow_family: Some("bootstrap".to_string()),
            slug: Some("poker".to_string()),
            template: WorkflowTemplate::Bootstrap,
            goal: "Bootstrap poker.".to_string(),
            managed_milestone: "reviewed".to_string(),
            dependencies: Vec::new(),
            produces: Vec::new(),
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        };
        let temp = tempfile::tempdir().expect("tempdir");

        let run_config = render_run_config(&lane, None, temp.path());

        assert!(!run_config.contains("[integration]"));
        assert!(run_config.contains("worktree_mode = \"clean\""));
    }

    #[test]
    fn bootstrap_run_config_targets_local_branch_when_origin_is_missing() {
        let lane = BlueprintLane {
            id: "poker".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Interface,
            title: "Poker".to_string(),
            family: "bootstrap".to_string(),
            workflow_family: Some("bootstrap".to_string()),
            slug: Some("poker".to_string()),
            template: WorkflowTemplate::Bootstrap,
            goal: "Bootstrap poker.".to_string(),
            managed_milestone: "reviewed".to_string(),
            dependencies: Vec::new(),
            produces: Vec::new(),
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        };
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = Repository::init(temp.path()).expect("git repo");
        repo.reference_symbolic("HEAD", "refs/heads/main", true, "main branch")
            .expect("set head");
        std::fs::write(temp.path().join("README.md"), "# Demo\n").expect("readme");
        let mut index = repo.index().expect("index");
        index
            .add_path(Path::new("README.md"))
            .expect("stage readme");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("tree");
        let tree = repo.find_tree(tree_id).expect("tree lookup");
        let sig = git2::Signature::now("Test", "test@example.com").expect("signature");
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("commit");

        let run_config = render_run_config(&lane, None, temp.path());

        assert!(run_config.contains("[integration]"));
        assert!(run_config.contains("target_branch = \"main\""));
    }
}

fn input_texts(blueprint: &ProgramBlueprint, target_repo: &Path) -> Vec<(String, String)> {
    let mut texts = Vec::new();
    for doctrine in &blueprint.inputs.doctrine_files {
        let absolute = target_repo.join(doctrine);
        if let Ok(contents) = std::fs::read_to_string(&absolute) {
            texts.push((
                format!("doctrine `{}`", doctrine.display()),
                contents.to_lowercase(),
            ));
        }
    }
    for evidence in &blueprint.inputs.evidence_paths {
        let absolute = target_repo.join(evidence);
        if let Ok(contents) = std::fs::read_to_string(&absolute) {
            texts.push((
                format!("evidence `{}`", evidence.display()),
                contents.to_lowercase(),
            ));
        }
    }
    texts
}

fn lane_terms(lane: &BlueprintLane) -> Vec<String> {
    let mut terms = vec![lane.id.to_lowercase(), lane.title.to_lowercase()];
    terms.extend(
        lane.title
            .split_whitespace()
            .map(|part| {
                part.trim_matches(|ch: char| !ch.is_alphanumeric())
                    .to_lowercase()
            })
            .filter(|part| !part.is_empty() && part.len() > 3),
    );
    terms.sort();
    terms.dedup();
    terms
}

fn report_set_drift<'a, I>(label: &str, current: I, desired: I, findings: &mut Vec<String>)
where
    I: Iterator<Item = &'a String> + Clone,
{
    let current = current.cloned().collect::<BTreeSet<_>>();
    let desired = desired.cloned().collect::<BTreeSet<_>>();
    for removed in current.difference(&desired) {
        findings.push(format!(
            "{label} `{removed}` exists in current package but not in blueprint"
        ));
    }
    for added in desired.difference(&current) {
        findings.push(format!(
            "{label} `{added}` exists in blueprint but not in current package"
        ));
    }
}

fn graph_name(lane: &BlueprintLane) -> String {
    lane.slug()
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let upper = first.to_uppercase().collect::<String>();
                    format!("{upper}{rest}", rest = chars.as_str())
                }
                None => String::new(),
            }
        })
        .collect()
}

fn write_file(
    path: &Path,
    contents: &str,
    written_files: &mut Vec<PathBuf>,
) -> Result<(), RenderError> {
    ensure_parent(path)?;
    fabro_workflows::write_text_atomic(path, contents, "rendered file").map_err(|source| {
        RenderError::Write {
            path: path.to_path_buf(),
            source: std::io::Error::other(source.to_string()),
        }
    })?;
    written_files.push(path.to_path_buf());
    Ok(())
}

fn ensure_parent(path: &Path) -> Result<(), RenderError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|source| RenderError::CreateDir {
        path: parent.to_path_buf(),
        source,
    })
}

fn escape_graph_attr(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

struct PackageLayout<'a> {
    blueprint: &'a ProgramBlueprint,
    target_repo: &'a Path,
}

impl<'a> PackageLayout<'a> {
    fn new(blueprint: &'a ProgramBlueprint, target_repo: &'a Path) -> Self {
        Self {
            blueprint,
            target_repo,
        }
    }

    fn fabro_root(&self) -> PathBuf {
        self.target_repo.join(&self.blueprint.package.fabro_root)
    }

    fn manifest_path(&self) -> PathBuf {
        self.fabro_root()
            .join("programs")
            .join(format!("{}.yaml", self.blueprint.program.id))
    }

    fn run_config_path(&self, lane: &BlueprintLane) -> PathBuf {
        self.fabro_root()
            .join("run-configs")
            .join(&lane.family)
            .join(format!("{}.toml", lane.slug()))
    }

    fn workflow_path(&self, lane: &BlueprintLane) -> PathBuf {
        self.fabro_root()
            .join("workflows")
            .join(lane.workflow_family())
            .join(format!("{}.fabro", lane.slug()))
    }

    fn prompt_dir(&self, lane: &BlueprintLane) -> PathBuf {
        self.fabro_root()
            .join("prompts")
            .join(lane.workflow_family())
            .join(lane.slug())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ManifestOut {
    version: u32,
    program: String,
    target_repo: String,
    state_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_dir: Option<String>,
    max_parallel: usize,
    units: Vec<ManifestUnitOut>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ManifestUnitOut {
    id: String,
    title: String,
    output_root: String,
    artifacts: Vec<ManifestArtifactOut>,
    milestones: Vec<raspberry_supervisor::manifest::MilestoneManifest>,
    lanes: Vec<ManifestLaneOut>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ManifestArtifactOut {
    id: String,
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ManifestLaneOut {
    id: String,
    kind: raspberry_supervisor::manifest::LaneKind,
    title: String,
    run_config: String,
    managed_milestone: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    depends_on: Vec<raspberry_supervisor::manifest::LaneDependency>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    produces: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof_state_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    program_manifest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    service_state_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    orchestration_state_path: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    checks: Vec<raspberry_supervisor::manifest::LaneCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_dir: Option<String>,
}

impl ManifestOut {
    fn from_blueprint(blueprint: &ProgramBlueprint) -> Self {
        let units = blueprint
            .units
            .iter()
            .map(|unit| ManifestUnitOut {
                id: unit.id.clone(),
                title: unit.title.clone(),
                output_root: repo_relative_string(&unit.output_root, 2),
                artifacts: unit
                    .artifacts
                    .iter()
                    .map(|artifact| ManifestArtifactOut {
                        id: artifact.id.clone(),
                        path: artifact.path.display().to_string(),
                    })
                    .collect(),
                milestones: unit.milestones.clone(),
                lanes: unit
                    .lanes
                    .iter()
                    .map(ManifestLaneOut::from_blueprint)
                    .collect(),
            })
            .collect();

        Self {
            version: blueprint.version,
            program: blueprint.program.id.clone(),
            target_repo: "../..".to_string(),
            state_path: repo_relative_string(
                &blueprint.program.state_path.clone().unwrap_or_else(|| {
                    PathBuf::from(format!(".raspberry/{}-state.json", blueprint.program.id))
                }),
                2,
            ),
            run_dir: blueprint
                .program
                .run_dir
                .as_ref()
                .map(|path| repo_relative_string(path, 2)),
            max_parallel: blueprint.program.max_parallel,
            units,
        }
    }
}

impl ManifestLaneOut {
    fn from_blueprint(lane: &BlueprintLane) -> Self {
        let run_config = lane
            .program_manifest
            .as_ref()
            .map(|path| repo_relative_string(path, 2))
            .unwrap_or_else(|| format!("../run-configs/{}/{}.toml", lane.family, lane.slug()));
        Self {
            id: lane.id.clone(),
            kind: lane.kind,
            title: lane.title.clone(),
            run_config,
            managed_milestone: lane.managed_milestone.clone(),
            depends_on: lane.dependencies.clone(),
            produces: lane.produces.clone(),
            proof_profile: lane.proof_profile.clone(),
            proof_state_path: lane
                .proof_state_path
                .as_ref()
                .map(|path| repo_relative_string(path, 2)),
            program_manifest: lane
                .program_manifest
                .as_ref()
                .map(|path| repo_relative_string(path, 2)),
            service_state_path: lane
                .service_state_path
                .as_ref()
                .map(|path| repo_relative_string(path, 2)),
            orchestration_state_path: lane
                .orchestration_state_path
                .as_ref()
                .map(|path| repo_relative_string(path, 2)),
            checks: lane.checks.iter().map(manifest_check).collect(),
            run_dir: lane
                .run_dir
                .as_ref()
                .map(|path| repo_relative_string(path, 2)),
        }
    }
}

fn manifest_check(check: &LaneCheck) -> LaneCheck {
    let probe = match &check.probe {
        LaneCheckProbe::FileExists { path } => LaneCheckProbe::FileExists {
            path: PathBuf::from(repo_relative_string(path, 2)),
        },
        LaneCheckProbe::JsonFieldEquals {
            path,
            field,
            equals,
        } => LaneCheckProbe::JsonFieldEquals {
            path: PathBuf::from(repo_relative_string(path, 2)),
            field: field.clone(),
            equals: equals.clone(),
        },
        LaneCheckProbe::CommandSucceeds { command } => LaneCheckProbe::CommandSucceeds {
            command: command.clone(),
        },
        LaneCheckProbe::CommandStdoutContains { command, contains } => {
            LaneCheckProbe::CommandStdoutContains {
                command: command.clone(),
                contains: contains.clone(),
            }
        }
    };

    LaneCheck {
        label: check.label.clone(),
        kind: check.kind,
        scope: check.scope,
        probe,
    }
}

fn repo_relative_string(path: &Path, levels_up: usize) -> String {
    let mut prefix = PathBuf::new();
    for _ in 0..levels_up {
        prefix.push("..");
    }
    prefix.join(path).display().to_string()
}
