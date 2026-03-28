use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use clap::{Args, Subcommand};
use fabro_model::{automation_chain, AutomationProfile, Provider};
use fabro_synthesis::{
    author_blueprint_for_create_with_planning_root, author_blueprint_for_evolve,
    import_existing_package, load_blueprint, reconcile_blueprint, render_blueprint, save_blueprint,
    ImportRequest, ReconcileRequest, RenderRequest,
};
use fabro_workflows::backend::{parse_cli_response, select_automation_codex_home};
use raspberry_supervisor::{
    load_plan_registry, load_plan_registry_from_planning_root,
    load_plan_registry_relaxed_from_planning_root, PlanRecord, ProgramRuntimeState,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Subcommand)]
pub enum SynthCommand {
    /// Import an existing Fabro workflow package into a blueprint file
    Import(SynthImportArgs),
    /// Create a checked-in Fabro workflow package from a blueprint
    Create(SynthCreateArgs),
    /// Steer the active malinka program from genesis + code + recent runtime evidence
    Evolve(SynthEvolveArgs),
    /// Eng-review mapping contracts with Opus 4.6 adversarial pass
    Review(SynthReviewArgs),
    /// Generate SPEC.md, PLANS.md, and numbered plans for an unfamiliar codebase, then run synth create
    Genesis(SynthGenesisArgs),
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn automation_cli_command(
    provider: Provider,
    model: &str,
    prompt_file: &Path,
    max_turns: usize,
    anthropic_text: bool,
) -> String {
    let prompt_file = shell_quote(&prompt_file.display().to_string());
    match provider {
        Provider::Anthropic => {
            let output_format = if anthropic_text { "text" } else { "stream-json" };
            format!(
                "cat {prompt_file} | CLAUDECODE= claude -p --output-format {output_format} --dangerously-skip-permissions --model {model} --max-turns {max_turns}"
            )
        }
        Provider::OpenAi
        | Provider::Kimi
        | Provider::Zai
        | Provider::Inception
        | Provider::OpenAiCompatible => format!(
            "cat {prompt_file} | codex exec --json --yolo -m {model}"
        ),
        Provider::Minimax => format!(
            "prompt=\"$(cat {prompt_file})\" && pi --provider minimax --mode json -p --no-session --no-extensions --no-skills --no-prompt-templates --tools read,bash,edit,write,grep,find,ls --model {model} --thinking high \"$prompt\""
        ),
        Provider::Gemini => format!("cat {prompt_file} | gemini -o json --yolo -m {model}"),
    }
}

fn run_automation_chain(
    profile: AutomationProfile,
    prompt_file: &Path,
    cwd: &Path,
    max_turns: usize,
    anthropic_text: bool,
) -> anyhow::Result<String> {
    let mut failures = Vec::new();
    for target in automation_chain(profile) {
        let mut command = std::process::Command::new("sh");
        command.arg("-c").arg(automation_cli_command(
            target.provider,
            target.model,
            prompt_file,
            max_turns,
            anthropic_text,
        ));
        command.current_dir(cwd);
        if target.provider == Provider::OpenAi {
            if let Some(codex_home) = select_automation_codex_home() {
                command.env("CODEX_HOME", codex_home);
            }
        }
        let output = command.output()?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let text = if target.provider == Provider::Anthropic && anthropic_text {
                stdout.trim().to_string()
            } else {
                parse_cli_response(target.provider, &stdout)
                    .map(|response| response.text)
                    .unwrap_or_else(|| stdout.to_string())
                    .trim()
                    .to_string()
            };
            if !text.is_empty() {
                return Ok(text);
            }
            failures.push(format!(
                "{}:{} returned success with empty output",
                target.provider.as_str(),
                target.model
            ));
            continue;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let parsed = if target.provider == Provider::Anthropic {
            None
        } else {
            parse_cli_response(target.provider, &stdout)
                .map(|response| response.text.trim().to_string())
                .filter(|text| !text.is_empty())
        };
        let detail = parsed
            .or_else(|| {
                let stderr = stderr.trim();
                (!stderr.is_empty()).then(|| stderr.to_string())
            })
            .or_else(|| {
                let stdout = stdout.trim();
                (!stdout.is_empty()).then(|| stdout.to_string())
            })
            .unwrap_or_else(|| "no stderr/stdout".to_string());
        failures.push(format!(
            "{}:{} failed: {}",
            target.provider.as_str(),
            target.model,
            detail
        ));
    }

    anyhow::bail!("all automation providers failed:\n{}", failures.join("\n"))
}

fn automation_profile_name(profile: AutomationProfile) -> &'static str {
    match profile {
        AutomationProfile::Write => "write",
        AutomationProfile::Review => "review",
        AutomationProfile::Synth => "synth",
        AutomationProfile::Unblock => "unblock",
        AutomationProfile::DeepReview => "deep_review",
        AutomationProfile::Adjudication => "adjudication",
    }
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
    /// Skip the eng-review pass after decomposition
    #[arg(long)]
    pub no_review: bool,
    #[arg(long, hide = true)]
    pub planning_root: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct SynthEvolveArgs {
    #[arg(long, hide = true)]
    pub blueprint: Option<PathBuf>,
    #[arg(long)]
    pub target_repo: PathBuf,
    #[arg(long, hide = true)]
    pub preview_root: Option<PathBuf>,
    #[arg(long)]
    pub program: Option<String>,
    #[arg(long, hide = true)]
    pub output_blueprint: Option<PathBuf>,
    /// Skip the Opus steering review and only emit a deterministic report
    #[arg(long)]
    pub no_review: bool,
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
        .filter(|plan| plan.composite && plan.category != raspberry_supervisor::PlanCategory::Meta)
        .collect();

    if composite_plans.is_empty() {
        println!("No composite plans to review.");
        return Ok(());
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
        "Eng-reviewing {} mapping contracts with {} profile ...",
        composite_plans.len(),
        automation_profile_name(AutomationProfile::Review)
    );
    let review_output = run_automation_chain(
        AutomationProfile::Review,
        prompt_file.path(),
        &args.target_repo,
        50,
        true,
    )?;

    let report_path = mappings_dir.join("review-report.md");
    if report_path.exists() {
        println!("Review report written to: {}", report_path.display());
    } else {
        println!("Review completed (report may be in stdout)");
        if !review_output.trim().is_empty() {
            println!("{}", review_output);
        }
    }

    Ok(())
}

fn run_decomposition_review(target_repo: &std::path::Path) -> anyhow::Result<()> {
    let registry = load_plan_registry(target_repo)?;
    let mappings_dir = target_repo
        .join(fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR)
        .join("plan-mappings");

    if !mappings_dir.is_dir() {
        return Ok(());
    }

    let composite_plans: Vec<_> = registry
        .plans
        .iter()
        .filter(|plan| {
            plan.composite
                && !plan.children.is_empty()
                && plan.category != raspberry_supervisor::PlanCategory::Meta
        })
        .collect();

    if composite_plans.is_empty() {
        return Ok(());
    }

    let mut contracts = Vec::new();
    for plan in &composite_plans {
        let mapping_path = mapping_snapshot_path(plan);
        let absolute = target_repo.join(&mapping_path);
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

    if contracts.is_empty() {
        return Ok(());
    }

    let review_prompt = format!(
        r#"You are an adversarial eng-review agent. Your job is to review how plans were decomposed into workflow mapping contracts, and fix any problems you find.

Working directory: `{target_repo}`

## Mapping contracts to review

{contracts}

## Review criteria

For EACH mapping contract, read the actual plan file and check:

### Structural integrity
1. **Milestone-to-child parity**: Does each child map to exactly one milestone in the plan's Progress section? Flag over-splits (one milestone became two children) and missing milestones (milestone has no child).
2. **Child ID quality**: IDs should be concise (2-4 words, kebab-case). Flag verbose IDs.
3. **Dependency ordering**: If two plans' children claim overlapping owned_surfaces, they need explicit dependency ordering or a shared foundation plan.

### Archetype and profile accuracy
4. **Archetype**: Almost everything is `implement`. Only `integration` for e2e/system tests. Only `orchestration` for meta-work spawning child programs. Only `report` for non-code artifacts. Flag misassignments.
5. **Lane kind**: `service` for daemons/APIs/agents/handlers. `interface` for TUI/web/mobile/CLI. `platform` for libraries/core modules. `artifact` for documentation. Flag service/UI work flattened to platform.
6. **Review profile**: `standard` for normal code. `foundation` for shared types/traits/SDK that downstream work depends on. `hardened` for security, crypto, financial logic, correctness-critical invariants. `ux` for user-facing surfaces. Flag misassignments — especially `hardened` being over-assigned to non-critical code.

### Proof and surface quality
7. **Proof commands**: Must appear verbatim in the plan text. Flag invented commands. Prefer specific test targets (`cargo test -p crate test_name`) over broad ones (`cargo test`).
8. **Owned surfaces**: Must be precise repo-relative paths from the plan. Flag vague paths, invented paths, or paths that don't exist in the repo.
9. **Cross-plan surface conflicts**: Do children from different plans claim the same owned surfaces? This is only valid with explicit dependency ordering.

### Completeness
10. **AC contract fields**: Every child should have where_surfaces, how_description, verification_plan, rollback_condition. Flag missing fields.
11. **Failure mode**: Does the plan's Decision Log include at least one failure scenario? If not, flag it.

### Engineering heuristics
12. **Complexity smell**: If a single plan produced more than 8 children, challenge whether it should be split into multiple plans.
13. **Boring by default**: Flag children that introduce novel infrastructure without justification. Default to proven technology.
14. **Blast radius**: Flag children whose owned_surfaces span multiple unrelated crates or modules — they may be too broad.

## Action

For each finding:
- If you can fix it (wrong archetype, missing field, bad ID), rewrite the mapping contract YAML directly using the Write tool.
- If it requires human judgment (plan should be split, milestone is vague), note it in the review report.

Write a structured review report to `{pkg_dir}/plan-mappings/review-report.md` with:
- Summary: N contracts reviewed, N findings, N auto-fixed
- Per-plan findings (cite specific child IDs)
- Items needing human attention

Be adversarial. The goal is to catch decomposition mistakes before they become workflow bugs."#,
        target_repo = target_repo.display(),
        contracts = contracts.join("\n---\n\n"),
        pkg_dir = fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR,
    );

    let prompt_file = tempfile::NamedTempFile::new()?;
    std::fs::write(prompt_file.path(), &review_prompt)?;

    println!(
        "Eng-reviewing {} decomposition contracts with {} profile ...",
        composite_plans.len(),
        automation_profile_name(AutomationProfile::Review)
    );
    if let Err(err) = run_automation_chain(
        AutomationProfile::Review,
        prompt_file.path(),
        target_repo,
        50,
        false,
    ) {
        eprintln!("  [review] review process failed; continuing with current mappings: {err}");
    }

    let report_path = mappings_dir.join("review-report.md");
    if report_path.exists() {
        println!("Decomposition review report: {}", report_path.display());
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
    println!(
        "{} profile is exploring the codebase and drafting a 180-day turnaround plan.",
        automation_profile_name(AutomationProfile::Synth)
    );
    println!();
    run_automation_chain(
        AutomationProfile::Synth,
        prompt_file.path(),
        &args.target_repo,
        200,
        false,
    )?;

    // Verify outputs were written
    let spec_path = genesis_dir.join("SPEC.md");
    let plans_md = genesis_dir.join("PLANS.md");
    let assessment_path = genesis_dir.join("ASSESSMENT.md");
    let report_path = genesis_dir.join("GENESIS-REPORT.md");
    let design_path = genesis_dir.join("DESIGN.md");
    let plans_dir = genesis_dir.join("plans");

    let plan_count = std::fs::read_dir(&plans_dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("md"))
                .count()
        })
        .unwrap_or(0);

    let check = |path: &std::path::Path| {
        if path.exists() {
            "written"
        } else {
            "missing"
        }
    };

    println!("Genesis complete:");
    println!("  ASSESSMENT.md: {}", check(&assessment_path));
    println!("  SPEC.md: {}", check(&spec_path));
    println!("  PLANS.md: {}", check(&plans_md));
    println!("  Plans: {plan_count} files in genesis/plans/");
    if design_path.exists() {
        println!("  DESIGN.md: written");
    }
    println!("  GENESIS-REPORT.md: {}", check(&report_path));

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
            no_review: false,
            planning_root: Some(PathBuf::from("genesis")),
        };
        create_command(&create_args)?;
    }

    Ok(())
}

