use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use chrono::{SecondsFormat, Utc};
use clap::{ArgAction, Args, Subcommand};
use fabro_synthesis::{
    author_blueprint_for_create, load_blueprint, render_blueprint, save_blueprint, BlueprintUnit,
    ProgramBlueprint, RenderRequest, WorkflowTemplate,
};
use raspberry_supervisor::{
    evaluate::evaluate_with_state, load_plan_registry, load_plan_registry_from_planning_root,
    refresh_program_state, AutodevProvenance, EvaluatedLane, FailureKind, LaneExecutionStatus,
    MaintenanceMode, PlanMappingSource, PlanMatrix, PlanRegistry, PlanStatusRow, ProgramManifest,
    ProgramRuntimeState,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

const SYNC_MARKER_PREFIX: &str = "fabro.paperclip.sync-key:";
const PAPERCLIP_DEFAULT_AUTOMATION_MODEL: &str = "MiniMax-M2.7-highspeed";

#[derive(Debug, Args)]
pub struct PaperclipArgs {
    #[command(subcommand)]
    pub command: PaperclipCommand,
}

#[derive(Debug, Subcommand)]
pub enum PaperclipCommand {
    /// Generate and bootstrap a repo-local Paperclip company on top of the current blueprint
    Bootstrap(PaperclipBootstrapArgs),
    /// Refresh the repo-local Paperclip bundle, company, and synchronized frontier issues
    Refresh(PaperclipRefreshArgs),
    /// Start or reuse the repo-local Paperclip server
    Start(PaperclipServerArgs),
    /// Stop the repo-local Paperclip server
    Stop(PaperclipServerArgs),
    /// Show repo-local Paperclip server and frontier status
    Status(PaperclipServerArgs),
    /// Print recent lines from the repo-local Paperclip server log
    Logs(PaperclipLogsArgs),
    /// Invoke a Paperclip heartbeat for the orchestrator or a generated agent
    Wake(PaperclipWakeArgs),
}

#[derive(Debug, Args, Clone)]
pub struct PaperclipRepoArgs {
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
}

#[derive(Debug, Args, Clone)]
pub struct PaperclipBootstrapArgs {
    #[command(flatten)]
    pub repo: PaperclipRepoArgs,
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub apply: bool,
}

#[derive(Debug, Args, Clone)]
pub struct PaperclipRefreshArgs {
    #[command(flatten)]
    pub repo: PaperclipRepoArgs,
}

#[derive(Debug, Args, Clone)]
pub struct PaperclipServerArgs {
    #[command(flatten)]
    pub repo: PaperclipRepoArgs,
    #[arg(long, default_value_t = false, action = ArgAction::Set)]
    pub watch: bool,
    #[arg(long, default_value_t = 3)]
    pub interval_secs: u64,
}

#[derive(Debug, Args, Clone)]
pub struct PaperclipLogsArgs {
    #[command(flatten)]
    pub repo: PaperclipRepoArgs,
    #[arg(long, default_value_t = 80)]
    pub lines: usize,
}

#[derive(Debug, Args, Clone)]
pub struct PaperclipWakeArgs {
    #[command(flatten)]
    pub repo: PaperclipRepoArgs,
    #[arg(long, default_value = "raspberry-orchestrator")]
    pub agent: String,
}

#[derive(Debug, Clone)]
struct PaperclipPaths {
    target_repo: PathBuf,
    program_id: String,
    company_name: String,
    data_dir: PathBuf,
    api_base: String,
    blueprint_path: PathBuf,
    manifest_path: PathBuf,
    bundle_root: PathBuf,
    scripts_root: PathBuf,
    paperclip_cli_script_path: PathBuf,
    orchestrator_script_path: PathBuf,
    minimax_agent_script_path: PathBuf,
    run_script_path: PathBuf,
    bootstrap_state_path: PathBuf,
}

struct PaperclipRepoContext {
    paths: PaperclipPaths,
    mission: BootstrapMission,
    frontier: FrontierSyncModel,
    plan_matrix: Option<PlanMatrix>,
    plan_dashboard: Option<PlanDashboardModel>,
    bundle: GeneratedBundle,
}

struct PaperclipApplySummary {
    company_id: String,
    goal_id: String,
    project_id: String,
    workspace_id: String,
    synced_issue_count: usize,
    synced_document_count: usize,
    synced_secret_count: usize,
    synced_comment_count: usize,
    synced_work_product_count: usize,
    synced_attachment_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GitBootstrapResult {
    initialized: bool,
    committed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GitTransientCleanupResult {
    removed_paths: usize,
}

const BOOTSTRAP_GITIGNORE_LINES: &[&str] = &[
    ".raspberry/",
    ".paperclip/",
    "malinka/paperclip/*/bootstrap-state.json",
];

struct PaperclipServerStatus {
    pid: Option<u32>,
    pid_live: bool,
    server_ready: bool,
    controller_pid: Option<u32>,
    controller_pid_live: bool,
    controller_acquired_at: Option<String>,
    controller_provenance: Option<String>,
    fabro_provenance: Option<String>,
    bootstrap_state: Option<serde_json::Value>,
    frontier: Option<FrontierSyncModel>,
    plan_matrix: Option<PlanMatrix>,
    openai_api_key_present: bool,
    anthropic_api_key_present: bool,
    local_cli_export_count: usize,
    synced_secret_count: usize,
    synced_work_product_count: usize,
    synced_attachment_count: usize,
    cost_summary: Option<PaperclipCostSummary>,
    budget_overview: Option<PaperclipBudgetOverview>,
    pending_approvals: usize,
    maintenance: Option<MaintenanceMode>,
}

pub async fn bootstrap_command(args: &PaperclipBootstrapArgs) -> Result<()> {
    let git_bootstrap = if args.apply {
        Some(ensure_target_repo_initialized(&args.repo.target_repo)?)
    } else {
        None
    };
    let context = prepare_paperclip_context(&args.repo)?;
    print_paperclip_context_summary(&context.paths, &context.mission.goal_title);
    if !args.apply {
        println!("Applied: no");
        return Ok(());
    }
    let summary = apply_paperclip_context(&context, args.repo.paperclip_cmd.as_deref()).await?;
    if let Some(git_bootstrap) = git_bootstrap {
        print_git_bootstrap_summary(git_bootstrap);
    }
    print_paperclip_apply_summary("Applied", &summary);
    let cleanup = prune_tracked_transient_paths(&context.paths.target_repo)?;
    print_git_transient_cleanup_summary(cleanup);
    Ok(())
}

pub async fn refresh_command(args: &PaperclipRefreshArgs) -> Result<()> {
    let git_bootstrap = ensure_target_repo_initialized(&args.repo.target_repo)?;
    let context = prepare_paperclip_context(&args.repo)?;
    print_paperclip_context_summary(&context.paths, &context.mission.goal_title);
    let summary = apply_paperclip_context(&context, args.repo.paperclip_cmd.as_deref()).await?;
    print_git_bootstrap_summary(git_bootstrap);
    print_paperclip_apply_summary("Refreshed", &summary);
    let cleanup = prune_tracked_transient_paths(&context.paths.target_repo)?;
    print_git_transient_cleanup_summary(cleanup);
    Ok(())
}

pub async fn start_command(args: &PaperclipServerArgs) -> Result<()> {
    let paths = resolve_paperclip_paths(&args.repo);
    ensure_private_data_dir(&paths.data_dir, &paths.target_repo)?;
    ensure_paperclip_server(
        args.repo.paperclip_cmd.as_deref(),
        &paths.data_dir,
        &paths.api_base,
    )
    .await?;
    let status = collect_paperclip_server_status(&paths).await?;
    print_paperclip_server_status(&paths, &status);
    Ok(())
}

pub async fn stop_command(args: &PaperclipServerArgs) -> Result<()> {
    let paths = resolve_paperclip_paths(&args.repo);
    stop_paperclip_server(&paths.data_dir, &paths.api_base).await?;
    let status = collect_paperclip_server_status(&paths).await?;
    print_paperclip_server_status(&paths, &status);
    Ok(())
}

pub async fn status_command(args: &PaperclipServerArgs) -> Result<()> {
    let paths = resolve_paperclip_paths(&args.repo);
    if args.watch {
        loop {
            let status = collect_paperclip_server_status(&paths).await?;
            print!("\x1b[2J\x1b[H");
            print_paperclip_server_status(&paths, &status);
            std::io::stdout().flush().ok();
            tokio::time::sleep(Duration::from_secs(args.interval_secs.max(1))).await;
        }
    }
    let status = collect_paperclip_server_status(&paths).await?;
    print_paperclip_server_status(&paths, &status);
    Ok(())
}

pub async fn logs_command(args: &PaperclipLogsArgs) -> Result<()> {
    let paths = resolve_paperclip_paths(&args.repo);
    let log_path = paths.data_dir.join("server.log");
    let contents = std::fs::read_to_string(&log_path)
        .with_context(|| format!("failed to read {}", log_path.display()))?;
    print!("{}", last_lines(&contents, args.lines));
    Ok(())
}

pub async fn wake_command(args: &PaperclipWakeArgs) -> Result<()> {
    let paths = resolve_paperclip_paths(&args.repo);
    if let Some(maintenance) = load_paperclip_maintenance(&paths)? {
        println!("Program: {}", paths.program_id);
        println!("Agent: {}", args.agent);
        println!("Maintenance: enabled");
        println!("Reason: {}", maintenance.reason);
        println!(
            "Wake command skipped while maintenance mode is enabled. Use `{}` to inspect current state.",
            paperclip_status_command(&paths)
        );
        return Ok(());
    }
    ensure_private_data_dir(&paths.data_dir, &paths.target_repo)?;
    ensure_paperclip_server(
        args.repo.paperclip_cmd.as_deref(),
        &paths.data_dir,
        &paths.api_base,
    )
    .await?;
    let state = load_bootstrap_state(&paths.bootstrap_state_path).with_context(|| {
        format!(
            "failed to load bootstrap state {}",
            paths.bootstrap_state_path.display()
        )
    })?;
    let company_id = state
        .get("companyId")
        .and_then(|value| value.as_str())
        .context("bootstrap state is missing companyId")?;
    let agent =
        resolve_managed_agent(&paths.api_base, company_id, &args.agent, Some(&state)).await?;
    let frontier_before = load_optional_frontier_sync_model(&paths)?;
    let run = invoke_agent_heartbeat(&paths.api_base, &agent.id).await?;
    let agent_slug = agent.slug.as_deref().unwrap_or(args.agent.as_str());
    let followthrough = if agent_slug == "raspberry-orchestrator" {
        Some(follow_through_orchestrator_wake(&paths, frontier_before.as_ref()).await?)
    } else {
        None
    };

    println!("Program: {}", paths.program_id);
    println!("Company ID: {company_id}");
    println!("Agent: {} ({})", agent_slug, agent.id);
    println!(
        "Wake command: {}",
        paperclip_wake_command(&paths, agent_slug)
    );
    println!("Result: {}", render_heartbeat_invoke_result(&run));
    if let Some(frontier) = frontier_before.as_ref() {
        println!("Frontier before:\n{}", render_frontier_summary(frontier));
    }
    if let Some(followthrough) = followthrough.as_ref() {
        println!(
            "Followthrough: {}",
            render_wake_followthrough(followthrough)
        );
        if let Some(frontier) = followthrough.after.as_ref() {
            println!("Frontier after:\n{}", render_frontier_summary(frontier));
        }
        if wake_followthrough_changed(frontier_before.as_ref(), followthrough) {
            let context = prepare_paperclip_context(&args.repo)?;
            let summary =
                apply_paperclip_context(&context, args.repo.paperclip_cmd.as_deref()).await?;
            print_paperclip_apply_summary("Refreshed after wake", &summary);
        }
    }
    Ok(())
}

fn resolve_paperclip_paths(args: &PaperclipRepoArgs) -> PaperclipPaths {
    let target_repo = args.target_repo.clone();
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
    let company_name = args
        .company_name
        .clone()
        .unwrap_or_else(|| humanize(&program_id));
    let data_dir = args
        .data_dir
        .clone()
        .unwrap_or_else(|| target_repo.join(".paperclip"));
    let api_base = args
        .api_base
        .clone()
        .unwrap_or_else(|| "http://127.0.0.1:3100".to_string());
    let pkg_dir = fabro_synthesis::blueprint::DEFAULT_PACKAGE_DIR;
    let bundle_root = target_repo
        .join(pkg_dir)
        .join("paperclip")
        .join(&program_id);
    let scripts_root = bundle_root.join("scripts");
    let blueprint_path = target_repo
        .join(pkg_dir)
        .join("blueprints")
        .join(format!("{program_id}.yaml"));
    let manifest_path = target_repo
        .join(pkg_dir)
        .join("programs")
        .join(format!("{program_id}.yaml"));

    PaperclipPaths {
        target_repo,
        program_id,
        company_name,
        data_dir,
        api_base,
        blueprint_path,
        manifest_path,
        bundle_root: bundle_root.clone(),
        scripts_root: scripts_root.clone(),
        paperclip_cli_script_path: scripts_root.join("fabro-paperclip.sh"),
        orchestrator_script_path: scripts_root.join("raspberry-orchestrator.sh"),
        minimax_agent_script_path: scripts_root.join("fabro-agent-minimax.sh"),
        run_script_path: scripts_root.join("run-paperclip.sh"),
        bootstrap_state_path: bundle_root.join("bootstrap-state.json"),
    }
}

fn prepare_paperclip_context(args: &PaperclipRepoArgs) -> Result<PaperclipRepoContext> {
    let paths = resolve_paperclip_paths(args);
    let blueprint = ensure_paperclip_blueprint(&paths)?;
    ensure_private_data_dir(&paths.data_dir, &paths.target_repo)?;
    std::fs::create_dir_all(&paths.scripts_root)?;

    let fabro_binary = current_fabro_binary()?;
    let raspberry_binary = current_raspberry_binary();
    let fabro_agent_binary = current_fabro_agent_binary();
    let fabro_repo = default_fabro_repo();
    write_paperclip_cli_script(
        &paths.paperclip_cli_script_path,
        &fabro_binary,
        &paths.target_repo,
        &paths.program_id,
    )?;
    write_orchestrator_script(
        &paths.orchestrator_script_path,
        &paths.target_repo,
        &paths.manifest_path,
        &fabro_binary,
        raspberry_binary.as_deref(),
    )?;
    write_minimax_agent_script(
        &paths.minimax_agent_script_path,
        fabro_agent_binary.as_deref(),
        fabro_repo.as_deref(),
    )?;
    write_run_script(
        &paths.run_script_path,
        default_paperclip_repo(),
        &paths.data_dir,
    )?;

    let mission = derive_bootstrap_mission(&blueprint, &paths.target_repo, &paths.company_name)?;
    let frontier = load_frontier_sync_model(
        &paths.program_id,
        &paths.target_repo,
        &paths.manifest_path,
        &paths.orchestrator_script_path,
        &paperclip_wake_command(&paths, "raspberry-orchestrator"),
        &paperclip_refresh_command(&paths),
        &paperclip_status_command(&paths),
    )?;
    let plan_matrix = raspberry_supervisor::load_plan_matrix(&paths.manifest_path).ok();
    let plan_registry = load_primary_or_genesis_plan_registry(&paths.target_repo);
    let plan_dashboard = plan_registry.as_ref().map(|reg| {
        build_plan_dashboard_model(&paths.program_id, reg, plan_matrix.as_ref(), &frontier)
    });
    let bundle = build_company_bundle(
        &blueprint,
        &paths.target_repo,
        &paths.company_name,
        &mission,
        &paths.orchestrator_script_path,
        &paths.minimax_agent_script_path,
        &frontier,
        plan_matrix.as_ref(),
        plan_dashboard.as_ref(),
    )?;
    write_bundle(&paths.bundle_root, &bundle)?;

    Ok(PaperclipRepoContext {
        paths,
        mission,
        frontier,
        plan_matrix,
        plan_dashboard,
        bundle,
    })
}

fn ensure_paperclip_blueprint(paths: &PaperclipPaths) -> Result<ProgramBlueprint> {
    let blueprint = if paths.blueprint_path.exists() {
        load_blueprint(&paths.blueprint_path)?
    } else {
        let authored = author_blueprint_for_create(&paths.target_repo, Some(&paths.program_id))?;
        save_blueprint(&paths.blueprint_path, &authored.blueprint)?;
        authored.blueprint
    };
    render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: &paths.target_repo,
    })?;
    Ok(blueprint)
}

async fn apply_paperclip_context(
    context: &PaperclipRepoContext,
    paperclip_cmd_override: Option<&str>,
) -> Result<PaperclipApplySummary> {
    let paths = &context.paths;
    let paperclip_cmd = paperclip_command(paperclip_cmd_override);
    ensure_paperclip_server(paperclip_cmd_override, &paths.data_dir, &paths.api_base).await?;
    let saved_company_id = load_bootstrap_state(&paths.bootstrap_state_path)
        .ok()
        .and_then(|state| {
            state
                .get("companyId")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
        });
    let existing_company_id = resolve_existing_company_id(
        &paths.api_base,
        &paths.company_name,
        saved_company_id.as_deref(),
    )
    .await?;
    if let Some(existing_company_id) = existing_company_id.as_deref() {
        let pruned = prune_existing_generated_agents(
            &paths.api_base,
            existing_company_id,
            &context.bundle.agents,
        )
        .await?;
        if pruned > 0 {
            eprintln!(
                "paperclip refresh: pruned {pruned} previously generated agent(s) before import"
            );
        }
    }
    let import_result = import_company_package(
        &paperclip_cmd,
        &paths.data_dir,
        &paths.api_base,
        &paths.bundle_root,
        &paths.company_name,
        existing_company_id.as_deref(),
    )
    .await?;
    let company_id = import_result.company.id.clone();
    let existing_state = load_bootstrap_state(&paths.bootstrap_state_path).ok();
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
    let existing_workspace_id = existing_state
        .as_ref()
        .and_then(|state| state.get("workspace"))
        .and_then(|value| value.get("id"))
        .and_then(|value| value.as_str());
    let goal = ensure_company_goal(
        &paths.api_base,
        &company_id,
        &context.mission,
        &context.frontier,
        existing_goal_id,
    )
    .await?;
    let project_sync = ensure_company_project(
        &paths.api_base,
        &company_id,
        &goal.id,
        &context.mission,
        &paths.target_repo,
        &context.frontier,
        existing_project_id,
        existing_workspace_id,
    )
    .await?;
    cleanup_generated_agent_duplicates(
        &paths.api_base,
        &company_id,
        &context.bundle.agents,
        &import_result.agents,
    )
    .await?;
    let synced_issue_ids = sync_coordination_issues(
        &paths.api_base,
        &company_id,
        &goal.id,
        &project_sync.project.id,
        &project_sync.workspace.id,
        &context.frontier,
        context.plan_dashboard.as_ref(),
        &context.bundle.agents,
        &import_result.agents,
        existing_state.as_ref(),
    )
    .await?;
    let synced_document_count = sync_coordination_documents(
        &paths.api_base,
        &context.frontier,
        context.plan_matrix.as_ref(),
        context.plan_dashboard.as_ref(),
        &synced_issue_ids,
    )
    .await?;
    let synced_comment_count = sync_coordination_comments(
        &paths.api_base,
        &context.frontier,
        context.plan_dashboard.as_ref(),
        &synced_issue_ids,
        existing_state.as_ref(),
    )
    .await?;
    let attachment_sync = sync_coordination_attachments(
        &paths.api_base,
        &company_id,
        &paths.target_repo,
        &context.frontier,
        &synced_issue_ids,
    )
    .await?;
    let synced_work_product_count = sync_coordination_work_products(
        &paths.api_base,
        &context.frontier,
        context.plan_dashboard.as_ref(),
        &synced_issue_ids,
        &attachment_sync.attachments_by_scope,
    )
    .await?;
    let synced_attachment_count = attachment_sync.synced_count;
    let synced_secrets =
        sync_company_secrets(&paths.api_base, &company_id, existing_state.as_ref()).await?;
    wire_generated_agent_secrets(
        &paths.api_base,
        &context.bundle.agents,
        &import_result.agents,
        &synced_secrets,
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
        "id": project_sync.project.id,
        "name": project_sync.project.name,
    });
    bootstrap_state["workspace"] = json!({
        "id": project_sync.workspace.id,
        "cwd": project_sync.workspace.cwd,
    });
    bootstrap_state["frontierSync"] = json!({
        "program": context.frontier.program.clone(),
        "manifestPath": context.frontier.manifest_path.display().to_string(),
        "statePath": context.frontier.state_path.display().to_string(),
        "wakeCommand": context.frontier.wake_command.clone(),
        "routeCommand": context.frontier.route_command.clone(),
        "refreshCommand": context.frontier.refresh_command.clone(),
        "statusCommand": context.frontier.status_command.clone(),
        "summary": context.frontier.summary.clone(),
        "provenance": load_frontier_provenance(&context.paths.manifest_path),
        "issueIds": synced_issue_ids,
        "snapshots": frontier_snapshots_json(&context.frontier),
        "documentsSynced": synced_document_count,
        "workProductsSynced": synced_work_product_count,
        "attachmentsSynced": synced_attachment_count,
        "updatedAt": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    });
    if let Some(dashboard) = context.plan_dashboard.as_ref() {
        let plan_issue_ids: BTreeMap<String, String> = synced_issue_ids
            .iter()
            .filter(|(key, _)| key.starts_with("plan/"))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        bootstrap_state["planSync"] = json!({
            "program": dashboard.program,
            "planCount": dashboard.plans.len(),
            "snapshots": plan_snapshots_json(dashboard),
            "issueIds": plan_issue_ids,
            "updatedAt": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        });
    }
    bootstrap_state["secrets"] = synced_secrets_to_json(&synced_secrets);
    save_bootstrap_state(&paths.bootstrap_state_path, &bootstrap_state)?;
    install_local_cli_for_agents(
        &paperclip_cmd,
        &paths.data_dir,
        &paths.api_base,
        &company_id,
        &context.bundle.agents,
        &import_result.agents,
    )?;

    Ok(PaperclipApplySummary {
        company_id,
        goal_id: goal.id,
        project_id: project_sync.project.id,
        workspace_id: project_sync.workspace.id,
        synced_issue_count: synced_issue_ids.len(),
        synced_document_count,
        synced_secret_count: synced_secrets.len(),
        synced_comment_count,
        synced_work_product_count,
        synced_attachment_count,
    })
}

async fn prune_existing_generated_agents(
    api_base: &str,
    company_id: &str,
    desired_agents: &[BundleAgent],
) -> Result<usize> {
    let client = reqwest::Client::new();
    let existing_agents = client
        .get(format!("{api_base}/api/companies/{company_id}/agents"))
        .send()
        .await
        .context("failed to list paperclip agents before import")?
        .error_for_status()
        .context("paperclip agent list request failed before import")?
        .json::<Vec<PaperclipManagedAgent>>()
        .await
        .context("failed to parse paperclip agents before import")?;

    let generated_agents = existing_agents
        .into_iter()
        .filter(|agent| {
            agent.metadata.as_ref().and_then(|metadata| {
                metadata
                    .get("source")
                    .and_then(|value| value.as_str())
                    .map(|value| value == "fabro.paperclip")
            }) == Some(true)
        })
        .collect::<Vec<_>>();
    if generated_agents.is_empty() {
        return Ok(0);
    }

    let desired_keys = desired_agents
        .iter()
        .filter_map(desired_agent_identity_key)
        .collect::<BTreeSet<_>>();
    let mut live_counts = BTreeMap::<String, usize>::new();
    for agent in &generated_agents {
        let Some(key) = managed_agent_identity_key(agent) else {
            return Ok(0);
        };
        *live_counts.entry(key).or_insert(0) += 1;
    }
    let live_keys = live_counts.keys().cloned().collect::<BTreeSet<_>>();
    let duplicates_present = live_counts.values().any(|count| *count > 1);
    let dirty = duplicates_present
        || generated_agents.len() > desired_agents.len()
        || live_keys != desired_keys;
    if !dirty {
        return Ok(0);
    }

    let generated_ids = generated_agents
        .into_iter()
        .map(|agent| agent.id)
        .collect::<Vec<_>>();

    for agent_id in &generated_ids {
        client
            .delete(format!("{api_base}/api/agents/{agent_id}"))
            .send()
            .await
            .with_context(|| format!("failed to delete generated paperclip agent {agent_id}"))?
            .error_for_status()
            .with_context(|| {
                format!("paperclip agent delete request failed for generated agent {agent_id}")
            })?;
    }

    Ok(generated_ids.len())
}

fn desired_agent_identity_key(agent: &BundleAgent) -> Option<String> {
    if agent.metadata_type == Some("mission_ceo") {
        return None;
    }
    if let (Some(unit), Some(lane_key)) = (agent.unit.as_deref(), agent.lane_key.as_deref()) {
        return Some(format!("lane:{unit}:{lane_key}"));
    }
    if let Some(metadata_type) = agent.metadata_type {
        return Some(format!("type:{metadata_type}"));
    }
    None
}

fn managed_agent_identity_key(agent: &PaperclipManagedAgent) -> Option<String> {
    let metadata = agent.metadata.as_ref()?;
    if let Some(metadata_type) = metadata.get("type").and_then(|value| value.as_str()) {
        return Some(format!("type:{metadata_type}"));
    }
    let unit = metadata.get("unit").and_then(|value| value.as_str())?;
    let lane_key = metadata.get("laneKey").and_then(|value| value.as_str())?;
    Some(format!("lane:{unit}:{lane_key}"))
}

fn print_paperclip_context_summary(paths: &PaperclipPaths, goal_title: &str) {
    println!("Program: {}", paths.program_id);
    println!("Paperclip bundle: {}", paths.bundle_root.display());
    println!("Data dir: {}", paths.data_dir.display());
    println!("API base: {}", paths.api_base);
    println!("Company goal: {goal_title}");
}

fn print_git_bootstrap_summary(result: GitBootstrapResult) {
    let status = match (result.initialized, result.committed) {
        (false, false) => "existing repo",
        (true, false) => "initialized empty repo",
        (_, true) => "initialized repo and seeded initial commit",
    };
    println!("Git bootstrap: {status}");
}

fn print_git_transient_cleanup_summary(result: GitTransientCleanupResult) {
    if result.removed_paths == 0 {
        return;
    }
    println!(
        "Git transient cleanup: untracked {} runtime files from index",
        result.removed_paths
    );
}

fn print_paperclip_apply_summary(label: &str, summary: &PaperclipApplySummary) {
    println!("{label}: yes");
    println!("Company ID: {}", summary.company_id);
    println!("Goal ID: {}", summary.goal_id);
    println!("Project ID: {}", summary.project_id);
    println!("Workspace ID: {}", summary.workspace_id);
    println!("Synced issues: {}", summary.synced_issue_count);
    println!("Synced documents: {}", summary.synced_document_count);
    println!("Synced secrets: {}", summary.synced_secret_count);
    println!("Synced comments: {}", summary.synced_comment_count);
    println!(
        "Synced work products: {}",
        summary.synced_work_product_count
    );
    println!("Synced attachments: {}", summary.synced_attachment_count);
}

async fn collect_paperclip_server_status(paths: &PaperclipPaths) -> Result<PaperclipServerStatus> {
    let pid = read_pid_file(&paths.data_dir.join("server.pid"))?;
    let controller = load_controller_status(paths);
    let bootstrap_state = load_bootstrap_state(&paths.bootstrap_state_path).ok();
    let company_id = bootstrap_state
        .as_ref()
        .and_then(|state| state.get("companyId"))
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);
    let frontier = if paths.manifest_path.exists() {
        load_frontier_sync_model(
            &paths.program_id,
            &paths.target_repo,
            &paths.manifest_path,
            &paths.orchestrator_script_path,
            &paperclip_wake_command(paths, "raspberry-orchestrator"),
            &paperclip_refresh_command(paths),
            &paperclip_status_command(paths),
        )
        .ok()
    } else {
        None
    };
    let cost_summary = if let Some(company_id) = company_id.as_deref() {
        fetch_company_cost_summary(&paths.api_base, company_id)
            .await
            .ok()
    } else {
        None
    };
    let budget_overview = if let Some(company_id) = company_id.as_deref() {
        fetch_company_budget_overview(&paths.api_base, company_id)
            .await
            .ok()
    } else {
        None
    };
    let pending_approvals = if let Some(company_id) = company_id.as_deref() {
        fetch_pending_approval_count(&paths.api_base, company_id)
            .await
            .unwrap_or_default()
    } else {
        0
    };
    let plan_matrix = if paths.manifest_path.exists() {
        raspberry_supervisor::load_plan_matrix(&paths.manifest_path).ok()
    } else {
        None
    };
    let maintenance = load_paperclip_maintenance(paths).ok().flatten();

    Ok(PaperclipServerStatus {
        pid,
        pid_live: pid.map(process_is_running).unwrap_or(false),
        server_ready: paperclip_server_ready(&paths.api_base).await,
        controller_pid: controller.pid,
        controller_pid_live: controller.pid_live,
        controller_acquired_at: controller.acquired_at,
        controller_provenance: controller.controller_provenance,
        fabro_provenance: controller.fabro_provenance,
        bootstrap_state,
        frontier,
        plan_matrix,
        openai_api_key_present: std::env::var_os("OPENAI_API_KEY").is_some(),
        anthropic_api_key_present: std::env::var_os("ANTHROPIC_API_KEY").is_some(),
        local_cli_export_count: count_local_cli_exports(&paths.data_dir),
        synced_secret_count: count_synced_secrets(paths),
        synced_work_product_count: count_synced_work_products(paths),
        synced_attachment_count: count_synced_attachments(paths),
        cost_summary,
        budget_overview,
        pending_approvals,
        maintenance,
    })
}

fn print_paperclip_server_status(paths: &PaperclipPaths, status: &PaperclipServerStatus) {
    println!("Program: {}", paths.program_id);
    println!("Data dir: {}", paths.data_dir.display());
    println!("API base: {}", paths.api_base);
    println!("Server ready: {}", yes_no(status.server_ready));
    println!(
        "Managed PID: {}",
        render_pid_status(status.pid, status.pid_live)
    );
    println!(
        "Server log: {}",
        paths.data_dir.join("server.log").display()
    );
    println!(
        "Raspberry controller: {}",
        render_pid_status(status.controller_pid, status.controller_pid_live)
    );
    if let Some(acquired_at) = status.controller_acquired_at.as_ref() {
        println!("Controller acquired at: {acquired_at}");
    }
    if let Some(provenance) = status.controller_provenance.as_ref() {
        println!("Controller provenance: {provenance}");
    }
    if let Some(provenance) = status.fabro_provenance.as_ref() {
        println!("Fabro provenance: {provenance}");
    }
    println!(
        "OPENAI_API_KEY present: {}",
        yes_no(status.openai_api_key_present)
    );
    println!(
        "ANTHROPIC_API_KEY present: {}",
        yes_no(status.anthropic_api_key_present)
    );
    println!("Local CLI exports: {}", status.local_cli_export_count);
    println!("Synced secrets: {}", status.synced_secret_count);
    println!("Synced work products: {}", status.synced_work_product_count);
    println!("Synced attachments: {}", status.synced_attachment_count);
    if let Some(costs) = status.cost_summary.as_ref() {
        println!(
            "Company spend: {} / {} cents ({:.1}%)",
            costs.spend_cents, costs.budget_cents, costs.utilization_percent
        );
    }
    if let Some(overview) = status.budget_overview.as_ref() {
        println!(
            "Budget overview: pending approvals {}, paused agents {}, paused projects {}",
            overview.pending_approval_count,
            overview.paused_agent_count,
            overview.paused_project_count
        );
    }
    println!("Pending approvals: {}", status.pending_approvals);
    if let Some(maintenance) = status.maintenance.as_ref() {
        println!("Maintenance mode: enabled");
        println!("Maintenance reason: {}", maintenance.reason);
    }
    if let Some(frontier) = status.frontier.as_ref() {
        println!("Frontier summary:");
        println!("{}", render_frontier_summary(frontier));
        println!("Frontier lanes:");
        println!("{}", render_frontier_lane_sets(frontier));
        let details = render_frontier_detail_sections(frontier);
        if !details.is_empty() {
            println!("Frontier details:");
            println!("{details}");
        }
    }
    if let Some(plan_matrix) = status.plan_matrix.as_ref() {
        println!("Plan matrix:");
        println!("{}", raspberry_supervisor::render_plan_matrix(plan_matrix));
    }
    if let Some(state) = status.bootstrap_state.as_ref() {
        if let Some(company_id) = state.get("companyId").and_then(|value| value.as_str()) {
            println!("Company ID: {company_id}");
        }
        if let Some(goal_id) = state
            .get("goal")
            .and_then(|value| value.get("id"))
            .and_then(|value| value.as_str())
        {
            println!("Goal ID: {goal_id}");
        }
        if let Some(project_id) = state
            .get("project")
            .and_then(|value| value.get("id"))
            .and_then(|value| value.as_str())
        {
            println!("Project ID: {project_id}");
        }
        if let Some(updated_at) = state
            .get("frontierSync")
            .and_then(|value| value.get("updatedAt"))
            .and_then(|value| value.as_str())
        {
            println!("Last sync: {updated_at}");
        }
    }
}

