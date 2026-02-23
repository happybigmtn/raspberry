pub mod backend;
pub mod run;
pub mod validate;

use std::path::Path;

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

use crate::event::PipelineEvent;
use crate::validation::{Diagnostic, Severity};

#[derive(Parser)]
#[command(name = "attractor", version, about = "DOT-based pipeline runner for AI workflows")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Launch a pipeline from a .dot file
    Run(RunArgs),
    /// Parse and validate a pipeline without executing
    Validate(ValidateArgs),
}

#[derive(Args)]
pub struct RunArgs {
    /// Path to the .dot pipeline file
    pub pipeline: PathBuf,

    /// Log/artifact directory
    #[arg(long)]
    pub logs_dir: Option<PathBuf>,

    /// Execute with simulated LLM backend
    #[arg(long)]
    pub dry_run: bool,

    /// Auto-approve all human gates
    #[arg(long)]
    pub auto_approve: bool,

    /// Resume from a checkpoint file
    #[arg(long)]
    pub resume: Option<PathBuf>,

    /// Override default LLM model
    #[arg(long)]
    pub model: Option<String>,

    /// Override default LLM provider
    #[arg(long)]
    pub provider: Option<String>,

    /// Verbosity level (-v summary, -vv full details)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Args)]
pub struct ValidateArgs {
    /// Path to the .dot pipeline file
    pub pipeline: PathBuf,
}

/// Read a .dot file from disk.
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn read_dot_file(path: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {e}", path.display()))
}

/// Print diagnostics to stderr, grouped by severity.
pub fn print_diagnostics(diagnostics: &[Diagnostic]) {
    for d in diagnostics {
        let prefix = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };
        let location = match (&d.node_id, &d.edge) {
            (Some(node), _) => format!(" [node: {node}]"),
            (_, Some((from, to))) => format!(" [edge: {from} -> {to}]"),
            _ => String::new(),
        };
        eprintln!("{prefix}{location}: {} ({})", d.message, d.rule);
    }
}

/// One-line summary of a pipeline event for `-v` output.
#[must_use]
pub fn format_event_summary(event: &PipelineEvent) -> String {
    match event {
        PipelineEvent::PipelineStarted { name, id } => {
            format!("[PIPELINE_STARTED] name={name} id={id}")
        }
        PipelineEvent::PipelineCompleted {
            duration_ms,
            artifact_count,
        } => {
            format!("[PIPELINE_COMPLETED] duration={duration_ms}ms artifacts={artifact_count}")
        }
        PipelineEvent::PipelineFailed { error, duration_ms } => {
            format!("[PIPELINE_FAILED] error=\"{error}\" duration={duration_ms}ms")
        }
        PipelineEvent::StageStarted { name, index } => {
            format!("[STAGE_STARTED] name={name} index={index}")
        }
        PipelineEvent::StageCompleted {
            name,
            index,
            duration_ms,
        } => {
            format!("[STAGE_COMPLETED] name={name} index={index} duration={duration_ms}ms")
        }
        PipelineEvent::StageFailed {
            name,
            index,
            error,
            will_retry,
        } => {
            format!(
                "[STAGE_FAILED] name={name} index={index} error=\"{error}\" will_retry={will_retry}"
            )
        }
        PipelineEvent::StageRetrying {
            name,
            index,
            attempt,
            delay_ms,
        } => {
            format!(
                "[STAGE_RETRYING] name={name} index={index} attempt={attempt} delay={delay_ms}ms"
            )
        }
        PipelineEvent::ParallelStarted { branch_count } => {
            format!("[PARALLEL_STARTED] branches={branch_count}")
        }
        PipelineEvent::ParallelBranchStarted { branch, index } => {
            format!("[PARALLEL_BRANCH_STARTED] branch={branch} index={index}")
        }
        PipelineEvent::ParallelBranchCompleted {
            branch,
            index,
            duration_ms,
            success,
        } => {
            format!("[PARALLEL_BRANCH_COMPLETED] branch={branch} index={index} duration={duration_ms}ms success={success}")
        }
        PipelineEvent::ParallelCompleted {
            duration_ms,
            success_count,
            failure_count,
        } => {
            format!("[PARALLEL_COMPLETED] duration={duration_ms}ms succeeded={success_count} failed={failure_count}")
        }
        PipelineEvent::InterviewStarted { question, stage } => {
            format!("[INTERVIEW_STARTED] stage={stage} question=\"{question}\"")
        }
        PipelineEvent::InterviewCompleted {
            question,
            answer,
            duration_ms,
        } => {
            format!(
                "[INTERVIEW_COMPLETED] question=\"{question}\" answer=\"{answer}\" duration={duration_ms}ms"
            )
        }
        PipelineEvent::InterviewTimeout {
            stage, duration_ms, ..
        } => {
            format!("[INTERVIEW_TIMEOUT] stage={stage} duration={duration_ms}ms")
        }
        PipelineEvent::CheckpointSaved { node_id } => {
            format!("[CHECKPOINT_SAVED] node={node_id}")
        }
    }
}

