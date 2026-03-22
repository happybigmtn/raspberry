use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

use crate::manifest::LaneKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRegistry {
    pub plans: Vec<PlanRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRecord {
    pub plan_id: String,
    pub path: PathBuf,
    pub title: String,
    pub category: PlanCategory,
    pub composite: bool,
    pub dependency_plan_ids: Vec<String>,
    pub mapping_contract_path: Option<PathBuf>,
    pub mapping_source: PlanMappingSource,
    pub bootstrap_required: bool,
    pub implementation_required: bool,
    pub declared_child_ids: Vec<String>,
    pub children: Vec<PlanChildRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanCategory {
    Meta,
    Foundation,
    Game,
    Interface,
    Service,
    Infrastructure,
    Verification,
    Economic,
    Unknown,
}

impl PlanCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Meta => "meta",
            Self::Foundation => "foundation",
            Self::Game => "game",
            Self::Interface => "interface",
            Self::Service => "service",
            Self::Infrastructure => "infrastructure",
            Self::Verification => "verification",
            Self::Economic => "economic",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanMappingSource {
    Inferred,
    Contract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkflowArchetype {
    Implement,
    Integration,
    Orchestration,
    Report,
}

impl WorkflowArchetype {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Implement => "implement",
            Self::Integration => "integration",
            Self::Orchestration => "orchestration",
            Self::Report => "report",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "implement"
            | "implement_module"
            | "module"
            | "bootstrap_contract"
            | "bootstrap"
            | "implement_cross_surface"
            | "cross_surface"
            | "service_surface"
            | "service"
            | "api_surface"
            | "api"
            | "client_surface"
            | "client"
            | "tui_surface"
            | "tui"
            | "verification_only"
            | "verification"
            | "verify"
            | "acceptance_testing"
            | "acceptance"
            | "acceptance_and_balance"
            | "migration"
            | "data_pipeline"
            | "web"
            | "mobile"
            | "frontend"
            | "daemon"
            | "worker"
            | "rest"
            | "grpc"
            | "graphql"
            | "pipeline"
            | "etl"
            | "fuzz"
            | "monte_carlo" => Some(Self::Implement),
            "integration_only" | "integration" | "e2e" | "end_to_end" => Some(Self::Integration),
            "orchestration_program" | "orchestration" => Some(Self::Orchestration),
            "review_or_report_only" | "report" | "review_only" | "recurring_report" => {
                Some(Self::Report)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewProfile {
    Standard,
    Foundation,
    Hardened,
    Ux,
}

impl ReviewProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Foundation => "foundation",
            Self::Hardened => "hardened",
            Self::Ux => "ux",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "standard" | "production_service" | "production" | "data_and_migration"
            | "migration_risky" | "data_integrity" => Some(Self::Standard),
            "foundation" | "shared_foundation" => Some(Self::Foundation),
            "hardened"
            | "security_sensitive"
            | "security"
            | "correctness_critical"
            | "correctness"
            | "economic_correctness"
            | "economic"
            | "financial"
            | "invariant" => Some(Self::Hardened),
            "ux" | "ux_facing" | "user_visible" | "visible" | "ui" => Some(Self::Ux),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanChildRecord {
    pub child_id: String,
    pub title: Option<String>,
    pub archetype: Option<WorkflowArchetype>,
    pub lane_kind: Option<LaneKind>,
    pub review_profile: Option<ReviewProfile>,
    pub proof_commands: Vec<String>,
    pub owned_surfaces: Vec<String>,
    pub where_surfaces: Option<String>,
    pub how_description: Option<String>,
    pub state_artifacts: Option<String>,
    pub required_tests: Option<String>,
    pub verification_plan: Option<String>,
    pub rollback_condition: Option<String>,
}

#[derive(Debug, Error)]
pub enum PlanRegistryError {
    #[error("failed to read plans directory {path}: {source}")]
    ReadPlans {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read plan {path}: {source}")]
    ReadPlan {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read mapping contract {path}: {source}")]
    ReadMappingContract {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse mapping contract {path}: {message}")]
    ParseMappingContract { path: PathBuf, message: String },
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PlanMappingContract {
    #[serde(default)]
    mapping_source: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    composite: Option<bool>,
    #[serde(default)]
    dependency_plan_ids: Vec<String>,
    #[serde(default)]
    bootstrap_required: Option<bool>,
    #[serde(default)]
    implementation_required: Option<bool>,
    #[serde(default)]
    child_ids: Vec<String>,
    #[serde(default)]
    children: Vec<PlanMappingChild>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PlanMappingChild {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    archetype: Option<String>,
    #[serde(default)]
    lane_kind: Option<String>,
    #[serde(default)]
    review_profile: Option<String>,
    #[serde(default)]
    proof_commands: Vec<String>,
    #[serde(default)]
    owned_surfaces: Vec<String>,
    #[serde(default, deserialize_with = "string_or_list")]
    where_surfaces: Option<String>,
    #[serde(default, deserialize_with = "string_or_list")]
    how_description: Option<String>,
    #[serde(default, deserialize_with = "string_or_list")]
    state_artifacts: Option<String>,
    #[serde(default, deserialize_with = "string_or_list")]
    required_tests: Option<String>,
    #[serde(default, deserialize_with = "string_or_list")]
    verification_plan: Option<String>,
    #[serde(default, deserialize_with = "string_or_list")]
    rollback_condition: Option<String>,
}

/// Accept either a string or a list of strings (joined with "; ").
/// LLMs sometimes emit YAML lists where the schema expects a scalar.
fn string_or_list<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct StringOrList;

    impl<'de> de::Visitor<'de> for StringOrList {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a string or list of strings")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(Some(v.to_owned()))
        }

        fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
            Ok(Some(v))
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut items = Vec::new();
            while let Some(item) = seq.next_element::<String>()? {
                items.push(item);
            }
            if items.is_empty() {
                Ok(None)
            } else {
                Ok(Some(items.join("; ")))
            }
        }
    }

    deserializer.deserialize_any(StringOrList)
}

pub fn load_plan_registry(target_repo: &Path) -> Result<PlanRegistry, PlanRegistryError> {
    load_plan_registry_from_planning_root(target_repo, Path::new(""))
}

pub fn load_plan_registry_from_planning_root(
    target_repo: &Path,
    planning_root: &Path,
) -> Result<PlanRegistry, PlanRegistryError> {
    let plans_dir = if planning_root.as_os_str().is_empty() {
        target_repo.join("plans")
    } else {
        target_repo.join(planning_root).join("plans")
    };
    if !plans_dir.is_dir() {
        return Ok(PlanRegistry { plans: Vec::new() });
    }

    let mut paths = std::fs::read_dir(&plans_dir)
        .map_err(|source| PlanRegistryError::ReadPlans {
            path: plans_dir.clone(),
            source,
        })?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| PlanRegistryError::ReadPlans {
            path: plans_dir.clone(),
            source,
        })?;
    paths.sort();

    let mut raw_plans = Vec::new();
    let mut id_by_path = BTreeMap::new();
    for absolute_path in paths {
        if absolute_path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let relative_path = absolute_path
            .strip_prefix(target_repo)
            .expect("plan file should live under target repo")
            .to_path_buf();
        if !is_numbered_plan_path(&relative_path) {
            continue;
        }
        let body = std::fs::read_to_string(&absolute_path).map_err(|source| {
            PlanRegistryError::ReadPlan {
                path: absolute_path.clone(),
                source,
            }
        })?;
        let title = markdown_title(&relative_path, &body);
        let plan_id = plan_id_from_path(&relative_path);
        id_by_path.insert(relative_path.clone(), plan_id.clone());
        raw_plans.push((relative_path, title, body, plan_id));
    }

    let plans = raw_plans
        .into_iter()
        .map(
            |(path, title, body, plan_id)| -> Result<PlanRecord, PlanRegistryError> {
                let detected_mapping_contract_path = mapping_contract_path(target_repo, &path);
                let mapping_contract = detected_mapping_contract_path
                    .as_ref()
                    .map(|contract_path| load_mapping_contract(target_repo, contract_path))
                    .transpose()?;
                let authoritative_contract = mapping_contract
                    .as_ref()
                    .filter(|contract| contract.mapping_source.as_deref() != Some("heuristic"));
                let mapping_contract_path = if authoritative_contract.is_some() {
                    detected_mapping_contract_path.clone()
                } else {
                    None
                };
                let mut title = title;
                if let Some(contract_title) =
                    authoritative_contract.and_then(|contract| contract.title.as_ref())
                {
                    title = contract_title.clone();
                }
                let inferred_category = categorize_plan(&path, &title, &body);
                let category = authoritative_contract
                    .and_then(|contract| parse_category(contract.category.as_deref()))
                    .unwrap_or(inferred_category);
                let mut declared_child_ids = declared_child_ids(&body);
                let contract_child_ids = authoritative_contract
                    .map(contract_child_ids)
                    .unwrap_or_default();
                if !contract_child_ids.is_empty() {
                    declared_child_ids = contract_child_ids;
                }
                let inferred_composite = category == PlanCategory::Meta
                    || declared_child_ids.len() > 1
                    || body
                        .lines()
                        .filter(|line| {
                            let trimmed = line.trim();
                            trimmed.starts_with("## ")
                                && !trimmed.eq_ignore_ascii_case("## progress")
                                && !trimmed.eq_ignore_ascii_case("## milestones")
                        })
                        .count()
                        > 3;
                let composite = authoritative_contract
                    .and_then(|contract| contract.composite)
                    .unwrap_or(inferred_composite);
                let mut dependency_plan_ids =
                    dependency_plan_ids(&body, &id_by_path, planning_root);
                if let Some(contract_dependencies) = authoritative_contract
                    .map(|contract| normalized_dependency_ids(&contract.dependency_plan_ids))
                    .filter(|dependencies| !dependencies.is_empty())
                {
                    dependency_plan_ids = contract_dependencies;
                }
                let inferred_executable = category != PlanCategory::Meta;
                let bootstrap_required = authoritative_contract
                    .and_then(|contract| contract.bootstrap_required)
                    .unwrap_or(inferred_executable);
                let implementation_required = authoritative_contract
                    .and_then(|contract| contract.implementation_required)
                    .unwrap_or(inferred_executable);
                let _executable = bootstrap_required || implementation_required;
                let children = authoritative_contract
                    .map(contract_child_records)
                    .unwrap_or_default();

                Ok(PlanRecord {
                    plan_id,
                    path,
                    title,
                    category,
                    composite,
                    dependency_plan_ids,
                    mapping_source: if mapping_contract_path.is_some() {
                        PlanMappingSource::Contract
                    } else {
                        PlanMappingSource::Inferred
                    },
                    mapping_contract_path,
                    bootstrap_required,
                    implementation_required,
                    declared_child_ids,
                    children,
                })
            },
        )
        .collect::<Result<Vec<_>, PlanRegistryError>>()?;

    Ok(PlanRegistry { plans })
}

pub fn plan_id_from_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut parts = stem.splitn(2, '-');
    let first = parts.next().unwrap_or_default();
    let remainder = if first.len() == 3 && first.chars().all(|ch| ch.is_ascii_digit()) {
        parts.next().unwrap_or_default()
    } else {
        stem.as_str()
    };
    let simplified = remainder
        .trim_end_matches("-game")
        .trim_end_matches("-plan")
        .trim_end_matches("-crate")
        .trim_end_matches("-trait");
    sanitize_identifier(simplified)
}

fn is_numbered_plan_path(path: &Path) -> bool {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let prefix = stem.split('-').next().unwrap_or_default();
    prefix.len() == 3 && prefix.chars().all(|ch| ch.is_ascii_digit())
}

fn markdown_title(path: &Path, body: &str) -> String {
    body.lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("# ")
                .map(str::trim)
                .filter(|title| !title.is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| {
            humanize_slug(
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("plan"),
            )
        })
}

fn humanize_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = String::new();
                    word.push(first.to_ascii_uppercase());
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn categorize_plan(path: &Path, title: &str, body: &str) -> PlanCategory {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let title_lower = title.to_ascii_lowercase();
    let text = format!("{stem}\n{}\n{}", title_lower, body.to_ascii_lowercase());

    if stem.contains("master-plan") || title_lower.contains("master plan") {
        return PlanCategory::Meta;
    }
    if text.contains("provably fair")
        || text.contains("verification")
        || text.contains("proof")
        || text.contains("audit")
    {
        return PlanCategory::Verification;
    }
    if text.contains("wallet rpc")
        || text.contains("infrastructure")
        || text.contains("deployment")
        || text.contains("node")
    {
        return PlanCategory::Infrastructure;
    }
    if text.contains("tui")
        || text.contains("terminal")
        || text.contains("client")
        || text.contains("shell")
    {
        return PlanCategory::Interface;
    }
    if text.contains("agent")
        || text.contains("server")
        || text.contains("service")
        || text.contains("websocket")
    {
        return PlanCategory::Service;
    }
    if text.contains("balance")
        || text.contains("payout")
        || text.contains("settlement")
        || text.contains("escrow")
        || text.contains("accounting")
    {
        return PlanCategory::Economic;
    }
    if stem.contains("game") || text.contains(" card game") || text.contains("casino") {
        return PlanCategory::Game;
    }
    if text.contains("foundation") || text.contains("workspace") || text.contains("core") {
        return PlanCategory::Foundation;
    }

    PlanCategory::Unknown
}

fn mapping_contract_path(target_repo: &Path, plan_path: &Path) -> Option<PathBuf> {
    let stem = plan_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    [
        format!("malinka/plan-mappings/{stem}.yaml"),
        format!("malinka/plan-mappings/{stem}.yml"),
        format!("malinka/plan-mappings/{stem}.json"),
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|candidate| target_repo.join(candidate).is_file())
}

fn load_mapping_contract(
    target_repo: &Path,
    contract_path: &Path,
) -> Result<PlanMappingContract, PlanRegistryError> {
    let absolute_path = target_repo.join(contract_path);
    let raw = std::fs::read_to_string(&absolute_path).map_err(|source| {
        PlanRegistryError::ReadMappingContract {
            path: absolute_path.clone(),
            source,
        }
    })?;
    if contract_path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        return serde_json::from_str(&raw).map_err(|source| {
            PlanRegistryError::ParseMappingContract {
                path: absolute_path,
                message: source.to_string(),
            }
        });
    }
    serde_yaml::from_str(&raw).map_err(|source| PlanRegistryError::ParseMappingContract {
        path: absolute_path,
        message: source.to_string(),
    })
}

fn contract_child_ids(contract: &PlanMappingContract) -> Vec<String> {
    let mut child_ids = contract
        .child_ids
        .iter()
        .map(|child_id| sanitize_identifier(child_id))
        .filter(|child_id| !child_id.is_empty())
        .collect::<Vec<_>>();
    child_ids.extend(contract.children.iter().filter_map(|child| {
        child
            .id
            .as_ref()
            .or(child.title.as_ref())
            .map(|value| sanitize_identifier(value))
            .filter(|child_id| !child_id.is_empty())
    }));
    child_ids.sort();
    child_ids.dedup();
    child_ids
}

fn contract_child_records(contract: &PlanMappingContract) -> Vec<PlanChildRecord> {
    contract
        .children
        .iter()
        .filter_map(|child| {
            let child_id = child
                .id
                .as_ref()
                .or(child.title.as_ref())
                .map(|value| sanitize_identifier(value))
                .filter(|id| !id.is_empty())?;
            Some(PlanChildRecord {
                child_id,
                title: child.title.clone(),
                archetype: child
                    .archetype
                    .as_deref()
                    .and_then(WorkflowArchetype::from_str),
                lane_kind: child.lane_kind.as_deref().and_then(parse_lane_kind),
                review_profile: child
                    .review_profile
                    .as_deref()
                    .and_then(ReviewProfile::from_str),
                proof_commands: child.proof_commands.clone(),
                owned_surfaces: child.owned_surfaces.clone(),
                where_surfaces: child.where_surfaces.clone(),
                how_description: child.how_description.clone(),
                state_artifacts: child.state_artifacts.clone(),
                required_tests: child.required_tests.clone(),
                verification_plan: child.verification_plan.clone(),
                rollback_condition: child.rollback_condition.clone(),
            })
        })
        .collect()
}

fn normalized_dependency_ids(values: &[String]) -> Vec<String> {
    let mut dependency_ids = values
        .iter()
        .map(|value| sanitize_identifier(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    dependency_ids.sort();
    dependency_ids.dedup();
    dependency_ids
}

fn parse_category(value: Option<&str>) -> Option<PlanCategory> {
    let value = value?.trim().to_ascii_lowercase();
    match value.as_str() {
        "meta" => Some(PlanCategory::Meta),
        "foundation" => Some(PlanCategory::Foundation),
        "game" => Some(PlanCategory::Game),
        "interface" => Some(PlanCategory::Interface),
        "service" => Some(PlanCategory::Service),
        "infrastructure" => Some(PlanCategory::Infrastructure),
        "verification" => Some(PlanCategory::Verification),
        "economic" => Some(PlanCategory::Economic),
        "unknown" => Some(PlanCategory::Unknown),
        _ => None,
    }
}

fn parse_lane_kind(value: &str) -> Option<LaneKind> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "artifact" | "report" => Some(LaneKind::Artifact),
        "service" | "daemon" | "server" | "worker" => Some(LaneKind::Service),
        "orchestration" => Some(LaneKind::Orchestration),
        "interface" | "ui" | "ux" | "client" | "cli" | "tui" | "web" | "mobile" => {
            Some(LaneKind::Interface)
        }
        "platform" | "foundation" | "module" | "library" | "core" => Some(LaneKind::Platform),
        "recurring" => Some(LaneKind::Recurring),
        "integration" | "e2e" => Some(LaneKind::Integration),
        _ => None,
    }
}

fn dependency_plan_ids(
    body: &str,
    id_by_path: &BTreeMap<PathBuf, String>,
    planning_root: &Path,
) -> Vec<String> {
    let mut plan_ids = explicit_plan_dependency_paths(body, planning_root)
        .into_iter()
        .filter_map(|path| id_by_path.get(&path).cloned())
        .collect::<Vec<_>>();
    plan_ids.sort();
    plan_ids.dedup();
    plan_ids
}

fn explicit_plan_dependency_paths(body: &str, planning_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for paragraph in planning_markdown_paragraphs(body) {
        let lower = paragraph.to_ascii_lowercase();
        let Some(index) = lower.find("depends on:") else {
            continue;
        };
        let mut segment = paragraph[index + "depends on:".len()..].to_string();
        if let Some(depended_on_by_index) = segment.to_ascii_lowercase().find("depended on by:") {
            segment.truncate(depended_on_by_index);
        }
        if let Some(next_sentence_index) = segment.find(". The ") {
            segment.truncate(next_sentence_index);
        }
        if let Some(next_sentence_index) = segment.find(". This ") {
            segment.truncate(next_sentence_index);
        }
        paths.extend(markdown_path_references(&segment, "plans", planning_root));
    }
    paths.sort();
    paths.dedup();
    paths
}

fn declared_child_ids(body: &str) -> Vec<String> {
    let mut children = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed
            .strip_prefix("- [ ] ")
            .or_else(|| trimmed.strip_prefix("- [x] "))
            .or_else(|| trimmed.strip_prefix("## "))
            .or_else(|| trimmed.strip_prefix("### "))
        else {
            continue;
        };
        let lower = rest.to_ascii_lowercase();
        if !(lower.starts_with("milestone") || lower.contains("milestone")) {
            continue;
        }
        let detail = rest
            .split_once(':')
            .map(|(_, detail)| detail.trim())
            .unwrap_or(rest);
        let child_id = sanitize_identifier(detail);
        if !child_id.is_empty() {
            children.push(child_id);
        }
    }
    children.sort();
    children.dedup();
    children
}

fn markdown_path_references(body: &str, root: &str, planning_root: &Path) -> Vec<PathBuf> {
    body.split('`')
        .filter_map(|chunk| resolve_markdown_reference(chunk.trim(), root, planning_root))
        .collect()
}

fn resolve_markdown_reference(trimmed: &str, root: &str, planning_root: &Path) -> Option<PathBuf> {
    if !trimmed.ends_with(".md") {
        return None;
    }
    if trimmed.starts_with(&format!("{root}/")) {
        return Some(if planning_root.as_os_str().is_empty() {
            PathBuf::from(trimmed)
        } else {
            planning_root.join(trimmed)
        });
    }
    if !planning_root.as_os_str().is_empty() {
        let prefixed = format!("{}/", planning_root.display());
        if trimmed.starts_with(&prefixed) {
            return Some(PathBuf::from(trimmed));
        }
    }
    None
}

fn planning_markdown_paragraphs(body: &str) -> Vec<String> {
    body.split("\n\n")
        .map(|chunk| {
            chunk
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|paragraph| !paragraph.is_empty())
        .collect()
}

fn sanitize_identifier(input: &str) -> String {
    let mut result = String::new();
    let mut previous_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            result.push('-');
            previous_dash = true;
        }
    }
    result.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn load_plan_registry_reads_numbered_plans_and_dependencies() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        fs::write(
            temp.path().join("plans/001-master-plan.md"),
            "# Master Plan\n",
        )
        .expect("master plan");
        fs::write(
            temp.path().join("plans/002-provably-fair-crate.md"),
            "# Provably Fair Crate\n",
        )
        .expect("provably fair");
        fs::write(
            temp.path().join("plans/013-house-agent.md"),
            "# House Agent\n\nThis plan depends on: `plans/002-provably-fair-crate.md`, `plans/016-casino-core-trait.md`.\n",
        )
        .expect("house");
        fs::write(
            temp.path().join("plans/016-casino-core-trait.md"),
            "# Casino Core Trait\n",
        )
        .expect("casino core");

        let registry = load_plan_registry(temp.path()).expect("registry");
        let house = registry
            .plans
            .iter()
            .find(|plan| plan.plan_id == "house-agent")
            .expect("house record");

        assert_eq!(registry.plans.len(), 4);
        assert_eq!(house.category, PlanCategory::Service);
        assert_eq!(
            house.dependency_plan_ids,
            vec!["casino-core".to_string(), "provably-fair".to_string()]
        );
    }

    #[test]
    fn load_plan_registry_marks_composite_plan_without_contract_as_needing_review() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        fs::write(
            temp.path().join("plans/005-craps-game.md"),
            concat!(
                "# Craps Game\n\n",
                "- [ ] Milestone 1: casino-core\n",
                "- [ ] Milestone 2: provably-fair\n",
                "- [ ] Milestone 3: house\n",
                "- [ ] Milestone 4: tui\n",
            ),
        )
        .expect("craps plan");

        let registry = load_plan_registry(temp.path()).expect("registry");
        let craps = registry.plans.first().expect("craps record");

        assert!(craps.composite);
        assert_eq!(
            craps.declared_child_ids,
            vec![
                "casino-core".to_string(),
                "house".to_string(),
                "provably-fair".to_string(),
                "tui".to_string()
            ]
        );
    }

    #[test]
    fn load_plan_registry_uses_mapping_contract_when_present() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        fs::create_dir_all(temp.path().join("malinka/plan-mappings")).expect("mapping dir");
        fs::write(
            temp.path().join("plans/005-craps-game.md"),
            concat!(
                "# Craps Game\n\n",
                "- [ ] Milestone 1: casino-core\n",
                "- [ ] Milestone 2: provably-fair\n",
            ),
        )
        .expect("craps plan");
        fs::write(
            temp.path()
                .join("malinka/plan-mappings/005-craps-game.yaml"),
            "children: []\n",
        )
        .expect("mapping contract");

        let registry = load_plan_registry(temp.path()).expect("registry");
        let craps = registry.plans.first().expect("craps record");

        assert_eq!(craps.mapping_source, PlanMappingSource::Contract);
        assert_eq!(
            craps.mapping_contract_path,
            Some(PathBuf::from("malinka/plan-mappings/005-craps-game.yaml"))
        );
    }

    #[test]
    fn load_plan_registry_applies_mapping_contract_overrides() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        fs::create_dir_all(temp.path().join("malinka/plan-mappings")).expect("mapping dir");
        fs::write(
            temp.path().join("plans/013-house-agent.md"),
            "# House Agent\n\nThis plan depends on: `plans/015-monero-infrastructure.md`.\n",
        )
        .expect("house plan");
        fs::write(
            temp.path().join("plans/015-monero-infrastructure.md"),
            "# Monero Infrastructure\n",
        )
        .expect("infra plan");
        fs::write(
            temp.path()
                .join("malinka/plan-mappings/013-house-agent.yaml"),
            concat!(
                "title: House Agent Contract\n",
                "category: service\n",
                "composite: false\n",
                "bootstrap_required: true\n",
                "implementation_required: false\n",
                "dependency_plan_ids:\n",
                "  - monero-infrastructure\n",
                "children:\n",
                "  - id: websocket-server\n",
                "  - title: wallet escrow\n",
            ),
        )
        .expect("mapping contract");

        let registry = load_plan_registry(temp.path()).expect("registry");
        let house = registry
            .plans
            .iter()
            .find(|plan| plan.plan_id == "house-agent")
            .expect("house record");

        assert_eq!(house.title, "House Agent Contract");
        assert_eq!(house.category, PlanCategory::Service);
        assert!(!house.composite);
        assert!(house.bootstrap_required);
        assert!(!house.implementation_required);
        assert_eq!(
            house.dependency_plan_ids,
            vec!["monero-infrastructure".to_string()]
        );
        assert_eq!(
            house.declared_child_ids,
            vec!["wallet-escrow".to_string(), "websocket-server".to_string()]
        );
    }

    #[test]
    fn load_plan_registry_parses_enriched_child_records() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        fs::create_dir_all(temp.path().join("malinka/plan-mappings")).expect("mapping dir");
        fs::write(
            temp.path().join("plans/005-craps-game.md"),
            "# Craps Game\n\n- [ ] Milestone 1: casino-core\n- [ ] Milestone 2: provably-fair\n",
        )
        .expect("craps plan");
        fs::write(
            temp.path().join("malinka/plan-mappings/005-craps-game.yaml"),
            concat!(
                "composite: true\n",
                "children:\n",
                "  - id: craps-casino-core\n",
                "    title: Craps Game Engine\n",
                "    archetype: implement_module\n",
                "    lane_kind: platform\n",
                "    review_profile: economic_correctness\n",
                "    proof_commands:\n",
                "      - \"cargo test -p casino-core --features craps\"\n",
                "    owned_surfaces:\n",
                "      - \"crates/casino-core/src/craps/\"\n",
                "    where_surfaces: \"crates/casino-core/src/craps/ and crates/casino-core/src/lib.rs\"\n",
                "    how_description: \"Add craps game engine with 30 bet types\"\n",
                "  - id: craps-provably-fair\n",
                "    archetype: verification\n",
                "    proof_commands:\n",
                "      - \"cargo test -p provably-fair\"\n",
                "    owned_surfaces:\n",
                "      - \"crates/provably-fair/src/dice.rs\"\n",
            ),
        )
        .expect("mapping contract");

        let registry = load_plan_registry(temp.path()).expect("registry");
        let craps = registry.plans.first().expect("craps record");

        assert!(craps.composite);
        assert_eq!(craps.children.len(), 2);

        // #given enriched child with archetype and review profile
        // #then parsed correctly
        let casino_core = &craps.children[0];
        assert_eq!(casino_core.child_id, "craps-casino-core");
        assert_eq!(casino_core.title.as_deref(), Some("Craps Game Engine"));
        assert_eq!(casino_core.archetype, Some(WorkflowArchetype::Implement));
        assert_eq!(casino_core.lane_kind, Some(LaneKind::Platform));
        assert_eq!(casino_core.review_profile, Some(ReviewProfile::Hardened));
        assert_eq!(
            casino_core.proof_commands,
            vec!["cargo test -p casino-core --features craps"]
        );
        assert_eq!(
            casino_core.owned_surfaces,
            vec!["crates/casino-core/src/craps/"]
        );
        assert_eq!(
            casino_core.where_surfaces.as_deref(),
            Some("crates/casino-core/src/craps/ and crates/casino-core/src/lib.rs")
        );
        assert_eq!(
            casino_core.how_description.as_deref(),
            Some("Add craps game engine with 30 bet types")
        );

        // #given child with shorthand archetype name
        // #then normalized to canonical variant
        let provably_fair = &craps.children[1];
        assert_eq!(provably_fair.child_id, "craps-provably-fair");
        assert_eq!(provably_fair.archetype, Some(WorkflowArchetype::Implement));
        assert_eq!(
            provably_fair.proof_commands,
            vec!["cargo test -p provably-fair"]
        );
    }

    #[test]
    fn load_plan_registry_from_planning_root_resolves_nested_plan_refs() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("genesis/plans")).expect("plans dir");
        fs::write(
            temp.path().join("genesis/plans/001-master-plan.md"),
            "# Master Plan\n",
        )
        .expect("master plan");
        fs::write(
            temp.path().join("genesis/plans/002-provably-fair-crate.md"),
            "# Provably Fair Crate\n",
        )
        .expect("provably fair");
        fs::write(
            temp.path().join("genesis/plans/013-house-agent.md"),
            "# House Agent\n\nThis plan depends on: `plans/002-provably-fair-crate.md`.\n",
        )
        .expect("house");

        let registry = load_plan_registry_from_planning_root(temp.path(), Path::new("genesis"))
            .expect("registry");
        let house = registry
            .plans
            .iter()
            .find(|plan| plan.plan_id == "house-agent")
            .expect("house record");

        assert_eq!(
            house.path,
            PathBuf::from("genesis/plans/013-house-agent.md")
        );
        assert_eq!(house.dependency_plan_ids, vec!["provably-fair".to_string()]);
    }
}
