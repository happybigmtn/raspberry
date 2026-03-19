use std::path::PathBuf;

use clap::{Args, Subcommand};
use fabro_synthesis::{
    ImportRequest, RenderRequest, ReconcileRequest, import_existing_package, load_blueprint,
    reconcile_blueprint, render_blueprint, save_blueprint,
};

#[derive(Debug, Subcommand)]
pub enum SynthCommand {
    /// Import an existing Fabro workflow package into a blueprint file
    Import(SynthImportArgs),
    /// Create a checked-in Fabro workflow package from a blueprint
    Create(SynthCreateArgs),
    /// Evolve an existing Fabro workflow package from a revised blueprint
    Evolve(SynthEvolveArgs),
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
    pub blueprint: PathBuf,
    #[arg(long)]
    pub target_repo: PathBuf,
}

#[derive(Debug, Args)]
pub struct SynthEvolveArgs {
    #[arg(long)]
    pub blueprint: PathBuf,
    #[arg(long)]
    pub target_repo: PathBuf,
    #[arg(long)]
    pub preview_root: Option<PathBuf>,
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
    let blueprint = load_blueprint(&args.blueprint)?;
    let report = render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: &args.target_repo,
    })?;

    println!("Program: {}", blueprint.program.id);
    println!("Mode: create");
    println!("Written files:");
    for path in report.written_files {
        println!("  {}", path.display());
    }
    Ok(())
}

pub fn evolve_command(args: &SynthEvolveArgs) -> anyhow::Result<()> {
    let blueprint = load_blueprint(&args.blueprint)?;
    let output_repo = args.preview_root.as_ref().unwrap_or(&args.target_repo);
    let report = reconcile_blueprint(ReconcileRequest {
        blueprint: &blueprint,
        current_repo: &args.target_repo,
        output_repo,
    })?;

    println!("Program: {}", blueprint.program.id);
    println!("Mode: evolve");
    if args.preview_root.is_some() {
        println!("Preview root: {}", output_repo.display());
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