fn load_paperclip_maintenance(paths: &PaperclipPaths) -> Result<Option<MaintenanceMode>> {
    if !paths.manifest_path.exists() {
        return Ok(None);
    }
    let manifest = ProgramManifest::load(&paths.manifest_path)?;
    raspberry_supervisor::load_active_maintenance(&paths.manifest_path, &manifest)
        .map_err(anyhow::Error::from)
}

fn render_pid_status(pid: Option<u32>, pid_live: bool) -> String {
    match pid {
        Some(value) if pid_live => format!("{value} (running)"),
        Some(value) => format!("{value} (stale)"),
        None => "none".to_string(),
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn count_local_cli_exports(data_dir: &Path) -> usize {
    std::fs::read_dir(data_dir.join("local-cli"))
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok()))
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("env"))
        .count()
}

fn count_synced_secrets(paths: &PaperclipPaths) -> usize {
    load_bootstrap_state(&paths.bootstrap_state_path)
        .ok()
        .and_then(|state| state.get("secrets").cloned())
        .and_then(|value| value.as_object().cloned())
        .map(|values| values.len())
        .unwrap_or_default()
}

fn count_synced_work_products(paths: &PaperclipPaths) -> usize {
    load_bootstrap_state(&paths.bootstrap_state_path)
        .ok()
        .and_then(|state| state.get("frontierSync").cloned())
        .and_then(|value| value.get("workProductsSynced").cloned())
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .unwrap_or_default()
}

fn count_synced_attachments(paths: &PaperclipPaths) -> usize {
    load_bootstrap_state(&paths.bootstrap_state_path)
        .ok()
        .and_then(|state| state.get("frontierSync").cloned())
        .and_then(|value| value.get("attachmentsSynced").cloned())
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .unwrap_or_default()
}

fn read_pid_file(path: &Path) -> Result<Option<u32>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<u32>()
        .map(Some)
        .with_context(|| format!("failed to parse pid file {}", path.display()))
}

async fn stop_paperclip_server(data_dir: &Path, api_base: &str) -> Result<()> {
    let pid_path = data_dir.join("server.pid");
    let pid = read_pid_file(&pid_path)?;
    let server_ready = paperclip_server_ready(api_base).await;
    let Some(pid) = pid else {
        if server_ready {
            bail!("paperclip server is responding at {api_base} but no managed pid file exists");
        }
        return Ok(());
    };

    if process_is_running(pid) {
        terminate_process(pid).await?;
    }
    if pid_path.exists() {
        std::fs::remove_file(&pid_path)?;
    }
    if paperclip_server_ready(api_base).await {
        bail!("paperclip server is still responding at {api_base} after stop");
    }
    Ok(())
}

