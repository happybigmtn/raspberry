use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
    evaluate::evaluate_with_state, refresh_program_state, EvaluatedLane, LaneExecutionStatus,
    ProgramManifest, ProgramRuntimeState,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;

const SYNC_MARKER_PREFIX: &str = "fabro.paperclip.sync-key:";

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
}

#[derive(Debug, Args, Clone)]
pub struct PaperclipLogsArgs {
    #[command(flatten)]
    pub repo: PaperclipRepoArgs,
    #[arg(long, default_value_t = 80)]
    pub lines: usize,
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
    orchestrator_script_path: PathBuf,
    run_script_path: PathBuf,
    bootstrap_state_path: PathBuf,
}

struct PaperclipRepoContext {
    paths: PaperclipPaths,
    mission: BootstrapMission,
    frontier: FrontierSyncModel,
    bundle: GeneratedBundle,
}

struct PaperclipApplySummary {
    company_id: String,
    goal_id: String,
    project_id: String,
    workspace_id: String,
    synced_issue_count: usize,
}

struct PaperclipServerStatus {
    pid: Option<u32>,
    pid_live: bool,
    server_ready: bool,
    bootstrap_state: Option<serde_json::Value>,
    frontier: Option<FrontierSyncModel>,
    openai_api_key_present: bool,
    anthropic_api_key_present: bool,
    local_cli_export_count: usize,
}

pub async fn bootstrap_command(args: &PaperclipBootstrapArgs) -> Result<()> {
    let context = prepare_paperclip_context(&args.repo)?;
    print_paperclip_context_summary(&context.paths, &context.mission.goal_title);
    if !args.apply {
        println!("Applied: no");
        return Ok(());
    }
    let summary = apply_paperclip_context(&context, args.repo.paperclip_cmd.as_deref()).await?;
    print_paperclip_apply_summary("Applied", &summary);
    Ok(())
}

pub async fn refresh_command(args: &PaperclipRefreshArgs) -> Result<()> {
    let context = prepare_paperclip_context(&args.repo)?;
    print_paperclip_context_summary(&context.paths, &context.mission.goal_title);
    let summary = apply_paperclip_context(&context, args.repo.paperclip_cmd.as_deref()).await?;
    print_paperclip_apply_summary("Refreshed", &summary);
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
    let bundle_root = target_repo
        .join("fabro")
        .join("paperclip")
        .join(&program_id);
    let scripts_root = bundle_root.join("scripts");
    let blueprint_path = target_repo
        .join("fabro")
        .join("blueprints")
        .join(format!("{program_id}.yaml"));
    let manifest_path = target_repo
        .join("fabro")
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
        orchestrator_script_path: scripts_root.join("raspberry-orchestrator.sh"),
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
    let raspberry_command = raspberry_command();
    write_orchestrator_script(
        &paths.orchestrator_script_path,
        &paths.target_repo,
        &paths.manifest_path,
        &fabro_binary,
        &raspberry_command,
    )?;
    write_run_script(
        &paths.run_script_path,
        &paperclip_command(args.paperclip_cmd.as_deref()),
        &paths.data_dir,
    )?;

    let mission = derive_bootstrap_mission(&blueprint, &paths.target_repo, &paths.company_name)?;
    let frontier = load_frontier_sync_model(
        &paths.program_id,
        &paths.target_repo,
        &paths.manifest_path,
        &paths.orchestrator_script_path,
        &paperclip_refresh_command(&paths),
        &paperclip_status_command(&paths),
    )?;
    let bundle = build_company_bundle(
        &blueprint,
        &paths.target_repo,
        &paths.company_name,
        &mission,
        &paths.orchestrator_script_path,
        &frontier,
    )?;
    write_bundle(&paths.bundle_root, &bundle)?;

    Ok(PaperclipRepoContext {
        paths,
        mission,
        frontier,
        bundle,
    })
}

