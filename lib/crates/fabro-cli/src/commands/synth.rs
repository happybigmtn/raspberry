use std::collections::BTreeSet;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use fabro_synthesis::{
    author_blueprint_for_create_with_planning_root, author_blueprint_for_evolve,
    import_existing_package, load_blueprint, reconcile_blueprint, render_blueprint, save_blueprint,
    ImportRequest, ReconcileRequest, RenderRequest,
};
use raspberry_supervisor::{load_plan_registry, load_plan_registry_from_planning_root, PlanRecord};
use serde::{Deserialize, Serialize};

#[derive(Debug, Subcommand)]
pub enum SynthCommand {
    /// Import an existing Fabro workflow package into a blueprint file
    Import(SynthImportArgs),
    /// Create a checked-in Fabro workflow package from a blueprint
    Create(SynthCreateArgs),
    /// Evolve an existing Fabro workflow package from a revised blueprint
    Evolve(SynthEvolveArgs),
    /// Eng-review mapping contracts with Opus 4.6 adversarial pass
    Review(SynthReviewArgs),
    /// Generate SPEC.md, PLANS.md, and numbered plans for an unfamiliar codebase, then run synth create
    Genesis(SynthGenesisArgs),
}

#[derive(Debug, Args)]
pub struct SynthImportArgs {
    #[arg(long)]
    pub target_repo: PathBuf,
    #[arg(long)]
    pub program: String,
    #[arg(long)]
    pub output: PathBuf,
}

#[derive(Debug, Args)]
pub struct SynthCreateArgs {
    #[arg(long)]
    pub blueprint: Option<PathBuf>,
    #[arg(long)]
    pub target_repo: PathBuf,
    #[arg(long)]
    pub program: Option<String>,
    #[arg(long)]
    pub output_blueprint: Option<PathBuf>,
    /// Skip Opus 4.6 decomposition pass (use deterministic heuristics only)
    #[arg(long)]
    pub no_decompose: bool,
    #[arg(long, hide = true)]
    pub planning_root: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct SynthEvolveArgs {
    #[arg(long)]
    pub blueprint: Option<PathBuf>,
    #[arg(long)]
    pub target_repo: PathBuf,
    #[arg(long)]
    pub preview_root: Option<PathBuf>,
    #[arg(long)]
    pub program: Option<String>,
    #[arg(long)]
    pub output_blueprint: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct SynthReviewArgs {
    #[arg(long)]
    pub target_repo: PathBuf,
    #[arg(long)]
    pub program: Option<String>,
}

pub fn review_command(args: &SynthReviewArgs) -> anyhow::Result<()> {
    let registry = load_plan_registry(&args.target_repo)?;
    let mappings_dir = args
        .target_repo
        .join(fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR)
        .join("plan-mappings");

    if !mappings_dir.is_dir() {
        anyhow::bail!(
            "no plan-mappings directory found at {}; run `synth create` first",
            mappings_dir.display()
        );
    }

    let composite_plans: Vec<_> = registry
        .plans
        .iter()
        .filter(|plan| {
            plan.composite
                && plan.declared_child_ids.len() > 1
                && plan.category != raspberry_supervisor::PlanCategory::Meta
        })
        .collect();

    if composite_plans.is_empty() {
        println!("No composite plans to review.");
        return Ok(());
    }

    let claude_check = std::process::Command::new("claude")
        .arg("--version")
        .output();
    if claude_check.is_err() || !claude_check.as_ref().unwrap().status.success() {
        anyhow::bail!("claude CLI not found; required for synth review");
    }

    // Build a manifest of all mapping contracts for Opus to review
    let mut contracts = Vec::new();
    for plan in &composite_plans {
        let mapping_path = mapping_snapshot_path(plan);
        let absolute = args.target_repo.join(&mapping_path);
        if absolute.exists() {
            let content = std::fs::read_to_string(&absolute).unwrap_or_default();
            contracts.push(format!(
                "## Plan: {} ({})\n\nMapping contract at `{}`:\n\n```yaml\n{}\n```\n",
                plan.title,
                plan.plan_id,
                mapping_path.display(),
                content,
            ));
        }
    }

    let review_prompt = format!(
        r#"You are an adversarial eng-review agent for plan decomposition contracts.

Review these mapping contracts for a Rust workspace casino project at `{}`.
For each plan, read the actual plan file to compare against the mapping contract.

{}

## Review checklist

For EACH mapping contract, check:

1. **Child count**: does the number of children match the plan's milestone count? Flag over-splits and duplicates.
2. **Archetype accuracy**: is each child's archetype correct? Game engines should be implement_module, not verification_only. House handlers should be service_surface. TUI screens should be tui_surface.
3. **Lane kind accuracy**: should this child run as platform, service, interface, artifact, integration, or orchestration? Flag service/UI work that was flattened to platform.
4. **Review profile accuracy**: `standard` for normal code, `foundation` for shared types/SDK/framework, `hardened` for security/crypto/financial/correctness-critical, `ux` for any user-facing surface. Flag misassignments.
5. **Proof commands**: do they appear verbatim in the plan text? Flag invented commands.
6. **Owned surfaces**: are they precise repo-relative paths from the plan? Flag vague or invented paths.
7. **Cross-plan surface conflicts**: do children from different plans claim the same owned surfaces without an implement_cross_surface archetype?
8. **Child ID quality**: are IDs concise (2-4 words)? Flag verbose IDs.
9. **AC contract completeness**: does every child have where_surfaces, how_description, verification_plan, rollback_condition?

Write a structured review report. For each finding, cite the specific plan and child.
Write the report to `{}/plan-mappings/review-report.md`.

Be adversarial. The goal is to catch decomposition mistakes before they become workflow bugs."#,
        args.target_repo.display(),
        contracts.join("\n---\n\n"),
        fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR,
    );

    let prompt_file = tempfile::NamedTempFile::new()?;
    std::fs::write(prompt_file.path(), &review_prompt)?;

    println!(
        "Eng-reviewing {} mapping contracts with {} ...",
        composite_plans.len(),
        DECOMPOSE_MODEL
    );

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cat {} | CLAUDECODE= claude -p --output-format text --dangerously-skip-permissions --model {} --max-turns 50",
            prompt_file.path().display(),
            DECOMPOSE_MODEL,
        ))
        .current_dir(&args.target_repo)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Opus review failed: {}", stderr.trim());
    }

    let report_path = mappings_dir.join("review-report.md");
    if report_path.exists() {
        println!("Review report written to: {}", report_path.display());
    } else {
        println!("Review completed (report may be in stdout)");
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            println!("{}", stdout);
        }
    }

    Ok(())
}

