use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use fabro_config::run::load_run_config;
use raspberry_supervisor::manifest::{
    LaneCheck, LaneCheckKind, LaneCheckProbe, LaneCheckScope, LaneDependency, LaneKind,
    MilestoneManifest, ProgramManifest,
};
use serde::{Deserialize, Serialize};

use crate::error::BlueprintError;
use crate::render::ImportRequest;

const SUPPORTED_BLUEPRINT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProgramBlueprint {
    #[serde(default = "default_version")]
    pub version: u32,
    pub program: BlueprintProgram,
    #[serde(default)]
    pub inputs: BlueprintInputs,
    #[serde(default)]
    pub package: BlueprintPackage,
    pub units: Vec<BlueprintUnit>,
    #[serde(default)]
    pub protocols: Vec<BlueprintProtocol>,
}

/// Declares a shared interface boundary between implementor and consumer units.
/// Used by `synth create` to auto-generate contract verification lanes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct BlueprintProtocol {
    pub id: String,
    pub trait_name: String,
    #[serde(default)]
    pub implementor_units: Vec<String>,
    #[serde(default)]
    pub consumer_units: Vec<String>,
    #[serde(default)]
    pub verification_command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct BlueprintProgram {
    pub id: String,
    #[serde(default = "default_max_parallel")]
    pub max_parallel: usize,
    #[serde(default)]
    pub state_path: Option<PathBuf>,
    #[serde(default)]
    pub run_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct BlueprintInputs {
    #[serde(default)]
    pub doctrine_files: Vec<PathBuf>,
    #[serde(default)]
    pub evidence_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct BlueprintPackage {
    #[serde(default = "default_fabro_root")]
    pub fabro_root: PathBuf,
}

impl Default for BlueprintPackage {
    fn default() -> Self {
        Self {
            fabro_root: default_fabro_root(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct BlueprintUnit {
    pub id: String,
    pub title: String,
    pub output_root: PathBuf,
    #[serde(default)]
    pub artifacts: Vec<BlueprintArtifact>,
    #[serde(default)]
    pub milestones: Vec<MilestoneManifest>,
    #[serde(default)]
    pub lanes: Vec<BlueprintLane>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct BlueprintArtifact {
    pub id: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct BlueprintLane {
    pub id: String,
    #[serde(default)]
    pub kind: LaneKind,
    pub title: String,
    pub family: String,
    #[serde(default)]
    pub workflow_family: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub template: WorkflowTemplate,
    pub goal: String,
    pub managed_milestone: String,
    #[serde(default)]
    pub dependencies: Vec<LaneDependency>,
    #[serde(default)]
    pub produces: Vec<String>,
    #[serde(default)]
    pub proof_profile: Option<String>,
    #[serde(default)]
    pub proof_state_path: Option<PathBuf>,
    #[serde(default)]
    pub program_manifest: Option<PathBuf>,
    #[serde(default)]
    pub service_state_path: Option<PathBuf>,
    #[serde(default)]
    pub orchestration_state_path: Option<PathBuf>,
    #[serde(default)]
    pub checks: Vec<LaneCheck>,
    #[serde(default)]
    pub run_dir: Option<PathBuf>,
    #[serde(default)]
    pub prompt_context: Option<String>,
    #[serde(default)]
    pub verify_command: Option<String>,
    #[serde(default)]
    pub health_command: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTemplate {
    #[default]
    Bootstrap,
    ServiceBootstrap,
    Implementation,
    Integration,
    Orchestration,
    RecurringReport,
}

pub fn load_blueprint(path: &Path) -> Result<ProgramBlueprint, BlueprintError> {
    let raw = std::fs::read_to_string(path).map_err(|source| BlueprintError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let blueprint: ProgramBlueprint =
        serde_yaml::from_str(&raw).map_err(|source| BlueprintError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
    let blueprint = normalize_loaded_blueprint(blueprint);
    validate_blueprint(path, &blueprint)?;
    Ok(blueprint)
}

pub fn save_blueprint(path: &Path, blueprint: &ProgramBlueprint) -> Result<(), BlueprintError> {
    let blueprint = normalize_loaded_blueprint(blueprint.clone());
    validate_blueprint(path, &blueprint)?;
    let yaml = serde_yaml::to_string(&blueprint).map_err(|source| BlueprintError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    let trimmed = yaml.trim_start_matches("---\n");
    fabro_workflows::write_text_atomic(path, trimmed, "blueprint").map_err(|source| {
        BlueprintError::Read {
            path: path.to_path_buf(),
            source: std::io::Error::other(source.to_string()),
        }
    })
}

fn normalize_loaded_blueprint(mut blueprint: ProgramBlueprint) -> ProgramBlueprint {
    for unit in &mut blueprint.units {
        normalize_legacy_integration_lane(unit);
        upgrade_implementation_unit(unit);
    }
    blueprint
}

fn normalize_legacy_integration_lane(unit: &mut BlueprintUnit) {
    let has_real_integration_lane = unit.lanes.iter().any(|lane| {
        lane.template == WorkflowTemplate::Integration
            && lane.kind == LaneKind::Integration
            && lane
                .dependencies
                .iter()
                .any(|dependency| dependency.lane.is_some())
            && lane.managed_milestone == "integrated"
    });
    if has_real_integration_lane {
        return;
    }

    for lane in &mut unit.lanes {
        if lane.template != WorkflowTemplate::Integration {
            continue;
        }
        lane.template = WorkflowTemplate::Implementation;
        if lane.kind == LaneKind::Integration {
            lane.kind = LaneKind::Platform;
        }
        if lane.family == "integration" {
            lane.family = "implementation".to_string();
        }
        if lane.workflow_family.as_deref() == Some("integration") {
            lane.workflow_family = Some("implementation".to_string());
        }
        lane.managed_milestone = "integrated".to_string();
        lane.produces = vec![
            "implementation".to_string(),
            "verification".to_string(),
            "quality".to_string(),
            "promotion".to_string(),
            "integration".to_string(),
        ];
    }

    let required_artifacts = [
        ("implementation", "implementation.md"),
        ("verification", "verification.md"),
        ("quality", "quality.md"),
        ("promotion", "promotion.md"),
        ("integration", "integration.md"),
    ];
    for (artifact_id, path) in required_artifacts {
        if !unit
            .artifacts
            .iter()
            .any(|artifact| artifact.id == artifact_id)
        {
            unit.artifacts.push(BlueprintArtifact {
                id: artifact_id.to_string(),
                path: PathBuf::from(path),
            });
        }
    }
    for milestone in [
        MilestoneManifest {
            id: "implemented".to_string(),
            requires: vec!["implementation".to_string()],
        },
        MilestoneManifest {
            id: "verified".to_string(),
            requires: vec![
                "implementation".to_string(),
                "verification".to_string(),
                "quality".to_string(),
            ],
        },
        MilestoneManifest {
            id: "merge_ready".to_string(),
            requires: vec![
                "implementation".to_string(),
                "verification".to_string(),
                "quality".to_string(),
                "promotion".to_string(),
            ],
        },
        MilestoneManifest {
            id: "integrated".to_string(),
            requires: vec!["integration".to_string()],
        },
    ] {
        if !unit.milestones.iter().any(|entry| entry.id == milestone.id) {
            unit.milestones.push(milestone);
        }
    }
}

pub fn validate_blueprint(path: &Path, blueprint: &ProgramBlueprint) -> Result<(), BlueprintError> {
    if blueprint.version != SUPPORTED_BLUEPRINT_VERSION {
        return Err(invalid(
            path,
            format!("unsupported blueprint version {}", blueprint.version),
        ));
    }
    if blueprint.program.id.trim().is_empty() {
        return Err(invalid(path, "program.id must not be empty"));
    }
    if blueprint.program.max_parallel == 0 {
        return Err(invalid(path, "program.max_parallel must be at least 1"));
    }
    if blueprint.units.is_empty() {
        return Err(invalid(path, "blueprint must define at least one unit"));
    }

    let mut unit_ids = BTreeSet::new();
    for unit in &blueprint.units {
        if unit.id.trim().is_empty() {
            return Err(invalid(path, "unit id must not be empty"));
        }
        if !unit_ids.insert(unit.id.clone()) {
            return Err(invalid(path, format!("duplicate unit id `{}`", unit.id)));
        }
        validate_unit(path, unit, &blueprint.units)?;
    }

    Ok(())
}

pub fn import_existing_package(req: ImportRequest<'_>) -> Result<ProgramBlueprint, BlueprintError> {
    let manifest_path = req
        .target_repo
        .join(DEFAULT_PACKAGE_DIR)
        .join("programs")
        .join(format!("{}.yaml", req.program));
    if !manifest_path.exists() {
        return Err(BlueprintError::MissingProgramManifest {
            path: manifest_path,
        });
    }

    let manifest =
        ProgramManifest::load(&manifest_path).map_err(|source| BlueprintError::Manifest {
            path: manifest_path.clone(),
            source,
        })?;

    let units = manifest
        .units
        .iter()
        .map(|(unit_id, unit)| import_unit(req, &manifest, &manifest_path, unit_id, unit))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ProgramBlueprint {
        version: SUPPORTED_BLUEPRINT_VERSION,
        program: BlueprintProgram {
            id: manifest.program.clone(),
            max_parallel: manifest.max_parallel,
            state_path: Some(repo_relative(
                &manifest.resolved_state_path(&manifest_path),
                req.target_repo,
            )?),
            run_dir: manifest
                .resolved_run_dir(&manifest_path)
                .map(|path| repo_relative(&path, req.target_repo))
                .transpose()?,
        },
        inputs: BlueprintInputs::default(),
        package: BlueprintPackage::default(),
        units,
        protocols: Vec::new(),
    })
}

impl BlueprintLane {
    pub fn slug(&self) -> &str {
        self.slug.as_deref().unwrap_or(&self.id)
    }

    pub fn workflow_family(&self) -> &str {
        self.workflow_family.as_deref().unwrap_or(&self.family)
    }
}

fn validate_unit(
    path: &Path,
    unit: &BlueprintUnit,
    all_units: &[BlueprintUnit],
) -> Result<(), BlueprintError> {
    let program_only_unit = unit
        .lanes
        .iter()
        .all(|lane| lane.program_manifest.is_some());
    if unit.title.trim().is_empty() {
        return Err(invalid(
            path,
            format!("unit `{}` title must not be empty", unit.id),
        ));
    }
    if unit.artifacts.is_empty() && !program_only_unit {
        return Err(invalid(
            path,
            format!("unit `{}` must define artifacts", unit.id),
        ));
    }
    if unit.milestones.is_empty() && !program_only_unit {
        return Err(invalid(
            path,
            format!("unit `{}` must define milestones", unit.id),
        ));
    }

    let artifact_ids = collect_unique(
        path,
        &unit.id,
        "artifact",
        unit.artifacts.iter().map(|a| &a.id),
    )?;
    let milestone_ids = collect_unique(
        path,
        &unit.id,
        "milestone",
        unit.milestones.iter().map(|m| &m.id),
    )?;
    let lane_ids = collect_unique(path, &unit.id, "lane", unit.lanes.iter().map(|l| &l.id))?;

    for milestone in &unit.milestones {
        for artifact in &milestone.requires {
            if !artifact_ids.contains(artifact) {
                return Err(invalid(
                    path,
                    format!(
                        "unit `{}` milestone `{}` requires unknown artifact `{artifact}`",
                        unit.id, milestone.id
                    ),
                ));
            }
        }
    }

    for lane in &unit.lanes {
        if lane.title.trim().is_empty() {
            return Err(invalid(
                path,
                format!("lane `{}` title must not be empty", lane.id),
            ));
        }
        if lane.family.trim().is_empty() {
            return Err(invalid(
                path,
                format!("lane `{}` family must not be empty", lane.id),
            ));
        }
        if lane.goal.trim().is_empty() {
            return Err(invalid(
                path,
                format!("lane `{}` goal must not be empty", lane.id),
            ));
        }
        if lane.program_manifest.is_none() && !milestone_ids.contains(&lane.managed_milestone) {
            return Err(invalid(
                path,
                format!(
                    "lane `{}` in unit `{}` manages unknown milestone `{}`",
                    lane.id, unit.id, lane.managed_milestone
                ),
            ));
        }
        for artifact in &lane.produces {
            if !artifact_ids.contains(artifact) {
                return Err(invalid(
                    path,
                    format!(
                        "lane `{}` in unit `{}` produces unknown artifact `{artifact}`",
                        lane.id, unit.id
                    ),
                ));
            }
        }
        validate_lane_verify_command(path, unit, lane)?;
        validate_dependencies(path, unit, lane, all_units, &lane_ids, &milestone_ids)?;
    }

    Ok(())
}

fn validate_lane_verify_command(
    path: &Path,
    unit: &BlueprintUnit,
    lane: &BlueprintLane,
) -> Result<(), BlueprintError> {
    let Some(verify_command) = lane.verify_command.as_deref() else {
        return Ok(());
    };
    let owned_surfaces = extract_owned_surfaces_from_goal(&lane.goal);
    if owned_surfaces.is_empty() {
        return Ok(());
    }

    if verify_command.contains("--workspace") {
        return Err(invalid(
            path,
            format!(
                "lane `{}` in unit `{}` uses a workspace-wide verify command despite lane-owned surfaces; narrow `{}` to the owned crate or file target",
                lane.id, unit.id, verify_command
            ),
        ));
    }

    for surface in &owned_surfaces {
        if let Some(expected_package) = expected_package_for_surface(surface) {
            let targets_package = verify_command.contains(&format!("-p {expected_package}"))
                || verify_command.contains(&format!("--package {expected_package}"))
                || verify_command.contains(&format!("--bin {expected_package}"));
            let names_another_package = verify_command.contains("-p ")
                || verify_command.contains("--package ")
                || verify_command.contains("--bin ");
            if names_another_package && !targets_package {
                return Err(invalid(
                    path,
                    format!(
                        "lane `{}` in unit `{}` verifies `{surface}` but command `{verify_command}` targets a different crate or binary than `{expected_package}`",
                        lane.id, unit.id
                    ),
                ));
            }
        }

        if let Some((bin_name, expected_selector)) = expected_bin_selector_for_surface(surface) {
            let targets_bin = verify_command.contains(&format!("--bin {bin_name}"));
            let has_selector = verify_command.contains(" -- ");
            let matches_selector = verify_command.contains(&format!(" -- {expected_selector}"))
                || verify_command.contains(&format!(" -- --nocapture {expected_selector}"))
                || verify_command.contains(&format!(" -- {expected_selector} "));
            if targets_bin && has_selector && !matches_selector {
                return Err(invalid(
                    path,
                    format!(
                        "lane `{}` in unit `{}` owns `{surface}` but verify command `{verify_command}` uses a mismatched selector; expected `{expected_selector}` for binary `{bin_name}`",
                        lane.id, unit.id
                    ),
                ));
            }
        }
    }

    Ok(())
}

fn extract_owned_surfaces_from_goal(goal: &str) -> Vec<String> {
    let Some((_, remainder)) = goal.split_once("Owned surfaces:") else {
        return Vec::new();
    };
    let mut surfaces = Vec::new();
    let mut collecting = false;
    for raw_line in remainder.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            if collecting {
                break;
            }
            continue;
        }
        if let Some(surface) = line.strip_prefix("- ") {
            collecting = true;
            surfaces.push(surface.trim().trim_matches('`').to_string());
            continue;
        }
        if collecting {
            break;
        }
    }
    surfaces
}

fn expected_package_for_surface(surface: &str) -> Option<&str> {
    let parts = surface.split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        ["crates", crate_name, "src", ..] => Some(*crate_name),
        ["bin", bin_name, "src", ..] => Some(*bin_name),
        _ => None,
    }
}

fn expected_bin_selector_for_surface(surface: &str) -> Option<(&str, &str)> {
    let parts = surface.split('/').collect::<Vec<_>>();
    let ["bin", bin_name, "src", file_name] = parts.as_slice() else {
        return None;
    };
    let selector = file_name.strip_suffix(".rs")?;
    if matches!(selector, "lib" | "main" | "mod") {
        return None;
    }
    Some((bin_name, selector))
}

fn validate_dependencies(
    path: &Path,
    unit: &BlueprintUnit,
    lane: &BlueprintLane,
    all_units: &[BlueprintUnit],
    lane_ids: &BTreeSet<String>,
    milestone_ids: &BTreeSet<String>,
) -> Result<(), BlueprintError> {
    for dependency in &lane.dependencies {
        let Some(target_unit) = all_units
            .iter()
            .find(|candidate| candidate.id == dependency.unit)
        else {
            return Err(invalid(
                path,
                format!(
                    "lane `{}` in unit `{}` depends on unknown unit `{}`",
                    lane.id, unit.id, dependency.unit
                ),
            ));
        };

        if dependency.unit == unit.id {
            if let Some(target_lane) = &dependency.lane {
                if !lane_ids.contains(target_lane) {
                    return Err(invalid(
                        path,
                        format!(
                            "lane `{}` in unit `{}` depends on unknown lane `{}`",
                            lane.id, unit.id, target_lane
                        ),
                    ));
                }
            }
            if let Some(target_milestone) = &dependency.milestone {
                let milestone_known_on_lane = dependency
                    .lane
                    .as_ref()
                    .and_then(|target_lane| {
                        unit.lanes
                            .iter()
                            .find(|candidate| &candidate.id == target_lane)
                    })
                    .map(|target_lane| target_lane.managed_milestone == *target_milestone)
                    .unwrap_or(false);
                if !milestone_ids.contains(target_milestone) && !milestone_known_on_lane {
                    return Err(invalid(
                        path,
                        format!(
                            "lane `{}` in unit `{}` depends on unknown milestone `{}`",
                            lane.id, unit.id, target_milestone
                        ),
                    ));
                }
            }
            continue;
        }

        if let Some(target_lane) = &dependency.lane {
            if !target_unit
                .lanes
                .iter()
                .any(|candidate| &candidate.id == target_lane)
            {
                return Err(invalid(
                    path,
                    format!(
                        "lane `{}` in unit `{}` depends on unknown lane `{}` in unit `{}`",
                        lane.id, unit.id, target_lane, dependency.unit
                    ),
                ));
            }
        }
        if let Some(target_milestone) = &dependency.milestone {
            let milestone_known_on_unit = target_unit
                .milestones
                .iter()
                .any(|candidate| &candidate.id == target_milestone);
            let milestone_known_on_lane = dependency
                .lane
                .as_ref()
                .and_then(|target_lane| {
                    target_unit
                        .lanes
                        .iter()
                        .find(|candidate| &candidate.id == target_lane)
                })
                .map(|target_lane| target_lane.managed_milestone == *target_milestone)
                .unwrap_or(false);
            if !milestone_known_on_unit && !milestone_known_on_lane {
                return Err(invalid(
                    path,
                    format!(
                        "lane `{}` in unit `{}` depends on unknown milestone `{}` in unit `{}`",
                        lane.id, unit.id, target_milestone, dependency.unit
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn collect_unique<'a, I>(
    path: &Path,
    unit_id: &str,
    kind: &str,
    values: I,
) -> Result<BTreeSet<String>, BlueprintError>
where
    I: Iterator<Item = &'a String>,
{
    let mut seen = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() {
            return Err(invalid(
                path,
                format!("unit `{unit_id}` has empty {kind} id"),
            ));
        }
        if !seen.insert(value.clone()) {
            return Err(invalid(
                path,
                format!("unit `{unit_id}` has duplicate {kind} id `{value}`"),
            ));
        }
    }
    Ok(seen)
}

fn import_unit(
    req: ImportRequest<'_>,
    manifest: &ProgramManifest,
    manifest_path: &Path,
    unit_id: &str,
    unit: &raspberry_supervisor::manifest::UnitManifest,
) -> Result<BlueprintUnit, BlueprintError> {
    let output_root = manifest
        .resolve_unit_output_root(manifest_path, unit_id)
        .ok_or_else(|| {
            invalid(
                manifest_path,
                format!("unit `{unit_id}` has no output_root"),
            )
        })?;
    let output_root = repo_relative(&output_root, req.target_repo)?;
    let output_root_abs = req.target_repo.join(&output_root);

    let artifacts = unit
        .artifacts
        .iter()
        .map(|(id, relative)| {
            let absolute = output_root_abs.join(relative);
            let relative_path = absolute
                .strip_prefix(&output_root_abs)
                .unwrap_or(relative.as_path())
                .to_path_buf();
            BlueprintArtifact {
                id: id.clone(),
                path: relative_path,
            }
        })
        .collect();

    let lanes = unit
        .lanes
        .iter()
        .map(|(lane_id, lane)| {
            import_lane(req, manifest, manifest_path, unit_id, lane_id, unit, lane)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut imported = BlueprintUnit {
        id: unit_id.to_string(),
        title: unit.title.clone(),
        output_root,
        artifacts,
        milestones: unit.milestones.clone(),
        lanes,
    };
    upgrade_implementation_unit(&mut imported);
    Ok(imported)
}

fn import_lane(
    req: ImportRequest<'_>,
    manifest: &ProgramManifest,
    manifest_path: &Path,
    unit_id: &str,
    lane_id: &str,
    unit: &raspberry_supervisor::manifest::UnitManifest,
    lane: &raspberry_supervisor::manifest::LaneManifest,
) -> Result<BlueprintLane, BlueprintError> {
    let program_manifest = manifest.resolve_lane_program_manifest(manifest_path, unit_id, lane_id);
    if let Some(program_manifest) = program_manifest {
        return Ok(BlueprintLane {
            id: lane_id.to_string(),
            kind: lane.kind,
            title: lane.title.clone(),
            family: "program".to_string(),
            workflow_family: None,
            slug: program_manifest
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToOwned::to_owned),
            template: WorkflowTemplate::Bootstrap,
            goal: format!("Coordinate the child program `{}`.", lane.title),
            managed_milestone: lane.managed_milestone.clone(),
            dependencies: lane.dependencies.clone(),
            produces: lane.produces.clone(),
            proof_profile: lane.proof_profile.clone(),
            proof_state_path: lane
                .proof_state_path
                .as_ref()
                .map(|path| {
                    repo_relative(
                        &resolve_relative_to_manifest(manifest_path, path),
                        req.target_repo,
                    )
                })
                .transpose()?,
            program_manifest: Some(repo_relative(&program_manifest, req.target_repo)?),
            service_state_path: lane
                .service_state_path
                .as_ref()
                .map(|path| {
                    repo_relative(
                        &resolve_relative_to_manifest(manifest_path, path),
                        req.target_repo,
                    )
                })
                .transpose()?,
            orchestration_state_path: lane
                .orchestration_state_path
                .as_ref()
                .map(|path| {
                    repo_relative(
                        &resolve_relative_to_manifest(manifest_path, path),
                        req.target_repo,
                    )
                })
                .transpose()?,
            checks: lane
                .checks
                .iter()
                .map(|check| normalize_check(manifest_path, check, req.target_repo))
                .collect::<Result<Vec<_>, _>>()?,
            run_dir: lane
                .run_dir
                .as_ref()
                .map(|path| {
                    repo_relative(
                        &resolve_relative_to_manifest(manifest_path, path),
                        req.target_repo,
                    )
                })
                .transpose()?,
            prompt_context: None,
            verify_command: None,
            health_command: None,
        });
    }

    let run_config = manifest
        .resolve_lane_run_config(manifest_path, unit_id, lane_id)
        .ok_or_else(|| invalid(manifest_path, format!("lane `{lane_id}` has no run_config")))?;
    let run_config_cfg =
        load_run_config(&run_config).map_err(|source| BlueprintError::RunConfig {
            path: run_config.clone(),
            source,
        })?;
    let family = run_config
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("bootstrap")
        .to_string();

    let verify_command = imported_verify_command(&lane.checks);
    let health_command = imported_health_command(&lane.checks);

    Ok(BlueprintLane {
        id: lane_id.to_string(),
        kind: lane.kind,
        title: lane.title.clone(),
        family,
        workflow_family: Path::new(&run_config_cfg.graph)
            .parent()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned),
        slug: run_config
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(ToOwned::to_owned),
        template: infer_template(unit, lane),
        goal: run_config_cfg
            .goal
            .unwrap_or_else(|| format!("Describe the `{lane_id}` lane goals.")),
        managed_milestone: lane.managed_milestone.clone(),
        dependencies: lane.dependencies.clone(),
        produces: lane.produces.clone(),
        proof_profile: lane.proof_profile.clone(),
        proof_state_path: lane
            .proof_state_path
            .as_ref()
            .map(|path| {
                repo_relative(
                    &resolve_relative_to_manifest(manifest_path, path),
                    req.target_repo,
                )
            })
            .transpose()?,
        program_manifest: lane
            .program_manifest
            .as_ref()
            .map(|path| {
                repo_relative(
                    &resolve_relative_to_manifest(manifest_path, path),
                    req.target_repo,
                )
            })
            .transpose()?,
        service_state_path: lane
            .service_state_path
            .as_ref()
            .map(|path| {
                repo_relative(
                    &resolve_relative_to_manifest(manifest_path, path),
                    req.target_repo,
                )
            })
            .transpose()?,
        orchestration_state_path: lane
            .orchestration_state_path
            .as_ref()
            .map(|path| {
                repo_relative(
                    &resolve_relative_to_manifest(manifest_path, path),
                    req.target_repo,
                )
            })
            .transpose()?,
        checks: lane
            .checks
            .iter()
            .map(|check| normalize_check(manifest_path, check, req.target_repo))
            .collect::<Result<Vec<_>, _>>()?,
        run_dir: lane
            .run_dir
            .as_ref()
            .map(|path| {
                repo_relative(
                    &resolve_relative_to_manifest(manifest_path, path),
                    req.target_repo,
                )
            })
            .transpose()?,
        prompt_context: None,
        verify_command,
        health_command,
    })
}

fn default_version() -> u32 {
    SUPPORTED_BLUEPRINT_VERSION
}

pub const DEFAULT_PACKAGE_DIR: &str = "malinka";

fn default_fabro_root() -> PathBuf {
    PathBuf::from(DEFAULT_PACKAGE_DIR)
}

const fn default_max_parallel() -> usize {
    1
}

fn infer_template(
    unit: &raspberry_supervisor::manifest::UnitManifest,
    lane: &raspberry_supervisor::manifest::LaneManifest,
) -> WorkflowTemplate {
    if lane.kind == LaneKind::Integration {
        return WorkflowTemplate::Integration;
    }
    if lane.kind == LaneKind::Orchestration || lane.program_manifest.is_some() {
        return WorkflowTemplate::Orchestration;
    }
    let artifact_ids = unit
        .artifacts
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let produced = lane
        .produces
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let looks_like_implementation = artifact_ids.contains("implementation")
        && artifact_ids.contains("verification")
        && artifact_ids.contains("promotion")
        && produced.contains("implementation")
        && produced.contains("verification")
        && produced.contains("promotion");
    if looks_like_implementation || lane.managed_milestone == "merge_ready" {
        return WorkflowTemplate::Implementation;
    }

    match lane.kind {
        LaneKind::Service => WorkflowTemplate::ServiceBootstrap,
        LaneKind::Recurring => WorkflowTemplate::RecurringReport,
        LaneKind::Orchestration => WorkflowTemplate::Orchestration,
        _ => WorkflowTemplate::Bootstrap,
    }
}

fn upgrade_implementation_unit(unit: &mut BlueprintUnit) {
    if !unit
        .lanes
        .iter()
        .any(|lane| lane.template == WorkflowTemplate::Implementation)
    {
        return;
    }

    if !unit
        .artifacts
        .iter()
        .any(|artifact| artifact.id == "quality")
    {
        let quality_path = unit
            .artifacts
            .iter()
            .find(|artifact| artifact.id == "implementation")
            .map(|artifact| artifact.path.with_file_name("quality.md"))
            .unwrap_or_else(|| PathBuf::from("quality.md"));
        unit.artifacts.push(BlueprintArtifact {
            id: "quality".to_string(),
            path: quality_path,
        });
    }

    if !unit
        .artifacts
        .iter()
        .any(|artifact| artifact.id == "integration")
    {
        let integration_path = unit
            .artifacts
            .iter()
            .find(|artifact| artifact.id == "implementation")
            .map(|artifact| artifact.path.with_file_name("integration.md"))
            .unwrap_or_else(|| PathBuf::from("integration.md"));
        unit.artifacts.push(BlueprintArtifact {
            id: "integration".to_string(),
            path: integration_path,
        });
    }

    for milestone in &mut unit.milestones {
        if matches!(milestone.id.as_str(), "verified" | "merge_ready")
            && !milestone
                .requires
                .iter()
                .any(|artifact| artifact == "quality")
        {
            if let Some(promotion_index) = milestone
                .requires
                .iter()
                .position(|artifact| artifact == "promotion")
            {
                milestone
                    .requires
                    .insert(promotion_index, "quality".to_string());
            } else {
                milestone.requires.push("quality".to_string());
            }
        }
    }

    if !unit
        .milestones
        .iter()
        .any(|milestone| milestone.id == "integrated")
    {
        unit.milestones.push(MilestoneManifest {
            id: "integrated".to_string(),
            requires: vec!["integration".to_string()],
        });
    }

    for lane in &mut unit.lanes {
        if lane.template != WorkflowTemplate::Implementation {
            continue;
        }
        if !lane.produces.iter().any(|artifact| artifact == "quality") {
            if let Some(promotion_index) = lane
                .produces
                .iter()
                .position(|artifact| artifact == "promotion")
            {
                lane.produces.insert(promotion_index, "quality".to_string());
            } else {
                lane.produces.push("quality".to_string());
            }
        }
        if !lane
            .produces
            .iter()
            .any(|artifact| artifact == "integration")
        {
            lane.produces.push("integration".to_string());
        }
    }

    unit.lanes
        .retain(|lane| lane.template != WorkflowTemplate::Integration);
}

fn repo_relative(path: &Path, target_repo: &Path) -> Result<PathBuf, BlueprintError> {
    let path = normalize_path(path);
    let target_repo = normalize_path(target_repo);
    path.strip_prefix(&target_repo)
        .map(normalize_relative_path)
        .map_err(|_| BlueprintError::PathOutsideTargetRepo { path, target_repo })
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

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(part) => normalized.push(part),
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(std::path::MAIN_SEPARATOR.to_string()),
        }
    }
    normalized
}

fn resolve_relative_to_manifest(manifest_path: &Path, relative: &Path) -> PathBuf {
    let parent = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(relative)
}

fn normalize_check(
    manifest_path: &Path,
    check: &raspberry_supervisor::manifest::LaneCheck,
    target_repo: &Path,
) -> Result<raspberry_supervisor::manifest::LaneCheck, BlueprintError> {
    use raspberry_supervisor::manifest::LaneCheckProbe;

    let probe = match &check.probe {
        LaneCheckProbe::FileExists { path } => LaneCheckProbe::FileExists {
            path: repo_relative(
                &resolve_relative_to_manifest(manifest_path, path),
                target_repo,
            )?,
        },
        LaneCheckProbe::JsonFieldEquals {
            path,
            field,
            equals,
        } => LaneCheckProbe::JsonFieldEquals {
            path: repo_relative(
                &resolve_relative_to_manifest(manifest_path, path),
                target_repo,
            )?,
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

    Ok(raspberry_supervisor::manifest::LaneCheck {
        label: check.label.clone(),
        kind: check.kind,
        scope: check.scope,
        probe,
    })
}

fn imported_verify_command(checks: &[LaneCheck]) -> Option<String> {
    checks.iter().find_map(|check| {
        if check.kind != LaneCheckKind::Proof || check.scope != LaneCheckScope::Running {
            return None;
        }
        match &check.probe {
            LaneCheckProbe::CommandSucceeds { command }
            | LaneCheckProbe::CommandStdoutContains { command, .. } => Some(command.clone()),
            _ => None,
        }
    })
}

fn imported_health_command(checks: &[LaneCheck]) -> Option<String> {
    checks.iter().find_map(|check| {
        if check.kind != LaneCheckKind::Health || check.scope != LaneCheckScope::Running {
            return None;
        }
        match &check.probe {
            LaneCheckProbe::CommandSucceeds { command }
            | LaneCheckProbe::CommandStdoutContains { command, .. } => Some(command.clone()),
            _ => None,
        }
    })
}

fn invalid(path: &Path, message: impl Into<String>) -> BlueprintError {
    BlueprintError::Invalid {
        path: path.to_path_buf(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn import_existing_package_preserves_implementation_template() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path();
        std::fs::create_dir_all(repo.join("malinka/programs")).expect("program dir");
        std::fs::create_dir_all(repo.join("malinka/run-configs/implement"))
            .expect("run-config dir");
        std::fs::write(
            repo.join("malinka/run-configs/implement/demo.toml"),
            "version = 1\ngraph = \"../../workflows/implement/demo.fabro\"\ngoal = \"Implement demo\"\ndirectory = \"../../..\"\n\n[sandbox]\nprovider = \"local\"\n\n[sandbox.local]\nworktree_mode = \"clean\"\n",
        )
        .expect("run config");
        std::fs::write(
            repo.join("malinka/programs/demo-implementation.yaml"),
            r#"
version: 1
program: demo-implementation
target_repo: ../..
state_path: ../../.raspberry/demo-implementation-state.json
max_parallel: 1
units:
  - id: demo
    title: Demo Delivery
    output_root: ../../outputs/demo
    artifacts:
      - id: implementation
        path: implementation.md
      - id: promotion
        path: promotion.md
      - id: review
        path: review.md
      - id: spec
        path: spec.md
      - id: verification
        path: verification.md
    milestones:
      - id: reviewed
        requires: [spec, review]
      - id: implemented
        requires: [spec, review, implementation]
      - id: verified
        requires: [spec, review, implementation, verification]
      - id: merge_ready
        requires: [spec, review, implementation, verification, promotion]
    lanes:
      - id: implement
        kind: interface
        title: Demo Implementation Lane
        run_config: ../run-configs/implement/demo.toml
        managed_milestone: merge_ready
        produces: [implementation, verification, promotion]
        checks:
          - label: demo_proof
            kind: proof
            scope: running
            type: command_succeeds
            command: cargo test -p demo
"#,
        )
        .expect("program manifest");

        let blueprint = import_existing_package(ImportRequest {
            target_repo: repo,
            program: "demo-implementation",
        })
        .expect("blueprint imports");

        assert_eq!(blueprint.units.len(), 1);
        assert_eq!(blueprint.units[0].lanes.len(), 1);
        assert_eq!(
            blueprint.units[0].lanes[0].template,
            WorkflowTemplate::Implementation
        );
        assert!(blueprint.units[0].artifacts.iter().any(
            |artifact| artifact.id == "quality" && artifact.path == PathBuf::from("quality.md")
        ));
        assert!(blueprint.units[0]
            .artifacts
            .iter()
            .any(|artifact| artifact.id == "integration"
                && artifact.path == PathBuf::from("integration.md")));
        assert!(blueprint.units[0].milestones.iter().any(|milestone| {
            milestone.id == "verified"
                && milestone
                    .requires
                    .iter()
                    .any(|artifact| artifact == "quality")
        }));
        assert!(blueprint.units[0].milestones.iter().any(|milestone| {
            milestone.id == "merge_ready"
                && milestone
                    .requires
                    .iter()
                    .any(|artifact| artifact == "quality")
        }));
        assert!(blueprint.units[0].milestones.iter().any(|milestone| {
            milestone.id == "integrated"
                && milestone
                    .requires
                    .iter()
                    .any(|artifact| artifact == "integration")
        }));
        assert!(blueprint.units[0].lanes[0]
            .produces
            .iter()
            .any(|artifact| artifact == "quality"));
        assert!(blueprint.units[0].lanes[0]
            .produces
            .iter()
            .any(|artifact| artifact == "integration"));
        assert_eq!(
            blueprint.units[0].lanes[0].verify_command.as_deref(),
            Some("cargo test -p demo")
        );
    }

    #[test]
    fn load_blueprint_normalizes_legacy_integration_template_without_lane_dependency() {
        let temp = tempfile::tempdir().expect("tempdir");
        let blueprint_path = temp.path().join("legacy.yaml");
        std::fs::write(
            &blueprint_path,
            r#"
version: 1
program:
  id: legacy
  max_parallel: 1
units:
  - id: provably-fair
    title: Provably Fair
    output_root: outputs/provably-fair
    artifacts:
      - id: spec
        path: spec.md
      - id: review
        path: review.md
    milestones:
      - id: reviewed
        requires: [spec, review]
    lanes:
      - id: provably-fair
        kind: platform
        title: Provably Fair Lane
        family: bootstrap
        workflow_family: bootstrap
        template: bootstrap
        goal: Bootstrap provably fair
        managed_milestone: reviewed
        produces: [spec, review]
  - id: provably-fair-integration-tests
    title: Cross-crate integration tests
    output_root: outputs/provably-fair-integration-tests
    artifacts:
      - id: spec
        path: spec.md
      - id: review
        path: review.md
    milestones:
      - id: reviewed
        requires: [spec, review]
    lanes:
      - id: provably-fair-integration-tests
        kind: platform
        title: Cross-crate integration tests Lane
        family: integration
        workflow_family: integration
        template: integration
        goal: Cross-crate integration tests
        managed_milestone: reviewed
        dependencies:
          - unit: provably-fair
            lane: null
            milestone: reviewed
        produces: [spec, review]
"#,
        )
        .expect("blueprint");

        let blueprint = load_blueprint(&blueprint_path).expect("normalized load");
        let unit = blueprint
            .units
            .iter()
            .find(|unit| unit.id == "provably-fair-integration-tests")
            .expect("normalized unit");
        let lane = &unit.lanes[0];

        assert_eq!(lane.template, WorkflowTemplate::Implementation);
        assert_eq!(lane.family, "implementation");
        assert_eq!(lane.workflow_family(), "implementation");
        assert_eq!(lane.kind, LaneKind::Platform);
        assert_eq!(lane.managed_milestone, "integrated");
        assert!(lane
            .produces
            .iter()
            .any(|artifact| artifact == "integration"));
        assert!(unit
            .artifacts
            .iter()
            .any(|artifact| artifact.id == "implementation"));
        assert!(unit
            .milestones
            .iter()
            .any(|milestone| milestone.id == "integrated"));
    }

    #[test]
    fn infer_template_prefers_orchestration_for_child_program_lanes() {
        let unit = raspberry_supervisor::manifest::UnitManifest {
            title: "Program".to_string(),
            output_root: None,
            artifacts: BTreeMap::new(),
            milestones: Vec::new(),
            lanes: BTreeMap::new(),
        };
        let lane = raspberry_supervisor::manifest::LaneManifest {
            kind: LaneKind::Orchestration,
            title: "Program Lane".to_string(),
            run_config: PathBuf::from("malinka/run-configs/orchestrate/program.toml"),
            managed_milestone: "coordinated".to_string(),
            dependencies: Vec::new(),
            produces: Vec::new(),
            proof_profile: None,
            proof_state_path: None,
            program_manifest: Some(PathBuf::from("malinka/programs/demo.yaml")),
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
        };

        assert_eq!(
            infer_template(&unit, &lane),
            WorkflowTemplate::Orchestration
        );
    }

    #[test]
    fn validate_blueprint_rejects_workspace_wide_verify_for_owned_surface_lane() {
        let temp = tempfile::tempdir().expect("tempdir");
        let blueprint_path = temp.path().join("rxmragent.yaml");
        std::fs::write(
            &blueprint_path,
            r#"
version: 1
program:
  id: demo
units:
  - id: house-session-tests
    title: House Session Tests
    output_root: outputs/house-session-tests
    artifacts:
      - id: verification
        path: verification.md
    milestones:
      - id: verified
        requires: [verification]
    lanes:
      - id: house-session-tests
        title: House Session Tests
        family: implementation
        workflow_family: implementation
        template: implementation
        goal: |
          Owned surfaces:
          - `bin/house/src/session.rs`
        managed_milestone: verified
        produces: [verification]
        verify_command: cargo test --workspace
"#,
        )
        .expect("blueprint");

        let error = load_blueprint(&blueprint_path).expect_err("workspace verify should fail");
        assert!(matches!(error, BlueprintError::Invalid { .. }));
        assert!(error.to_string().contains("workspace-wide verify command"));
    }

    #[test]
    fn validate_blueprint_rejects_mismatched_bin_selector_for_owned_surface_lane() {
        let temp = tempfile::tempdir().expect("tempdir");
        let blueprint_path = temp.path().join("rxmragent.yaml");
        std::fs::write(
            &blueprint_path,
            r#"
version: 1
program:
  id: demo
units:
  - id: house-session-tests
    title: House Session Tests
    output_root: outputs/house-session-tests
    artifacts:
      - id: verification
        path: verification.md
    milestones:
      - id: verified
        requires: [verification]
    lanes:
      - id: house-session-tests
        title: House Session Tests
        family: implementation
        workflow_family: implementation
        template: implementation
        goal: |
          Owned surfaces:
          - `bin/house/src/session.rs`
        managed_milestone: verified
        produces: [verification]
        verify_command: cargo test --bin house -- protocol
"#,
        )
        .expect("blueprint");

        let error = load_blueprint(&blueprint_path).expect_err("mismatched selector should fail");
        assert!(matches!(error, BlueprintError::Invalid { .. }));
        assert!(error.to_string().contains("mismatched selector"));
        assert!(error.to_string().contains("session"));
    }
}
