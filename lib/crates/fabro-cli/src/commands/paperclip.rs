use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{ArgAction, Args, Subcommand};
use fabro_synthesis::{
    author_blueprint_for_create, load_blueprint, render_blueprint, save_blueprint, BlueprintUnit,
    ProgramBlueprint, RenderRequest, WorkflowTemplate,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Args)]
pub struct PaperclipArgs {
    #[command(subcommand)]
    pub command: PaperclipCommand,
}

#[derive(Debug, Subcommand)]
pub enum PaperclipCommand {
    /// Generate and bootstrap a repo-local Paperclip company on top of the current blueprint
    Bootstrap(PaperclipBootstrapArgs),
}

#[derive(Debug, Args, Clone)]
pub struct PaperclipBootstrapArgs {
    #[arg(long)]
    pub target_repo: PathBuf,
    #[arg(long)]
    pub program: Option<String>,
    #[arg(long)]
    pub company_name: Option<String>,
    #[arg(long)]
    pub data_dir: Option<PathBuf>,
    #[arg(long)]
    pub api_base: Option<String>,
    #[arg(long)]
    pub paperclip_cmd: Option<String>,
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub apply: bool,
}

pub async fn bootstrap_command(args: &PaperclipBootstrapArgs) -> Result<()> {
    let target_repo = &args.target_repo;
    let program_id = args
        .program
        .clone()
        .or_else(|| {
            target_repo
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
        })
        .unwrap_or_else(|| "repo".to_string());
    let blueprint_path = target_repo
        .join("fabro")
        .join("blueprints")
        .join(format!("{program_id}.yaml"));
    let manifest_path = target_repo
        .join("fabro")
        .join("programs")
        .join(format!("{program_id}.yaml"));

    let blueprint = if blueprint_path.exists() {
        load_blueprint(&blueprint_path)?
    } else {
        let authored = author_blueprint_for_create(target_repo, Some(&program_id))?;
        save_blueprint(&blueprint_path, &authored.blueprint)?;
        render_blueprint(RenderRequest {
            blueprint: &authored.blueprint,
            target_repo,
        })?;
        authored.blueprint
    };
    if !manifest_path.exists() {
        render_blueprint(RenderRequest {
            blueprint: &blueprint,
            target_repo,
        })?;
    }

    let data_dir = args
        .data_dir
        .clone()
        .unwrap_or_else(|| target_repo.join(".paperclip"));
    let api_base = args
        .api_base
        .clone()
        .unwrap_or_else(|| "http://127.0.0.1:3100".to_string());
    let company_name = args
        .company_name
        .clone()
        .unwrap_or_else(|| humanize(&program_id));
    let mission = derive_bootstrap_mission(&blueprint, target_repo, &company_name)?;
    let bundle_root = target_repo
        .join("fabro")
        .join("paperclip")
        .join(&program_id);
    let scripts_root = bundle_root.join("scripts");
    ensure_private_data_dir(&data_dir, target_repo)?;
    std::fs::create_dir_all(&scripts_root)?;

    let fabro_binary = current_fabro_binary()?;
    let raspberry_command = raspberry_command();
    write_orchestrator_script(
        &scripts_root.join("raspberry-orchestrator.sh"),
        target_repo,
        &manifest_path,
        &fabro_binary,
        &raspberry_command,
    )?;
    write_run_script(
        &scripts_root.join("run-paperclip.sh"),
        &paperclip_command(args.paperclip_cmd.as_deref()),
        &data_dir,
    )?;

    let bundle = build_company_bundle(
        &blueprint,
        target_repo,
        &company_name,
        &mission,
        &scripts_root.join("raspberry-orchestrator.sh"),
    )?;
    write_bundle(&bundle_root, &bundle)?;

    println!("Program: {program_id}");
    println!("Paperclip bundle: {}", bundle_root.display());
    println!("Data dir: {}", data_dir.display());
    println!("API base: {api_base}");
    println!("Company goal: {}", mission.goal_title);

    if !args.apply {
        println!("Applied: no");
        return Ok(());
    }

    ensure_paperclip_server(
        &paperclip_command(args.paperclip_cmd.as_deref()),
        &data_dir,
        &api_base,
    )
    .await?;
    let bootstrap_state_path = bundle_root.join("bootstrap-state.json");
    let existing_company_id = load_bootstrap_state(&bootstrap_state_path)
        .ok()
        .and_then(|state| {
            state
                .get("companyId")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
        });
    let existing_company_id = match existing_company_id {
        Some(company_id) => Some(company_id),
        None => find_existing_company_id(&api_base, &company_name).await?,
    };
    let import_result = import_company_package(
        &paperclip_command(args.paperclip_cmd.as_deref()),
        &data_dir,
        &api_base,
        &bundle_root,
        &company_name,
        existing_company_id.as_deref(),
    )?;
    let company_id = import_result.company.id.clone();
    let existing_state = load_bootstrap_state(&bootstrap_state_path).ok();
    let existing_goal_id = existing_state
        .as_ref()
        .and_then(|state| state.get("goal"))
        .and_then(|value| value.get("id"))
        .and_then(|value| value.as_str());
    let existing_project_id = existing_state
        .as_ref()
        .and_then(|state| state.get("project"))
        .and_then(|value| value.get("id"))
        .and_then(|value| value.as_str());
    let goal = ensure_company_goal(&api_base, &company_id, &mission, existing_goal_id).await?;
    let project = ensure_company_project(
        &api_base,
        &company_id,
        &goal.id,
        &mission,
        target_repo,
        existing_project_id,
    )
    .await?;
    cleanup_generated_agent_duplicates(
        &api_base,
        &company_id,
        &bundle.agents,
        &import_result.agents,
    )
    .await?;
    let mut bootstrap_state = serde_json::to_value(&import_result)
        .context("failed to serialize paperclip import result")?;
    bootstrap_state["companyId"] = json!(company_id);
    bootstrap_state["goal"] = json!({
        "id": goal.id,
        "title": goal.title,
    });
    bootstrap_state["project"] = json!({
        "id": project.id,
        "name": project.name,
    });
    save_bootstrap_state(&bootstrap_state_path, &bootstrap_state)?;
    install_local_cli_for_agents(
        &paperclip_command(args.paperclip_cmd.as_deref()),
        &data_dir,
        &api_base,
        &company_id,
        &bundle.agents,
        &import_result.agents,
    )?;

    println!("Applied: yes");
    println!("Company ID: {company_id}");
    println!("Goal ID: {}", goal.id);
    println!("Project ID: {}", project.id);
    Ok(())
}