#[derive(Debug, Args)]
pub struct SynthGenesisArgs {
    #[arg(long)]
    pub target_repo: PathBuf,
    #[arg(long)]
    pub program: Option<String>,
    /// Skip the synth create step after generating plans
    #[arg(long)]
    pub plans_only: bool,
}

pub fn genesis_command(args: &SynthGenesisArgs) -> anyhow::Result<()> {
    let claude_check = std::process::Command::new("claude")
        .arg("--version")
        .output();
    if claude_check.is_err() || !claude_check.as_ref().unwrap().status.success() {
        anyhow::bail!("claude CLI not found; required for synth genesis");
    }

    let genesis_dir = args.target_repo.join("genesis");
    if genesis_dir.exists() {
        let has_contents = std::fs::read_dir(&genesis_dir)?
            .filter_map(Result::ok)
            .next()
            .is_some();
        if has_contents {
            anyhow::bail!(
                "genesis directory {} is not empty; clear it or use --plans-only after reviewing the existing corpus",
                genesis_dir.display()
            );
        }
    }
    std::fs::create_dir_all(genesis_dir.join("plans"))?;

    let prompt = build_genesis_prompt(&args.target_repo);
    let prompt_file = tempfile::NamedTempFile::new()?;
    std::fs::write(prompt_file.path(), &prompt)?;

    println!(
        "Running genesis analysis on {} ...",
        args.target_repo.display()
    );
    println!("Opus is exploring the codebase and drafting a 180-day turnaround plan.");

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cat {} | CLAUDECODE= claude -p --output-format text --dangerously-skip-permissions --model {} --max-turns 200",
            prompt_file.path().display(),
            DECOMPOSE_MODEL,
        ))
        .current_dir(&args.target_repo)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Genesis failed: {}", stderr.trim());
    }

    // Verify outputs were written
    let spec_path = genesis_dir.join("SPEC.md");
    let plans_md = genesis_dir.join("PLANS.md");
    let plans_dir = genesis_dir.join("plans");

    let plan_count = std::fs::read_dir(&plans_dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("md"))
                .count()
        })
        .unwrap_or(0);

    println!("Genesis complete:");
    println!(
        "  SPEC.md: {}",
        if spec_path.exists() {
            "written"
        } else {
            "missing"
        }
    );
    println!(
        "  PLANS.md: {}",
        if plans_md.exists() {
            "written"
        } else {
            "missing"
        }
    );
    println!("  Plans: {plan_count} files in genesis/plans/");

    if plan_count == 0 {
        anyhow::bail!("Genesis produced no plan files in genesis/plans/");
    }

    if !args.plans_only {
        println!("Running synth create on generated plans...");
        let create_args = SynthCreateArgs {
            blueprint: None,
            target_repo: args.target_repo.clone(),
            program: args.program.clone(),
            output_blueprint: None,
            no_decompose: false,
            planning_root: Some(PathBuf::from("genesis")),
        };
        create_command(&create_args)?;
    }

    Ok(())
}

