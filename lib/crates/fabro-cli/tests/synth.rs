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

#[test]
fn synth_render_writes_package() {
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
        .stdout(predicate::str::contains("fabro/programs/craps.yaml"));

    assert!(target.join("fabro/programs/craps.yaml").exists());
}

#[test]
fn synth_create_authors_blueprint_from_repo_docs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("zend");
    fs::create_dir_all(target.join("plans")).expect("plans dir");
    fs::create_dir_all(target.join("specs")).expect("specs dir");
    fs::write(target.join("README.md"), "# Zend\n").expect("readme");
    fs::write(target.join("SPEC.md"), "# Root Spec\n").expect("spec");
    fs::write(
        target.join("plans/2026-03-19-build-home-command-center.md"),
        "# Build the Zend Home Command Center\n\n- [ ] Create the first honest slice\n",
    )
    .expect("plan");
    fs::write(
        target.join("specs/2026-03-19-zend-product-spec.md"),
        "# Zend Product Spec\n",
    )
    .expect("product spec");

    fabro()
        .args([
            "synth",
            "create",
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
            "--program",
            "zend",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mode: create"))
        .stdout(predicate::str::contains("Blueprint:"))
        .stdout(predicate::str::contains(
            "selected lane `home-command-center` -> `bootstrap`",
        ))
        .stdout(predicate::str::contains("fabro/programs/zend.yaml"));

    let blueprint =
        fs::read_to_string(target.join("fabro/blueprints/zend.yaml")).expect("blueprint exists");
    assert!(blueprint.contains("id: zend"));
    assert!(blueprint.contains("template: bootstrap"));
    assert!(target.join("fabro/programs/zend.yaml").exists());
}

#[test]
fn synth_import_writes_blueprint() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output = temp.path().join("myosu-update.yaml");

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
        .stdout(predicate::str::contains("Mode: import"))
        .stdout(predicate::str::contains("Blueprint:"));

    let blueprint = fs::read_to_string(&output).expect("blueprint exists");
    assert!(blueprint.contains("program:"));
    assert!(blueprint.contains("id: myosu-update"));
    assert!(blueprint.contains("id: poker"));
}

#[test]
fn synth_evolve_updates_existing_package() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("repo");
    copy_dir(&fixture("update-myosu/current"), &target).expect("copy current fixture");
    copy_dir(&fixture("update-myosu/doctrine"), &target.join("doctrine"))
        .expect("copy doctrine fixture");
    copy_dir(&fixture("update-myosu/evidence"), &target.join("evidence"))
        .expect("copy evidence fixture");
    copy_dir(&fixture("update-myosu/runtime"), &target.join(".raspberry"))
        .expect("copy runtime fixture");
    copy_dir(&fixture("update-myosu/outputs"), &target.join("outputs"))
        .expect("copy outputs fixture");
    let preview = temp.path().join("preview");

    fabro()
        .args([
            "synth",
            "evolve",
            "--blueprint",
            fixture("update-myosu/blueprint.yaml")
                .to_str()
                .expect("utf-8 blueprint path"),
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
            "--preview-root",
            preview.to_str().expect("utf-8 preview path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mode: evolve"))
        .stdout(predicate::str::contains("Preview root:"))
        .stdout(predicate::str::contains("tutorial"))
        .stdout(predicate::str::contains("doctrine input found"))
        .stdout(predicate::str::contains("runtime state found"))
        .stdout(predicate::str::contains("runtime state reports lane"))
        .stdout(predicate::str::contains("artifact missing"))
        .stdout(predicate::str::contains("appears ready for execution"))
        .stdout(predicate::str::contains(
            "implementation follow-on is ready",
        ))
        .stdout(predicate::str::contains("Recommendations:"))
        .stdout(predicate::str::contains(
            "execute the next ready bootstrap lane(s) first: games:tutorial",
        ))
        .stdout(predicate::str::contains(
            "add implementation-family packages in this order: games:poker",
        ))
        .stdout(predicate::str::contains(
            "myosu-games-poker-implementation.yaml",
        ))
        .stdout(predicate::str::contains(
            "plus an implementation-family package for `games:poker`",
        ))
        .stdout(predicate::str::contains(
            "fabro/run-configs/implement/poker.toml",
        ));

    let manifest = fs::read_to_string(target.join("fabro/programs/myosu-update.yaml"))
        .expect("manifest exists");
    assert!(!manifest.contains("id: tutorial"));

    let preview_manifest = fs::read_to_string(preview.join("fabro/programs/myosu-update.yaml"))
        .expect("preview manifest exists");
    assert!(preview_manifest.contains("id: tutorial"));

    let preview_implementation_manifest =
        fs::read_to_string(preview.join("fabro/programs/myosu-games-poker-implementation.yaml"))
            .expect("preview implementation manifest exists");
    assert!(preview_implementation_manifest.contains("program: myosu-games-poker-implementation"));

    assert!(preview
        .join("fabro/run-configs/implement/poker.toml")
        .exists());
    assert!(preview
        .join("fabro/workflows/implement/poker.fabro")
        .exists());

    let original_workflow = fs::read_to_string(fixture(
        "update-myosu/current/fabro/workflows/bootstrap/poker.fabro",
    ))
    .expect("original workflow exists");
    let preview_workflow =
        fs::read_to_string(preview.join("fabro/workflows/bootstrap/poker.fabro"))
            .expect("preview workflow exists");
    assert_eq!(preview_workflow, original_workflow);
    let original_polish = fs::read_to_string(fixture(
        "update-myosu/current/fabro/prompts/bootstrap/poker/polish.md",
    ))
    .expect("original polish prompt exists");
    let preview_polish =
        fs::read_to_string(preview.join("fabro/prompts/bootstrap/poker/polish.md"))
            .expect("preview polish prompt exists");
    assert_eq!(preview_polish, original_polish);
}

