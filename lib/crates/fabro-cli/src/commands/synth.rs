use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Stdio;

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
    /// Skip the eng-review pass after decomposition
    #[arg(long)]
    pub no_review: bool,
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
    /// Skip the eng-review pass after reconciliation
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
        .filter(|plan| {
            plan.composite
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

    let claude_check = std::process::Command::new("claude")
        .arg("--version")
        .output();
    if claude_check.is_err() || !claude_check.as_ref().unwrap().status.success() {
        eprintln!("  [review] claude CLI not found; skipping decomposition review");
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
                plan.title, plan.plan_id, mapping_path.display(), content,
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
        "Eng-reviewing {} decomposition contracts ...",
        composite_plans.len()
    );

    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cat {} | CLAUDECODE= claude -p --output-format stream-json --dangerously-skip-permissions --model {} --max-turns 50",
            prompt_file.path().display(),
            DECOMPOSE_MODEL,
        ))
        .current_dir(target_repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Stream stderr
    let stderr_handle = {
        let stderr = child.stderr.take().expect("stderr piped");
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines().flatten() {
                eprintln!("  [review] {line}");
            }
        })
    };

    // Stream stdout JSON for live output
    let stdout_handle = {
        let stdout = child.stdout.take().expect("stdout piped");
        std::thread::spawn(move || {
            use std::io::{BufRead, Write};
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines().flatten() {
                let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) else {
                    continue;
                };
                let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if event_type == "assistant" {
                    if let Some(content) = event
                        .get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_array())
                    {
                        for block in content {
                            if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                                if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                    eprint!("{text}");
                                    let _ = std::io::stderr().flush();
                                }
                            }
                            if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                                if let Some(name) = block.get("name").and_then(|v| v.as_str()) {
                                    let hint = block
                                        .get("input")
                                        .and_then(|i| {
                                            i.get("file_path").or_else(|| i.get("command"))
                                        })
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    let short = if hint.len() > 60 { &hint[..60] } else { hint };
                                    eprintln!("\n  [review-tool] {name}: {short}");
                                }
                            }
                        }
                    }
                }
            }
        })
    };

    let status = child.wait()?;
    let _ = stderr_handle.join();
    let _ = stdout_handle.join();
    eprintln!();

    if !status.success() {
        eprintln!("  [review] review process exited with non-zero status; continuing with current mappings");
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
    println!();

    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cat {} | CLAUDECODE= claude -p --output-format stream-json --dangerously-skip-permissions --model {} --max-turns 200",
            prompt_file.path().display(),
            DECOMPOSE_MODEL,
        ))
        .current_dir(&args.target_repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Stream stderr for CLI-level messages
    let stderr_handle = {
        let stderr = child.stderr.take().expect("stderr piped");
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stderr);
            let mut collected = Vec::new();
            for line in reader.lines() {
                let Ok(line) = line else { break };
                eprintln!("  [genesis] {line}");
                collected.push(line);
            }
            collected
        })
    };

    // Stream stdout JSON lines for real-time Claude output
    let stdout_handle = {
        let stdout = child.stdout.take().expect("stdout piped");
        std::thread::spawn(move || {
            use std::io::{BufRead, Write};
            let reader = std::io::BufReader::new(stdout);
            let mut success = false;
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) else {
                    continue;
                };
                let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match event_type {
                    "assistant" => {
                        // Text content from Claude
                        if let Some(message) = event.get("message") {
                            if let Some(content) = message.get("content").and_then(|c| c.as_array()) {
                                for block in content {
                                    if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                                        if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                            eprint!("{text}");
                                            let _ = std::io::stderr().flush();
                                        }
                                    }
                                    if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                                        if let Some(name) = block.get("name").and_then(|v| v.as_str()) {
                                            let path_hint = block
                                                .get("input")
                                                .and_then(|i| i.get("file_path").or_else(|| i.get("command")))
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("");
                                            let short = if path_hint.len() > 60 {
                                                &path_hint[..60]
                                            } else {
                                                path_hint
                                            };
                                            eprintln!("\n  [tool] {name}: {short}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "result" => {
                        // Final result
                        if let Some(sub) = event.get("subtype").and_then(|v| v.as_str()) {
                            eprintln!("\n  [genesis] result: {sub}");
                            if sub == "success" {
                                success = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
            success
        })
    };

    let status = child.wait()?;
    let genesis_success = stdout_handle.join().unwrap_or(false);
    let stderr_lines = stderr_handle.join().unwrap_or_default();

    if !status.success() && !genesis_success {
        let stderr = stderr_lines.join("\n");
        anyhow::bail!("Genesis failed: {}", stderr.trim());
    }

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

# Phase 1: THINK (Office Hours)

Explore the codebase thoroughly. Spawn an agent team to read in parallel:
- Build files: Cargo.toml / package.json / pyproject.toml / go.mod
- Source structure: src/, lib/, app/ — module boundaries, public API surface
- Existing docs: README, SPEC.md, SPECS.md
- **Existing plans**: Read EVERY file in `plans/` and `specs/` directories. These are the team's current plans — your job is to assess, challenge, and enhance them, not start from scratch. Note which plans are strong, which are weak, which are missing, and which contradict each other.
- Git history: recent 50 commits — who's active, what's changing, what's abandoned
- Test coverage: test directories, CI config, what's tested vs. untested
- Dependency graph: what does this project depend on, what depends on it

Then answer these six forcing questions (write answers to `genesis/ASSESSMENT.md`):

1. **Demand reality**: Who uses this? What specific behavior proves real demand — payments, daily usage, panic if it vanished? Or is it a side project, prototype, abandoned experiment?
2. **Status quo**: What's the current workflow without this project? What pain does it solve? What duct-tape alternatives exist?
3. **Desperate specificity**: Who is the ONE person this is for? What's their title, what keeps them up at night, what gets them promoted? Not "developers" — a real role with real constraints.
4. **Narrowest wedge**: What's the smallest thing this codebase does that someone would pay for or depend on THIS WEEK? Strip away the vision — what's the kernel of value?
5. **Observation & surprise**: What surprised you during exploration? What behavior exists that the original author probably didn't intend? What's half-built in a revealing way?
6. **Future-fit**: How does this project compound as the world changes? Or does it decay? What bet is it making?

## ASSESSMENT.md structure

Write `genesis/ASSESSMENT.md` with:
- Answers to the six questions above
- What works, what's broken, what's half-built
- Tech debt inventory (with file paths)
- Security risks found
- Test coverage gaps (with specific untested modules)
- The ONE sentence that captures what this project actually is
- **Existing plan assessment**: For each plan in `plans/`, rate it (strong/weak/missing context/contradicts X) and state what genesis will do with it (carry forward as-is, enhance, split, merge, or replace with rationale)

# Phase 2: PLAN (Strategic + Engineering + Design)

## 2a. Strategic Plan (CEO lens)

For the master plan and each numbered plan, apply:

- **Scope discipline**: What is the minimum change set? What can be deferred?
- **Inversion reflex**: For each goal, also state what makes it fail. Put this in the Decision Log.
- **Focus as subtraction**: 3-8 milestones per plan. A plan with 15 milestones is trying to do too much — split it.
- **Reversibility**: Prefer plans that can be rolled back. Flag one-way doors explicitly.
- **Existing code reuse**: Before proposing new abstractions, verify what already exists. Don't rebuild what you can extend.
- **Narrowest wedge first**: Phase 0 plans should deliver value in 30 days, not prepare for value in 90.

## 2b. Engineering Plan (Eng Manager lens)

For every plan with technical content:

- **ASCII architecture diagram**: Every plan gets a component/data flow diagram of affected modules. No exceptions.
- **Complexity smell**: >8 files or >2 new abstractions = challenge whether simpler approach exists.
- **Failure mode analysis**: For each new codepath, name one realistic production failure (timeout, nil, race condition, stale data) and how it's handled.
- **Proof command quality**: Every milestone needs a specific proof command. `cargo test -p {{crate}} {{test_name}}` beats `cargo test`. Content assertions beat `test -f`.
- **Boring by default**: Novel infrastructure needs explicit justification. Default to proven technology.
- **Separate structural from behavioral**: Plans that refactor AND add features simultaneously are red flags. Split them.
- **DRY across plans**: If two plans touch the same module, they need a shared dependency plan or explicit ordering.
- **Test plan**: For each milestone, state which tests prove it's done — module, assertions, edge cases.

## 2c. Design Plan (Designer lens)

For any plan touching user-facing surfaces:

- **Information architecture**: What does the user see first, second, third? Include ASCII mockups.
- **Interaction states**: Loading, empty, error, success, partial — all specified. Empty states are features.
- **Edge cases**: 47-char names, zero results, network failure mid-action, first-time vs. power user.
- **Accessibility as scope**: Keyboard nav, screen reader, contrast, 44px+ touch targets — deliverables, not polish.
- **Responsive intent**: Specific layout changes per viewport, not just "stacks on mobile."
- **No AI slop**: "Clean modern dashboard" is a vibe, not a plan. Name specific layout choices and information hierarchy.

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

d. `genesis/plans/002-*.md` through `genesis/plans/N-*.md` — one ExecPlan per work stream:
   - Purpose, Progress (milestones with `- [ ]`), Decision Log, proof commands
   - Cover: tech debt, missing tests, broken features, new features, infrastructure, docs
   - Name specific files, crates, modules, functions — concrete, not generic
   - ASCII diagrams for architecture and data flow
   - 3-8 milestones per plan, each with a real proof command

**Carry-forward rule**: Every existing plan in `plans/` must appear in `genesis/plans/`. For plans you assessed as strong, copy them into genesis with the same filename and number. For plans you enhanced, write the enhanced version. For plans you split or merged, write the new plans and note the provenance in the Decision Log. No existing plan should silently disappear — if you're dropping one, write a short `genesis/plans/NNN-dropped-*.md` explaining why.

# Phase 4: REVIEW (Self-Review Pass)

After writing all plans, review the corpus against these checklists:

## Structural review
- [ ] Every plan references specific file paths, not vague module descriptions
- [ ] Every milestone has a proof command that would actually work in this repo
- [ ] No two plans claim the same files without explicit dependency ordering
- [ ] Master plan references all numbered plans
- [ ] Each numbered plan has 3-8 milestones (not more, not fewer)

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

If any check fails, fix the plan before moving on. Do not write a plan you wouldn't approve as a reviewer.

# Phase 5: VERIFY

Write `genesis/GENESIS-REPORT.md` summarizing:
- Total plans generated
- Assessment highlights (from Phase 1)
- Plans needing human attention (flagged during review)
- Known gaps (things you couldn't assess without running the code)
- Recommended next steps for the operator

## Rules

- Be specific to THIS codebase. Don't write generic plans.
- Reference actual file paths, module names, function names from exploration.
- Include ASCII diagrams for architecture, data flow, and state machines.
- Write all files using the Write tool. Do NOT output content to stdout.
- 10-20 numbered plans in the master plan. 3-8 milestones per plan.
- Use the ExecPlan format from PLANS.md for every plan.

Begin with Phase 1: explore the codebase and write ASSESSMENT.md."#,
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

        // Eng-review the decomposition before rendering into workflows
        if !args.no_review {
            run_decomposition_review(&args.target_repo)?;
            // Re-author again in case review rewrote mapping contracts
            let reviewed = author_blueprint_for_create_with_planning_root(
                &args.target_repo,
                Some(&blueprint.program.id),
                Some(&planning_root),
            )?;
            save_blueprint(&blueprint_path, &reviewed.blueprint)?;
        }

        let re_authored = author_blueprint_for_create_with_planning_root(
            &args.target_repo,
            Some(&blueprint.program.id),
            Some(&planning_root),
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

    if !args.no_review {
        run_decomposition_review(output_repo)?;
    }

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