fn build_genesis_prompt(target_repo: &std::path::Path) -> String {
    format!(
        r#"You are the interim CEO/CTO of this codebase at `{target_repo}`. The board has asked you to prepare a detailed 180-day turnaround plan.

Your mission: explore this codebase thoroughly, understand what exists, what's missing, and what's broken, then produce a comprehensive plan corpus.

## Process

1. **Explore the codebase**: Read Cargo.toml/package.json/pyproject.toml, src/ structure, existing docs (README, any specs), git log (recent 50 commits), test coverage, CI config, dependency graph. Use the plan-ceo-skill for strategic scope review, plan-eng-skill for architecture lockdown, and plan-design-skill for UX/design review. Spawn an agent team to review the codebase in parallel.

2. **Assess the state**: What works? What's broken? What's half-built? What's the tech debt? What are the security risks? What's the test coverage? Who are the users?

3. **Write the plan corpus** to the `genesis/` directory:

   a. `genesis/SPEC.md` — Project specification:
      - What this project is and does
      - Who it's for (target users/operators)
      - Architectural overview (major components, data flow, deployment model)
      - Key design decisions already made
      - Technology stack and dependencies

   b. `genesis/PLANS.md` — Planning conventions:
      - ExecPlan format (Progress, Surprises & Discoveries, Decision Log, Outcomes & Retrospective)
      - Milestone structure
      - Proof command conventions for this project
      - How plans reference each other

   c. `genesis/plans/001-master-plan.md` — 180-day turnaround roadmap:
      - Phase 0 (days 1-30): Stabilization — fix critical bugs, add missing tests, document what exists
      - Phase 1 (days 31-90): Foundation — establish shared patterns, build core abstractions
      - Phase 2 (days 91-150): Growth — implement major features, expand test coverage
      - Phase 3 (days 151-180): Polish — optimize performance, improve UX, prepare for release
      - Each phase lists the numbered plans it depends on

   d. `genesis/plans/002-*.md` through `genesis/plans/N-*.md` — one ExecPlan per major work stream:
      - Each plan has: Purpose, Progress (milestones with `- [ ]`), Decision Log, proof commands
      - Plans should cover: existing tech debt, missing tests, broken features, new features, infrastructure, documentation
      - Each plan should name specific files, crates, modules, functions — be concrete, not generic
      - Proof commands should be real commands that work in this repo (cargo test, npm test, pytest, etc.)

## Rules

- Be specific to THIS codebase. Don't write generic plans.
- Reference actual file paths, module names, function names you found during exploration.
- Plans must have concrete milestones with proof commands, not vague goals.
- The master plan should have 10-20 numbered plan references.
- Each numbered plan should have 3-8 milestones.
- Use the ExecPlan format from PLANS.md for every plan.
- Write all files using the Write tool. Do NOT output content to stdout.

Begin by exploring the codebase, then write the genesis documents."#,
        target_repo = target_repo.display(),
    )
}

pub fn import_command(args: &SynthImportArgs) -> anyhow::Result<()> {
    let blueprint = import_existing_package(ImportRequest {
        target_repo: &args.target_repo,
        program: &args.program,
    })?;
    save_blueprint(&args.output, &blueprint)?;

    println!("Program: {}", blueprint.program.id);
    println!("Mode: import");
    println!("Blueprint: {}", args.output.display());
    Ok(())
}