#[derive(Debug, Clone)]
struct BootstrapMission {
    company_description: String,
    goal_title: String,
    goal_description: String,
    project_name: String,
    project_description: String,
    workspace_name: String,
}

struct BundleAgent {
    slug: String,
    name: String,
    adapter_type: &'static str,
}

struct GeneratedBundle {
    manifest: serde_json::Value,
    company_markdown: String,
    agent_markdowns: Vec<(String, String)>,
    agents: Vec<BundleAgent>,
}

fn build_company_bundle(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
    company_name: &str,
    mission: &BootstrapMission,
    orchestrator_script: &Path,
) -> Result<GeneratedBundle> {
    let description = mission.company_description.clone();
    let mut manifest_agents = Vec::new();
    let mut agent_markdowns = Vec::new();
    let mut agents = Vec::new();

    let ceo_slug = "mission-ceo";
    let ceo_name = "Mission CEO".to_string();
    manifest_agents.push(json!({
        "slug": ceo_slug,
        "name": ceo_name,
        "path": format!("agents/{ceo_slug}/AGENTS.md"),
        "role": "ceo",
        "title": "Mission CEO",
        "icon": "crown",
        "capabilities": "Own company mission, set priorities, and review lane strategy.",
        "reportsToSlug": serde_json::Value::Null,
        "adapterType": "claude_local",
        "adapterConfig": {
            "cwd": target_repo.display().to_string(),
            "model": "claude-sonnet-4-6"
        },
        "runtimeConfig": {
            "heartbeat": {
                "enabled": false,
                "intervalSec": 0,
                "wakeOnDemand": true,
                "maxConcurrentRuns": 1
            }
        },
        "permissions": {},
        "budgetMonthlyCents": 0,
        "metadata": {
            "source": "fabro.paperclip",
            "type": "mission_ceo"
        }
    }));
    agent_markdowns.push((
        format!("agents/{ceo_slug}/AGENTS.md"),
        build_agent_markdown(
            &ceo_name,
            ceo_slug,
            "ceo",
            format!(
                "You own the company mission for `{}`.\n\nCompany goal:\n- {}\n\nPriorities:\n- keep work aligned to the repo blueprint\n- promote lane decomposition that matches the real plan\n- route execution through Raspberry rather than bypassing it\n- prefer honest progress over optimistic summaries\n",
                blueprint.program.id,
                mission.goal_title,
            ),
        ),
    ));
    agents.push(BundleAgent {
        slug: ceo_slug.to_string(),
        name: ceo_name,
        adapter_type: "claude_local",
    });

    let orchestrator_slug = "raspberry-orchestrator";
    manifest_agents.push(json!({
        "slug": orchestrator_slug,
        "name": "Raspberry Orchestrator",
        "path": format!("agents/{orchestrator_slug}/AGENTS.md"),
        "role": "pm",
        "title": "Raspberry Orchestrator",
        "icon": "circuit-board",
        "capabilities": "Run repo-local Raspberry plan/status/execute/autodev loops.",
        "reportsToSlug": ceo_slug,
        "adapterType": "process",
        "adapterConfig": {
            "command": format!("bash {}", shell_quote(&orchestrator_script.display().to_string())),
            "cwd": target_repo.display().to_string(),
            "timeoutSec": 1800
        },
        "runtimeConfig": {
            "heartbeat": {
                "enabled": true,
                "intervalSec": 300,
                "wakeOnDemand": true,
                "maxConcurrentRuns": 1
            }
        },
        "permissions": {},
        "budgetMonthlyCents": 0,
        "metadata": {
            "source": "fabro.paperclip",
            "type": "raspberry_orchestrator",
            "program": blueprint.program.id
        }
    }));
    agent_markdowns.push((
        format!("agents/{orchestrator_slug}/AGENTS.md"),
        build_agent_markdown(
            "Raspberry Orchestrator",
            orchestrator_slug,
            "pm",
            format!(
                "You operate the repo-local Raspberry control plane for `{}`.\n\nRun the orchestration script, inspect the resulting status, and promote the company mission by advancing honest ready lanes only.\n",
                blueprint.program.id
            ),
        ),
    ));
    agents.push(BundleAgent {
        slug: orchestrator_slug.to_string(),
        name: "Raspberry Orchestrator".to_string(),
        adapter_type: "process",
    });

    for unit in &blueprint.units {
        let lane = unit
            .lanes
            .first()
            .context("paperclip bootstrap expects one primary lane per unit")?;
        let slug = unit.id.clone();
        let name = unit.title.clone();
        let role = lane_role(unit, lane);
        let adapter_type = lane_adapter_type(unit, lane);
        let model = if adapter_type == "claude_local" {
            json!("claude-sonnet-4-6")
        } else {
            json!("gpt-5.3-codex")
        };
        manifest_agents.push(json!({
            "slug": slug,
            "name": name,
            "path": format!("agents/{}/AGENTS.md", unit.id),
            "role": role,
            "title": unit.title,
            "icon": serde_json::Value::Null,
            "capabilities": format!("Own the `{}` lane and its artifacts.", lane.id),
            "reportsToSlug": orchestrator_slug,
            "adapterType": adapter_type,
            "adapterConfig": {
                "cwd": target_repo.display().to_string(),
                "model": model
            },
            "runtimeConfig": {
                "heartbeat": {
                    "enabled": false,
                    "intervalSec": 0,
                    "wakeOnDemand": true,
                    "maxConcurrentRuns": 1
                }
            },
            "permissions": {},
            "budgetMonthlyCents": 0,
            "metadata": {
                "source": "fabro.paperclip",
                "laneKey": lane.id,
                "family": lane.family,
                "unit": unit.id
            }
        }));
        agent_markdowns.push((
            format!("agents/{}/AGENTS.md", unit.id),
            build_agent_markdown(
                &name,
                &unit.id,
                role,
                format!(
                    "You own the `{}` frontier in repo `{}`.\n\nCompany goal:\n{}\n\nLane goal:\n{}\n\nArtifacts:\n{}\n\nDependencies:\n{}\n\nDo not bypass Raspberry. Work inside the repo and keep outputs aligned with the lane contract.\n",
                    lane.id,
                    blueprint.program.id,
                    mission.goal_title,
                    lane.goal,
                    unit.artifacts
                        .iter()
                        .map(|artifact| format!("- {}", artifact.path.display()))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    lane.dependencies
                        .iter()
                        .map(|dependency| format!(
                            "- {}{}{}",
                            dependency.unit,
                            dependency
                                .lane
                                .as_ref()
                                .map(|lane| format!(":{}", lane))
                                .unwrap_or_default(),
                            dependency
                                .milestone
                                .as_ref()
                                .map(|milestone| format!("@{}", milestone))
                                .unwrap_or_default()
                        ))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
            ),
        ));
        agents.push(BundleAgent {
            slug: unit.id.clone(),
            name,
            adapter_type,
        });
    }

    let manifest = json!({
        "schemaVersion": 1,
        "generatedAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "source": serde_json::Value::Null,
        "includes": { "company": true, "agents": true },
        "company": {
            "path": "COMPANY.md",
            "name": company_name,
            "description": description,
            "brandColor": serde_json::Value::Null,
            "requireBoardApprovalForNewAgents": true
        },
        "agents": manifest_agents,
        "requiredSecrets": [
            {
                "key": "OPENAI_API_KEY",
                "description": "Set OPENAI_API_KEY for Codex local agents",
                "agentSlug": serde_json::Value::Null,
                "providerHint": "openai"
            },
            {
                "key": "ANTHROPIC_API_KEY",
                "description": "Set ANTHROPIC_API_KEY for Claude local agents",
                "agentSlug": serde_json::Value::Null,
                "providerHint": "anthropic"
            }
        ]
    });
    let company_markdown = build_company_markdown(company_name, &description, &agents);

    Ok(GeneratedBundle {
        manifest,
        company_markdown,
        agent_markdowns,
        agents,
    })
}

fn lane_role(unit: &BlueprintUnit, lane: &fabro_synthesis::BlueprintLane) -> &'static str {
    if lane.template == WorkflowTemplate::RecurringReport || unit.id.contains("proof") {
        return "qa";
    }
    if lane.template == WorkflowTemplate::Orchestration {
        return "pm";
    }
    "engineer"
}

