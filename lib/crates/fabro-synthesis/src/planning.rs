use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use raspberry_supervisor::manifest::{LaneDependency, LaneKind, MilestoneManifest};

use crate::blueprint::{
    BlueprintArtifact, BlueprintInputs, BlueprintLane, BlueprintPackage, BlueprintProgram,
    BlueprintUnit, ProgramBlueprint, WorkflowTemplate,
};
use crate::error::PlanningError;

const ROOT_DOCTRINE_FILES: &[&str] = &[
    "README.md",
    "SPEC.md",
    "SPECS.md",
    "PLANS.md",
    "DESIGN.md",
    "AGENTS.md",
    "CLAUDE.md",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredBlueprint {
    pub blueprint: ProgramBlueprint,
    pub notes: Vec<String>,
    pub active_plan: Option<PathBuf>,
    pub active_spec: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct PlanningCorpus {
    repo_name: String,
    doctrine_files: Vec<PathBuf>,
    evidence_paths: Vec<PathBuf>,
    plan_docs: Vec<PlanningDocument>,
    active_plan: Option<PlanningDocument>,
    active_spec: Option<PlanningDocument>,
}

#[derive(Debug, Clone)]
struct PlanningDocument {
    path: PathBuf,
    title: String,
    body: String,
}

#[derive(Debug, Clone)]
struct LaneIntent {
    id: String,
    title: String,
    output_root: PathBuf,
    family: WorkflowTemplate,
    kind: LaneKind,
    goal: String,
    prompt_context: Option<String>,
    dependencies: Vec<LaneDependency>,
    produces: Vec<String>,
    health_command: Option<String>,
    verify_command: Option<String>,
    milestones: Vec<MilestoneManifest>,
    artifacts: Vec<BlueprintArtifact>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TaskCategory {
    Foundations,
    ControlPlane,
    Service,
    Client,
    Agent,
    Proof,
}

#[derive(Debug, Clone)]
struct PlanTask {
    text: String,
    category: TaskCategory,
}

#[derive(Debug, Clone)]
struct ExplicitUnitSpec {
    id: String,
    title: String,
    description: String,
}

pub fn author_blueprint_for_create(
    target_repo: &Path,
    program_override: Option<&str>,
) -> Result<AuthoredBlueprint, PlanningError> {
    let corpus = load_planning_corpus(target_repo)?;
    let program_id = program_override
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| sanitize_identifier(&corpus.repo_name));
    let intents = derive_lane_intents(target_repo, &corpus, &program_id);
    let notes = build_authoring_notes(&corpus, &intents);
    let blueprint = ProgramBlueprint {
        version: 1,
        program: BlueprintProgram {
            id: program_id.clone(),
            max_parallel: derive_max_parallel(&intents),
            state_path: Some(PathBuf::from(format!(".raspberry/{program_id}-state.json"))),
            run_dir: None,
        },
        inputs: BlueprintInputs {
            doctrine_files: corpus.doctrine_files.clone(),
            evidence_paths: corpus.evidence_paths.clone(),
        },
        package: BlueprintPackage::default(),
        units: intents.iter().map(materialize_unit).collect(),
    };

    Ok(AuthoredBlueprint {
        blueprint,
        notes,
        active_plan: corpus.active_plan.map(|doc| doc.path),
        active_spec: corpus.active_spec.map(|doc| doc.path),
    })
}

pub fn author_blueprint_for_evolve(
    target_repo: &Path,
    program_override: Option<&str>,
) -> Result<AuthoredBlueprint, PlanningError> {
    let corpus = match load_planning_corpus(target_repo) {
        Ok(corpus) => Some(corpus),
        Err(PlanningError::MissingPlanningCorpus { .. }) => None,
        Err(error) => return Err(error),
    };
    let program_id = resolve_existing_program_id(target_repo, program_override)?;
    let mut notes = Vec::new();
    let mut blueprint = crate::blueprint::import_existing_package(crate::render::ImportRequest {
        target_repo,
        program: &program_id,
    })
    .map_err(PlanningError::Blueprint)?;
    if let Some(corpus) = &corpus {
        blueprint.inputs = BlueprintInputs {
            doctrine_files: corpus.doctrine_files.clone(),
            evidence_paths: corpus.evidence_paths.clone(),
        };
        if accepts_doctrine_frontier_merge(&blueprint) {
            let authored = author_blueprint_for_create(target_repo, Some(&program_id))?;
            let added_units = merge_missing_doctrine_units(&mut blueprint, &authored.blueprint);
            for unit_id in added_units {
                notes.push(format!(
                    "added doctrine-derived frontier unit `{unit_id}` to parent program"
                ));
            }
        }
    }

    if let Some(corpus) = &corpus {
        if let Some(plan) = &corpus.active_plan {
            notes.push(format!("active plan: {}", plan.path.display()));
        }
        if let Some(spec) = &corpus.active_spec {
            notes.push(format!("active spec: {}", spec.path.display()));
        }
    }
    if corpus.is_some() {
        notes.push(format!(
            "imported existing package for `{program_id}` and attached repo doctrine/evidence inputs"
        ));
    } else {
        notes.push(format!(
            "imported existing package for `{program_id}` without additional planning inputs"
        ));
    }

    Ok(AuthoredBlueprint {
        blueprint,
        notes,
        active_plan: corpus
            .as_ref()
            .and_then(|value| value.active_plan.clone().map(|doc| doc.path)),
        active_spec: corpus
            .as_ref()
            .and_then(|value| value.active_spec.clone().map(|doc| doc.path)),
    })
}

fn accepts_doctrine_frontier_merge(blueprint: &ProgramBlueprint) -> bool {
    blueprint
        .units
        .iter()
        .flat_map(|unit| unit.lanes.iter())
        .any(|lane| {
            !matches!(
                lane.template,
                WorkflowTemplate::Implementation | WorkflowTemplate::Integration
            )
        })
}

fn merge_missing_doctrine_units(
    blueprint: &mut ProgramBlueprint,
    authored: &ProgramBlueprint,
) -> Vec<String> {
    let mut added_units = Vec::new();
    for unit in &authored.units {
        if blueprint
            .units
            .iter()
            .any(|existing| existing.id == unit.id)
        {
            continue;
        }
        blueprint.units.push(unit.clone());
        added_units.push(unit.id.clone());
    }
    added_units
}

fn load_planning_corpus(target_repo: &Path) -> Result<PlanningCorpus, PlanningError> {
    let repo_name = target_repo
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo")
        .to_string();
    let doctrine_files = ROOT_DOCTRINE_FILES
        .iter()
        .map(PathBuf::from)
        .filter(|path| target_repo.join(path).is_file())
        .collect::<Vec<_>>();
    let plan_docs = load_markdown_dir(target_repo, "plans")?;
    let spec_docs = load_markdown_dir(target_repo, "specs")?;
    let root_spec = load_optional_document(target_repo, Path::new("SPEC.md"))?;
    let root_specs = load_optional_document(target_repo, Path::new("SPECS.md"))?;

    let active_plan = select_primary_plan(&plan_docs);
    let active_spec = spec_docs
        .last()
        .cloned()
        .or(root_spec.clone())
        .or(root_specs.clone());

    let mut evidence_paths = spec_docs
        .iter()
        .map(|doc| doc.path.clone())
        .collect::<Vec<_>>();
    evidence_paths.extend(plan_docs.iter().map(|doc| doc.path.clone()));
    if evidence_paths.is_empty() {
        if let Some(spec) = &root_spec {
            evidence_paths.push(spec.path.clone());
        } else if let Some(specs) = &root_specs {
            evidence_paths.push(specs.path.clone());
        }
    }

    if doctrine_files.is_empty() && evidence_paths.is_empty() {
        return Err(PlanningError::MissingPlanningCorpus {
            target_repo: target_repo.to_path_buf(),
        });
    }

    Ok(PlanningCorpus {
        repo_name,
        doctrine_files,
        evidence_paths,
        plan_docs,
        active_plan,
        active_spec,
    })
}

fn select_primary_plan(plan_docs: &[PlanningDocument]) -> Option<PlanningDocument> {
    if plan_docs.is_empty() {
        return None;
    }

    let reference_counts = plan_docs
        .iter()
        .flat_map(|doc| markdown_path_references(&doc.body, "plans"))
        .fold(BTreeMap::<PathBuf, usize>::new(), |mut counts, path| {
            *counts.entry(path).or_insert(0) += 1;
            counts
        });

    let mut best: Option<(&PlanningDocument, i64, usize)> = None;
    for (index, doc) in plan_docs.iter().enumerate() {
        let stem = doc
            .path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let title = doc.title.to_ascii_lowercase();
        let referenced = reference_counts.get(&doc.path).copied().unwrap_or(0) as i64;
        let mut score = referenced * 100;
        if stem.contains("master-plan") || title.contains("master plan") {
            score += 2_000;
        }
        if stem.starts_with("001-") {
            score += 300;
        }
        if stem.contains("mvp") || title.contains("mvp") {
            score += 150;
        }
        if title.contains("workspace") {
            score += 25;
        }
        let rank = (score, usize::MAX - index);
        if best
            .as_ref()
            .map(|(_, best_score, best_index)| rank > (*best_score, *best_index))
            .unwrap_or(true)
        {
            best = Some((doc, score, usize::MAX - index));
        }
    }

    best.map(|(doc, _, _)| doc.clone())
}

fn markdown_path_references(body: &str, root: &str) -> Vec<PathBuf> {
    body.split('`')
        .filter_map(|chunk| {
            let trimmed = chunk.trim();
            if !trimmed.starts_with(&format!("{root}/")) || !trimmed.ends_with(".md") {
                return None;
            }
            Some(PathBuf::from(trimmed))
        })
        .collect()
}

fn load_markdown_dir(
    target_repo: &Path,
    directory: &str,
) -> Result<Vec<PlanningDocument>, PlanningError> {
    let path = target_repo.join(directory);
    if !path.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries = fs::read_dir(&path)
        .map_err(|source| PlanningError::Read {
            path: path.clone(),
            source,
        })?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| PlanningError::Read {
            path: path.clone(),
            source,
        })?;
    entries.sort();

    let mut documents = Vec::new();
    for entry in entries {
        if entry.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let relative = entry
            .strip_prefix(target_repo)
            .expect("entry should be inside target repo");
        documents.push(load_document(target_repo, relative)?);
    }
    Ok(documents)
}

fn load_optional_document(
    target_repo: &Path,
    relative: &Path,
) -> Result<Option<PlanningDocument>, PlanningError> {
    let absolute = target_repo.join(relative);
    if !absolute.is_file() {
        return Ok(None);
    }
    load_document(target_repo, relative).map(Some)
}

fn load_document(target_repo: &Path, relative: &Path) -> Result<PlanningDocument, PlanningError> {
    let absolute = target_repo.join(relative);
    let body = fs::read_to_string(&absolute).map_err(|source| PlanningError::Read {
        path: absolute.clone(),
        source,
    })?;
    Ok(PlanningDocument {
        path: relative.to_path_buf(),
        title: markdown_title(relative, &body),
        body,
    })
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
                    .unwrap_or("document"),
            )
        })
}

