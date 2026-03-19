use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use raspberry_supervisor::{EvaluatedLane, ProgramManifest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactEntry {
    pub id: String,
    pub path: std::path::PathBuf,
    pub exists: bool,
}

pub fn collect_lane_artifacts(
    manifest_path: &Path,
    manifest: &ProgramManifest,
    lane: &EvaluatedLane,
) -> Vec<ArtifactEntry> {
    let mut artifacts = Vec::new();
    for artifact in manifest.resolve_lane_artifacts(manifest_path, &lane.unit_id, &lane.lane_id) {
        artifacts.push(ArtifactEntry {
            id: artifact.id,
            exists: artifact.path.is_file(),
            path: artifact.path,
        });
    }
    artifacts
}

pub fn preview_artifact(entry: &ArtifactEntry) -> Result<String> {
    if !entry.exists {
        return Ok(format!(
            "Missing artifact: {}\nPath: {}",
            entry.id,
            entry.path.display()
        ));
    }

    let bytes = fs::read(&entry.path).with_context(|| {
        format!(
            "failed to read curated artifact `{}` from {}",
            entry.id,
            entry.path.display()
        )
    })?;
    let contents = String::from_utf8_lossy(&bytes);
    Ok(format!(
        "Artifact: {}\nPath: {}\n\n{}",
        entry.id,
        entry.path.display(),
        contents
    ))
}