fn lane_adapter_type(unit: &BlueprintUnit, lane: &fabro_synthesis::BlueprintLane) -> &'static str {
    if lane.template == WorkflowTemplate::RecurringReport || unit.id.contains("proof") {
        return "claude_local";
    }
    "codex_local"
}

fn write_bundle(root: &Path, bundle: &GeneratedBundle) -> Result<()> {
    std::fs::create_dir_all(root)?;
    std::fs::write(
        root.join("paperclip.manifest.json"),
        serde_json::to_string_pretty(&bundle.manifest)?,
    )?;
    std::fs::write(root.join("COMPANY.md"), &bundle.company_markdown)?;
    for (relative, markdown) in &bundle.agent_markdowns {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, markdown)?;
    }
    Ok(())
}

fn build_company_markdown(name: &str, description: &str, agents: &[BundleAgent]) -> String {
    let mut body = format!("# {}\n\n{}\n\n# Agents\n", name, description);
    for agent in agents {
        body.push_str(&format!("- {} - {}\n", agent.slug, agent.name));
    }
    format!(
        "---\nkind: company\nname: {}\ndescription: {}\nbrandColor: null\nrequireBoardApprovalForNewAgents: true\n---\n\n{}",
        serde_json::to_string(name).expect("json"),
        serde_json::to_string(description).expect("json"),
        body
    )
}