fn derive_primary_intent(
    target_repo: &Path,
    corpus: &PlanningCorpus,
    program_id: &str,
) -> LaneIntent {
    let plan_title = corpus
        .active_plan
        .as_ref()
        .map(|doc| doc.title.clone())
        .or_else(|| corpus.active_spec.as_ref().map(|doc| doc.title.clone()))
        .unwrap_or_else(|| humanize_slug(&corpus.repo_name));
    let unchecked_tasks = corpus
        .active_plan
        .as_ref()
        .map(|doc| open_tasks(&doc.body))
        .unwrap_or_default();
    let family = select_family(target_repo, corpus);
    let unit_id = derive_unit_id(&plan_title, program_id);
    let output_root = PathBuf::from("outputs").join(&unit_id);
    let (kind, artifacts, milestones, produces, health_command, verify_command) =
        family_contract(&family, &output_root, target_repo);
    let goal = build_goal(
        &plan_title,
        &family,
        &output_root,
        corpus,
        &unchecked_tasks,
        &artifacts,
    );
    let prompt_context = build_prompt_context(corpus, &unchecked_tasks, &artifacts);

    LaneIntent {
        id: unit_id.clone(),
        title: humanize_slug(&unit_id),
        output_root,
        family,
        kind,
        goal,
        prompt_context,
        dependencies: Vec::new(),
        produces,
        health_command,
        verify_command,
        milestones,
        artifacts,
    }
}

