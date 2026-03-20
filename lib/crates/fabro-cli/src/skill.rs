use std::path::Path;

use anyhow::{bail, Result};
use clap::{Args, ValueEnum};
use tracing::{debug, info};

const SKILL_MD: &str = include_str!("../../../../skills/fabro-create-workflow/SKILL.md");
const REF_DOT_LANGUAGE: &str =
    include_str!("../../../../skills/fabro-create-workflow/references/dot-language.md");
const REF_EXAMPLE_WORKFLOWS: &str =
    include_str!("../../../../skills/fabro-create-workflow/references/example-workflows.md");
const REF_RUN_CONFIGURATION: &str =
    include_str!("../../../../skills/fabro-create-workflow/references/run-configuration.md");
const REF_RASPBERRY_AUTHORING: &str =
    include_str!("../../../../skills/fabro-create-workflow/references/raspberry-authoring.md");
const REF_RASPBERRY_EXAMPLES: &str =
    include_str!("../../../../skills/fabro-create-workflow/references/raspberry-examples.md");
const REF_PROGRAM_SYNTHESIS: &str =
    include_str!("../../../../skills/fabro-create-workflow/references/program-synthesis.md");
const REF_PROGRAM_INTERVIEW: &str =
    include_str!("../../../../skills/fabro-create-workflow/references/program-interview.md");
const REF_PROGRAM_BLUEPRINT_SCHEMA: &str =
    include_str!("../../../../skills/fabro-create-workflow/references/program-blueprint-schema.md");
const REF_PROGRAM_EVOLUTION: &str =
    include_str!("../../../../skills/fabro-create-workflow/references/program-evolution.md");

const SKILL_FILES: &[(&str, &str)] = &[
    ("SKILL.md", SKILL_MD),
    ("references/dot-language.md", REF_DOT_LANGUAGE),
    ("references/example-workflows.md", REF_EXAMPLE_WORKFLOWS),
    ("references/run-configuration.md", REF_RUN_CONFIGURATION),
    ("references/raspberry-authoring.md", REF_RASPBERRY_AUTHORING),
    ("references/raspberry-examples.md", REF_RASPBERRY_EXAMPLES),
    ("references/program-synthesis.md", REF_PROGRAM_SYNTHESIS),
    ("references/program-interview.md", REF_PROGRAM_INTERVIEW),
    (
        "references/program-blueprint-schema.md",
        REF_PROGRAM_BLUEPRINT_SCHEMA,
    ),
    ("references/program-evolution.md", REF_PROGRAM_EVOLUTION),
];

#[derive(Clone, ValueEnum)]
pub enum SkillDir {
    Claude,
    Agents,
}

#[derive(Args)]
pub struct SkillInstallArgs {
    /// Where to install: user-level or project-level
    #[arg(long = "for", default_value = "user")]
    scope: SkillScope,

    /// Target directory convention
    #[arg(long)]
    dir: SkillDir,

    /// Overwrite existing skill without prompting
    #[arg(long)]
    force: bool,
}

#[derive(Clone, ValueEnum)]
pub enum SkillScope {
    User,
    Project,
}

/// Install all skill files under `base_dir/fabro-create-workflow/`.
pub fn install_skill_to(base_dir: &Path) -> Result<()> {
    let skill_dir = base_dir.join("fabro-create-workflow");

    for (rel_path, content) in SKILL_FILES {
        let dest = skill_dir.join(rel_path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        debug!(file = %rel_path, "Writing skill file");
        std::fs::write(&dest, content)?;
    }

    info!(path = %skill_dir.display(), "Skill installed");
    Ok(())
}

pub fn run_skill_install(args: &SkillInstallArgs) -> Result<()> {
    let base_dir = resolve_base_dir(&args.scope, &args.dir)?;
    let skill_dir = base_dir.join("fabro-create-workflow");

    if skill_dir.exists() && !args.force {
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!(
                "Skill directory already exists at {}. Overwrite?",
                skill_dir.display()
            ))
            .default(false)
            .interact()?;

        if !confirm {
            bail!("Aborted: skill directory already exists");
        }
    }

    install_skill_to(&base_dir)
}

fn resolve_base_dir(scope: &SkillScope, dir: &SkillDir) -> Result<std::path::PathBuf> {
    let dir_name = match dir {
        SkillDir::Claude => ".claude",
        SkillDir::Agents => ".agents",
    };

    let root = match scope {
        SkillScope::User => {
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        }
        SkillScope::Project => std::env::current_dir()?,
    };

    Ok(root.join(dir_name).join("skills"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_files_are_non_empty() {
        assert!(!SKILL_MD.is_empty());
        assert!(!REF_DOT_LANGUAGE.is_empty());
        assert!(!REF_EXAMPLE_WORKFLOWS.is_empty());
        assert!(!REF_RUN_CONFIGURATION.is_empty());
        assert!(!REF_RASPBERRY_AUTHORING.is_empty());
        assert!(!REF_RASPBERRY_EXAMPLES.is_empty());
        assert!(!REF_PROGRAM_SYNTHESIS.is_empty());
        assert!(!REF_PROGRAM_INTERVIEW.is_empty());
        assert!(!REF_PROGRAM_BLUEPRINT_SCHEMA.is_empty());
        assert!(!REF_PROGRAM_EVOLUTION.is_empty());
    }

    #[test]
    fn install_writes_all_files() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("skills");

        install_skill_to(&base).unwrap();

        for (rel_path, content) in SKILL_FILES {
            let path = base.join("fabro-create-workflow").join(rel_path);
            assert!(path.exists(), "Missing file: {rel_path}");
            let written = std::fs::read_to_string(&path).unwrap();
            assert_eq!(written, *content, "Content mismatch: {rel_path}");
        }
    }

    #[test]
    fn install_overwrites_existing_files() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("skills");

        install_skill_to(&base).unwrap();

        // Write a sentinel to one file
        let sentinel_path = base.join("fabro-create-workflow/SKILL.md");
        std::fs::write(&sentinel_path, "old content").unwrap();

        // Re-install should overwrite
        install_skill_to(&base).unwrap();

        let content = std::fs::read_to_string(&sentinel_path).unwrap();
        assert_eq!(content, SKILL_MD);
    }
}
