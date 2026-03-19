use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use raspberry_supervisor::{
    evaluate_program, execute_selected_lanes, orchestrate_program, render_grouped_summary,
    render_status_table, AutodevSettings, AutodevStopReason, DispatchSettings,
    LaneExecutionStatus, ProgramManifest,
};

#[derive(Debug, Parser)]
#[command(name = "raspberry", about = "Raspberry supervisory control-plane CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Plan(ManifestArgs),
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
    #[arg(long, default_value_t = 5)]
    max_cycles: usize,
    #[arg(long, default_value_t = 500)]
    poll_interval_ms: u64,
    #[arg(long, default_value_t = 1800)]
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
        Commands::Status(args) => run_status(&args.manifest),
        Commands::Watch(args) => run_watch(args),
        Commands::Execute(args) => run_execute(args),
        Commands::Autodev(args) => run_autodev(args),
        Commands::Tui(args) => run_tui(args),
    }
}

fn run_plan(manifest_path: &Path) -> Result<()> {
    let program = evaluate_program(manifest_path)?;
    println!("{}", render_grouped_summary(&program));
    Ok(())
}

fn run_status(manifest_path: &Path) -> Result<()> {
    let program = evaluate_program(manifest_path)?;
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
        let program = evaluate_program(&args.manifest)?;
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
            fabro_bin: args.fabro_bin.clone(),
            max_parallel_override: args.max_parallel,
            doctrine_files: Vec::new(),
            evidence_paths: Vec::new(),
            preview_evolve_root: None,
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
    let report = orchestrate_program(
        &args.manifest,
        &AutodevSettings {
            fabro_bin: args.fabro_bin.clone(),
            max_parallel_override: args.max_parallel,
            max_cycles: args.max_cycles,
            poll_interval_ms: args.poll_interval_ms,
            evolve_every_seconds: args.evolve_every_seconds,
            doctrine_files: args.doctrine_files.clone(),
            evidence_paths: args.evidence_paths.clone(),
            preview_evolve_root: args.preview_evolve_root.clone(),
        },
    )?;

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
    };
    println!("Stop reason: {stop_reason}");
    Ok(())
}

fn run_tui(args: TuiArgs) -> Result<()> {
    raspberry_tui::run(&args.manifest)
}
