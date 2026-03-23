use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use raspberry_supervisor::{
    autodev_report_path, evaluate_program, evaluate_program_local, execute_selected_lanes,
    load_optional_autodev_report, load_plan_matrix, orchestrate_program, render_grouped_summary,
    render_plan_matrix, render_status_table, sync_autodev_report_with_program, AutodevCycleReport,
    AutodevProvenance, AutodevReport, AutodevSettings, AutodevStopReason, DispatchSettings,
    LaneExecutionStatus, ProgramManifest,
};

const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("RASPBERRY_GIT_SHA"),
    " ",
    env!("RASPBERRY_BUILD_DATE"),
    ")"
);

#[derive(Debug, Parser)]
#[command(
    name = "raspberry",
    version,
    long_version = LONG_VERSION,
    about = "Raspberry supervisory control-plane CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Plan(ManifestArgs),
    PlanMatrix(ManifestArgs),
    Status(ManifestArgs),
    Watch(WatchArgs),
    Execute(ExecuteArgs),
    Autodev(AutodevArgs),
    Tui(TuiArgs),
}

#[derive(Debug, Args)]
struct ManifestArgs {
    #[arg(long)]
    manifest: PathBuf,
}

#[derive(Debug, Args)]
struct WatchArgs {
    #[arg(long)]
    manifest: PathBuf,
    #[arg(long, default_value_t = 500)]
    interval_ms: u64,
    #[arg(long, default_value_t = 0)]
    iterations: usize,
}

#[derive(Debug, Args)]
struct ExecuteArgs {
    #[arg(long)]
    manifest: PathBuf,
    #[arg(long, default_value = "fabro")]
    fabro_bin: PathBuf,
    #[arg(long)]
    max_parallel: Option<usize>,
    #[arg(long = "lane")]
    lanes: Vec<String>,
}

#[derive(Debug, Args)]
struct AutodevArgs {
    #[arg(long)]
    manifest: PathBuf,
    #[arg(long, default_value = "fabro")]
    fabro_bin: PathBuf,
    #[arg(long)]
    max_parallel: Option<usize>,
    #[arg(long)]
    frontier_budget: Option<usize>,
    #[arg(
        long,
        default_value_t = 5,
        help = "Maximum scheduler cycles before stopping (0 = unlimited)"
    )]
    max_cycles: usize,
    #[arg(long, default_value_t = 500)]
    poll_interval_ms: u64,
    #[arg(long, default_value_t = 21600)]
    evolve_every_seconds: u64,
    #[arg(long = "doctrine")]
    doctrine_files: Vec<PathBuf>,
    #[arg(long = "evidence")]
    evidence_paths: Vec<PathBuf>,
    #[arg(long)]
    preview_evolve_root: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct TuiArgs {
    #[arg(long)]
    manifest: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Plan(args) => run_plan(&args.manifest),
        Commands::PlanMatrix(args) => run_plan_matrix(&args.manifest),
        Commands::Status(args) => run_status(&args.manifest),
        Commands::Watch(args) => run_watch(args),
        Commands::Execute(args) => run_execute(args),
        Commands::Autodev(args) => run_autodev(args),
        Commands::Tui(args) => run_tui(args),
    }
}

fn run_plan(manifest_path: &Path) -> Result<()> {
    let manifest = ProgramManifest::load(manifest_path)?;
    let program = evaluate_program_local(manifest_path)?;
    sync_autodev_report_with_program(manifest_path, &manifest, &program)?;
    println!("{}", render_grouped_summary(&program));
    Ok(())
}

fn run_plan_matrix(manifest_path: &Path) -> Result<()> {
    let matrix = load_plan_matrix(manifest_path)?;
    println!("{}", render_plan_matrix(&matrix));
    Ok(())
}

fn run_status(manifest_path: &Path) -> Result<()> {
    let manifest = ProgramManifest::load(manifest_path)?;
    let program = evaluate_program_local(manifest_path)?;
    sync_autodev_report_with_program(manifest_path, &manifest, &program)?;
    if let Some(report) = read_autodev_report(manifest_path, &manifest) {
        print_autodev_provenance(&report);
    }
    println!("{}", render_status_table(&program));
    Ok(())
}

