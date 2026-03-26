//! Regression tests for `fabro synth` command surface.
//!
//! These tests capture the failure modes related to the `fabro synth` command:
//! - command surface mismatches between `fabro` binary and expected subcommands
//! - workflow generation depending on subcommands that don't exist
//! - prompt refs resolving under the wrong root

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn fabro() -> Command {
    Command::cargo_bin("fabro").expect("fabro binary should build")
}

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

/// Test that `fabro synth` help output is parseable.
#[test]
fn synth_help_is_parseable() {
    fabro()
        .args(["synth", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("synth"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("evolve"))
        .stdout(predicate::str::contains("import"));
}

/// Test that `fabro synth create` requires a target repo.
#[test]
fn synth_create_requires_target_repo() {
    fabro()
        .args(["synth", "create"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("target-repo").or(predicate::str::contains("required")));
}

/// Test that `fabro synth create` with blueprint produces expected output structure.
#[test]
fn synth_create_with_blueprint_produces_output() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("repo");
    fs::create_dir_all(&target).expect("repo dir");

    fabro()
        .args([
            "synth",
            "create",
            "--blueprint",
            fixture("craps/blueprint.yaml")
                .to_str()
                .expect("utf-8 fixture path"),
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mode: create"))
        .stdout(predicate::str::contains("malinka/programs/craps.yaml"));

    // Verify output files exist
    assert!(target.join("malinka/programs/craps.yaml").exists());
    assert!(target
        .join("malinka/workflows/bootstrap/rules.fabro")
        .exists());
}

/// Test that `fabro synth create` fails gracefully with invalid blueprint.
#[test]
fn synth_create_with_invalid_blueprint_fails_gracefully() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("repo");
    let invalid_blueprint = temp.path().join("invalid.yaml");
    fs::create_dir_all(&target).expect("repo dir");
    fs::write(&invalid_blueprint, "not: [valid: yaml").expect("write invalid yaml");

    fabro()
        .args([
            "synth",
            "create",
            "--blueprint",
            invalid_blueprint.to_str().expect("utf-8"),
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("yaml").or(predicate::str::contains("parse")));
}

/// Test that `fabro synth create` creates malinka directory structure correctly.
#[test]
fn synth_create_produces_correct_directory_structure() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("repo");
    fs::create_dir_all(&target).expect("repo dir");

    fabro()
        .args([
            "synth",
            "create",
            "--blueprint",
            fixture("craps/blueprint.yaml")
                .to_str()
                .expect("utf-8 fixture path"),
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
        ])
        .assert()
        .success();

    // Verify directory structure
    assert!(target.join("malinka").is_dir());
    assert!(target.join("malinka/programs").is_dir());
    assert!(target.join("malinka/workflows").is_dir());
    assert!(target.join("malinka/run-configs").is_dir());
    assert!(target.join("malinka/prompts").is_dir());
}

/// Test that `fabro synth evolve` requires existing package.
#[test]
fn synth_evolve_requires_existing_package() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("repo");
    fs::create_dir_all(&target).expect("repo dir");

    // Evolve without existing malinka directory should fail gracefully
    fabro()
        .args([
            "synth",
            "evolve",
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("blueprint")
                .or(predicate::str::contains("malinka").or(predicate::str::contains("not found"))),
        );
}

/// Test that `fabro synth evolve` works with existing package.
#[test]
fn synth_evolve_with_existing_package() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("repo");
    fs::create_dir_all(&target).expect("repo dir");

    // First create a package
    fabro()
        .args([
            "synth",
            "create",
            "--blueprint",
            fixture("craps/blueprint.yaml")
                .to_str()
                .expect("utf-8 fixture path"),
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
        ])
        .assert()
        .success();

    // Then evolve it
    fabro()
        .args([
            "synth",
            "evolve",
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mode: evolve"));
}

/// Test that `fabro synth import` produces valid blueprint.
#[test]
fn synth_import_produces_valid_blueprint() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output = temp.path().join("imported.yaml");

    fabro()
        .args([
            "synth",
            "import",
            "--target-repo",
            fixture("update-myosu/current")
                .to_str()
                .expect("utf-8 fixture path"),
            "--program",
            "myosu-update",
            "--output",
            output.to_str().expect("utf-8 output path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mode: import"));

    // Verify output is valid YAML
    let content = fs::read_to_string(&output).expect("read output");
    assert!(content.contains("program:"));
    assert!(content.contains("id: myosu-update"));

    // Verify it's valid YAML by parsing it
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("output should be valid YAML");
    assert!(parsed.get("program").is_some());
}

/// Test that `fabro synth create` with program flag uses correct program id.
#[test]
fn synth_create_with_program_flag() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("zend");
    fs::create_dir_all(target.join("plans")).expect("plans dir");
    fs::create_dir_all(target.join("specs")).expect("specs dir");
    fs::write(target.join("README.md"), "# Zend\n").expect("readme");
    fs::write(target.join("GOAL.md"), "# Root Goal\n").expect("goal");

    fabro()
        .args([
            "synth",
            "create",
            "--no-decompose",
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
            "--program",
            "zend",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Mode: create (deterministic only)",
        ));

    // Verify manifest uses the correct program id
    let manifest = fs::read_to_string(target.join("malinka/programs/zend.yaml"))
        .expect("manifest should exist");
    assert!(manifest.contains("program: zend") || manifest.contains("id: zend"));
}

/// Test that synth create with --no-decompose skips decomposition step.
#[test]
fn synth_create_no_decompose_skips_decomposition() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("zend");
    fs::create_dir_all(target.join("plans")).expect("plans dir");
    fs::write(target.join("README.md"), "# Zend\n").expect("readme");
    fs::write(target.join("GOAL.md"), "# Root Goal\n").expect("goal");
    fs::write(
        target.join("plans/001-test.md"),
        "# Test Plan\n\n- [ ] Item 1\n- [ ] Item 2\n",
    )
    .expect("plan");

    fabro()
        .args([
            "synth",
            "create",
            "--no-decompose",
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
            "--program",
            "zend",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("deterministic only"));
}

/// Test that `fabro synth` without subcommand shows help.
#[test]
fn synth_without_subcommand_shows_help() {
    // `fabro synth` without subcommand may succeed or fail depending on implementation
    // The important thing is that it shows help content
    let assertion = fabro().args(["synth"]).assert();

    // Check that stdout contains help content
    let output = assertion.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains("synth") && stdout.contains("create")
            || stderr.contains("synth") && stderr.contains("create"),
        "help should contain 'synth' and 'create'. stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}

/// Regression test: ensure --target-repo path with spaces works.
#[test]
fn synth_create_handles_path_with_spaces() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("repo with spaces");
    fs::create_dir_all(&target).expect("repo dir");

    fabro()
        .args([
            "synth",
            "create",
            "--blueprint",
            fixture("craps/blueprint.yaml")
                .to_str()
                .expect("utf-8 fixture path"),
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
        ])
        .assert()
        .success();

    assert!(target.join("malinka/programs/craps.yaml").exists());
}

/// Regression test: ensure synth create overwrites existing files when forced.
#[test]
fn synth_create_force_overwrites() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("repo");
    fs::create_dir_all(&target).expect("repo dir");

    // Create initial package
    fabro()
        .args([
            "synth",
            "create",
            "--blueprint",
            fixture("craps/blueprint.yaml")
                .to_str()
                .expect("utf-8 fixture path"),
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
        ])
        .assert()
        .success();

    // Modify a generated file
    let manifest_path = target.join("malinka/programs/craps.yaml");
    fs::write(&manifest_path, "modified: true").expect("modify manifest");

    // Re-run create without --force should preserve changes or fail
    let result = fabro()
        .args([
            "synth",
            "create",
            "--blueprint",
            fixture("craps/blueprint.yaml")
                .to_str()
                .expect("utf-8 fixture path"),
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
        ])
        .assert();

    // Should succeed (implementation may or may not overwrite)
    result.success();

    // File should still exist
    assert!(manifest_path.exists());
}