fn ensure_paperclip_blueprint(paths: &PaperclipPaths) -> Result<ProgramBlueprint> {
    let blueprint = if paths.blueprint_path.exists() {
        load_blueprint(&paths.blueprint_path)?
    } else {
        let authored = author_blueprint_for_create(&paths.target_repo, Some(&paths.program_id))?;
        save_blueprint(&paths.blueprint_path, &authored.blueprint)?;
        render_blueprint(RenderRequest {
            blueprint: &authored.blueprint,
            target_repo: &paths.target_repo,
        })?;
        authored.blueprint
    };
    if !paths.manifest_path.exists() {
        render_blueprint(RenderRequest {
            blueprint: &blueprint,
            target_repo: &paths.target_repo,
        })?;
    }
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
    let import_result = import_company_package(
        &paperclip_cmd,
        &paths.data_dir,
        &paths.api_base,
        &paths.bundle_root,
        &paths.company_name,
        existing_company_id.as_deref(),
    )?;
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
        &context.frontier,
        &context.bundle.agents,
        &import_result.agents,
        existing_state.as_ref(),
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
        "routeCommand": context.frontier.route_command.clone(),
        "refreshCommand": context.frontier.refresh_command.clone(),
        "statusCommand": context.frontier.status_command.clone(),
        "summary": context.frontier.summary.clone(),
        "issueIds": synced_issue_ids,
        "updatedAt": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    });
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
    })
}

fn print_paperclip_context_summary(paths: &PaperclipPaths, goal_title: &str) {
    println!("Program: {}", paths.program_id);
    println!("Paperclip bundle: {}", paths.bundle_root.display());
    println!("Data dir: {}", paths.data_dir.display());
    println!("API base: {}", paths.api_base);
    println!("Company goal: {goal_title}");
}

fn print_paperclip_apply_summary(label: &str, summary: &PaperclipApplySummary) {
    println!("{label}: yes");
    println!("Company ID: {}", summary.company_id);
    println!("Goal ID: {}", summary.goal_id);
    println!("Project ID: {}", summary.project_id);
    println!("Workspace ID: {}", summary.workspace_id);
    println!("Synced issues: {}", summary.synced_issue_count);
}

