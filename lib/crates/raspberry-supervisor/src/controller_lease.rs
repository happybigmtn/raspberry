use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::manifest::ProgramManifest;

const AUTODEV_LEASE_SCHEMA_VERSION: &str = "raspberry.autodev-lease.v1";

#[derive(Debug, Error)]
pub enum ControllerLeaseError {
    #[error("failed to create autodev lease dir {path}: {source}")]
    CreateLeaseDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read autodev lease file {path}: {source}")]
    ReadLease {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse autodev lease file {path}: {source}")]
    ParseLease {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize autodev lease file {path}: {source}")]
    SerializeLease {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to write autodev lease file {path}: {source}")]
    WriteLease {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "autodev controller already running for program `{program}` via pid={pid} \
         (acquired_at={acquired_at})"
    )]
    AlreadyRunning {
        path: PathBuf,
        program: String,
        pid: u32,
        acquired_at: DateTime<Utc>,
    },
}

#[derive(Debug)]
pub struct AutodevLeaseGuard {
    path: PathBuf,
    lease: AutodevLease,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AutodevLease {
    schema_version: String,
    program: String,
    manifest: PathBuf,
    pid: u32,
    process_start_ticks: Option<u64>,
    acquired_at: DateTime<Utc>,
}

pub fn acquire_autodev_lease(
    manifest_path: &Path,
    manifest: &ProgramManifest,
) -> Result<AutodevLeaseGuard, ControllerLeaseError> {
    let path = autodev_lease_path(manifest_path, manifest);
    let Some(parent) = path.parent() else {
        return Err(ControllerLeaseError::CreateLeaseDir {
            path: path.clone(),
            source: std::io::Error::other("missing lease parent directory"),
        });
    };
    fs::create_dir_all(parent).map_err(|source| ControllerLeaseError::CreateLeaseDir {
        path: parent.to_path_buf(),
        source,
    })?;

    let lease = AutodevLease {
        schema_version: AUTODEV_LEASE_SCHEMA_VERSION.to_string(),
        program: manifest.program.clone(),
        manifest: manifest_path.to_path_buf(),
        pid: std::process::id(),
        process_start_ticks: current_process_start_ticks(),
        acquired_at: Utc::now(),
    };

    if try_write_lease(&path, &lease)? {
        return Ok(AutodevLeaseGuard { path, lease });
    }

    let existing = load_lease(&path)?;
    if autodev_owner_active(&existing) {
        return Err(ControllerLeaseError::AlreadyRunning {
            path,
            program: existing.program,
            pid: existing.pid,
            acquired_at: existing.acquired_at,
        });
    }

    fs::remove_file(&path).map_err(|source| ControllerLeaseError::WriteLease {
        path: path.clone(),
        source,
    })?;
    if try_write_lease(&path, &lease)? {
        return Ok(AutodevLeaseGuard { path, lease });
    }

    let existing = load_lease(&path)?;
    Err(ControllerLeaseError::AlreadyRunning {
        path,
        program: existing.program,
        pid: existing.pid,
        acquired_at: existing.acquired_at,
    })
}

impl Drop for AutodevLeaseGuard {
    fn drop(&mut self) {
        let Ok(existing) = load_lease(&self.path) else {
            return;
        };
        if existing != self.lease {
            return;
        }
        let _ = fs::remove_file(&self.path);
    }
}

fn autodev_owner_active(lease: &AutodevLease) -> bool {
    let proc_path = PathBuf::from("/proc").join(lease.pid.to_string());
    if !proc_path.exists() {
        return false;
    }

    let Some(expected_ticks) = lease.process_start_ticks else {
        return true;
    };
    process_start_ticks_for_pid(lease.pid) == Some(expected_ticks)
}

fn current_process_start_ticks() -> Option<u64> {
    process_start_ticks_for_pid(std::process::id())
}

fn process_start_ticks_for_pid(pid: u32) -> Option<u64> {
    let path = PathBuf::from("/proc").join(pid.to_string()).join("stat");
    let raw = fs::read_to_string(path).ok()?;
    let close_paren = raw.rfind(')')?;
    let rest = raw.get(close_paren + 1..)?.trim();
    rest.split_whitespace().nth(19)?.parse().ok()
}

fn try_write_lease(path: &Path, lease: &AutodevLease) -> Result<bool, ControllerLeaseError> {
    let json =
        serde_json::to_string_pretty(lease).map_err(|source| ControllerLeaseError::SerializeLease {
            path: path.to_path_buf(),
            source,
        })?;
    match OpenOptions::new().create_new(true).write(true).open(path) {
        Ok(mut file) => {
            file.write_all(json.as_bytes())
                .map_err(|source| ControllerLeaseError::WriteLease {
                    path: path.to_path_buf(),
                    source,
                })?;
            Ok(true)
        }
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(source) => Err(ControllerLeaseError::WriteLease {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn load_lease(path: &Path) -> Result<AutodevLease, ControllerLeaseError> {
    let raw = fs::read_to_string(path).map_err(|source| ControllerLeaseError::ReadLease {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| ControllerLeaseError::ParseLease {
        path: path.to_path_buf(),
        source,
    })
}

fn autodev_lease_path(manifest_path: &Path, manifest: &ProgramManifest) -> PathBuf {
    manifest
        .resolved_target_repo(manifest_path)
        .join(".raspberry")
        .join(format!("{}-autodev.lock", manifest.program))
}

#[cfg(test)]
mod tests {
    use super::{acquire_autodev_lease, autodev_lease_path};
    use crate::manifest::ProgramManifest;
    use chrono::Utc;
    use std::fs;

    fn write_manifest(temp: &tempfile::TempDir) -> std::path::PathBuf {
        let path = temp.path().join("program.yaml");
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../test/fixtures/raspberry-supervisor/program.yaml");
        fs::copy(&fixture_path, &path).expect("manifest copied");
        path
    }

    #[test]
    fn acquire_autodev_lease_rejects_duplicate_active_owner() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_path = write_manifest(&temp);
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");

        let _lease = acquire_autodev_lease(&manifest_path, &manifest).expect("lease acquired");
        let err = acquire_autodev_lease(&manifest_path, &manifest).expect_err("duplicate fails");

        assert!(
            err.to_string()
                .contains("autodev controller already running for program `raspberry-demo`")
        );
    }

    #[test]
    fn acquire_autodev_lease_reclaims_stale_owner() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_path = write_manifest(&temp);
        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let path = autodev_lease_path(&manifest_path, &manifest);
        fs::create_dir_all(path.parent().expect("parent")).expect("lease dir");
        fs::write(
            &path,
            serde_json::json!({
                "schema_version": "raspberry.autodev-lease.v1",
                "program": "raspberry-demo",
                "manifest": manifest_path,
                "pid": 999_999,
                "process_start_ticks": 1,
                "acquired_at": Utc::now(),
            })
            .to_string(),
        )
        .expect("stale lease written");

        let lease = acquire_autodev_lease(&manifest_path, &manifest).expect("lease acquired");
        let saved = fs::read_to_string(&path).expect("lease saved");

        assert!(saved.contains(&std::process::id().to_string()));
        drop(lease);
        assert!(!path.exists());
    }
}
