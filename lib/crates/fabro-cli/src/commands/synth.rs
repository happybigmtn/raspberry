use std::path::PathBuf;

use clap::{Args, Subcommand};
use fabro_synthesis::{
    author_blueprint_for_create, author_blueprint_for_evolve, cleanup_obsolete_package_files,
    import_existing_package, load_blueprint, reconcile_blueprint, render_blueprint, save_blueprint,
    ImportRequest, ReconcileRequest, RenderRequest,
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
    pub blueprint: Option<PathBuf>,
    #[arg(long)]
    pub target_repo: PathBuf,
    #[arg(long)]
    pub program: Option<String>,
    #[arg(long)]
    pub output_blueprint: Option<PathBuf>,
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
    let mut previous_blueprint = None;
    let (blueprint, blueprint_path, notes) = if let Some(path) = &args.blueprint {
        (load_blueprint(path)?, path.clone(), Vec::new())
    } else {
        let authored = author_blueprint_for_create(&args.target_repo, args.program.as_deref())?;
        let path = args.output_blueprint.clone().unwrap_or_else(|| {
            default_blueprint_path(&args.target_repo, &authored.blueprint.program.id)
        });
        previous_blueprint = if path.exists() {
            load_blueprint(&path).ok()
        } else {
            None
        };
        save_blueprint(&path, &authored.blueprint)?;
        (authored.blueprint, path, authored.notes)
    };
    if let Some(previous) = previous_blueprint.as_ref() {
        cleanup_obsolete_package_files(previous, &blueprint, &args.target_repo)?;
    }
    let report = render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: &args.target_repo,
    })?;

    println!("Program: {}", blueprint.program.id);
    println!("Mode: create");
    println!("Blueprint: {}", blueprint_path.display());
    if !notes.is_empty() {
        println!("Notes:");
        for note in notes {
            println!("  - {note}");
        }
    }
    println!("Written files:");
    for path in report.written_files {
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
        .join("fabro")
        .join("blueprints")
        .join(format!("{program}.yaml"))
}