fn build_agent_markdown(name: &str, slug: &str, role: &str, body: String) -> String {
    format!(
        "---\nkind: agent\nname: {}\nslug: {}\nrole: {}\n---\n\n{}",
        serde_json::to_string(name).expect("json"),
        serde_json::to_string(slug).expect("json"),
        serde_json::to_string(role).expect("json"),
        body
    )
}

fn write_orchestrator_script(
    path: &Path,
    target_repo: &Path,
    manifest_path: &Path,
    fabro_binary: &Path,
    raspberry_command: &str,
) -> Result<()> {
    let body = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\ncd {}\n{} autodev --manifest {} --fabro-bin {} --max-cycles 1 --poll-interval-ms 1 --evolve-every-seconds 0\n",
        shell_quote(&target_repo.display().to_string()),
        raspberry_command,
        shell_quote(&manifest_path.display().to_string()),
        shell_quote(&fabro_binary.display().to_string()),
    );
    std::fs::write(path, body)?;
    Ok(())
}

fn write_run_script(path: &Path, paperclip_cmd: &str, data_dir: &Path) -> Result<()> {
    let body = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\nexec {} run --data-dir {}\n",
        paperclip_cmd,
        shell_quote(&data_dir.display().to_string()),
    );
    std::fs::write(path, body)?;
    Ok(())
}

fn current_fabro_binary() -> Result<PathBuf> {
    std::env::current_exe().context("failed to resolve current fabro binary")
}

fn raspberry_command() -> String {
    let current = std::env::current_exe().ok();
    if let Some(path) = current
        .as_ref()
        .map(|path| path.with_file_name("raspberry"))
    {
        if path.exists() {
            return shell_quote(&path.display().to_string());
        }
    }
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .components()
        .collect::<PathBuf>();
    format!(
        "cargo run -q --manifest-path {} -p raspberry-cli --",
        shell_quote(&workspace_root.join("Cargo.toml").display().to_string())
    )
}

fn paperclip_command(override_value: Option<&str>) -> String {
    if let Some(value) = override_value {
        return value.to_string();
    }
    let local_repo = Path::new("/home/r/coding/paperclip");
    if local_repo.is_dir() {
        return format!(
            "pnpm --silent --dir {} paperclipai",
            shell_quote(&local_repo.display().to_string())
        );
    }
    "paperclipai".to_string()
}

