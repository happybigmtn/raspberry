use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub type ArtifactKey = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProgramManifest {
    pub program: String,
    pub target_repo: PathBuf,
    pub state_path: PathBuf,
    pub max_parallel: usize,
    pub run_dir: Option<PathBuf>,
    pub units: BTreeMap<String, UnitManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnitManifest {
    pub title: String,
    pub output_root: Option<PathBuf>,
    pub artifacts: BTreeMap<ArtifactKey, PathBuf>,
    pub milestones: Vec<MilestoneManifest>,
    pub lanes: BTreeMap<String, LaneManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct MilestoneManifest {
    pub id: String,
    #[serde(default)]
    pub requires: Vec<ArtifactKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LaneManifest {
    pub kind: LaneKind,
    pub title: String,
    pub run_config: PathBuf,
    pub managed_milestone: String,
    pub dependencies: Vec<LaneDependency>,
    pub produces: Vec<ArtifactKey>,
    pub proof_profile: Option<String>,
    pub proof_state_path: Option<PathBuf>,
    pub service_state_path: Option<PathBuf>,
    pub orchestration_state_path: Option<PathBuf>,
    pub checks: Vec<LaneCheck>,
    pub run_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneKind {
    Artifact,
    Service,
    Orchestration,
    Interface,
    Platform,
    Recurring,
}

impl Default for LaneKind {
    fn default() -> Self {
        Self::Artifact
    }
}

impl fmt::Display for LaneKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Artifact => "artifact",
            Self::Service => "service",
            Self::Orchestration => "orchestration",
            Self::Interface => "interface",
            Self::Platform => "platform",
            Self::Recurring => "recurring",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct LaneCheck {
    pub label: String,
    #[serde(default)]
    pub kind: LaneCheckKind,
    #[serde(default)]
    pub scope: LaneCheckScope,
    #[serde(flatten)]
    pub probe: LaneCheckProbe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneCheckKind {
    Precondition,
    Proof,
    Health,
}

impl Default for LaneCheckKind {
    fn default() -> Self {
        Self::Precondition
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneCheckScope {
    Ready,
    Running,
}

impl Default for LaneCheckScope {
    fn default() -> Self {
        Self::Ready
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LaneCheckProbe {
    FileExists { path: PathBuf },
    JsonFieldEquals { path: PathBuf, field: String, equals: Value },
    CommandSucceeds { command: String },
    CommandStdoutContains { command: String, contains: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct LaneDependency {
    pub unit: String,
    #[serde(default)]
    pub lane: Option<String>,
    #[serde(default)]
    pub milestone: Option<String>,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("failed to read program manifest {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse program manifest {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("program manifest {path} is invalid: {message}")]
    Invalid { path: PathBuf, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
enum ProgramManifestSource {
    Map(MapProgramManifest),
    List(ListProgramManifest),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct MapProgramManifest {
    pub program: String,
    #[serde(default = "default_target_repo")]
    pub target_repo: PathBuf,
    #[serde(default = "default_state_path")]
    pub state_path: PathBuf,
    #[serde(default = "default_max_parallel")]
    pub max_parallel: usize,
    #[serde(default)]
    pub run_dir: Option<PathBuf>,
    pub units: BTreeMap<String, MapUnitManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct MapUnitManifest {
    pub title: String,
    #[serde(default)]
    pub output_root: Option<PathBuf>,
    #[serde(default)]
    pub artifacts: BTreeMap<ArtifactKey, PathBuf>,
    #[serde(default)]
    pub milestones: Vec<MilestoneManifest>,
    pub lanes: BTreeMap<String, MapLaneManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct MapLaneManifest {
    #[serde(default)]
    pub kind: LaneKind,
    pub title: String,
    pub run_config: PathBuf,
    pub managed_milestone: String,
    #[serde(default)]
    pub dependencies: Vec<LaneDependency>,
    #[serde(default)]
    pub produces: Vec<ArtifactKey>,
    #[serde(default)]
    pub proof_profile: Option<String>,
    #[serde(default)]
    pub proof_state_path: Option<PathBuf>,
    #[serde(default)]
    pub service_state_path: Option<PathBuf>,
    #[serde(default)]
    pub orchestration_state_path: Option<PathBuf>,
    #[serde(default)]
    pub checks: Vec<LaneCheck>,
    #[serde(default)]
    pub run_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ListProgramManifest {
    #[serde(default)]
    pub version: Option<u32>,
    pub program: String,
    #[serde(default = "default_target_repo")]
    pub target_repo: PathBuf,
    #[serde(default = "default_state_path")]
    pub state_path: PathBuf,
    #[serde(default = "default_max_parallel")]
    pub max_parallel: usize,
    #[serde(default)]
    pub run_dir: Option<PathBuf>,
    pub units: Vec<ListUnitManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ListUnitManifest {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub output_root: Option<PathBuf>,
    #[serde(default)]
    pub artifacts: Vec<ListArtifactManifest>,
    #[serde(default)]
    pub milestones: Vec<MilestoneManifest>,
    #[serde(default)]
    pub lanes: Vec<ListLaneManifest>,
    #[serde(default)]
    pub run_config: Option<PathBuf>,
    #[serde(default)]
    pub managed_milestone: Option<String>,
    #[serde(default, alias = "depends_on")]
    pub dependencies: Vec<LaneDependency>,
    #[serde(default)]
    pub produces: Vec<ArtifactKey>,
    #[serde(default)]
    pub run_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ListArtifactManifest {
    pub id: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ListLaneManifest {
    pub id: String,
    #[serde(default)]
    pub kind: LaneKind,
    pub title: String,
    pub run_config: PathBuf,
    pub managed_milestone: String,
    #[serde(default, alias = "depends_on")]
    pub dependencies: Vec<LaneDependency>,
    #[serde(default)]
    pub produces: Vec<ArtifactKey>,
    #[serde(default)]
    pub proof_profile: Option<String>,
    #[serde(default)]
    pub proof_state_path: Option<PathBuf>,
    #[serde(default)]
    pub service_state_path: Option<PathBuf>,
    #[serde(default)]
    pub orchestration_state_path: Option<PathBuf>,
    #[serde(default)]
    pub checks: Vec<LaneCheck>,
    #[serde(default)]
    pub run_dir: Option<PathBuf>,
}

fn default_target_repo() -> PathBuf {
    PathBuf::from(".")
}

fn default_state_path() -> PathBuf {
    PathBuf::from(".raspberry/program-state.json")
}

const fn default_max_parallel() -> usize {
    1
}

impl ProgramManifest {
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let raw = std::fs::read_to_string(path).map_err(|source| ManifestError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let source: ProgramManifestSource =
            serde_yaml::from_str(&raw).map_err(|source| ManifestError::Parse {
                path: path.to_path_buf(),
                source,
            })?;
        let manifest = match source {
            ProgramManifestSource::Map(manifest) => normalize_map_manifest(manifest),
            ProgramManifestSource::List(manifest) => normalize_list_manifest(path, manifest)?,
        };
        manifest.validate(path)?;
        Ok(manifest)
    }

    pub fn resolved_target_repo(&self, manifest_path: &Path) -> PathBuf {
        resolve_relative(manifest_path, &self.target_repo)
    }

    pub fn resolved_state_path(&self, manifest_path: &Path) -> PathBuf {
        resolve_relative(manifest_path, &self.state_path)
    }

    pub fn resolved_run_dir(&self, manifest_path: &Path) -> Option<PathBuf> {
        self.run_dir
            .as_ref()
            .map(|path| resolve_relative(manifest_path, path))
    }

    pub fn resolve_unit_output_root(&self, manifest_path: &Path, unit: &str) -> Option<PathBuf> {
        self.units
            .get(unit)
            .and_then(|spec| spec.output_root.as_ref())
            .map(|spec| resolve_relative(manifest_path, spec))
    }

    pub fn resolve_lane_run_config(
        &self,
        manifest_path: &Path,
        unit: &str,
        lane: &str,
    ) -> Option<PathBuf> {
        self.units
            .get(unit)
            .and_then(|spec| spec.lanes.get(lane))
            .map(|lane_spec| resolve_relative(manifest_path, &lane_spec.run_config))
    }

    pub fn resolve_lane_run_dir(
        &self,
        manifest_path: &Path,
        unit: &str,
        lane: &str,
    ) -> Option<PathBuf> {
        self.units
            .get(unit)
            .and_then(|spec| spec.lanes.get(lane))
            .and_then(|lane_spec| lane_spec.run_dir.as_ref())
            .map(|path| resolve_relative(manifest_path, path))
    }

    fn validate(&self, manifest_path: &Path) -> Result<(), ManifestError> {
        if self.program.trim().is_empty() {
            return Err(invalid_manifest(manifest_path, "program must not be empty"));
        }
        if self.units.is_empty() {
            return Err(invalid_manifest(
                manifest_path,
                "manifest must define at least one unit",
            ));
        }
        if self.max_parallel == 0 {
            return Err(invalid_manifest(
                manifest_path,
                "max_parallel must be at least 1",
            ));
        }

        for (unit_id, unit) in &self.units {
            validate_unit(self, manifest_path, unit_id, unit)?;
        }

        Ok(())
    }
}

fn normalize_map_manifest(manifest: MapProgramManifest) -> ProgramManifest {
    ProgramManifest {
        program: manifest.program,
        target_repo: manifest.target_repo,
        state_path: manifest.state_path,
        max_parallel: manifest.max_parallel,
        run_dir: manifest.run_dir,
        units: manifest
            .units
            .into_iter()
            .map(|(unit_id, unit)| {
                (
                    unit_id,
                    UnitManifest {
                        title: unit.title,
                        output_root: unit.output_root,
                        artifacts: unit.artifacts,
                        milestones: unit.milestones,
                        lanes: unit
                            .lanes
                            .into_iter()
                            .map(|(lane_id, lane)| {
                                (
                                    lane_id,
                                    LaneManifest {
                                    title: lane.title,
                                        kind: lane.kind,
                                        run_config: lane.run_config,
                                        managed_milestone: lane.managed_milestone,
                                        dependencies: lane.dependencies,
                                        produces: lane.produces,
                                        proof_profile: lane.proof_profile,
                                        proof_state_path: lane.proof_state_path,
                                        service_state_path: lane.service_state_path,
                                        orchestration_state_path: lane.orchestration_state_path,
                                        checks: lane.checks,
                                        run_dir: lane.run_dir,
                                    },
                                )
                            })
                            .collect(),
                    },
                )
            })
            .collect(),
    }
}

fn normalize_list_manifest(
    manifest_path: &Path,
    manifest: ListProgramManifest,
) -> Result<ProgramManifest, ManifestError> {
    let mut units = BTreeMap::new();
    for unit in manifest.units {
        let lanes = if unit.lanes.is_empty() {
            let run_config = unit.run_config.clone().ok_or_else(|| {
                invalid_manifest(
                    manifest_path,
                    format!(
                        "unit `{}` must define either lanes or a unit-level run_config",
                        unit.id
                    ),
                )
            })?;
            let managed_milestone = unit.managed_milestone.clone().ok_or_else(|| {
                invalid_manifest(
                    manifest_path,
                    format!(
                        "unit `{}` must define managed_milestone when using unit-level run_config",
                        unit.id
                    ),
                )
            })?;
            BTreeMap::from([(
                "default".to_string(),
                        LaneManifest {
                            title: unit.title.clone(),
                            kind: LaneKind::Artifact,
                            run_config,
                            managed_milestone,
                            dependencies: unit.dependencies.clone(),
                            produces: unit.produces.clone(),
                            proof_profile: None,
                            proof_state_path: None,
                            service_state_path: None,
                            orchestration_state_path: None,
                            checks: Vec::new(),
                            run_dir: unit.run_dir.clone(),
                        },
                    )])
        } else {
            unit.lanes
                .into_iter()
                .map(|lane| {
                    (
                        lane.id,
                        LaneManifest {
                            title: lane.title,
                            kind: lane.kind,
                            run_config: lane.run_config,
                            managed_milestone: lane.managed_milestone,
                            dependencies: lane.dependencies,
                            produces: lane.produces,
                            proof_profile: lane.proof_profile,
                            proof_state_path: lane.proof_state_path,
                            service_state_path: lane.service_state_path,
                            orchestration_state_path: lane.orchestration_state_path,
                            checks: lane.checks,
                            run_dir: lane.run_dir,
                        },
                    )
                })
                .collect()
        };

        let artifacts = unit
            .artifacts
            .into_iter()
            .map(|artifact| (artifact.id, artifact.path))
            .collect::<BTreeMap<_, _>>();
        units.insert(
            unit.id,
            UnitManifest {
                title: unit.title,
                output_root: unit.output_root,
                artifacts,
                milestones: unit.milestones,
                lanes,
            },
        );
    }

    Ok(ProgramManifest {
        program: manifest.program,
        target_repo: manifest.target_repo,
        state_path: manifest.state_path,
        max_parallel: manifest.max_parallel,
        run_dir: manifest.run_dir,
        units,
    })
}

fn validate_unit(
    manifest: &ProgramManifest,
    manifest_path: &Path,
    unit_id: &str,
    unit: &UnitManifest,
) -> Result<(), ManifestError> {
    if unit.lanes.is_empty() {
        return Err(invalid_manifest(
            manifest_path,
            format!("unit `{unit_id}` must define at least one lane"),
        ));
    }

    let artifact_ids = unit.artifacts.keys().cloned().collect::<BTreeSet<_>>();
    let milestone_ids = unit
        .milestones
        .iter()
        .map(|milestone| milestone.id.clone())
        .collect::<BTreeSet<_>>();

    for milestone in &unit.milestones {
        if milestone.id.trim().is_empty() {
            return Err(invalid_manifest(
                manifest_path,
                format!("unit `{unit_id}` has a milestone with an empty id"),
            ));
        }
        for artifact_id in &milestone.requires {
            if !artifact_ids.contains(artifact_id) {
                return Err(invalid_manifest(
                    manifest_path,
                    format!(
                        "unit `{unit_id}` milestone `{}` references unknown artifact `{artifact_id}`",
                        milestone.id
                    ),
                ));
            }
        }
    }

    for (lane_id, lane) in &unit.lanes {
        if !milestone_ids.contains(&lane.managed_milestone) {
            return Err(invalid_manifest(
                manifest_path,
                format!(
                    "lane `{unit_id}:{lane_id}` references unknown managed milestone `{}`",
                    lane.managed_milestone
                ),
            ));
        }
        for produced in &lane.produces {
            if !artifact_ids.contains(produced) {
                return Err(invalid_manifest(
                    manifest_path,
                    format!(
                        "lane `{unit_id}:{lane_id}` references unknown produced artifact `{produced}`",
                    ),
                ));
            }
        }
        for check in &lane.checks {
            if check.label.trim().is_empty() {
                return Err(invalid_manifest(
                    manifest_path,
                    format!("lane `{unit_id}:{lane_id}` contains a check with an empty label"),
                ));
            }
        }
        for dependency in &lane.dependencies {
            let Some(dep_unit) = manifest.units.get(&dependency.unit) else {
                return Err(invalid_manifest(
                    manifest_path,
                    format!(
                        "lane `{unit_id}:{lane_id}` references unknown dependency unit `{}`",
                        dependency.unit
                    ),
                ));
            };
            if let Some(dep_lane) = dependency.lane.as_ref() {
                if !dep_unit.lanes.contains_key(dep_lane) {
                    return Err(invalid_manifest(
                        manifest_path,
                        format!(
                            "lane `{unit_id}:{lane_id}` references unknown dependency lane `{}:{dep_lane}`",
                            dependency.unit
                        ),
                    ));
                }
            }
            if let Some(dep_milestone) = dependency.milestone.as_ref() {
                if !dep_unit
                    .milestones
                    .iter()
                    .any(|milestone| milestone.id == *dep_milestone)
                {
                    return Err(invalid_manifest(
                        manifest_path,
                        format!(
                            "lane `{unit_id}:{lane_id}` references unknown dependency milestone `{}:{dep_milestone}`",
                            dependency.unit
                        ),
                    ));
                }
            }
        }
    }

    Ok(())
}

fn invalid_manifest(path: &Path, message: impl Into<String>) -> ManifestError {
    ManifestError::Invalid {
        path: path.to_path_buf(),
        message: message.into(),
    }
}

fn resolve_relative(manifest_path: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    let base = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    base.join(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_manifest_from_fixture() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../test/fixtures/raspberry-supervisor/program.yaml");
        let manifest = ProgramManifest::load(&path).expect("fixture manifest should load");
        assert_eq!(manifest.program, "raspberry-demo");
        assert_eq!(manifest.max_parallel, 2);
        assert!(manifest.units.contains_key("runtime"));
        assert!(manifest.units.contains_key("consensus"));
        assert!(manifest.units.contains_key("p2p"));
    }

    #[test]
    fn load_list_manifest_from_myosu_fixture() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml");
        let manifest = ProgramManifest::load(&path).expect("myosu-style fixture should load");
        assert_eq!(manifest.program, "myosu-bootstrap");
        assert!(manifest.units.contains_key("chain"));
        assert!(manifest.units.contains_key("validator"));
        assert!(manifest.units.contains_key("operations"));
        assert!(manifest
            .units
            .get("validator")
            .expect("validator unit")
            .lanes
            .contains_key("oracle"));
    }

    #[test]
    fn rejects_unknown_dependency_lane() {
        let manifest = ProgramManifest {
            program: "demo".to_string(),
            target_repo: PathBuf::from("."),
            state_path: PathBuf::from(".raspberry/program-state.json"),
            max_parallel: 1,
            run_dir: None,
            units: BTreeMap::from([(
                "runtime".to_string(),
                UnitManifest {
                    title: "Runtime".to_string(),
                    output_root: Some(PathBuf::from("out")),
                    artifacts: BTreeMap::from([("review".to_string(), PathBuf::from("review.md"))]),
                    milestones: vec![MilestoneManifest {
                        id: "reviewed".to_string(),
                        requires: vec!["review".to_string()],
                    }],
                    lanes: BTreeMap::from([(
                        "chapter".to_string(),
                        LaneManifest {
                            kind: LaneKind::Artifact,
                            title: "Chapter".to_string(),
                            run_config: PathBuf::from("chapter.toml"),
                            managed_milestone: "reviewed".to_string(),
                            dependencies: vec![LaneDependency {
                                unit: "runtime".to_string(),
                                lane: Some("missing".to_string()),
                                milestone: Some("reviewed".to_string()),
                            }],
                            produces: Vec::new(),
                            proof_profile: None,
                            proof_state_path: None,
                            service_state_path: None,
                            orchestration_state_path: None,
                            checks: Vec::new(),
                            run_dir: None,
                        },
                    )]),
                },
            )]),
        };

        let path = PathBuf::from("program.yaml");
        let error = manifest.validate(&path).expect_err("manifest should be invalid");
        assert!(error
            .to_string()
            .contains("references unknown dependency lane"));
    }
}