fn run_watch(args: WatchArgs) -> Result<()> {
    let until_settled = args.iterations == 0;
    let total_iterations = if until_settled {
        usize::MAX
    } else {
        args.iterations
    };

    for index in 0..total_iterations {
        let manifest = ProgramManifest::load(&args.manifest)?;
        let program = evaluate_program_local(&args.manifest)?;
        sync_autodev_report_with_program(&args.manifest, &manifest, &program)?;
        println!("Iteration {}:", index + 1);
        println!("{}", render_status_table(&program));
        if until_settled
            && !program
                .lanes
                .iter()
                .any(|lane| lane.status == LaneExecutionStatus::Running)
        {
            break;
        }
        if index + 1 >= total_iterations {
            break;
        }
        thread::sleep(Duration::from_millis(args.interval_ms));
    }

    Ok(())
}

fn run_execute(args: ExecuteArgs) -> Result<()> {
    let manifest = ProgramManifest::load(&args.manifest)?;
    let program = evaluate_program(&args.manifest)?;
    let fabro_bin = resolve_fabro_bin(&args.fabro_bin);
    let selected = if args.lanes.is_empty() {
        program
            .lanes
            .iter()
            .filter(|lane| lane.status == LaneExecutionStatus::Ready)
            .map(|lane| lane.lane_key.clone())
            .collect::<Vec<_>>()
    } else {
        args.lanes.clone()
    };

    if selected.is_empty() {
        bail!("no ready lanes selected for execution");
    }

    let outcomes = execute_selected_lanes(
        &args.manifest,
        &selected,
        &DispatchSettings {
            fabro_bin,
            max_parallel_override: args.max_parallel,
            doctrine_files: Vec::new(),
            evidence_paths: Vec::new(),
            preview_evolve_root: None,
            manifest_stack: Vec::new(),
        },
    )
    .with_context(|| format!("failed to execute lanes for program `{}`", manifest.program))?;

    println!("Program: {}", manifest.program);
    println!(
        "Dispatch parallelism: {}",
        args.max_parallel.unwrap_or(program.max_parallel)
    );
    for outcome in outcomes {
        if outcome.exit_status == 0 {
            let run_id = outcome.fabro_run_id.as_deref().unwrap_or("unknown");
            println!(
                "{} [submitted] run_id={} exit_status={}",
                outcome.lane_key, run_id, outcome.exit_status
            );
        } else {
            println!(
                "{} [failed] exit_status={}",
                outcome.lane_key, outcome.exit_status
            );
        }
    }
    Ok(())
}

fn run_autodev(args: AutodevArgs) -> Result<()> {
    let manifest = ProgramManifest::load(&args.manifest)?;
    let report_path = autodev_report_path(&args.manifest, &manifest);
    let fabro_bin = resolve_fabro_bin(&args.fabro_bin);
    println!("Autodev live report: {}", report_path.display());
    let heartbeat_interval_ms = args.poll_interval_ms.max(100);
    let stop_heartbeat = Arc::new(AtomicBool::new(false));
    let heartbeat_handle = spawn_autodev_heartbeat(
        args.manifest.clone(),
        manifest,
        Arc::clone(&stop_heartbeat),
        Duration::from_millis(heartbeat_interval_ms),
    );

    let report = orchestrate_program(
        &args.manifest,
        &AutodevSettings {
            fabro_bin,
            max_parallel_override: args.max_parallel,
            frontier_budget: args.frontier_budget,
            max_cycles: args.max_cycles,
            poll_interval_ms: args.poll_interval_ms,
            evolve_every_seconds: args.evolve_every_seconds,
            doctrine_files: args.doctrine_files.clone(),
            evidence_paths: args.evidence_paths.clone(),
            preview_evolve_root: args.preview_evolve_root.clone(),
            manifest_stack: Vec::new(),
        },
    );

    stop_heartbeat.store(true, Ordering::Relaxed);
    let _ = heartbeat_handle.join();
    let report = report?;

    println!("Program: {}", report.program);
    println!("Autodev cycles: {}", report.cycles.len());
    for cycle in &report.cycles {
        println!("Cycle {}:", cycle.cycle);
        if cycle.evolved {
            println!(
                "  evolve: applied{}",
                cycle
                    .evolve_target
                    .as_deref()
                    .map(|target| format!(" target={target}"))
                    .unwrap_or_default()
            );
        } else {
            println!("  evolve: skipped");
        }
        if cycle.ready_lanes.is_empty() {
            println!("  ready: none");
        } else {
            println!("  ready: {}", cycle.ready_lanes.join(", "));
        }
        if cycle.dispatched.is_empty() {
            println!("  dispatched: none");
        } else {
            for outcome in &cycle.dispatched {
                let run_id = outcome.fabro_run_id.as_deref().unwrap_or("unknown");
                println!("  dispatched: {} run_id={run_id}", outcome.lane_key);
            }
        }
        println!("  running_after: {}", cycle.running_after);
        println!("  complete_after: {}", cycle.complete_after);
    }
    let stop_reason = match report.stop_reason {
        AutodevStopReason::Settled => "settled",
        AutodevStopReason::CycleLimit => "cycle_limit",
        AutodevStopReason::Maintenance => "maintenance",
    };
    println!("Stop reason: {stop_reason}");
    Ok(())
}

