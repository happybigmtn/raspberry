use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use raspberry_supervisor::{
    LaneExecutionStatus, ProgramManifest, evaluate_program, execute_selected_lanes,
    render_grouped_summary, render_status_table,
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Plan(args) => run_plan(&args.manifest),
        Commands::Status(args) => run_status(&args.manifest),
        Commands::Watch(args) => run_watch(args),
        Commands::Execute(args) => run_execute(args),
    }
}

fn run_plan(manifest_path: &PathBuf) -> Result<()> {
    let program = evaluate_program(manifest_path)?;
    println!("{}", render_grouped_summary(&program));
    Ok(())
}

fn run_status(manifest_path: &PathBuf) -> Result<()> {
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
        &args.fabro_bin,
        args.max_parallel,
    )
        .with_context(|| format!("failed to execute lanes for program `{}`", manifest.program))?;

    println!("Program: {}", manifest.program);
    println!(
        "Dispatch parallelism: {}",
        args.max_parallel.unwrap_or(program.max_parallel)
    );
    for outcome in outcomes {
        if outcome.exit_status == 0 {
            let run_id = outcome
                .fabro_run_id
                .as_deref()
                .unwrap_or("unknown");
            println!(
                "{} [submitted] run_id={} exit_status={}",
                outcome.lane_key, run_id, outcome.exit_status
            );
        } else {
            println!("{} [failed] exit_status={}", outcome.lane_key, outcome.exit_status);
        }
    }
    Ok(())
}