async fn collect_paperclip_server_status(paths: &PaperclipPaths) -> Result<PaperclipServerStatus> {
    let pid = read_pid_file(&paths.data_dir.join("server.pid"))?;
    let bootstrap_state = load_bootstrap_state(&paths.bootstrap_state_path).ok();
    let frontier = if paths.manifest_path.exists() {
        load_frontier_sync_model(
            &paths.program_id,
            &paths.target_repo,
            &paths.manifest_path,
            &paths.orchestrator_script_path,
            &paperclip_refresh_command(paths),
            &paperclip_status_command(paths),
        )
        .ok()
    } else {
        None
    };

    Ok(PaperclipServerStatus {
        pid,
        pid_live: pid.map(process_is_running).unwrap_or(false),
        server_ready: paperclip_server_ready(&paths.api_base).await,
        bootstrap_state,
        frontier,
        openai_api_key_present: std::env::var_os("OPENAI_API_KEY").is_some(),
        anthropic_api_key_present: std::env::var_os("ANTHROPIC_API_KEY").is_some(),
        local_cli_export_count: count_local_cli_exports(&paths.data_dir),
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
        "OPENAI_API_KEY present: {}",
        yes_no(status.openai_api_key_present)
    );
    println!(
        "ANTHROPIC_API_KEY present: {}",
        yes_no(status.anthropic_api_key_present)
    );
    println!("Local CLI exports: {}", status.local_cli_export_count);
    if let Some(frontier) = status.frontier.as_ref() {
        println!("Frontier summary:");
        println!("{}", render_frontier_summary(frontier));
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

#[derive(Debug, Clone)]
struct FrontierSyncModel {
    program: String,
    manifest_path: PathBuf,
    state_path: PathBuf,
    route_command: String,
    refresh_command: String,
    status_command: String,
    summary: FrontierSummary,
    entries: Vec<FrontierSyncEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct FrontierSummary {
    ready: usize,
    running: usize,
    blocked: usize,
    failed: usize,
    complete: usize,
}

#[derive(Debug, Clone)]
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
    route_command: String,
    artifact_paths: Vec<String>,
    artifact_statuses: Vec<String>,
    dependency_keys: Vec<String>,
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
}

#[derive(Debug, Clone)]
struct PaperclipProjectSync {
    project: PaperclipProject,
    workspace: PaperclipWorkspace,
}

struct BundleAgent {
    slug: String,
    name: String,
    adapter_type: &'static str,
    metadata_type: Option<&'static str>,
    unit: Option<String>,
    lane_key: Option<String>,
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

fn build_company_bundle(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
    company_name: &str,
    mission: &BootstrapMission,
    orchestrator_script: &Path,
    frontier: &FrontierSyncModel,
) -> Result<GeneratedBundle> {
    let description = mission.company_description.clone();
    let mut manifest_agents = Vec::new();
    let mut agent_markdowns = Vec::new();
    let mut agents = Vec::new();
    push_bundle_agent(
        &mut manifest_agents,
        &mut agent_markdowns,
        &mut agents,
        mission_ceo_draft(blueprint, target_repo, mission),
    );
    push_bundle_agent(
        &mut manifest_agents,
        &mut agent_markdowns,
        &mut agents,
        orchestrator_draft(blueprint, target_repo, orchestrator_script, frontier),
    );
    for unit in &blueprint.units {
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
                lane_agent_draft(
                    blueprint,
                    target_repo,
                    mission,
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
    let company_markdown = build_company_markdown(company_name, &description, frontier, &agents);

    Ok(GeneratedBundle {
        manifest,
        company_markdown,
        agent_markdowns,
        agents,
    })
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
) -> BundleAgentDraft {
    let slug = "mission-ceo";
    let name = "Mission CEO".to_string();
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
        }),
        relative_path: format!("agents/{slug}/AGENTS.md"),
        markdown: build_agent_markdown(
            &name,
            slug,
            "ceo",
            format!(
                "You own the company mission for `{}`.\n\nCompany goal:\n- {}\n\nPriorities:\n- keep work aligned to the repo blueprint\n- promote lane decomposition that matches the real plan\n- route execution through Raspberry rather than bypassing it\n- prefer honest progress over optimistic summaries\n",
                blueprint.program.id, mission.goal_title,
            ),
        ),
        agent: BundleAgent {
            slug: slug.to_string(),
            name,
            adapter_type: "claude_local",
            metadata_type: Some("mission_ceo"),
            unit: None,
            lane_key: None,
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
        }),
        relative_path: format!("agents/{slug}/AGENTS.md"),
        markdown: build_agent_markdown(
            &name,
            slug,
            "pm",
            format!(
                "You operate the repo-local Raspberry control plane for `{}`.\n\nCurrent frontier:\n{}\n\nExecution route:\n- Run `{}` to let Raspberry evaluate and advance the frontier.\n- Refresh Paperclip with `{}` after package or frontier changes.\n- Do not create parallel execution flows outside Raspberry.\n- Use Paperclip for coordination, escalation, review, and handoff only.\n",
                blueprint.program.id,
                render_frontier_summary(frontier),
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
        },
    }
}

fn lane_agent_draft(
    blueprint: &ProgramBlueprint,
    target_repo: &Path,
    mission: &BootstrapMission,
    frontier: &FrontierSyncModel,
    unit: &BlueprintUnit,
    lane: &fabro_synthesis::BlueprintLane,
    frontier_entry: Option<&FrontierSyncEntry>,
) -> BundleAgentDraft {
    let slug = lane_agent_slug(unit, lane);
    let name = lane_agent_name(unit, lane);
    let role = lane_role(unit, lane);
    let adapter_type = lane_adapter_type(unit, lane);
    let lane_key = format!("{}:{}", unit.id, lane.id);
    let model = if adapter_type == "claude_local" {
        json!("claude-sonnet-4-6")
    } else {
        json!("gpt-5.3-codex")
    };
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
                "laneId": lane.id,
                "laneKey": lane_key.clone(),
                "family": lane.family,
                "unit": unit.id
            }
        }),
        relative_path: format!("agents/{slug}/AGENTS.md"),
        markdown: build_agent_markdown(
            &name,
            &slug,
            role,
            format!(
                "You coordinate the `{}` frontier in repo `{}`.\n\nCompany goal:\n{}\n\nLane goal:\n{}\n\nCurrent frontier state:\n{}\n\nArtifacts:\n{}\n\nDependencies:\n{}\n\nExecution route:\n- Run `{}` when this frontier needs Raspberry to evaluate or advance work.\n- Refresh Paperclip with `{}` after route execution or package changes.\n- Use Paperclip to triage, review, escalate, and explain blockers.\n- Do not bypass Raspberry with direct ad hoc execution.\n",
                lane.id,
                blueprint.program.id,
                mission.goal_title,
                lane.goal,
                render_frontier_entry(frontier_entry),
                lane_artifact_block(frontier_entry, unit),
                lane_dependency_block(frontier_entry, lane),
                frontier.route_command,
                frontier.refresh_command,
            ),
        ),
        agent: BundleAgent {
            slug,
            name,
            adapter_type,
            metadata_type: None,
            unit: Some(unit.id.clone()),
            lane_key: Some(lane_key),
        },
    }
}