pub fn create_command(args: &SynthCreateArgs) -> anyhow::Result<()> {
    let planning_root = normalize_planning_root(&args.target_repo, args.planning_root.as_deref())?;
    let (blueprint, blueprint_path, notes) = if let Some(path) = &args.blueprint {
        (load_blueprint(path)?, path.clone(), Vec::new())
    } else {
        let authored = author_blueprint_for_create_with_planning_root(
            &args.target_repo,
            args.program.as_deref(),
            Some(&planning_root),
        )?;
        let path = args.output_blueprint.clone().unwrap_or_else(|| {
            default_blueprint_path(&args.target_repo, &authored.blueprint.program.id)
        });
        save_blueprint(&path, &authored.blueprint)?;
        (authored.blueprint, path, authored.notes)
    };
    if args.blueprint.is_some()
        && (blueprint_path.starts_with(
            args.target_repo
                .join(fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR),
        ) || !blueprint_path.exists())
    {
        save_blueprint(&blueprint_path, &blueprint)?;
    }
    if !args.no_decompose {
        let decomposed = run_opus_decomposition(&args.target_repo, &planning_root)?;
        println!(
            "Opus decomposition: decomposed {} composite plan(s)",
            decomposed.refreshed_paths.len()
        );

        // Write heuristic mappings only for plans Opus didn't decompose
        let _ = write_plan_mapping_snapshots(
            &args.target_repo,
            &planning_root,
            Some(&decomposed.refreshed_paths),
            Some(&decomposed.expected_paths),
        )?;

        // Re-author blueprint consuming the Opus-written mapping contracts
        let re_authored = author_blueprint_for_create_with_planning_root(
            &args.target_repo,
            Some(&blueprint.program.id),
            Some(&planning_root),
        )?;
        save_blueprint(&blueprint_path, &re_authored.blueprint)?;
        let _ = write_plan_mapping_snapshots(
            &args.target_repo,
            &planning_root,
            Some(&decomposed.refreshed_paths),
            Some(&decomposed.expected_paths),
        )?;
        let report = render_blueprint(RenderRequest {
            blueprint: &re_authored.blueprint,
            target_repo: &args.target_repo,
        })?;

        println!("Program: {}", re_authored.blueprint.program.id);
        println!("Mode: create (Opus decomposition)");
        println!("Blueprint: {}", blueprint_path.display());
        println!("Written files:");
        for path in report.written_files {
            println!("  {}", path.display());
        }
        return Ok(());
    }

    // Fallback: deterministic heuristics only (--no-decompose)
    let written_mapping_files =
        write_plan_mapping_snapshots(&args.target_repo, &planning_root, None, None)?;
    let report = render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: &args.target_repo,
    })?;

    println!("Program: {}", blueprint.program.id);
    println!("Mode: create (deterministic only)");
    println!("Blueprint: {}", blueprint_path.display());
    if !notes.is_empty() {
        println!("Notes:");
        for note in notes {
            println!("  - {note}");
        }
    }
    println!("Written files:");
    for path in report
        .written_files
        .into_iter()
        .chain(written_mapping_files.into_iter())
    {
        println!("  {}", path.display());
    }
    Ok(())
}

pub fn evolve_command(args: &SynthEvolveArgs) -> anyhow::Result<()> {
    let (blueprint, blueprint_path, notes) = if let Some(path) = &args.blueprint {
        (load_blueprint(path)?, path.clone(), Vec::new())
    } else {
        let authored = author_blueprint_for_evolve(&args.target_repo, args.program.as_deref())?;
        let path = args.output_blueprint.clone().unwrap_or_else(|| {
            default_blueprint_path(&args.target_repo, &authored.blueprint.program.id)
        });
        save_blueprint(&path, &authored.blueprint)?;
        (authored.blueprint, path, authored.notes)
    };
    let output_repo = args.preview_root.as_ref().unwrap_or(&args.target_repo);
    let report = reconcile_blueprint(ReconcileRequest {
        blueprint: &blueprint,
        current_repo: &args.target_repo,
        output_repo,
    })?;

    println!("Program: {}", blueprint.program.id);
    println!("Mode: evolve");
    println!("Blueprint: {}", blueprint_path.display());
    if args.preview_root.is_some() {
        println!("Preview root: {}", output_repo.display());
    }
    if !notes.is_empty() {
        println!("Notes:");
        for note in notes {
            println!("  - {note}");
        }
    }
    println!("Findings:");
    for finding in report.findings {
        println!("  - {finding}");
    }
    if !report.recommendations.is_empty() {
        println!("Recommendations:");
        for recommendation in report.recommendations {
            println!("  - {recommendation}");
        }
    }
    println!("Written files:");
    for path in report.written_files {
        println!("  {}", path.display());
    }
    Ok(())
}

