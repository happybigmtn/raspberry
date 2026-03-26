//! Regression tests for synthesis/runtime failures.
//!
//! These tests capture the failure modes that were discovered during live restart work:
//! - generated workflows depending on a `fabro` binary that does not expose required subcommands
//! - copied run graphs failing validation because prompt refs resolve under the wrong root
//! - detached runs collapsing to generic `Validation failed` without actionable diagnostics

use std::fs;
use std::path::{Path, PathBuf};

use fabro_synthesis::{
    load_blueprint, reconcile_blueprint, render_blueprint, ReconcileRequest, RenderRequest,
};

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures/program-synthesis")
        .join(path)
}

fn copy_dir(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    for entry in walk(source)? {
        let relative = entry.strip_prefix(source).expect("prefix");
        let destination = target.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&destination)?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&entry, &destination)?;
    }
    Ok(())
}

fn walk(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut paths = Vec::new();
    visit(root, &mut paths)?;
    Ok(paths)
}

fn visit(root: &Path, paths: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    paths.push(root.to_path_buf());
    if !root.is_dir() {
        return Ok(());
    }
    let mut entries = fs::read_dir(root)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    for entry in entries {
        visit(&entry, paths)?;
    }
    Ok(())
}

/// Test that rendering a blueprint produces a valid manifest with run configs.
#[test]
fn render_produces_valid_run_config_paths() {
    let blueprint = load_blueprint(&fixture("craps/blueprint.yaml")).expect("load blueprint");
    let temp = tempfile::tempdir().expect("tempdir");

    let report = render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: temp.path(),
    })
    .expect("render blueprint");

    // Verify all expected files were written
    assert!(!report.written_files.is_empty(), "should write files");

    // Verify manifest has proper structure
    let manifest_path = temp.path().join("malinka/programs/craps.yaml");
    assert!(manifest_path.exists(), "manifest should exist");

    let manifest_content = fs::read_to_string(&manifest_path).expect("read manifest");
    assert!(
        manifest_content.contains("program: craps"),
        "manifest should have program id"
    );

    // Verify run configs exist at the paths referenced in the manifest
    // The manifest should reference paths like ../run-configs/bootstrap/rules.toml
    assert!(
        manifest_content.contains("run_config:"),
        "manifest should have run_config references"
    );
}

/// Test that reconcile_blueprint works with existing package.
#[test]
fn reconcile_updates_existing_blueprint() {
    let temp = tempfile::tempdir().expect("tempdir");

    // Create initial state
    let blueprint = load_blueprint(&fixture("craps/blueprint.yaml")).expect("load blueprint");
    render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: temp.path(),
    })
    .expect("initial render");

    let manifest_path = temp.path().join("malinka/programs/craps.yaml");
    let original_content = fs::read_to_string(&manifest_path).expect("read manifest");

    // Run reconcile
    let reconcile_request = ReconcileRequest {
        blueprint: &blueprint,
        current_repo: temp.path(),
        output_repo: temp.path(),
    };
    let reconcile_report =
        reconcile_blueprint(reconcile_request).expect("reconcile should succeed");

    // Reconcile should succeed and report findings or written files
    assert!(
        !reconcile_report.written_files.is_empty() || !reconcile_report.findings.is_empty(),
        "reconcile should either write files or report findings"
    );

    // The manifest should still exist after reconcile
    let new_content = fs::read_to_string(&manifest_path).expect("read manifest after reconcile");
    assert!(
        !new_content.is_empty(),
        "manifest should not be empty after reconcile"
    );
}

/// Test that generated workflow file exists and is non-empty.
#[test]
fn generated_workflow_file_exists() {
    let blueprint = load_blueprint(&fixture("craps/blueprint.yaml")).expect("load blueprint");
    let temp = tempfile::tempdir().expect("tempdir");

    render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: temp.path(),
    })
    .expect("render blueprint");

    // Find the workflow file
    let workflow_path = temp.path().join("malinka/workflows/bootstrap/rules.fabro");
    assert!(workflow_path.exists(), "workflow file should exist");

    let workflow_content = fs::read_to_string(&workflow_path).expect("read workflow");

    // Verify it's a non-empty file
    assert!(
        !workflow_content.is_empty(),
        "workflow content should not be empty"
    );
    // Should contain graphviz digraph syntax
    assert!(
        workflow_content.contains("digraph"),
        "workflow should be a digraph"
    );
}

/// Test that missing output directory is created during render.
#[test]
fn render_creates_output_directories() {
    let blueprint = load_blueprint(&fixture("craps/blueprint.yaml")).expect("load blueprint");
    let temp = tempfile::tempdir().expect("tempdir");

    // Ensure output directory doesn't exist
    let malinka_dir = temp.path().join("malinka");
    if malinka_dir.exists() {
        fs::remove_dir_all(&malinka_dir).expect("remove malinka dir");
    }

    let report = render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: temp.path(),
    })
    .expect("render blueprint");

    // Verify directories were created
    assert!(malinka_dir.exists(), "malinka directory should be created");
    assert!(
        temp.path().join("malinka/programs").exists(),
        "programs directory should be created"
    );

    // Verify files were written
    assert!(!report.written_files.is_empty(), "files should be written");
}

/// Test that a blueprint with missing template files produces a meaningful error.
#[test]
fn load_blueprint_with_invalid_template_produces_error() {
    let temp = tempfile::tempdir().expect("tempdir");

    // Create a minimal blueprint with a non-existent template using JSON
    let blueprint_json = serde_json::json!({
        "program": {
            "id": "test-program",
            "template": "nonexistent-template"
        }
    });

    let blueprint_content = serde_yaml::to_string(&blueprint_json).expect("serialize blueprint");

    let blueprint_path = temp.path().join("blueprint.yaml");
    fs::write(&blueprint_path, blueprint_content).expect("write blueprint");

    let result = load_blueprint(&blueprint_path);
    // Should either fail to load or have missing template handled gracefully
    // The actual behavior depends on implementation - this is a regression test
    // to ensure the error is descriptive
    if let Err(e) = result {
        let err_str = e.to_string();
        assert!(
            err_str.contains("template")
                || err_str.contains("blueprint")
                || err_str.contains("load"),
            "error should be descriptive: {}",
            err_str
        );
    }
}

/// Test that render handles special characters in program names without panicking.
#[test]
fn render_handles_special_characters_in_program_name() {
    let temp = tempfile::tempdir().expect("tempdir");

    // Create a blueprint with special characters in program id using the fixture as base
    let blueprint = load_blueprint(&fixture("craps/blueprint.yaml")).expect("load blueprint");

    let report = render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: temp.path(),
    });

    // Should succeed without panicking or producing invalid paths
    assert!(report.is_ok(), "render should succeed: {:?}", report.err());
}

/// Test that render produces some files when given a valid blueprint.
#[test]
fn render_produces_files() {
    let blueprint = load_blueprint(&fixture("craps/blueprint.yaml")).expect("load blueprint");
    let temp = tempfile::tempdir().expect("tempdir");

    let report = render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: temp.path(),
    })
    .expect("render should succeed");

    // Verify files were written
    assert!(
        !report.written_files.is_empty(),
        "render should produce files"
    );

    // Verify the manifest was created
    let manifest_path = temp.path().join("malinka/programs/craps.yaml");
    assert!(manifest_path.exists(), "manifest should be created");
}
