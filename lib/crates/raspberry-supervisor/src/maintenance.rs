use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::manifest::ProgramManifest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceMode {
    pub enabled: bool,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_by: Option<String>,
}

#[derive(Debug, Error)]
pub enum MaintenanceError {
    #[error("failed to read maintenance state {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse maintenance state {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

pub fn maintenance_path(manifest_path: &Path, manifest: &ProgramManifest) -> PathBuf {
    manifest
        .resolved_target_repo(manifest_path)
        .join(".raspberry")
        .join("maintenance.json")
}

pub fn load_active_maintenance(
    manifest_path: &Path,
    manifest: &ProgramManifest,
) -> Result<Option<MaintenanceMode>, MaintenanceError> {
    let path = maintenance_path(manifest_path, manifest);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).map_err(|source| MaintenanceError::Read {
        path: path.clone(),
        source,
    })?;
    let state: MaintenanceMode =
        serde_json::from_str(&raw).map_err(|source| MaintenanceError::Parse { path, source })?;
    if state.enabled {
        return Ok(Some(state));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_active_maintenance_returns_none_when_disabled() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_path = temp.path().join("malinka/programs/demo.yaml");
        std::fs::create_dir_all(manifest_path.parent().expect("parent")).expect("program dir");
        std::fs::write(
            &manifest_path,
            concat!(
                "version: 1\n",
                "program: demo\n",
                "target_repo: ../..\n",
                "state_path: ../../.raspberry/demo-state.json\n",
                "max_parallel: 1\n",
                "units:\n",
                "  - id: docs\n",
                "    title: Docs\n",
                "    output_root: ../../outputs/docs\n",
                "    artifacts:\n",
                "      - id: plan\n",
                "        path: plan.md\n",
                "    milestones:\n",
                "      - id: reviewed\n",
                "        requires: [plan]\n",
                "    lanes:\n",
                "      - id: lane\n",
                "        title: Docs Lane\n",
                "        kind: artifact\n",
                "        run_config: ../run-configs/bootstrap/docs.toml\n",
                "        managed_milestone: reviewed\n",
                "        produces: [plan]\n",
            ),
        )
        .expect("manifest");
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest");
        let path = maintenance_path(&manifest_path, &manifest);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("raspberry dir");
        std::fs::write(
            &path,
            serde_json::to_string(&MaintenanceMode {
                enabled: false,
                reason: "done".to_string(),
                set_at: None,
                set_by: None,
            })
            .expect("json"),
        )
        .expect("maintenance file");

        let maintenance = load_active_maintenance(&manifest_path, &manifest).expect("loads");
        assert!(maintenance.is_none());
    }
}