fn default_blueprint_path(target_repo: &std::path::Path, program: &str) -> PathBuf {
    target_repo
        .join(fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR)
        .join("blueprints")
        .join(format!("{program}.yaml"))
}

fn normalize_planning_root(
    target_repo: &std::path::Path,
    planning_root: Option<&std::path::Path>,
) -> anyhow::Result<PathBuf> {
    let Some(root) = planning_root else {
        return Ok(PathBuf::new());
    };
    if root.is_absolute() {
        let relative = root.strip_prefix(target_repo).map_err(|_| {
            anyhow::anyhow!(
                "planning root {} must live under target repo {}",
                root.display(),
                target_repo.display()
            )
        })?;
        return Ok(relative.to_path_buf());
    }
    Ok(root.to_path_buf())
}

#[derive(Debug, Serialize)]
struct MappingSnapshot {
    mapping_source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    generated_by_run: Option<String>,
    title: String,
    category: String,
    composite: bool,
    bootstrap_required: bool,
    implementation_required: bool,
    dependency_plan_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<MappingChildSnapshot>,
}

#[derive(Debug, Serialize)]
struct MappingChildSnapshot {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    archetype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lane_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    review_profile: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    proof_commands: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    owned_surfaces: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct MappingMetadata {
    #[serde(default)]
    mapping_source: Option<String>,
    #[serde(default)]
    generated_by_run: Option<String>,
}

#[derive(Debug, Default)]
struct OpusDecompositionReport {
    refreshed_paths: BTreeSet<PathBuf>,
    expected_paths: BTreeSet<PathBuf>,
}

fn write_plan_mapping_snapshots(
    target_repo: &std::path::Path,
    planning_root: &std::path::Path,
    refreshed_opus_paths: Option<&BTreeSet<PathBuf>>,
    expected_opus_paths: Option<&BTreeSet<PathBuf>>,
) -> anyhow::Result<Vec<PathBuf>> {
    let registry = load_plan_registry_from_planning_root(target_repo, planning_root)?;
    let mut written = Vec::new();
    for plan in registry.plans {
        let relative_path = mapping_snapshot_path(&plan);
        let absolute_path = target_repo.join(&relative_path);
        if absolute_path.exists() {
            let metadata = load_mapping_metadata(&absolute_path).unwrap_or_default();
            let is_non_heuristic = metadata
                .mapping_source
                .as_deref()
                .is_some_and(|source| source != "heuristic");
            let is_expected_current_opus =
                expected_opus_paths.is_some_and(|paths| paths.contains(&relative_path));
            let is_fresh_current_opus =
                refreshed_opus_paths.is_some_and(|paths| paths.contains(&relative_path));
            if is_non_heuristic && (!is_expected_current_opus || is_fresh_current_opus) {
                written.push(absolute_path);
                continue;
            }
        }
        let children = if !plan.children.is_empty() {
            plan.children
                .iter()
                .map(|child| MappingChildSnapshot {
                    id: child.child_id.clone(),
                    title: child.title.clone(),
                    archetype: child.archetype.map(|a| a.as_str().to_string()),
                    lane_kind: child.lane_kind.map(|kind| kind.to_string()),
                    review_profile: child.review_profile.map(|r| r.as_str().to_string()),
                    proof_commands: child.proof_commands.clone(),
                    owned_surfaces: child.owned_surfaces.clone(),
                })
                .collect()
        } else if plan.composite {
            infer_child_snapshots(target_repo, &plan)
        } else {
            Vec::new()
        };
        let snapshot = MappingSnapshot {
            mapping_source: "heuristic".to_string(),
            generated_by_run: None,
            title: plan.title.clone(),
            category: plan.category.as_str().to_string(),
            composite: plan.composite,
            bootstrap_required: plan.bootstrap_required,
            implementation_required: plan.implementation_required,
            dependency_plan_ids: plan.dependency_plan_ids.clone(),
            children,
        };
        let yaml = serde_yaml::to_string(&snapshot)?;
        let trimmed = yaml.trim_start_matches("---\n");
        fabro_workflows::write_text_atomic(&absolute_path, trimmed, "plan mapping")
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        written.push(absolute_path);
    }
    Ok(written)
}

fn infer_child_snapshots(
    target_repo: &std::path::Path,
    plan: &PlanRecord,
) -> Vec<MappingChildSnapshot> {
    let plan_body = target_repo
        .join(&plan.path)
        .to_str()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default();
    let lower = plan_body.to_ascii_lowercase();

    plan.declared_child_ids
        .iter()
        .map(|child_id| {
            let archetype = infer_archetype_from_child_id(child_id, &lower);
            let review_profile = infer_review_profile(child_id, &lower);
            let proof_commands = infer_proof_commands(child_id, &plan_body);
            let owned_surfaces = infer_owned_surfaces(child_id, &plan_body);

            MappingChildSnapshot {
                id: child_id.clone(),
                title: None,
                archetype: Some(archetype.to_string()),
                lane_kind: Some(infer_lane_kind_from_child_id(child_id).to_string()),
                review_profile: Some(review_profile.to_string()),
                proof_commands,
                owned_surfaces,
            }
        })
        .collect()
}

fn load_mapping_metadata(path: &std::path::Path) -> Option<MappingMetadata> {
    let raw = std::fs::read_to_string(path).ok()?;
    if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        return serde_json::from_str(&raw).ok();
    }
    serde_yaml::from_str(&raw).ok()
}