#[cfg(unix)]
async fn terminate_process(pid: u32) -> Result<()> {
    send_signal(pid, libc::SIGTERM, true)?;
    for _ in 0..20 {
        if !process_is_running(pid) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    send_signal(pid, libc::SIGKILL, true)?;
    for _ in 0..20 {
        if !process_is_running(pid) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    bail!("paperclip server pid {pid} did not exit after SIGTERM/SIGKILL");
}

#[cfg(not(unix))]
async fn terminate_process(_pid: u32) -> Result<()> {
    bail!("paperclip stop is not implemented on this platform")
}

#[cfg(unix)]
fn process_is_running(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if result == 0 {
        return true;
    }
    matches!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(code) if code == libc::EPERM
    )
}

#[cfg(not(unix))]
fn process_is_running(_pid: u32) -> bool {
    false
}

#[cfg(unix)]
fn send_signal(pid: u32, signal: i32, ignore_missing: bool) -> Result<()> {
    let result = unsafe { libc::kill(pid as libc::pid_t, signal) };
    if result == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if ignore_missing && error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(error).with_context(|| format!("failed to signal pid {pid} with {signal}"))
}

#[cfg(not(unix))]
fn send_signal(_pid: u32, _signal: i32, _ignore_missing: bool) -> Result<()> {
    bail!("signals are not implemented on this platform")
}

fn last_lines(contents: &str, line_count: usize) -> String {
    if line_count == 0 {
        return String::new();
    }
    let lines = contents.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(line_count);
    let mut output = lines[start..].join("\n");
    if !output.is_empty() {
        output.push('\n');
    }
    output
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrontierSyncModel {
    program: String,
    manifest_path: PathBuf,
    state_path: PathBuf,
    route_command: String,
    wake_command: String,
    refresh_command: String,
    status_command: String,
    summary: FrontierSummary,
    entries: Vec<FrontierSyncEntry>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct FrontierSummary {
    ready: usize,
    running: usize,
    blocked: usize,
    failed: usize,
    complete: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrontierSyncEntry {
    sync_key: String,
    lane_key: String,
    unit_id: String,
    unit_title: String,
    lane_id: String,
    lane_title: String,
    lane_kind: String,
    status: LaneExecutionStatus,
    detail: String,
    current_run_id: Option<String>,
    last_run_id: Option<String>,
    current_stage: Option<String>,
    current_stage_provider: Option<String>,
    current_stage_cli_name: Option<String>,
    time_in_current_stage_secs: Option<i64>,
    current_stage_idle_secs: Option<i64>,
    last_started_at: Option<String>,
    last_finished_at: Option<String>,
    last_exit_status: Option<i32>,
    last_usage_summary: Option<String>,
    last_completed_stage: Option<String>,
    last_stage_duration_ms: Option<u64>,
    last_stdout_snippet: Option<String>,
    last_stderr_snippet: Option<String>,
    failure_kind: Option<String>,
    blocker_reason: Option<String>,
    wake_command: String,
    route_command: String,
    refresh_command: String,
    artifact_paths: Vec<String>,
    artifact_statuses: Vec<String>,
    dependency_keys: Vec<String>,
    next_operator_move: String,
}

// ---------------------------------------------------------------------------
// Plan dashboard model — plan-root-keyed sync layer
// ---------------------------------------------------------------------------

#[allow(dead_code)] // Summary metrics are retained for the upcoming plan-root dashboard output.
#[derive(Debug, Clone)]
struct PlanDashboardModel {
    program: String,
    plans: Vec<PlanDashboardEntry>,
    summary: PlanDashboardSummary,
}

#[allow(dead_code)] // Summary metrics are retained for the upcoming plan-root dashboard output.
#[derive(Debug, Clone)]
struct PlanDashboardSummary {
    total: usize,
    represented: usize,
    in_motion: usize,
    needs_attention: usize,
    complete: usize,
}

#[derive(Debug, Clone)]
struct PlanDashboardEntry {
    plan_id: String,
    title: String,
    category: String,
    #[allow(dead_code)] // Retained for future plan-root narrative output and tests.
    composite: bool,
    path: String,
    status: String,
    current_stage: Option<String>,
    current_run_id: Option<String>,
    risk: String,
    next_move: String,
    mapping_source: String,
    children: Vec<PlanDashboardChild>,
}

#[allow(dead_code)] // Meta classification remains useful for docs/tests even when sync materialization uses child presence.
impl PlanDashboardEntry {
    fn is_syncable(&self) -> bool {
        self.category != "meta"
    }
}

fn should_materialize_plan_agent(plan: &PlanDashboardEntry) -> bool {
    let _ = plan;
    true
}

#[derive(Debug, Clone)]
struct PlanDashboardChild {
    child_id: String,
    title: String,
    archetype: Option<String>,
    review_profile: Option<String>,
    owned_surfaces: Vec<String>,
    sync_key: String,
    status: String,
    current_stage: Option<String>,
    current_run_id: Option<String>,
    next_operator_move: String,
}

#[derive(Debug, Clone)]
struct DesiredIssue {
    title: String,
    description: String,
    status: String,
    priority: String,
    parent_id: Option<String>,
    assignee_agent_id: Option<String>,
    project_workspace_id: Option<String>,
    assignee_adapter_overrides: Option<serde_json::Value>,
    execution_workspace_preference: Option<String>,
}

struct DesiredWorkProduct {
    external_id: String,
    title: String,
    status: String,
    health_status: String,
    is_primary: bool,
    url: Option<String>,
    summary: Option<String>,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
struct PaperclipProjectSync {
    project: PaperclipProject,
    workspace: PaperclipWorkspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WakeFollowthroughStatus {
    ObservedViaHeartbeat,
    RouteFallbackRan,
    RouteAlreadyActive,
    NoObservedChange,
}

#[derive(Debug, Clone)]
struct WakeFollowthrough {
    status: WakeFollowthroughStatus,
    after: Option<FrontierSyncModel>,
}

#[derive(Debug, Clone, Default)]
struct ControllerStatus {
    pid: Option<u32>,
    pid_live: bool,
    acquired_at: Option<String>,
    controller_provenance: Option<String>,
    fabro_provenance: Option<String>,
}

const ORCHESTRATOR_WAKE_WAIT_MS: u64 = 3_000;
const ORCHESTRATOR_WAKE_POLL_MS: u64 = 250;

struct BundleAgent {
    slug: String,
    name: String,
    adapter_type: &'static str,
    metadata_type: Option<&'static str>,
    unit: Option<String>,
    lane_key: Option<String>,
    plan_id: Option<String>,
}

struct GeneratedBundle {
    manifest: serde_json::Value,
    company_markdown: String,
    agent_markdowns: Vec<(String, String)>,
    agents: Vec<BundleAgent>,
}

struct BundleAgentDraft {
    manifest: serde_json::Value,
    relative_path: String,
    markdown: String,
    agent: BundleAgent,
}

#[allow(clippy::too_many_arguments)] // Bundle generation wires together repo, frontier, and Paperclip surfaces in one place.
fn build_company_bundle(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
    company_name: &str,
    mission: &BootstrapMission,
    orchestrator_script: &Path,
    minimax_agent_script: &Path,
    frontier: &FrontierSyncModel,
    plan_matrix: Option<&PlanMatrix>,
    plan_dashboard: Option<&PlanDashboardModel>,
) -> Result<GeneratedBundle> {
    let description = mission.company_description.clone();
    let effective_plan_dashboard = plan_dashboard
        .cloned()
        .filter(|dashboard| !dashboard.plans.is_empty())
        .or_else(|| synthesize_bundle_plan_dashboard(target_repo, &blueprint.program.id, frontier));
    let materialized_plan_ids =
        collect_materialized_plan_ids(target_repo, effective_plan_dashboard.as_ref());
    let mut manifest_agents = Vec::new();
    let mut agent_markdowns = Vec::new();
    let mut agents = Vec::new();
    push_bundle_agent(
        &mut manifest_agents,
        &mut agent_markdowns,
        &mut agents,
        mission_ceo_draft(blueprint, target_repo, mission, minimax_agent_script),
    );
    push_bundle_agent(
        &mut manifest_agents,
        &mut agent_markdowns,
        &mut agents,
        orchestrator_draft(blueprint, target_repo, orchestrator_script, frontier),
    );
    for unit in &blueprint.units {
        if !materialized_plan_ids.contains(&unit.id) {
            continue;
        }
        for lane in &unit.lanes {
            let lane_key = format!("{}:{}", unit.id, lane.id);
            let frontier_entry = frontier
                .entries
                .iter()
                .find(|entry| entry.lane_key == lane_key);
            push_bundle_agent(
                &mut manifest_agents,
                &mut agent_markdowns,
                &mut agents,
                top_level_plan_agent_draft(
                    blueprint,
                    target_repo,
                    mission,
                    minimax_agent_script,
                    frontier,
                    unit,
                    lane,
                    frontier_entry,
                ),
            );
        }
    }

    let manifest = json!({
        "schemaVersion": 1,
        "generatedAt": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
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
    let company_markdown = build_company_markdown(
        company_name,
        &description,
        frontier,
        plan_matrix,
        effective_plan_dashboard.as_ref(),
        &agents,
    );

    Ok(GeneratedBundle {
        manifest,
        company_markdown,
        agent_markdowns,
        agents,
    })
}

fn synthesize_bundle_plan_dashboard(
    target_repo: &Path,
    program_id: &str,
    frontier: &FrontierSyncModel,
) -> Option<PlanDashboardModel> {
    let plans_dir = target_repo.join("genesis").join("plans");
    if !plans_dir.is_dir() {
        return None;
    }

    let mut paths = std::fs::read_dir(&plans_dir)
        .ok()?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    paths.sort();

    let mut plans = Vec::new();
    for absolute_path in paths {
        if absolute_path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Some(stem) = absolute_path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let prefix = stem.split('-').next().unwrap_or_default();
        if prefix.len() != 3 || !prefix.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }

        let Ok(body) = std::fs::read_to_string(&absolute_path) else {
            continue;
        };
        let title = body
            .lines()
            .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| stem.to_string());
        let mut plan_id = stem
            .split_once('-')
            .map(|(_, rest)| rest.to_string())
            .unwrap_or_else(|| stem.to_string());
        for suffix in ["-game", "-plan", "-crate", "-trait"] {
            if let Some(stripped) = plan_id.strip_suffix(suffix) {
                plan_id = stripped.to_string();
            }
        }

        let live_plan_entry = plan_frontier_entry(frontier, &plan_id);
        let status = live_plan_entry
            .map(|entry| entry.status.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let category = if plan_id == "master" {
            "meta"
        } else if plan_id.contains("test")
            || plan_id.contains("verify")
            || plan_id.contains("verification")
            || plan_id.contains("performance")
        {
            "verification"
        } else if plan_id.contains("infrastructure")
            || plan_id.contains("chain")
            || plan_id.contains("install")
        {
            "infrastructure"
        } else if plan_id.contains("tui") || plan_id.contains("ui") || plan_id.contains("dashboard")
        {
            "interface"
        } else if plan_id.contains("wallet")
            || plan_id.contains("house")
            || plan_id.contains("blueprint")
            || plan_id.contains("faucet")
        {
            "service"
        } else {
            "unknown"
        };

        let Ok(relative_path) = absolute_path.strip_prefix(target_repo) else {
            continue;
        };
        plans.push(PlanDashboardEntry {
            plan_id,
            title,
            category: category.to_string(),
            composite: true,
            path: relative_path.display().to_string(),
            status,
            current_stage: live_plan_entry.and_then(|entry| entry.current_stage.clone()),
            current_run_id: live_plan_entry.and_then(|entry| entry.current_run_id.clone()),
            risk: String::new(),
            next_move: live_plan_entry
                .map(|entry| entry.next_operator_move.clone())
                .unwrap_or_else(|| "inspect plan".to_string()),
            mapping_source: "inferred".to_string(),
            children: Vec::new(),
        });
    }

    if plans.is_empty() {
        return None;
    }

    let represented = plans.iter().filter(|plan| plan.status != "unknown").count();
    let in_motion = plans
        .iter()
        .filter(|plan| plan.status.contains("running") || plan.status.contains("ready"))
        .count();
    let needs_attention = plans
        .iter()
        .filter(|plan| {
            plan.status.contains("failed")
                || plan.status.contains("blocked")
                || plan.status == "unmodeled"
        })
        .count();
    let complete = plans
        .iter()
        .filter(|plan| plan.status.contains("complete") || plan.status == "reviewed")
        .count();

    Some(PlanDashboardModel {
        program: program_id.to_string(),
        summary: PlanDashboardSummary {
            total: plans.len(),
            represented,
            in_motion,
            needs_attention,
            complete,
        },
        plans,
    })
}

fn collect_materialized_plan_ids(
    target_repo: &Path,
    dashboard: Option<&PlanDashboardModel>,
) -> BTreeSet<String> {
    let mut ids = dashboard
        .map(|dashboard| {
            dashboard
                .plans
                .iter()
                .filter(|plan| should_materialize_plan_agent(plan))
                .map(|plan| plan.plan_id.clone())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();

    if !ids.is_empty() {
        return ids;
    }

    let plans_dir = target_repo.join("genesis").join("plans");
    let Ok(entries) = std::fs::read_dir(&plans_dir) else {
        return ids;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let prefix = stem.split('-').next().unwrap_or_default();
        if prefix.len() != 3 || !prefix.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        let mut plan_id = stem
            .split_once('-')
            .map(|(_, rest)| rest.to_string())
            .unwrap_or_else(|| stem.to_string());
        for suffix in ["-game", "-plan", "-crate", "-trait"] {
            if let Some(stripped) = plan_id.strip_suffix(suffix) {
                plan_id = stripped.to_string();
            }
        }
        ids.insert(plan_id);
    }

    ids
}

fn push_bundle_agent(
    manifest_agents: &mut Vec<serde_json::Value>,
    agent_markdowns: &mut Vec<(String, String)>,
    agents: &mut Vec<BundleAgent>,
    draft: BundleAgentDraft,
) {
    manifest_agents.push(draft.manifest);
    agent_markdowns.push((draft.relative_path, draft.markdown));
    agents.push(draft.agent);
}

fn mission_ceo_draft(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
    mission: &BootstrapMission,
    minimax_agent_script: &Path,
) -> BundleAgentDraft {
    let slug = "mission-ceo";
    let name = "Mission CEO".to_string();
    let prompt = format!(
        "You own the company mission for `{}`.\n\nCompany goal:\n- {}\n\nPriorities:\n- keep work aligned to the repo blueprint\n- promote lane decomposition that matches the real plan\n- route execution through Raspberry rather than bypassing it\n- prefer honest progress over optimistic summaries\n",
        blueprint.program.id, mission.goal_title,
    );
    BundleAgentDraft {
        manifest: json!({
            "slug": slug,
            "name": name.clone(),
            "path": format!("agents/{slug}/AGENTS.md"),
            "role": "ceo",
            "title": "Mission CEO",
            "icon": "crown",
            "capabilities": "Own company mission, set priorities, and review lane strategy.",
            "reportsToSlug": serde_json::Value::Null,
            "adapterType": "process",
            "adapterConfig": {
                "command": "bash",
                "args": [minimax_agent_script.display().to_string()],
                "cwd": target_repo.display().to_string(),
                "promptTemplate": prompt,
                "timeoutSec": 1800
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
        }),
        relative_path: format!("agents/{slug}/AGENTS.md"),
        markdown: build_agent_markdown(&name, slug, "ceo", prompt),
        agent: BundleAgent {
            slug: slug.to_string(),
            name,
            adapter_type: "process",
            metadata_type: Some("mission_ceo"),
            unit: None,
            lane_key: None,
            plan_id: None,
        },
    }
}

fn orchestrator_draft(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
    orchestrator_script: &Path,
    frontier: &FrontierSyncModel,
) -> BundleAgentDraft {
    let slug = "raspberry-orchestrator";
    let name = "Raspberry Orchestrator".to_string();
    BundleAgentDraft {
        manifest: json!({
            "slug": slug,
            "name": name.clone(),
            "path": format!("agents/{slug}/AGENTS.md"),
            "role": "pm",
            "title": "Raspberry Orchestrator",
            "icon": "circuit-board",
            "capabilities": "Inspect and advance the repo-local Raspberry frontier.",
            "reportsToSlug": "mission-ceo",
            "adapterType": "process",
            "adapterConfig": {
                "command": "bash",
                "args": [orchestrator_script.display().to_string()],
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
        }),
        relative_path: format!("agents/{slug}/AGENTS.md"),
        markdown: build_agent_markdown(
            &name,
            slug,
            "pm",
            format!(
                "You operate the repo-local Raspberry control plane for `{}`.\n\nLive frontier inspection:\n- run `{}` for the current frontier summary\n- use `{}` to ask Paperclip to wake Raspberry\n- use `{}` as the direct repo-local fallback\n- run `{}` after frontier movement or package changes\n\nExecution route:\n- Do not create parallel execution flows outside Raspberry.\n- Use Paperclip for coordination, escalation, review, and handoff only.\n- Treat live state as volatile; inspect current status before making claims about counts or lane settlement.\n",
                blueprint.program.id,
                frontier.status_command,
                frontier.wake_command,
                frontier.route_command,
                frontier.refresh_command,
            ),
        ),
        agent: BundleAgent {
            slug: slug.to_string(),
            name,
            adapter_type: "process",
            metadata_type: Some("raspberry_orchestrator"),
            unit: None,
            lane_key: None,
            plan_id: None,
        },
    }
}

#[allow(dead_code)] // Reserved for optional lane-level Paperclip agents if operators re-enable them.
#[allow(clippy::too_many_arguments)] // Lane drafts are assembled from explicit blueprint and frontier inputs.
fn lane_agent_draft(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
    mission: &BootstrapMission,
    minimax_agent_script: &Path,
    frontier: &FrontierSyncModel,
    unit: &BlueprintUnit,
    lane: &fabro_synthesis::BlueprintLane,
    frontier_entry: Option<&FrontierSyncEntry>,
) -> BundleAgentDraft {
    let slug = lane_agent_slug(unit, lane);
    let name = lane_agent_name(unit, lane);
    let role = lane_role(unit, lane);
    let adapter_type = "process";
    let lane_key = format!("{}:{}", unit.id, lane.id);
    let prompt = format!(
        "You coordinate the `{}` frontier in repo `{}`.\n\nCompany goal:\n{}\n\nLane goal:\n{}\n\nLive frontier inspection:\n- lane key: `{}`\n- sync key: `{}`\n- inspect current status with `{}` before asserting readiness, blockage, or completion\n- use `{}` after frontier movement to refresh Paperclip state\n\nArtifacts:\n{}\n\nDependencies:\n{}\n\nExecution route:\n- Wake the Raspberry Orchestrator with `{}` when this frontier needs Raspberry to evaluate or advance work.\n- Run `{}` as the direct repo-local fallback.\n- Keep the lane plan in the Paperclip `plan` document aligned with repo truth.\n- Use Paperclip to triage, review, escalate, and explain blockers.\n- Do not bypass Raspberry with direct ad hoc execution.\n",
        lane.id,
        blueprint.program.id,
        mission.goal_title,
        lane.goal,
        lane_key,
        lane_sync_key(&blueprint.program.id, &lane_key),
        frontier.status_command,
        frontier.refresh_command,
        lane_artifact_block(frontier_entry, unit),
        lane_dependency_block(frontier_entry, lane),
        frontier.wake_command,
        frontier.route_command,
    );
    let adapter_config = serde_json::Map::from_iter([
        ("command".to_string(), json!("bash")),
        (
            "args".to_string(),
            json!([minimax_agent_script.display().to_string()]),
        ),
        ("cwd".to_string(), json!(target_repo.display().to_string())),
        ("promptTemplate".to_string(), json!(prompt.clone())),
        ("timeoutSec".to_string(), json!(1800)),
    ]);
    BundleAgentDraft {
        manifest: json!({
            "slug": slug.clone(),
            "name": name.clone(),
            "path": format!("agents/{slug}/AGENTS.md"),
            "role": role,
            "title": name.clone(),
            "icon": serde_json::Value::Null,
            "capabilities": format!("Own the `{}` lane and its artifacts.", lane.id),
            "reportsToSlug": "raspberry-orchestrator",
            "adapterType": adapter_type,
            "adapterConfig": serde_json::Value::Object(adapter_config),
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
                "laneId": lane.id,
                "laneKey": lane_key.clone(),
                "family": lane.family,
                "unit": unit.id
            }
        }),
        relative_path: format!("agents/{slug}/AGENTS.md"),
        markdown: build_agent_markdown(&name, &slug, role, prompt),
        agent: BundleAgent {
            slug,
            name,
            adapter_type,
            metadata_type: None,
            unit: Some(unit.id.clone()),
            lane_key: Some(lane_key),
            plan_id: None,
        },
    }
}

#[allow(clippy::too_many_arguments)] // Top-level plan agents reuse lane-draft inputs plus plan-level context.
fn top_level_plan_agent_draft(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
    mission: &BootstrapMission,
    minimax_agent_script: &Path,
    frontier: &FrontierSyncModel,
    unit: &BlueprintUnit,
    lane: &fabro_synthesis::BlueprintLane,
    frontier_entry: Option<&FrontierSyncEntry>,
) -> BundleAgentDraft {
    let mut draft = lane_agent_draft(
        blueprint,
        target_repo,
        mission,
        minimax_agent_script,
        frontier,
        unit,
        lane,
        frontier_entry,
    );
    draft.agent.plan_id = Some(unit.id.clone());
    draft.agent.metadata_type = Some("plan_root");
    draft
}

fn render_plan_children_summary(children: &[PlanDashboardChild]) -> String {
    if children.is_empty() {
        return "No children defined.".to_string();
    }
    children
        .iter()
        .map(|c| {
            let stage = c
                .current_stage
                .as_deref()
                .map(|value| format!(" | stage `{value}`"))
                .unwrap_or_default();
            let run = c
                .current_run_id
                .as_deref()
                .map(|value| format!(" | run `{value}`"))
                .unwrap_or_default();
            format!(
                "- `{}`: {} ({}) — status `{}`{}{} — next: {} — surfaces: {}",
                c.child_id,
                c.title,
                c.archetype.as_deref().unwrap_or("implement"),
                c.status,
                stage,
                run,
                c.next_operator_move,
                if c.owned_surfaces.is_empty() {
                    "none".to_string()
                } else {
                    c.owned_surfaces.join(", ")
                },
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[allow(dead_code)] // Retained while the repo migrates from legacy plan-root imports to top-level lane ownership.
fn plan_root_agent_draft(
    target_repo: &Path,
    frontier: &FrontierSyncModel,
    plan: &PlanDashboardEntry,
) -> BundleAgentDraft {
    let slug = format!("plan-{}", plan.plan_id);
    let name = format!("Plan: {}", plan.title);
    let role = match plan.category.as_str() {
        "meta" => "pm",
        "verification" => "qa",
        _ => "engineer",
    };
    let adapter_type = match plan.category.as_str() {
        "meta" | "verification" => "claude_local",
        _ => "codex_local",
    };

    let children_summary = render_plan_children_summary(&plan.children);

    let body = format!(
        "# {name}\n\nYou are the plan-root agent for `{plan_id}` in `{repo}`.\n\n## Plan Status\n\n- Status: {status}\n- Current stage: {current_stage}\n- Current run: {current_run}\n- Risk: {risk}\n- Next move: {next_move}\n- Mapping: {mapping}\n- Category: {category}\n- Composite: {composite}\n\n## Children\n\n{children}\n\n## Commands\n\n- Wake orchestrator: `{wake}`\n- Route fallback: `{route}`\n- Refresh Paperclip: `{refresh}`\n- Inspect status: `{status_cmd}`\n",
        name = name,
        plan_id = plan.plan_id,
        repo = target_repo.display(),
        status = plan.status,
        current_stage = plan.current_stage.as_deref().unwrap_or("none"),
        current_run = plan.current_run_id.as_deref().unwrap_or("none"),
        risk = plan.risk,
        next_move = plan.next_move,
        mapping = plan.mapping_source,
        category = plan.category,
        composite = plan.composite,
        children = children_summary,
        wake = frontier.wake_command,
        route = frontier.route_command,
        refresh = frontier.refresh_command,
        status_cmd = frontier.status_command,
    );

    let mut adapter_config = serde_json::Map::from_iter([
        ("cwd".to_string(), json!(target_repo.display().to_string())),
        (
            "model".to_string(),
            json!(PAPERCLIP_DEFAULT_AUTOMATION_MODEL),
        ),
    ]);
    if adapter_type == "claude_local" {
        adapter_config.insert("dangerouslySkipPermissions".to_string(), json!(true));
    } else {
        let codex_home = preferred_automation_codex_home()
            .unwrap_or_else(|| target_repo.join(".paperclip").join("codex").join(&slug));
        adapter_config.insert(
            "env".to_string(),
            json!({
                "CODEX_HOME": codex_home.display().to_string()
            }),
        );
        adapter_config.insert(
            "dangerouslyBypassApprovalsAndSandbox".to_string(),
            json!(true),
        );
    }

    BundleAgentDraft {
        manifest: json!({
            "slug": slug,
            "name": name.clone(),
            "path": format!("agents/{slug}/AGENTS.md"),
            "role": role,
            "title": name.clone(),
            "icon": serde_json::Value::Null,
            "capabilities": format!("Own the `{}` plan and its coordination artifacts.", plan.plan_id),
            "reportsToSlug": "raspberry-orchestrator",
            "adapterType": adapter_type,
            "adapterConfig": serde_json::Value::Object(adapter_config),
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
                "type": "plan_root",
                "planId": plan.plan_id,
                "category": plan.category,
            }
        }),
        relative_path: format!("agents/{slug}/AGENTS.md"),
        markdown: build_agent_markdown(&name, &slug, role, body),
        agent: BundleAgent {
            slug,
            name,
            adapter_type,
            metadata_type: Some("plan_root"),
            unit: None,
            lane_key: None,
            plan_id: Some(plan.plan_id.clone()),
        },
    }
}

fn preferred_automation_codex_home() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let primary_home = home.join(".codex");
    let config_paths = [
        home.join(".config/autonomy/codex-rotator.json"),
        home.join(".config/rsociety/codex-rotator.json"),
    ];

    for path in config_paths {
        let Ok(raw) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(config) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };
        if config.get("enabled").and_then(|value| value.as_bool()) != Some(true) {
            continue;
        }
        let Some(slots) = config.get("slots").and_then(|value| value.as_array()) else {
            continue;
        };
        for slot in slots {
            let Some(codex_home) = slot
                .get("codexHome")
                .and_then(|value| value.as_str())
                .map(PathBuf::from)
            else {
                continue;
            };
            if codex_home == primary_home {
                continue;
            }
            if codex_home.join("auth.json").exists() {
                return Some(codex_home);
            }
        }
    }

    for slot_name in ["slot1", "slot2", "slot3", "slot4", "slot5"] {
        let codex_home = home.join(format!(".codex-{slot_name}/.codex"));
        if codex_home.join("auth.json").exists() {
            return Some(codex_home);
        }
    }

    None
}

#[allow(dead_code)] // Reserved for optional lane-level Paperclip agents if operators re-enable them.
fn lane_agent_slug(unit: &BlueprintUnit, lane: &fabro_synthesis::BlueprintLane) -> String {
    if unit.lanes.len() == 1 {
        return unit.id.clone();
    }
    format!("{}--{}", unit.id, lane.id)
}

#[allow(dead_code)] // Reserved for optional lane-level Paperclip agents if operators re-enable them.
fn lane_agent_name(unit: &BlueprintUnit, lane: &fabro_synthesis::BlueprintLane) -> String {
    if unit.lanes.len() == 1 {
        return unit.title.clone();
    }
    format!("{} / {}", unit.title, lane.title)
}

#[allow(dead_code)] // Reserved for optional lane-level Paperclip agents if operators re-enable them.
fn lane_artifact_block(entry: Option<&FrontierSyncEntry>, unit: &BlueprintUnit) -> String {
    entry
        .map(|frontier_entry| {
            bullet_block(
                &frontier_entry.artifact_paths,
                "No curated artifacts recorded.",
            )
        })
        .unwrap_or_else(|| {
            unit.artifacts
                .iter()
                .map(|artifact| format!("- {}", artifact.path.display()))
                .collect::<Vec<_>>()
                .join("\n")
        })
}

#[allow(dead_code)] // Reserved for optional lane-level Paperclip agents if operators re-enable them.
fn lane_dependency_block(
    entry: Option<&FrontierSyncEntry>,
    lane: &fabro_synthesis::BlueprintLane,
) -> String {
    entry
        .map(|frontier_entry| {
            bullet_block(&frontier_entry.dependency_keys, "No explicit dependencies.")
        })
        .unwrap_or_else(|| {
            lane.dependencies
                .iter()
                .map(render_blueprint_dependency)
                .collect::<Vec<_>>()
                .join("\n")
        })
}

#[allow(dead_code)] // Reserved for optional lane-level Paperclip agents if operators re-enable them.
fn render_blueprint_dependency(dependency: &raspberry_supervisor::LaneDependency) -> String {
    format!(
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
    )
}

#[allow(dead_code)] // Reserved for optional lane-level Paperclip agents if operators re-enable them.
fn lane_role(unit: &BlueprintUnit, lane: &fabro_synthesis::BlueprintLane) -> &'static str {
    if lane.template == WorkflowTemplate::RecurringReport || unit.id.contains("proof") {
        return "qa";
    }
    if lane.template == WorkflowTemplate::Orchestration {
        return "pm";
    }
    "engineer"
}

#[allow(dead_code)] // Reserved for optional lane-level Paperclip agents if operators re-enable them.
fn lane_adapter_type(unit: &BlueprintUnit, lane: &fabro_synthesis::BlueprintLane) -> &'static str {
    let _ = (unit, lane);
    "process"
}

fn write_bundle(root: &Path, bundle: &GeneratedBundle) -> Result<()> {
    std::fs::create_dir_all(root)?;
    let agents_dir = root.join("agents");
    if agents_dir.exists() {
        std::fs::remove_dir_all(&agents_dir)
            .with_context(|| format!("failed to prune {}", agents_dir.display()))?;
    }
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

fn build_company_markdown(
    name: &str,
    description: &str,
    frontier: &FrontierSyncModel,
    plan_matrix: Option<&PlanMatrix>,
    plan_dashboard: Option<&PlanDashboardModel>,
    agents: &[BundleAgent],
) -> String {
    let mut body = format!("# {}\n\n{}\n", name, description);
    if let Some(plan_matrix) = plan_matrix {
        body.push_str(&format!(
            "\n# Plans\n\n## Plan Status Summary\n\n{}\n",
            render_plan_matrix_summary(plan_matrix)
        ));
        let needing_attention =
            render_plan_attention_section(plan_matrix, "Plans Needing Attention", |row| {
                row.current_status.contains("failed")
                    || row.current_status.contains("blocked")
                    || row.current_status == "unmodeled"
            });
        if let Some(section) = needing_attention {
            body.push_str(&format!("\n{}\n", section));
        }
        let in_motion = render_plan_attention_section(plan_matrix, "Plans In Motion", |row| {
            row.current_status.contains("running")
                || row.current_status.contains("ready")
                || row.current_status.contains("implementation_")
        });
        if let Some(section) = in_motion {
            body.push_str(&format!("\n{}\n", section));
        }
        body.push_str(&format!(
            "\n## Plan Matrix\n\n```\n{}\n```\n",
            raspberry_supervisor::render_plan_matrix(plan_matrix)
        ));
    }
    if let Some(dashboard) = plan_dashboard {
        body.push_str(&render_plan_category_sections(dashboard));
    }
    body.push_str(&format!(
        "\n# Frontier (Lane Detail)\n\n{}\n\n## Lane Sets\n\n{}\n",
        render_frontier_summary(frontier),
        render_frontier_lane_sets(frontier),
    ));
    let details = render_frontier_detail_sections(frontier);
    if !details.is_empty() {
        body.push_str(&format!("\n## Live Details\n\n{}\n", details));
    }
    body.push_str("\n# Agents\n\n## Plan Agents\n");
    for agent in agents {
        if agent.plan_id.is_some() {
            body.push_str(&format!("- {} - {}\n", agent.slug, agent.name));
        }
    }
    body.push_str("\n## Lane Agents\n");
    for agent in agents {
        if agent.plan_id.is_none() {
            body.push_str(&format!("- {} - {}\n", agent.slug, agent.name));
        }
    }
    format!(
        "---\nkind: company\nname: {}\ndescription: {}\nbrandColor: null\nrequireBoardApprovalForNewAgents: true\n---\n\n{}",
        serde_json::to_string(name).expect("json"),
        serde_json::to_string(description).expect("json"),
        body
    )
}

fn render_plan_category_sections(dashboard: &PlanDashboardModel) -> String {
    let categories = [
        "foundation",
        "game",
        "interface",
        "service",
        "infrastructure",
        "verification",
        "economic",
        "unknown",
    ];
    let mut sections = Vec::new();
    for category in &categories {
        let plans: Vec<&PlanDashboardEntry> = dashboard
            .plans
            .iter()
            .filter(|p| p.category == *category)
            .collect();
        if plans.is_empty() {
            continue;
        }
        let label = format!("{}{}", category[..1].to_uppercase(), &category[1..]);
        let mut lines = vec![format!("\n## {} Plans\n", label)];
        for plan in plans {
            lines.push(format!(
                "- `{}`: {} / {} / next: {}",
                plan.plan_id, plan.status, plan.mapping_source, plan.next_move
            ));
        }
        sections.push(lines.join("\n"));
    }
    sections.join("\n")
}

fn render_plan_matrix_summary(plan_matrix: &PlanMatrix) -> String {
    let total = plan_matrix.rows.len();
    let represented = plan_matrix
        .rows
        .iter()
        .filter(|row| row.represented_in_blueprint)
        .count();
    let mapped = plan_matrix
        .rows
        .iter()
        .filter(|row| row.mapping_status == "mapped")
        .count();
    let contract_backed = plan_matrix
        .rows
        .iter()
        .filter(|row| row.current_risk.contains("mapping exists"))
        .count();
    let in_motion = plan_matrix
        .rows
        .iter()
        .filter(|row| {
            row.current_status.contains("running")
                || row.current_status.contains("ready")
                || row.current_status.contains("implementation_")
        })
        .count();

    [
        format!("- total plans: {total}"),
        format!("- represented in blueprint: {represented}"),
        format!("- mapped plans: {mapped}"),
        format!("- contract-backed mappings surfaced in status: {contract_backed}"),
        format!("- currently in motion: {in_motion}"),
    ]
    .join("\n")
}

fn render_plan_attention_section<F>(
    plan_matrix: &PlanMatrix,
    title: &str,
    predicate: F,
) -> Option<String>
where
    F: Fn(&PlanStatusRow) -> bool,
{
    let rows = plan_matrix
        .rows
        .iter()
        .filter(|row| predicate(row))
        .take(8)
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return None;
    }
    let mut body = vec![format!("## {title}")];
    for row in rows {
        body.push(format!(
            "- `{}`: {} / {} / next: {}",
            row.plan_id, row.mapping_status, row.current_status, row.next_operator_move
        ));
    }
    Some(body.join("\n"))
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

fn load_frontier_sync_model(
    program_id: &str,
    target_repo: &Path,
    manifest_path: &Path,
    orchestrator_script: &Path,
    wake_command: &str,
    refresh_command: &str,
    status_command: &str,
) -> Result<FrontierSyncModel> {
    let manifest = ProgramManifest::load(manifest_path).with_context(|| {
        format!(
            "failed to load Raspberry manifest {}",
            manifest_path.display()
        )
    })?;
    let state_path = manifest.resolved_state_path(manifest_path);
    let mut state = ProgramRuntimeState::load_optional(&state_path)
        .with_context(|| format!("failed to load Raspberry state {}", state_path.display()))?
        .unwrap_or_else(|| ProgramRuntimeState::new(program_id));
    refresh_program_state(manifest_path, &manifest, &mut state)
        .context("failed to refresh Raspberry program state in memory")?;
    let program = evaluate_with_state(manifest_path, &manifest, Some(&state));
    let route_command = format!(
        "bash {}",
        repo_relative_display(orchestrator_script, target_repo),
    );

    Ok(FrontierSyncModel {
        program: program.program.clone(),
        manifest_path: normalize_storage_path(manifest_path),
        state_path: normalize_storage_path(&state_path),
        route_command: route_command.clone(),
        wake_command: wake_command.to_string(),
        refresh_command: refresh_command.to_string(),
        status_command: status_command.to_string(),
        summary: summarize_frontier(&program),
        entries: build_frontier_entries(
            program_id,
            &manifest,
            manifest_path,
            &program,
            wake_command,
            &route_command,
            refresh_command,
        ),
    })
}

fn load_optional_frontier_sync_model(paths: &PaperclipPaths) -> Result<Option<FrontierSyncModel>> {
    if !paths.manifest_path.exists() {
        return Ok(None);
    }
    let frontier = load_frontier_sync_model(
        &paths.program_id,
        &paths.target_repo,
        &paths.manifest_path,
        &paths.orchestrator_script_path,
        &paperclip_wake_command(paths, "raspberry-orchestrator"),
        &paperclip_refresh_command(paths),
        &paperclip_status_command(paths),
    )?;
    Ok(Some(frontier))
}

async fn follow_through_orchestrator_wake(
    paths: &PaperclipPaths,
    frontier_before: Option<&FrontierSyncModel>,
) -> Result<WakeFollowthrough> {
    let observed = wait_for_frontier_change(paths, frontier_before).await?;
    if observed.is_some() {
        return Ok(WakeFollowthrough {
            status: WakeFollowthroughStatus::ObservedViaHeartbeat,
            after: observed,
        });
    }

    let route_output = run_orchestrator_route(paths)?;
    let active_controller = output_indicates_active_autodev_controller(&route_output);
    if !route_output.status.success() && !active_controller {
        bail!(
            "Raspberry route fallback failed with exit_status={}: stdout=\n{}\n\nstderr=\n{}",
            route_output.status,
            String::from_utf8_lossy(&route_output.stdout),
            String::from_utf8_lossy(&route_output.stderr)
        );
    }
    let status = if active_controller {
        WakeFollowthroughStatus::RouteAlreadyActive
    } else {
        WakeFollowthroughStatus::RouteFallbackRan
    };
    let after = load_optional_frontier_sync_model(paths)?;
    if frontier_before
        .zip(after.as_ref())
        .map(|(before, after)| !frontier_sync_unchanged(before, after))
        .unwrap_or(false)
    {
        return Ok(WakeFollowthrough { status, after });
    }

    Ok(WakeFollowthrough {
        status: WakeFollowthroughStatus::NoObservedChange,
        after,
    })
}

async fn wait_for_frontier_change(
    paths: &PaperclipPaths,
    frontier_before: Option<&FrontierSyncModel>,
) -> Result<Option<FrontierSyncModel>> {
    let Some(frontier_before) = frontier_before else {
        return Ok(None);
    };
    let polls = (ORCHESTRATOR_WAKE_WAIT_MS / ORCHESTRATOR_WAKE_POLL_MS).max(1);
    for _ in 0..polls {
        tokio::time::sleep(Duration::from_millis(ORCHESTRATOR_WAKE_POLL_MS)).await;
        let Some(frontier_after) = load_optional_frontier_sync_model(paths)? else {
            continue;
        };
        if !frontier_sync_unchanged(frontier_before, &frontier_after) {
            return Ok(Some(frontier_after));
        }
    }
    Ok(None)
}

fn run_orchestrator_route(paths: &PaperclipPaths) -> Result<Output> {
    Command::new("bash")
        .arg(&paths.orchestrator_script_path)
        .current_dir(&paths.target_repo)
        .output()
        .with_context(|| {
            format!(
                "failed to run Raspberry route fallback {}",
                paths.orchestrator_script_path.display()
            )
        })
}

fn frontier_sync_unchanged(before: &FrontierSyncModel, after: &FrontierSyncModel) -> bool {
    before == after
}

fn wake_followthrough_changed(
    frontier_before: Option<&FrontierSyncModel>,
    followthrough: &WakeFollowthrough,
) -> bool {
    let Some(after) = followthrough.after.as_ref() else {
        return false;
    };
    let Some(before) = frontier_before else {
        return true;
    };
    !frontier_sync_unchanged(before, after)
}

fn output_indicates_active_autodev_controller(output: &Output) -> bool {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr_indicates_active_autodev_controller(&stdout, &stderr)
}

fn load_controller_status(paths: &PaperclipPaths) -> ControllerStatus {
    let lease_path = paths
        .target_repo
        .join(".raspberry")
        .join(format!("{}-autodev.lock", paths.program_id));
    let Ok(raw) = std::fs::read_to_string(&lease_path) else {
        return ControllerStatus::default();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return ControllerStatus::default();
    };
    let pid = value
        .get("pid")
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok());
    let acquired_at = value
        .get("acquired_at")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);
    let provenance = load_frontier_provenance(&paths.manifest_path)
        .and_then(|value| serde_json::from_value::<AutodevProvenance>(value).ok());
    ControllerStatus {
        pid,
        pid_live: pid.map(process_is_running).unwrap_or(false),
        acquired_at,
        controller_provenance: provenance
            .as_ref()
            .map(|value| format_binary_provenance(&value.controller)),
        fabro_provenance: provenance
            .as_ref()
            .map(|value| format_binary_provenance(&value.fabro_bin)),
    }
}

fn load_frontier_provenance(manifest_path: &Path) -> Option<serde_json::Value> {
    let manifest = ProgramManifest::load(manifest_path).ok()?;
    raspberry_supervisor::load_optional_autodev_report(manifest_path, &manifest)
        .ok()
        .flatten()
        .and_then(|report| serde_json::to_value(report.provenance).ok())
        .filter(|value| !value.is_null())
}

fn format_binary_provenance(binary: &raspberry_supervisor::BinaryProvenance) -> String {
    match binary.version.as_deref() {
        Some(version) => format!("{version} @ {}", binary.path),
        None => binary.path.clone(),
    }
}

fn stderr_indicates_active_autodev_controller(stdout: &str, stderr: &str) -> bool {
    let combined = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    combined.contains("autodev controller already running for program")
}

fn render_wake_followthrough(followthrough: &WakeFollowthrough) -> String {
    match followthrough.status {
        WakeFollowthroughStatus::ObservedViaHeartbeat => {
            "frontier moved after heartbeat".to_string()
        }
        WakeFollowthroughStatus::RouteFallbackRan => {
            "frontier did not move immediately; ran repo-local Raspberry route fallback"
                .to_string()
        }
        WakeFollowthroughStatus::RouteAlreadyActive => {
            "frontier did not move immediately; route fallback reported an active autodev controller"
                .to_string()
        }
        WakeFollowthroughStatus::NoObservedChange => {
            "frontier still shows no observable change after heartbeat and route fallback"
                .to_string()
        }
    }
}

fn summarize_frontier(program: &raspberry_supervisor::EvaluatedProgram) -> FrontierSummary {
    let mut summary = FrontierSummary {
        ready: 0,
        running: 0,
        blocked: 0,
        failed: 0,
        complete: 0,
    };
    for lane in &program.lanes {
        match lane.status {
            LaneExecutionStatus::Ready => summary.ready += 1,
            LaneExecutionStatus::Running => summary.running += 1,
            LaneExecutionStatus::Blocked => summary.blocked += 1,
            LaneExecutionStatus::Failed => summary.failed += 1,
            LaneExecutionStatus::Complete => summary.complete += 1,
        }
    }
    summary
}

fn build_frontier_entries(
    program_id: &str,
    manifest: &ProgramManifest,
    manifest_path: &Path,
    program: &raspberry_supervisor::EvaluatedProgram,
    wake_command: &str,
    route_command: &str,
    refresh_command: &str,
) -> Vec<FrontierSyncEntry> {
    let mut lanes = program.lanes.clone();
    lanes.sort_by(|left, right| left.lane_key.cmp(&right.lane_key));
    lanes
        .into_iter()
        .map(|lane| {
            build_frontier_entry(
                program_id,
                manifest,
                manifest_path,
                &lane,
                wake_command,
                route_command,
                refresh_command,
            )
        })
        .collect()
}

fn build_frontier_entry(
    program_id: &str,
    manifest: &ProgramManifest,
    manifest_path: &Path,
    lane: &EvaluatedLane,
    wake_command: &str,
    route_command: &str,
    refresh_command: &str,
) -> FrontierSyncEntry {
    let target_repo = manifest.resolved_target_repo(manifest_path);
    let artifacts = manifest.resolve_lane_artifacts(manifest_path, &lane.unit_id, &lane.lane_id);
    let artifact_paths = artifacts
        .iter()
        .map(|artifact| repo_relative_display(&artifact.path, &target_repo))
        .collect::<Vec<_>>();
    let artifact_statuses = artifacts
        .iter()
        .map(|artifact| {
            let display = repo_relative_display(&artifact.path, &target_repo);
            if artifact.path.exists() {
                format!("present: {display}")
            } else {
                format!("missing: {display}")
            }
        })
        .collect::<Vec<_>>();
    let dependency_keys = manifest
        .units
        .get(&lane.unit_id)
        .and_then(|unit| unit.lanes.get(&lane.lane_id))
        .map(|lane_manifest| {
            lane_manifest
                .dependencies
                .iter()
                .map(render_dependency_key)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    FrontierSyncEntry {
        sync_key: lane_sync_key(program_id, &lane.lane_key),
        lane_key: lane.lane_key.clone(),
        unit_id: lane.unit_id.clone(),
        unit_title: lane.unit_title.clone(),
        lane_id: lane.lane_id.clone(),
        lane_title: lane.lane_title.clone(),
        lane_kind: lane.lane_kind.to_string(),
        status: lane.status,
        detail: lane.detail.clone(),
        current_run_id: lane.current_run_id.clone(),
        last_run_id: lane.last_run_id.clone(),
        current_stage: lane.current_stage.clone(),
        current_stage_provider: lane.current_stage_provider.clone(),
        current_stage_cli_name: lane.current_stage_cli_name.clone(),
        time_in_current_stage_secs: lane.time_in_current_stage_secs,
        current_stage_idle_secs: lane.current_stage_idle_secs,
        last_started_at: format_timestamp(lane.last_started_at),
        last_finished_at: format_timestamp(lane.last_finished_at),
        last_exit_status: lane.last_exit_status,
        last_usage_summary: lane.last_usage_summary.clone(),
        last_completed_stage: lane.last_completed_stage_label.clone(),
        last_stage_duration_ms: lane.last_stage_duration_ms,
        last_stdout_snippet: lane.last_stdout_snippet.clone(),
        last_stderr_snippet: lane.last_stderr_snippet.clone(),
        failure_kind: lane.failure_kind.map(|kind| format!("{kind:?}")),
        blocker_reason: blocker_reason_for_lane(lane),
        wake_command: wake_command.to_string(),
        route_command: route_command.to_string(),
        refresh_command: refresh_command.to_string(),
        artifact_paths,
        artifact_statuses,
        dependency_keys,
        next_operator_move: next_operator_move_for_lane(
            lane,
            wake_command,
            route_command,
            refresh_command,
        ),
    }
}

fn render_dependency_key(dependency: &raspberry_supervisor::LaneDependency) -> String {
    format!(
        "{}{}{}",
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
            .unwrap_or_default(),
    )
}

fn blocker_reason_for_lane(lane: &EvaluatedLane) -> Option<String> {
    if let Some(error) = lane
        .last_error
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        return Some(error.clone());
    }
    match lane.status {
        LaneExecutionStatus::Blocked | LaneExecutionStatus::Failed => Some(lane.detail.clone()),
        _ => None,
    }
}

fn format_timestamp(value: Option<chrono::DateTime<Utc>>) -> Option<String> {
    value.map(|timestamp| timestamp.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn next_operator_move_for_lane(
    lane: &EvaluatedLane,
    wake_command: &str,
    route_command: &str,
    refresh_command: &str,
) -> String {
    match lane.status {
        LaneExecutionStatus::Ready => format!(
            "Wake the orchestrator with `{wake_command}` to let Raspberry dispatch ready work, then `{refresh_command}` to refresh Paperclip. Use `{route_command}` as the direct fallback."
        ),
        LaneExecutionStatus::Running => {
            if let Some(run_id) = lane.current_run_id.as_ref() {
                return format!(
                    "Monitor `fabro inspect {run_id}` or `fabro logs {run_id}` while work is running, then `{refresh_command}` after it settles."
                );
            }
            format!(
                "Monitor the active Raspberry run and refresh Paperclip with `{refresh_command}` after it settles."
            )
        }
        LaneExecutionStatus::Blocked => format!(
            "Resolve the blocker in repo truth, then rerun `{wake_command}` and `{refresh_command}`."
        ),
        LaneExecutionStatus::Failed => {
            if lane.failure_kind == Some(FailureKind::ProviderAccessLimited) {
                return format!(
                    "Restore provider auth or credits for this lane's configured agent, then rerun `{wake_command}` and `{refresh_command}`."
                );
            }
            if let Some(run_id) = lane.last_run_id.as_ref() {
                return format!(
                    "Inspect `fabro inspect {run_id}` and `fabro logs {run_id}`, fix the underlying cause, then rerun `{wake_command}` and `{refresh_command}`."
                );
            }
            format!(
                "Inspect the last failure, fix the cause in repo truth, then rerun `{wake_command}` and `{refresh_command}`."
            )
        }
        LaneExecutionStatus::Complete => format!(
            "No action unless the frontier changes. Run `{refresh_command}` after blueprint or state changes."
        ),
    }
}

fn render_frontier_summary(frontier: &FrontierSyncModel) -> String {
    [
        format!("- ready: {}", frontier.summary.ready),
        format!("- running: {}", frontier.summary.running),
        format!("- blocked: {}", frontier.summary.blocked),
        format!("- failed: {}", frontier.summary.failed),
        format!("- complete: {}", frontier.summary.complete),
        format!("- wake: `{}`", frontier.wake_command),
        format!("- route: `{}`", frontier.route_command),
        format!("- refresh: `{}`", frontier.refresh_command),
        format!("- status: `{}`", frontier.status_command),
    ]
    .join("\n")
}

fn render_frontier_detail_sections(frontier: &FrontierSyncModel) -> String {
    let sections = [
        render_frontier_status_section(frontier, LaneExecutionStatus::Running, "Running"),
        render_frontier_status_section(frontier, LaneExecutionStatus::Failed, "Failed"),
        render_frontier_status_section(frontier, LaneExecutionStatus::Ready, "Ready"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    sections.join("\n\n")
}

fn render_frontier_status_section(
    frontier: &FrontierSyncModel,
    status: LaneExecutionStatus,
    title: &str,
) -> Option<String> {
    let entries = frontier
        .entries
        .iter()
        .filter(|entry| entry.status == status)
        .collect::<Vec<_>>();
    if entries.is_empty() {
        return None;
    }
    let mut body = vec![format!("## {title}")];
    for entry in entries {
        body.push(format!("### `{}`", entry.lane_key));
        body.push(render_frontier_entry(Some(entry)));
    }
    Some(body.join("\n"))
}

fn render_frontier_entry(entry: Option<&FrontierSyncEntry>) -> String {
    let Some(entry) = entry else {
        return "- status: unknown\n- sync: Raspberry frontier state unavailable".to_string();
    };
    let mut lines = vec![
        format!("- status: {}", entry.status),
        format!("- lane: `{}`", entry.lane_key),
        format!("- kind: {}", entry.lane_kind),
        format!("- detail: {}", entry.detail),
    ];
    if let Some(stage) = entry.current_stage.as_ref() {
        lines.push(format!("- current stage: {}", stage));
    }
    if let Some(run_id) = entry.current_run_id.as_ref() {
        lines.push(format!("- current run: {}", run_id));
    }
    if let Some(run_id) = entry.last_run_id.as_ref() {
        lines.push(format!("- last run: {}", run_id));
    }
    if let Some(reason) = entry.blocker_reason.as_ref() {
        lines.push(format!("- blocker: {}", reason));
    }
    if let Some((landing_state, landing_detail)) = trunk_delivery_state_for_run(
        entry
            .last_run_id
            .as_deref()
            .or(entry.current_run_id.as_deref()),
    ) {
        lines.push(format!("- trunk landing: {}", landing_state));
        lines.push(format!("- trunk landing detail: {}", landing_detail));
    }
    lines.push(format!("- next move: {}", entry.next_operator_move));
    lines.join("\n")
}

fn trunk_delivery_state_for_run(run_id: Option<&str>) -> Option<(String, String)> {
    let run_id = run_id?;
    let base = fabro_workflows::run_lookup::default_runs_base();
    let run_dir = fabro_workflows::run_lookup::find_run_by_prefix(&base, run_id).ok()?;
    let run_config = fabro_config::run::load_run_config(&run_dir.join("run.toml")).ok()?;
    if !run_config
        .integration
        .as_ref()
        .is_some_and(|config| config.enabled)
    {
        return None;
    }

    if let Ok(raw) = std::fs::read_to_string(run_dir.join("direct_integration.json")) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
            let target_branch = value
                .get("target_branch")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let pushed = value
                .get("pushed")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            if pushed {
                return Some((
                    "landed".to_string(),
                    format!("integrated to {target_branch}"),
                ));
            }
            return Some((
                "not_landed".to_string(),
                format!("integration recorded locally for {target_branch}"),
            ));
        }
    }

    if let Ok(raw) = std::fs::read_to_string(run_dir.join("progress.jsonl")) {
        let mut failed_pushes = Vec::new();
        for line in raw.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if value.get("event").and_then(|value| value.as_str()) != Some("GitPush") {
                continue;
            }
            if value.get("success").and_then(|value| value.as_bool()) == Some(false) {
                let branch = value
                    .get("branch")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown");
                failed_pushes.push(branch.to_string());
            }
        }
        if !failed_pushes.is_empty() {
            return Some((
                "push_failed".to_string(),
                format!("push failed for {}", failed_pushes.join(", ")),
            ));
        }
    }

    Some((
        "not_landed".to_string(),
        "integration enabled but no landed record was found".to_string(),
    ))
}

fn repo_relative_display(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn normalize_storage_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn paperclip_wake_command(paths: &PaperclipPaths, agent_slug: &str) -> String {
    format!(
        "bash {} wake --agent {}",
        repo_relative_display(&paths.paperclip_cli_script_path, &paths.target_repo),
        shell_quote(agent_slug),
    )
}

fn paperclip_refresh_command(paths: &PaperclipPaths) -> String {
    format!(
        "bash {} refresh",
        repo_relative_display(&paths.paperclip_cli_script_path, &paths.target_repo),
    )
}

fn paperclip_status_command(paths: &PaperclipPaths) -> String {
    format!(
        "bash {} status",
        repo_relative_display(&paths.paperclip_cli_script_path, &paths.target_repo),
    )
}

fn frontier_root_sync_key(program_id: &str) -> String {
    format!("frontier/{program_id}/root")
}

fn lane_sync_key(program_id: &str, lane_key: &str) -> String {
    format!("frontier/{program_id}/lane/{lane_key}")
}

fn plan_root_sync_key(program_id: &str, plan_id: &str) -> String {
    format!("plan/{program_id}/{plan_id}")
}

fn plan_child_sync_key(program_id: &str, plan_id: &str, child_id: &str) -> String {
    format!("plan/{program_id}/{plan_id}/{child_id}")
}

fn load_primary_or_genesis_plan_registry(target_repo: &Path) -> Option<PlanRegistry> {
    let primary = load_plan_registry(target_repo).ok();
    if primary
        .as_ref()
        .is_some_and(|registry| !registry.plans.is_empty())
    {
        return primary;
    }

    let genesis = load_plan_registry_from_planning_root(target_repo, Path::new("genesis")).ok();
    if genesis
        .as_ref()
        .is_some_and(|registry| !registry.plans.is_empty())
    {
        return genesis;
    }

    infer_plan_registry_from_genesis(target_repo).ok()
}

fn infer_plan_registry_from_genesis(target_repo: &Path) -> Result<PlanRegistry> {
    let plans_dir = target_repo.join("genesis").join("plans");
    if !plans_dir.is_dir() {
        return Ok(PlanRegistry { plans: Vec::new() });
    }

    let mut paths = std::fs::read_dir(&plans_dir)
        .with_context(|| format!("failed to read {}", plans_dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("failed to enumerate {}", plans_dir.display()))?;
    paths.sort();

    let plans = paths
        .into_iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .filter_map(|absolute_path| {
            let stem = absolute_path.file_stem()?.to_str()?;
            let prefix = stem.split('-').next().unwrap_or_default();
            if prefix.len() != 3 || !prefix.chars().all(|ch| ch.is_ascii_digit()) {
                return None;
            }

            let Ok(body) = std::fs::read_to_string(&absolute_path) else {
                return None;
            };
            let Ok(relative_path) = absolute_path.strip_prefix(target_repo) else {
                return None;
            };
            let title = body
                .lines()
                .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| stem.to_string());
            let mut plan_id = stem
                .split_once('-')
                .map(|(_, rest)| rest.to_string())
                .unwrap_or_else(|| stem.to_string());
            for suffix in ["-game", "-plan", "-crate", "-trait"] {
                if let Some(stripped) = plan_id.strip_suffix(suffix) {
                    plan_id = stripped.to_string();
                }
            }

            let plan_id_lower = plan_id.to_ascii_lowercase();
            let category = if plan_id_lower == "master" {
                raspberry_supervisor::PlanCategory::Meta
            } else if plan_id_lower.contains("test")
                || plan_id_lower.contains("verify")
                || plan_id_lower.contains("verification")
                || plan_id_lower.contains("performance")
            {
                raspberry_supervisor::PlanCategory::Verification
            } else if plan_id_lower.contains("infrastructure")
                || plan_id_lower.contains("chain")
                || plan_id_lower.contains("install")
            {
                raspberry_supervisor::PlanCategory::Infrastructure
            } else if plan_id_lower.contains("tui")
                || plan_id_lower.contains("ui")
                || plan_id_lower.contains("dashboard")
            {
                raspberry_supervisor::PlanCategory::Interface
            } else if plan_id_lower.contains("wallet")
                || plan_id_lower.contains("house")
                || plan_id_lower.contains("blueprint")
                || plan_id_lower.contains("faucet")
            {
                raspberry_supervisor::PlanCategory::Service
            } else {
                raspberry_supervisor::PlanCategory::Unknown
            };

            Some(raspberry_supervisor::PlanRecord {
                plan_id,
                path: relative_path.to_path_buf(),
                title,
                category,
                composite: true,
                dependency_plan_ids: Vec::new(),
                mapping_contract_path: None,
                mapping_source: raspberry_supervisor::PlanMappingSource::Inferred,
                bootstrap_required: category != raspberry_supervisor::PlanCategory::Meta,
                implementation_required: category != raspberry_supervisor::PlanCategory::Meta,
                declared_child_ids: Vec::new(),
                children: Vec::new(),
            })
        })
        .collect();

    Ok(PlanRegistry { plans })
}

fn build_plan_dashboard_model(
    program_id: &str,
    registry: &PlanRegistry,
    matrix: Option<&PlanMatrix>,
    frontier: &FrontierSyncModel,
) -> PlanDashboardModel {
    let status_by_id: BTreeMap<&str, &PlanStatusRow> = matrix
        .map(|m| {
            m.rows
                .iter()
                .map(|row| (row.plan_id.as_str(), row))
                .collect()
        })
        .unwrap_or_default();

    let mut plans = Vec::new();
    let mut represented = 0usize;
    let mut in_motion = 0usize;
    let mut needs_attention = 0usize;
    let mut complete = 0usize;

    for plan in &registry.plans {
        let row = status_by_id.get(plan.plan_id.as_str());
        let live_plan_entry = plan_frontier_entry(frontier, &plan.plan_id);
        let status = row
            .map(|r| r.current_status.clone())
            .or_else(|| live_plan_entry.map(|entry| entry.status.to_string()))
            .unwrap_or_else(|| "unknown".to_string());
        let risk = row.map(|r| r.current_risk.clone()).unwrap_or_default();
        let next_move = row
            .map(|r| r.next_operator_move.clone())
            .or_else(|| live_plan_entry.map(|entry| entry.next_operator_move.clone()))
            .unwrap_or_else(|| "inspect plan".to_string());

        if row.map(|r| r.represented_in_blueprint).unwrap_or(false) {
            represented += 1;
        }
        if status.contains("running") || status.contains("ready") {
            in_motion += 1;
        }
        if status.contains("failed") || status.contains("blocked") || status == "unmodeled" {
            needs_attention += 1;
        }
        if status.contains("complete") || status == "reviewed" {
            complete += 1;
        }

        let children: Vec<PlanDashboardChild> = plan
            .children
            .iter()
            .map(|child| {
                let live_entry = frontier_child_entry(frontier, &child.child_id);
                PlanDashboardChild {
                    child_id: child.child_id.clone(),
                    title: child
                        .title
                        .clone()
                        .unwrap_or_else(|| child.child_id.clone()),
                    archetype: child.archetype.map(|a| a.as_str().to_string()),
                    review_profile: child.review_profile.map(|p| p.as_str().to_string()),
                    owned_surfaces: child.owned_surfaces.clone(),
                    sync_key: plan_child_sync_key(program_id, &plan.plan_id, &child.child_id),
                    status: live_entry
                        .map(|entry| entry.status.to_string())
                        .unwrap_or_else(|| "unmodeled".to_string()),
                    current_stage: live_entry.and_then(|entry| entry.current_stage.clone()),
                    current_run_id: live_entry.and_then(|entry| entry.current_run_id.clone()),
                    next_operator_move: live_entry
                        .map(|entry| entry.next_operator_move.clone())
                        .unwrap_or_else(|| next_move.clone()),
                }
            })
            .collect();

        let mapping_source = match plan.mapping_source {
            PlanMappingSource::Contract => "contract",
            PlanMappingSource::Inferred => "inferred",
        };

        plans.push(PlanDashboardEntry {
            plan_id: plan.plan_id.clone(),
            title: plan.title.clone(),
            category: plan.category.as_str().to_string(),
            composite: plan.composite,
            path: plan.path.display().to_string(),
            status,
            current_stage: live_plan_entry.and_then(|entry| entry.current_stage.clone()),
            current_run_id: live_plan_entry.and_then(|entry| entry.current_run_id.clone()),
            risk,
            next_move,
            mapping_source: mapping_source.to_string(),
            children,
        });
    }

    PlanDashboardModel {
        program: program_id.to_string(),
        plans,
        summary: PlanDashboardSummary {
            total: registry.plans.len(),
            represented,
            in_motion,
            needs_attention,
            complete,
        },
    }
}

fn plan_frontier_entry<'a>(
    frontier: &'a FrontierSyncModel,
    plan_id: &str,
) -> Option<&'a FrontierSyncEntry> {
    frontier
        .entries
        .iter()
        .find(|entry| entry.unit_id == plan_id && entry.lane_id == plan_id)
        .or_else(|| {
            frontier
                .entries
                .iter()
                .find(|entry| entry.unit_id == plan_id)
        })
}

fn frontier_child_entry<'a>(
    frontier: &'a FrontierSyncModel,
    child_id: &str,
) -> Option<&'a FrontierSyncEntry> {
    frontier
        .entries
        .iter()
        .find(|entry| entry.unit_id == child_id)
        .or_else(|| {
            frontier
                .entries
                .iter()
                .find(|entry| entry.lane_id == child_id)
        })
}

fn write_paperclip_cli_script(
    path: &Path,
    fabro_binary: &Path,
    target_repo: &Path,
    program_id: &str,
) -> Result<()> {
    let body = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\n\nfabro_bin=\"${{FABRO_BIN:-}}\"\nif [ -z \"$fabro_bin\" ]; then\n  if [ -x {fabro_fallback} ]; then\n    fabro_bin={fabro_fallback}\n  elif command -v fabro >/dev/null 2>&1; then\n    fabro_bin=\"$(command -v fabro)\"\n  else\n    echo \"Unable to resolve fabro binary. Set FABRO_BIN or install/build fabro.\" >&2\n    exit 1\n  fi\nfi\n\nexec \"$fabro_bin\" paperclip \"$@\" --target-repo {target_repo} --program {program}\n",
        fabro_fallback = shell_quote(&fabro_binary.display().to_string()),
        target_repo = shell_quote(&target_repo.display().to_string()),
        program = shell_quote(program_id),
    );
    std::fs::write(path, body)?;
    Ok(())
}

fn write_orchestrator_script(
    path: &Path,
    target_repo: &Path,
    manifest_path: &Path,
    fabro_binary: &Path,
    raspberry_binary: Option<&Path>,
) -> Result<()> {
    let fabro_fallback = shell_quote(&fabro_binary.display().to_string());
    let raspberry_resolution = raspberry_binary
        .map(|path| {
            format!(
                "  if [ -x {fallback} ]; then\n    raspberry_bin={fallback}\n  elif command -v raspberry >/dev/null 2>&1; then\n    raspberry_bin=\"$(command -v raspberry)\"\n  else\n    echo \"Unable to resolve raspberry binary. Set RASPBERRY_BIN or install/build raspberry.\" >&2\n    exit 1\n  fi\n",
                fallback = shell_quote(&path.display().to_string()),
            )
        })
        .unwrap_or_else(|| {
            "  if command -v raspberry >/dev/null 2>&1; then\n    raspberry_bin=\"$(command -v raspberry)\"\n  else\n    echo \"Unable to resolve raspberry binary. Set RASPBERRY_BIN or install/build raspberry.\" >&2\n    exit 1\n  fi\n".to_string()
        });
    let body = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\ncd {repo}\n\nfabro_bin=\"${{FABRO_BIN:-}}\"\nif [ -z \"$fabro_bin\" ]; then\n  if [ -x {fabro_fallback} ]; then\n    fabro_bin={fabro_fallback}\n  elif command -v fabro >/dev/null 2>&1; then\n    fabro_bin=\"$(command -v fabro)\"\n  else\n    echo \"Unable to resolve fabro binary. Set FABRO_BIN or install/build fabro.\" >&2\n    exit 1\n  fi\nfi\n\nraspberry_bin=\"${{RASPBERRY_BIN:-}}\"\nif [ -z \"$raspberry_bin\" ]; then\n{raspberry_resolution}fi\n\nexec \"$raspberry_bin\" autodev --manifest {manifest} --fabro-bin \"$fabro_bin\" --max-cycles 1 --poll-interval-ms 1 --evolve-every-seconds 0\n",
        repo = shell_quote(&target_repo.display().to_string()),
        fabro_fallback = fabro_fallback,
        raspberry_resolution = raspberry_resolution,
        manifest = shell_quote(&manifest_path.display().to_string()),
    );
    std::fs::write(path, body)?;
    Ok(())
}

fn write_minimax_agent_script(
    path: &Path,
    fabro_agent_binary: Option<&Path>,
    fabro_repo: Option<&Path>,
) -> Result<()> {
    let binary_resolution = fabro_agent_binary
        .map(|path| {
            format!(
                "  if [ -x {fallback} ]; then\n    fabro_agent_bin={fallback}\n  elif command -v fabro-agent >/dev/null 2>&1; then\n    fabro_agent_bin=\"$(command -v fabro-agent)\"\n  else\n    fabro_agent_bin=\"\"\n  fi\n",
                fallback = shell_quote(&path.display().to_string()),
            )
        })
        .unwrap_or_else(|| {
            "  if command -v fabro-agent >/dev/null 2>&1; then\n    fabro_agent_bin=\"$(command -v fabro-agent)\"\n  else\n    fabro_agent_bin=\"\"\n  fi\n".to_string()
        });
    let cargo_fallback = fabro_repo.map(|repo| {
        format!(
            "cd {repo}\nexec cargo run --quiet -p fabro-agent -- --provider minimax --model {model} --permissions full --auto-approve --output-format json -- \"$prompt\"\n",
            repo = shell_quote(&repo.display().to_string()),
            model = shell_quote(PAPERCLIP_DEFAULT_AUTOMATION_MODEL),
        )
    }).unwrap_or_else(|| {
        "echo \"Unable to resolve fabro-agent. Set FABRO_AGENT_BIN or run from a fabro checkout.\" >&2\nexit 1\n".to_string()
    });
    let body = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\n\nprompt=\"$(cat)\"\n\nfabro_agent_bin=\"${{FABRO_AGENT_BIN:-}}\"\nif [ -z \"$fabro_agent_bin\" ]; then\n{binary_resolution}fi\n\nif [ -n \"$fabro_agent_bin\" ]; then\n  exec \"$fabro_agent_bin\" --provider minimax --model {model} --permissions full --auto-approve --output-format json -- \"$prompt\"\nfi\n\n{cargo_fallback}",
        binary_resolution = binary_resolution,
        model = shell_quote(PAPERCLIP_DEFAULT_AUTOMATION_MODEL),
        cargo_fallback = cargo_fallback,
    );
    std::fs::write(path, body)?;
    Ok(())
}

fn write_run_script(path: &Path, paperclip_repo: Option<PathBuf>, data_dir: &Path) -> Result<()> {
    let tmp_dir = paperclip_instance_root(data_dir).join("tmp");
    let fallback_repo = paperclip_repo
        .as_ref()
        .map(|path| shell_quote(&path.display().to_string()));
    let resolution = fallback_repo
        .map(|repo| {
            format!(
                "elif [ -n \"${{PAPERCLIP_REPO:-}}\" ]; then\n  exec pnpm --silent --dir \"$PAPERCLIP_REPO\" paperclipai run --data-dir {data_dir}\nelif [ -d {repo} ]; then\n  exec pnpm --silent --dir {repo} paperclipai run --data-dir {data_dir}\nelse\n  echo \"Unable to resolve Paperclip CLI. Set PAPERCLIP_CMD, set PAPERCLIP_REPO, or install paperclipai.\" >&2\n  exit 1\nfi\n",
                repo = repo,
                data_dir = shell_quote(&data_dir.display().to_string()),
            )
        })
        .unwrap_or_else(|| {
            format!(
                "elif [ -n \"${{PAPERCLIP_REPO:-}}\" ]; then\n  exec pnpm --silent --dir \"$PAPERCLIP_REPO\" paperclipai run --data-dir {data_dir}\nelse\n  echo \"Unable to resolve Paperclip CLI. Set PAPERCLIP_CMD, set PAPERCLIP_REPO, or install paperclipai.\" >&2\n  exit 1\nfi\n",
                data_dir = shell_quote(&data_dir.display().to_string()),
            )
        });
    let body = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\nmkdir -p {tmp_dir}\nexport TMPDIR={tmp_dir}\n\nif [ -n \"${{PAPERCLIP_CMD:-}}\" ]; then\n  exec bash -lc \"$PAPERCLIP_CMD run --data-dir {data_dir}\"\nelif command -v paperclipai >/dev/null 2>&1; then\n  exec paperclipai run --data-dir {data_dir}\n{resolution}",
        tmp_dir = shell_quote(&tmp_dir.display().to_string()),
        data_dir = shell_quote(&data_dir.display().to_string()),
        resolution = resolution,
    );
    std::fs::write(path, body)?;
    Ok(())
}

fn current_fabro_binary() -> Result<PathBuf> {
    std::env::current_exe().context("failed to resolve current fabro binary")
}

fn current_fabro_agent_binary() -> Option<PathBuf> {
    let current = std::env::current_exe().ok()?;
    let sibling = current.with_file_name("fabro-agent");
    sibling.exists().then_some(sibling)
}

fn current_raspberry_binary() -> Option<PathBuf> {
    let current = std::env::current_exe().ok()?;
    let sibling = current.with_file_name("raspberry");
    sibling.exists().then_some(sibling)
}

fn default_fabro_repo() -> Option<PathBuf> {
    let local_repo = PathBuf::from("/home/r/coding/fabro");
    local_repo.is_dir().then_some(local_repo)
}

fn default_paperclip_repo() -> Option<PathBuf> {
    let local_repo = PathBuf::from("/home/r/coding/paperclip");
    local_repo.is_dir().then_some(local_repo)
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

fn paperclip_server_command(override_value: Option<&str>, data_dir: &Path) -> Result<String> {
    let tmp_dir = paperclip_instance_root(data_dir).join("tmp");
    if let Some(value) = override_value {
        return Ok(format!(
            "mkdir -p {tmp} && exec env TMPDIR={tmp} {cmd} run --data-dir {data_dir}",
            tmp = shell_quote(&tmp_dir.display().to_string()),
            cmd = value,
            data_dir = shell_quote(&data_dir.display().to_string()),
        ));
    }

    let local_repo = Path::new("/home/r/coding/paperclip");
    let tsx_cli = local_repo.join("cli/node_modules/tsx/dist/cli.mjs");
    let server_entry = local_repo.join("server/src/index.ts");
    let config_path = paperclip_instance_root(data_dir).join("config.json");
    let env_path = paperclip_instance_root(data_dir).join(".env");
    if local_repo.is_dir() && tsx_cli.exists() && server_entry.exists() {
        return Ok(format!(
            "cd {} && mkdir -p {} && exec env TMPDIR={} PAPERCLIP_HOME={} PAPERCLIP_CONFIG={} DOTENV_CONFIG_PATH={} PAPERCLIP_UI_DEV_MIDDLEWARE=true node {} {}",
            shell_quote(&local_repo.display().to_string()),
            shell_quote(&tmp_dir.display().to_string()),
            shell_quote(&tmp_dir.display().to_string()),
            shell_quote(&data_dir.display().to_string()),
            shell_quote(&config_path.display().to_string()),
            shell_quote(&env_path.display().to_string()),
            shell_quote(&tsx_cli.display().to_string()),
            shell_quote(&server_entry.display().to_string()),
        ));
    }

    Ok(format!(
        "mkdir -p {tmp} && exec env TMPDIR={tmp} {cmd} run --data-dir {data_dir}",
        tmp = shell_quote(&tmp_dir.display().to_string()),
        cmd = paperclip_command(None),
        data_dir = shell_quote(&data_dir.display().to_string()),
    ))
}

fn ensure_local_paperclip_instance(data_dir: &Path, api_base: &str) -> Result<()> {
    let instance_root = paperclip_instance_root(data_dir);
    let config_path = instance_root.join("config.json");
    if !config_path.exists() {
        seed_local_paperclip_config(&config_path, api_base)?;
    }
    ensure_local_paperclip_env(&config_path)?;
    ensure_local_paperclip_master_key(&config_path)?;
    Ok(())
}

fn paperclip_instance_root(data_dir: &Path) -> PathBuf {
    data_dir.join("instances").join("default")
}

fn paperclip_server_host_port(api_base: &str) -> (String, u16) {
    reqwest::Url::parse(api_base)
        .ok()
        .and_then(|url| {
            let host = url.host_str()?.to_string();
            let port = url.port_or_known_default()?;
            Some((host, port))
        })
        .unwrap_or_else(|| ("127.0.0.1".to_string(), 3100))
}

fn seed_local_paperclip_config(config_path: &Path, api_base: &str) -> Result<()> {
    let instance_root = config_path
        .parent()
        .context("paperclip config path should have a parent directory")?;
    let db_dir = instance_root.join("db");
    let backup_dir = instance_root.join("data").join("backups");
    let storage_dir = instance_root.join("data").join("storage");
    let log_dir = instance_root.join("logs");
    let key_file_path = instance_root.join("secrets").join("master.key");
    std::fs::create_dir_all(instance_root)?;
    let (server_host, server_port) = paperclip_server_host_port(api_base);

    let config = json!({
        "$meta": {
            "version": 1,
            "updatedAt": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            "source": "onboard",
        },
        "database": {
            "mode": "embedded-postgres",
            "embeddedPostgresDataDir": db_dir.display().to_string(),
            "embeddedPostgresPort": 54329,
            "backup": {
                "enabled": true,
                "intervalMinutes": 60,
                "retentionDays": 30,
                "dir": backup_dir.display().to_string(),
            },
        },
        "logging": {
            "mode": "file",
            "logDir": log_dir.display().to_string(),
        },
        "server": {
            "deploymentMode": "local_trusted",
            "exposure": "private",
            "host": server_host,
            "port": server_port,
            "allowedHostnames": [],
            "serveUi": true,
        },
        "auth": {
            "baseUrlMode": "auto",
            "disableSignUp": false,
        },
        "storage": {
            "provider": "local_disk",
            "localDisk": {
                "baseDir": storage_dir.display().to_string(),
            },
            "s3": {
                "bucket": "paperclip",
                "region": "us-east-1",
                "prefix": "",
                "forcePathStyle": false,
            },
        },
        "secrets": {
            "provider": "local_encrypted",
            "strictMode": false,
            "localEncrypted": {
                "keyFilePath": key_file_path.display().to_string(),
            },
        },
    });

    write_private_file(
        config_path,
        &format!("{}\n", serde_json::to_string_pretty(&config)?),
    )?;
    Ok(())
}

fn ensure_local_paperclip_env(config_path: &Path) -> Result<()> {
    let env_path = config_path
        .parent()
        .context("paperclip config path should have a parent directory")?
        .join(".env");
    let secret = random_secret();
    if !env_path.exists() {
        write_private_file(
            &env_path,
            &format!("# Paperclip environment variables\nPAPERCLIP_AGENT_JWT_SECRET={secret}\n"),
        )?;
        return Ok(());
    }

    let contents = std::fs::read_to_string(&env_path)?;
    if contents.contains("PAPERCLIP_AGENT_JWT_SECRET=") {
        return Ok(());
    }
    let trimmed = contents.trim_end();
    let next = if trimmed.is_empty() {
        format!("PAPERCLIP_AGENT_JWT_SECRET={secret}\n")
    } else {
        format!("{trimmed}\nPAPERCLIP_AGENT_JWT_SECRET={secret}\n")
    };
    write_private_file(&env_path, &next)?;
    Ok(())
}

fn ensure_local_paperclip_master_key(config_path: &Path) -> Result<()> {
    let key_path = config_path
        .parent()
        .context("paperclip config path should have a parent directory")?
        .join("secrets")
        .join("master.key");
    if key_path.exists() {
        return Ok(());
    }
    if let Some(parent) = key_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let key = base64::engine::general_purpose::STANDARD.encode(rand::random::<[u8; 32]>());
    write_private_file(&key_path, &format!("{key}\n"))?;
    Ok(())
}

fn random_secret() -> String {
    format!(
        "{}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rand::random::<[u8; 24]>()),
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rand::random::<[u8; 24]>()),
    )
}

fn write_private_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    file.write_all(contents.as_bytes())?;
    set_private_permissions(path)?;
    Ok(())
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

async fn ensure_paperclip_server(
    paperclip_cmd_override: Option<&str>,
    data_dir: &Path,
    api_base: &str,
) -> Result<()> {
    ensure_local_paperclip_instance(data_dir, api_base)?;
    if paperclip_server_ready(api_base).await {
        return Ok(());
    }

    let log_path = data_dir.join("server.log");
    let pid_path = data_dir.join("server.pid");
    std::fs::create_dir_all(data_dir)?;
    let server_cmd = paperclip_server_command(paperclip_cmd_override, data_dir)?;
    let child = spawn_detached_paperclip_server(&server_cmd, &log_path)
        .context("failed to start paperclip server")?;
    write_private_file(&pid_path, &format!("{}\n", child.id()))?;

    for _ in 0..30 {
        if paperclip_server_ready(api_base).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("paperclip server did not become ready at {api_base}");
}

fn spawn_detached_paperclip_server(
    server_cmd: &str,
    log_path: &Path,
) -> Result<std::process::Child> {
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let stderr = stdout.try_clone()?;

    match Command::new("setsid")
        .arg("bash")
        .arg("-lc")
        .arg(server_cmd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
    {
        Ok(child) => Ok(child),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            let stdout = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)?;
            let stderr = stdout.try_clone()?;
            Ok(Command::new("bash")
                .arg("-lc")
                .arg(server_cmd)
                .stdin(Stdio::null())
                .stdout(Stdio::from(stdout))
                .stderr(Stdio::from(stderr))
                .spawn()?)
        }
        Err(error) => Err(error.into()),
    }
}

async fn paperclip_server_ready(api_base: &str) -> bool {
    reqwest::Client::new()
        .get(format!("{api_base}/api/health"))
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

async fn import_company_package(
    _paperclip_cmd: &str,
    _data_dir: &Path,
    api_base: &str,
    bundle_root: &Path,
    company_name: &str,
    existing_company_id: Option<&str>,
) -> Result<PaperclipImportResult> {
    let manifest_path = bundle_root.join("paperclip.manifest.json");
    let manifest_raw = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_raw)
        .with_context(|| format!("failed to parse {}", manifest_path.display()))?;

    let company_path_str = manifest
        .get("company")
        .and_then(|c| c.get("path"))
        .and_then(|p| p.as_str())
        .unwrap_or("COMPANY.md");
    let company_md =
        std::fs::read_to_string(bundle_root.join(company_path_str)).unwrap_or_default();

    let mut agent_files = serde_json::Map::new();
    if let Some(agents) = manifest.get("agents").and_then(|a| a.as_array()) {
        for agent in agents {
            if let Some(path) = agent.get("path").and_then(|p| p.as_str()) {
                let content = std::fs::read_to_string(bundle_root.join(path)).unwrap_or_default();
                agent_files.insert(path.to_string(), json!(content));
            }
        }
    }

    let mut files = serde_json::Map::new();
    files.insert(company_path_str.to_string(), json!(company_md));
    for (path, content) in &agent_files {
        files.insert(path.clone(), content.clone());
    }

    // The Paperclip manifest schema expects source to be null or {companyId: uuid, companyName: string}.
    // Our generated manifests have source: null which is valid.
    let mut manifest = manifest;
    if manifest
        .get("company")
        .and_then(|c| c.get("brandColor"))
        .map(|v| v.is_null())
        == Some(true)
    {
        manifest["company"]["brandColor"] = json!("");
    }

    let target = if let Some(company_id) = existing_company_id {
        json!({
            "mode": "existing_company",
            "companyId": company_id,
            "collision": "replace"
        })
    } else {
        json!({
            "mode": "new_company",
            "name": company_name
        })
    };

    let body = json!({
        "source": {
            "type": "inline",
            "manifest": manifest,
            "files": files,
        },
        "target": target,
        "include": {
            "company": true,
            "agents": true
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{api_base}/api/companies/import"))
        .json(&body)
        .send()
        .await
        .context("failed to send import request")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        bail!("paperclip company import failed ({}): {}", status, text);
    }

    response
        .json::<PaperclipImportResult>()
        .await
        .context("failed to parse import response")
}

async fn sync_company_secrets(
    api_base: &str,
    company_id: &str,
    existing_state: Option<&serde_json::Value>,
) -> Result<BTreeMap<String, SyncedSecret>> {
    let existing_secrets = list_company_secrets(api_base, company_id).await?;
    let mut synced = BTreeMap::new();

    for spec in SECRET_SYNC_SPECS {
        let Some(value) = std::env::var_os(spec.env_key).and_then(|value| {
            let value = value.to_string_lossy().trim().to_string();
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        }) else {
            continue;
        };

        let value_hash = sha256_hex(&value);
        let previous = existing_state
            .and_then(|state| state.get("secrets"))
            .and_then(|value| value.get(spec.env_key));
        let previous_secret_id = previous
            .and_then(|value| value.get("secretId"))
            .and_then(|value| value.as_str());
        let previous_hash = previous
            .and_then(|value| value.get("valueHash"))
            .and_then(|value| value.as_str());
        let existing_secret = previous_secret_id
            .and_then(|secret_id| {
                existing_secrets
                    .iter()
                    .find(|secret| secret.id == secret_id)
            })
            .or_else(|| {
                existing_secrets
                    .iter()
                    .find(|secret| secret.name == spec.secret_name)
            });

        let secret = if let Some(secret) = existing_secret {
            if previous_hash != Some(value_hash.as_str()) {
                rotate_company_secret(api_base, &secret.id, &value).await?
            } else {
                secret.clone()
            }
        } else {
            create_company_secret(
                api_base,
                company_id,
                spec.secret_name,
                spec.description,
                &value,
            )
            .await?
        };

        synced.insert(
            spec.env_key.to_string(),
            SyncedSecret {
                secret_id: secret.id,
                name: secret.name,
                value_hash,
            },
        );
    }

    Ok(synced)
}

fn synced_secrets_to_json(secrets: &BTreeMap<String, SyncedSecret>) -> serde_json::Value {
    let mut values = serde_json::Map::new();
    for (env_key, secret) in secrets {
        values.insert(
            env_key.clone(),
            json!({
                "secretId": secret.secret_id,
                "name": secret.name,
                "valueHash": secret.value_hash,
            }),
        );
    }
    serde_json::Value::Object(values)
}

async fn list_company_secrets(api_base: &str, company_id: &str) -> Result<Vec<PaperclipSecret>> {
    reqwest::Client::new()
        .get(format!("{api_base}/api/companies/{company_id}/secrets"))
        .send()
        .await
        .context("failed to list paperclip secrets")?
        .error_for_status()
        .context("paperclip secret list request failed")?
        .json::<Vec<PaperclipSecret>>()
        .await
        .context("failed to parse paperclip secret list response")
}

async fn create_company_secret(
    api_base: &str,
    company_id: &str,
    name: &str,
    description: &str,
    value: &str,
) -> Result<PaperclipSecret> {
    reqwest::Client::new()
        .post(format!("{api_base}/api/companies/{company_id}/secrets"))
        .json(&json!({
            "name": name,
            "description": description,
            "value": value,
        }))
        .send()
        .await
        .context("failed to create paperclip secret")?
        .error_for_status()
        .context("paperclip secret create request failed")?
        .json::<PaperclipSecret>()
        .await
        .context("failed to parse created paperclip secret")
}

async fn rotate_company_secret(
    api_base: &str,
    secret_id: &str,
    value: &str,
) -> Result<PaperclipSecret> {
    reqwest::Client::new()
        .post(format!("{api_base}/api/secrets/{secret_id}/rotate"))
        .json(&json!({
            "value": value,
        }))
        .send()
        .await
        .with_context(|| format!("failed to rotate paperclip secret {secret_id}"))?
        .error_for_status()
        .with_context(|| format!("paperclip secret rotate request failed for {secret_id}"))?
        .json::<PaperclipSecret>()
        .await
        .context("failed to parse rotated paperclip secret")
}

async fn wire_generated_agent_secrets(
    api_base: &str,
    bundle_agents: &[BundleAgent],
    imported_agents: &[PaperclipImportAgent],
    synced_secrets: &BTreeMap<String, SyncedSecret>,
) -> Result<()> {
    for bundle_agent in bundle_agents {
        let Some(imported) = imported_agents
            .iter()
            .find(|agent| agent.slug == bundle_agent.slug)
        else {
            continue;
        };
        let Some(agent_id) = imported.id.as_deref() else {
            continue;
        };
        let Some(secret_env_key) = secret_env_key_for_adapter(bundle_agent.adapter_type) else {
            continue;
        };
        let Some(secret) = synced_secrets.get(secret_env_key) else {
            continue;
        };

        let agent = get_managed_agent(api_base, agent_id).await?;
        let existing_config = agent
            .adapter_config
            .as_ref()
            .and_then(|value| value.as_object())
            .cloned()
            .unwrap_or_default();
        let existing_env = existing_config
            .get("env")
            .and_then(|value| value.as_object())
            .cloned()
            .unwrap_or_default();
        let desired_binding = json!({
            "type": "secret_ref",
            "secretId": secret.secret_id,
            "version": "latest",
        });
        if existing_env.get(secret_env_key) == Some(&desired_binding) {
            continue;
        }

        let mut next_env = existing_env;
        next_env.insert(secret_env_key.to_string(), desired_binding);
        let mut next_config = existing_config;
        next_config.insert("env".to_string(), serde_json::Value::Object(next_env));
        patch_managed_agent_config(api_base, agent_id, &serde_json::Value::Object(next_config))
            .await?;
    }

    Ok(())
}

fn secret_env_key_for_adapter(adapter_type: &str) -> Option<&'static str> {
    match adapter_type {
        "claude_local" => Some("ANTHROPIC_API_KEY"),
        _ => None,
    }
}

async fn get_managed_agent(api_base: &str, agent_id: &str) -> Result<PaperclipManagedAgent> {
    reqwest::Client::new()
        .get(format!("{api_base}/api/agents/{agent_id}"))
        .send()
        .await
        .with_context(|| format!("failed to fetch paperclip agent {agent_id}"))?
        .error_for_status()
        .with_context(|| format!("paperclip agent get request failed for {agent_id}"))?
        .json::<PaperclipManagedAgent>()
        .await
        .context("failed to parse paperclip agent response")
}

async fn patch_managed_agent_config(
    api_base: &str,
    agent_id: &str,
    adapter_config: &serde_json::Value,
) -> Result<PaperclipManagedAgent> {
    reqwest::Client::new()
        .patch(format!("{api_base}/api/agents/{agent_id}"))
        .json(&json!({
            "adapterConfig": adapter_config,
        }))
        .send()
        .await
        .with_context(|| format!("failed to patch paperclip agent {agent_id}"))?
        .error_for_status()
        .with_context(|| format!("paperclip agent patch request failed for {agent_id}"))?
        .json::<PaperclipManagedAgent>()
        .await
        .context("failed to parse patched paperclip agent")
}

fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn sha256_bytes_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

async fn resolve_managed_agent(
    api_base: &str,
    company_id: &str,
    agent_slug: &str,
    existing_state: Option<&serde_json::Value>,
) -> Result<PaperclipManagedAgent> {
    if let Some(agent_id) = existing_state
        .and_then(|state| state.get("agents"))
        .and_then(|value| value.as_array())
        .and_then(|agents| {
            agents.iter().find_map(|agent| {
                (agent.get("slug").and_then(|value| value.as_str()) == Some(agent_slug))
                    .then(|| agent.get("id").and_then(|value| value.as_str()))
                    .flatten()
            })
        })
    {
        if let Ok(agent) = get_managed_agent(api_base, agent_id).await {
            return Ok(agent);
        }
    }

    let agents = list_company_agents(api_base, company_id).await?;
    agents
        .into_iter()
        .find(|agent| agent.slug.as_deref() == Some(agent_slug))
        .with_context(|| format!("paperclip agent `{agent_slug}` not found"))
}

async fn list_company_agents(
    api_base: &str,
    company_id: &str,
) -> Result<Vec<PaperclipManagedAgent>> {
    reqwest::Client::new()
        .get(format!("{api_base}/api/companies/{company_id}/agents"))
        .send()
        .await
        .context("failed to list paperclip agents")?
        .error_for_status()
        .context("paperclip agent list request failed")?
        .json::<Vec<PaperclipManagedAgent>>()
        .await
        .context("failed to parse paperclip agent list response")
}

async fn invoke_agent_heartbeat(
    api_base: &str,
    agent_id: &str,
) -> Result<PaperclipHeartbeatInvokeResponse> {
    reqwest::Client::new()
        .post(format!("{api_base}/api/agents/{agent_id}/heartbeat/invoke"))
        .send()
        .await
        .with_context(|| format!("failed to invoke heartbeat for paperclip agent {agent_id}"))?
        .error_for_status()
        .with_context(|| format!("heartbeat invoke request failed for paperclip agent {agent_id}"))?
        .json::<PaperclipHeartbeatInvokeResponse>()
        .await
        .context("failed to parse heartbeat invoke response")
}

async fn fetch_company_cost_summary(
    api_base: &str,
    company_id: &str,
) -> Result<PaperclipCostSummary> {
    reqwest::Client::new()
        .get(format!(
            "{api_base}/api/companies/{company_id}/costs/summary"
        ))
        .send()
        .await
        .with_context(|| format!("failed to fetch company cost summary for {company_id}"))?
        .error_for_status()
        .context("paperclip company cost summary request failed")?
        .json::<PaperclipCostSummary>()
        .await
        .context("failed to parse company cost summary")
}

async fn fetch_company_budget_overview(
    api_base: &str,
    company_id: &str,
) -> Result<PaperclipBudgetOverview> {
    reqwest::Client::new()
        .get(format!(
            "{api_base}/api/companies/{company_id}/budgets/overview"
        ))
        .send()
        .await
        .with_context(|| format!("failed to fetch company budget overview for {company_id}"))?
        .error_for_status()
        .context("paperclip company budget overview request failed")?
        .json::<PaperclipBudgetOverview>()
        .await
        .context("failed to parse company budget overview")
}

async fn fetch_pending_approval_count(api_base: &str, company_id: &str) -> Result<usize> {
    let approvals = reqwest::Client::new()
        .get(format!("{api_base}/api/companies/{company_id}/approvals"))
        .query(&[("status", "pending")])
        .send()
        .await
        .with_context(|| format!("failed to list pending approvals for {company_id}"))?
        .error_for_status()
        .context("paperclip pending approvals request failed")?
        .json::<Vec<serde_json::Value>>()
        .await
        .context("failed to parse pending approvals response")?;
    Ok(approvals.len())
}

fn render_heartbeat_invoke_result(response: &PaperclipHeartbeatInvokeResponse) -> String {
    match (response.id.as_deref(), response.status.as_deref()) {
        (_, Some("skipped")) => "skipped".to_string(),
        (Some(run_id), Some(status)) => format!("{status} ({run_id})"),
        (Some(run_id), None) => format!("accepted ({run_id})"),
        (None, Some(status)) => status.to_string(),
        (None, None) => "accepted".to_string(),
    }
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
            eprintln!(
                "warning: paperclip local-cli install failed for {}: {}",
                agent.slug,
                String::from_utf8_lossy(&output.stderr).trim()
            );
            continue;
        }
        let response: LocalCliInstall = match parse_json_stdout(
            &output.stdout,
            &format!("paperclip agent local-cli for {}", agent.slug),
        ) {
            Ok(response) => response,
            Err(error) => {
                eprintln!(
                    "warning: paperclip local-cli parse failed for {}: {}",
                    agent.slug, error
                );
                continue;
            }
        };
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
#[serde(rename_all = "camelCase")]
struct PaperclipCostSummary {
    spend_cents: u64,
    budget_cents: u64,
    utilization_percent: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaperclipBudgetOverview {
    paused_agent_count: u64,
    paused_project_count: u64,
    pending_approval_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct PaperclipProject {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
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
    slug: Option<String>,
    #[serde(rename = "adapterConfig", default)]
    adapter_config: Option<serde_json::Value>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaperclipIssueDocument {
    latest_revision_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PaperclipSecret {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaperclipHeartbeatInvokeResponse {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

struct SecretSyncSpec {
    env_key: &'static str,
    secret_name: &'static str,
    description: &'static str,
}

struct SyncedSecret {
    secret_id: String,
    name: String,
    value_hash: String,
}

struct AttachmentSpec {
    filename: String,
    path: PathBuf,
    content_type: &'static str,
}

struct AttachmentSyncResult {
    synced_count: usize,
    attachments_by_scope: BTreeMap<String, BTreeMap<String, PaperclipAttachment>>,
}

struct SyncIssueAttachmentsResult {
    synced_count: usize,
    attachments_by_filename: BTreeMap<String, PaperclipAttachment>,
}

const SECRET_SYNC_SPECS: [SecretSyncSpec; 2] = [
    SecretSyncSpec {
        env_key: "OPENAI_API_KEY",
        secret_name: "openai-api-key",
        description: "Fabro-synced OpenAI API key for generated local agents",
    },
    SecretSyncSpec {
        env_key: "ANTHROPIC_API_KEY",
        secret_name: "anthropic-api-key",
        description: "Fabro-synced Anthropic API key for generated local agents",
    },
];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaperclipIssue {
    id: String,
    title: String,
    #[serde(default)]
    description: Option<String>,
    status: String,
    #[serde(default)]
    parent_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaperclipWorkProduct {
    id: String,
    #[serde(default)]
    external_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaperclipAttachment {
    id: String,
    #[serde(default)]
    original_filename: Option<String>,
    #[serde(rename = "contentPath", default)]
    content_path: Option<String>,
    sha256: String,
}

fn derive_bootstrap_mission(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
    company_name: &str,
) -> Result<BootstrapMission> {
    let spec_title = latest_markdown_title(target_repo, "specs")
        .or_else(|| markdown_title_for_file(&target_repo.join("SPEC.md")))
        .unwrap_or_else(|| format!("{} specification", humanize(&blueprint.program.id)));
    let plan_title = preferred_plan_title(target_repo)
        .or_else(|| latest_markdown_title(target_repo, "plans"))
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
            company_name, blueprint.program.id, plan_title, spec_title, fronts
        ),
        goal_title: format!("Advance {}", company_name),
        goal_description: format!(
            "{} Keep work aligned with `{}` and `{}` and move the current frontier honestly across {}.",
            goal_sentence, spec_title, plan_title, fronts
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

fn preferred_plan_title(target_repo: &Path) -> Option<String> {
    let path = target_repo.join("plans");
    let mut entries = std::fs::read_dir(path)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .collect::<Vec<_>>();
    entries.sort();
    if entries.is_empty() {
        return None;
    }

    let mut docs = entries
        .into_iter()
        .filter_map(|path| {
            let body = std::fs::read_to_string(&path).ok()?;
            let title = markdown_title_for_file(&path)?;
            Some((path, title, body))
        })
        .collect::<Vec<_>>();
    let reference_counts = docs
        .iter()
        .flat_map(|(_, _, body)| markdown_plan_references(body))
        .fold(BTreeMap::<PathBuf, usize>::new(), |mut counts, path| {
            *counts.entry(path).or_insert(0) += 1;
            counts
        });

    docs.sort_by(|(left_path, left_title, _), (right_path, right_title, _)| {
        let left_score = preferred_plan_score(left_path, left_title, &reference_counts);
        let right_score = preferred_plan_score(right_path, right_title, &reference_counts);
        right_score
            .cmp(&left_score)
            .then_with(|| left_path.cmp(right_path))
    });
    docs.first().map(|(_, title, _)| title.clone())
}

fn preferred_plan_score(
    path: &Path,
    title: &str,
    reference_counts: &BTreeMap<PathBuf, usize>,
) -> i64 {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let title = title.to_ascii_lowercase();
    let mut score = reference_counts.get(path).copied().unwrap_or(0) as i64 * 100;
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
    score
}

fn markdown_plan_references(body: &str) -> Vec<PathBuf> {
    body.split('`')
        .filter_map(|chunk| {
            let trimmed = chunk.trim();
            if !trimmed.starts_with("plans/") || !trimmed.ends_with(".md") {
                return None;
            }
            Some(PathBuf::from(trimmed))
        })
        .collect()
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

fn ensure_target_repo_initialized(target_repo: &Path) -> Result<GitBootstrapResult> {
    ensure_target_repo_gitignore(target_repo)?;
    let git_marker = target_repo.join(".git");
    let initialized = if git_marker.exists() {
        false
    } else {
        let output = Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(target_repo)
            .output()
            .with_context(|| format!("failed to initialize git repo {}", target_repo.display()))?;
        if !output.status.success() {
            bail!(
                "git init failed for {}: {}",
                target_repo.display(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        true
    };

    let has_commit = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(target_repo)
        .output()
        .with_context(|| format!("failed to inspect git repo {}", target_repo.display()))?
        .status
        .success();
    if has_commit {
        return Ok(GitBootstrapResult {
            initialized,
            committed: false,
        });
    }

    let add = Command::new("git")
        .args(["add", "-A"])
        .current_dir(target_repo)
        .output()
        .with_context(|| format!("failed to stage bootstrap repo {}", target_repo.display()))?;
    if !add.status.success() {
        bail!(
            "git add failed for {}: {}",
            target_repo.display(),
            String::from_utf8_lossy(&add.stderr)
        );
    }

    let diff = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(target_repo)
        .output()
        .with_context(|| {
            format!(
                "failed to inspect staged bootstrap diff {}",
                target_repo.display()
            )
        })?;
    if diff.status.success() {
        return Ok(GitBootstrapResult {
            initialized,
            committed: false,
        });
    }

    let commit = Command::new("git")
        .args([
            "-c",
            "user.name=Fabro Bootstrap",
            "-c",
            "user.email=bootstrap@fabro.local",
            "commit",
            "-m",
            "chore(repo): bootstrap fabro workspace",
        ])
        .current_dir(target_repo)
        .output()
        .with_context(|| {
            format!(
                "failed to create initial git commit for {}",
                target_repo.display()
            )
        })?;
    if !commit.status.success() {
        bail!(
            "git commit failed for {}: {}",
            target_repo.display(),
            String::from_utf8_lossy(&commit.stderr)
        );
    }

    Ok(GitBootstrapResult {
        initialized,
        committed: true,
    })
}

fn ensure_target_repo_gitignore(target_repo: &Path) -> Result<()> {
    let path = target_repo.join(".gitignore");
    let mut lines = if path.exists() {
        std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?
            .lines()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut changed = false;
    for entry in BOOTSTRAP_GITIGNORE_LINES {
        if lines.iter().any(|line| line.trim() == *entry) {
            continue;
        }
        lines.push((*entry).to_string());
        changed = true;
    }
    if !changed {
        return Ok(());
    }
    let mut body = lines.join("\n");
    body.push('\n');
    std::fs::write(&path, body).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn prune_tracked_transient_paths(target_repo: &Path) -> Result<GitTransientCleanupResult> {
    if !target_repo.join(".git").exists() {
        return Ok(GitTransientCleanupResult { removed_paths: 0 });
    }

    let ls_files = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(target_repo)
        .output()
        .with_context(|| format!("failed to list tracked files in {}", target_repo.display()))?;
    if !ls_files.status.success() {
        bail!(
            "git ls-files failed for {}: {}",
            target_repo.display(),
            String::from_utf8_lossy(&ls_files.stderr)
        );
    }

    let tracked = ls_files
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| String::from_utf8(entry.to_vec()).ok())
        .filter(|path| {
            path == ".paperclip/.gitignore"
                || path.starts_with(".raspberry/")
                || path.starts_with(".paperclip/")
                || path.ends_with("/bootstrap-state.json")
        })
        .collect::<Vec<_>>();
    if tracked.is_empty() {
        return Ok(GitTransientCleanupResult { removed_paths: 0 });
    }

    let mut command = Command::new("git");
    command
        .args(["rm", "--cached", "--ignore-unmatch", "-r", "--"])
        .args(&tracked)
        .current_dir(target_repo);
    let output = command.output().with_context(|| {
        format!(
            "failed to prune tracked runtime files in {}",
            target_repo.display()
        )
    })?;
    if !output.status.success() {
        bail!(
            "git rm --cached failed for {}: {}",
            target_repo.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(GitTransientCleanupResult {
        removed_paths: tracked.len(),
    })
}

fn write_secrets_gitignore(directory: &Path) -> Result<()> {
    let gitignore = directory.join(".gitignore");
    if gitignore.exists() {
        return Ok(());
    }
    std::fs::write(gitignore, "*\n!.gitignore\n")?;
    Ok(())
}

fn goal_description_for_frontier(
    mission: &BootstrapMission,
    frontier: &FrontierSyncModel,
) -> String {
    format!(
        "{}\n\nCurrent Raspberry frontier:\n{}",
        mission.goal_description,
        render_frontier_summary(frontier),
    )
}

fn project_workspace_metadata(
    frontier: &FrontierSyncModel,
    target_repo: &Path,
) -> serde_json::Value {
    json!({
        "source": "fabro.paperclip",
        "program": frontier.program.clone(),
        "manifestPath": repo_relative_display(&frontier.manifest_path, target_repo),
        "statePath": repo_relative_display(&frontier.state_path, target_repo),
        "wakeCommand": frontier.wake_command.clone(),
        "routeCommand": frontier.route_command.clone(),
        "refreshCommand": frontier.refresh_command.clone(),
        "statusCommand": frontier.status_command.clone(),
        "summary": frontier.summary.clone(),
        "updatedAt": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    })
}

async fn ensure_company_goal(
    api_base: &str,
    company_id: &str,
    mission: &BootstrapMission,
    frontier: &FrontierSyncModel,
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
                "description": goal_description_for_frontier(mission, frontier),
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
            "description": goal_description_for_frontier(mission, frontier),
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

async fn resolve_existing_company_id(
    api_base: &str,
    company_name: &str,
    preferred_company_id: Option<&str>,
) -> Result<Option<String>> {
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
    if let Some(company_id) = preferred_company_id {
        if companies.iter().any(|company| company.id == company_id) {
            return Ok(Some(company_id.to_string()));
        }
    }
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
            match client
                .delete(format!("{api_base}/api/agents/{duplicate_id}"))
                .send()
                .await
            {
                Ok(resp) if !resp.status().is_success() => {
                    eprintln!(
                        "warning: failed to delete duplicate agent {duplicate_id}: {}",
                        resp.status()
                    );
                }
                Err(err) => {
                    eprintln!("warning: failed to delete duplicate agent {duplicate_id}: {err}");
                }
                _ => {}
            }
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
    if let Some(metadata_type) = desired_agent.metadata_type {
        return metadata.get("type").and_then(|value| value.as_str()) == Some(metadata_type);
    }
    metadata.get("unit").and_then(|value| value.as_str()) == desired_agent.unit.as_deref()
        && metadata.get("laneKey").and_then(|value| value.as_str())
            == desired_agent.lane_key.as_deref()
}

#[allow(clippy::too_many_arguments)] // Project sync requires repo, mission, frontier, and preferred ids together.
async fn ensure_company_project(
    api_base: &str,
    company_id: &str,
    goal_id: &str,
    mission: &BootstrapMission,
    target_repo: &Path,
    frontier: &FrontierSyncModel,
    preferred_project_id: Option<&str>,
    preferred_workspace_id: Option<&str>,
) -> Result<PaperclipProjectSync> {
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

    let workspace = ensure_project_workspace(
        api_base,
        &project.id,
        mission,
        target_repo,
        frontier,
        preferred_workspace_id,
    )
    .await?;
    let project = ensure_project_execution_workspace_policy(api_base, &project, &workspace).await?;
    Ok(PaperclipProjectSync { project, workspace })
}

async fn ensure_project_execution_workspace_policy(
    api_base: &str,
    project: &PaperclipProject,
    workspace: &PaperclipWorkspace,
) -> Result<PaperclipProject> {
    reqwest::Client::new()
        .patch(format!("{api_base}/api/projects/{}", project.id))
        .json(&json!({
            "executionWorkspacePolicy": {
                "enabled": true,
                "defaultMode": "shared_workspace",
                "allowIssueOverride": true,
                "defaultProjectWorkspaceId": workspace.id,
            }
        }))
        .send()
        .await
        .with_context(|| {
            format!(
                "failed to update project execution workspace policy {}",
                project.id
            )
        })?
        .error_for_status()
        .context("paperclip project execution workspace policy update failed")?
        .json::<PaperclipProject>()
        .await
        .context("failed to parse project after workspace policy update")
}

async fn ensure_project_workspace(
    api_base: &str,
    project_id: &str,
    mission: &BootstrapMission,
    target_repo: &Path,
    frontier: &FrontierSyncModel,
    preferred_workspace_id: Option<&str>,
) -> Result<PaperclipWorkspace> {
    let client = reqwest::Client::new();
    let desired_cwd = target_repo.display().to_string();
    let metadata = project_workspace_metadata(frontier, target_repo);
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

    let existing = preferred_workspace_id
        .and_then(|workspace_id| {
            workspaces
                .iter()
                .find(|workspace| workspace.id == workspace_id)
        })
        .or_else(|| {
            workspaces
                .iter()
                .find(|workspace| workspace.cwd.as_deref() == Some(desired_cwd.as_str()))
        });

    if let Some(existing) = existing {
        return client
            .patch(format!(
                "{api_base}/api/projects/{project_id}/workspaces/{}",
                existing.id
            ))
            .json(&json!({
                "name": mission.workspace_name,
                "cwd": desired_cwd,
                "sourceType": "local_path",
                "metadata": metadata,
                "isPrimary": true,
            }))
            .send()
            .await
            .context("failed to update paperclip workspace")?
            .error_for_status()
            .context("paperclip workspace update request failed")?
            .json::<PaperclipWorkspace>()
            .await
            .context("failed to parse updated project workspace");
    }

    client
        .post(format!("{api_base}/api/projects/{project_id}/workspaces"))
        .json(&json!({
            "name": mission.workspace_name,
            "cwd": desired_cwd,
            "sourceType": "local_path",
            "metadata": metadata,
            "isPrimary": true,
        }))
        .send()
        .await
        .context("failed to create paperclip workspace")?
        .error_for_status()
        .context("paperclip workspace create request failed")?
        .json::<PaperclipWorkspace>()
        .await
        .context("failed to parse created project workspace")
}

#[allow(clippy::too_many_arguments)] // Coordination issue sync spans company, project, frontier, agents, and prior state.
async fn sync_coordination_issues(
    api_base: &str,
    company_id: &str,
    goal_id: &str,
    project_id: &str,
    project_workspace_id: &str,
    frontier: &FrontierSyncModel,
    plan_dashboard: Option<&PlanDashboardModel>,
    bundle_agents: &[BundleAgent],
    imported_agents: &[PaperclipImportAgent],
    existing_state: Option<&serde_json::Value>,
) -> Result<BTreeMap<String, String>> {
    let agent_ids = imported_agent_ids(imported_agents);
    let orchestrator_agent_id = agent_ids.get("raspberry-orchestrator").cloned();
    let materialized_lane_keys = bundle_agents
        .iter()
        .filter_map(|agent| agent.lane_key.clone())
        .collect::<BTreeSet<_>>();
    let lane_agent_ids = bundle_agents
        .iter()
        .filter_map(|agent| {
            agent.lane_key.as_ref().and_then(|lane_key| {
                agent_ids
                    .get(&agent.slug)
                    .map(|agent_id| (lane_key.clone(), agent_id.clone()))
            })
        })
        .collect::<BTreeMap<_, _>>();
    let plan_agent_ids: BTreeMap<String, String> = bundle_agents
        .iter()
        .filter_map(|agent| {
            agent.plan_id.as_ref().and_then(|plan_id| {
                agent_ids
                    .get(&agent.slug)
                    .map(|agent_id| (plan_id.clone(), agent_id.clone()))
            })
        })
        .collect();
    let mut synced = BTreeMap::new();
    let mut existing =
        collect_existing_sync_issues(api_base, company_id, project_id, existing_state).await?;

    let root_key = frontier_root_sync_key(&frontier.program);
    let root_existing = existing.remove(&root_key);
    let root_issue = upsert_sync_issue(
        api_base,
        company_id,
        goal_id,
        project_id,
        root_existing.as_ref(),
        &desired_root_issue(
            frontier,
            orchestrator_agent_id.clone(),
            project_workspace_id,
        ),
    )
    .await?;
    synced.insert(root_key, root_issue.id.clone());

    if plan_dashboard.is_none() {
        for entry in &frontier.entries {
            if !materialized_lane_keys.contains(&entry.lane_key) {
                continue;
            }
            let existing_issue = existing.remove(&entry.sync_key);
            let desired = desired_lane_issue(
                entry,
                &root_issue.id,
                orchestrator_agent_id.clone(),
                lane_agent_ids.get(&entry.lane_key).cloned(),
                existing_issue.is_some(),
                project_workspace_id,
            );
            let Some(desired) = desired else {
                continue;
            };
            let issue = upsert_sync_issue(
                api_base,
                company_id,
                goal_id,
                project_id,
                existing_issue.as_ref(),
                &desired,
            )
            .await?;
            synced.insert(entry.sync_key.clone(), issue.id.clone());
        }
    }

    // Plan-root-keyed issues
    if let Some(dashboard) = plan_dashboard {
        for plan in &dashboard.plans {
            if !should_materialize_plan_agent(plan) {
                continue;
            }
            let plan_key = plan_root_sync_key(&dashboard.program, &plan.plan_id);
            let existing_plan_issue = existing.remove(&plan_key);
            let desired = desired_plan_root_issue(
                &dashboard.program,
                plan,
                &root_issue.id,
                plan_agent_ids.get(&plan.plan_id).cloned(),
                project_workspace_id,
            );
            let plan_issue = upsert_sync_issue(
                api_base,
                company_id,
                goal_id,
                project_id,
                existing_plan_issue.as_ref(),
                &desired,
            )
            .await?;
            synced.insert(plan_key, plan_issue.id.clone());
        }
    }

    for (sync_key, issue) in existing {
        let desired = desired_archived_issue(sync_key.clone(), &issue);
        let archived = upsert_sync_issue(
            api_base,
            company_id,
            goal_id,
            project_id,
            Some(&issue),
            &desired,
        )
        .await?;
        synced.insert(sync_key, archived.id);
    }

    Ok(synced)
}

async fn sync_coordination_documents(
    api_base: &str,
    frontier: &FrontierSyncModel,
    plan_matrix: Option<&PlanMatrix>,
    plan_dashboard: Option<&PlanDashboardModel>,
    synced_issue_ids: &BTreeMap<String, String>,
) -> Result<usize> {
    let mut synced = 0usize;

    if let Some(issue_id) = synced_issue_ids.get(&frontier_root_sync_key(&frontier.program)) {
        upsert_issue_plan_document(
            api_base,
            issue_id,
            "Frontier plan",
            &render_root_issue_plan_document(frontier, plan_matrix),
        )
        .await?;
        synced += 1;
    }

    if plan_dashboard.is_none() {
        for entry in &frontier.entries {
            if !synced_issue_ids.contains_key(&entry.sync_key) {
                continue;
            }
            let Some(issue_id) = synced_issue_ids.get(&entry.sync_key) else {
                continue;
            };
            upsert_issue_plan_document(
                api_base,
                issue_id,
                &format!("{} plan", entry.lane_title),
                &render_lane_issue_plan_document(entry),
            )
            .await?;
            synced += 1;
        }
    }

    if let Some(dashboard) = plan_dashboard {
        for plan in &dashboard.plans {
            if !should_materialize_plan_agent(plan) {
                continue;
            }
            let plan_key = plan_root_sync_key(&dashboard.program, &plan.plan_id);
            let Some(issue_id) = synced_issue_ids.get(&plan_key) else {
                continue;
            };
            upsert_issue_plan_document(
                api_base,
                issue_id,
                &format!("{} plan", plan.title),
                &render_plan_root_issue_plan_document(plan),
            )
            .await?;
            synced += 1;
        }
    }

    Ok(synced)
}

fn render_plan_root_issue_plan_document(plan: &PlanDashboardEntry) -> String {
    let mut body = format!("# {}\n\n## Status\n\n- Status: {}\n- Current stage: {}\n- Current run: {}\n- Risk: {}\n- Next move: {}\n- Mapping: {}\n- Category: {}\n- Path: `{}`\n",
        plan.title, plan.status, plan.current_stage.as_deref().unwrap_or("none"), plan.current_run_id.as_deref().unwrap_or("none"), plan.risk, plan.next_move,
        plan.mapping_source, plan.category, plan.path,
    );
    if !plan.children.is_empty() {
        body.push_str(
            "\n## Children\n\n| Child | Archetype | Status | Stage | Run | Next Move | Surfaces |\n|---|---|---|---|---|---|---|\n",
        );
        for child in &plan.children {
            body.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                child.title,
                child.archetype.as_deref().unwrap_or("implement"),
                child.status,
                child.current_stage.as_deref().unwrap_or("-"),
                child.current_run_id.as_deref().unwrap_or("-"),
                child.next_operator_move,
                if child.owned_surfaces.is_empty() {
                    "none".to_string()
                } else {
                    child.owned_surfaces.join(", ")
                },
            ));
        }
    }
    body
}

async fn sync_coordination_comments(
    api_base: &str,
    frontier: &FrontierSyncModel,
    plan_dashboard: Option<&PlanDashboardModel>,
    synced_issue_ids: &BTreeMap<String, String>,
    existing_state: Option<&serde_json::Value>,
) -> Result<usize> {
    let previous_snapshots = synced_snapshots_from_state(existing_state);
    let current_snapshots = frontier_snapshots(frontier);
    let mut comment_count = 0usize;

    let root_key = frontier_root_sync_key(&frontier.program);
    if let (Some(issue_id), Some(current_snapshot)) = (
        synced_issue_ids.get(&root_key),
        current_snapshots.get(&root_key),
    ) {
        let previous_snapshot = previous_snapshots.get(&root_key);
        if snapshot_changed(previous_snapshot, current_snapshot) {
            post_issue_comment(
                api_base,
                issue_id,
                &render_root_transition_comment(frontier, previous_snapshot, current_snapshot),
            )
            .await?;
            comment_count += 1;
        }
    }

    if plan_dashboard.is_none() {
        for entry in &frontier.entries {
            if !synced_issue_ids.contains_key(&entry.sync_key) {
                continue;
            }
            let Some(issue_id) = synced_issue_ids.get(&entry.sync_key) else {
                continue;
            };
            let Some(current_snapshot) = current_snapshots.get(&entry.sync_key) else {
                continue;
            };
            let previous_snapshot = previous_snapshots.get(&entry.sync_key);
            if snapshot_changed(previous_snapshot, current_snapshot) {
                post_issue_comment(
                    api_base,
                    issue_id,
                    &render_lane_transition_comment(entry, previous_snapshot, current_snapshot),
                )
                .await?;
                comment_count += 1;
            }
        }
    }

    // Plan status transition comments
    if let Some(dashboard) = plan_dashboard {
        let previous_plan_snapshots = synced_plan_snapshots_from_state(existing_state);
        let current_plan_snapshots = plan_snapshots(dashboard);
        for plan in &dashboard.plans {
            if !should_materialize_plan_agent(plan) {
                continue;
            }
            let plan_key = plan_root_sync_key(&dashboard.program, &plan.plan_id);
            let Some(issue_id) = synced_issue_ids.get(&plan_key) else {
                continue;
            };
            let Some(current) = current_plan_snapshots.get(&plan_key) else {
                continue;
            };
            let previous = previous_plan_snapshots.get(&plan_key);
            if snapshot_changed(previous, current) {
                post_issue_comment(
                    api_base,
                    issue_id,
                    &render_plan_transition_comment(plan, previous, current),
                )
                .await?;
                comment_count += 1;
            }
        }
    }

    Ok(comment_count)
}

fn plan_snapshots(dashboard: &PlanDashboardModel) -> BTreeMap<String, serde_json::Value> {
    let mut snapshots = BTreeMap::new();
    for plan in &dashboard.plans {
        if !should_materialize_plan_agent(plan) {
            continue;
        }
        let key = plan_root_sync_key(&dashboard.program, &plan.plan_id);
        snapshots.insert(
            key,
            json!({
                "status": plan.status,
                "risk": plan.risk,
                "childCount": plan.children.len(),
                "nextMove": plan.next_move,
            }),
        );
    }
    snapshots
}

fn plan_snapshots_json(dashboard: &PlanDashboardModel) -> serde_json::Value {
    let mut values = serde_json::Map::new();
    for (key, value) in plan_snapshots(dashboard) {
        values.insert(key, value);
    }
    serde_json::Value::Object(values)
}

fn synced_plan_snapshots_from_state(
    existing_state: Option<&serde_json::Value>,
) -> BTreeMap<String, serde_json::Value> {
    existing_state
        .and_then(|state| state.get("planSync"))
        .and_then(|value| value.get("snapshots"))
        .and_then(|value| value.as_object())
        .map(|values| {
            values
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default()
}

fn render_plan_transition_comment(
    plan: &PlanDashboardEntry,
    previous: Option<&serde_json::Value>,
    current: &serde_json::Value,
) -> String {
    let mut body = format!("## Plan Sync Update\n\n- plan: `{}`\n", plan.plan_id);
    body.push_str(&format!(
        "- previous snapshot: `{}`\n",
        compact_snapshot(previous)
    ));
    body.push_str(&format!(
        "- current snapshot: `{}`\n- next move: {}\n",
        compact_snapshot(Some(current)),
        plan.next_move,
    ));
    body
}

async fn sync_coordination_work_products(
    api_base: &str,
    frontier: &FrontierSyncModel,
    plan_dashboard: Option<&PlanDashboardModel>,
    synced_issue_ids: &BTreeMap<String, String>,
    attachments_by_scope: &BTreeMap<String, BTreeMap<String, PaperclipAttachment>>,
) -> Result<usize> {
    let mut synced = 0usize;
    let root_key = frontier_root_sync_key(&frontier.program);

    if let Some(issue_id) = synced_issue_ids.get(&root_key) {
        synced += sync_issue_work_products(
            api_base,
            issue_id,
            &desired_root_work_products(frontier, attachments_by_scope.get(&root_key), api_base),
        )
        .await?;
    }

    for entry in &frontier.entries {
        let Some(issue_id) = synced_issue_ids.get(&entry.sync_key) else {
            continue;
        };
        synced += sync_issue_work_products(
            api_base,
            issue_id,
            &desired_lane_work_products(entry, attachments_by_scope.get(&entry.sync_key), api_base),
        )
        .await?;
    }

    if let Some(dashboard) = plan_dashboard {
        for plan in &dashboard.plans {
            if !should_materialize_plan_agent(plan) {
                continue;
            }
            let plan_key = plan_root_sync_key(&dashboard.program, &plan.plan_id);
            let Some(issue_id) = synced_issue_ids.get(&plan_key) else {
                continue;
            };
            synced += sync_issue_work_products(
                api_base,
                issue_id,
                &desired_plan_root_work_products(&dashboard.program, plan),
            )
            .await?;
        }
    }

    Ok(synced)
}

fn desired_plan_root_work_products(
    program_id: &str,
    plan: &PlanDashboardEntry,
) -> Vec<DesiredWorkProduct> {
    let plan_key = plan_root_sync_key(program_id, &plan.plan_id);
    let status = if plan.status.contains("complete") || plan.status == "reviewed" {
        "ready_for_review"
    } else if plan.status.contains("running") {
        "active"
    } else if plan.status.contains("failed") {
        "failed"
    } else {
        "draft"
    };
    let health = if plan.status.contains("failed") {
        "unhealthy"
    } else {
        "healthy"
    };

    vec![DesiredWorkProduct {
        external_id: format!("{plan_key}::plan_file"),
        title: format!("{} (plan file)", plan.title),
        status: status.to_string(),
        health_status: health.to_string(),
        is_primary: true,
        url: None,
        summary: Some(format!(
            "Status: {} | Next: {}",
            plan.status, plan.next_move
        )),
        metadata: json!({
            "source": "fabro.paperclip.plan",
            "planId": plan.plan_id,
            "path": plan.path,
            "category": plan.category,
        }),
    }]
}

async fn sync_coordination_attachments(
    api_base: &str,
    company_id: &str,
    _target_repo: &Path,
    frontier: &FrontierSyncModel,
    synced_issue_ids: &BTreeMap<String, String>,
) -> Result<AttachmentSyncResult> {
    let mut attachments_by_scope = BTreeMap::new();
    let mut synced_count = 0usize;

    let root_key = frontier_root_sync_key(&frontier.program);
    if let Some(issue_id) = synced_issue_ids.get(&root_key) {
        let synced = sync_issue_attachments(
            api_base,
            company_id,
            issue_id,
            &desired_root_attachment_specs(frontier),
        )
        .await?;
        synced_count += synced.synced_count;
        attachments_by_scope.insert(root_key, synced.attachments_by_filename);
    }

    if !synced_issue_ids.keys().any(|key| key.starts_with("plan/")) {
        for entry in &frontier.entries {
            if !synced_issue_ids.contains_key(&entry.sync_key) {
                continue;
            }
            let Some(issue_id) = synced_issue_ids.get(&entry.sync_key) else {
                continue;
            };
            let synced = sync_issue_attachments(
                api_base,
                company_id,
                issue_id,
                &desired_lane_attachment_specs(_target_repo, entry),
            )
            .await?;
            synced_count += synced.synced_count;
            attachments_by_scope.insert(entry.sync_key.clone(), synced.attachments_by_filename);
        }
    }

    Ok(AttachmentSyncResult {
        synced_count,
        attachments_by_scope,
    })
}

fn desired_root_attachment_specs(frontier: &FrontierSyncModel) -> Vec<AttachmentSpec> {
    vec![
        AttachmentSpec {
            filename: format!("{}-program-manifest.yaml", frontier.program),
            path: frontier.manifest_path.clone(),
            content_type: "text/plain",
        },
        AttachmentSpec {
            filename: format!("{}-program-state.json", frontier.program),
            path: frontier.state_path.clone(),
            content_type: "application/json",
        },
    ]
}

#[allow(dead_code)] // Reserved for optional lane-level attachment sync if operators re-enable it.
fn desired_lane_attachment_specs(
    target_repo: &Path,
    entry: &FrontierSyncEntry,
) -> Vec<AttachmentSpec> {
    entry
        .artifact_paths
        .iter()
        .map(|artifact| AttachmentSpec {
            filename: lane_attachment_filename(entry, artifact),
            path: target_repo.join(artifact),
            content_type: content_type_for_path(artifact),
        })
        .collect()
}

async fn sync_issue_attachments(
    api_base: &str,
    company_id: &str,
    issue_id: &str,
    desired: &[AttachmentSpec],
) -> Result<SyncIssueAttachmentsResult> {
    if desired.is_empty() {
        return Ok(SyncIssueAttachmentsResult {
            synced_count: 0,
            attachments_by_filename: BTreeMap::new(),
        });
    }

    let existing = list_issue_attachments(api_base, issue_id).await?;
    let managed = existing
        .into_iter()
        .filter_map(|attachment| {
            let filename = attachment.original_filename.clone()?;
            Some((filename, attachment))
        })
        .collect::<BTreeMap<_, _>>();
    let mut attachments_by_filename = managed.clone();
    let mut synced = 0usize;

    for attachment in desired {
        if !attachment.path.exists() {
            continue;
        }
        let bytes = std::fs::read(&attachment.path)
            .with_context(|| format!("failed to read {}", attachment.path.display()))?;
        let file_hash = sha256_bytes_hex(&bytes);
        if let Some(existing_attachment) = managed.get(&attachment.filename) {
            if existing_attachment.sha256 == file_hash {
                attachments_by_filename
                    .insert(attachment.filename.clone(), existing_attachment.clone());
                synced += 1;
                continue;
            }
            delete_issue_attachment(api_base, &existing_attachment.id).await?;
        }
        let uploaded = upload_issue_attachment(
            api_base,
            company_id,
            issue_id,
            &attachment.filename,
            attachment.content_type,
            bytes,
        )
        .await?;
        attachments_by_filename.insert(attachment.filename.clone(), uploaded);
        synced += 1;
    }

    Ok(SyncIssueAttachmentsResult {
        synced_count: synced,
        attachments_by_filename,
    })
}

fn desired_root_issue(
    frontier: &FrontierSyncModel,
    assignee_agent_id: Option<String>,
    project_workspace_id: &str,
) -> DesiredIssue {
    let status = root_issue_status(frontier);

    DesiredIssue {
        title: format!("Raspberry frontier: {}", humanize(&frontier.program)),
        description: with_sync_marker(
            format!(
                "This issue tracks the live Raspberry frontier for `{}`.\n\nSummary:\n{}\n\nLane sets:\n{}\n\nExecution route:\n- Wake the orchestrator with `{}`.\n- Run `{}` as the direct repo-local fallback.\n- Refresh Paperclip with `{}`.\n- Inspect local status with `{}`.\n- See the synced `plan` document for the current operator playbook.\n",
                frontier.program,
                render_frontier_summary(frontier),
                render_frontier_lane_sets(frontier),
                frontier.wake_command,
                frontier.route_command,
                frontier.refresh_command,
                frontier.status_command,
            ),
            &frontier_root_sync_key(&frontier.program),
        ),
        status: normalize_issue_status(status, assignee_agent_id.as_ref()),
        priority: "high".to_string(),
        parent_id: None,
        assignee_agent_id,
        project_workspace_id: Some(project_workspace_id.to_string()),
        assignee_adapter_overrides: Some(json!({
            "useProjectWorkspace": true,
        })),
        execution_workspace_preference: Some("reuse_existing".to_string()),
    }
}

#[allow(dead_code)] // Reserved for optional lane-level issue sync if operators re-enable it.
fn desired_lane_issue(
    entry: &FrontierSyncEntry,
    root_issue_id: &str,
    fallback_assignee_id: Option<String>,
    direct_assignee_id: Option<String>,
    existing_issue: bool,
    project_workspace_id: &str,
) -> Option<DesiredIssue> {
    if entry.status == LaneExecutionStatus::Complete && !existing_issue {
        return None;
    }

    let assignee_agent_id = direct_assignee_id.or(fallback_assignee_id);
    let status = match entry.status {
        LaneExecutionStatus::Ready => "todo",
        LaneExecutionStatus::Running => "in_progress",
        LaneExecutionStatus::Blocked | LaneExecutionStatus::Failed => "blocked",
        LaneExecutionStatus::Complete => "done",
    };
    let priority = match entry.status {
        LaneExecutionStatus::Failed => "high",
        LaneExecutionStatus::Blocked => "high",
        LaneExecutionStatus::Running => "medium",
        LaneExecutionStatus::Ready => "medium",
        LaneExecutionStatus::Complete => "low",
    };

    Some(DesiredIssue {
        title: format!(
            "Raspberry frontier: {} / {}",
            entry.unit_title, entry.lane_title,
        ),
        description: with_sync_marker(render_lane_issue_description(entry), &entry.sync_key),
        status: normalize_issue_status(status, assignee_agent_id.as_ref()),
        priority: priority.to_string(),
        parent_id: Some(root_issue_id.to_string()),
        assignee_agent_id,
        project_workspace_id: Some(project_workspace_id.to_string()),
        assignee_adapter_overrides: Some(json!({
            "useProjectWorkspace": true,
        })),
        execution_workspace_preference: Some("reuse_existing".to_string()),
    })
}

fn desired_archived_issue(sync_key: String, issue: &PaperclipIssue) -> DesiredIssue {
    DesiredIssue {
        title: issue.title.clone(),
        description: with_sync_marker(
            format!(
                "This synchronized frontier issue no longer maps to an active Raspberry frontier.\n\nPrevious status: `{}`\n\nThe next Paperclip refresh will reopen it if the same deterministic frontier key returns.\n",
                issue.status,
            ),
            &sync_key,
        ),
        status: "cancelled".to_string(),
        priority: "low".to_string(),
        parent_id: issue.parent_id.clone(),
        assignee_agent_id: None,
        project_workspace_id: None,
        assignee_adapter_overrides: None,
        execution_workspace_preference: None,
    }
}

fn desired_plan_root_issue(
    program_id: &str,
    plan: &PlanDashboardEntry,
    root_issue_id: &str,
    assignee_agent_id: Option<String>,
    project_workspace_id: &str,
) -> DesiredIssue {
    let status = plan_status_to_issue_status(&plan.status);
    let priority = plan_status_to_priority(&plan.status);
    let sync_key = plan_root_sync_key(program_id, &plan.plan_id);

    let children_summary = render_plan_children_summary(&plan.children);

    DesiredIssue {
        title: format!("Plan: {}", plan.title),
        description: with_sync_marker(
            format!(
                "Plan `{plan_id}` ({category})\n\n- Status: {status}\n- Current stage: {current_stage}\n- Current run: {current_run}\n- Risk: {risk}\n- Next: {next}\n- Mapping: {mapping}\n- Path: `{path}`\n\n## Children\n\n{children}\n",
                plan_id = plan.plan_id,
                category = plan.category,
                status = plan.status,
                current_stage = plan.current_stage.as_deref().unwrap_or("none"),
                current_run = plan.current_run_id.as_deref().unwrap_or("none"),
                risk = plan.risk,
                next = plan.next_move,
                mapping = plan.mapping_source,
                path = plan.path,
                children = children_summary,
            ),
            &sync_key,
        ),
        status: normalize_issue_status(status, assignee_agent_id.as_ref()),
        priority: priority.to_string(),
        parent_id: Some(root_issue_id.to_string()),
        assignee_agent_id,
        project_workspace_id: Some(project_workspace_id.to_string()),
        assignee_adapter_overrides: Some(json!({
            "useProjectWorkspace": true,
        })),
        execution_workspace_preference: Some("reuse_existing".to_string()),
    }
}

#[allow(dead_code)] // Reserved for optional child-issue sync if plan-root decomposition is reintroduced.
fn desired_plan_child_issue(
    plan: &PlanDashboardEntry,
    child: &PlanDashboardChild,
    plan_issue_id: &str,
    fallback_assignee_id: Option<String>,
    project_workspace_id: &str,
) -> DesiredIssue {
    let surfaces = if child.owned_surfaces.is_empty() {
        "none".to_string()
    } else {
        child.owned_surfaces.join(", ")
    };

    DesiredIssue {
        title: format!("Plan: {} / {}", plan.title, child.title),
        description: with_sync_marker(
            format!(
                "Child `{child_id}` of plan `{plan_id}`\n\n- Archetype: {archetype}\n- Review profile: {profile}\n- Surfaces: {surfaces}\n",
                child_id = child.child_id,
                plan_id = plan.plan_id,
                archetype = child.archetype.as_deref().unwrap_or("implement"),
                profile = child.review_profile.as_deref().unwrap_or("standard"),
                surfaces = surfaces,
            ),
            &child.sync_key,
        ),
        status: "todo".to_string(),
        priority: "medium".to_string(),
        parent_id: Some(plan_issue_id.to_string()),
        assignee_agent_id: fallback_assignee_id,
        project_workspace_id: Some(project_workspace_id.to_string()),
        assignee_adapter_overrides: Some(json!({
            "useProjectWorkspace": true,
        })),
        execution_workspace_preference: Some("reuse_existing".to_string()),
    }
}

fn plan_status_to_issue_status(status: &str) -> &'static str {
    if status.contains("failed") || status.contains("blocked") {
        "blocked"
    } else if status.contains("running") {
        "in_progress"
    } else if status.contains("complete") || status == "reviewed" {
        "done"
    } else {
        "todo"
    }
}

fn plan_status_to_priority(status: &str) -> &'static str {
    if status.contains("failed") {
        "critical"
    } else if status.contains("blocked") || status == "unmodeled" {
        "high"
    } else {
        "medium"
    }
}

fn render_root_issue_plan_document(
    frontier: &FrontierSyncModel,
    plan_matrix: Option<&PlanMatrix>,
) -> String {
    let mut body = String::from("# Raspberry Plans\n");
    if let Some(plan_matrix) = plan_matrix {
        body.push_str(&format!(
            "\n## Plan Status Summary\n\n{}\n",
            render_plan_matrix_summary(plan_matrix)
        ));
        if let Some(section) =
            render_plan_attention_section(plan_matrix, "Plans Needing Attention", |row| {
                row.current_status.contains("failed")
                    || row.current_status.contains("blocked")
                    || row.current_status == "unmodeled"
            })
        {
            body.push_str(&format!("\n{}\n", section));
        }
        body.push_str(&format!(
            "\n## Plan Matrix\n\n```\n{}\n```\n",
            raspberry_supervisor::render_plan_matrix(plan_matrix)
        ));
    }
    body.push_str(&format!(
        "\n## Frontier Summary\n\n{}\n\n## Lane Sets\n\n{}\n\n## Operator Loop\n\n1. Check current state with `{}`.\n2. Wake the orchestrator with `{}`.\n3. Use `{}` only as the direct repo-local fallback.\n4. Refresh Paperclip with `{}` after the frontier moves.\n",
        render_frontier_summary(frontier),
        render_frontier_lane_sets(frontier),
        frontier.status_command,
        frontier.wake_command,
        frontier.route_command,
        frontier.refresh_command,
    ));
    body
}

#[allow(dead_code)] // Reserved for optional lane-level issue sync if operators re-enable it.
fn render_lane_issue_plan_document(entry: &FrontierSyncEntry) -> String {
    format!(
        "# {}\n\n## Current Frontier State\n\n{}\n\n## Next Operator Move\n\n- {}\n\n## Commands\n\n- Wake the orchestrator: `{}`\n- Direct repo-local fallback: `{}`\n- Refresh Paperclip: `{}`\n\n## Artifacts\n\n{}\n\n## Dependencies\n\n{}\n",
        entry.lane_title,
        render_frontier_entry(Some(entry)),
        entry.next_operator_move,
        entry.wake_command,
        entry.route_command,
        entry.refresh_command,
        bullet_block(&entry.artifact_statuses, "No curated artifacts recorded."),
        bullet_block(&entry.dependency_keys, "No explicit dependencies."),
    )
}

async fn upsert_issue_plan_document(
    api_base: &str,
    issue_id: &str,
    title: &str,
    body: &str,
) -> Result<()> {
    let existing = get_issue_document(api_base, issue_id, "plan").await?;
    let response = reqwest::Client::new()
        .put(format!("{api_base}/api/issues/{issue_id}/documents/plan"))
        .json(&json!({
            "title": title,
            "format": "markdown",
            "body": body,
            "changeSummary": "Synced from Raspberry frontier",
            "baseRevisionId": existing.as_ref().and_then(|doc| doc.latest_revision_id.as_ref()),
        }))
        .send()
        .await
        .with_context(|| format!("failed to upsert plan document for issue {issue_id}"))?;
    if response.status() == reqwest::StatusCode::CONFLICT {
        let latest = get_issue_document(api_base, issue_id, "plan").await?;
        reqwest::Client::new()
            .put(format!("{api_base}/api/issues/{issue_id}/documents/plan"))
            .json(&json!({
                "title": title,
                "format": "markdown",
                "body": body,
                "changeSummary": "Synced from Raspberry frontier",
                "baseRevisionId": latest.as_ref().and_then(|doc| doc.latest_revision_id.as_ref()),
            }))
            .send()
            .await
            .with_context(|| format!("failed to retry plan document upsert for issue {issue_id}"))?
            .error_for_status()
            .with_context(|| format!("plan document retry request failed for issue {issue_id}"))?;
        return Ok(());
    }
    response
        .error_for_status()
        .with_context(|| format!("plan document request failed for issue {issue_id}"))?;
    Ok(())
}

async fn get_issue_document(
    api_base: &str,
    issue_id: &str,
    key: &str,
) -> Result<Option<PaperclipIssueDocument>> {
    let response = reqwest::Client::new()
        .get(format!("{api_base}/api/issues/{issue_id}/documents/{key}"))
        .send()
        .await
        .with_context(|| format!("failed to fetch issue document `{key}` for issue {issue_id}"))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    response
        .error_for_status()
        .with_context(|| format!("issue document get request failed for issue {issue_id}"))?
        .json::<PaperclipIssueDocument>()
        .await
        .map(Some)
        .context("failed to parse issue document response")
}

async fn post_issue_comment(api_base: &str, issue_id: &str, body: &str) -> Result<()> {
    reqwest::Client::new()
        .post(format!("{api_base}/api/issues/{issue_id}/comments"))
        .json(&json!({
            "body": body,
        }))
        .send()
        .await
        .with_context(|| format!("failed to post paperclip comment for issue {issue_id}"))?
        .error_for_status()
        .with_context(|| format!("paperclip issue comment request failed for issue {issue_id}"))?;
    Ok(())
}

async fn sync_issue_work_products(
    api_base: &str,
    issue_id: &str,
    desired_products: &[DesiredWorkProduct],
) -> Result<usize> {
    let existing_products = list_issue_work_products(api_base, issue_id).await?;
    let existing_by_external_id = existing_products
        .iter()
        .filter_map(|product| {
            product
                .external_id
                .as_ref()
                .map(|external_id| (external_id.clone(), product.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let desired_ids = desired_products
        .iter()
        .map(|product| product.external_id.clone())
        .collect::<BTreeSet<_>>();
    let mut synced = 0usize;

    for desired in desired_products {
        if let Some(existing) = existing_by_external_id.get(&desired.external_id) {
            update_issue_work_product(api_base, &existing.id, desired).await?;
        } else {
            create_issue_work_product(api_base, issue_id, desired).await?;
        }
        synced += 1;
    }

    for existing in existing_products {
        let Some(external_id) = existing.external_id.as_ref() else {
            continue;
        };
        if !external_id.starts_with("fabro.paperclip::") {
            continue;
        }
        if desired_ids.contains(external_id) {
            continue;
        }
        delete_issue_work_product(api_base, &existing.id).await?;
    }

    Ok(synced)
}

async fn list_issue_work_products(
    api_base: &str,
    issue_id: &str,
) -> Result<Vec<PaperclipWorkProduct>> {
    reqwest::Client::new()
        .get(format!("{api_base}/api/issues/{issue_id}/work-products"))
        .send()
        .await
        .with_context(|| format!("failed to list work products for issue {issue_id}"))?
        .error_for_status()
        .with_context(|| format!("work product list request failed for issue {issue_id}"))?
        .json::<Vec<PaperclipWorkProduct>>()
        .await
        .context("failed to parse work product list response")
}

async fn create_issue_work_product(
    api_base: &str,
    issue_id: &str,
    desired: &DesiredWorkProduct,
) -> Result<PaperclipWorkProduct> {
    reqwest::Client::new()
        .post(format!("{api_base}/api/issues/{issue_id}/work-products"))
        .json(&json!({
            "type": "artifact",
            "provider": "paperclip",
            "externalId": desired.external_id,
            "title": desired.title,
            "url": desired.url,
            "status": desired.status,
            "reviewState": "none",
            "isPrimary": desired.is_primary,
            "healthStatus": desired.health_status,
            "summary": desired.summary,
            "metadata": desired.metadata,
        }))
        .send()
        .await
        .with_context(|| format!("failed to create work product for issue {issue_id}"))?
        .error_for_status()
        .with_context(|| format!("work product create request failed for issue {issue_id}"))?
        .json::<PaperclipWorkProduct>()
        .await
        .context("failed to parse created work product")
}

async fn update_issue_work_product(
    api_base: &str,
    work_product_id: &str,
    desired: &DesiredWorkProduct,
) -> Result<PaperclipWorkProduct> {
    reqwest::Client::new()
        .patch(format!("{api_base}/api/work-products/{work_product_id}"))
        .json(&json!({
            "externalId": desired.external_id,
            "title": desired.title,
            "url": desired.url,
            "status": desired.status,
            "isPrimary": desired.is_primary,
            "healthStatus": desired.health_status,
            "summary": desired.summary,
            "metadata": desired.metadata,
        }))
        .send()
        .await
        .with_context(|| format!("failed to update work product {work_product_id}"))?
        .error_for_status()
        .with_context(|| format!("work product patch request failed for {work_product_id}"))?
        .json::<PaperclipWorkProduct>()
        .await
        .context("failed to parse updated work product")
}

async fn delete_issue_work_product(api_base: &str, work_product_id: &str) -> Result<()> {
    reqwest::Client::new()
        .delete(format!("{api_base}/api/work-products/{work_product_id}"))
        .send()
        .await
        .with_context(|| format!("failed to delete work product {work_product_id}"))?
        .error_for_status()
        .with_context(|| format!("work product delete request failed for {work_product_id}"))?;
    Ok(())
}

async fn list_issue_attachments(
    api_base: &str,
    issue_id: &str,
) -> Result<Vec<PaperclipAttachment>> {
    reqwest::Client::new()
        .get(format!("{api_base}/api/issues/{issue_id}/attachments"))
        .send()
        .await
        .with_context(|| format!("failed to list attachments for issue {issue_id}"))?
        .error_for_status()
        .with_context(|| format!("attachment list request failed for issue {issue_id}"))?
        .json::<Vec<PaperclipAttachment>>()
        .await
        .context("failed to parse attachment list response")
}

async fn upload_issue_attachment(
    api_base: &str,
    company_id: &str,
    issue_id: &str,
    filename: &str,
    content_type: &str,
    bytes: Vec<u8>,
) -> Result<PaperclipAttachment> {
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(filename.to_string())
        .mime_str(content_type)
        .with_context(|| format!("invalid content type {content_type}"))?;
    let form = reqwest::multipart::Form::new().part("file", part);

    reqwest::Client::new()
        .post(format!(
            "{api_base}/api/companies/{company_id}/issues/{issue_id}/attachments"
        ))
        .multipart(form)
        .send()
        .await
        .with_context(|| format!("failed to upload attachment for issue {issue_id}"))?
        .error_for_status()
        .with_context(|| format!("attachment upload request failed for issue {issue_id}"))?
        .json::<PaperclipAttachment>()
        .await
        .context("failed to parse uploaded attachment response")
}

async fn delete_issue_attachment(api_base: &str, attachment_id: &str) -> Result<()> {
    let response = reqwest::Client::new()
        .delete(format!("{api_base}/api/attachments/{attachment_id}"))
        .send()
        .await
        .with_context(|| format!("failed to delete attachment {attachment_id}"))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(());
    }
    response
        .error_for_status()
        .with_context(|| format!("attachment delete request failed for {attachment_id}"))?;
    Ok(())
}

#[allow(dead_code)] // Reserved for optional lane-level issue sync if operators re-enable it.
fn render_lane_issue_description(entry: &FrontierSyncEntry) -> String {
    let artifacts = bullet_block(&entry.artifact_statuses, "No curated artifacts recorded.");
    let dependencies = bullet_block(&entry.dependency_keys, "No explicit dependencies.");
    let run_commands = lane_run_commands(entry);
    let mut body = format!(
        "This issue mirrors Raspberry frontier `{}`.\n\nStatus:\n- state: {}\n- unit: {} (`{}`)\n- lane: {} (`{}`)\n- kind: {}\n- detail: {}\n",
        entry.lane_key,
        entry.status,
        entry.unit_title,
        entry.unit_id,
        entry.lane_title,
        entry.lane_id,
        entry.lane_kind,
        entry.detail,
    );
    if let Some(run_id) = entry.current_run_id.as_ref() {
        body.push_str(&format!("- current run: {}\n", run_id));
    }
    if let Some(run_id) = entry.last_run_id.as_ref() {
        body.push_str(&format!("- last run: {}\n", run_id));
    }
    if let Some(stage) = entry.current_stage.as_ref() {
        body.push_str(&format!("- current stage: {}\n", stage));
    }
    if let Some(provider) = entry.current_stage_provider.as_ref() {
        body.push_str(&format!("- current provider: {}\n", provider));
    }
    if let Some(cli_name) = entry.current_stage_cli_name.as_ref() {
        body.push_str(&format!("- current cli: {}\n", cli_name));
    }
    if let Some(value) = entry.time_in_current_stage_secs {
        body.push_str(&format!("- time in current stage secs: {}\n", value));
    }
    if let Some(value) = entry.current_stage_idle_secs {
        body.push_str(&format!("- current stage idle secs: {}\n", value));
    }
    if let Some(value) = entry.last_started_at.as_ref() {
        body.push_str(&format!("- last started at: {}\n", value));
    }
    if let Some(value) = entry.last_finished_at.as_ref() {
        body.push_str(&format!("- last finished at: {}\n", value));
    }
    if let Some(value) = entry.last_exit_status {
        body.push_str(&format!("- last exit status: {}\n", value));
    }
    if let Some(kind) = entry.failure_kind.as_ref() {
        body.push_str(&format!("- failure kind: {}\n", kind));
    }
    if let Some(reason) = entry.blocker_reason.as_ref() {
        body.push_str(&format!("- blocker reason: {}\n", reason));
    }
    if let Some(value) = entry.last_completed_stage.as_ref() {
        body.push_str(&format!("- last completed stage: {}\n", value));
    }
    if let Some(value) = entry.last_stage_duration_ms {
        body.push_str(&format!("- last stage duration ms: {}\n", value));
    }
    if let Some(value) = entry.last_usage_summary.as_ref() {
        body.push_str(&format!("- last usage: {}\n", value));
    }
    body.push_str(&format!(
        "\nNext operator move:\n- {}\n\nArtifacts:\n{}\n\nDependencies:\n{}\n\nRun inspection:\n{}\n",
        entry.next_operator_move,
        artifacts,
        dependencies,
        bullet_block(&run_commands, "No run commands available yet."),
    ));
    if let Some(value) = entry.last_stderr_snippet.as_ref() {
        body.push_str(&format!(
            "\nLast stderr snippet:\n```text\n{}\n```\n",
            value.trim()
        ));
    }
    if let Some(value) = entry.last_stdout_snippet.as_ref() {
        body.push_str(&format!(
            "\nLast stdout snippet:\n```text\n{}\n```\n",
            value.trim()
        ));
    }
    body.push_str(&format!(
        "\nExecution route:\n- Wake the orchestrator with `{}`.\n- Run `{}` as the direct repo-local fallback.\n- Refresh Paperclip with `{}`.\n- See the synced `plan` document for the current lane playbook.\n- Coordinate in Paperclip, but let Raspberry move the frontier.\n",
        entry.wake_command,
        entry.route_command,
        entry.refresh_command,
    ));
    body
}

#[allow(dead_code)] // Reserved for optional lane-level issue sync if operators re-enable it.
fn lane_run_commands(entry: &FrontierSyncEntry) -> Vec<String> {
    let mut commands = Vec::new();
    if let Some(run_id) = entry.current_run_id.as_ref() {
        commands.push(format!("fabro inspect {run_id}"));
        commands.push(format!("fabro logs {run_id}"));
    }
    if let Some(run_id) = entry.last_run_id.as_ref() {
        let inspect = format!("fabro inspect {run_id}");
        if !commands.contains(&inspect) {
            commands.push(inspect);
            commands.push(format!("fabro logs {run_id}"));
        }
    }
    commands
}

fn render_frontier_lane_sets(frontier: &FrontierSyncModel) -> String {
    [
        render_lane_set(frontier, LaneExecutionStatus::Ready, "ready"),
        render_lane_set(frontier, LaneExecutionStatus::Running, "running"),
        render_lane_set(frontier, LaneExecutionStatus::Blocked, "blocked"),
        render_lane_set(frontier, LaneExecutionStatus::Failed, "failed"),
    ]
    .join("\n")
}

fn render_lane_set(
    frontier: &FrontierSyncModel,
    status: LaneExecutionStatus,
    label: &str,
) -> String {
    let lanes = frontier
        .entries
        .iter()
        .filter(|entry| entry.status == status)
        .map(|entry| format!("`{}`", entry.lane_key))
        .collect::<Vec<_>>();
    if lanes.is_empty() {
        return format!("- {label}: none");
    }
    format!("- {label}: {}", lanes.join(", "))
}

#[allow(dead_code)] // Reserved for optional lane-level issue sync if operators re-enable it.
fn bullet_block(values: &[String], empty_message: &str) -> String {
    if values.is_empty() {
        return format!("- {}", empty_message);
    }
    values
        .iter()
        .map(|value| format!("- {}", value))
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_issue_status(status: &str, assignee_agent_id: Option<&String>) -> String {
    if status == "in_progress" && assignee_agent_id.is_none() {
        return "todo".to_string();
    }
    status.to_string()
}

async fn collect_existing_sync_issues(
    api_base: &str,
    company_id: &str,
    project_id: &str,
    existing_state: Option<&serde_json::Value>,
) -> Result<BTreeMap<String, PaperclipIssue>> {
    let issues = list_company_issues(api_base, company_id, project_id).await?;
    let issues_by_id = issues
        .iter()
        .map(|issue| (issue.id.clone(), issue.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut issues_by_key = issues
        .iter()
        .filter_map(|issue| {
            extract_sync_key(issue.description.as_deref()).map(|sync_key| (sync_key, issue.clone()))
        })
        .collect::<BTreeMap<_, _>>();

    for (sync_key, issue_id) in synced_issue_ids_from_state(existing_state) {
        if issues_by_key.contains_key(&sync_key) {
            continue;
        }
        if let Some(issue) = issues_by_id.get(&issue_id) {
            issues_by_key.insert(sync_key, issue.clone());
        }
    }

    Ok(issues_by_key)
}

async fn list_company_issues(
    api_base: &str,
    company_id: &str,
    project_id: &str,
) -> Result<Vec<PaperclipIssue>> {
    reqwest::Client::new()
        .get(format!("{api_base}/api/companies/{company_id}/issues"))
        .query(&[("projectId", project_id)])
        .send()
        .await
        .context("failed to list paperclip issues")?
        .error_for_status()
        .context("paperclip issue list request failed")?
        .json::<Vec<PaperclipIssue>>()
        .await
        .context("failed to parse paperclip issues response")
}

fn synced_issue_ids_from_state(
    existing_state: Option<&serde_json::Value>,
) -> BTreeMap<String, String> {
    existing_state
        .and_then(|state| state.get("frontierSync"))
        .and_then(|value| value.get("issueIds"))
        .and_then(|value| value.as_object())
        .map(|values| {
            values
                .iter()
                .filter_map(|(key, value)| {
                    value
                        .as_str()
                        .map(|issue_id| (key.clone(), issue_id.to_string()))
                })
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default()
}

fn synced_snapshots_from_state(
    existing_state: Option<&serde_json::Value>,
) -> BTreeMap<String, serde_json::Value> {
    existing_state
        .and_then(|state| state.get("frontierSync"))
        .and_then(|value| value.get("snapshots"))
        .and_then(|value| value.as_object())
        .map(|values| {
            values
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default()
}

fn frontier_snapshots(frontier: &FrontierSyncModel) -> BTreeMap<String, serde_json::Value> {
    let mut snapshots = BTreeMap::new();
    snapshots.insert(
        frontier_root_sync_key(&frontier.program),
        json!({
            "status": root_issue_status(frontier),
            "ready": frontier.summary.ready,
            "running": frontier.summary.running,
            "blocked": frontier.summary.blocked,
            "failed": frontier.summary.failed,
            "complete": frontier.summary.complete,
        }),
    );
    for entry in &frontier.entries {
        snapshots.insert(entry.sync_key.clone(), frontier_entry_snapshot(entry));
    }
    snapshots
}

fn frontier_snapshots_json(frontier: &FrontierSyncModel) -> serde_json::Value {
    let mut values = serde_json::Map::new();
    for (key, value) in frontier_snapshots(frontier) {
        values.insert(key, value);
    }
    serde_json::Value::Object(values)
}

fn frontier_entry_snapshot(entry: &FrontierSyncEntry) -> serde_json::Value {
    json!({
        "status": entry.status.to_string(),
        "detail": entry.detail,
        "currentRunId": entry.current_run_id,
        "lastRunId": entry.last_run_id,
        "blockerReason": entry.blocker_reason,
        "failureKind": entry.failure_kind,
    })
}

fn root_issue_status(frontier: &FrontierSyncModel) -> &'static str {
    if frontier.summary.failed > 0 || frontier.summary.blocked > 0 {
        "blocked"
    } else if frontier.summary.running > 0 {
        "in_progress"
    } else if frontier.summary.ready > 0 {
        "todo"
    } else {
        "done"
    }
}

fn snapshot_changed(previous: Option<&serde_json::Value>, current: &serde_json::Value) -> bool {
    previous != Some(current)
}

fn render_root_transition_comment(
    frontier: &FrontierSyncModel,
    previous: Option<&serde_json::Value>,
    current: &serde_json::Value,
) -> String {
    format!(
        "## Frontier Sync Update\n\n{}\n\n- previous snapshot: `{}`\n- current snapshot: `{}`\n- wake: `{}`\n- route fallback: `{}`\n- refresh after movement: `{}`\n- plan document: `plan`\n",
        render_frontier_summary(frontier),
        compact_snapshot(previous),
        compact_snapshot(Some(current)),
        frontier.wake_command,
        frontier.route_command,
        frontier.refresh_command,
    )
}

#[allow(dead_code)] // Reserved for optional lane-level transition comments if operators re-enable them.
fn render_lane_transition_comment(
    entry: &FrontierSyncEntry,
    previous: Option<&serde_json::Value>,
    current: &serde_json::Value,
) -> String {
    format!(
        "## Frontier Sync Update\n\n- lane: `{}`\n- previous snapshot: `{}`\n- current snapshot: `{}`\n- next move: {}\n- wake orchestrator: `{}`\n- route fallback: `{}`\n- refresh after movement: `{}`\n- plan document: `plan`\n",
        entry.lane_key,
        compact_snapshot(previous),
        compact_snapshot(Some(current)),
        entry.next_operator_move,
        entry.wake_command,
        entry.route_command,
        entry.refresh_command,
    )
}

fn desired_root_work_products(
    frontier: &FrontierSyncModel,
    attachments: Option<&BTreeMap<String, PaperclipAttachment>>,
    api_base: &str,
) -> Vec<DesiredWorkProduct> {
    let manifest_filename = format!("{}-program-manifest.yaml", frontier.program);
    let state_filename = format!("{}-program-state.json", frontier.program);
    vec![
        DesiredWorkProduct {
            external_id: format!(
                "fabro.paperclip::{}::manifest",
                frontier_root_sync_key(&frontier.program)
            ),
            title: "Program manifest".to_string(),
            status: "active".to_string(),
            health_status: path_health(&frontier.manifest_path),
            is_primary: true,
            url: attachment_url(
                attachments.and_then(|attachments| attachments.get(&manifest_filename)),
                api_base,
            ),
            summary: Some("Authoritative Raspberry program manifest.".to_string()),
            metadata: json!({
                "path": frontier.manifest_path.display().to_string(),
                "attachmentFilename": manifest_filename,
                "kind": "program_manifest",
                "syncKey": frontier_root_sync_key(&frontier.program),
            }),
        },
        DesiredWorkProduct {
            external_id: format!(
                "fabro.paperclip::{}::state",
                frontier_root_sync_key(&frontier.program)
            ),
            title: "Program state".to_string(),
            status: root_work_product_status(frontier),
            health_status: path_health(&frontier.state_path),
            is_primary: false,
            url: attachment_url(
                attachments.and_then(|attachments| attachments.get(&state_filename)),
                api_base,
            ),
            summary: Some("Current Raspberry program runtime state.".to_string()),
            metadata: json!({
                "path": frontier.state_path.display().to_string(),
                "attachmentFilename": state_filename,
                "kind": "program_state",
                "syncKey": frontier_root_sync_key(&frontier.program),
            }),
        },
    ]
}

fn desired_lane_work_products(
    entry: &FrontierSyncEntry,
    attachments: Option<&BTreeMap<String, PaperclipAttachment>>,
    api_base: &str,
) -> Vec<DesiredWorkProduct> {
    entry
        .artifact_paths
        .iter()
        .enumerate()
        .map(|(index, path)| DesiredWorkProduct {
            external_id: format!(
                "fabro.paperclip::{}::artifact::{}",
                entry.sync_key,
                sha256_hex(path)
            ),
            title: path.clone(),
            status: lane_work_product_status(entry),
            health_status: if entry
                .artifact_statuses
                .get(index)
                .is_some_and(|status| status.starts_with("present:"))
            {
                "healthy".to_string()
            } else {
                "unhealthy".to_string()
            },
            is_primary: index == 0,
            url: attachment_url(
                attachments.and_then(|attachments| {
                    attachments.get(&lane_attachment_filename(entry, path))
                }),
                api_base,
            ),
            summary: Some(entry.detail.clone()),
            metadata: json!({
                "path": path,
                "attachmentFilename": lane_attachment_filename(entry, path),
                "kind": "lane_artifact",
                "syncKey": entry.sync_key,
                "laneKey": entry.lane_key,
                "status": entry.status.to_string(),
            }),
        })
        .collect()
}

fn root_work_product_status(frontier: &FrontierSyncModel) -> String {
    if frontier.summary.failed > 0 || frontier.summary.blocked > 0 {
        return "failed".to_string();
    }
    if frontier.summary.running > 0 {
        return "active".to_string();
    }
    if frontier.summary.ready > 0 {
        return "draft".to_string();
    }
    "ready_for_review".to_string()
}

fn lane_work_product_status(entry: &FrontierSyncEntry) -> String {
    match entry.status {
        LaneExecutionStatus::Ready => "draft",
        LaneExecutionStatus::Running => "active",
        LaneExecutionStatus::Blocked | LaneExecutionStatus::Failed => "failed",
        LaneExecutionStatus::Complete => "ready_for_review",
    }
    .to_string()
}

fn path_health(path: &Path) -> String {
    if path.exists() {
        "healthy".to_string()
    } else {
        "unhealthy".to_string()
    }
}

fn lane_attachment_filename(entry: &FrontierSyncEntry, artifact_path: &str) -> String {
    let basename = Path::new(artifact_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    format!(
        "{}-{}-{}",
        sanitize_filename_component(&entry.unit_id),
        sanitize_filename_component(&entry.lane_id),
        sanitize_filename_component(basename),
    )
}

fn sanitize_filename_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

#[allow(dead_code)] // Reserved for optional lane-level attachment sync if operators re-enable it.
fn content_type_for_path(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some("json") => "application/json",
        Some("md") => "text/markdown",
        Some("txt") => "text/plain",
        Some("csv") => "text/csv",
        Some("html") => "text/html",
        Some("yaml") | Some("yml") => "text/plain",
        _ => "text/plain",
    }
}

fn attachment_url(attachment: Option<&PaperclipAttachment>, api_base: &str) -> Option<String> {
    let content_path = attachment.and_then(|attachment| attachment.content_path.as_ref())?;
    Some(format!("{api_base}{content_path}"))
}

fn compact_snapshot(snapshot: Option<&serde_json::Value>) -> String {
    snapshot
        .and_then(|value| serde_json::to_string(value).ok())
        .unwrap_or_else(|| "none".to_string())
}

fn imported_agent_ids(imported_agents: &[PaperclipImportAgent]) -> BTreeMap<String, String> {
    imported_agents
        .iter()
        .filter_map(|agent| agent.id.as_ref().map(|id| (agent.slug.clone(), id.clone())))
        .collect()
}

async fn upsert_sync_issue(
    api_base: &str,
    company_id: &str,
    goal_id: &str,
    project_id: &str,
    existing_issue: Option<&PaperclipIssue>,
    desired: &DesiredIssue,
) -> Result<PaperclipIssue> {
    let client = reqwest::Client::new();
    let payload = json!({
        "goalId": goal_id,
        "projectId": project_id,
        "projectWorkspaceId": desired.project_workspace_id,
        "assigneeAdapterOverrides": desired.assignee_adapter_overrides,
        "parentId": desired.parent_id,
        "title": desired.title,
        "description": desired.description,
        "status": desired.status,
        "priority": desired.priority,
        "assigneeAgentId": desired.assignee_agent_id,
        "executionWorkspacePreference": desired.execution_workspace_preference,
    });

    let response = if let Some(issue) = existing_issue {
        client
            .patch(format!("{api_base}/api/issues/{}", issue.id))
            .json(&payload)
            .send()
            .await
            .with_context(|| format!("failed to update paperclip issue {}", issue.id))?
    } else {
        client
            .post(format!("{api_base}/api/companies/{company_id}/issues"))
            .json(&payload)
            .send()
            .await
            .context("failed to create paperclip issue")?
    };

    response
        .error_for_status()
        .context("paperclip issue upsert request failed")?
        .json::<PaperclipIssue>()
        .await
        .context("failed to parse paperclip issue response")
}

fn with_sync_marker(body: String, sync_key: &str) -> String {
    format!(
        "{}\n\n<!-- {} {} -->\n",
        body.trim_end(),
        SYNC_MARKER_PREFIX,
        sync_key,
    )
}

fn extract_sync_key(description: Option<&str>) -> Option<String> {
    let description = description?;
    description.lines().find_map(|line| {
        let trimmed = line.trim();
        let inner = trimmed.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
        let sync_key = inner.strip_prefix(SYNC_MARKER_PREFIX)?.trim();
        if sync_key.is_empty() {
            return None;
        }
        Some(sync_key.to_string())
    })
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
    shlex::try_quote(value).map_or_else(
        |_| format!("'{}'", value.replace('\'', "'\\''")),
        |quoted| quoted.to_string(),
    )
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
    use httpmock::Method::GET;
    use httpmock::MockServer;
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
    fn derive_bootstrap_mission_prefers_referenced_master_plan_over_latest_leaf() {
        let temp = tempdir().expect("tempdir");
        std::fs::write(
            temp.path().join("SPEC.md"),
            "# rXMRagent Spec\n\nThe goal is to launch a zero-human game studio.\n",
        )
        .expect("write spec");
        std::fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        std::fs::write(
            temp.path().join("plans/001-rxmr-poker-mvp.md"),
            "# rXMR Casino MVP — Provably Fair Terminal Games on a Privacy Chain\n",
        )
        .expect("write master plan");
        std::fs::write(
            temp.path().join("plans/018-war-game.md"),
            "# Casino War — The Simplest Card Game in the Catalog\n\nThis plan builds on `plans/001-rxmr-poker-mvp.md`.\n",
        )
        .expect("write leaf plan");
        std::fs::write(
            temp.path().join("plans/021-wheel-game.md"),
            "# Wheel of Fortune — Seed-Derived Multiplier Wheel for rXMR Casino\n\nThis plan builds on `plans/001-rxmr-poker-mvp.md`.\n",
        )
        .expect("write later leaf plan");
        let blueprint = sample_blueprint();

        let mission =
            derive_bootstrap_mission(&blueprint, temp.path(), "Rxmragent").expect("mission");

        assert!(mission
            .company_description
            .contains("rXMR Casino MVP — Provably Fair Terminal Games on a Privacy Chain"));
        assert!(!mission.company_description.contains("Casino War"));
        assert!(!mission.company_description.contains("Wheel of Fortune"));
    }

    #[test]
    fn lane_adapter_type_uses_process_agents() {
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
        assert_eq!(lane_adapter_type(&unit, &lane), "process");
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

    #[test]
    fn sync_marker_round_trips() {
        let body = with_sync_marker("Frontier body".to_string(), "frontier/zend/root");

        assert_eq!(
            extract_sync_key(Some(&body)),
            Some("frontier/zend/root".to_string()),
        );
    }

    #[test]
    fn ensure_local_paperclip_instance_seeds_config_env_and_key() {
        let temp = tempdir().expect("tempdir");
        let data_dir = temp.path().join(".paperclip");

        ensure_local_paperclip_instance(&data_dir, "http://127.0.0.1:3112")
            .expect("seed local paperclip instance");

        let instance_root = data_dir.join("instances").join("default");
        let config_path = instance_root.join("config.json");
        let env_path = instance_root.join(".env");
        let key_path = instance_root.join("secrets").join("master.key");

        assert!(config_path.exists());
        assert!(env_path.exists());
        assert!(key_path.exists());

        let config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(config_path).expect("read config"))
                .expect("parse config");
        assert_eq!(
            config["server"]["deploymentMode"].as_str(),
            Some("local_trusted"),
        );
        assert_eq!(config["server"]["host"].as_str(), Some("127.0.0.1"));
        assert_eq!(config["server"]["port"].as_u64(), Some(3112));
    }

    #[test]
    fn generated_agent_matches_lane_agent_by_unit_and_lane_key() {
        let desired = BundleAgent {
            slug: "command-center-client".to_string(),
            name: "Command Center Client".to_string(),
            adapter_type: "codex_local",
            metadata_type: None,
            unit: Some("command-center-client".to_string()),
            lane_key: Some("command-center-client:program".to_string()),
            plan_id: None,
        };
        let managed = PaperclipManagedAgent {
            id: "agent-1".to_string(),
            slug: None,
            adapter_config: None,
            metadata: Some(json!({
                "source": "fabro.paperclip",
                "unit": "command-center-client",
                "laneKey": "command-center-client:program",
            })),
        };

        assert!(generated_agent_matches(&managed, &desired));
    }

    #[tokio::test]
    async fn resolve_existing_company_id_falls_back_from_stale_saved_id() {
        let server = MockServer::start_async().await;
        let saved_id = "old-company-id";
        let live_id = "live-company-id";
        let mock = server.mock(|when, then| {
            when.method(GET).path("/api/companies");
            then.status(200)
                .header("content-type", "application/json")
                .body(format!(r#"[{{"id":"{live_id}","name":"Zend"}}]"#,));
        });

        let resolved = resolve_existing_company_id(&server.base_url(), "Zend", Some(saved_id))
            .await
            .expect("resolve company id");

        mock.assert();
        assert_eq!(resolved, Some(live_id.to_string()));
    }

    #[tokio::test]
    async fn resolve_managed_agent_falls_back_from_stale_saved_agent_id() {
        let server = MockServer::start_async().await;
        let company_id = "company-1";
        let stale_id = "stale-agent";
        let live_id = "live-agent";
        let slug = "raspberry-orchestrator";

        let stale_lookup = server.mock(|when, then| {
            when.method(GET).path(format!("/api/agents/{stale_id}"));
            then.status(404);
        });
        let list_agents = server.mock(|when, then| {
            when.method(GET)
                .path(format!("/api/companies/{company_id}/agents"));
            then.status(200).header("content-type", "application/json").body(format!(
                r#"[{{"id":"{live_id}","slug":"{slug}","name":"Raspberry Orchestrator","adapterType":"process"}}]"#
            ));
        });

        let existing_state = json!({
            "agents": [
                { "slug": slug, "id": stale_id }
            ]
        });

        let resolved =
            resolve_managed_agent(&server.base_url(), company_id, slug, Some(&existing_state))
                .await
                .expect("resolve managed agent");

        stale_lookup.assert();
        list_agents.assert();
        assert_eq!(resolved.id, live_id);
        assert_eq!(resolved.slug.as_deref(), Some(slug));
    }

    #[test]
    fn paperclip_server_command_wraps_override_with_exec() {
        let command = paperclip_server_command(Some("paperclipai"), Path::new("/tmp/demo"))
            .expect("server command");

        assert!(command.contains("exec env TMPDIR="));
        assert!(command.contains("paperclipai run --data-dir "));
    }

    #[test]
    fn build_company_bundle_generates_distinct_agents_for_multiple_lanes() {
        let temp = tempdir().expect("tempdir");
        let blueprint = ProgramBlueprint {
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
                    path: PathBuf::from("outputs/client/plan.md"),
                }],
                milestones: Vec::new(),
                lanes: vec![
                    BlueprintLane {
                        id: "plan".to_string(),
                        kind: Default::default(),
                        title: "Plan".to_string(),
                        family: "bootstrap".to_string(),
                        workflow_family: None,
                        slug: None,
                        template: WorkflowTemplate::Bootstrap,
                        goal: "Plan the client".to_string(),
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
                    },
                    BlueprintLane {
                        id: "implement".to_string(),
                        kind: Default::default(),
                        title: "Implement".to_string(),
                        family: "implementation".to_string(),
                        workflow_family: None,
                        slug: None,
                        template: WorkflowTemplate::Bootstrap,
                        goal: "Implement the client".to_string(),
                        managed_milestone: "verified".to_string(),
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
                    },
                ],
            }],
        };
        let mission = BootstrapMission {
            company_description: "Zend company".to_string(),
            goal_title: "Advance Zend".to_string(),
            goal_description: "Advance Zend honestly.".to_string(),
            project_name: "Zend Workspace".to_string(),
            project_description: "Repo workspace".to_string(),
            workspace_name: "Zend repo".to_string(),
        };
        let frontier = FrontierSyncModel {
            program: "zend".to_string(),
            manifest_path: temp.path().join("malinka/programs/zend.yaml"),
            state_path: temp.path().join(".raspberry/zend-state.json"),
            wake_command: "fabro paperclip wake --target-repo /tmp/zend --program zend --agent raspberry-orchestrator".to_string(),
            route_command: "bash fabro/paperclip/zend/scripts/raspberry-orchestrator.sh"
                .to_string(),
            refresh_command: "fabro paperclip refresh --target-repo /tmp/zend --program zend"
                .to_string(),
            status_command: "fabro paperclip status --target-repo /tmp/zend --program zend"
                .to_string(),
            summary: FrontierSummary {
                ready: 2,
                running: 0,
                blocked: 0,
                failed: 0,
                complete: 0,
            },
            entries: vec![
                FrontierSyncEntry {
                    sync_key: "frontier/zend/lane/command-center-client:plan".to_string(),
                    lane_key: "command-center-client:plan".to_string(),
                    unit_id: "command-center-client".to_string(),
                    unit_title: "Command Center Client".to_string(),
                    lane_id: "plan".to_string(),
                    lane_title: "Plan".to_string(),
                    lane_kind: "artifact".to_string(),
                    status: LaneExecutionStatus::Ready,
                    detail: "ready".to_string(),
                    current_run_id: None,
                    last_run_id: None,
                    current_stage: None,
                    current_stage_provider: None,
                    current_stage_cli_name: None,
                    time_in_current_stage_secs: None,
                    current_stage_idle_secs: None,
                    last_started_at: None,
                    last_finished_at: None,
                    last_exit_status: None,
                    last_usage_summary: None,
                    last_completed_stage: None,
                    last_stage_duration_ms: None,
                    last_stdout_snippet: None,
                    last_stderr_snippet: None,
                    failure_kind: None,
                    blocker_reason: None,
                    wake_command: "fabro paperclip wake --target-repo /tmp/zend --program zend --agent raspberry-orchestrator".to_string(),
                    route_command: "bash fabro/paperclip/zend/scripts/raspberry-orchestrator.sh"
                        .to_string(),
                    refresh_command: "fabro paperclip refresh --target-repo /tmp/zend --program zend".to_string(),
                    artifact_paths: vec!["outputs/client/plan.md".to_string()],
                    artifact_statuses: vec!["missing: outputs/client/plan.md".to_string()],
                    dependency_keys: Vec::new(),
                    next_operator_move: "Run the route".to_string(),
                },
                FrontierSyncEntry {
                    sync_key: "frontier/zend/lane/command-center-client:implement".to_string(),
                    lane_key: "command-center-client:implement".to_string(),
                    unit_id: "command-center-client".to_string(),
                    unit_title: "Command Center Client".to_string(),
                    lane_id: "implement".to_string(),
                    lane_title: "Implement".to_string(),
                    lane_kind: "artifact".to_string(),
                    status: LaneExecutionStatus::Ready,
                    detail: "ready".to_string(),
                    current_run_id: None,
                    last_run_id: None,
                    current_stage: None,
                    current_stage_provider: None,
                    current_stage_cli_name: None,
                    time_in_current_stage_secs: None,
                    current_stage_idle_secs: None,
                    last_started_at: None,
                    last_finished_at: None,
                    last_exit_status: None,
                    last_usage_summary: None,
                    last_completed_stage: None,
                    last_stage_duration_ms: None,
                    last_stdout_snippet: None,
                    last_stderr_snippet: None,
                    failure_kind: None,
                    blocker_reason: None,
                    wake_command: "fabro paperclip wake --target-repo /tmp/zend --program zend --agent raspberry-orchestrator".to_string(),
                    route_command: "bash fabro/paperclip/zend/scripts/raspberry-orchestrator.sh"
                        .to_string(),
                    refresh_command: "fabro paperclip refresh --target-repo /tmp/zend --program zend".to_string(),
                    artifact_paths: vec!["outputs/client/implementation.md".to_string()],
                    artifact_statuses: vec!["missing: outputs/client/implementation.md".to_string()],
                    dependency_keys: Vec::new(),
                    next_operator_move: "Run the route".to_string(),
                },
            ],
        };

        let bundle = build_company_bundle(
            &blueprint,
            temp.path(),
            "Zend",
            &mission,
            &temp
                .path()
                .join("malinka/paperclip/zend/scripts/raspberry-orchestrator.sh"),
            &temp
                .path()
                .join("malinka/paperclip/zend/scripts/fabro-agent-minimax.sh"),
            &frontier,
            None,
            None,
        )
        .expect("build company bundle");

        let lane_agents = bundle
            .agents
            .iter()
            .filter(|agent| agent.lane_key.is_some())
            .collect::<Vec<_>>();
        assert!(lane_agents.is_empty());
    }

    #[test]
    fn process_lane_agents_use_minimax_wrapper_and_prompt_template() {
        let temp = tempdir().expect("tempdir");
        let unit = BlueprintUnit {
            id: "wallet".to_string(),
            title: "Wallet".to_string(),
            output_root: PathBuf::from("outputs/wallet"),
            artifacts: Vec::new(),
            milestones: Vec::new(),
            lanes: vec![BlueprintLane {
                id: "implement".to_string(),
                kind: Default::default(),
                title: "Implement".to_string(),
                family: "implement".to_string(),
                workflow_family: None,
                slug: None,
                template: WorkflowTemplate::Bootstrap,
                goal: "Implement wallet".to_string(),
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
        };
        let lane = &unit.lanes[0];
        let frontier = FrontierSyncModel {
            program: "demo".to_string(),
            manifest_path: temp.path().join("malinka/programs/demo.yaml"),
            state_path: temp.path().join(".raspberry/demo-state.json"),
            wake_command: "wake".to_string(),
            route_command: "route".to_string(),
            refresh_command: "refresh".to_string(),
            status_command: "status".to_string(),
            summary: FrontierSummary {
                ready: 0,
                running: 0,
                blocked: 0,
                failed: 0,
                complete: 0,
            },
            entries: Vec::new(),
        };
        let mission = BootstrapMission {
            company_description: "Demo".to_string(),
            goal_title: "Demo".to_string(),
            goal_description: "Demo".to_string(),
            project_name: "Demo".to_string(),
            project_description: "Demo".to_string(),
            workspace_name: "Demo".to_string(),
        };

        let draft = lane_agent_draft(
            &ProgramBlueprint {
                version: 1,
                program: BlueprintProgram {
                    id: "demo".to_string(),
                    max_parallel: 1,
                    state_path: None,
                    run_dir: None,
                },
                inputs: Default::default(),
                package: Default::default(),
                units: vec![unit.clone()],
            },
            temp.path(),
            &mission,
            &temp.path().join("fabro-agent-minimax.sh"),
            &frontier,
            &unit,
            lane,
            None,
        );

        let adapter_config = draft.manifest.get("adapterConfig").expect("adapter config");
        let command = adapter_config
            .get("command")
            .and_then(|value| value.as_str())
            .expect("command");
        let first_arg = adapter_config
            .get("args")
            .and_then(|value| value.as_array())
            .and_then(|value| value.first())
            .and_then(|value| value.as_str())
            .expect("arg");
        let prompt_template = adapter_config
            .get("promptTemplate")
            .and_then(|value| value.as_str())
            .expect("prompt template");
        let timeout = adapter_config
            .get("timeoutSec")
            .and_then(|value| value.as_i64())
            .expect("timeout");

        assert_eq!(draft.agent.adapter_type, "process");
        assert_eq!(command, "bash");
        assert_eq!(
            first_arg,
            temp.path()
                .join("fabro-agent-minimax.sh")
                .display()
                .to_string()
        );
        assert!(prompt_template.contains("You coordinate the `implement` frontier"));
        assert_eq!(timeout, 1800);
    }

    #[test]
    fn orchestrator_process_agent_uses_command_and_args() {
        let temp = tempdir().expect("tempdir");
        let frontier = FrontierSyncModel {
            program: "demo".to_string(),
            manifest_path: temp.path().join("malinka/programs/demo.yaml"),
            state_path: temp.path().join(".raspberry/demo-state.json"),
            wake_command: "wake".to_string(),
            route_command: "route".to_string(),
            refresh_command: "refresh".to_string(),
            status_command: "status".to_string(),
            summary: FrontierSummary {
                ready: 0,
                running: 0,
                blocked: 0,
                failed: 0,
                complete: 0,
            },
            entries: Vec::new(),
        };
        let blueprint = ProgramBlueprint {
            version: 1,
            program: BlueprintProgram {
                id: "demo".to_string(),
                max_parallel: 1,
                state_path: None,
                run_dir: None,
            },
            inputs: Default::default(),
            package: Default::default(),
            units: Vec::new(),
        };
        let script_path = temp
            .path()
            .join("malinka/paperclip/demo/scripts/raspberry-orchestrator.sh");

        let draft = orchestrator_draft(&blueprint, temp.path(), &script_path, &frontier);
        let adapter_config = draft.manifest.get("adapterConfig").expect("adapter config");
        let expected_arg = script_path.display().to_string();

        assert_eq!(
            adapter_config
                .get("command")
                .and_then(|value| value.as_str()),
            Some("bash")
        );
        assert_eq!(
            adapter_config
                .get("args")
                .and_then(|value| value.as_array())
                .and_then(|value| value.first())
                .and_then(|value| value.as_str()),
            Some(expected_arg.as_str())
        );
    }

    #[test]
    fn mission_ceo_agent_uses_minimax_process_wrapper() {
        let temp = tempdir().expect("tempdir");
        let blueprint = ProgramBlueprint {
            version: 1,
            program: BlueprintProgram {
                id: "demo".to_string(),
                max_parallel: 1,
                state_path: None,
                run_dir: None,
            },
            inputs: Default::default(),
            package: Default::default(),
            units: Vec::new(),
        };
        let mission = BootstrapMission {
            company_description: "Demo".to_string(),
            goal_title: "Demo".to_string(),
            goal_description: "Demo".to_string(),
            project_name: "Demo".to_string(),
            project_description: "Demo".to_string(),
            workspace_name: "Demo".to_string(),
        };

        let draft = mission_ceo_draft(
            &blueprint,
            temp.path(),
            &mission,
            &temp.path().join("fabro-agent-minimax.sh"),
        );
        let adapter_config = draft.manifest.get("adapterConfig").expect("adapter config");
        let command = adapter_config
            .get("command")
            .and_then(|value| value.as_str())
            .expect("command");
        let first_arg = adapter_config
            .get("args")
            .and_then(|value| value.as_array())
            .and_then(|value| value.first())
            .and_then(|value| value.as_str())
            .expect("arg");
        let prompt_template = adapter_config
            .get("promptTemplate")
            .and_then(|value| value.as_str())
            .expect("prompt");

        assert_eq!(draft.agent.adapter_type, "process");
        assert_eq!(command, "bash");
        assert_eq!(
            first_arg,
            temp.path()
                .join("fabro-agent-minimax.sh")
                .display()
                .to_string()
        );
        assert!(prompt_template.contains("You own the company mission for `demo`."));
    }

    #[test]
    fn render_lane_issue_description_includes_next_move_and_run_commands() {
        let entry = FrontierSyncEntry {
            sync_key: "frontier/zend/lane/client:implement".to_string(),
            lane_key: "client:implement".to_string(),
            unit_id: "client".to_string(),
            unit_title: "Client".to_string(),
            lane_id: "implement".to_string(),
            lane_title: "Implement".to_string(),
            lane_kind: "artifact".to_string(),
            status: LaneExecutionStatus::Failed,
            detail: "verification failed".to_string(),
            current_run_id: None,
            last_run_id: Some("01KMTEST".to_string()),
            current_stage: Some("Verify".to_string()),
            current_stage_provider: None,
            current_stage_cli_name: None,
            time_in_current_stage_secs: None,
            current_stage_idle_secs: None,
            last_started_at: Some("2026-03-20T12:00:00Z".to_string()),
            last_finished_at: Some("2026-03-20T12:04:00Z".to_string()),
            last_exit_status: Some(1),
            last_usage_summary: Some("prompt=123 completion=45".to_string()),
            last_completed_stage: Some("Implement".to_string()),
            last_stage_duration_ms: Some(240000),
            last_stdout_snippet: Some("stdout snippet".to_string()),
            last_stderr_snippet: Some("stderr snippet".to_string()),
            failure_kind: Some("Verification".to_string()),
            blocker_reason: Some("tests failed".to_string()),
            wake_command: "fabro paperclip wake --target-repo /tmp/zend --program zend --agent raspberry-orchestrator".to_string(),
            route_command: "bash fabro/paperclip/zend/scripts/raspberry-orchestrator.sh"
                .to_string(),
            refresh_command: "fabro paperclip refresh --target-repo /tmp/zend --program zend".to_string(),
            artifact_paths: vec!["outputs/client/implementation.md".to_string()],
            artifact_statuses: vec!["present: outputs/client/implementation.md".to_string()],
            dependency_keys: vec!["spec@reviewed".to_string()],
            next_operator_move: "Inspect the failed run and rerun the route.".to_string(),
        };

        let rendered = render_lane_issue_description(&entry);

        assert!(rendered.contains("Next operator move:"));
        assert!(rendered.contains("fabro inspect 01KMTEST"));
        assert!(rendered.contains("present: outputs/client/implementation.md"));
        assert!(rendered.contains("stderr snippet"));
    }

    #[test]
    fn render_frontier_detail_sections_includes_running_and_failed_entries() {
        let frontier = FrontierSyncModel {
            program: "zend".to_string(),
            manifest_path: PathBuf::from("/tmp/zend/fabro/programs/zend.yaml"),
            state_path: PathBuf::from("/tmp/zend/.raspberry/zend-state.json"),
            wake_command: "wake".to_string(),
            route_command: "route".to_string(),
            refresh_command: "refresh".to_string(),
            status_command: "status".to_string(),
            summary: FrontierSummary {
                ready: 1,
                running: 1,
                blocked: 0,
                failed: 1,
                complete: 0,
            },
            entries: vec![
                FrontierSyncEntry {
                    sync_key: "frontier/zend/lane/client:implement".to_string(),
                    lane_key: "client:implement".to_string(),
                    unit_id: "client".to_string(),
                    unit_title: "Client".to_string(),
                    lane_id: "implement".to_string(),
                    lane_title: "Implement".to_string(),
                    lane_kind: "artifact".to_string(),
                    status: LaneExecutionStatus::Running,
                    detail: "running".to_string(),
                    current_run_id: Some("01RUN".to_string()),
                    last_run_id: Some("01RUN".to_string()),
                    current_stage: Some("Implement".to_string()),
                    current_stage_provider: None,
                    current_stage_cli_name: None,
                    time_in_current_stage_secs: None,
                    current_stage_idle_secs: None,
                    last_started_at: None,
                    last_finished_at: None,
                    last_exit_status: None,
                    last_usage_summary: None,
                    last_completed_stage: None,
                    last_stage_duration_ms: None,
                    last_stdout_snippet: None,
                    last_stderr_snippet: None,
                    failure_kind: None,
                    blocker_reason: None,
                    wake_command: "wake".to_string(),
                    route_command: "route".to_string(),
                    refresh_command: "refresh".to_string(),
                    artifact_paths: vec![],
                    artifact_statuses: vec![],
                    dependency_keys: vec![],
                    next_operator_move: "watch".to_string(),
                },
                FrontierSyncEntry {
                    sync_key: "frontier/zend/lane/wallet:implement".to_string(),
                    lane_key: "wallet:implement".to_string(),
                    unit_id: "wallet".to_string(),
                    unit_title: "Wallet".to_string(),
                    lane_id: "implement".to_string(),
                    lane_title: "Implement".to_string(),
                    lane_kind: "artifact".to_string(),
                    status: LaneExecutionStatus::Failed,
                    detail: "failed".to_string(),
                    current_run_id: None,
                    last_run_id: Some("01FAIL".to_string()),
                    current_stage: Some("Review".to_string()),
                    current_stage_provider: None,
                    current_stage_cli_name: None,
                    time_in_current_stage_secs: None,
                    current_stage_idle_secs: None,
                    last_started_at: None,
                    last_finished_at: None,
                    last_exit_status: Some(1),
                    last_usage_summary: None,
                    last_completed_stage: None,
                    last_stage_duration_ms: None,
                    last_stdout_snippet: None,
                    last_stderr_snippet: Some("boom".to_string()),
                    failure_kind: Some("proof_script_failure".to_string()),
                    blocker_reason: Some("tests failed".to_string()),
                    wake_command: "wake".to_string(),
                    route_command: "route".to_string(),
                    refresh_command: "refresh".to_string(),
                    artifact_paths: vec![],
                    artifact_statuses: vec![],
                    dependency_keys: vec![],
                    next_operator_move: "fix".to_string(),
                },
            ],
        };

        let rendered = render_frontier_detail_sections(&frontier);

        assert!(rendered.contains("## Running"));
        assert!(rendered.contains("## Failed"));
        assert!(rendered.contains("`client:implement`"));
        assert!(rendered.contains("`wallet:implement`"));
    }

    #[test]
    fn desired_lane_issue_sets_workspace_binding_and_override() {
        let entry = FrontierSyncEntry {
            sync_key: "frontier/zend/lane/client:implement".to_string(),
            lane_key: "client:implement".to_string(),
            unit_id: "client".to_string(),
            unit_title: "Client".to_string(),
            lane_id: "implement".to_string(),
            lane_title: "Implement".to_string(),
            lane_kind: "artifact".to_string(),
            status: LaneExecutionStatus::Ready,
            detail: "ready".to_string(),
            current_run_id: None,
            last_run_id: None,
            current_stage: None,
            current_stage_provider: None,
            current_stage_cli_name: None,
            time_in_current_stage_secs: None,
            current_stage_idle_secs: None,
            last_started_at: None,
            last_finished_at: None,
            last_exit_status: None,
            last_usage_summary: None,
            last_completed_stage: None,
            last_stage_duration_ms: None,
            last_stdout_snippet: None,
            last_stderr_snippet: None,
            failure_kind: None,
            blocker_reason: None,
            wake_command: "fabro paperclip wake --target-repo /tmp/zend --program zend --agent raspberry-orchestrator".to_string(),
            route_command: "bash fabro/paperclip/zend/scripts/raspberry-orchestrator.sh".to_string(),
            refresh_command: "fabro paperclip refresh --target-repo /tmp/zend --program zend".to_string(),
            artifact_paths: vec!["outputs/client/implementation.md".to_string()],
            artifact_statuses: vec!["missing: outputs/client/implementation.md".to_string()],
            dependency_keys: vec!["spec@reviewed".to_string()],
            next_operator_move: "Wake the orchestrator.".to_string(),
        };

        let desired = desired_lane_issue(
            &entry,
            "root-1",
            Some("orchestrator-1".to_string()),
            Some("agent-1".to_string()),
            true,
            "workspace-1",
        )
        .expect("desired lane issue");

        assert_eq!(desired.project_workspace_id.as_deref(), Some("workspace-1"));
        assert_eq!(
            desired.assignee_adapter_overrides,
            Some(json!({ "useProjectWorkspace": true }))
        );
    }

    #[test]
    fn build_company_markdown_includes_lane_sets_and_live_details() {
        let frontier = FrontierSyncModel {
            program: "zend".to_string(),
            manifest_path: PathBuf::from("/tmp/zend/fabro/programs/zend.yaml"),
            state_path: PathBuf::from("/tmp/zend/.raspberry/zend-state.json"),
            wake_command: "wake".to_string(),
            route_command: "route".to_string(),
            refresh_command: "refresh".to_string(),
            status_command: "status".to_string(),
            summary: FrontierSummary {
                ready: 0,
                running: 1,
                blocked: 0,
                failed: 1,
                complete: 0,
            },
            entries: vec![
                FrontierSyncEntry {
                    sync_key: "frontier/zend/lane/client:implement".to_string(),
                    lane_key: "client:implement".to_string(),
                    unit_id: "client".to_string(),
                    unit_title: "Client".to_string(),
                    lane_id: "implement".to_string(),
                    lane_title: "Implement".to_string(),
                    lane_kind: "artifact".to_string(),
                    status: LaneExecutionStatus::Running,
                    detail: "running".to_string(),
                    current_run_id: Some("01RUN".to_string()),
                    last_run_id: Some("01RUN".to_string()),
                    current_stage: Some("Implement".to_string()),
                    current_stage_provider: None,
                    current_stage_cli_name: None,
                    time_in_current_stage_secs: None,
                    current_stage_idle_secs: None,
                    last_started_at: None,
                    last_finished_at: None,
                    last_exit_status: None,
                    last_usage_summary: None,
                    last_completed_stage: None,
                    last_stage_duration_ms: None,
                    last_stdout_snippet: None,
                    last_stderr_snippet: None,
                    failure_kind: None,
                    blocker_reason: None,
                    wake_command: "wake".to_string(),
                    route_command: "route".to_string(),
                    refresh_command: "refresh".to_string(),
                    artifact_paths: vec![],
                    artifact_statuses: vec![],
                    dependency_keys: vec![],
                    next_operator_move: "watch".to_string(),
                },
                FrontierSyncEntry {
                    sync_key: "frontier/zend/lane/wallet:implement".to_string(),
                    lane_key: "wallet:implement".to_string(),
                    unit_id: "wallet".to_string(),
                    unit_title: "Wallet".to_string(),
                    lane_id: "implement".to_string(),
                    lane_title: "Implement".to_string(),
                    lane_kind: "artifact".to_string(),
                    status: LaneExecutionStatus::Failed,
                    detail: "failed".to_string(),
                    current_run_id: None,
                    last_run_id: Some("01FAIL".to_string()),
                    current_stage: Some("Review".to_string()),
                    current_stage_provider: None,
                    current_stage_cli_name: None,
                    time_in_current_stage_secs: None,
                    current_stage_idle_secs: None,
                    last_started_at: None,
                    last_finished_at: None,
                    last_exit_status: Some(1),
                    last_usage_summary: None,
                    last_completed_stage: None,
                    last_stage_duration_ms: None,
                    last_stdout_snippet: None,
                    last_stderr_snippet: Some("boom".to_string()),
                    failure_kind: Some("proof_script_failure".to_string()),
                    blocker_reason: Some("tests failed".to_string()),
                    wake_command: "wake".to_string(),
                    route_command: "route".to_string(),
                    refresh_command: "refresh".to_string(),
                    artifact_paths: vec![],
                    artifact_statuses: vec![],
                    dependency_keys: vec![],
                    next_operator_move: "fix".to_string(),
                },
            ],
        };
        let plan_matrix = PlanMatrix {
            program: "zend".to_string(),
            rows: vec![
                PlanStatusRow {
                    plan_id: "craps".to_string(),
                    plan_file: "plans/005-craps-game.md".to_string(),
                    title: "Craps".to_string(),
                    category: "game".to_string(),
                    composite: true,
                    mapping_status: "mapped".to_string(),
                    child_count: 6,
                    represented_in_blueprint: true,
                    has_bootstrap_lane: true,
                    has_implementation_lane: false,
                    has_real_verify_gate: false,
                    current_status: "unmodeled".to_string(),
                    current_risk:
                        "mapped from plan structure, but synthesis has not rendered this composite plan yet"
                            .to_string(),
                    next_operator_move: "extend synthesis coverage for this plan".to_string(),
                },
                PlanStatusRow {
                    plan_id: "poker".to_string(),
                    plan_file: "plans/003-poker-game.md".to_string(),
                    title: "Poker".to_string(),
                    category: "game".to_string(),
                    composite: false,
                    mapping_status: "mapped".to_string(),
                    child_count: 1,
                    represented_in_blueprint: true,
                    has_bootstrap_lane: true,
                    has_implementation_lane: true,
                    has_real_verify_gate: true,
                    current_status: "implementation_running".to_string(),
                    current_risk: "implementing".to_string(),
                    next_operator_move: "wait".to_string(),
                },
            ],
        };
        let agents = vec![BundleAgent {
            slug: "client".to_string(),
            name: "Client".to_string(),
            adapter_type: "codex_local",
            metadata_type: None,
            unit: Some("client".to_string()),
            lane_key: Some("client:implement".to_string()),
            plan_id: None,
        }];

        let markdown =
            build_company_markdown("Zend", "desc", &frontier, Some(&plan_matrix), None, &agents);

        assert!(markdown.contains("## Plan Status Summary"));
        assert!(markdown.contains("## Plans Needing Attention"));
        assert!(markdown.contains("## Plans In Motion"));
        assert!(markdown.contains("## Plan Matrix"));
        assert!(markdown.contains("## Lane Sets"));
        assert!(markdown.contains("## Live Details"));
        assert!(markdown.contains("## Running"));
        assert!(markdown.contains("## Failed"));
    }

    #[test]
    fn render_root_issue_plan_document_prefers_plan_sections_when_matrix_present() {
        let frontier = FrontierSyncModel {
            program: "zend".to_string(),
            manifest_path: PathBuf::from("/tmp/zend/fabro/programs/zend.yaml"),
            state_path: PathBuf::from("/tmp/zend/.raspberry/zend-state.json"),
            wake_command: "wake".to_string(),
            route_command: "route".to_string(),
            refresh_command: "refresh".to_string(),
            status_command: "status".to_string(),
            summary: FrontierSummary {
                ready: 1,
                running: 0,
                blocked: 0,
                failed: 0,
                complete: 0,
            },
            entries: Vec::new(),
        };
        let plan_matrix = PlanMatrix {
            program: "zend".to_string(),
            rows: vec![PlanStatusRow {
                plan_id: "craps".to_string(),
                plan_file: "plans/005-craps-game.md".to_string(),
                title: "Craps".to_string(),
                category: "game".to_string(),
                composite: true,
                mapping_status: "mapped".to_string(),
                child_count: 6,
                represented_in_blueprint: true,
                has_bootstrap_lane: true,
                has_implementation_lane: false,
                has_real_verify_gate: false,
                current_status: "unmodeled".to_string(),
                current_risk:
                    "mapped from plan structure, but synthesis has not rendered this composite plan yet"
                        .to_string(),
                next_operator_move: "extend synthesis coverage for this plan".to_string(),
            }],
        };

        let rendered = render_root_issue_plan_document(&frontier, Some(&plan_matrix));

        assert!(rendered.contains("# Raspberry Plans"));
        assert!(rendered.contains("## Plan Status Summary"));
        assert!(rendered.contains("## Plan Matrix"));
        assert!(rendered.contains("## Frontier Summary"));
        assert!(rendered.contains("extend synthesis coverage for this plan"));
    }

    #[test]
    fn desired_root_work_products_use_attachment_urls() {
        let frontier = FrontierSyncModel {
            program: "zend".to_string(),
            manifest_path: PathBuf::from("/tmp/zend/fabro/programs/zend.yaml"),
            state_path: PathBuf::from("/tmp/zend/.raspberry/zend-state.json"),
            wake_command: "fabro paperclip wake --target-repo /tmp/zend --program zend --agent raspberry-orchestrator".to_string(),
            route_command: "bash fabro/paperclip/zend/scripts/raspberry-orchestrator.sh".to_string(),
            refresh_command: "fabro paperclip refresh --target-repo /tmp/zend --program zend".to_string(),
            status_command: "fabro paperclip status --target-repo /tmp/zend --program zend".to_string(),
            summary: FrontierSummary {
                ready: 0,
                running: 1,
                blocked: 0,
                failed: 0,
                complete: 0,
            },
            entries: Vec::new(),
        };
        let attachments = BTreeMap::from([
            (
                "zend-program-manifest.yaml".to_string(),
                PaperclipAttachment {
                    id: "att-1".to_string(),
                    original_filename: Some("zend-program-manifest.yaml".to_string()),
                    content_path: Some("/api/attachments/att-1/content".to_string()),
                    sha256: "abc".to_string(),
                },
            ),
            (
                "zend-program-state.json".to_string(),
                PaperclipAttachment {
                    id: "att-2".to_string(),
                    original_filename: Some("zend-program-state.json".to_string()),
                    content_path: Some("/api/attachments/att-2/content".to_string()),
                    sha256: "def".to_string(),
                },
            ),
        ]);

        let products =
            desired_root_work_products(&frontier, Some(&attachments), "http://127.0.0.1:3100");

        assert_eq!(products.len(), 2);
        assert_eq!(
            products[0].url.as_deref(),
            Some("http://127.0.0.1:3100/api/attachments/att-1/content")
        );
        assert_eq!(
            products[1].url.as_deref(),
            Some("http://127.0.0.1:3100/api/attachments/att-2/content")
        );
    }

    #[test]
    fn last_lines_returns_requested_tail() {
        let rendered = last_lines("a\nb\nc\nd\n", 2);

        assert_eq!(rendered, "c\nd\n");
    }

    #[tokio::test]
    async fn delete_issue_attachment_ignores_not_found() {
        let server = MockServer::start();
        let attachment_id = "missing-attachment";
        let _mock = server.mock(|when, then| {
            when.method(httpmock::Method::DELETE)
                .path(format!("/api/attachments/{attachment_id}"));
            then.status(404);
        });

        delete_issue_attachment(&server.base_url(), attachment_id)
            .await
            .expect("404 should be treated as already deleted");
    }

    #[test]
    fn ensure_target_repo_initialized_creates_initial_commit_for_plain_directory() {
        use git2::Repository;

        let temp = tempdir().expect("tempdir");
        std::fs::write(temp.path().join("README.md"), "# Demo\n").expect("readme");

        let result = ensure_target_repo_initialized(temp.path()).expect("git bootstrap");
        let repo = Repository::open(temp.path()).expect("repo opens");
        let head = repo.head().expect("head");
        let commit = head.peel_to_commit().expect("commit");

        assert!(result.initialized);
        assert!(result.committed);
        assert_eq!(head.name(), Some("refs/heads/main"));
        assert_eq!(
            commit.summary(),
            Some("chore(repo): bootstrap fabro workspace")
        );
    }

    #[test]
    fn ensure_target_repo_gitignore_adds_transient_runtime_entries() {
        let temp = tempdir().expect("tempdir");
        std::fs::write(temp.path().join(".gitignore"), "node_modules/\n").expect("gitignore");

        ensure_target_repo_gitignore(temp.path()).expect("seed gitignore");
        let body = std::fs::read_to_string(temp.path().join(".gitignore")).expect("read gitignore");

        assert!(body.contains("node_modules/"));
        assert!(body.contains(".raspberry/"));
        assert!(body.contains(".paperclip/"));
        assert!(body.contains("malinka/paperclip/*/bootstrap-state.json"));
    }

    #[test]
    fn prune_tracked_transient_paths_untracks_runtime_files() {
        use git2::Repository;

        let temp = tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join(".raspberry")).expect("raspberry dir");
        std::fs::create_dir_all(temp.path().join(".paperclip")).expect("paperclip dir");
        std::fs::create_dir_all(temp.path().join("malinka/paperclip/demo")).expect("bundle dir");
        std::fs::write(temp.path().join(".raspberry/state.json"), "{}").expect("state");
        std::fs::write(temp.path().join(".paperclip/.gitignore"), "*\n").expect("pc gitignore");
        std::fs::write(
            temp.path()
                .join("malinka/paperclip/demo/bootstrap-state.json"),
            "{}",
        )
        .expect("bootstrap state");

        let init = Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(temp.path())
            .output()
            .expect("git init");
        assert!(init.status.success());
        let add = Command::new("git")
            .args(["add", "-A"])
            .current_dir(temp.path())
            .output()
            .expect("git add");
        assert!(add.status.success());
        let commit = Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "seed tracked runtime state",
            ])
            .current_dir(temp.path())
            .output()
            .expect("git commit");
        assert!(commit.status.success());

        let cleanup = prune_tracked_transient_paths(temp.path()).expect("cleanup");
        let tracked = Command::new("git")
            .args(["ls-files"])
            .current_dir(temp.path())
            .output()
            .expect("ls-files");
        let tracked = String::from_utf8(tracked.stdout).expect("utf8");

        assert!(cleanup.removed_paths >= 2);
        assert!(!tracked.contains(".raspberry/state.json"));
        assert!(!tracked.contains(".paperclip/.gitignore"));
        assert!(!tracked.contains("malinka/paperclip/demo/bootstrap-state.json"));
        let _repo = Repository::open(temp.path()).expect("repo opens");
    }

    #[test]
    fn ensure_paperclip_blueprint_rerenders_package_after_git_bootstrap() {
        let temp = tempdir().expect("tempdir");
        std::fs::write(temp.path().join("README.md"), "# Zend\n").expect("readme");
        std::fs::write(temp.path().join("SPEC.md"), "# Root Spec\n").expect("spec");
        std::fs::create_dir_all(temp.path().join("plans")).expect("plans dir");
        std::fs::write(
            temp.path().join("plans/001-master-plan.md"),
            "# Zend Master Plan\n\nPhase 0:\n- [ ] Command Center Client (plan 003)\n",
        )
        .expect("master plan");
        std::fs::write(
            temp.path().join("plans/003-command-center-client.md"),
            "# Command Center Client\n",
        )
        .expect("leaf plan");
        let args = PaperclipRepoArgs {
            target_repo: temp.path().to_path_buf(),
            program: Some("zend".to_string()),
            company_name: None,
            data_dir: None,
            api_base: None,
            paperclip_cmd: None,
        };
        let paths = resolve_paperclip_paths(&args);
        ensure_paperclip_blueprint(&paths).expect("initial render");
        let run_config_path =
            walkdir::WalkDir::new(temp.path().join("malinka/run-configs/bootstrap"))
                .into_iter()
                .filter_map(Result::ok)
                .map(|entry| entry.path().to_path_buf())
                .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("toml"))
                .expect("bootstrap run config");
        let before = std::fs::read_to_string(&run_config_path).expect("initial run config");
        assert!(!before.contains("[integration]"));

        let bootstrap = ensure_target_repo_initialized(temp.path()).expect("git bootstrap");
        assert!(bootstrap.initialized);

        ensure_paperclip_blueprint(&paths).expect("rerender after git");
        let after = std::fs::read_to_string(&run_config_path).expect("updated run config");
        assert!(after.contains("[integration]"));
        assert!(after.contains("target_branch = \"main\""));
    }

    #[test]
    fn frontier_sync_unchanged_detects_runtime_progress() {
        let before = FrontierSyncModel {
            program: "zend".to_string(),
            manifest_path: PathBuf::from("/tmp/zend/fabro/programs/zend.yaml"),
            state_path: PathBuf::from("/tmp/zend/.raspberry/zend-state.json"),
            wake_command: "wake".to_string(),
            route_command: "route".to_string(),
            refresh_command: "refresh".to_string(),
            status_command: "status".to_string(),
            summary: FrontierSummary {
                ready: 1,
                running: 0,
                blocked: 0,
                failed: 0,
                complete: 0,
            },
            entries: vec![FrontierSyncEntry {
                sync_key: "frontier/zend/lane/client:implement".to_string(),
                lane_key: "client:implement".to_string(),
                unit_id: "client".to_string(),
                unit_title: "Client".to_string(),
                lane_id: "implement".to_string(),
                lane_title: "Implement".to_string(),
                lane_kind: "artifact".to_string(),
                status: LaneExecutionStatus::Ready,
                detail: "ready".to_string(),
                current_run_id: None,
                last_run_id: None,
                current_stage: None,
                current_stage_provider: None,
                current_stage_cli_name: None,
                time_in_current_stage_secs: None,
                current_stage_idle_secs: None,
                last_started_at: None,
                last_finished_at: None,
                last_exit_status: None,
                last_usage_summary: None,
                last_completed_stage: None,
                last_stage_duration_ms: None,
                last_stdout_snippet: None,
                last_stderr_snippet: None,
                failure_kind: None,
                blocker_reason: None,
                wake_command: "wake".to_string(),
                route_command: "route".to_string(),
                refresh_command: "refresh".to_string(),
                artifact_paths: Vec::new(),
                artifact_statuses: Vec::new(),
                dependency_keys: Vec::new(),
                next_operator_move: "wake".to_string(),
            }],
        };
        let mut after = before.clone();
        after.summary.ready = 0;
        after.summary.running = 1;
        after.entries[0].status = LaneExecutionStatus::Running;
        after.entries[0].current_run_id = Some("01KMTEST".to_string());

        assert!(!frontier_sync_unchanged(&before, &after));
        assert!(frontier_sync_unchanged(&before, &before));
    }

    #[test]
    fn stderr_indicates_active_autodev_controller_detects_lock_message() {
        assert!(stderr_indicates_active_autodev_controller(
            "",
            "Error: autodev controller already running for program `zend` via pid=123"
        ));
        assert!(!stderr_indicates_active_autodev_controller(
            "",
            "Error: failed to load manifest"
        ));
    }

    #[test]
    fn wake_followthrough_changed_detects_frontier_updates() {
        let before = FrontierSyncModel {
            program: "zend".to_string(),
            manifest_path: PathBuf::from("/tmp/zend/fabro/programs/zend.yaml"),
            state_path: PathBuf::from("/tmp/zend/.raspberry/zend-state.json"),
            wake_command: "wake".to_string(),
            route_command: "route".to_string(),
            refresh_command: "refresh".to_string(),
            status_command: "status".to_string(),
            summary: FrontierSummary {
                ready: 1,
                running: 0,
                blocked: 0,
                failed: 0,
                complete: 0,
            },
            entries: Vec::new(),
        };
        let mut after = before.clone();
        after.summary.ready = 0;
        after.summary.running = 1;

        assert!(wake_followthrough_changed(
            Some(&before),
            &WakeFollowthrough {
                status: WakeFollowthroughStatus::ObservedViaHeartbeat,
                after: Some(after),
            }
        ));
        assert!(!wake_followthrough_changed(
            Some(&before),
            &WakeFollowthrough {
                status: WakeFollowthroughStatus::NoObservedChange,
                after: Some(before.clone()),
            }
        ));
        assert!(!wake_followthrough_changed(
            Some(&before),
            &WakeFollowthrough {
                status: WakeFollowthroughStatus::NoObservedChange,
                after: None,
            }
        ));
    }

    #[test]
    fn write_orchestrator_script_prefers_runtime_overrides_with_built_binary_fallbacks() {
        let temp = tempdir().expect("tempdir");
        let script_path = temp.path().join("raspberry-orchestrator.sh");
        let target_repo = temp.path().join("repo");
        let manifest_path = target_repo.join("malinka/programs/demo.yaml");
        let fabro_binary = PathBuf::from("/opt/fabro/bin/fabro");
        let raspberry_binary = PathBuf::from("/opt/fabro/bin/raspberry");

        write_orchestrator_script(
            &script_path,
            &target_repo,
            &manifest_path,
            &fabro_binary,
            Some(&raspberry_binary),
        )
        .expect("write script");
        let body = std::fs::read_to_string(&script_path).expect("read script");

        assert!(body.contains("FABRO_BIN"));
        assert!(body.contains("RASPBERRY_BIN"));
        assert!(body.contains("/opt/fabro/bin/fabro"));
        assert!(body.contains("/opt/fabro/bin/raspberry"));
        assert!(body.contains("command -v fabro"));
        assert!(body.contains("command -v raspberry"));
    }

    #[test]
    fn write_paperclip_cli_script_uses_repo_scoped_wrapper() {
        let temp = tempdir().expect("tempdir");
        let script_path = temp.path().join("fabro-paperclip.sh");
        let fabro_binary = PathBuf::from("/opt/fabro/bin/fabro");
        let target_repo = temp.path().join("repo");

        write_paperclip_cli_script(&script_path, &fabro_binary, &target_repo, "demo")
            .expect("write wrapper");
        let body = std::fs::read_to_string(&script_path).expect("read wrapper");

        assert!(body.contains("FABRO_BIN"));
        assert!(body.contains("paperclip \"$@\""));
        assert!(body.contains(&target_repo.display().to_string()));
        assert!(body.contains("--program demo"));
    }

    #[test]
    fn write_run_script_prefers_env_and_path_before_repo_fallback() {
        let temp = tempdir().expect("tempdir");
        let script_path = temp.path().join("run-paperclip.sh");
        let data_dir = temp.path().join(".paperclip");
        let paperclip_repo = PathBuf::from("/home/r/coding/paperclip");

        write_run_script(&script_path, Some(paperclip_repo.clone()), &data_dir)
            .expect("write run script");
        let body = std::fs::read_to_string(&script_path).expect("read script");

        assert!(body.contains("PAPERCLIP_CMD"));
        assert!(body.contains("command -v paperclipai"));
        assert!(body.contains("PAPERCLIP_REPO"));
        assert!(body.contains(&paperclip_repo.display().to_string()));
    }

    #[test]
    fn generated_agent_markdown_avoids_stale_frontier_snapshots() {
        let temp = tempdir().expect("tempdir");
        let blueprint = sample_blueprint();
        let mission = BootstrapMission {
            company_description: "Zend company".to_string(),
            goal_title: "Advance Zend".to_string(),
            goal_description: "Advance Zend honestly.".to_string(),
            project_name: "Zend Workspace".to_string(),
            project_description: "Repo workspace".to_string(),
            workspace_name: "Zend repo".to_string(),
        };
        let frontier = FrontierSyncModel {
            program: "zend".to_string(),
            manifest_path: temp.path().join("malinka/programs/zend.yaml"),
            state_path: temp.path().join(".raspberry/zend-state.json"),
            wake_command: "wake".to_string(),
            route_command: "route".to_string(),
            refresh_command: "refresh".to_string(),
            status_command: "status".to_string(),
            summary: FrontierSummary {
                ready: 2,
                running: 1,
                blocked: 0,
                failed: 3,
                complete: 4,
            },
            entries: vec![FrontierSyncEntry {
                sync_key: "frontier/zend/lane/command-center-client:command-center-client"
                    .to_string(),
                lane_key: "command-center-client:command-center-client".to_string(),
                unit_id: "command-center-client".to_string(),
                unit_title: "Command Center Client".to_string(),
                lane_id: "command-center-client".to_string(),
                lane_title: "Command Center Client".to_string(),
                lane_kind: "artifact".to_string(),
                status: LaneExecutionStatus::Running,
                detail: "run active at stage `Specify`".to_string(),
                current_run_id: Some("01KMTEST".to_string()),
                last_run_id: Some("01KMTEST".to_string()),
                current_stage: Some("Specify".to_string()),
                current_stage_provider: None,
                current_stage_cli_name: None,
                time_in_current_stage_secs: None,
                current_stage_idle_secs: None,
                last_started_at: None,
                last_finished_at: None,
                last_exit_status: None,
                last_usage_summary: None,
                last_completed_stage: None,
                last_stage_duration_ms: None,
                last_stdout_snippet: None,
                last_stderr_snippet: None,
                failure_kind: None,
                blocker_reason: None,
                wake_command: "wake".to_string(),
                route_command: "route".to_string(),
                refresh_command: "refresh".to_string(),
                artifact_paths: vec!["command-center-client/plan.md".to_string()],
                artifact_statuses: vec!["missing: command-center-client/plan.md".to_string()],
                dependency_keys: Vec::new(),
                next_operator_move: "refresh".to_string(),
            }],
        };

        let bundle = build_company_bundle(
            &blueprint,
            temp.path(),
            "Zend",
            &mission,
            &temp
                .path()
                .join("malinka/paperclip/zend/scripts/raspberry-orchestrator.sh"),
            &temp
                .path()
                .join("malinka/paperclip/zend/scripts/fabro-agent-minimax.sh"),
            &frontier,
            None,
            None,
        )
        .expect("build bundle");
        let orchestrator_markdown = bundle
            .agent_markdowns
            .iter()
            .find(|(path, _)| path.ends_with("raspberry-orchestrator/AGENTS.md"))
            .map(|(_, markdown)| markdown)
            .expect("orchestrator markdown");
        assert!(!orchestrator_markdown.contains("- ready:"));
        assert!(orchestrator_markdown.contains("Live frontier inspection:"));
        assert!(orchestrator_markdown.contains("Treat live state as volatile"));
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

    fn sample_plan_dashboard() -> PlanDashboardModel {
        PlanDashboardModel {
            program: "rxmragent".to_string(),
            plans: vec![
                PlanDashboardEntry {
                    plan_id: "casino-core".to_string(),
                    title: "Casino Core Trait".to_string(),
                    category: "foundation".to_string(),
                    composite: true,
                    path: "plans/016-casino-core-trait.md".to_string(),
                    status: "bootstrap_ready".to_string(),
                    current_stage: Some("Implement".to_string()),
                    current_run_id: Some("01TESTRUN".to_string()),
                    risk: "foundation plan".to_string(),
                    next_move: "dispatch bootstrap lane".to_string(),
                    mapping_source: "contract".to_string(),
                    children: vec![PlanDashboardChild {
                        child_id: "trait-def".to_string(),
                        title: "Trait Definition".to_string(),
                        archetype: Some("implement".to_string()),
                        review_profile: Some("foundation".to_string()),
                        owned_surfaces: vec!["crates/casino-core/src/lib.rs".to_string()],
                        sync_key: "plan/rxmragent/casino-core/trait-def".to_string(),
                        status: "bootstrap_ready".to_string(),
                        current_stage: Some("Implement".to_string()),
                        current_run_id: Some("01TESTRUN".to_string()),
                        next_operator_move: "dispatch bootstrap lane".to_string(),
                    }],
                },
                PlanDashboardEntry {
                    plan_id: "craps".to_string(),
                    title: "Craps Game".to_string(),
                    category: "game".to_string(),
                    composite: true,
                    path: "plans/005-craps-game.md".to_string(),
                    status: "unmodeled".to_string(),
                    current_stage: Some("Review".to_string()),
                    current_run_id: Some("01CRAPSRUN".to_string()),
                    risk: "evidence only".to_string(),
                    next_move: "extend synthesis coverage".to_string(),
                    mapping_source: "inferred".to_string(),
                    children: vec![
                        PlanDashboardChild {
                            child_id: "craps-engine".to_string(),
                            title: "Craps Engine".to_string(),
                            archetype: Some("implement".to_string()),
                            review_profile: Some("hardened".to_string()),
                            owned_surfaces: vec!["crates/casino-core/src/craps/".to_string()],
                            sync_key: "plan/rxmragent/craps/craps-engine".to_string(),
                            status: "implementation_running".to_string(),
                            current_stage: Some("Review".to_string()),
                            current_run_id: Some("01CRAPSRUN".to_string()),
                            next_operator_move: "watch".to_string(),
                        },
                        PlanDashboardChild {
                            child_id: "craps-pf".to_string(),
                            title: "Craps Provably Fair".to_string(),
                            archetype: Some("implement".to_string()),
                            review_profile: Some("hardened".to_string()),
                            owned_surfaces: vec!["crates/provably-fair/src/craps.rs".to_string()],
                            sync_key: "plan/rxmragent/craps/craps-pf".to_string(),
                            status: "unmodeled".to_string(),
                            current_stage: None,
                            current_run_id: None,
                            next_operator_move: "extend synthesis coverage".to_string(),
                        },
                    ],
                },
                PlanDashboardEntry {
                    plan_id: "master".to_string(),
                    title: "Master Plan".to_string(),
                    category: "meta".to_string(),
                    composite: false,
                    path: "plans/001-master-plan.md".to_string(),
                    status: "meta_plan".to_string(),
                    current_stage: None,
                    current_run_id: None,
                    risk: "coordination plan".to_string(),
                    next_move: "inspect downstream".to_string(),
                    mapping_source: "inferred".to_string(),
                    children: vec![],
                },
            ],
            summary: PlanDashboardSummary {
                total: 3,
                represented: 1,
                in_motion: 1,
                needs_attention: 1,
                complete: 0,
            },
        }
    }

    #[test]
    fn plan_sync_key_format() {
        assert_eq!(
            plan_root_sync_key("rxmragent", "craps"),
            "plan/rxmragent/craps"
        );
        assert_eq!(
            plan_child_sync_key("rxmragent", "craps", "craps-engine"),
            "plan/rxmragent/craps/craps-engine"
        );
    }

    #[test]
    fn build_plan_dashboard_model_aggregates_registry_and_matrix() {
        use raspberry_supervisor::plan_registry::*;
        use std::path::PathBuf;

        let registry = PlanRegistry {
            plans: vec![PlanRecord {
                plan_id: "craps".to_string(),
                path: PathBuf::from("plans/005-craps-game.md"),
                title: "Craps Game".to_string(),
                category: PlanCategory::Game,
                composite: true,
                dependency_plan_ids: vec!["casino-core".to_string()],
                mapping_contract_path: None,
                mapping_source: PlanMappingSource::Inferred,
                bootstrap_required: true,
                implementation_required: true,
                declared_child_ids: vec!["craps-engine".to_string(), "craps-pf".to_string()],
                children: vec![PlanChildRecord {
                    child_id: "craps-engine".to_string(),
                    title: Some("Craps Engine".to_string()),
                    archetype: Some(WorkflowArchetype::Implement),
                    lane_kind: None,
                    review_profile: Some(ReviewProfile::Hardened),
                    proof_commands: vec![],
                    owned_surfaces: vec!["crates/casino-core/src/craps/".to_string()],
                    where_surfaces: None,
                    how_description: None,
                    state_artifacts: None,
                    required_tests: None,
                    verification_plan: None,
                    rollback_condition: None,
                }],
            }],
        };
        let matrix = PlanMatrix {
            program: "rxmragent".to_string(),
            rows: vec![PlanStatusRow {
                plan_id: "craps".to_string(),
                plan_file: "plans/005-craps-game.md".to_string(),
                title: "Craps Game".to_string(),
                category: "game".to_string(),
                composite: true,
                mapping_status: "mapped".to_string(),
                child_count: 2,
                represented_in_blueprint: false,
                has_bootstrap_lane: false,
                has_implementation_lane: false,
                has_real_verify_gate: false,
                current_status: "unmodeled".to_string(),
                current_risk: "evidence only".to_string(),
                next_operator_move: "extend synthesis".to_string(),
            }],
        };
        let frontier = FrontierSyncModel {
            program: "rxmragent".to_string(),
            manifest_path: PathBuf::from("malinka/programs/rxmragent.yaml"),
            state_path: PathBuf::from(".raspberry/rxmragent-state.json"),
            route_command: "bash route.sh".to_string(),
            wake_command: "fabro paperclip wake".to_string(),
            refresh_command: "fabro paperclip refresh".to_string(),
            status_command: "fabro paperclip status".to_string(),
            summary: FrontierSummary {
                ready: 0,
                running: 1,
                blocked: 0,
                failed: 0,
                complete: 0,
            },
            entries: vec![FrontierSyncEntry {
                sync_key: "frontier/rxmragent/lane/craps-engine:implement".to_string(),
                lane_key: "craps-engine:implement".to_string(),
                unit_id: "craps-engine".to_string(),
                unit_title: "Craps Engine".to_string(),
                lane_id: "implement".to_string(),
                lane_title: "Implement".to_string(),
                lane_kind: "platform".to_string(),
                status: LaneExecutionStatus::Running,
                detail: "run active at stage `Review`".to_string(),
                current_run_id: Some("01CRAPSRUN".to_string()),
                last_run_id: Some("01CRAPSRUN".to_string()),
                current_stage: Some("Review".to_string()),
                current_stage_provider: None,
                current_stage_cli_name: None,
                time_in_current_stage_secs: None,
                current_stage_idle_secs: None,
                last_started_at: None,
                last_finished_at: None,
                last_exit_status: None,
                last_usage_summary: None,
                last_completed_stage: Some("Implement".to_string()),
                last_stage_duration_ms: Some(42),
                last_stdout_snippet: None,
                last_stderr_snippet: None,
                failure_kind: None,
                blocker_reason: None,
                wake_command: "fabro paperclip wake".to_string(),
                route_command: "bash route.sh".to_string(),
                refresh_command: "fabro paperclip refresh".to_string(),
                artifact_paths: vec![],
                artifact_statuses: vec![],
                dependency_keys: vec![],
                next_operator_move: "watch".to_string(),
            }],
        };

        let dashboard =
            build_plan_dashboard_model("rxmragent", &registry, Some(&matrix), &frontier);

        assert_eq!(dashboard.summary.total, 1);
        assert_eq!(dashboard.summary.needs_attention, 1);
        assert_eq!(dashboard.plans.len(), 1);
        assert_eq!(dashboard.plans[0].plan_id, "craps");
        assert_eq!(dashboard.plans[0].status, "unmodeled");
        assert_eq!(dashboard.plans[0].children.len(), 1);
        assert_eq!(dashboard.plans[0].children[0].child_id, "craps-engine");
        assert_eq!(
            dashboard.plans[0].children[0].sync_key,
            "plan/rxmragent/craps/craps-engine"
        );
        assert_eq!(dashboard.plans[0].children[0].status, "running");
        assert_eq!(
            dashboard.plans[0].children[0].current_stage.as_deref(),
            Some("Review")
        );
        assert_eq!(
            dashboard.plans[0].children[0].current_run_id.as_deref(),
            Some("01CRAPSRUN")
        );
    }

    #[test]
    fn build_company_markdown_plans_primary_when_dashboard_present() {
        let frontier = FrontierSyncModel {
            program: "rxmragent".to_string(),
            manifest_path: PathBuf::from("malinka/programs/rxmragent.yaml"),
            state_path: PathBuf::from(".raspberry/rxmragent-state.json"),
            route_command: "bash route.sh".to_string(),
            wake_command: "fabro paperclip wake".to_string(),
            refresh_command: "fabro paperclip refresh".to_string(),
            status_command: "fabro paperclip status".to_string(),
            summary: FrontierSummary {
                ready: 1,
                running: 0,
                blocked: 0,
                failed: 0,
                complete: 0,
            },
            entries: vec![],
        };
        let dashboard = sample_plan_dashboard();

        let md = build_company_markdown(
            "rXMRbro",
            "Monero casino",
            &frontier,
            None,
            Some(&dashboard),
            &[],
        );

        // Plans section appears before frontier
        let plans_pos = md.find("## Foundation Plans").expect("foundation section");
        let frontier_pos = md
            .find("# Frontier (Lane Detail)")
            .expect("frontier section");
        assert!(
            plans_pos < frontier_pos,
            "plans must appear before frontier"
        );
        assert!(md.contains("`casino-core`:"));
        assert!(md.contains("## Game Plans"));
        assert!(md.contains("`craps`:"));
        // Meta plans should not appear in category sections
        assert!(!md.contains("## Meta Plans"));
    }

    #[test]
    fn desired_plan_root_issue_uses_plan_sync_key() {
        let dashboard = sample_plan_dashboard();
        let plan = &dashboard.plans[0]; // casino-core

        let issue =
            desired_plan_root_issue("rxmragent", plan, "root-issue-id", None, "workspace-id");

        assert_eq!(issue.title, "Plan: Casino Core Trait");
        assert!(issue.description.contains("plan/rxmragent/casino-core"));
        assert!(issue
            .description
            .contains("fabro.paperclip.sync-key: plan/rxmragent/casino-core"));
        assert_eq!(issue.parent_id, Some("root-issue-id".to_string()));
    }

    #[test]
    fn plan_snapshots_detect_status_transition() {
        let mut dashboard = sample_plan_dashboard();
        let snap1 = plan_snapshots(&dashboard);

        // Mutate status to simulate transition
        dashboard.plans[0].status = "bootstrap_running".to_string();
        let snap2 = plan_snapshots(&dashboard);

        let key = plan_root_sync_key("rxmragent", "casino-core");
        let prev = snap1.get(&key);
        let curr = snap2.get(&key).expect("current snapshot");
        assert!(snapshot_changed(prev, curr));

        // Meta plans are now included in plan-level Paperclip reporting.
        let meta_key = plan_root_sync_key("rxmragent", "master");
        assert!(snap1.get(&meta_key).is_some());
    }

    #[test]
    fn build_company_bundle_includes_plan_root_agents() {
        let blueprint = sample_blueprint();
        let frontier = FrontierSyncModel {
            program: "rxmragent".to_string(),
            manifest_path: PathBuf::from("malinka/programs/rxmragent.yaml"),
            state_path: PathBuf::from(".raspberry/rxmragent-state.json"),
            route_command: "bash route.sh".to_string(),
            wake_command: "fabro paperclip wake".to_string(),
            refresh_command: "fabro paperclip refresh".to_string(),
            status_command: "fabro paperclip status".to_string(),
            summary: FrontierSummary {
                ready: 0,
                running: 0,
                blocked: 0,
                failed: 0,
                complete: 0,
            },
            entries: vec![],
        };
        let dashboard = sample_plan_dashboard();
        let temp = tempdir().expect("tempdir");

        let bundle = build_company_bundle(
            &blueprint,
            temp.path(),
            "Test Co",
            &BootstrapMission {
                goal_title: "Test".to_string(),
                goal_description: "Test goal".to_string(),
                project_name: "Test Project".to_string(),
                project_description: "Test desc".to_string(),
                workspace_name: "test-ws".to_string(),
                company_description: "Test company".to_string(),
            },
            &temp.path().join("orchestrator.sh"),
            &temp.path().join("fabro-agent-minimax.sh"),
            &frontier,
            None,
            Some(&dashboard),
        )
        .expect("bundle");

        let plan_agents: Vec<&BundleAgent> = bundle
            .agents
            .iter()
            .filter(|a| a.plan_id.is_some())
            .collect();
        assert!(plan_agents.is_empty());
    }

    #[test]
    fn plan_status_to_issue_status_maps_correctly() {
        assert_eq!(plan_status_to_issue_status("bootstrap_ready"), "todo");
        assert_eq!(
            plan_status_to_issue_status("implementation_running"),
            "in_progress"
        );
        assert_eq!(plan_status_to_issue_status("bootstrap_failed"), "blocked");
        assert_eq!(plan_status_to_issue_status("bootstrap_blocked"), "blocked");
        assert_eq!(plan_status_to_issue_status("reviewed"), "done");
        assert_eq!(
            plan_status_to_issue_status("implementation_complete"),
            "done"
        );
        assert_eq!(plan_status_to_issue_status("unmodeled"), "todo");
        assert_eq!(plan_status_to_issue_status("meta_plan"), "todo");
    }

    #[test]
    fn desired_plan_child_issue_includes_sync_key_and_surfaces() {
        let dashboard = sample_plan_dashboard();
        let plan = &dashboard.plans[1]; // craps
        let child = &plan.children[0]; // craps-engine

        let issue = desired_plan_child_issue(plan, child, "plan-issue-id", None, "workspace-id");

        assert_eq!(issue.title, "Plan: Craps Game / Craps Engine");
        assert!(issue
            .description
            .contains("plan/rxmragent/craps/craps-engine"));
        assert!(issue.description.contains("hardened"));
        assert!(issue.description.contains("crates/casino-core/src/craps/"));
        assert_eq!(issue.parent_id, Some("plan-issue-id".to_string()));
    }

    #[test]
    fn is_syncable_excludes_meta_plans() {
        let dashboard = sample_plan_dashboard();
        assert!(dashboard.plans[0].is_syncable()); // casino-core (foundation)
        assert!(dashboard.plans[1].is_syncable()); // craps (game)
        assert!(!dashboard.plans[2].is_syncable()); // master (meta)
    }
}
