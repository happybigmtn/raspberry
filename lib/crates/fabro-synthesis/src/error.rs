use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlueprintError {
    #[error("failed to read blueprint {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse blueprint {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("blueprint {path} is invalid: {message}")]
    Invalid { path: PathBuf, message: String },
    #[error("existing program manifest {path} is missing")]
    MissingProgramManifest { path: PathBuf },
    #[error("failed to interpret existing program manifest {path}: {source}")]
    Manifest {
        path: PathBuf,
        #[source]
        source: raspberry_supervisor::manifest::ManifestError,
    },
    #[error("failed to load run config {path}: {source}")]
    RunConfig {
        path: PathBuf,
        #[source]
        source: anyhow::Error,
    },
    #[error("path {path} is not inside target repo {target_repo}")]
    PathOutsideTargetRepo { path: PathBuf, target_repo: PathBuf },
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error(transparent)]
    Blueprint(#[from] BlueprintError),
    #[error("failed to create directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write file {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize manifest {path}: {source}")]
    ManifestSerialize {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
}