fn infer_archetype_from_child_id(child_id: &str, plan_lower: &str) -> &'static str {
    let id_lower = child_id.to_ascii_lowercase();
    if id_lower.contains("e2e")
        || id_lower.contains("end-to-end")
        || id_lower.contains("integration")
    {
        return "integration_only";
    }
    if id_lower.contains("acceptance")
        || id_lower.contains("balance")
        || id_lower.contains("edge-case")
        || id_lower.contains("monte-carlo")
    {
        return "acceptance_and_balance";
    }
    if id_lower.contains("verification")
        || id_lower.contains("verify")
        || id_lower.contains("provably-fair")
    {
        return "verification_only";
    }
    if id_lower.contains("house")
        || id_lower.contains("agent")
        || id_lower.contains("server")
        || id_lower.contains("service")
        || id_lower.contains("session-handler")
    {
        return "service_surface";
    }
    if id_lower.contains("tui")
        || id_lower.contains("screen")
        || id_lower.contains("client")
        || id_lower.contains("terminal")
    {
        return "tui_surface";
    }
    if id_lower.contains("migration") {
        return "migration";
    }
    if id_lower.contains("orchestrat") {
        return "orchestration_program";
    }
    if id_lower.contains("report") || id_lower.contains("review-only") {
        return "review_or_report_only";
    }
    // Multi-surface detection from plan text
    if plan_lower.contains("cross-surface") || plan_lower.contains("cross surface") {
        return "implement_cross_surface";
    }
    "implement_module"
}

fn infer_review_profile(child_id: &str, plan_lower: &str) -> &'static str {
    let id_lower = child_id.to_ascii_lowercase();
    if id_lower.contains("provably-fair")
        || id_lower.contains("verification")
        || id_lower.contains("verify")
        || plan_lower.contains("security")
    {
        return "security_sensitive";
    }
    if id_lower.contains("casino-core")
        || id_lower.contains("settlement")
        || id_lower.contains("payout")
        || id_lower.contains("balance")
        || id_lower.contains("acceptance")
    {
        return "economic_correctness";
    }
    if id_lower.contains("tui") || id_lower.contains("screen") || id_lower.contains("client") {
        return "user_visible";
    }
    if id_lower.contains("house")
        || id_lower.contains("agent")
        || id_lower.contains("server")
        || id_lower.contains("service")
    {
        return "production_service";
    }
    if id_lower.contains("migration") {
        return "migration_risky";
    }
    if id_lower.contains("foundation")
        || id_lower.contains("core")
        || id_lower.contains("trait")
        || id_lower.contains("shared")
    {
        return "shared_foundation";
    }
    "standard"
}

fn infer_lane_kind_from_child_id(child_id: &str) -> &'static str {
    let id_lower = child_id.to_ascii_lowercase();
    if id_lower.contains("integration")
        || id_lower.contains("e2e")
        || id_lower.contains("end-to-end")
    {
        return "integration";
    }
    if id_lower.contains("orchestrat") {
        return "orchestration";
    }
    if id_lower.contains("report") || id_lower.contains("review-only") {
        return "artifact";
    }
    if id_lower.contains("house")
        || id_lower.contains("agent")
        || id_lower.contains("server")
        || id_lower.contains("service")
        || id_lower.contains("daemon")
        || id_lower.contains("worker")
        || id_lower.contains("handler")
        || id_lower.contains("session")
        || id_lower.contains("rpc")
        || id_lower.contains("api")
        || id_lower.contains("websocket")
    {
        return "service";
    }
    if id_lower.contains("tui")
        || id_lower.contains("screen")
        || id_lower.contains("client")
        || id_lower.contains("terminal")
        || id_lower.contains("frontend")
        || id_lower.contains("web-ui")
        || id_lower.contains("mobile")
        || id_lower.contains("shell")
        || id_lower.contains("cli")
        || id_lower.contains("dashboard")
        || id_lower.contains("widget")
    {
        return "interface";
    }
    "platform"
}