fn lane_agent_slug(unit: &BlueprintUnit, lane: &fabro_synthesis::BlueprintLane) -> String {
    if unit.lanes.len() == 1 {
        return unit.id.clone();
    }
    format!("{}--{}", unit.id, lane.id)
}

fn lane_agent_name(unit: &BlueprintUnit, lane: &fabro_synthesis::BlueprintLane) -> String {
    if unit.lanes.len() == 1 {
        return unit.title.clone();
    }
    format!("{} / {}", unit.title, lane.title)
}

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

fn build_company_markdown(
    name: &str,
    description: &str,
    frontier: &FrontierSyncModel,
    agents: &[BundleAgent],
) -> String {
    let mut body = format!(
        "# {}\n\n{}\n\n# Frontier\n\n{}\n\n# Agents\n",
        name,
        description,
        render_frontier_summary(frontier),
    );
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

fn load_frontier_sync_model(
    program_id: &str,
    target_repo: &Path,
    manifest_path: &Path,
    orchestrator_script: &Path,
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
        refresh_command: refresh_command.to_string(),
        status_command: status_command.to_string(),
        summary: summarize_frontier(&program),
        entries: build_frontier_entries(
            program_id,
            &manifest,
            manifest_path,
            &program,
            &route_command,
            refresh_command,
        ),
    })
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
        route_command: route_command.to_string(),
        artifact_paths,
        artifact_statuses,
        dependency_keys,
        next_operator_move: next_operator_move_for_lane(lane, route_command, refresh_command),
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
    route_command: &str,
    refresh_command: &str,
) -> String {
    match lane.status {
        LaneExecutionStatus::Ready => format!(
            "Run `{route_command}` to let Raspberry dispatch ready work, then `{refresh_command}` to refresh Paperclip."
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
            "Resolve the blocker in repo truth, then rerun `{route_command}` and `{refresh_command}`."
        ),
        LaneExecutionStatus::Failed => {
            if let Some(run_id) = lane.last_run_id.as_ref() {
                return format!(
                    "Inspect `fabro inspect {run_id}` and `fabro logs {run_id}`, fix the underlying cause, then rerun `{route_command}` and `{refresh_command}`."
                );
            }
            format!(
                "Inspect the last failure, fix the cause in repo truth, then rerun `{route_command}` and `{refresh_command}`."
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
        format!("- route: `{}`", frontier.route_command),
        format!("- refresh: `{}`", frontier.refresh_command),
        format!("- status: `{}`", frontier.status_command),
    ]
    .join("\n")
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
    lines.push(format!("- next move: {}", entry.next_operator_move));
    lines.join("\n")
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

fn paperclip_refresh_command(paths: &PaperclipPaths) -> String {
    format!(
        "fabro paperclip refresh --target-repo {} --program {}",
        shell_quote(&paths.target_repo.display().to_string()),
        shell_quote(&paths.program_id),
    )
}

fn paperclip_status_command(paths: &PaperclipPaths) -> String {
    format!(
        "fabro paperclip status --target-repo {} --program {}",
        shell_quote(&paths.target_repo.display().to_string()),
        shell_quote(&paths.program_id),
    )
}

fn frontier_root_sync_key(program_id: &str) -> String {
    format!("frontier/{program_id}/root")
}

fn lane_sync_key(program_id: &str, lane_key: &str) -> String {
    format!("frontier/{program_id}/lane/{lane_key}")
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

fn paperclip_server_command(override_value: Option<&str>, data_dir: &Path) -> Result<String> {
    if let Some(value) = override_value {
        return Ok(format!(
            "exec {} run --data-dir {}",
            value,
            shell_quote(&data_dir.display().to_string()),
        ));
    }

    let local_repo = Path::new("/home/r/coding/paperclip");
    let tsx_cli = local_repo.join("cli/node_modules/tsx/dist/cli.mjs");
    let server_entry = local_repo.join("server/src/index.ts");
    let config_path = paperclip_instance_root(data_dir).join("config.json");
    let env_path = paperclip_instance_root(data_dir).join(".env");
    if local_repo.is_dir() && tsx_cli.exists() && server_entry.exists() {
        return Ok(format!(
            "cd {} && exec env PAPERCLIP_HOME={} PAPERCLIP_CONFIG={} DOTENV_CONFIG_PATH={} node {} {}",
            shell_quote(&local_repo.display().to_string()),
            shell_quote(&data_dir.display().to_string()),
            shell_quote(&config_path.display().to_string()),
            shell_quote(&env_path.display().to_string()),
            shell_quote(&tsx_cli.display().to_string()),
            shell_quote(&server_entry.display().to_string()),
        ));
    }

    Ok(format!(
        "exec {} run --data-dir {}",
        paperclip_command(None),
        shell_quote(&data_dir.display().to_string()),
    ))
}

fn ensure_local_paperclip_instance(data_dir: &Path) -> Result<()> {
    let instance_root = paperclip_instance_root(data_dir);
    let config_path = instance_root.join("config.json");
    if !config_path.exists() {
        seed_local_paperclip_config(&config_path)?;
    }
    ensure_local_paperclip_env(&config_path)?;
    ensure_local_paperclip_master_key(&config_path)?;
    Ok(())
}

fn paperclip_instance_root(data_dir: &Path) -> PathBuf {
    data_dir.join("instances").join("default")
}

fn seed_local_paperclip_config(config_path: &Path) -> Result<()> {
    let instance_root = config_path
        .parent()
        .context("paperclip config path should have a parent directory")?;
    let db_dir = instance_root.join("db");
    let backup_dir = instance_root.join("data").join("backups");
    let storage_dir = instance_root.join("data").join("storage");
    let log_dir = instance_root.join("logs");
    let key_file_path = instance_root.join("secrets").join("master.key");
    std::fs::create_dir_all(instance_root)?;

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
            "host": "127.0.0.1",
            "port": 3100,
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
    ensure_local_paperclip_instance(data_dir)?;
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
    metadata: Option<serde_json::Value>,
}

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
    if let Some(metadata_type) = desired_agent.metadata_type {
        return metadata.get("type").and_then(|value| value.as_str()) == Some(metadata_type);
    }
    metadata.get("unit").and_then(|value| value.as_str()) == desired_agent.unit.as_deref()
        && metadata.get("laneKey").and_then(|value| value.as_str())
            == desired_agent.lane_key.as_deref()
}

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
    Ok(PaperclipProjectSync { project, workspace })
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

async fn sync_coordination_issues(
    api_base: &str,
    company_id: &str,
    goal_id: &str,
    project_id: &str,
    frontier: &FrontierSyncModel,
    bundle_agents: &[BundleAgent],
    imported_agents: &[PaperclipImportAgent],
    existing_state: Option<&serde_json::Value>,
) -> Result<BTreeMap<String, String>> {
    let agent_ids = imported_agent_ids(imported_agents);
    let orchestrator_agent_id = agent_ids.get("raspberry-orchestrator").cloned();
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
        &desired_root_issue(frontier, orchestrator_agent_id.clone()),
    )
    .await?;
    synced.insert(root_key, root_issue.id.clone());

    for entry in &frontier.entries {
        let existing_issue = existing.remove(&entry.sync_key);
        let desired = desired_lane_issue(
            entry,
            &root_issue.id,
            orchestrator_agent_id.clone(),
            lane_agent_ids.get(&entry.lane_key).cloned(),
            existing_issue.is_some(),
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

fn desired_root_issue(
    frontier: &FrontierSyncModel,
    assignee_agent_id: Option<String>,
) -> DesiredIssue {
    let status = if frontier.summary.failed > 0 || frontier.summary.blocked > 0 {
        "blocked"
    } else if frontier.summary.running > 0 {
        "in_progress"
    } else if frontier.summary.ready > 0 {
        "todo"
    } else {
        "done"
    };

    DesiredIssue {
        title: format!("Raspberry frontier: {}", humanize(&frontier.program)),
        description: with_sync_marker(
            format!(
                "This issue tracks the live Raspberry frontier for `{}`.\n\nSummary:\n{}\n\nLane sets:\n{}\n\nExecution route:\n- `{}`\n- Refresh Paperclip with `{}`.\n- Inspect local status with `{}`.\n",
                frontier.program,
                render_frontier_summary(frontier),
                render_frontier_lane_sets(frontier),
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
    }
}

fn desired_lane_issue(
    entry: &FrontierSyncEntry,
    root_issue_id: &str,
    fallback_assignee_id: Option<String>,
    direct_assignee_id: Option<String>,
    existing_issue: bool,
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
    }
}

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
        "\nExecution route:\n- `{}`\n- Coordinate in Paperclip, but let Raspberry move the frontier.\n",
        entry.route_command,
    ));
    body
}

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
        "parentId": desired.parent_id,
        "title": desired.title,
        "description": desired.description,
        "status": desired.status,
        "priority": desired.priority,
        "assigneeAgentId": desired.assignee_agent_id,
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

        ensure_local_paperclip_instance(&data_dir).expect("seed local paperclip instance");

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
        };
        let managed = PaperclipManagedAgent {
            id: "agent-1".to_string(),
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

    #[test]
    fn paperclip_server_command_wraps_override_with_exec() {
        let command = paperclip_server_command(Some("paperclipai"), Path::new("/tmp/demo"))
            .expect("server command");

        assert!(command.starts_with("exec paperclipai run --data-dir "));
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
            manifest_path: temp.path().join("fabro/programs/zend.yaml"),
            state_path: temp.path().join(".raspberry/zend-state.json"),
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
                    route_command: "bash fabro/paperclip/zend/scripts/raspberry-orchestrator.sh"
                        .to_string(),
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
                    route_command: "bash fabro/paperclip/zend/scripts/raspberry-orchestrator.sh"
                        .to_string(),
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
                .join("fabro/paperclip/zend/scripts/raspberry-orchestrator.sh"),
            &frontier,
        )
        .expect("build company bundle");

        let lane_agents = bundle
            .agents
            .iter()
            .filter(|agent| agent.lane_key.is_some())
            .collect::<Vec<_>>();
        assert_eq!(lane_agents.len(), 2);
        assert!(lane_agents
            .iter()
            .any(|agent| agent.slug == "command-center-client--plan"));
        assert!(lane_agents
            .iter()
            .any(|agent| agent.slug == "command-center-client--implement"));
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
            route_command: "bash fabro/paperclip/zend/scripts/raspberry-orchestrator.sh"
                .to_string(),
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
    fn last_lines_returns_requested_tail() {
        let rendered = last_lines("a\nb\nc\nd\n", 2);

        assert_eq!(rendered, "c\nd\n");
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