fn derive_lane_intents(
    target_repo: &Path,
    corpus: &PlanningCorpus,
    program_id: &str,
) -> Vec<LaneIntent> {
    let explicit_units = corpus
        .active_spec
        .as_ref()
        .map(|spec| parse_explicit_units_from_spec(&spec.body))
        .unwrap_or_default();
    if !explicit_units.is_empty() {
        return derive_explicit_unit_intents(target_repo, corpus, &explicit_units);
    }
    if let Some(intents) = derive_master_plan_intents(target_repo, corpus) {
        return intents;
    }

    let tasks = corpus
        .active_plan
        .as_ref()
        .map(|doc| open_tasks(&doc.body))
        .unwrap_or_default();
    if tasks.is_empty() {
        return vec![derive_primary_intent(target_repo, corpus, program_id)];
    }

    let categorized = tasks
        .into_iter()
        .map(|text| PlanTask {
            category: categorize_task(&text),
            text,
        })
        .collect::<Vec<_>>();
    let categories = ordered_present_categories(&categorized);
    if categories.len() <= 1 {
        return vec![derive_primary_intent(target_repo, corpus, program_id)];
    }

    let lane_keys = categories
        .iter()
        .map(|category| {
            (
                *category,
                lane_identity_for_category(*category, &categorized),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut intents = Vec::new();

    for category in categories {
        let identity = lane_keys
            .get(&category)
            .expect("lane identity should exist for every category");
        let category_tasks = categorized
            .iter()
            .filter(|task| task.category == category)
            .map(|task| task.text.clone())
            .collect::<Vec<_>>();
        let family = select_family_for_category(target_repo, corpus, category, &category_tasks);
        let output_root = PathBuf::from("outputs").join(&identity.id);
        let (kind, artifacts, milestones, produces, health_command, verify_command) =
            category_contract(category, family, &output_root, target_repo);
        let dependencies = category_dependencies(category)
            .into_iter()
            .filter_map(|dependency| lane_keys.get(&dependency))
            .map(|identity| LaneDependency {
                unit: identity.id.clone(),
                lane: None,
                milestone: Some("reviewed".to_string()),
            })
            .collect::<Vec<_>>();
        let goal = build_goal(
            &identity.title,
            &family,
            &output_root,
            corpus,
            &category_tasks,
            &artifacts,
        );
        let prompt_context = build_prompt_context(corpus, &category_tasks, &artifacts);

        intents.push(LaneIntent {
            id: identity.id.clone(),
            title: identity.title.clone(),
            output_root,
            family,
            kind,
            goal,
            prompt_context,
            dependencies,
            produces,
            health_command,
            verify_command,
            milestones,
            artifacts,
        });
    }

    intents
}

fn derive_master_plan_intents(
    target_repo: &Path,
    corpus: &PlanningCorpus,
) -> Option<Vec<LaneIntent>> {
    let active_plan = corpus.active_plan.as_ref()?;
    let referenced_plans = referenced_bootstrap_plan_docs(corpus, active_plan);
    if referenced_plans.len() < 2 {
        return None;
    }

    let selected = referenced_plans
        .iter()
        .map(|doc| {
            let id = plan_unit_id(doc);
            (doc.path.clone(), id)
        })
        .collect::<BTreeMap<_, _>>();
    let mut intents = Vec::new();

    for doc in referenced_plans {
        let id = selected
            .get(&doc.path)
            .expect("selected plan id should exist")
            .clone();
        let title = humanize_slug(&id);
        let unit = ExplicitUnitSpec {
            id: id.clone(),
            title: title.clone(),
            description: doc.title.clone(),
        };
        let family = explicit_unit_family(target_repo, &unit);
        let category = explicit_unit_category(&unit);
        let output_root = PathBuf::from("outputs").join(&id);
        let (kind, artifacts, milestones, produces, health_command, verify_command) =
            category_contract(category, family, &output_root, target_repo);
        let dependencies = explicit_plan_dependency_paths(&doc.body)
            .into_iter()
            .filter_map(|path| selected.get(&path))
            .fold(
                BTreeMap::<String, LaneDependency>::new(),
                |mut acc, dependency_id| {
                    acc.entry(dependency_id.clone())
                        .or_insert_with(|| LaneDependency {
                            unit: dependency_id.clone(),
                            lane: None,
                            milestone: Some("reviewed".to_string()),
                        });
                    acc
                },
            )
            .into_values()
            .collect::<Vec<_>>();
        let goal = build_goal(
            &title,
            &family,
            &output_root,
            corpus,
            std::slice::from_ref(&doc.title),
            &artifacts,
        );
        let prompt_context = Some(
            [
                format!("Program plan:\n- `{}`", active_plan.path.display()),
                format!("Work plan:\n- `{}`", doc.path.display()),
                format!("Plan title:\n- {}", doc.title),
                format!(
                    "Artifacts to write:\n{}",
                    artifacts
                        .iter()
                        .map(|artifact| format!("- `{}`", artifact.path.display()))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
            ]
            .join("\n\n"),
        );

        intents.push(LaneIntent {
            id,
            title,
            output_root,
            family,
            kind,
            goal,
            prompt_context,
            dependencies,
            produces,
            health_command,
            verify_command,
            milestones,
            artifacts,
        });
    }

    Some(intents)
}

fn referenced_bootstrap_plan_docs(
    corpus: &PlanningCorpus,
    active_plan: &PlanningDocument,
) -> Vec<PlanningDocument> {
    let mut resolved = Vec::new();
    let mut seen = BTreeMap::<PathBuf, ()>::new();
    let has_phases = active_plan.body.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("Phase 0") || trimmed.starts_with("Phase 1")
    });
    let mut current_phase = None::<usize>;

    for line in active_plan.body.lines() {
        let trimmed = line.trim();
        if let Some(phase) = parse_phase_heading(trimmed) {
            current_phase = Some(phase);
            continue;
        }
        if has_phases && current_phase.unwrap_or(usize::MAX) > 1 {
            continue;
        }
        let Some(task) = trimmed
            .strip_prefix("- [ ] ")
            .or_else(|| trimmed.strip_prefix("- [x] "))
        else {
            continue;
        };
        for doc in referenced_plan_docs_for_task(corpus, task) {
            if seen.insert(doc.path.clone(), ()).is_some() {
                continue;
            }
            resolved.push(doc);
        }
    }

    resolved
}

fn explicit_plan_dependency_paths(body: &str) -> Vec<PathBuf> {
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
        paths.extend(markdown_path_references(&segment, "plans"));
    }
    paths.sort();
    paths.dedup();
    paths
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

fn parse_phase_heading(trimmed: &str) -> Option<usize> {
    let rest = trimmed.strip_prefix("Phase ")?;
    let digits = rest
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

fn referenced_plan_docs_for_task(corpus: &PlanningCorpus, task: &str) -> Vec<PlanningDocument> {
    let lower = task.to_ascii_lowercase();
    let mut docs = markdown_path_references(task, "plans")
        .into_iter()
        .filter_map(|path| {
            corpus
                .plan_docs
                .iter()
                .find(|doc| doc.path == path)
                .cloned()
        })
        .collect::<Vec<_>>();

    for doc in &corpus.plan_docs {
        let stem = doc
            .path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        let prefix = stem.split('-').next().unwrap_or_default().trim();
        if prefix.len() == 3 && prefix.chars().all(|ch| ch.is_ascii_digit()) {
            if lower.contains(&format!("plan {prefix}")) {
                docs.push(doc.clone());
            }
        }
    }
    docs.sort_by(|left, right| left.path.cmp(&right.path));
    docs.dedup_by(|left, right| left.path == right.path);
    docs
}

fn plan_unit_id(doc: &PlanningDocument) -> String {
    let stem = doc
        .path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let without_prefix = stem
        .strip_prefix(
            stem.split('-')
                .next()
                .filter(|prefix| prefix.len() == 3 && prefix.chars().all(|ch| ch.is_ascii_digit()))
                .map(|prefix| format!("{prefix}-"))
                .as_deref()
                .unwrap_or(""),
        )
        .unwrap_or(&stem)
        .to_string();
    let simplified = without_prefix
        .trim_end_matches("-game")
        .trim_end_matches("-plan")
        .trim_end_matches("-crate")
        .trim_end_matches("-trait")
        .to_string();
    sanitize_identifier(&simplified)
}

fn parse_explicit_units_from_spec(body: &str) -> Vec<ExplicitUnitSpec> {
    let mut in_manifest_section = false;
    let mut units = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") || trimmed.starts_with("### ") {
            in_manifest_section = trimmed.eq_ignore_ascii_case("### Raspberry Program Manifest");
            continue;
        }
        if !in_manifest_section {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("- **") else {
            continue;
        };
        let Some(end) = rest.find("**") else {
            continue;
        };
        let raw_id = rest[..end].trim();
        let description = rest[end + 2..]
            .trim_start()
            .strip_prefix(':')
            .unwrap_or(rest[end + 2..].trim_start())
            .trim();
        if raw_id.is_empty() || description.is_empty() {
            continue;
        }
        let id = sanitize_identifier(raw_id);
        if id.is_empty() {
            continue;
        }
        units.push(ExplicitUnitSpec {
            id: id.clone(),
            title: humanize_slug(&id),
            description: description.to_string(),
        });
    }

    units
}

fn derive_explicit_unit_intents(
    target_repo: &Path,
    corpus: &PlanningCorpus,
    units: &[ExplicitUnitSpec],
) -> Vec<LaneIntent> {
    let identity_by_id = units
        .iter()
        .map(|unit| (unit.id.clone(), unit.title.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut intents = Vec::new();

    for unit in units {
        let family = explicit_unit_family(target_repo, unit);
        let output_root = PathBuf::from("outputs").join(&unit.id);
        let category = explicit_unit_category(unit);
        let (kind, artifacts, milestones, produces, health_command, verify_command) =
            category_contract(category, family, &output_root, target_repo);
        let related_plans = related_plan_docs_for_unit(corpus, unit);
        let dependencies = explicit_unit_dependencies(unit, &identity_by_id);
        let goal = build_explicit_unit_goal(
            unit,
            family,
            &output_root,
            corpus,
            &related_plans,
            &artifacts,
        );
        let prompt_context =
            build_explicit_unit_prompt_context(unit, corpus, &related_plans, &artifacts);

        intents.push(LaneIntent {
            id: unit.id.clone(),
            title: unit.title.clone(),
            output_root,
            family,
            kind,
            goal,
            prompt_context,
            dependencies,
            produces,
            health_command,
            verify_command,
            milestones,
            artifacts,
        });
    }

    intents
}

fn explicit_unit_family(target_repo: &Path, unit: &ExplicitUnitSpec) -> WorkflowTemplate {
    let text = format!(
        "{}\n{}",
        unit.title.to_ascii_lowercase(),
        unit.description.to_ascii_lowercase()
    );
    if text.contains("monitoring") || text.contains("operations") || text.contains("deployment") {
        return WorkflowTemplate::ServiceBootstrap;
    }
    if text.contains("websocket") || text.contains("wallet escrow") || text.contains("remote") {
        return WorkflowTemplate::ServiceBootstrap;
    }
    if text.contains("depends on") || text.contains("tui") || text.contains("menu") {
        return WorkflowTemplate::Bootstrap;
    }
    if has_existing_child_programs(target_repo) && has_child_program_cues(&text) {
        return WorkflowTemplate::Orchestration;
    }
    WorkflowTemplate::Bootstrap
}

fn explicit_unit_category(unit: &ExplicitUnitSpec) -> TaskCategory {
    let id = unit.id.as_str();
    let text = format!(
        "{}\n{}",
        unit.title.to_ascii_lowercase(),
        unit.description.to_ascii_lowercase()
    );
    if id == "house" || text.contains("websocket") || text.contains("wallet escrow") {
        return TaskCategory::Service;
    }
    if id == "shell" || text.contains("tui") || text.contains("menu") {
        return TaskCategory::Client;
    }
    if id == "provably-fair" || text.contains("verification") || text.contains("shuffle") {
        return TaskCategory::Proof;
    }
    if id == "infra" || text.contains("monitoring") || text.contains("node operations") {
        return TaskCategory::Service;
    }
    TaskCategory::Foundations
}

fn explicit_unit_dependencies(
    unit: &ExplicitUnitSpec,
    identity_by_id: &BTreeMap<String, String>,
) -> Vec<LaneDependency> {
    let mut dependencies = Vec::new();
    let mut seen = BTreeMap::<String, ()>::new();
    let lower = unit.description.to_ascii_lowercase();
    for dependency_id in identity_by_id.keys() {
        if dependency_id == &unit.id {
            continue;
        }
        let exact = dependency_id.replace('-', " ");
        if !lower.contains(&exact) {
            continue;
        }
        if seen.insert(dependency_id.clone(), ()).is_some() {
            continue;
        }
        dependencies.push(LaneDependency {
            unit: dependency_id.clone(),
            lane: None,
            milestone: Some("reviewed".to_string()),
        });
    }
    dependencies
}

fn related_plan_docs_for_unit(
    corpus: &PlanningCorpus,
    unit: &ExplicitUnitSpec,
) -> Vec<PlanningDocument> {
    let mut scored = corpus
        .plan_docs
        .iter()
        .map(|doc| (score_plan_for_unit(doc, unit), doc))
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();
    scored.sort_by(|(left_score, left_doc), (right_score, right_doc)| {
        right_score
            .cmp(left_score)
            .then_with(|| left_doc.path.cmp(&right_doc.path))
    });
    scored
        .into_iter()
        .take(3)
        .map(|(_, doc)| doc.clone())
        .collect()
}

fn score_plan_for_unit(doc: &PlanningDocument, unit: &ExplicitUnitSpec) -> usize {
    let stem = doc
        .path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let title = doc.title.to_ascii_lowercase();
    let body = doc.body.to_ascii_lowercase();
    let keywords = unit_keywords(unit);

    let mut score = 0usize;
    for keyword in keywords {
        if stem.contains(&keyword) {
            score += 20;
        }
        if title.contains(&keyword) {
            score += 25;
        }
        if body.contains(&keyword) {
            score += 5;
        }
    }
    if stem.starts_with("001-") {
        score += 2;
    }
    score
}

fn unit_keywords(unit: &ExplicitUnitSpec) -> Vec<String> {
    let mut keywords = vec![unit.id.clone(), unit.id.replace('-', " ")];
    keywords.extend(
        unit.title
            .split_whitespace()
            .map(|part| part.to_ascii_lowercase())
            .filter(|part| part.len() > 3),
    );
    keywords.extend(
        unit.description
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .map(|part| part.to_ascii_lowercase())
            .filter(|part| part.len() > 4)
            .filter(|part| {
                !matches!(
                    part.as_str(),
                    "lanes"
                        | "their"
                        | "reaching"
                        | "milestone"
                        | "screen"
                        | "implementation"
                        | "deterministic"
                        | "protocol"
                        | "management"
                        | "wallet"
                )
            }),
    );
    match unit.id.as_str() {
        "house" => keywords.extend(["house agent", "websocket", "escrow"].map(str::to_string)),
        "shell" => keywords.extend(["tui shell", "game menu"].map(str::to_string)),
        "provably-fair" => keywords.extend(["provably fair", "shuffle"].map(str::to_string)),
        "infra" => keywords.extend(["infrastructure", "monero"].map(str::to_string)),
        "poker" => keywords.extend(["nlhe", "blueprint"].map(str::to_string)),
        _ => {}
    }
    keywords.sort();
    keywords.dedup();
    keywords
}

fn build_explicit_unit_goal(
    unit: &ExplicitUnitSpec,
    family: WorkflowTemplate,
    output_root: &Path,
    corpus: &PlanningCorpus,
    related_plans: &[PlanningDocument],
    artifacts: &[BlueprintArtifact],
) -> String {
    let plan_tasks = if related_plans.is_empty() {
        vec![unit.description.clone()]
    } else {
        related_plans.iter().map(|doc| doc.title.clone()).collect()
    };
    build_goal(
        &unit.title,
        &family,
        output_root,
        corpus,
        &plan_tasks,
        artifacts,
    )
}

fn build_explicit_unit_prompt_context(
    unit: &ExplicitUnitSpec,
    corpus: &PlanningCorpus,
    related_plans: &[PlanningDocument],
    artifacts: &[BlueprintArtifact],
) -> Option<String> {
    let mut sections = vec![format!(
        "Spec unit:\n- `{}` — {}",
        unit.id, unit.description
    )];
    if let Some(spec) = &corpus.active_spec {
        sections.push(format!("Active spec:\n- `{}`", spec.path.display()));
    }
    if !related_plans.is_empty() {
        sections.push(format!(
            "Related plans:\n{}",
            related_plans
                .iter()
                .map(|doc| format!("- `{}`", doc.path.display()))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    } else if let Some(plan) = &corpus.active_plan {
        sections.push(format!("Program plan:\n- `{}`", plan.path.display()));
    }
    if !artifacts.is_empty() {
        sections.push(format!(
            "Artifacts to write:\n{}",
            artifacts
                .iter()
                .map(|artifact| format!("- `{}`", artifact.path.display()))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    Some(sections.join("\n\n"))
}

fn derive_max_parallel(intents: &[LaneIntent]) -> usize {
    if intents.len() <= 1 {
        return 1;
    }
    intents
        .iter()
        .filter(|intent| intent.dependencies.is_empty())
        .count()
        .max(1)
        .min(3)
}

fn select_family(target_repo: &Path, corpus: &PlanningCorpus) -> WorkflowTemplate {
    let corpus_text = combined_corpus_text(corpus);
    let recurring_focus = recurring_focus_text(corpus);
    if has_child_program_cues(&corpus_text) && has_existing_child_programs(target_repo) {
        return WorkflowTemplate::Orchestration;
    }
    if has_recurring_cues(&recurring_focus) {
        return WorkflowTemplate::RecurringReport;
    }
    if has_service_health_cues(&corpus_text) {
        return WorkflowTemplate::ServiceBootstrap;
    }
    if repo_has_reviewed_slice(target_repo) && guess_proof_command(target_repo).is_some() {
        return WorkflowTemplate::Implementation;
    }
    WorkflowTemplate::Bootstrap
}

fn select_family_for_category(
    target_repo: &Path,
    corpus: &PlanningCorpus,
    category: TaskCategory,
    category_tasks: &[String],
) -> WorkflowTemplate {
    let task_text = category_tasks.join("\n").to_lowercase();
    match category {
        TaskCategory::Service => {
            if has_service_health_cues(&task_text)
                || task_text.contains("service")
                || task_text.contains("daemon")
                || task_text.contains("health")
                || task_text.contains("miner")
            {
                WorkflowTemplate::ServiceBootstrap
            } else {
                WorkflowTemplate::Bootstrap
            }
        }
        TaskCategory::Agent
            if has_child_program_cues(&task_text) && has_existing_child_programs(target_repo) =>
        {
            WorkflowTemplate::Orchestration
        }
        _ => {
            let family = select_family(target_repo, corpus);
            if family == WorkflowTemplate::RecurringReport
                || family == WorkflowTemplate::Orchestration
            {
                WorkflowTemplate::Bootstrap
            } else {
                family
            }
        }
    }
}

#[derive(Debug, Clone)]
struct LaneIdentity {
    id: String,
    title: String,
}

fn category_contract(
    category: TaskCategory,
    family: WorkflowTemplate,
    output_root: &Path,
    target_repo: &Path,
) -> (
    LaneKind,
    Vec<BlueprintArtifact>,
    Vec<MilestoneManifest>,
    Vec<String>,
    Option<String>,
    Option<String>,
) {
    match category {
        TaskCategory::Foundations => reviewed_contract(
            LaneKind::Platform,
            "foundation_plan",
            "foundation-plan.md",
            family,
            output_root,
            target_repo,
        ),
        TaskCategory::ControlPlane => reviewed_contract(
            LaneKind::Platform,
            "control_plane_contract",
            "control-plane-contract.md",
            family,
            output_root,
            target_repo,
        ),
        TaskCategory::Service => reviewed_contract(
            LaneKind::Service,
            "service_contract",
            "service-contract.md",
            family,
            output_root,
            target_repo,
        ),
        TaskCategory::Client => reviewed_contract(
            LaneKind::Interface,
            "client_surface",
            "client-surface.md",
            family,
            output_root,
            target_repo,
        ),
        TaskCategory::Agent => reviewed_contract(
            LaneKind::Platform,
            "agent_adapter",
            "agent-adapter.md",
            family,
            output_root,
            target_repo,
        ),
        TaskCategory::Proof => reviewed_contract(
            LaneKind::Platform,
            "validation_plan",
            "validation-plan.md",
            family,
            output_root,
            target_repo,
        ),
    }
}

fn reviewed_contract(
    kind: LaneKind,
    primary_id: &str,
    primary_file: &str,
    family: WorkflowTemplate,
    output_root: &Path,
    target_repo: &Path,
) -> (
    LaneKind,
    Vec<BlueprintArtifact>,
    Vec<MilestoneManifest>,
    Vec<String>,
    Option<String>,
    Option<String>,
) {
    match family {
        WorkflowTemplate::Implementation => implementation_contract(output_root, target_repo),
        WorkflowTemplate::RecurringReport => (
            LaneKind::Recurring,
            vec![BlueprintArtifact {
                id: "report".to_string(),
                path: PathBuf::from("report.md"),
            }],
            vec![MilestoneManifest {
                id: "reported".to_string(),
                requires: vec!["report".to_string()],
            }],
            vec!["report".to_string()],
            None,
            None,
        ),
        _ => (
            kind,
            vec![
                BlueprintArtifact {
                    id: primary_id.to_string(),
                    path: PathBuf::from(primary_file),
                },
                BlueprintArtifact {
                    id: "review".to_string(),
                    path: PathBuf::from("review.md"),
                },
            ],
            vec![MilestoneManifest {
                id: "reviewed".to_string(),
                requires: vec![primary_id.to_string(), "review".to_string()],
            }],
            vec![primary_id.to_string(), "review".to_string()],
            if family == WorkflowTemplate::ServiceBootstrap {
                explicit_health_command(target_repo).or_else(|| guess_proof_command(target_repo))
            } else {
                None
            },
            None,
        ),
    }
}

fn categorize_task(text: &str) -> TaskCategory {
    let lower = text.to_lowercase();
    if [
        "hermes",
        "agent",
        "adapter",
        "delegated",
        "capabilities can be delegated",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        return TaskCategory::Agent;
    }
    if [
        "test",
        "prove",
        "audit",
        "transcript",
        "observability",
        "error taxonomy",
        "failure classes",
        "no_local_hashing",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        return TaskCategory::Proof;
    }
    if [
        "inbox",
        "event spine",
        "event journal",
        "principalid",
        "pairing record",
        "pairing records",
        "capability",
        "observe",
        "control permissions",
        "receipt",
        "alert",
        "message",
        "conversation",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        return TaskCategory::ControlPlane;
    }
    if [
        "service",
        "daemon",
        "miner backend",
        "miner simulator",
        "bootstrap_home_miner",
        "lan-only",
        "snapshot",
        "health",
        "bind",
        "home-miner",
        "control commands",
        "control flow",
        "start or stop",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        return TaskCategory::Service;
    }
    if [
        "mobile-shaped",
        "command-center",
        "gateway client",
        "onboarding",
        "pair_gateway_client",
        "read_miner_status",
        "set_mining_mode",
        "device name",
        "home",
        "bottom tab",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        return TaskCategory::Client;
    }
    TaskCategory::Foundations
}

fn ordered_present_categories(tasks: &[PlanTask]) -> Vec<TaskCategory> {
    [
        TaskCategory::Foundations,
        TaskCategory::ControlPlane,
        TaskCategory::Service,
        TaskCategory::Client,
        TaskCategory::Agent,
        TaskCategory::Proof,
    ]
    .into_iter()
    .filter(|category| tasks.iter().any(|task| task.category == *category))
    .collect()
}

fn lane_identity_for_category(category: TaskCategory, tasks: &[PlanTask]) -> LaneIdentity {
    let task_text = tasks
        .iter()
        .filter(|task| task.category == category)
        .map(|task| task.text.to_lowercase())
        .collect::<Vec<_>>()
        .join("\n");
    match category {
        TaskCategory::Foundations => LaneIdentity {
            id: "foundations".to_string(),
            title: "Foundations".to_string(),
        },
        TaskCategory::ControlPlane => LaneIdentity {
            id: if task_text.contains("event spine") || task_text.contains("inbox") {
                "private-control-plane".to_string()
            } else {
                "control-plane".to_string()
            },
            title: if task_text.contains("event spine") || task_text.contains("inbox") {
                "Private Control Plane".to_string()
            } else {
                "Control Plane".to_string()
            },
        },
        TaskCategory::Service => LaneIdentity {
            id: if task_text.contains("miner") {
                "home-miner-service".to_string()
            } else {
                "service".to_string()
            },
            title: if task_text.contains("miner") {
                "Home Miner Service".to_string()
            } else {
                "Service".to_string()
            },
        },
        TaskCategory::Client => LaneIdentity {
            id: if task_text.contains("command-center") || task_text.contains("gateway client") {
                "command-center-client".to_string()
            } else {
                "client".to_string()
            },
            title: if task_text.contains("command-center") || task_text.contains("gateway client") {
                "Command Center Client".to_string()
            } else {
                "Client".to_string()
            },
        },
        TaskCategory::Agent => LaneIdentity {
            id: if task_text.contains("hermes") {
                "hermes-adapter".to_string()
            } else {
                "agent-integration".to_string()
            },
            title: if task_text.contains("hermes") {
                "Hermes Adapter".to_string()
            } else {
                "Agent Integration".to_string()
            },
        },
        TaskCategory::Proof => LaneIdentity {
            id: "proof-and-validation".to_string(),
            title: "Proof And Validation".to_string(),
        },
    }
}

fn category_dependencies(category: TaskCategory) -> Vec<TaskCategory> {
    match category {
        TaskCategory::Foundations => Vec::new(),
        TaskCategory::ControlPlane => vec![TaskCategory::Foundations],
        TaskCategory::Service => vec![TaskCategory::Foundations, TaskCategory::ControlPlane],
        TaskCategory::Client => vec![TaskCategory::ControlPlane, TaskCategory::Service],
        TaskCategory::Agent => vec![TaskCategory::ControlPlane, TaskCategory::Service],
        TaskCategory::Proof => vec![
            TaskCategory::ControlPlane,
            TaskCategory::Service,
            TaskCategory::Client,
            TaskCategory::Agent,
        ],
    }
}

fn family_contract(
    family: &WorkflowTemplate,
    output_root: &Path,
    target_repo: &Path,
) -> (
    LaneKind,
    Vec<BlueprintArtifact>,
    Vec<MilestoneManifest>,
    Vec<String>,
    Option<String>,
    Option<String>,
) {
    match family {
        WorkflowTemplate::RecurringReport => (
            LaneKind::Recurring,
            vec![BlueprintArtifact {
                id: "report".to_string(),
                path: PathBuf::from("report.md"),
            }],
            vec![MilestoneManifest {
                id: "reported".to_string(),
                requires: vec!["report".to_string()],
            }],
            vec!["report".to_string()],
            None,
            None,
        ),
        WorkflowTemplate::ServiceBootstrap => {
            let health_command =
                explicit_health_command(target_repo).or_else(|| guess_proof_command(target_repo));
            (
                LaneKind::Service,
                vec![
                    BlueprintArtifact {
                        id: "spec".to_string(),
                        path: PathBuf::from("spec.md"),
                    },
                    BlueprintArtifact {
                        id: "review".to_string(),
                        path: PathBuf::from("review.md"),
                    },
                ],
                vec![MilestoneManifest {
                    id: "reviewed".to_string(),
                    requires: vec!["spec".to_string(), "review".to_string()],
                }],
                vec!["spec".to_string(), "review".to_string()],
                health_command,
                None,
            )
        }
        WorkflowTemplate::Implementation => implementation_contract(output_root, target_repo),
        WorkflowTemplate::Orchestration => (
            LaneKind::Orchestration,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
            None,
        ),
        WorkflowTemplate::Integration => (
            LaneKind::Integration,
            vec![BlueprintArtifact {
                id: "integration".to_string(),
                path: PathBuf::from("integration.md"),
            }],
            vec![MilestoneManifest {
                id: "integrated".to_string(),
                requires: vec!["integration".to_string()],
            }],
            vec!["integration".to_string()],
            None,
            None,
        ),
        WorkflowTemplate::Bootstrap => (
            infer_bootstrap_kind(output_root),
            vec![
                BlueprintArtifact {
                    id: "spec".to_string(),
                    path: PathBuf::from("spec.md"),
                },
                BlueprintArtifact {
                    id: "review".to_string(),
                    path: PathBuf::from("review.md"),
                },
            ],
            vec![MilestoneManifest {
                id: "reviewed".to_string(),
                requires: vec!["spec".to_string(), "review".to_string()],
            }],
            vec!["spec".to_string(), "review".to_string()],
            None,
            None,
        ),
    }
}

fn implementation_contract(
    output_root: &Path,
    target_repo: &Path,
) -> (
    LaneKind,
    Vec<BlueprintArtifact>,
    Vec<MilestoneManifest>,
    Vec<String>,
    Option<String>,
    Option<String>,
) {
    let proof_command = guess_proof_command(target_repo);
    let kind = infer_bootstrap_kind(output_root);
    (
        kind,
        vec![
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
        vec![
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
        ],
        vec![
            "implementation".to_string(),
            "verification".to_string(),
            "quality".to_string(),
            "promotion".to_string(),
            "integration".to_string(),
        ],
        explicit_health_command(target_repo),
        proof_command,
    )
}

fn materialize_unit(intent: &LaneIntent) -> BlueprintUnit {
    BlueprintUnit {
        id: intent.id.clone(),
        title: intent.title.clone(),
        output_root: intent.output_root.clone(),
        artifacts: intent.artifacts.clone(),
        milestones: intent.milestones.clone(),
        lanes: vec![BlueprintLane {
            id: intent.id.clone(),
            kind: intent.kind,
            title: format!("{} Lane", intent.title),
            family: family_name(intent.family),
            workflow_family: Some(family_name(intent.family)),
            slug: Some(intent.id.clone()),
            template: intent.family,
            goal: intent.goal.clone(),
            managed_milestone: intent
                .milestones
                .last()
                .map(|milestone| milestone.id.clone())
                .unwrap_or_else(|| "reviewed".to_string()),
            dependencies: intent.dependencies.clone(),
            produces: intent.produces.clone(),
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: intent.prompt_context.clone(),
            verify_command: intent.verify_command.clone(),
            health_command: intent.health_command.clone(),
        }],
    }
}

fn family_name(family: WorkflowTemplate) -> String {
    match family {
        WorkflowTemplate::Bootstrap => "bootstrap",
        WorkflowTemplate::ServiceBootstrap => "service_bootstrap",
        WorkflowTemplate::Implementation => "implementation",
        WorkflowTemplate::Integration => "integration",
        WorkflowTemplate::Orchestration => "orchestration",
        WorkflowTemplate::RecurringReport => "recurring_report",
    }
    .to_string()
}

fn build_authoring_notes(corpus: &PlanningCorpus, intents: &[LaneIntent]) -> Vec<String> {
    let mut notes = Vec::new();
    if let Some(plan) = &corpus.active_plan {
        notes.push(format!("active plan: {}", plan.path.display()));
    }
    if let Some(spec) = &corpus.active_spec {
        notes.push(format!("active spec: {}", spec.path.display()));
    }
    for intent in intents {
        notes.push(format!(
            "selected lane `{}` -> `{}`",
            intent.id,
            family_name(intent.family)
        ));
    }
    notes
}

fn build_goal(
    title: &str,
    family: &WorkflowTemplate,
    output_root: &Path,
    corpus: &PlanningCorpus,
    unchecked_tasks: &[String],
    artifacts: &[BlueprintArtifact],
) -> String {
    let family_line = match family {
        WorkflowTemplate::Bootstrap => {
            "Bootstrap the first honest reviewed slice for this frontier."
        }
        WorkflowTemplate::ServiceBootstrap => {
            "Bootstrap the first service slice and establish a deterministic health surface."
        }
        WorkflowTemplate::Implementation => {
            "Implement the next reviewed slice and prove merge readiness honestly."
        }
        WorkflowTemplate::RecurringReport => {
            "Produce the recurring report artifact for this frontier."
        }
        WorkflowTemplate::Orchestration => "Coordinate the child program for this frontier.",
        WorkflowTemplate::Integration => "Integrate the settled slice directly onto trunk.",
    };
    let mut goal = format!("{title}\n\n{family_line}\n\nInputs:");
    for path in corpus
        .doctrine_files
        .iter()
        .chain(corpus.evidence_paths.iter())
        .take(6)
    {
        goal.push_str(&format!("\n- `{}`", path.display()));
    }
    if !unchecked_tasks.is_empty() {
        goal.push_str("\n\nCurrent frontier tasks:");
        for task in unchecked_tasks.iter().take(6) {
            goal.push_str(&format!("\n- {task}"));
        }
    }
    if !artifacts.is_empty() {
        goal.push_str("\n\nRequired durable artifacts:");
        for artifact in artifacts {
            goal.push_str(&format!(
                "\n- `{}`",
                output_root.join(&artifact.path).display()
            ));
        }
    }
    goal
}

fn build_prompt_context(
    corpus: &PlanningCorpus,
    unchecked_tasks: &[String],
    artifacts: &[BlueprintArtifact],
) -> Option<String> {
    let mut sections = Vec::new();
    if let Some(plan) = &corpus.active_plan {
        sections.push(format!("Active plan:\n- `{}`", plan.path.display()));
    }
    if let Some(spec) = &corpus.active_spec {
        sections.push(format!("Active spec:\n- `{}`", spec.path.display()));
    }
    if !unchecked_tasks.is_empty() {
        sections.push(format!(
            "Open tasks:\n{}",
            unchecked_tasks
                .iter()
                .take(8)
                .map(|task| format!("- {task}"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !artifacts.is_empty() {
        sections.push(format!(
            "Artifacts to write:\n{}",
            artifacts
                .iter()
                .map(|artifact| format!("- `{}`", artifact.path.display()))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if sections.is_empty() {
        None
    } else {
        Some(sections.join("\n\n"))
    }
}

fn open_tasks(body: &str) -> Vec<String> {
    let mut tasks = Vec::new();
    let mut current = None::<String>;

    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(task) = trimmed.strip_prefix("- [ ] ") {
            if let Some(current_task) = current.take() {
                tasks.push(current_task);
            }
            current = Some(task.trim().to_string());
            continue;
        }
        if trimmed.starts_with("- [x] ")
            || trimmed.starts_with("#")
            || trimmed.starts_with("##")
            || trimmed.starts_with("###")
        {
            if let Some(current_task) = current.take() {
                tasks.push(current_task);
            }
            continue;
        }
        let Some(current_task) = current.as_mut() else {
            continue;
        };
        if trimmed.is_empty() {
            tasks.push(current.take().expect("task exists"));
            continue;
        }
        current_task.push(' ');
        current_task.push_str(trimmed);
    }

    if let Some(current_task) = current {
        tasks.push(current_task);
    }

    tasks
}

fn combined_corpus_text(corpus: &PlanningCorpus) -> String {
    let mut text = String::new();
    if let Some(plan) = &corpus.active_plan {
        text.push_str(&plan.title.to_lowercase());
        text.push('\n');
        text.push_str(&plan.body.to_lowercase());
        text.push('\n');
    }
    if let Some(spec) = &corpus.active_spec {
        text.push_str(&spec.title.to_lowercase());
        text.push('\n');
        text.push_str(&spec.body.to_lowercase());
        text.push('\n');
    }
    text
}

fn recurring_focus_text(corpus: &PlanningCorpus) -> String {
    let mut text = String::new();
    if let Some(plan) = &corpus.active_plan {
        text.push_str(&plan.title.to_lowercase());
        text.push('\n');
        text.push_str(&open_tasks(&plan.body).join("\n").to_lowercase());
        text.push('\n');
    }
    if let Some(spec) = &corpus.active_spec {
        text.push_str(&spec.title.to_lowercase());
        text.push('\n');
    }
    text
}

fn has_child_program_cues(text: &str) -> bool {
    [
        "child program",
        "supervise",
        "orchestrate another program",
        "coordinate child",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn has_recurring_cues(text: &str) -> bool {
    [
        "weekly",
        "daily",
        "recurring",
        "scorecard",
        "retro",
        "status report",
        "operational audit",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn has_service_health_cues(text: &str) -> bool {
    [
        "/health",
        "health endpoint",
        "health surface",
        "ready log",
        "rpc method",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn has_existing_child_programs(target_repo: &Path) -> bool {
    target_repo.join("fabro").join("programs").is_dir()
        && fs::read_dir(target_repo.join("fabro").join("programs"))
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .any(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("yaml"))
}

fn repo_has_reviewed_slice(target_repo: &Path) -> bool {
    walkdir::WalkDir::new(target_repo.join("outputs"))
        .into_iter()
        .filter_map(Result::ok)
        .any(|entry| entry.file_name() == "review.md")
}

fn explicit_health_command(target_repo: &Path) -> Option<String> {
    if target_repo.join("Cargo.toml").is_file() {
        return Some("cargo test -- --nocapture health".to_string());
    }
    None
}

fn guess_proof_command(target_repo: &Path) -> Option<String> {
    if target_repo.join("Cargo.toml").is_file() {
        return Some("cargo test".to_string());
    }
    if target_repo.join("package.json").is_file() {
        return Some("npm test".to_string());
    }
    if target_repo.join("pyproject.toml").is_file() {
        return Some("pytest".to_string());
    }
    None
}

fn resolve_existing_program_id(
    target_repo: &Path,
    program_override: Option<&str>,
) -> Result<String, PlanningError> {
    if let Some(program) = program_override {
        return Ok(program.to_string());
    }
    let programs_dir = target_repo.join("fabro").join("programs");
    if !programs_dir.is_dir() {
        return Err(PlanningError::MissingExistingProgram {
            target_repo: target_repo.to_path_buf(),
        });
    }
    let mut programs = fs::read_dir(&programs_dir)
        .map_err(|source| PlanningError::Read {
            path: programs_dir.clone(),
            source,
        })?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| PlanningError::Read {
            path: programs_dir.clone(),
            source,
        })?
        .into_iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
        .filter_map(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();
    programs.sort();
    match programs.as_slice() {
        [] => Err(PlanningError::MissingExistingProgram {
            target_repo: target_repo.to_path_buf(),
        }),
        [program] => Ok(program.clone()),
        _ => Err(PlanningError::AmbiguousExistingProgram {
            target_repo: target_repo.to_path_buf(),
            programs,
        }),
    }
}

fn derive_unit_id(title: &str, program_id: &str) -> String {
    let mut words = title
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_lowercase())
        .filter(|word| {
            !matches!(
                word.as_str(),
                "build" | "implement" | "create" | "the" | "a" | "an"
            )
        })
        .filter(|word| word != program_id)
        .collect::<Vec<_>>();
    if words.is_empty() {
        return program_id.to_string();
    }
    if words.len() > 4 {
        words.truncate(4);
    }
    words.join("-")
}

fn infer_bootstrap_kind(output_root: &Path) -> LaneKind {
    let text = output_root.display().to_string().to_lowercase();
    if text.contains("service") || text.contains("daemon") || text.contains("server") {
        LaneKind::Service
    } else if text.contains("client")
        || text.contains("app")
        || text.contains("command-center")
        || text.contains("home")
    {
        LaneKind::Interface
    } else {
        LaneKind::Artifact
    }
}

fn sanitize_identifier(value: &str) -> String {
    let mut identifier = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while identifier.contains("--") {
        identifier = identifier.replace("--", "-");
    }
    identifier.trim_matches('-').to_string()
}

fn humanize_slug(value: &str) -> String {
    value
        .split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!("{}{}", first.to_uppercase(), chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_authoring_uses_repo_doctrine_and_latest_plan() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("README.md"), "# Zend\n").expect("readme");
        fs::write(temp.path().join("SPEC.md"), "# Root Spec\n").expect("spec");
        fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        fs::create_dir_all(temp.path().join("specs")).expect("specs dir");
        fs::write(
            temp.path()
                .join("plans/2026-03-19-build-home-command-center.md"),
            "# Build the Zend Home Command Center\n\n- [ ] Implement the first slice\n",
        )
        .expect("plan");
        fs::write(
            temp.path().join("specs/2026-03-19-product-spec.md"),
            "# Zend Product Spec\n",
        )
        .expect("product spec");

        let authored =
            author_blueprint_for_create(temp.path(), Some("zend")).expect("author blueprint");

        assert_eq!(authored.blueprint.program.id, "zend");
        assert_eq!(authored.blueprint.units.len(), 1);
        assert_eq!(
            authored.blueprint.units[0].lanes[0].template,
            WorkflowTemplate::Bootstrap
        );
        assert_eq!(
            authored.active_plan,
            Some(PathBuf::from(
                "plans/2026-03-19-build-home-command-center.md"
            ))
        );
        assert!(authored
            .blueprint
            .inputs
            .doctrine_files
            .contains(&PathBuf::from("README.md")));
    }

    #[test]
    fn create_authoring_decomposes_multi_front_plan_into_multiple_lanes() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("README.md"), "# Zend\n").expect("readme");
        fs::write(temp.path().join("SPEC.md"), "# Root Spec\n").expect("spec");
        fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        fs::create_dir_all(temp.path().join("specs")).expect("specs dir");
        fs::write(
            temp.path().join("plans/2026-03-19-build-home-command-center.md"),
            "# Build the Zend Home Command Center\n\n- [ ] Create repo scaffolding for implementation artifacts\n- [ ] Add the encrypted operations inbox and route pairing approvals into it\n- [ ] Implement a local home-miner control service\n- [ ] Implement a thin mobile-shaped gateway client\n- [ ] Add a Zend-native gateway contract and a Hermes adapter\n- [ ] Add automated tests and proof transcripts\n",
        )
        .expect("plan");
        fs::write(
            temp.path().join("specs/2026-03-19-product-spec.md"),
            "# Zend Product Spec\n",
        )
        .expect("product spec");

        let authored =
            author_blueprint_for_create(temp.path(), Some("zend")).expect("author blueprint");
        let unit_ids = authored
            .blueprint
            .units
            .iter()
            .map(|unit| unit.id.clone())
            .collect::<Vec<_>>();

        assert!(unit_ids.contains(&"foundations".to_string()));
        assert!(unit_ids.contains(&"private-control-plane".to_string()));
        assert!(unit_ids.contains(&"home-miner-service".to_string()));
        assert!(unit_ids.contains(&"command-center-client".to_string()));
        assert!(unit_ids.contains(&"hermes-adapter".to_string()));
        assert!(unit_ids.contains(&"proof-and-validation".to_string()));
    }

    #[test]
    fn open_tasks_collects_wrapped_checklist_items() {
        let body = r#"
- [ ] Add the minimal inbox architecture contract for milestone 1, including a
  shared `PrincipalId` that also owns future inbox access.
- [ ] Implement a local home-miner control service that exposes safe status and
  control operations without performing any work on the client device.
"#;

        let tasks = open_tasks(body);

        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].contains("shared `PrincipalId`"));
        assert!(tasks[1].contains("control operations"));
    }

    #[test]
    fn evolve_authoring_adds_missing_doctrine_units_to_parent_program() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("README.md"), "# Zend\n").expect("readme");
        fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        fs::create_dir_all(temp.path().join("fabro/run-configs/bootstrap"))
            .expect("run-config dir");
        fs::write(
            temp.path()
                .join("fabro/run-configs/bootstrap/foundations.toml"),
            "version = 1\n",
        )
        .expect("run config");
        fs::write(
            temp.path().join("plans/2026-03-20-expand-zend.md"),
            "# Expand Zend\n\n- [ ] Add the encrypted operations inbox\n- [ ] Implement a local home-miner control service\n",
        )
        .expect("plan");
        fs::create_dir_all(temp.path().join("fabro/programs")).expect("program dir");
        fs::write(
            temp.path().join("fabro/programs/zend.yaml"),
            r#"
version: 1
program: zend
target_repo: ../..
state_path: ../../.raspberry/zend-state.json
max_parallel: 1
units:
  - id: foundations
    title: Foundations
    output_root: outputs/foundations
    artifacts:
      - id: foundation_plan
        path: foundation-plan.md
      - id: review
        path: review.md
    milestones:
      - id: reviewed
        requires: [foundation_plan, review]
    lanes:
      - id: foundations
        kind: platform
        title: Foundations Lane
        run_config: ../run-configs/bootstrap/foundations.toml
        managed_milestone: reviewed
        produces: [foundation_plan, review]
"#,
        )
        .expect("manifest");

        let authored =
            author_blueprint_for_evolve(temp.path(), Some("zend")).expect("author blueprint");
        let unit_ids = authored
            .blueprint
            .units
            .iter()
            .map(|unit| unit.id.clone())
            .collect::<Vec<_>>();

        assert!(unit_ids.contains(&"foundations".to_string()));
        assert!(unit_ids.contains(&"private-control-plane".to_string()));
        assert!(unit_ids.contains(&"home-miner-service".to_string()));
    }

    #[test]
    fn evolve_authoring_keeps_implementation_programs_doctrine_scoped() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("README.md"), "# Zend\n").expect("readme");
        fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        fs::create_dir_all(temp.path().join("fabro/run-configs/implementation"))
            .expect("implement run-config dir");
        fs::create_dir_all(temp.path().join("fabro/run-configs/integration"))
            .expect("integrate run-config dir");
        fs::write(
            temp.path()
                .join("fabro/run-configs/implementation/private-control-plane.toml"),
            "version = 1\n",
        )
        .expect("implement run config");
        fs::write(
            temp.path()
                .join("fabro/run-configs/integration/private-control-plane.toml"),
            "version = 1\n",
        )
        .expect("integrate run config");
        fs::write(
            temp.path().join("plans/2026-03-20-expand-zend.md"),
            "# Expand Zend\n\n- [ ] Add the encrypted operations inbox\n- [ ] Implement a local home-miner control service\n",
        )
        .expect("plan");
        fs::create_dir_all(temp.path().join("fabro/programs")).expect("program dir");
        fs::write(
            temp.path()
                .join("fabro/programs/zend-private-control-plane-implementation.yaml"),
            r#"
version: 1
program: zend-private-control-plane-implementation
target_repo: ../..
state_path: ../../.raspberry/zend-private-control-plane-implementation-state.json
max_parallel: 1
units:
  - id: private-control-plane
    title: Private Control Plane
    output_root: outputs/private-control-plane
    artifacts:
      - id: spec
        path: control-plane-contract.md
      - id: review
        path: review.md
      - id: implementation
        path: implementation.md
      - id: verification
        path: verification.md
      - id: quality
        path: quality.md
      - id: promotion
        path: promotion.md
      - id: integration
        path: integration.md
    milestones:
      - id: reviewed
        requires: [spec, review]
      - id: implemented
        requires: [implementation]
      - id: verified
        requires: [verification]
      - id: merge_ready
        requires: [implementation, verification, quality, promotion]
      - id: integrated
        requires: [integration]
    lanes:
      - id: private-control-plane-implement
        kind: platform
        title: Implement
        run_config: ../run-configs/implementation/private-control-plane.toml
        managed_milestone: merge_ready
        produces: [implementation, verification, quality, promotion]
      - id: private-control-plane-integrate
        kind: integration
        title: Integrate
        run_config: ../run-configs/integration/private-control-plane.toml
        managed_milestone: integrated
        depends_on:
          - unit: private-control-plane
            lane: private-control-plane-implement
            milestone: merge_ready
        produces: [integration]
"#,
        )
        .expect("manifest");

        let authored = author_blueprint_for_evolve(
            temp.path(),
            Some("zend-private-control-plane-implementation"),
        )
        .expect("author blueprint");

        assert_eq!(authored.blueprint.units.len(), 1);
        assert_eq!(authored.blueprint.units[0].id, "private-control-plane");
    }

    #[test]
    fn create_authoring_prefers_explicit_spec_units_over_leaf_plan_drift() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("README.md"), "# rXMRagent\n").expect("readme");
        fs::write(
            temp.path().join("SPEC.md"),
            "# Root Spec\n\nUse a spec before a plan.\n",
        )
        .expect("root spec");
        fs::create_dir_all(temp.path().join("specs")).expect("specs dir");
        fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        fs::write(
            temp.path().join("specs/001-rxmragent-founding.md"),
            concat!(
                "# Decision Spec: rXMRagent — Zero-Human Game Studio on rXMR\n\n",
                "## Studio Structure (Paperclip Company)\n\n",
                "### Raspberry Program Manifest\n\n",
                "The program manifest defines the first units:\n\n",
                "- **poker**: The heads-up NLHE game. Lanes: GameVariant implementation, TUI poker screen, blueprint integration.\n",
                "- **blackjack**: The blackjack game. Lanes: GameVariant implementation, TUI blackjack screen, dealer rules.\n",
                "- **house**: The remote house agent. Lanes: multi-game session management, WebSocket protocol, rXMR wallet escrow, deployment.\n",
                "- **shell**: The TUI shell (game menu, wallet screen, verification). Depends on poker and blackjack reaching their `verified` milestone.\n",
                "- **provably-fair**: The core fairness crate. Lanes: seed protocol, deterministic shuffle, action_seed mapping, session verification.\n",
                "- **infra**: Node operations and monitoring. Recurring lane.\n",
            ),
        )
        .expect("founding spec");
        fs::write(
            temp.path().join("plans/001-rxmr-poker-mvp.md"),
            "# rXMR Casino MVP — Provably Fair Terminal Games on a Privacy Chain\n\n- [ ] Milestone 1: Provably fair crate\n- [ ] Milestone 2: Poker game\n- [ ] Milestone 3: Blackjack game\n- [ ] Milestone 4: House agent\n- [ ] Milestone 5: TUI shell\n",
        )
        .expect("mvp plan");
        fs::write(
            temp.path().join("plans/002-provably-fair-crate.md"),
            "# Provably Fair Crate: The Trust Surface of the rXMR Casino\n",
        )
        .expect("provably fair");
        fs::write(
            temp.path().join("plans/003-poker-game.md"),
            "# Poker Game Implementation (GameVariant, TUI Screen, Blueprint Integration)\n",
        )
        .expect("poker plan");
        fs::write(
            temp.path().join("plans/004-blackjack-game.md"),
            "# Blackjack Game: GameVariant, Dealer Rules, TUI Screen, Verification\n",
        )
        .expect("blackjack plan");
        fs::write(
            temp.path().join("plans/013-house-agent.md"),
            "# House Agent: The Remote Multi-Game Casino Server\n",
        )
        .expect("house plan");
        fs::write(
            temp.path().join("plans/014-tui-shell.md"),
            "# TUI Shell: The Terminal Casino Client\n",
        )
        .expect("shell plan");
        fs::write(
            temp.path().join("plans/015-monero-infrastructure.md"),
            "# Monero Infrastructure: Node, Wallet RPC, and Deployment\n",
        )
        .expect("infra plan");
        fs::write(
            temp.path().join("plans/018-war-game.md"),
            "# Casino War — The Simplest Card Game in the Catalog\n",
        )
        .expect("war plan");
        fs::write(
            temp.path().join("plans/021-wheel-game.md"),
            "# Wheel of Fortune — Seed-Derived Multiplier Wheel for rXMR Casino\n",
        )
        .expect("wheel plan");

        let authored =
            author_blueprint_for_create(temp.path(), Some("rxmragent")).expect("author blueprint");
        let unit_ids = authored
            .blueprint
            .units
            .iter()
            .map(|unit| unit.id.clone())
            .collect::<Vec<_>>();
        let poker_goal = &authored
            .blueprint
            .units
            .iter()
            .find(|unit| unit.id == "poker")
            .expect("poker unit")
            .lanes[0]
            .goal;

        assert_eq!(
            authored.active_plan,
            Some(PathBuf::from("plans/001-rxmr-poker-mvp.md"))
        );
        assert!(unit_ids.contains(&"poker".to_string()));
        assert!(unit_ids.contains(&"blackjack".to_string()));
        assert!(unit_ids.contains(&"house".to_string()));
        assert!(unit_ids.contains(&"shell".to_string()));
        assert!(unit_ids.contains(&"provably-fair".to_string()));
        assert!(unit_ids.contains(&"infra".to_string()));
        assert!(!unit_ids.contains(&"foundations".to_string()));
        assert!(!poker_goal.contains("War"));
        assert!(poker_goal.contains("Poker Game Implementation"));
    }

    #[test]
    fn create_authoring_decomposes_master_plan_references_into_units() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("README.md"), "# rXMRagent\n").expect("readme");
        fs::write(
            temp.path().join("SPEC.md"),
            "# Root Spec\n\nThis repo builds a zero-human casino.\n",
        )
        .expect("root spec");
        fs::create_dir_all(temp.path().join("specs")).expect("specs dir");
        fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        fs::write(
            temp.path().join("specs/001-rxmragent-founding.md"),
            "# Decision Spec: rXMRagent\n",
        )
        .expect("founding spec");
        fs::write(
            temp.path().join("plans/001-master-plan.md"),
            concat!(
                "# rxmr-play Master Plan\n\n",
                "## Progress\n\n",
                "Phase 0 (Foundation):\n",
                "- [ ] Provably fair crate (plan 002)\n",
                "- [ ] Casino-core crate with GameVariant trait (plan 016)\n",
                "- [ ] House agent skeleton with WebSocket server (plan 013)\n",
                "- [ ] TUI shell skeleton with three-layer layout (plan 014)\n",
                "- [ ] Monero infrastructure (plan 015)\n\n",
                "Phase 1 (Day 1 -- ship together):\n",
                "- [ ] Poker (plan 003)\n",
                "- [ ] Blackjack (plan 004)\n\n",
                "Phase 2 (Later):\n",
                "- [ ] Faucet (plan 027)\n",
            ),
        )
        .expect("master plan");
        fs::write(
            temp.path().join("plans/002-provably-fair-crate.md"),
            "# Provably Fair Crate: The Trust Surface of the rXMR Casino\n",
        )
        .expect("plan 002");
        fs::write(
            temp.path().join("plans/003-poker-game.md"),
            "# Poker Game Implementation\n",
        )
        .expect("plan 003");
        fs::write(
            temp.path().join("plans/004-blackjack-game.md"),
            "# Blackjack Game\n",
        )
        .expect("plan 004");
        fs::write(
            temp.path().join("plans/013-house-agent.md"),
            "# House Agent\n",
        )
        .expect("plan 013");
        fs::write(temp.path().join("plans/014-tui-shell.md"), "# TUI Shell\n").expect("plan 014");
        fs::write(
            temp.path().join("plans/015-monero-infrastructure.md"),
            "# Monero Infrastructure\n",
        )
        .expect("plan 015");
        fs::write(
            temp.path().join("plans/016-casino-core-trait.md"),
            "# Casino Core Trait\n",
        )
        .expect("plan 016");
        fs::write(temp.path().join("plans/027-faucet.md"), "# rXMR Faucet\n").expect("plan 027");

        let authored =
            author_blueprint_for_create(temp.path(), Some("rxmragent")).expect("author blueprint");
        let unit_ids = authored
            .blueprint
            .units
            .iter()
            .map(|unit| unit.id.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            authored.active_plan,
            Some(PathBuf::from("plans/001-master-plan.md"))
        );
        assert!(unit_ids.contains(&"provably-fair".to_string()));
        assert!(unit_ids.contains(&"casino-core".to_string()));
        assert!(unit_ids.contains(&"house-agent".to_string()));
        assert!(unit_ids.contains(&"tui-shell".to_string()));
        assert!(unit_ids.contains(&"monero-infrastructure".to_string()));
        assert!(unit_ids.contains(&"poker".to_string()));
        assert!(unit_ids.contains(&"blackjack".to_string()));
        assert!(!unit_ids.contains(&"faucet".to_string()));
        assert!(!unit_ids.contains(&"foundations".to_string()));
    }

    #[test]
    fn create_authoring_master_plan_dependencies_use_explicit_depends_on_clauses() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("README.md"), "# rXMRagent\n").expect("readme");
        fs::write(temp.path().join("SPEC.md"), "# Root Spec\n").expect("spec");
        fs::create_dir_all(temp.path().join("specs")).expect("specs dir");
        fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        fs::write(
            temp.path().join("specs/001-rxmragent-founding.md"),
            "# Decision Spec: rXMRagent\n",
        )
        .expect("founding spec");
        fs::write(
            temp.path().join("plans/001-master-plan.md"),
            concat!(
                "# rxmr-play Master Plan\n\n",
                "Phase 0:\n",
                "- [ ] Provably fair crate (plan 002)\n",
                "- [ ] Casino-core crate with GameVariant trait (plan 016)\n",
                "- [ ] House agent skeleton with WebSocket server (plan 013)\n",
                "- [ ] TUI shell skeleton with three-layer layout (plan 014)\n",
                "- [ ] Monero infrastructure (plan 015)\n",
            ),
        )
        .expect("master plan");
        fs::write(
            temp.path().join("plans/002-provably-fair-crate.md"),
            "# Provably Fair Crate\n\nThis plan depends on: `plans/015-monero-infrastructure.md`.\n",
        )
        .expect("plan 002");
        fs::write(
            temp.path().join("plans/013-house-agent.md"),
            "# House Agent\n\nThis plan depends on: `plans/002-provably-fair-crate.md`, `plans/016-casino-core-trait.md`, and `plans/015-monero-infrastructure.md`. This plan is depended on by: `plans/014-tui-shell.md`.\n",
        )
        .expect("plan 013");
        fs::write(
            temp.path().join("plans/014-tui-shell.md"),
            "# TUI Shell\n\nThis plan depends on: `plans/002-provably-fair-crate.md`, `plans/016-casino-core-trait.md`, `plans/013-house-agent.md`, and `plans/015-monero-infrastructure.md`.\n",
        )
        .expect("plan 014");
        fs::write(
            temp.path().join("plans/015-monero-infrastructure.md"),
            "# Monero Infrastructure\n",
        )
        .expect("plan 015");
        fs::write(
            temp.path().join("plans/016-casino-core-trait.md"),
            "# Casino Core Trait\n\nThis plan depends on: `plans/002-provably-fair-crate.md`.\n",
        )
        .expect("plan 016");

        let authored =
            author_blueprint_for_create(temp.path(), Some("rxmragent")).expect("author blueprint");
        let casino_core = authored
            .blueprint
            .units
            .iter()
            .find(|unit| unit.id == "casino-core")
            .expect("casino-core unit");
        let house_agent = authored
            .blueprint
            .units
            .iter()
            .find(|unit| unit.id == "house-agent")
            .expect("house-agent unit");
        let tui_shell = authored
            .blueprint
            .units
            .iter()
            .find(|unit| unit.id == "tui-shell")
            .expect("tui-shell unit");

        let casino_core_dependencies = &casino_core.lanes[0].dependencies;
        let house_agent_dependencies = &house_agent.lanes[0].dependencies;
        let tui_shell_dependencies = &tui_shell.lanes[0].dependencies;

        assert_eq!(
            casino_core_dependencies
                .iter()
                .map(|dependency| dependency.unit.clone())
                .collect::<Vec<_>>(),
            vec!["provably-fair".to_string()]
        );
        assert_eq!(
            house_agent_dependencies
                .iter()
                .map(|dependency| dependency.unit.clone())
                .collect::<Vec<_>>(),
            vec![
                "casino-core".to_string(),
                "monero-infrastructure".to_string(),
                "provably-fair".to_string(),
            ]
        );
        assert_eq!(
            tui_shell_dependencies
                .iter()
                .map(|dependency| dependency.unit.clone())
                .collect::<Vec<_>>(),
            vec![
                "casino-core".to_string(),
                "house-agent".to_string(),
                "monero-infrastructure".to_string(),
                "provably-fair".to_string(),
            ]
        );
    }
}