fn resolve_fabro_bin(requested: &Path) -> PathBuf {
    if requested != Path::new("fabro") {
        return requested.to_path_buf();
    }

    let local_debug =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../target-local/debug/fabro");
    if local_debug.exists() {
        return local_debug;
    }

    requested.to_path_buf()
}

fn spawn_autodev_heartbeat(
    manifest_path: PathBuf,
    manifest: ProgramManifest,
    stop: Arc<AtomicBool>,
    interval: Duration,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut printed_cycles = 0usize;
        loop {
            if let Some(report) = read_autodev_report(&manifest_path, &manifest) {
                for cycle in report.cycles.iter().skip(printed_cycles) {
                    println!("{}", format_autodev_cycle_heartbeat(cycle));
                }
                printed_cycles = report.cycles.len();
            }

            if stop.load(Ordering::Relaxed) {
                break;
            }

            thread::sleep(interval);
        }

        if let Some(report) = read_autodev_report(&manifest_path, &manifest) {
            for cycle in report.cycles.iter().skip(printed_cycles) {
                println!("{}", format_autodev_cycle_heartbeat(cycle));
            }
        }
    })
}

fn read_autodev_report(manifest_path: &Path, manifest: &ProgramManifest) -> Option<AutodevReport> {
    load_optional_autodev_report(manifest_path, manifest)
        .ok()
        .flatten()
}

fn format_autodev_cycle_heartbeat(cycle: &AutodevCycleReport) -> String {
    let evolve = if cycle.evolved { "applied" } else { "skipped" };
    let ready = cycle.ready_lanes.len();
    let replayed = cycle.replayed_lanes.len();
    let dispatched = cycle.dispatched.len();

    format!(
        "[autodev] cycle={} evolve={} ready={} replayed={} regenerate_noop={} dispatched={} running={} complete={}",
        cycle.cycle,
        evolve,
        ready,
        replayed,
        cycle.regenerate_noop_lanes.len(),
        dispatched,
        cycle.running_after,
        cycle.complete_after
    )
}

fn print_autodev_provenance(report: &AutodevReport) {
    let Some(provenance) = report.provenance.as_ref() else {
        return;
    };
    println!(
        "Controller provenance: {}",
        format_binary_provenance(provenance, true)
    );
    println!(
        "Fabro provenance: {}",
        format_binary_provenance(provenance, false)
    );
}

fn format_binary_provenance(provenance: &AutodevProvenance, controller: bool) -> String {
    let binary = if controller {
        &provenance.controller
    } else {
        &provenance.fabro_bin
    };
    match binary.version.as_deref() {
        Some(version) => format!("{version} @ {}", binary.path),
        None => binary.path.clone(),
    }
}

fn run_tui(args: TuiArgs) -> Result<()> {
    raspberry_tui::run(&args.manifest)
}