#[test]
fn synth_evolve_can_import_current_package_without_blueprint_flag() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("repo");
    copy_dir(&fixture("update-myosu/current"), &target).expect("copy current fixture");
    copy_dir(&fixture("update-myosu/doctrine"), &target.join("doctrine"))
        .expect("copy doctrine fixture");
    copy_dir(&fixture("update-myosu/evidence"), &target.join("evidence"))
        .expect("copy evidence fixture");
    copy_dir(&fixture("update-myosu/runtime"), &target.join(".raspberry"))
        .expect("copy runtime fixture");
    copy_dir(&fixture("update-myosu/outputs"), &target.join("outputs"))
        .expect("copy outputs fixture");
    let preview = temp.path().join("preview");

    fabro()
        .args([
            "synth",
            "evolve",
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
            "--preview-root",
            preview.to_str().expect("utf-8 preview path"),
            "--program",
            "myosu-update",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mode: evolve"))
        .stdout(predicate::str::contains("Blueprint:"))
        .stdout(predicate::str::contains(
            "imported existing package for `myosu-update` without additional planning inputs",
        ));

    assert!(target.join("fabro/blueprints/myosu-update.yaml").exists());
    assert!(preview.join("fabro/programs/myosu-update.yaml").exists());
}

#[test]
fn synth_evolve_emits_service_follow_on_with_health_gate() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("repo");
    copy_dir(&fixture("service-follow-on/current"), &target).expect("copy current fixture");
    copy_dir(
        &fixture("service-follow-on/outputs"),
        &target.join("outputs"),
    )
    .expect("copy outputs fixture");
    let preview = temp.path().join("preview");

    fabro()
        .args([
            "synth",
            "evolve",
            "--blueprint",
            fixture("service-follow-on/blueprint.yaml")
                .to_str()
                .expect("utf-8 blueprint path"),
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
            "--preview-root",
            preview.to_str().expect("utf-8 preview path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "myosu-miner-service-implementation.yaml",
        ));

    assert!(preview
        .join("fabro/programs/myosu-miner-service-implementation.yaml")
        .exists());
    let workflow =
        fs::read_to_string(preview.join("fabro/workflows/implement/miner-service.fabro"))
            .expect("service workflow exists");
    assert!(workflow.contains("label=\"Health\""));
    assert!(workflow.contains("curl http://{ip}:{port}/health"));

    let review_prompt =
        fs::read_to_string(preview.join("fabro/prompts/implement/miner-service/review.md"))
            .expect("service review prompt exists");
    assert!(review_prompt.contains("First health gate"));
    assert!(review_prompt.contains("Health surfaces to preserve"));
    assert!(review_prompt.contains("Observability surfaces to preserve"));
    assert!(review_prompt.contains("Start: **Start slices 1 and 3 immediately**"));
    assert!(review_prompt.contains("Parallel: **Parallelize**: begin `myosu-miner` CLI skeleton"));
}