async fn ensure_paperclip_server(
    paperclip_cmd: &str,
    data_dir: &Path,
    api_base: &str,
) -> Result<()> {
    if paperclip_server_ready(api_base).await {
        return Ok(());
    }

    let log_path = data_dir.join("server.log");
    let pid_path = data_dir.join("server.pid");
    std::fs::create_dir_all(data_dir)?;
    let start_cmd = format!(
        "nohup {} run --data-dir {} > {} 2>&1 & echo $! > {}",
        paperclip_cmd,
        shell_quote(&data_dir.display().to_string()),
        shell_quote(&log_path.display().to_string()),
        shell_quote(&pid_path.display().to_string()),
    );
    let output = Command::new("bash")
        .arg("-lc")
        .arg(start_cmd)
        .output()
        .context("failed to start paperclip server")?;
    if !output.status.success() {
        bail!(
            "paperclip bootstrap failed to start server: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    for _ in 0..30 {
        if paperclip_server_ready(api_base).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("paperclip server did not become ready at {api_base}");
}

async fn paperclip_server_ready(api_base: &str) -> bool {
    reqwest::Client::new()
        .get(format!("{api_base}/api/health"))
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

fn import_company_package(
    paperclip_cmd: &str,
    data_dir: &Path,
    api_base: &str,
    bundle_root: &Path,
    company_name: &str,
    existing_company_id: Option<&str>,
) -> Result<PaperclipImportResult> {
    let mut import_cmd = format!(
        "{} company import --json --data-dir {} --api-base {} --from {} --include company,agents",
        paperclip_cmd,
        shell_quote(&data_dir.display().to_string()),
        shell_quote(api_base),
        shell_quote(&bundle_root.display().to_string()),
    );
    if let Some(company_id) = existing_company_id {
        import_cmd.push_str(" --target existing --company-id ");
        import_cmd.push_str(&shell_quote(company_id));
        import_cmd.push_str(" --collision replace");
    } else {
        import_cmd.push_str(" --target new --new-company-name ");
        import_cmd.push_str(&shell_quote(company_name));
    }
    let output = Command::new("bash")
        .arg("-lc")
        .arg(import_cmd)
        .output()
        .context("failed to import paperclip company package")?;
    if !output.status.success() {
        bail!(
            "paperclip company import failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    parse_json_stdout(&output.stdout, "paperclip company import")
}

fn install_local_cli_for_agents(
    paperclip_cmd: &str,
    data_dir: &Path,
    api_base: &str,
    company_id: &str,
    agents: &[BundleAgent],
    imported_agents: &[PaperclipImportAgent],
) -> Result<()> {
    let env_root = data_dir.join("local-cli");
    std::fs::create_dir_all(&env_root)?;
    write_secrets_gitignore(&env_root)?;
    let mut installed_skill_targets = BTreeSet::new();
    for agent in agents {
        if !matches!(agent.adapter_type, "claude_local" | "codex_local") {
            continue;
        }
        let agent_ref = imported_agents
            .iter()
            .find(|imported| imported.slug == agent.slug)
            .and_then(|imported| imported.id.as_deref())
            .unwrap_or(agent.slug.as_str());
        let install_skills = installed_skill_targets.insert(agent.adapter_type);
        let cmd = format!(
            "{} agent local-cli {} --company-id {} --data-dir {} --api-base {} --json{}",
            paperclip_cmd,
            shell_quote(agent_ref),
            shell_quote(company_id),
            shell_quote(&data_dir.display().to_string()),
            shell_quote(api_base),
            if install_skills {
                ""
            } else {
                " --no-install-skills"
            },
        );
        let output = Command::new("bash")
            .arg("-lc")
            .arg(cmd)
            .output()
            .with_context(|| format!("failed to install local cli for {}", agent.slug))?;
        if !output.status.success() {
            bail!(
                "paperclip local-cli install failed for {}: {}",
                agent.slug,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let response: LocalCliInstall = parse_json_stdout(
            &output.stdout,
            &format!("paperclip agent local-cli for {}", agent.slug),
        )?;
        let env_path = env_root.join(format!("{}.env", agent.slug));
        std::fs::write(env_path, response.exports)?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct LocalCliInstall {
    exports: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PaperclipImportResult {
    company: PaperclipImportCompany,
    agents: Vec<PaperclipImportAgent>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PaperclipImportCompany {
    id: String,
    name: String,
    action: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PaperclipImportAgent {
    slug: String,
    id: Option<String>,
    action: String,
    name: String,
    reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PaperclipGoal {
    id: String,
    title: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PaperclipProject {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PaperclipWorkspace {
    id: String,
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PaperclipCompany {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PaperclipManagedAgent {
    id: String,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

fn derive_bootstrap_mission(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
    company_name: &str,
) -> Result<BootstrapMission> {
    let spec_title = latest_markdown_title(target_repo, "specs")
        .or_else(|| markdown_title_for_file(&target_repo.join("SPEC.md")))
        .unwrap_or_else(|| format!("{} specification", humanize(&blueprint.program.id)));
    let plan_title = latest_markdown_title(target_repo, "plans")
        .unwrap_or_else(|| format!("{} execution plan", humanize(&blueprint.program.id)));
    let fronts = blueprint
        .units
        .iter()
        .map(|unit| unit.title.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let goal_sentence = extract_goal_sentence(target_repo).unwrap_or_else(|| {
        format!(
            "Advance {} according to the checked-in plan and spec.",
            company_name
        )
    });

    Ok(BootstrapMission {
        company_description: format!(
            "{} is a repo-local Paperclip company for `{}`. It exists to execute `{}` against `{}` while keeping Fabro synthesis and Raspberry orchestration as the source of truth across these fronts: {}.",
            company_name,
            blueprint.program.id,
            plan_title,
            spec_title,
            fronts
        ),
        goal_title: format!("Advance {}", company_name),
        goal_description: format!(
            "{} Keep work aligned with `{}` and `{}` and move the current frontier honestly across {}.",
            goal_sentence,
            spec_title,
            plan_title,
            fronts
        ),
        project_name: format!("{} Workspace", company_name),
        project_description: format!(
            "Repo-local execution workspace for `{}` rooted at {}.",
            blueprint.program.id,
            target_repo.display()
        ),
        workspace_name: format!("{} repo", humanize(&blueprint.program.id)),
    })
}

fn latest_markdown_title(target_repo: &Path, directory: &str) -> Option<String> {
    let path = target_repo.join(directory);
    let mut entries = std::fs::read_dir(path)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .collect::<Vec<_>>();
    entries.sort();
    let latest = entries.pop()?;
    markdown_title_for_file(&latest)
}

fn markdown_title_for_file(path: &Path) -> Option<String> {
    let body = std::fs::read_to_string(path).ok()?;
    body.lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(ToOwned::to_owned))
}

fn extract_goal_sentence(target_repo: &Path) -> Option<String> {
    let candidates = [
        latest_markdown_path(target_repo, "specs"),
        Some(target_repo.join("SPEC.md")),
        latest_markdown_path(target_repo, "plans"),
    ];
    for candidate in candidates.into_iter().flatten() {
        let body = std::fs::read_to_string(&candidate).ok()?;
        for paragraph in markdown_paragraphs(&body) {
            let lower = paragraph.to_ascii_lowercase();
            if lower.contains("whole-system goal is")
                || lower.contains("goal is to")
                || lower.starts_with("the goal is")
            {
                return Some(paragraph);
            }
        }
    }
    None
}

fn markdown_paragraphs(body: &str) -> Vec<String> {
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

fn latest_markdown_path(target_repo: &Path, directory: &str) -> Option<PathBuf> {
    let path = target_repo.join(directory);
    let mut entries = std::fs::read_dir(path)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .collect::<Vec<_>>();
    entries.sort();
    entries.pop()
}

fn ensure_private_data_dir(data_dir: &Path, target_repo: &Path) -> Result<()> {
    std::fs::create_dir_all(data_dir)?;
    if data_dir.starts_with(target_repo) {
        write_secrets_gitignore(data_dir)?;
    }
    Ok(())
}

fn write_secrets_gitignore(directory: &Path) -> Result<()> {
    let gitignore = directory.join(".gitignore");
    if gitignore.exists() {
        return Ok(());
    }
    std::fs::write(gitignore, "*\n!.gitignore\n")?;
    Ok(())
}

async fn ensure_company_goal(
    api_base: &str,
    company_id: &str,
    mission: &BootstrapMission,
    preferred_goal_id: Option<&str>,
) -> Result<PaperclipGoal> {
    let client = reqwest::Client::new();
    let goals = client
        .get(format!("{api_base}/api/companies/{company_id}/goals"))
        .send()
        .await
        .context("failed to list paperclip goals")?
        .error_for_status()
        .context("paperclip goal list request failed")?
        .json::<Vec<PaperclipGoal>>()
        .await
        .context("failed to parse paperclip goals response")?;

    let existing = preferred_goal_id
        .and_then(|goal_id| goals.iter().find(|goal| goal.id == goal_id))
        .or_else(|| goals.iter().find(|goal| goal.title == mission.goal_title));

    if let Some(goal) = existing {
        return client
            .patch(format!("{api_base}/api/goals/{}", goal.id))
            .json(&json!({
                "title": mission.goal_title,
                "description": mission.goal_description,
                "level": "company",
                "status": "active",
            }))
            .send()
            .await
            .context("failed to update paperclip goal")?
            .error_for_status()
            .context("paperclip goal update request failed")?
            .json::<PaperclipGoal>()
            .await
            .context("failed to parse updated paperclip goal");
    }

    client
        .post(format!("{api_base}/api/companies/{company_id}/goals"))
        .json(&json!({
            "title": mission.goal_title,
            "description": mission.goal_description,
            "level": "company",
            "status": "active",
        }))
        .send()
        .await
        .context("failed to create paperclip goal")?
        .error_for_status()
        .context("paperclip goal create request failed")?
        .json::<PaperclipGoal>()
        .await
        .context("failed to parse created paperclip goal")
}

async fn find_existing_company_id(api_base: &str, company_name: &str) -> Result<Option<String>> {
    let client = reqwest::Client::new();
    let companies = client
        .get(format!("{api_base}/api/companies"))
        .send()
        .await
        .context("failed to list paperclip companies")?
        .error_for_status()
        .context("paperclip company list request failed")?
        .json::<Vec<PaperclipCompany>>()
        .await
        .context("failed to parse paperclip company list response")?;
    Ok(companies
        .into_iter()
        .find(|company| company.name == company_name)
        .map(|company| company.id))
}

async fn cleanup_generated_agent_duplicates(
    api_base: &str,
    company_id: &str,
    desired_agents: &[BundleAgent],
    imported_agents: &[PaperclipImportAgent],
) -> Result<()> {
    let client = reqwest::Client::new();
    let existing_agents = client
        .get(format!("{api_base}/api/companies/{company_id}/agents"))
        .send()
        .await
        .context("failed to list paperclip agents for duplicate cleanup")?
        .error_for_status()
        .context("paperclip agent list request failed during duplicate cleanup")?
        .json::<Vec<PaperclipManagedAgent>>()
        .await
        .context("failed to parse paperclip agents for duplicate cleanup")?;

    for desired_agent in desired_agents {
        let keep_id = imported_agents
            .iter()
            .find(|agent| agent.slug == desired_agent.slug)
            .and_then(|agent| agent.id.as_deref());
        let Some(keep_id) = keep_id else {
            continue;
        };

        let duplicate_ids = existing_agents
            .iter()
            .filter(|agent| generated_agent_matches(agent, desired_agent))
            .filter(|agent| agent.id != keep_id)
            .map(|agent| agent.id.clone())
            .collect::<Vec<_>>();

        for duplicate_id in duplicate_ids {
            client
                .delete(format!("{api_base}/api/agents/{duplicate_id}"))
                .send()
                .await
                .with_context(|| {
                    format!("failed to delete duplicate paperclip agent {duplicate_id}")
                })?
                .error_for_status()
                .with_context(|| {
                    format!("paperclip duplicate agent delete failed for {duplicate_id}")
                })?;
        }
    }

    Ok(())
}

fn generated_agent_matches(agent: &PaperclipManagedAgent, desired_agent: &BundleAgent) -> bool {
    let Some(metadata) = agent.metadata.as_ref() else {
        return false;
    };
    if metadata.get("source").and_then(|value| value.as_str()) != Some("fabro.paperclip") {
        return false;
    }
    match desired_agent.slug.as_str() {
        "mission-ceo" => {
            metadata.get("type").and_then(|value| value.as_str()) == Some("mission_ceo")
        }
        "raspberry-orchestrator" => {
            metadata.get("type").and_then(|value| value.as_str()) == Some("raspberry_orchestrator")
        }
        _ => {
            metadata.get("laneKey").and_then(|value| value.as_str())
                == Some(desired_agent.slug.as_str())
        }
    }
}

async fn ensure_company_project(
    api_base: &str,
    company_id: &str,
    goal_id: &str,
    mission: &BootstrapMission,
    target_repo: &Path,
    preferred_project_id: Option<&str>,
) -> Result<PaperclipProject> {
    let client = reqwest::Client::new();
    let projects = client
        .get(format!("{api_base}/api/companies/{company_id}/projects"))
        .send()
        .await
        .context("failed to list paperclip projects")?
        .error_for_status()
        .context("paperclip project list request failed")?
        .json::<Vec<PaperclipProject>>()
        .await
        .context("failed to parse paperclip projects response")?;

    let existing = preferred_project_id
        .and_then(|project_id| projects.iter().find(|project| project.id == project_id))
        .or_else(|| {
            projects
                .iter()
                .find(|project| project.name == mission.project_name)
        });

    let project = if let Some(project) = existing {
        client
            .patch(format!("{api_base}/api/projects/{}", project.id))
            .json(&json!({
                "name": mission.project_name,
                "description": mission.project_description,
                "status": "in_progress",
                "goalIds": [goal_id],
            }))
            .send()
            .await
            .context("failed to update paperclip project")?
            .error_for_status()
            .context("paperclip project update request failed")?
            .json::<PaperclipProject>()
            .await
            .context("failed to parse updated paperclip project")?
    } else {
        client
            .post(format!("{api_base}/api/companies/{company_id}/projects"))
            .json(&json!({
                "name": mission.project_name,
                "description": mission.project_description,
                "status": "in_progress",
                "goalIds": [goal_id],
                "workspace": {
                    "name": mission.workspace_name,
                    "cwd": target_repo.display().to_string(),
                    "sourceType": "local_path",
                    "isPrimary": true,
                }
            }))
            .send()
            .await
            .context("failed to create paperclip project")?
            .error_for_status()
            .context("paperclip project create request failed")?
            .json::<PaperclipProject>()
            .await
            .context("failed to parse created paperclip project")?
    };

    ensure_project_workspace(api_base, &project.id, mission, target_repo).await?;
    Ok(project)
}

async fn ensure_project_workspace(
    api_base: &str,
    project_id: &str,
    mission: &BootstrapMission,
    target_repo: &Path,
) -> Result<()> {
    let client = reqwest::Client::new();
    let desired_cwd = target_repo.display().to_string();
    let workspaces = client
        .get(format!("{api_base}/api/projects/{project_id}/workspaces"))
        .send()
        .await
        .context("failed to list paperclip project workspaces")?
        .error_for_status()
        .context("paperclip project workspace list request failed")?
        .json::<Vec<PaperclipWorkspace>>()
        .await
        .context("failed to parse project workspaces response")?;

    if let Some(existing) = workspaces
        .iter()
        .find(|workspace| workspace.cwd.as_deref() == Some(desired_cwd.as_str()))
    {
        client
            .patch(format!(
                "{api_base}/api/projects/{project_id}/workspaces/{}",
                existing.id
            ))
            .json(&json!({
                "name": mission.workspace_name,
                "cwd": desired_cwd,
                "sourceType": "local_path",
                "isPrimary": true,
            }))
            .send()
            .await
            .context("failed to update paperclip workspace")?
            .error_for_status()
            .context("paperclip workspace update request failed")?;
        return Ok(());
    }

    client
        .post(format!("{api_base}/api/projects/{project_id}/workspaces"))
        .json(&json!({
            "name": mission.workspace_name,
            "cwd": desired_cwd,
            "sourceType": "local_path",
            "isPrimary": true,
        }))
        .send()
        .await
        .context("failed to create paperclip workspace")?
        .error_for_status()
        .context("paperclip workspace create request failed")?;
    Ok(())
}

fn load_bootstrap_state(path: &Path) -> Result<serde_json::Value> {
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn save_bootstrap_state(path: &Path, value: &serde_json::Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn humanize(value: &str) -> String {
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

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn parse_json_stdout<T: DeserializeOwned>(stdout: &[u8], command_name: &str) -> Result<T> {
    let raw = String::from_utf8(stdout.to_vec())
        .with_context(|| format!("{command_name} did not emit valid UTF-8"))?;
    if let Ok(parsed) = serde_json::from_str::<T>(raw.trim()) {
        return Ok(parsed);
    }
    let Some(json_start) = raw.find('{').or_else(|| raw.find('[')) else {
        bail!("{command_name} did not emit JSON");
    };
    serde_json::from_str(&raw[json_start..])
        .with_context(|| format!("{command_name} output did not contain a parseable JSON payload"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fabro_synthesis::{
        BlueprintArtifact, BlueprintLane, BlueprintProgram, BlueprintUnit, ProgramBlueprint,
    };
    use tempfile::tempdir;

    #[test]
    fn derive_bootstrap_mission_prefers_repo_goal_sentence() {
        let temp = tempdir().expect("tempdir");
        std::fs::write(
            temp.path().join("SPEC.md"),
            "# Zend Spec\n\nThe whole-system goal is to launch Zend as an agent-first product on top of the\nhome command center.\n",
        )
        .expect("write spec");
        std::fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        std::fs::write(
            temp.path().join("plans/2026-03-19-zend.md"),
            "# Build Zend Home Command Center\n",
        )
        .expect("write plan");
        let blueprint = sample_blueprint();

        let mission =
            derive_bootstrap_mission(&blueprint, temp.path(), "Zend").expect("derive mission");

        assert!(mission
            .goal_description
            .contains("agent-first product on top of the home command center"));
        assert!(mission
            .company_description
            .contains("Build Zend Home Command Center"));
    }

    #[test]
    fn lane_adapter_type_prefers_claude_for_reports() {
        let lane = BlueprintLane {
            id: "proof".to_string(),
            kind: Default::default(),
            title: "Proof".to_string(),
            family: "qa".to_string(),
            workflow_family: None,
            slug: None,
            template: WorkflowTemplate::RecurringReport,
            goal: "Verify proof".to_string(),
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
        let unit = BlueprintUnit {
            id: "proof-and-validation".to_string(),
            title: "Proof and Validation".to_string(),
            output_root: PathBuf::from("proof"),
            artifacts: Vec::new(),
            milestones: Vec::new(),
            lanes: vec![lane.clone()],
        };

        assert_eq!(lane_role(&unit, &lane), "qa");
        assert_eq!(lane_adapter_type(&unit, &lane), "claude_local");
    }

    #[test]
    fn parse_json_stdout_recovers_payload_after_pnpm_banner() {
        let raw = br#"
> paperclip@ paperclipai /home/r/coding/paperclip
> node cli/src/index.ts company import --json

{"company":{"id":"123","name":"Zend","action":"created"},"agents":[]}
"#;

        let parsed: PaperclipImportResult =
            parse_json_stdout(raw, "paperclip company import").expect("parse payload");

        assert_eq!(parsed.company.id, "123");
        assert_eq!(parsed.company.name, "Zend");
    }

    fn sample_blueprint() -> ProgramBlueprint {
        ProgramBlueprint {
            version: 1,
            program: BlueprintProgram {
                id: "zend".to_string(),
                max_parallel: 2,
                state_path: None,
                run_dir: None,
            },
            inputs: Default::default(),
            package: Default::default(),
            units: vec![BlueprintUnit {
                id: "command-center-client".to_string(),
                title: "Command Center Client".to_string(),
                output_root: PathBuf::from("command-center-client"),
                artifacts: vec![BlueprintArtifact {
                    id: "plan".to_string(),
                    path: PathBuf::from("prompts/bootstrap/command-center-client/plan.md"),
                }],
                milestones: Vec::new(),
                lanes: vec![BlueprintLane {
                    id: "command-center-client".to_string(),
                    kind: Default::default(),
                    title: "Command Center Client".to_string(),
                    family: "bootstrap".to_string(),
                    workflow_family: None,
                    slug: None,
                    template: WorkflowTemplate::Bootstrap,
                    goal: "Ship the command center client".to_string(),
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
                }],
            }],
        }
    }
}
