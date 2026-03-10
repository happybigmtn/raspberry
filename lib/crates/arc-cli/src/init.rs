use anyhow::{bail, Context, Result};
use std::path::PathBuf;

pub async fn run_init() -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git")?;

    if !output.status.success() {
        bail!("not a git repository — run `git init` first");
    }

    let repo_root = PathBuf::from(
        String::from_utf8(output.stdout)
            .context("git output was not valid UTF-8")?
            .trim(),
    );

    let arc_toml = repo_root.join("arc.toml");
    if arc_toml.exists() {
        bail!(
            "already initialized — arc.toml exists at {}",
            arc_toml.display()
        );
    }

    // Create arc.toml
    std::fs::write(
        &arc_toml,
        "\
# Arc project configuration
# https://docs.arc.computer/getting-started/quick-start

version = 1

[arc]
root = \"arc/\"

# Disable retrospective analysis after workflow runs:
# retro = false

# Auto-create pull requests on successful workflow runs:
# [pull_request]
# enabled = true
# draft = true
",
    )
    .with_context(|| format!("failed to write {}", arc_toml.display()))?;

    let green = console::Style::new().green();
    let bold = console::Style::new().bold();
    let dim = console::Style::new().dim();
    eprintln!("  {} {}", green.apply_to("✔"), dim.apply_to("arc.toml"));

    // Create hello workflow directory
    let workflow_dir = repo_root.join("arc/workflows/hello");
    std::fs::create_dir_all(&workflow_dir)
        .with_context(|| format!("failed to create {}", workflow_dir.display()))?;

    // Create workflow.dot
    let dot_path = workflow_dir.join("workflow.dot");
    std::fs::write(
        &dot_path,
        r#"digraph Hello {
    graph [goal="Say hello and demonstrate a basic arc workflow"]
    rankdir=LR

    start [shape=Mdiamond, label="Start"]
    exit  [shape=Msquare, label="Exit"]

    greet [label="Greet", prompt="Say hello! Introduce yourself and explain that this is a test of the arc workflow engine."]

    start -> greet -> exit
}
"#,
    )
    .with_context(|| format!("failed to write {}", dot_path.display()))?;
    eprintln!(
        "  {} {}",
        green.apply_to("✔"),
        dim.apply_to("arc/workflows/hello/workflow.dot")
    );

    // Create workflow.toml
    let toml_path = workflow_dir.join("workflow.toml");
    std::fs::write(
        &toml_path,
        "version = 1\ngraph = \"workflow.dot\"\n\n[sandbox]\nprovider = \"local\"\n",
    )
    .with_context(|| format!("failed to write {}", toml_path.display()))?;
    eprintln!(
        "  {} {}",
        green.apply_to("✔"),
        dim.apply_to("arc/workflows/hello/workflow.toml")
    );

    eprintln!(
        "\n{} Run a workflow with:\n\n  {}",
        bold.apply_to("Project initialized!"),
        console::Style::new()
            .cyan()
            .bold()
            .apply_to("arc run hello")
    );

    Ok(())
}