fn infer_proof_commands(child_id: &str, plan_body: &str) -> Vec<String> {
    let mut commands = Vec::new();

    // Look for cargo test/build commands that reference surfaces matching this child
    let child_parts: Vec<&str> = child_id.split('-').collect();
    for line in plan_body.lines() {
        let trimmed = line.trim().trim_start_matches("- ");
        let trimmed = trimmed.trim_start_matches('`').trim_end_matches('`');
        if !trimmed.starts_with("cargo ") {
            continue;
        }
        let line_lower = trimmed.to_ascii_lowercase();
        let is_relevant = child_parts
            .iter()
            .any(|part| part.len() >= 3 && line_lower.contains(part));
        if is_relevant {
            commands.push(trimmed.to_string());
        }
    }
    commands.sort();
    commands.dedup();
    commands
}

fn infer_owned_surfaces(child_id: &str, plan_body: &str) -> Vec<String> {
    let mut surfaces = Vec::new();
    let child_parts: Vec<&str> = child_id.split('-').collect();

    for line in plan_body.lines() {
        let trimmed = line.trim();
        // Look for path references like `crates/casino-core/src/craps/`
        for segment in trimmed.split('`') {
            let seg = segment.trim();
            if !seg.starts_with("crates/") && !seg.starts_with("bin/") && !seg.starts_with("src/") {
                continue;
            }
            let seg_lower = seg.to_ascii_lowercase();
            let is_relevant = child_parts
                .iter()
                .any(|part| part.len() >= 3 && seg_lower.contains(part));
            if is_relevant {
                surfaces.push(seg.to_string());
            }
        }
    }
    surfaces.sort();
    surfaces.dedup();
    surfaces
}

const DECOMPOSE_MODEL: &str = "claude-opus-4-6";

fn run_opus_decomposition(
    target_repo: &std::path::Path,
    planning_root: &std::path::Path,
) -> anyhow::Result<OpusDecompositionReport> {
    let registry = load_plan_registry_from_planning_root(target_repo, planning_root)?;
    let composite_plans: Vec<_> = registry
        .plans
        .iter()
        .filter(|plan| {
            plan.composite
                && plan.declared_child_ids.len() > 1
                && plan.category != raspberry_supervisor::PlanCategory::Meta
        })
        .collect();

    if composite_plans.is_empty() {
        return Ok(OpusDecompositionReport::default());
    }

    let claude_check = std::process::Command::new("claude")
        .arg("--version")
        .output();
    if claude_check.is_err() || !claude_check.as_ref().unwrap().status.success() {
        anyhow::bail!(
            "claude CLI not found; required for decomposition. \
             Use --no-decompose for heuristic-only mode, or install: \
             npm install -g @anthropic-ai/claude-code"
        );
    }

    let mappings_dir = target_repo
        .join(fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR)
        .join("plan-mappings");
    std::fs::create_dir_all(&mappings_dir)?;

    let plan_manifest = composite_plans
        .iter()
        .map(|plan| {
            format!(
                "- plan_id: {}\n  path: {}\n  category: {}\n  dependency_plan_ids: [{}]\n  output: {}/plan-mappings/{}.yaml",
                plan.plan_id,
                plan.path.display(),
                plan.category.as_str(),
                plan.dependency_plan_ids.join(", "),
                fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR,
                plan.path.file_stem().and_then(|s| s.to_str()).unwrap_or("plan"),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let run_id = format!(
        "opus-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0)
    );
    let prompt = build_batch_decomposition_prompt(&plan_manifest, target_repo, &run_id);
    let prompt_file = tempfile::NamedTempFile::new()?;
    std::fs::write(prompt_file.path(), &prompt)?;

    let count = composite_plans.len();
    let expected_paths = composite_plans
        .iter()
        .map(|plan| mapping_snapshot_path(plan))
        .collect::<BTreeSet<_>>();
    println!(
        "Decomposing {count} composite plans with {} (parallel agent team) ...",
        DECOMPOSE_MODEL
    );

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cat {} | CLAUDECODE= claude -p --output-format text --dangerously-skip-permissions --model {} --max-turns 200",
            prompt_file.path().display(),
            DECOMPOSE_MODEL,
        ))
        .current_dir(target_repo)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Opus decomposition failed: {}", stderr.trim());
    }

    // Count only mappings refreshed by this invocation.
    let mut refreshed_paths = BTreeSet::new();
    for plan in &composite_plans {
        let mapping_path = mapping_snapshot_path(plan);
        let absolute_path = target_repo.join(&mapping_path);
        if let Some(metadata) = load_mapping_metadata(&absolute_path) {
            if metadata.mapping_source.as_deref() == Some("opus")
                && metadata.generated_by_run.as_deref() == Some(run_id.as_str())
            {
                refreshed_paths.insert(mapping_path);
            }
        }
    }

    println!(
        "  Decomposed {}/{count} composite plans",
        refreshed_paths.len()
    );
    Ok(OpusDecompositionReport {
        refreshed_paths,
        expected_paths,
    })
}

