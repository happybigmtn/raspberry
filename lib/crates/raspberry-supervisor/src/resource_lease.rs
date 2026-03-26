use std::collections::{BTreeSet, HashMap};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

use chrono::{DateTime, Utc};
use fabro_config::run::resolve_graph_path;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const RESOURCE_LEASE_SCHEMA_VERSION: &str = "raspberry.resource-lease.v2";
const ZEND_DAEMON_PORT_START: u16 = 18080;
const ZEND_DAEMON_PORT_END: u16 = 18129;

#[derive(Debug, Error)]
pub enum ResourceLeaseError {
    #[error("failed to read run config {path}: {source}")]
    ReadRunConfig {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read workflow graph {path}: {source}")]
    ReadWorkflowGraph {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create lease dir {path}: {source}")]
    CreateLeaseDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read lease file {path}: {source}")]
    ReadLease {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse lease file {path}: {source}")]
    ParseLease {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize lease file {path}: {source}")]
    SerializeLease {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to write lease file {path}: {source}")]
    WriteLease {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("no available Zend daemon ports in the lease range")]
    NoAvailableZendDaemonPort,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ResourceLease {
    schema_version: String,
    resource: String,
    lane_key: String,
    port: u16,
    #[serde(default)]
    owner_pid: Option<u32>,
    #[serde(default)]
    owner_process_start_ticks: Option<u64>,
    acquired_at: DateTime<Utc>,
}

pub fn env_for_run_config(
    target_repo: &Path,
    lane_key: &str,
    run_config: &Path,
) -> Result<Option<HashMap<String, String>>, ResourceLeaseError> {
    if run_config.extension().and_then(|ext| ext.to_str()) != Some("toml") {
        return Ok(None);
    }
    let raw =
        fs::read_to_string(run_config).map_err(|source| ResourceLeaseError::ReadRunConfig {
            path: run_config.to_path_buf(),
            source,
        })?;
    if !run_config_requests_zend_daemon_env(&raw)
        && !workflow_graph_requests_zend_daemon_env(run_config, &raw)?
    {
        return Ok(None);
    }

    let lease = acquire_zend_daemon_lease(target_repo, lane_key)?;
    let mut env = HashMap::new();
    env.insert(
        "FABRO_PASSTHROUGH_ENV".to_string(),
        "ZEND_BIND_HOST,ZEND_BIND_PORT,ZEND_DAEMON_URL".to_string(),
    );
    env.insert("ZEND_BIND_HOST".to_string(), "127.0.0.1".to_string());
    env.insert("ZEND_BIND_PORT".to_string(), lease.port.to_string());
    env.insert(
        "ZEND_DAEMON_URL".to_string(),
        format!("http://127.0.0.1:{}", lease.port),
    );
    Ok(Some(env))
}

pub fn release_for_lane(target_repo: &Path, lane_key: &str) -> Result<bool, ResourceLeaseError> {
    let root = lease_root(target_repo);
    if !root.is_dir() {
        return Ok(false);
    }
    let mut changed = false;
    for entry in fs::read_dir(&root).map_err(|source| ResourceLeaseError::CreateLeaseDir {
        path: root.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| ResourceLeaseError::CreateLeaseDir {
            path: root.clone(),
            source,
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let lease = load_lease(&path)?;
        if lease.lane_key != lane_key {
            continue;
        }
        fs::remove_file(&path).map_err(|source| ResourceLeaseError::WriteLease {
            path: path.clone(),
            source,
        })?;
        changed = true;
    }
    Ok(changed)
}

pub fn cleanup_leases(
    target_repo: &Path,
    running_lane_keys: &BTreeSet<String>,
) -> Result<bool, ResourceLeaseError> {
    let root = lease_root(target_repo);
    if !root.is_dir() {
        return Ok(false);
    }

    let mut changed = false;
    for entry in fs::read_dir(&root).map_err(|source| ResourceLeaseError::CreateLeaseDir {
        path: root.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| ResourceLeaseError::CreateLeaseDir {
            path: root.clone(),
            source,
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let lease = load_lease(&path)?;
        if running_lane_keys.contains(&lease.lane_key) && resource_lease_owner_active(&lease) {
            continue;
        }
        fs::remove_file(&path).map_err(|source| ResourceLeaseError::WriteLease {
            path: path.clone(),
            source,
        })?;
        changed = true;
    }

    Ok(changed)
}

fn acquire_zend_daemon_lease(
    target_repo: &Path,
    lane_key: &str,
) -> Result<ResourceLease, ResourceLeaseError> {
    let root = lease_root(target_repo);
    fs::create_dir_all(&root).map_err(|source| ResourceLeaseError::CreateLeaseDir {
        path: root.clone(),
        source,
    })?;

    for port in ZEND_DAEMON_PORT_START..=ZEND_DAEMON_PORT_END {
        let path = lease_path(&root, port);
        if !path.exists() {
            continue;
        }
        let lease = load_lease(&path)?;
        if lease.lane_key == lane_key && resource_lease_owner_active(&lease) {
            return Ok(lease);
        }
        remove_stale_lease_if_needed(&path, &lease)?;
    }

    for port in ZEND_DAEMON_PORT_START..=ZEND_DAEMON_PORT_END {
        let path = lease_path(&root, port);
        if path.exists() {
            let lease = load_lease(&path)?;
            remove_stale_lease_if_needed(&path, &lease)?;
            if path.exists() {
                continue;
            }
        }
        let lease = ResourceLease {
            schema_version: RESOURCE_LEASE_SCHEMA_VERSION.to_string(),
            resource: "zend_daemon_port".to_string(),
            lane_key: lane_key.to_string(),
            port,
            owner_pid: Some(process::id()),
            owner_process_start_ticks: current_process_start_ticks(),
            acquired_at: Utc::now(),
        };
        if try_write_lease(&path, &lease)? {
            return Ok(lease);
        }
    }

    Err(ResourceLeaseError::NoAvailableZendDaemonPort)
}

fn run_config_requests_zend_daemon_env(raw: &str) -> bool {
    raw.contains("${env.ZEND_BIND_PORT}")
        || raw.contains("${env.ZEND_DAEMON_URL}")
        || raw.contains("${env.ZEND_BIND_HOST}")
}

fn workflow_graph_requests_zend_daemon_env(
    run_config: &Path,
    raw_run_config: &str,
) -> Result<bool, ResourceLeaseError> {
    let Some(graph) = graph_value_from_run_config(raw_run_config) else {
        return Ok(false);
    };
    let graph_path = resolve_graph_path(run_config, &graph);
    let raw = fs::read_to_string(&graph_path).map_err(|source| {
        ResourceLeaseError::ReadWorkflowGraph {
            path: graph_path,
            source,
        }
    })?;
    Ok(workflow_graph_contains_zend_daemon_usage(&raw))
}

fn workflow_graph_contains_zend_daemon_usage(raw: &str) -> bool {
    raw.contains("bootstrap_home_miner.sh")
        || raw.contains("pair_gateway_client.sh")
        || raw.contains("read_miner_status.sh")
        || raw.contains("set_mining_mode.sh")
        || raw.contains("127.0.0.1:8080")
}

fn graph_value_from_run_config(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || !trimmed.starts_with("graph") {
            continue;
        }
        let (_, value) = trimmed.split_once('=')?;
        let value = value
            .split('#')
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn try_write_lease(path: &Path, lease: &ResourceLease) -> Result<bool, ResourceLeaseError> {
    let json = serde_json::to_string_pretty(lease).map_err(|source| {
        ResourceLeaseError::SerializeLease {
            path: path.to_path_buf(),
            source,
        }
    })?;
    match OpenOptions::new().create_new(true).write(true).open(path) {
        Ok(mut file) => {
            file.write_all(json.as_bytes())
                .map_err(|source| ResourceLeaseError::WriteLease {
                    path: path.to_path_buf(),
                    source,
                })?;
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(source) => Err(ResourceLeaseError::WriteLease {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn load_lease(path: &Path) -> Result<ResourceLease, ResourceLeaseError> {
    let raw = fs::read_to_string(path).map_err(|source| ResourceLeaseError::ReadLease {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| ResourceLeaseError::ParseLease {
        path: path.to_path_buf(),
        source,
    })
}

fn lease_root(target_repo: &Path) -> PathBuf {
    target_repo.join(".raspberry").join("resource-leases")
}

fn lease_path(root: &Path, port: u16) -> PathBuf {
    root.join(format!("zend-daemon-port-{port}.json"))
}

fn remove_stale_lease_if_needed(
    path: &Path,
    lease: &ResourceLease,
) -> Result<(), ResourceLeaseError> {
    if resource_lease_owner_active(lease) {
        return Ok(());
    }
    fs::remove_file(path).map_err(|source| ResourceLeaseError::WriteLease {
        path: path.to_path_buf(),
        source,
    })
}

fn resource_lease_owner_active(lease: &ResourceLease) -> bool {
    let Some(pid) = lease.owner_pid else {
        return false;
    };
    let proc_path = PathBuf::from("/proc").join(pid.to_string());
    if !proc_path.exists() {
        return false;
    }
    let Some(expected_ticks) = lease.owner_process_start_ticks else {
        return true;
    };
    process_start_ticks_for_pid(pid) == Some(expected_ticks)
}

fn current_process_start_ticks() -> Option<u64> {
    process_start_ticks_for_pid(process::id())
}

fn process_start_ticks_for_pid(pid: u32) -> Option<u64> {
    let path = PathBuf::from("/proc").join(pid.to_string()).join("stat");
    let raw = fs::read_to_string(path).ok()?;
    let close_paren = raw.rfind(')')?;
    let rest = raw.get(close_paren + 1..)?.trim();
    rest.split_whitespace().nth(19)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_for_run_config_allocates_zend_daemon_port() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workflow = temp.path().join("workflow.fabro");
        let run_config = temp.path().join("run.toml");
        fs::write(
            &workflow,
            "digraph demo { graph [goal=\"./scripts/bootstrap_home_miner.sh\"] }",
        )
        .expect("workflow");
        fs::write(
            &run_config,
            format!("version = 1\ngraph = \"{}\"\n", workflow.display()),
        )
        .expect("run config");

        let env = env_for_run_config(temp.path(), "demo:lane", &run_config)
            .expect("lease env")
            .expect("env should be present");

        assert_eq!(
            env.get("FABRO_PASSTHROUGH_ENV").map(String::as_str),
            Some("ZEND_BIND_HOST,ZEND_BIND_PORT,ZEND_DAEMON_URL")
        );
        assert_eq!(
            env.get("ZEND_BIND_HOST").map(String::as_str),
            Some("127.0.0.1")
        );
        assert!(env.contains_key("ZEND_BIND_PORT"));
        assert!(env.contains_key("ZEND_DAEMON_URL"));
    }

    #[test]
    fn env_for_program_manifest_skips_leasing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest = temp.path().join("program.yaml");
        fs::write(&manifest, "version: 1\nprogram: demo\nunits: []\n").expect("manifest");

        let env = env_for_run_config(temp.path(), "demo:program", &manifest)
            .expect("lease lookup should succeed");

        assert!(env.is_none());
    }

    #[test]
    fn env_for_run_config_does_not_require_unrelated_host_env() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workflow = temp.path().join("workflow.fabro");
        let run_config = temp.path().join("run.toml");
        fs::write(
            &workflow,
            "digraph demo { graph [goal=\"./scripts/bootstrap_home_miner.sh\"] }",
        )
        .expect("workflow");
        fs::write(
            &run_config,
            format!(
                "version = 1\ngraph = \"{}\"\n[sandbox.env]\nANTHROPIC_AUTH_TOKEN = \"${{env.MINIMAX_API_KEY}}\"\n",
                workflow.display()
            ),
        )
        .expect("run config");

        let env = env_for_run_config(temp.path(), "demo:lane", &run_config)
            .expect("lease env lookup should succeed");

        assert!(env.is_some());
    }

    #[test]
    fn cleanup_leases_removes_non_running_lanes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = lease_root(temp.path());
        fs::create_dir_all(&root).expect("lease dir");
        let path = lease_path(&root, ZEND_DAEMON_PORT_START);
        let lease = ResourceLease {
            schema_version: RESOURCE_LEASE_SCHEMA_VERSION.to_string(),
            resource: "zend_daemon_port".to_string(),
            lane_key: "demo:lane".to_string(),
            port: ZEND_DAEMON_PORT_START,
            owner_pid: Some(process::id()),
            owner_process_start_ticks: current_process_start_ticks(),
            acquired_at: Utc::now(),
        };
        fs::write(&path, serde_json::to_string_pretty(&lease).expect("json")).expect("write lease");

        let changed = cleanup_leases(temp.path(), &BTreeSet::new()).expect("cleanup");

        assert!(changed);
        assert!(!path.exists());
    }

    #[test]
    fn release_for_lane_removes_matching_lease() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = lease_root(temp.path());
        fs::create_dir_all(&root).expect("lease dir");
        let path = lease_path(&root, ZEND_DAEMON_PORT_START);
        let lease = ResourceLease {
            schema_version: RESOURCE_LEASE_SCHEMA_VERSION.to_string(),
            resource: "zend_daemon_port".to_string(),
            lane_key: "demo:lane".to_string(),
            port: ZEND_DAEMON_PORT_START,
            owner_pid: Some(process::id()),
            owner_process_start_ticks: current_process_start_ticks(),
            acquired_at: Utc::now(),
        };
        fs::write(&path, serde_json::to_string_pretty(&lease).expect("json")).expect("write lease");

        let changed = release_for_lane(temp.path(), "demo:lane").expect("release");

        assert!(changed);
        assert!(!path.exists());
    }

    #[test]
    fn cleanup_leases_reclaims_dead_owner_even_when_lane_looks_running() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = lease_root(temp.path());
        fs::create_dir_all(&root).expect("lease dir");
        let path = lease_path(&root, ZEND_DAEMON_PORT_START);
        let lease = ResourceLease {
            schema_version: RESOURCE_LEASE_SCHEMA_VERSION.to_string(),
            resource: "zend_daemon_port".to_string(),
            lane_key: "demo:lane".to_string(),
            port: ZEND_DAEMON_PORT_START,
            owner_pid: Some(u32::MAX),
            owner_process_start_ticks: Some(u64::MAX),
            acquired_at: Utc::now(),
        };
        fs::write(&path, serde_json::to_string_pretty(&lease).expect("json")).expect("write lease");

        let changed = cleanup_leases(temp.path(), &BTreeSet::from(["demo:lane".to_string()]))
            .expect("cleanup");

        assert!(changed);
        assert!(!path.exists());
    }

    #[test]
    fn acquire_zend_daemon_lease_reclaims_stale_owner() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = lease_root(temp.path());
        fs::create_dir_all(&root).expect("lease dir");
        let path = lease_path(&root, ZEND_DAEMON_PORT_START);
        let stale = ResourceLease {
            schema_version: "raspberry.resource-lease.v1".to_string(),
            resource: "zend_daemon_port".to_string(),
            lane_key: "old:lane".to_string(),
            port: ZEND_DAEMON_PORT_START,
            owner_pid: None,
            owner_process_start_ticks: None,
            acquired_at: Utc::now(),
        };
        fs::write(&path, serde_json::to_string_pretty(&stale).expect("json")).expect("write lease");

        let lease = acquire_zend_daemon_lease(temp.path(), "demo:lane").expect("lease");

        assert_eq!(lease.port, ZEND_DAEMON_PORT_START);
        assert_eq!(lease.lane_key, "demo:lane");
        assert_eq!(lease.owner_pid, Some(process::id()));
    }
}
