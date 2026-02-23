use anyhow::bail;

use crate::pipeline::PipelineBuilder;
use crate::validation::Severity;

use super::{print_diagnostics, read_dot_file, ValidateArgs};

/// Parse and validate a pipeline file without executing it.
///
/// # Errors
///
/// Returns an error if the file cannot be read, parsed, or has validation errors.
pub fn validate_command(args: &ValidateArgs) -> anyhow::Result<()> {
    let source = read_dot_file(&args.pipeline)?;
    let (graph, diagnostics) = PipelineBuilder::new().prepare(&source)?;

    println!(
        "Parsed pipeline: {} ({} nodes, {} edges)",
        graph.name,
        graph.nodes.len(),
        graph.edges.len(),
    );

    print_diagnostics(&diagnostics);

    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        bail!("Validation failed");
    }

    println!("Validation: OK");
    Ok(())
}