fn build_batch_decomposition_prompt(
    plan_manifest: &str,
    target_repo: &std::path::Path,
    run_id: &str,
) -> String {
    format!(
        r#"You are a plan decomposition engine for a Rust workspace project.

Your working directory is `{target_repo}`. You have {plan_count} composite plans to decompose into YAML mapping contracts. Process them ALL — spawn parallel agents to handle them concurrently.

## Plans to decompose

{plan_manifest}

## For each plan

1. Read the plan file from the `path` listed above
2. Decompose it into a YAML mapping contract
3. Write the YAML to the `output` path listed above

## YAML mapping contract format

Each mapping contract must contain:

```yaml
mapping_source: opus
generated_by_run: "{run_id}"
title: "from the plan's H1 heading"
category: one of meta, foundation, game, interface, service, infrastructure, verification, economic
composite: true
bootstrap_required: true
implementation_required: true
dependency_plan_ids: [kebab-case IDs from "depends on:" in plan text]
children:
  - id: concise-kebab-case (2-4 words, e.g., casino-core, provably-fair, house-handler)
    title: human-readable milestone name
    archetype: one of implement, integration, orchestration, report
    lane_kind: one of platform, service, interface, artifact, integration, orchestration
    review_profile: one of standard, foundation, hardened, ux
    proof_commands: [exact cargo test/build commands from plan text ONLY]
    owned_surfaces: [repo-relative paths from plan]
    where_surfaces: one-line summary
    how_description: one-line behavior change description
    required_tests: concrete test commands
    verification_plan: what proves this child is done
    rollback_condition: what reopens this child
```

## Critical rules

1. One child per milestone in the plan's Progress section. Do NOT duplicate or split.
2. Do NOT invent proof commands — only use commands that appear verbatim in the plan.
3. Child IDs must be concise. Bad: `craps-game-engine-state-machine-30-bet-types`. Good: `casino-core`.
4. Archetype: almost everything is `implement`. Only use `integration` for e2e/system tests, `orchestration` for meta-work spawning child programs, `report` for non-code artifacts.
5. Lane kind: `service` for daemons, APIs, agents, handlers, and anything with health/operator surfaces. `interface` for TUI/web/mobile/CLI user-facing work. `platform` for libraries and core modules.
6. Review profile: `standard` for normal code. `foundation` for shared types/traits/SDK that downstream work depends on. `hardened` for security, crypto, financial logic, correctness-critical invariants — anything where bugs are catastrophic. `ux` for user-facing surfaces (TUI, web, mobile, CLI).
7. Write each YAML file directly using the Write tool. Do NOT output YAML to stdout.

Process all {plan_count} plans now. Use parallel agents for speed."#,
        target_repo = target_repo.display(),
        run_id = run_id,
        plan_count = plan_manifest
            .lines()
            .filter(|l| l.starts_with("- plan_id:"))
            .count(),
    )
}

fn mapping_snapshot_path(plan: &PlanRecord) -> PathBuf {
    if let Some(existing) = &plan.mapping_contract_path {
        return existing.clone();
    }
    let stem = plan
        .path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("plan");
    PathBuf::from(fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR)
        .join("plan-mappings")
        .join(format!("{stem}.yaml"))
}