fn build_genesis_prompt(target_repo: &std::path::Path) -> String {
    format!(
        r#"You are the interim CEO/CTO of this codebase at `{target_repo}`. The board has asked you to prepare a detailed 180-day turnaround plan.

Run this as a full sprint: Think → Plan → Build → Review → Verify. Each phase feeds the next. Write all output files using the Write tool, never to stdout.

# Phase 1: THINK (Deep Codebase Review)

You MUST perform a deep review of the actual source code, not just documentation and plans. Reading plans without reading the code they describe leads to an assessment anchored on claims rather than reality. Your job is to verify what actually exists.

## 1a. Source Code Review (MANDATORY — do this FIRST)

Spawn a parallel agent team to conduct a deep codebase review. Each agent applies a specific review methodology non-interactively (no user questions — just findings). Each agent should read actual source files, produce structured findings, and report back.

**Agent 1: Engineering Review (Eng Manager lens)**
Apply the plan-eng-review methodology to the entire codebase:
- Read the main entry points: processor, instruction dispatch, state definitions, binary entry points
- Map the architecture: draw the component dependency graph, data flows (all four paths: happy, nil, empty, error)
- For each major codepath, produce a failure mode entry: `CODEPATH | FAILURE MODE | HANDLED? | TEST? | USER SEES | LOGGED?` — any row with HANDLED=N + TEST=N + USER SEES=Silent = CRITICAL GAP
- Produce an error & rescue map: for every module that can fail, what exceptions exist, are they rescued, what does the user see?
- Look for: unwrap() calls, unsafe blocks, TODO/FIXME, dead code, DRY violations, cyclomatic complexity >5
- Assess coupling: which components are tightly coupled? Which boundaries are clean?
- Performance: N+1 queries, unbounded collections, missing indexes, connection pool pressure
- Observability: are codepaths logged? Can you reconstruct a bug from logs alone?
- Deployment: migration safety, rollback posture, feature flags, what breaks when old+new code run simultaneously?

**Agent 2: Security & Trust Boundary Review (Staff Engineer lens)**
Apply the /review security methodology to the entire codebase:
- For every API route and endpoint: who can call it, what auth is required, what data can they access?
- Trace trust boundaries end-to-end: where does user input enter, how is it validated, where could it escape?
- Check for: hardcoded secrets, missing auth checks, privilege escalation, replay attacks
- Check for: injection (SQL, command, template, prompt), XSS, CSRF, timing attacks in financial logic
- Read deployment scripts: plain-text credentials, root-privilege services, SSH key management, upgrade authority
- For each finding: threat, likelihood (H/M/L), impact (H/M/L), mitigated?

**Agent 3: Frontend & Design Review (Designer lens)**
Apply the plan-design-review methodology to existing UI surfaces:
- Read the app router / page structure and identify all user-facing routes
- Read 5-10 key hooks and components that drive core functionality
- Rate each dimension 0-10: information architecture, interaction state coverage (loading/empty/error/success/partial for every feature), user journey coherence, AI slop risk, responsive intent, accessibility
- Produce an interaction state coverage table: `FEATURE | LOADING | EMPTY | ERROR | SUCCESS | PARTIAL` — flag missing states
- Check AI slop: generic card grids? Hero sections? "Clean modern UI"? Dashboard widgets that look like every SaaS template?
- If DESIGN.md exists, check whether implementation follows it — flag drift
- Check accessibility: keyboard nav, screen reader support, contrast, 44px+ touch targets

**Agent 4: Test & CI Coverage Review (QA Engineer lens)**
Apply QA-level rigor to the test infrastructure:
- Read CI configuration and every test command in package.json / Cargo.toml
- Read 3-5 test files per major component to assess test QUALITY (not just count)
- For each major feature claimed in docs, check: does a test actually verify this behavior?
- Produce a test coverage map: `FEATURE | UNIT? | INTEGRATION? | E2E? | EDGE CASES?`
- Look for: mocked-everything tests, tests that can't fail, green-on-failure patterns
- Check for chaos/failure tests: what happens when external services fail? Network errors? Timeouts?
- Verify CI pipeline: does it run all tests? Any disabled/skipped suites? Flaky test patterns?

**Agent 5: CEO Review — Ops, Evidence & Strategic Context**
Apply the plan-ceo-review methodology to the operational surface:
- Read git log for last 100 commits — identify patterns, who's active, what's changing
- Read all doctrine files: GOAL.md, AGENTS.md, README.md, DEPLOYMENT.md, INVARIANTS.md, DESIGN.md, CLAUDE.md
- Read EVERY file in `plans/` and `specs/` — for each plan with completed milestones [x], those represent WORK ALREADY DONE
- Read EVERY file in `genesis/` if it exists — understand what was planned AND executed in prior genesis runs
- If proof/evidence artifacts exist (bundles, reports, drill results), read them
- Read deployment and ops scripts, systemd units, Docker config — understand operational topology
- Apply premise challenge: is this the right product? What would happen if it did nothing for 6 months?
- Apply dream state mapping: what does the ideal state look like in 12 months?
- Apply inversion reflex: what would make this project fail?
- Produce an implementation status table: for each previously claimed capability, verify against code reality using "verified" / "partially verified" / "stub only" / "not found"

## 1b. Documentation and Plans Review

- Read ALL doctrine files: README, GOAL.md, AGENTS.md, SPEC.md, INVARIANTS.md, DESIGN.md, CLAUDE.md
- Read EVERY file in `plans/` and `specs/` directories
- Read EVERY file in `genesis/` if it exists (this may be a previous genesis run — understand what was planned AND what was executed)
- For each plan: check its Progress section for completed `[x]` milestones. Plans with completed milestones represent WORK ALREADY DONE, not just intentions.

## 1c. Git Metrics & Runtime Evidence (Retro lens)

Extract structured metrics from git history the way an engineering retro would:

- Run `git log --oneline -100` and `git shortlog -sn --since="3 months ago"` — produce a contributor breakdown: who committed, how many, what areas
- Identify commit patterns: fix-to-feature ratio (`git log --oneline | grep -ci "fix"` vs total), test ratio (commits touching test files vs total), agent vs human commits
- Detect hotspot files: `git log --format=format: --name-only -100 | sort | uniq -c | sort -rn | head -20` — which files change most often? Hotspots are where bugs live.
- Detect shipping velocity: commits per week over the last month, any stalls or bursts
- Check for operational evidence: proof artifacts, deployment logs, health check results, drill outputs
- Check for environment files (.env.example, .env.local patterns) to understand deployment topology
- Produce a metrics summary for the assessment:
```
METRIC                    | VALUE
Commits (90 days)         | N
Contributors              | N (list)
Fix ratio                 | N% (fixes / total)
Test ratio                | N% (test-touching / total)
Agent commits             | N%
Hotspot files             | top 5
Shipping velocity         | commits/week trend
```

## 1d. Cross-Reference Claims Against Code

This is the critical step most assessments skip. For each major claim in the docs:
- If GOAL.md says "zero-edge" → read the actual rebate/dividend code and verify
- If a plan says "settlement proven" → read the settlement tests and check they pass meaningful assertions
- If docs say "gasless betting works" → read the gasless API route and verify the flow end-to-end in code
- If a plan is marked complete → verify the surfaces it claims to own actually exist and work

Do NOT trust documentation over code. If docs say X is implemented but the code shows stubs, the assessment must say "claimed but not verified" or "stub only."

## 1e. Documentation Staleness Audit (document-release lens)

For every `.md` file in the repo root (README.md, GOAL.md, AGENTS.md, DEPLOYMENT.md, SPEC.md, INVARIANTS.md, CLAUDE.md, DESIGN.md, etc.):

1. Read the doc file and identify the features, components, workflows, and file paths it describes.
2. For each reference, check whether the referenced file/feature/path still exists in the current working tree.
3. Produce a staleness table:
```
DOC FILE     | REFERENCE              | STATUS
README.md    | genesis/plans/001...   | STALE — file deleted
GOAL.md      | plans/a2-devnet-...    | CURRENT — file exists
AGENTS.md    | services/risk/...      | STALE — empty shell, not running service
```
4. Flag any doc that references deleted files, renamed paths, superseded plans, or capabilities that code review found to be stubs.
5. Flag any doc that hasn't been updated in >30 days but describes code that changed recently (compare doc mtime vs code mtime in the areas it describes).

This audit feeds directly into the assessment's tech debt inventory and into Plan 002 (or equivalent) for reference integrity fixes.

## 1f. Write Assessment

**The Reframe (do this BEFORE the forcing questions):**

Before answering any questions, challenge the product's self-description. Read how the project describes itself in README, GOAL.md, and SPEC.md. Then ask: is that actually what this codebase IS, based on what you found in the source code? The most valuable insight from an office-hours session is the reframe — "you said X, but what you actually built is Y."

Write the reframe as the opening of ASSESSMENT.md:
- "The project describes itself as: [quote from docs]"
- "Based on code review, what it actually is: [your honest assessment]"
- If they match, say so. If they don't, explain the gap — this gap is the single most important strategic finding.

**Six Forcing Questions:**

Answer these six forcing questions (write answers to `genesis/ASSESSMENT.md`):

1. **Demand reality**: Who uses this? What specific behavior proves real demand — payments, daily usage, panic if it vanished? Or is it a side project, prototype, abandoned experiment?
2. **Status quo**: What's the current workflow without this project? What pain does it solve? What duct-tape alternatives exist?
3. **Desperate specificity**: Who is the ONE person this is for? What's their title, what keeps them up at night, what gets them promoted? Not "developers" — a real role with real constraints.
4. **Narrowest wedge**: What's the smallest thing this codebase does that someone would pay for or depend on THIS WEEK? Strip away the vision — what's the kernel of value?
5. **Observation & surprise**: What surprised you during exploration? What behavior exists that the original author probably didn't intend? What's half-built in a revealing way?
6. **Future-fit**: How does this project compound as the world changes? Or does it decay? What bet is it making?

## ASSESSMENT.md structure

Write `genesis/ASSESSMENT.md` with these sections in order:

1. **The Reframe** — project's self-description vs. code reality
2. **The ONE sentence** that captures what this project actually is
3. **Six Forcing Questions** — demand, status quo, specificity, wedge, surprise, future-fit
4. **Source code findings** — for each major component, what you found by reading the actual code (not just what docs say). Include specific file paths, line counts, patterns observed.
5. **What works** (verified by reading code), **what's broken** (verified by reading code), **what's half-built** (stubs found, partial implementations)
6. **Tech debt inventory** with file paths and line numbers where possible
7. **Security risks found** from actual code review, not just docs
8. **Test coverage gaps** with specific untested modules — verified by reading test files
9. **Git metrics summary** — contributor breakdown, fix ratio, test ratio, agent vs human, hotspot files, velocity trend (from section 1c)
10. **Documentation staleness table** — every stale reference, deleted file, or drift found (from section 1e)
11. **Implementation status** — for each previously completed plan or claimed capability, state whether the code backs up the claim. Use "verified", "partially verified", "stub only", or "not found" for each.
12. **Existing plan assessment** — for each plan in `plans/`, rate it and state what genesis will do with it
13. **Code review coverage** — list the key source files you actually read, grouped by component (proves the assessment is grounded in code)

# Phase 2: PLAN (Full Review Stack)

Apply the full review stack to the codebase — not just to plans, but to the entire codebase as assessed in Phase 1. Each review lens produces findings that shape the plan corpus in Phase 3.

## 2a. CEO Review (Strategic Lens)

Think like a CEO doing a turnaround review. Apply these cognitive patterns:

**Premise challenge**: Is this the right product to build? Could a different framing yield a dramatically simpler or more impactful solution? What would happen if the project did nothing for 6 months?

**Dream state mapping**: Describe the ideal end state 12 months from now. For every plan you will write, check whether it moves toward or away from that state:
```
CURRENT STATE  →  THIS PLAN  →  12-MONTH IDEAL
```

**Inversion reflex**: For every goal, also ask "what would make this fail?" Put failure scenarios in every plan's Decision Log.

**Focus as subtraction**: 3-8 milestones per plan. A plan with 15 milestones is trying to do too much — split it. The primary value-add is deciding what NOT to do.

**Implementation alternatives**: For each major plan, identify 2-3 distinct approaches. One must be minimal viable, one must be ideal architecture. Name which you chose and why in the Decision Log.

**Don't restart completed work**: If source code review confirms a capability is already implemented and tested, the plan should build on it, not re-implement it. Mark already-proven milestones as complete with evidence.

**Reversibility**: Prefer plans that can be rolled back. Flag one-way doors explicitly. Rate each plan 1-5 on reversibility.

**Narrowest wedge first**: Phase 0 plans should deliver value in 30 days, not prepare for value in 90.

## 2b. Engineering Review (Eng Manager Lens)

Review the codebase the way a rigorous engineering manager would before approving a plan. For every plan with technical content:

**Scope challenge**:
- What existing code already partially or fully solves each sub-problem? Map every planned feature to existing code.
- If the plan touches >8 files or introduces >2 new abstractions, challenge whether a simpler approach exists.
- Verify before planning: grep for the function/file/test each milestone targets. If it already exists, mark the milestone as pre-satisfied.

**Architecture review**:
- ASCII architecture diagram for every plan touching 3+ modules. No exceptions.
- Map data flows with all four paths: happy path, nil input, empty input, upstream error.
- Identify coupling: which components become coupled that weren't before?
- Scaling: what breaks first under 10x load?

**Error & failure mode analysis**:
For each major codepath in the codebase, identify realistic production failures. Include a failure mode table in each plan:
```
CODEPATH | FAILURE MODE | HANDLED? | TEST? | USER SEES | LOGGED?
```
Any codepath with HANDLED=N, TEST=N, USER SEES=Silent → mark as **CRITICAL GAP**.

**Security & trust boundary review**:
- New attack vectors: endpoints, params, file paths, background jobs
- Auth: is every data access scoped to the right user/role?
- Secrets: in env vars, not hardcoded? Rotatable?
- Injection: SQL, command, template, LLM prompt injection

**Test quality** (not just count):
- For each planned feature, does a meaningful test actually exist?
- Diagram all new UX flows, data flows, codepaths, branching outcomes. For each, verify test coverage.
- Flag: mocked-everything tests, green-on-failure patterns, tests that can't fail.

**Proof command quality**: Every milestone needs a specific proof command. `cargo test -p {{crate}} -- {{test_name}}` beats `cargo test`. Content assertions beat `test -f`.

**Observability**: New codepaths need structured logging, metrics, or traces. If a bug is reported 3 weeks post-ship, can you reconstruct what happened from logs alone?

**Deployment safety**: For each plan, what's the rollback procedure? Feature flags? Migration safety? What happens when old code and new code run simultaneously?

## 2c. Design Review (Designer Lens)

For any plan touching user-facing surfaces, review with a designer's eye. Rate each dimension 0-10.

**Information architecture**: What does the user see first, second, third? Include ASCII mockup of screen hierarchy. Apply constraint worship — if you can only show 3 things, which 3 matter most?

**Interaction state coverage**: For every UI feature, fill in this table:
```
FEATURE      | LOADING | EMPTY | ERROR | SUCCESS | PARTIAL
```
Empty states are features. "No items found." is not a design. Every empty state needs warmth, a primary action, and context.

**User journey**: Storyboard the emotional arc — 5-sec visceral, 5-min behavioral, long-term reflective. What does the user feel at each step?

**AI slop detection**: Flag generic patterns that could be any SaaS template. "Cards with icons" → what differentiates these? "Clean modern UI" → meaningless, replace with actual design decisions. "Dashboard with widgets" → what makes this NOT every other dashboard?

**DESIGN.md alignment**: If a design system exists, calibrate every UI decision against it. If not, flag the gap.

**Responsive intent**: Specific layout changes per viewport, not just "stacks on mobile." Each breakpoint gets intentional design.

**Accessibility as scope**: Keyboard nav, screen readers, contrast, 44px+ touch targets — these are deliverables, not polish. Specify them in the plan or they won't exist.

If the project has user-facing surfaces and no existing design system, write `genesis/DESIGN.md` with:
- Aesthetic direction and rationale
- Typography: display, body, UI, code fonts with modular scale
- Color palette: primary, secondary, neutrals, semantic (error/warning/success/info)
- Spacing scale with base unit
- Layout approach per breakpoint

# Phase 3: BUILD (Write the Plan Corpus)

Write to the `genesis/` directory:

a. `genesis/SPEC.md` — What this project is, who it's for, architecture (with ASCII diagram), tech stack, key decisions already made. Write this based on the format of SPEC.md from the Root Directory.

b. `genesis/PLANS.md` — ExecPlan conventions (Progress, Surprises & Discoveries, Decision Log, Outcomes & Retrospective), milestone structure, proof command conventions. Copy this file from the Root Directory.

c. `genesis/plans/001-master-plan.md` — 180-day turnaround roadmap:
   - Phase 0 (days 1-30): Stabilization — critical bugs, missing tests, documentation
   - Phase 1 (days 31-90): Foundation — shared patterns, core abstractions
   - Phase 2 (days 91-150): Growth — major features, expanded test coverage
   - Phase 3 (days 151-180): Polish — performance, UX, release prep
   - Each phase lists numbered plan dependencies

d. `genesis/plans/002-*.md` through `genesis/plans/N-*.md` — one ExecPlan per work stream.

Each plan MUST follow this full skeleton (not a subset — every section is required):

```
# NNN — <Short, action-oriented title>

## Purpose / Big Picture
2-4 sentences: what someone gains after this change and how they can see it working.
State the user-visible behavior this plan enables.

## Context and Orientation
Describe the CURRENT state relevant to this plan. Assume the reader is a complete
beginner who has only the repo and this file. Name key files and modules by FULL
repo-relative path. Define any terms of art. This section should be 10-30 lines
minimum — enough that a new contributor can understand where they are starting from.

## Architecture
ASCII diagram showing the components this plan touches and their relationships.
Required for any plan touching 3+ modules. Include data flow direction arrows.

## Progress
- [x] (pre-satisfied) M1. Title — cite evidence from Phase 1 code review
- [ ] M2. Title
  - Surfaces: `path/a`, `path/b` — specific files this milestone owns
  - What exists after: concrete description of the deliverable
  - Why now: one sentence on why this milestone matters for the plan
  - Proof: `specific command that proves this milestone is done`
  - Tests: `specific test command that catches regressions`
- [ ] M3. Title
  ...

## Surprises & Discoveries
- Observation: [something non-obvious found during Phase 1 code review]
  Evidence: [file path, line number, or command output]

## Decision Log
- Decision: [concrete choice made]
  - Why: [rationale]
  - Failure mode: [what goes wrong if this decision is wrong]
  - Mitigation: [how the plan limits damage]
  - Reversible: [yes/no and how]
At least 2 entries per plan. At least one must be a failure scenario.

## Validation and Acceptance
Describe how to exercise the system after all milestones complete.
Include specific commands to run and what output to expect.
This is the "done" definition for the entire plan.

## Outcomes & Retrospective
_Updated after milestones complete._
```

**Plan depth requirements:**
- Each plan should be 80-200 lines. Plans under 50 lines are too thin — they lack the context a new contributor needs. Plans over 300 lines should be split.
- The **Context and Orientation** section is the most commonly skipped — and the most important. A plan that says "fix the settlement worker" without explaining what the settlement worker is, where it lives, how it works, and what state it's currently in is useless to anyone but the original author.
- Each milestone's **Surfaces** field must name specific repo-relative file paths, not vague descriptions like "the frontend" or "the test suite."
- Each milestone's **Proof** command must be copy-pasteable and specific. `cargo test` is not acceptable. `cargo test -p rsociety-executor -- settlement::drain_correctness` is.
- Cover: tech debt, missing tests, broken features, new features, infrastructure, docs
- 3-8 milestones per plan

**Carry-forward rule**: Every existing plan in `plans/` must appear in `genesis/plans/`. For plans you assessed as strong, copy them into genesis with the same filename and number. For plans you enhanced, write the enhanced version. For plans you split or merged, write the new plans and note the provenance in the Decision Log. No existing plan should silently disappear — if you're dropping one, write a short `genesis/plans/NNN-dropped-*.md` explaining why.

**Implementation-ready rule**: If an existing or genesis plan already names owned surfaces, concrete proof commands, and explicit validation or acceptance criteria, preserve it as implementation-ready. Do NOT rewrite it into a bootstrap-only plan whose only deliverables are `spec.md` and `review.md`. Bootstrap is only for plans that are still strategy-heavy and need a narrower executable slice first.

**Pre-satisfied milestone rule**: If source code review confirmed that a milestone's target already exists and works, mark it `[x]` in the Progress section with a note like "pre-satisfied: verified in Phase 1 code review" and cite the file path. Do not create milestones for work that is already done.

# Phase 4: REVIEW (Self-Review Pass)

After writing all plans, review the corpus against these checklists:

## Depth review (check FIRST — thin plans are the most common failure)
- [ ] Every plan has a Context and Orientation section of at least 10 lines
- [ ] Every plan is at least 80 lines total (if under 50, it is too thin — add context, file paths, architecture)
- [ ] Every milestone names specific file paths in its Surfaces field
- [ ] Every Decision Log has at least 2 entries including one failure scenario
- [ ] No milestone uses vague proof commands like `cargo test` or `npm test` — must be specific

## Structural review
- [ ] Every plan references specific file paths, not vague module descriptions
- [ ] Every milestone has a proof command that would actually work in this repo
- [ ] No two plans claim the same files without explicit dependency ordering
- [ ] Master plan references all numbered plans
- [ ] Each numbered plan has 3-8 milestones (not more, not fewer)
- [ ] Each numbered plan has a Surprises & Discoveries section

## Completeness review
- [ ] Tech debt identified during exploration is covered by at least one plan
- [ ] Untested modules identified during exploration have test plans
- [ ] Broken features identified during exploration have fix plans
- [ ] If UI surfaces exist: design plan covers states, accessibility, responsive
- [ ] Every plan has at least one failure scenario in its Decision Log

## Adversarial review
- [ ] Read each plan as a skeptical engineer: what's the first thing that would go wrong?
- [ ] Check for plans that are secretly huge — >8 files touched = split it
- [ ] Check for plans that depend on unstated assumptions — make them explicit
- [ ] Check for vague milestones: "set up infrastructure" → what specifically?

## Reality-check review
- [ ] No plan proposes re-implementing something the code review found already working
- [ ] No plan treats a completed capability as unstarted
- [ ] Claims in the assessment are backed by specific file paths from code review
- [ ] Pre-satisfied milestones are correctly marked `[x]`

If any check fails, fix the plan before moving on. Do not write a plan you wouldn't approve as a reviewer.

# Phase 5: VERIFY

Write `genesis/GENESIS-REPORT.md` summarizing:
- Total plans generated
- Assessment highlights (from Phase 1)
- Plans needing human attention (flagged during review)
- Known gaps (things you couldn't assess without running the code)
- Recommended next steps for the operator
- **Code review coverage**: List the key source files you actually read during Phase 1, grouped by component. This proves the assessment is grounded in code, not just docs.

## Rules

- Be specific to THIS codebase. Don't write generic plans.
- Reference actual file paths, module names, function names from exploration.
- Include ASCII diagrams for architecture, data flow, and state machines.
- Write all files using the Write tool. Do NOT output content to stdout.
- 10-20 numbered plans in the master plan. 3-8 milestones per plan.
- Use the ExecPlan format from PLANS.md for every plan.
- NEVER assess a component without reading its source code first.
- NEVER claim something is "broken" or "a stub" without verifying by reading the file.
- NEVER plan to implement something that already works — mark it pre-satisfied instead.

Begin with Phase 1: explore the codebase by reading actual source files, then write ASSESSMENT.md."#,
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
        (load_blueprint(path)?, path.clone(), Vec::<String>::new())
    } else {
        let program_id = args
            .program
            .clone()
            .unwrap_or_else(|| infer_program_id_from_repo_name(&args.target_repo));
        let blueprint_path = args
            .output_blueprint
            .clone()
            .unwrap_or_else(|| default_blueprint_path(&args.target_repo, &program_id));

        if !args.no_decompose {
            let decomposed = run_opus_decomposition(&args.target_repo, &planning_root)?;
            println!(
                "Opus decomposition: decomposed {} composite plan(s)",
                decomposed.refreshed_paths.len()
            );

            let _ = write_plan_mapping_snapshots(
                &args.target_repo,
                &planning_root,
                Some(&decomposed.refreshed_paths),
                Some(&decomposed.expected_paths),
            )?;
            validate_plan_mapping_snapshots(&args.target_repo, &planning_root)?;

            let mut authored = author_blueprint_for_create_with_planning_root(
                &args.target_repo,
                Some(&program_id),
                Some(&planning_root),
            )?;
            save_blueprint(&blueprint_path, &authored.blueprint)?;

            let _ = write_plan_mapping_snapshots(
                &args.target_repo,
                &planning_root,
                Some(&decomposed.refreshed_paths),
                Some(&decomposed.expected_paths),
            )?;
            validate_plan_mapping_snapshots(&args.target_repo, &planning_root)?;

            if !args.no_review {
                run_decomposition_review(&args.target_repo)?;
                validate_plan_mapping_snapshots(&args.target_repo, &planning_root)?;
                authored = author_blueprint_for_create_with_planning_root(
                    &args.target_repo,
                    Some(&program_id),
                    Some(&planning_root),
                )?;
                save_blueprint(&blueprint_path, &authored.blueprint)?;
            }

            let report = render_blueprint(RenderRequest {
                blueprint: &authored.blueprint,
                target_repo: &args.target_repo,
            })?;

            println!("Program: {}", authored.blueprint.program.id);
            println!("Mode: create (Opus decomposition)");
            println!("Blueprint: {}", blueprint_path.display());
            println!("Written files:");
            for path in report.written_files {
                println!("  {}", path.display());
            }
            return Ok(());
        }

        let written_mapping_files =
            write_plan_mapping_snapshots(&args.target_repo, &planning_root, None, None)?;
        validate_plan_mapping_snapshots(&args.target_repo, &planning_root)?;
        let authored = author_blueprint_for_create_with_planning_root(
            &args.target_repo,
            Some(&program_id),
            Some(&planning_root),
        )?;
        save_blueprint(&blueprint_path, &authored.blueprint)?;
        let report = render_blueprint(RenderRequest {
            blueprint: &authored.blueprint,
            target_repo: &args.target_repo,
        })?;

        println!("Program: {}", authored.blueprint.program.id);
        println!("Mode: create (deterministic only)");
        println!("Blueprint: {}", blueprint_path.display());
        if !authored.notes.is_empty() {
            println!("Notes:");
            for note in authored.notes {
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
        return Ok(());
    };
    if args.blueprint.is_some()
        && (blueprint_path.starts_with(
            args.target_repo
                .join(fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR),
        ) || !blueprint_path.exists())
    {
        save_blueprint(&blueprint_path, &blueprint)?;
    }
    if args.blueprint.is_some() {
        let report = render_blueprint(RenderRequest {
            blueprint: &blueprint,
            target_repo: &args.target_repo,
        })?;
        println!("Program: {}", blueprint.program.id);
        println!("Mode: create (existing blueprint)");
        println!("Blueprint: {}", blueprint_path.display());
        println!("Written files:");
        for path in report.written_files {
            println!("  {}", path.display());
        }
        return Ok(());
    }
    println!("Program: {}", blueprint.program.id);
    println!("Mode: create (existing blueprint)");
    println!("Blueprint: {}", blueprint_path.display());
    if !notes.is_empty() {
        println!("Notes:");
        for note in notes {
            println!("  - {note}");
        }
    }
    Ok(())
}

fn infer_program_id_from_repo_name(target_repo: &std::path::Path) -> String {
    let repo_name = target_repo
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");
    repo_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn evolve_command(args: &SynthEvolveArgs) -> anyhow::Result<()> {
    let output_repo = args.preview_root.as_ref().unwrap_or(&args.target_repo);
    let program = resolve_existing_program_id(&args.target_repo, args.program.as_deref())?;
    if args.preview_root.is_some() {
        seed_preview_root_from_current_package(&args.target_repo, output_repo, &program)?;
    }
    let manifest_path = default_program_manifest_path(output_repo, &program);
    if !manifest_path.exists() {
        anyhow::bail!(
            "program manifest {} does not exist; run `fabro synth create` first",
            manifest_path.display()
        );
    }

    let report_path = default_steering_report_path(output_repo, &program);
    let recent_outputs =
        collect_recent_output_files(&args.target_repo, Duration::from_secs(6 * 60 * 60), 24)?;
    let recent_output_lines = recent_outputs
        .iter()
        .map(|path| format!("- {}", path.display()))
        .collect::<Vec<_>>();
    let runtime_summary = summarize_runtime_state(&args.target_repo, &program)?;
    let autodev_summary = summarize_autodev_report(&args.target_repo, &program)?;

    if args.no_review {
        let authored = author_blueprint_for_evolve(&args.target_repo, Some(&program))?;
        let reconcile_report = reconcile_blueprint(ReconcileRequest {
            blueprint: &authored.blueprint,
            current_repo: &args.target_repo,
            output_repo,
        })?;
        write_deterministic_steering_report(
            &program,
            &manifest_path,
            &report_path,
            &recent_output_lines,
            &runtime_summary,
            &autodev_summary,
            &reconcile_report.findings,
            &reconcile_report.recommendations,
            &reconcile_report.written_files,
        )?;
        println!("Program: {program}");
        println!("Mode: evolve (deterministic reconcile)");
        if args.preview_root.is_some() {
            println!("Preview root: {}", output_repo.display());
        }
        println!("Report: {}", report_path.display());
        println!("Runtime summary: {runtime_summary}");
        println!("Autodev summary: {autodev_summary}");
        println!("Written files:");
        if reconcile_report.written_files.is_empty() {
            println!("  {}", report_path.display());
        } else {
            for path in &reconcile_report.written_files {
                println!("  {}", path.display());
            }
        }
        return Ok(());
    }

    let prompt = build_steering_prompt(
        &args.target_repo,
        output_repo,
        &program,
        &manifest_path,
        &report_path,
        &recent_output_lines,
        &runtime_summary,
        &autodev_summary,
    );
    let prompt_file = tempfile::NamedTempFile::new()?;
    std::fs::write(prompt_file.path(), &prompt)?;

    run_automation_chain(
        AutomationProfile::Synth,
        prompt_file.path(),
        &args.target_repo,
        80,
        true,
    )?;

    let written_files = collect_malinka_written_files(output_repo)?;
    println!("Program: {program}");
    println!(
        "Mode: evolve ({} steering)",
        automation_profile_name(AutomationProfile::Synth)
    );
    println!("Report: {}", report_path.display());
    if args.preview_root.is_some() {
        println!("Preview root: {}", output_repo.display());
    }
    println!("Runtime summary: {runtime_summary}");
    println!("Autodev summary: {autodev_summary}");
    if !recent_output_lines.is_empty() {
        println!("Recent outputs:");
        for line in &recent_output_lines {
            println!("  {line}");
        }
    }
    println!("Written files:");
    if written_files.is_empty() {
        println!("  {}", report_path.display());
    } else {
        for path in written_files {
            println!("  {}", path.display());
        }
    }
    Ok(())
}

const DEFAULT_STEERING_LOOKBACK_HOURS: u64 = 6;

fn seed_preview_root_from_current_package(
    target_repo: &std::path::Path,
    output_repo: &std::path::Path,
    program: &str,
) -> anyhow::Result<()> {
    let target_manifest = default_program_manifest_path(target_repo, program);
    let output_manifest = default_program_manifest_path(output_repo, program);
    if output_manifest.exists() || !target_manifest.exists() {
        return Ok(());
    }
    let source_root = target_repo.join(fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR);
    let destination_root = output_repo.join(fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR);
    copy_directory_recursive(&source_root, &destination_root)
}

fn copy_directory_recursive(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> anyhow::Result<()> {
    if !source.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            copy_directory_recursive(&source_path, &destination_path)?;
        } else {
            if let Some(parent) = destination_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn default_program_manifest_path(target_repo: &std::path::Path, program: &str) -> PathBuf {
    target_repo
        .join(fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR)
        .join("programs")
        .join(format!("{program}.yaml"))
}

fn default_steering_report_path(target_repo: &std::path::Path, program: &str) -> PathBuf {
    target_repo
        .join(fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR)
        .join("steering")
        .join(format!("{program}.md"))
}

fn collect_recent_output_files(
    target_repo: &std::path::Path,
    lookback: Duration,
    limit: usize,
) -> anyhow::Result<Vec<PathBuf>> {
    let root = target_repo.join("outputs");
    if !root.exists() {
        return Ok(Vec::new());
    }
    let cutoff = SystemTime::now()
        .checked_sub(lookback)
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut files = Vec::new();
    let mut stack = vec![root];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            if modified < cutoff {
                continue;
            }
            files.push((modified, path));
        }
    }
    files.sort_by(|left, right| right.0.cmp(&left.0));
    Ok(files
        .into_iter()
        .take(limit)
        .map(|(_, path)| {
            path.strip_prefix(target_repo)
                .map(PathBuf::from)
                .unwrap_or(path)
        })
        .collect())
}

fn summarize_runtime_state(target_repo: &std::path::Path, program: &str) -> anyhow::Result<String> {
    let path = target_repo
        .join(".raspberry")
        .join(format!("{program}-state.json"));
    let Some(state) = ProgramRuntimeState::load_optional(&path)
        .map_err(|error| anyhow::anyhow!(error.to_string()))?
    else {
        return Ok("no runtime state found".to_string());
    };
    let mut blocked = 0usize;
    let mut complete = 0usize;
    let mut failed = 0usize;
    let mut ready = 0usize;
    let mut running = 0usize;
    let mut running_details = Vec::new();
    let mut failed_details = Vec::new();
    for (lane_key, lane) in &state.lanes {
        match lane.status.to_string().as_str() {
            "blocked" => blocked += 1,
            "complete" => complete += 1,
            "failed" => {
                failed += 1;
                failed_details.push(format!(
                    "{lane_key}@{}",
                    lane.failure_kind
                        .map(|kind| kind.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ));
            }
            "ready" => ready += 1,
            "running" => {
                running += 1;
                running_details.push(format!(
                    "{lane_key}@{}",
                    lane.current_stage_label
                        .clone()
                        .unwrap_or_else(|| "active".to_string())
                ));
            }
            _ => {}
        }
    }
    running_details.sort();
    failed_details.sort();
    Ok(format!(
        "counts: complete={complete} ready={ready} running={running} blocked={blocked} failed={failed}; running=[{}]; failed=[{}]",
        running_details.join(", "),
        failed_details.join(", "),
    ))
}

fn summarize_autodev_report(
    target_repo: &std::path::Path,
    program: &str,
) -> anyhow::Result<String> {
    let path = target_repo
        .join(".raspberry")
        .join(format!("{program}-autodev.json"));
    if !path.exists() {
        return Ok("no autodev report found".to_string());
    }
    let value = serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(&path)?)
        .unwrap_or_else(|_| serde_json::json!({}));
    let cycles = value
        .get("cycles")
        .and_then(|cycles| cycles.as_array())
        .map(|cycles| cycles.len())
        .unwrap_or(0);
    let last_cycle = value
        .get("cycles")
        .and_then(|cycles| cycles.as_array())
        .and_then(|cycles| cycles.last())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    Ok(format!(
        "cycles={cycles}; last_cycle={}",
        serde_json::to_string(&last_cycle).unwrap_or_else(|_| "{}".to_string())
    ))
}

fn deterministic_steering_report(
    program: &str,
    manifest_path: &std::path::Path,
    report_path: &std::path::Path,
    recent_outputs: &[String],
    runtime_summary: &str,
    autodev_summary: &str,
    findings: &[String],
    recommendations: &[String],
    written_files: &[PathBuf],
) -> String {
    let findings = summarize_deterministic_findings(findings);
    let recommendations = summarize_deterministic_recommendations(recommendations);
    let mut body = String::new();
    body.push_str(&format!("# Steering Review for {program}\n\n"));
    body.push_str("## Verdict\n\n");
    body.push_str("Deterministic reconcile completed without automation review.\n\n");
    body.push_str("## Evidence\n\n");
    body.push_str(&format!(
        "- Program manifest: `{}`\n",
        manifest_path.display()
    ));
    body.push_str(&format!("- Report path: `{}`\n", report_path.display()));
    body.push_str(&format!("- Runtime summary: {runtime_summary}\n"));
    body.push_str(&format!("- Autodev summary: {autodev_summary}\n"));
    if !recent_outputs.is_empty() {
        body.push_str("- Recent outputs:\n");
        for line in recent_outputs {
            body.push_str(&format!("  - {line}\n"));
        }
    }
    body.push_str("\n## Changes Made\n\n");
    if written_files.is_empty() {
        body.push_str("- No malinka files changed during deterministic reconcile.\n");
    } else {
        for path in written_files {
            body.push_str(&format!("- `{}`\n", path.display()));
        }
    }
    body.push_str("\n## Findings\n\n");
    if findings.is_empty() {
        body.push_str("- No deterministic findings.\n");
    } else {
        for finding in findings {
            body.push_str(&format!("- {finding}\n"));
        }
    }
    body.push_str("\n## Recommendations\n\n");
    if recommendations.is_empty() {
        body.push_str("- No deterministic recommendations.\n");
    } else {
        for recommendation in recommendations {
            body.push_str(&format!("- {recommendation}\n"));
        }
    }
    body.push_str("\n## Why These Changes Preserve Genesis Strategy\n\n");
    body.push_str("- Deterministic reconcile rewrote malinka from the current package, repo doctrine, and runtime evidence without introducing LLM-only steering drift.\n");
    body.push_str("\n## Next Risks\n\n");
    body.push_str("- Deterministic reconcile skips the adversarial steering review pass, so nuanced reprioritization may still benefit from a reviewed evolve run.\n");
    body
}

fn summarize_deterministic_findings(findings: &[String]) -> Vec<String> {
    let mut doctrine_count = 0usize;
    let mut evidence_count = 0usize;
    let mut missing_artifacts = 0usize;
    let mut runtime_missing = 0usize;
    let mut ready_lanes = Vec::new();
    let mut other = Vec::new();

    for finding in findings {
        if finding.starts_with("doctrine input found:") {
            doctrine_count += 1;
            continue;
        }
        if finding.starts_with("evidence input found:") {
            evidence_count += 1;
            continue;
        }
        if finding.starts_with("artifact missing:") {
            missing_artifacts += 1;
            continue;
        }
        if finding.starts_with("runtime state missing:") {
            runtime_missing += 1;
            continue;
        }
        if let Some(lane) = finding
            .strip_prefix("lane `")
            .and_then(|rest| rest.strip_suffix("` appears ready for execution"))
        {
            ready_lanes.push(lane.to_string());
            continue;
        }
        other.push(finding.clone());
    }

    let mut summarized = Vec::new();
    summarized.extend(other.into_iter().take(12));
    let prioritized_ready = prioritize_lane_keys(ready_lanes);
    if !prioritized_ready.is_empty() {
        summarized.push(format!(
            "ready lanes (highest priority first): {}",
            summarize_lane_list(&prioritized_ready, 6)
        ));
    }
    if doctrine_count > 0 {
        summarized.push(format!("doctrine inputs attached: {doctrine_count}"));
    }
    if evidence_count > 0 {
        summarized.push(format!("evidence inputs attached: {evidence_count}"));
    }
    if runtime_missing > 0 {
        summarized.push("runtime state missing".to_string());
    }
    if missing_artifacts > 0 {
        summarized.push(format!("missing expected artifacts: {missing_artifacts}"));
    }
    summarized
}

fn summarize_deterministic_recommendations(recommendations: &[String]) -> Vec<String> {
    recommendations
        .iter()
        .map(|recommendation| {
            if let Some(rest) =
                recommendation.strip_prefix("execute the next ready bootstrap lane(s) first: ")
            {
                let lanes = rest.split(", ").map(str::to_string).collect::<Vec<_>>();
                let prioritized = prioritize_lane_keys(lanes);
                return format!(
                    "execute the next ready lanes first: {}",
                    summarize_lane_list(&prioritized, 6)
                );
            }
            if let Some(rest) = recommendation.strip_prefix(
                "leave the workflow package unchanged for now and execute the next ready lane(s): ",
            ) {
                let lanes = rest.split(", ").map(str::to_string).collect::<Vec<_>>();
                let prioritized = prioritize_lane_keys(lanes);
                return format!(
                    "leave the workflow package unchanged and execute: {}",
                    summarize_lane_list(&prioritized, 6)
                );
            }
            recommendation.clone()
        })
        .take(10)
        .collect()
}

fn lane_priority_score(lane: &str) -> i32 {
    let unit = lane.split(':').next().unwrap_or(lane);
    let mut score = 50;

    if unit == "master" {
        score -= 40;
    }
    if unit.starts_with("phase-") && unit.ends_with("-gate") {
        score -= 30;
    }
    if unit.contains("-parent-") {
        score -= 25;
    }
    if unit.contains("document") || unit.contains("release") || unit.ends_with("-retro") {
        score -= 15;
    }
    if unit.contains("benchmark") {
        score -= 10;
    }

    if unit.contains("autodev-efficiency")
        || unit.contains("greenfield-bootstrap")
        || unit.contains("provider-policy")
        || unit.contains("test-coverage")
    {
        score += 40;
    } else if unit.contains("error-handling") || unit.contains("workspace-integration") {
        score += 30;
    } else if unit.contains("sprint-contracts") || unit.contains("genesis-onboarding") {
        score += 20;
    }

    if lane.contains("live-validation") || lane.contains("fresh-install-test") {
        score += 10;
    }

    score
}

fn prioritize_lane_keys(mut lanes: Vec<String>) -> Vec<String> {
    lanes.sort_by(|left, right| {
        lane_priority_score(right)
            .cmp(&lane_priority_score(left))
            .then_with(|| left.cmp(right))
    });
    if lanes.iter().any(|lane| lane_priority_score(lane) >= 60) {
        lanes.retain(|lane| lane_priority_score(lane) >= 30);
    }
    lanes.dedup();
    lanes
}

fn summarize_lane_list(lanes: &[String], limit: usize) -> String {
    if lanes.len() <= limit {
        return lanes.join(", ");
    }
    let head = lanes.iter().take(limit).cloned().collect::<Vec<_>>();
    format!("{} (+{} more)", head.join(", "), lanes.len() - limit)
}

fn write_deterministic_steering_report(
    program: &str,
    manifest_path: &std::path::Path,
    report_path: &std::path::Path,
    recent_outputs: &[String],
    runtime_summary: &str,
    autodev_summary: &str,
    findings: &[String],
    recommendations: &[String],
    written_files: &[PathBuf],
) -> anyhow::Result<()> {
    let body = deterministic_steering_report(
        program,
        manifest_path,
        report_path,
        recent_outputs,
        runtime_summary,
        autodev_summary,
        findings,
        recommendations,
        written_files,
    );
    fabro_workflows::write_text_atomic(report_path, &body, "steering report")
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

#[allow(clippy::too_many_arguments)]
fn build_steering_prompt(
    target_repo: &std::path::Path,
    output_repo: &std::path::Path,
    program: &str,
    manifest_path: &std::path::Path,
    report_path: &std::path::Path,
    recent_outputs: &[String],
    runtime_summary: &str,
    autodev_summary: &str,
) -> String {
    let recent_outputs_block = if recent_outputs.is_empty() {
        "- none observed in the last 6 hours".to_string()
    } else {
        recent_outputs.join("\n")
    };
    format!(
        r#"You are the bounded steering layer for Fabro.

Working repo: `{target_repo}`
Write root: `{output_repo}`
Program: `{program}`
Lookback window: last {lookback_hours} hours

Strategic contract:
- `genesis/` is the long-horizon strategy and MUST NOT be edited.
- Source code outside `malinka/` is observational evidence only and MUST NOT be edited.
- You MAY edit only these paths under the write root:
  - `malinka/programs/{program}.yaml`
  - `{report_path}`

Your job is to reconcile three truths:
1. `genesis/` strategy
2. actual code/runtime evidence
3. current execution steer in `malinka/`

Do NOT run a broad genesis process. Do NOT rewrite the plan architecture wholesale. Do NOT regenerate prompts, mappings, workflows, run configs, blueprints, or paperclip assets. Prefer the smallest program-manifest-only steering change that improves the next 6 hours of autodev behavior.

Current live steer:
- Program manifest: `{manifest_path}`
- Runtime summary: {runtime_summary}
- Autodev summary: {autodev_summary}
- Recent outputs modified in the lookback window:
{recent_outputs_block}

Required workflow:
1. Inspect `genesis/`, the current codebase, `.raspberry/{program}-state.json`, `.raspberry/{program}-autodev.json`, and the current `malinka/` program steer.
2. Decide whether autodev is working well over the last {lookback_hours} hours.
3. If it is not working well, make bounded edits ONLY to `malinka/programs/{program}.yaml` to improve throughput and keep focus on the strategic fronts described in `genesis/`.
4. Always write a steering review report to `{report_path}`.

The steering report must contain these sections:
- `# Steering Review for {program}`
- `## Verdict`
- `## Evidence`
- `## Changes Made`
- `## Why These Changes Preserve Genesis Strategy`
- `## Next Risks`

Editing guidance:
- Restrict edits to `malinka/programs/{program}.yaml`.
- Do not touch plan mappings, prompts, run configs, workflows, paperclip config, or blueprints.
- Do not delete major frontiers unless the last {lookback_hours} hours of evidence clearly show they are off-strategy relative to `genesis/`.
- Bias toward improving focus and throughput, not rewriting intent.

When finished, ensure any changed files are saved only at `malinka/programs/{program}.yaml` and the report exists at `{report_path}`."#,
        target_repo = target_repo.display(),
        output_repo = output_repo.display(),
        program = program,
        manifest_path = manifest_path.display(),
        report_path = report_path.display(),
        runtime_summary = runtime_summary,
        autodev_summary = autodev_summary,
        recent_outputs_block = recent_outputs_block,
        lookback_hours = DEFAULT_STEERING_LOOKBACK_HOURS,
    )
}

fn collect_malinka_written_files(output_repo: &std::path::Path) -> anyhow::Result<Vec<PathBuf>> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(output_repo)
        .args(["status", "--short", "--", "malinka"])
        .output()?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut paths = text
        .lines()
        .filter_map(|line| line.get(3..).map(str::trim))
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn default_blueprint_path(target_repo: &std::path::Path, program: &str) -> PathBuf {
    target_repo
        .join(fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR)
        .join("blueprints")
        .join(format!("{program}.yaml"))
}

fn resolve_existing_program_id(
    target_repo: &std::path::Path,
    program_override: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(program) = program_override {
        return Ok(program.to_string());
    }
    let programs_dir = target_repo
        .join(fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR)
        .join("programs");
    if !programs_dir.is_dir() {
        anyhow::bail!(
            "no programs directory found at {}; run `fabro synth create` first",
            programs_dir.display()
        );
    }
    let mut programs = std::fs::read_dir(&programs_dir)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|ext| ext.to_str()) == Some("yaml")).then_some(path)
        })
        .collect::<Vec<_>>();
    programs.sort();
    let Some(path) = programs.first() else {
        anyhow::bail!(
            "no existing program manifests found in {}; pass --program explicitly",
            programs_dir.display()
        );
    };
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        anyhow::bail!("failed to derive program id from {}", path.display());
    };
    Ok(stem.to_string())
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
    plan_id: String,
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

#[derive(Debug, Default, Deserialize)]
struct MappingValidationContract {
    #[serde(default)]
    plan_id: Option<String>,
    #[serde(default)]
    dependency_plan_ids: Vec<String>,
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
    let registry = load_plan_registry_relaxed_from_planning_root(target_repo, planning_root)?;
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
            plan_id: plan.plan_id.clone(),
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

fn validate_plan_mapping_snapshots(
    target_repo: &std::path::Path,
    planning_root: &std::path::Path,
) -> anyhow::Result<()> {
    let registry = load_plan_registry_relaxed_from_planning_root(target_repo, planning_root)?;
    let known_plan_ids = registry
        .plans
        .iter()
        .map(|plan| plan.plan_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let mut errors = Vec::new();

    for plan in &registry.plans {
        let relative_path = mapping_snapshot_path(plan);
        let absolute_path = target_repo.join(&relative_path);
        if !absolute_path.exists() {
            continue;
        }
        let raw = std::fs::read_to_string(&absolute_path)?;
        let contract: MappingValidationContract = serde_yaml::from_str(&raw).map_err(|error| {
            anyhow::anyhow!("failed to parse {}: {error}", absolute_path.display())
        })?;

        match contract
            .plan_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(plan_id) if plan_id == plan.plan_id => {}
            Some(plan_id) => errors.push(format!(
                "{}: plan_id `{plan_id}` does not match filename-derived id `{}`",
                relative_path.display(),
                plan.plan_id
            )),
            None => errors.push(format!(
                "{}: missing required plan_id (expected `{}`)",
                relative_path.display(),
                plan.plan_id
            )),
        }

        let unknown_dependencies = contract
            .dependency_plan_ids
            .iter()
            .filter(|dependency| !known_plan_ids.contains(dependency.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if !unknown_dependencies.is_empty() {
            errors.push(format!(
                "{}: dependency_plan_ids must use exact known plan ids; unknown {:?}",
                relative_path.display(),
                unknown_dependencies
            ));
        }
    }

    if errors.is_empty() {
        return Ok(());
    }

    anyhow::bail!(
        "plan mapping validation failed:\n{}",
        errors
            .into_iter()
            .map(|error| format!("- {error}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
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
    if id_lower.contains("integration-test")
        || id_lower.contains("integration-tests")
        || id_lower.contains("e2e-test")
        || id_lower.contains("e2e-tests")
        || id_lower.contains("regression-test")
        || id_lower.contains("regression-tests")
    {
        return "implement_module";
    }
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
    if id_lower.contains("readme")
        || id_lower.contains("changelog")
        || id_lower.contains("runbook")
        || id_lower.contains("architecture-guide")
        || id_lower.contains("troubleshooting")
        || id_lower.contains("operator-quickstart")
        || id_lower.contains("command-reference")
        || id_lower.contains("doc-freshness")
        || id_lower.contains("version-bump")
        || id_lower.contains("tag")
    {
        return "artifact";
    }
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

fn run_opus_decomposition(
    target_repo: &std::path::Path,
    planning_root: &std::path::Path,
) -> anyhow::Result<OpusDecompositionReport> {
    let registry = load_plan_registry_from_planning_root(target_repo, planning_root)?;
    let composite_plans: Vec<_> = registry
        .plans
        .iter()
        .filter(|plan| plan.composite && plan.category != raspberry_supervisor::PlanCategory::Meta)
        .collect();

    if composite_plans.is_empty() {
        return Ok(OpusDecompositionReport::default());
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
        "Decomposing {count} composite plans with {} profile (parallel agent team) ...",
        automation_profile_name(AutomationProfile::Synth)
    );
    run_automation_chain(
        AutomationProfile::Synth,
        prompt_file.path(),
        target_repo,
        200,
        true,
    )?;
    let refreshed_paths =
        refreshed_opus_paths_for_run(target_repo, &composite_plans, run_id.as_str());

    println!(
        "  Decomposed {}/{count} composite plans",
        refreshed_paths.len()
    );
    Ok(OpusDecompositionReport {
        refreshed_paths,
        expected_paths,
    })
}

fn refreshed_opus_paths_for_run(
    target_repo: &std::path::Path,
    composite_plans: &[&PlanRecord],
    run_id: &str,
) -> BTreeSet<PathBuf> {
    let mut refreshed_paths = BTreeSet::new();
    for plan in composite_plans {
        let mapping_path = mapping_snapshot_path(plan);
        let absolute_path = target_repo.join(&mapping_path);
        if let Some(metadata) = load_mapping_metadata(&absolute_path) {
            if metadata.mapping_source.as_deref() == Some("opus")
                && metadata.generated_by_run.as_deref() == Some(run_id)
            {
                refreshed_paths.insert(mapping_path);
            }
        }
    }
    refreshed_paths
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
plan_id: exact plan_id from the manifest above
title: "from the plan's H1 heading"
category: see category rules below
composite: true
bootstrap_required: true or false per bootstrap rules below
implementation_required: true unless the plan is truly meta/report-only
dependency_plan_ids: see dependency rules below
children:
  - id: concise-kebab-case (2-4 words, e.g., casino-core, provably-fair, house-handler)
    title: human-readable milestone name
    archetype: one of implement, integration, orchestration, report
    lane_kind: one of platform, service, interface, artifact, integration, orchestration
    review_profile: one of standard, foundation, hardened, ux
    proof_commands: see proof command rules below
    owned_surfaces: [repo-relative paths from plan]
    where_surfaces: one-line summary
    how_description: one-line behavior change description
    acceptance_criteria: see acceptance criteria rules below
    required_tests: concrete test commands derived from acceptance criteria
    verification_plan: what proves this child is done
    rollback_condition: what reopens this child
```

## Category rules

Pick the ONE category that best describes the plan's primary purpose:

| Category | When to use | Examples |
|----------|-------------|---------|
| `foundation` | Shared types, traits, SDK, core abstractions that other plans depend on | casino-core trait, game-engine SDK, shared config |
| `game` | Game logic, game engines, game-specific features | poker, blackjack, craps, any game implementation |
| `interface` | User-facing surfaces: TUI, web, mobile, CLI, dashboards | TUI shell, web dashboard, CLI tools |
| `service` | Long-running daemons, agents, APIs, binary entry points | miner binary, validator binary, house agent, RPC server |
| `infrastructure` | Chain, networking, deployment, CI/CD, devops, monitoring | chain restart, CI pipeline, operational setup |
| `verification` | Test suites, coverage, audits, formal verification | test coverage sprint, security audit |
| `economic` | Financial logic, tokenomics, emission, staking, rewards | emission schedule, staking mechanics |
| `meta` | Coordination plans, master plans (NEVER use for actual work plans) | master plan only |

Do NOT default to `verification`. Most plans are `foundation`, `game`, `service`, or `infrastructure`.

## Dependency rules

Extract dependency_plan_ids from ALL of these signals in the plan text:
- Explicit: "depends on:", "requires:", "blocked by:", "after:"
- References: "plans/007-chain-restart.md", "see plan 007"
- Implicit: "once the chain is running" → depends on chain-restart plan
- Cross-references: "uses casino-core trait" → depends on casino-core

Use ONLY exact `plan_id` values that appear in the manifest above. Copy them verbatim. Do NOT invent semantic slugs. Do NOT emit numbered filenames. Do NOT shorten or normalize beyond the exact listed ids.

dependency_plan_ids must NEVER be empty for plans that clearly depend on other plans. Read the plan text carefully for implicit dependencies.

## Bootstrap rules

- Set `bootstrap_required: false` when the plan is already implementation-ready:
  - names owned surfaces or exact file paths
  - names concrete proof commands or tests
  - names explicit validation / acceptance criteria
- Set `bootstrap_required: true` only when the plan is still strategic, ambiguous, or missing the executable details above.
- For dropped plans, still emit the exact full `plan_id` from the manifest above.

## Proof command rules

1. Use commands that appear verbatim in the plan text when available.
2. If the plan doesn't contain a verbatim command but describes what should be tested, construct a reasonable proof command from the crate/module names mentioned. E.g., if the plan says "ensure myosu-miner builds" → `cargo build -p myosu-miner`.
3. Every child MUST have at least one proof_command. Never leave this empty. At minimum use `cargo check -p <crate>` for Rust or the equivalent build command for the project.
4. Prefer specific test targets (`cargo test -p crate -- test_name`) over broad ones (`cargo test`).

## Acceptance criteria rules

Each child MUST have 2-5 acceptance criteria that describe **behavioral outcomes**, not implementation approach. The verify gate will enforce these, so they must be testable.

**Good AC** (behavioral, observable, testable):
- "Given a Player bet of 100 units, settlement on Player win pays 200 units"
- "cargo test -p casino-core -- baccarat::settlement passes with ≥3 test cases covering Player/Banker/Tie"
- "MmapBlueprint::strategy() returns non-empty probability distributions that sum to 1.0 for all street/position combinations"
- "The CI workflow runs cargo check, cargo test, and cargo clippy on push to main"

**Bad AC** (structural, vague, untestable):
- "Add BaccaratVariant struct" (structural — says WHAT to add, not WHAT it must do)
- "Implement settlement logic" (vague — no observable outcome)
- "Write tests" (circular — the tests ARE the verification)

For each AC, derive a corresponding entry in `required_tests` that would verify the behavioral outcome. The required_tests should be concrete shell commands or test function names.

## Critical rules

1. One child per milestone in the plan's Progress section. Do NOT duplicate or split.
2. Child IDs must be concise. Bad: `craps-game-engine-state-machine-30-bet-types`. Good: `casino-core`.
3. Archetype: almost everything is `implement`. Only use `integration` for e2e/system tests, `orchestration` for meta-work spawning child programs, `report` for non-code artifacts.
4. Lane kind: `service` for daemons, APIs, agents, handlers, and anything with health/operator surfaces. `interface` for TUI/web/mobile/CLI user-facing work. `platform` for libraries and core modules. `artifact` for documentation/reports.
5. Review profile: `standard` for normal code. `foundation` for shared types/traits/SDK that downstream work depends on. `hardened` for security, crypto, financial logic, correctness-critical invariants — anything where bugs are catastrophic. `ux` for user-facing surfaces (TUI, web, mobile, CLI).
6. Write each YAML file directly using the Write tool. Do NOT output YAML to stdout.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_lane_kind_marks_doc_guides_as_artifacts() {
        assert_eq!(
            infer_lane_kind_from_child_id(
                "documentation-and-operator-runbook-architecture-guide-for-contributors"
            ),
            "artifact"
        );
        assert_eq!(
            infer_lane_kind_from_child_id("release-preparation-readme-and-changelog"),
            "artifact"
        );
    }

    #[test]
    fn infer_archetype_treats_integration_tests_as_implementation_work() {
        assert_eq!(
            infer_archetype_from_child_id("autodev-integration-test", ""),
            "implement_module"
        );
    }

    #[test]
    fn batch_decomposition_prompt_requires_exact_plan_ids() {
        let prompt = build_batch_decomposition_prompt(
            "- plan_id: casino-core\n  path: plans/004-casino-core-trait.md\n  category: foundation\n  dependency_plan_ids: []\n  output: malinka/plan-mappings/004-casino-core-trait.yaml",
            Path::new("/tmp/repo"),
            "opus-test",
        );
        assert!(prompt.contains("plan_id: exact plan_id from the manifest above"));
        assert!(prompt.contains("Use ONLY exact `plan_id` values"));
        assert!(prompt.contains("bootstrap_required: false"));
    }

    #[test]
    fn validate_plan_mapping_snapshots_rejects_missing_plan_id_and_unknown_dependencies() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        std::fs::create_dir_all(temp.path().join("malinka/plan-mappings")).expect("mappings dir");
        std::fs::write(
            temp.path().join("plans/001-master-plan.md"),
            "# Master Plan\n",
        )
        .expect("master");
        std::fs::write(
            temp.path().join("plans/005-craps-game.md"),
            "# Craps Game\n",
        )
        .expect("craps");
        std::fs::write(
            temp.path()
                .join("malinka/plan-mappings/005-craps-game.yaml"),
            concat!(
                "mapping_source: opus\n",
                "dependency_plan_ids:\n",
                "  - phase-1-devnet-endurance\n",
            ),
        )
        .expect("mapping");

        let error =
            validate_plan_mapping_snapshots(temp.path(), Path::new("")).expect_err("should fail");
        let rendered = error.to_string();
        assert!(rendered.contains("missing required plan_id"));
        assert!(rendered.contains("unknown [\"phase-1-devnet-endurance\"]"));
    }
}