/// Multi-line detail view of a pipeline event for `-vv` output.
#[must_use]
pub fn format_event_detail(event: &PipelineEvent) -> String {
    match event {
        PipelineEvent::PipelineStarted { name, id } => {
            format!(
                "── PIPELINE_STARTED ─────────────────────────\n  name: {name}\n  id:   {id}\n"
            )
        }
        PipelineEvent::PipelineCompleted {
            duration_ms,
            artifact_count,
        } => {
            format!("── PIPELINE_COMPLETED ───────────────────────\n  duration_ms:    {duration_ms}\n  artifact_count: {artifact_count}\n")
        }
        PipelineEvent::PipelineFailed { error, duration_ms } => {
            format!("── PIPELINE_FAILED ──────────────────────────\n  error:       {error}\n  duration_ms: {duration_ms}\n")
        }
        PipelineEvent::StageStarted { name, index } => {
            format!(
                "── STAGE_STARTED ────────────────────────────\n  name:  {name}\n  index: {index}\n"
            )
        }
        PipelineEvent::StageCompleted {
            name,
            index,
            duration_ms,
        } => {
            format!("── STAGE_COMPLETED ──────────────────────────\n  name:        {name}\n  index:       {index}\n  duration_ms: {duration_ms}\n")
        }
        PipelineEvent::StageFailed {
            name,
            index,
            error,
            will_retry,
        } => {
            format!("── STAGE_FAILED ─────────────────────────────\n  name:       {name}\n  index:      {index}\n  error:      {error}\n  will_retry: {will_retry}\n")
        }
        PipelineEvent::StageRetrying {
            name,
            index,
            attempt,
            delay_ms,
        } => {
            format!("── STAGE_RETRYING ───────────────────────────\n  name:     {name}\n  index:    {index}\n  attempt:  {attempt}\n  delay_ms: {delay_ms}\n")
        }
        PipelineEvent::ParallelStarted { branch_count } => {
            format!("── PARALLEL_STARTED ─────────────────────────\n  branch_count: {branch_count}\n")
        }
        PipelineEvent::ParallelBranchStarted { branch, index } => {
            format!("── PARALLEL_BRANCH_STARTED ──────────────────\n  branch: {branch}\n  index:  {index}\n")
        }
        PipelineEvent::ParallelBranchCompleted {
            branch,
            index,
            duration_ms,
            success,
        } => {
            format!("── PARALLEL_BRANCH_COMPLETED ────────────────\n  branch:      {branch}\n  index:       {index}\n  duration_ms: {duration_ms}\n  success:     {success}\n")
        }
        PipelineEvent::ParallelCompleted {
            duration_ms,
            success_count,
            failure_count,
        } => {
            format!("── PARALLEL_COMPLETED ───────────────────────\n  duration_ms:   {duration_ms}\n  success_count: {success_count}\n  failure_count: {failure_count}\n")
        }
        PipelineEvent::InterviewStarted { question, stage } => {
            format!("── INTERVIEW_STARTED ────────────────────────\n  stage:    {stage}\n  question: {question}\n")
        }
        PipelineEvent::InterviewCompleted {
            question,
            answer,
            duration_ms,
        } => {
            format!("── INTERVIEW_COMPLETED ──────────────────────\n  question:    {question}\n  answer:      {answer}\n  duration_ms: {duration_ms}\n")
        }
        PipelineEvent::InterviewTimeout {
            question,
            stage,
            duration_ms,
        } => {
            format!("── INTERVIEW_TIMEOUT ────────────────────────\n  question:    {question}\n  stage:       {stage}\n  duration_ms: {duration_ms}\n")
        }
        PipelineEvent::CheckpointSaved { node_id } => {
            format!(
                "── CHECKPOINT_SAVED ─────────────────────────\n  node_id: {node_id}\n"
            )
        }
    }
}
