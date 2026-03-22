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
        .stdout(predicate::str::contains("malinka/programs/craps.yaml"));

    assert!(target.join("malinka/programs/craps.yaml").exists());
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
        .stdout(predicate::str::contains("malinka/programs/zend.yaml"));

    let blueprint =
        fs::read_to_string(target.join("malinka/blueprints/zend.yaml")).expect("blueprint exists");
    assert!(blueprint.contains("id: zend"));
    assert!(blueprint.contains("template: bootstrap"));
    assert!(target.join("malinka/programs/zend.yaml").exists());
}

#[test]
fn synth_create_writes_plan_mapping_snapshots() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("rxmragent");
    fs::create_dir_all(target.join("plans")).expect("plans dir");
    fs::write(target.join("README.md"), "# rXMRagent\n").expect("readme");
    fs::write(target.join("SPEC.md"), "# Root Spec\n").expect("spec");
    fs::write(target.join("plans/001-master-plan.md"), "# Master Plan\n").expect("master");
    fs::write(
        target.join("plans/005-craps-game.md"),
        concat!(
            "# Craps Game\n\n",
            "- [ ] Milestone 1: casino-core\n",
            "- [ ] Milestone 2: provably-fair\n",
            "- [ ] Milestone 3: house\n",
        ),
    )
    .expect("craps plan");

    fabro()
        .args([
            "synth",
            "create",
            "--no-decompose",
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
            "--program",
            "rxmragent",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "malinka/plan-mappings/005-craps-game.yaml",
        ));

    let mapping = fs::read_to_string(target.join("malinka/plan-mappings/005-craps-game.yaml"))
        .expect("mapping exists");
    assert!(mapping.contains("title: Craps Game"));
    assert!(mapping.contains("category: game"));
    assert!(mapping.contains("mapping_source: heuristic"));
    assert!(mapping.contains("composite: true"));
    assert!(mapping.contains("children:"));
    assert!(mapping.contains("id: casino-core"));
    assert!(mapping.contains("lane_kind: platform"));
    assert!(mapping.contains("id: provably-fair"));
    assert!(mapping.contains("id: house"));
    assert!(mapping.contains("lane_kind: service"));
}

#[test]
fn synth_create_refreshes_heuristic_mappings_when_plan_changes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("rxmragent");
    fs::create_dir_all(target.join("plans")).expect("plans dir");
    fs::write(target.join("README.md"), "# rXMRagent\n").expect("readme");
    fs::write(target.join("SPEC.md"), "# Root Spec\n").expect("spec");
    fs::write(target.join("plans/001-master-plan.md"), "# Master Plan\n").expect("master");
    fs::write(
        target.join("plans/005-craps-game.md"),
        "# Craps Game\n\n- [ ] Milestone 1: casino-core\n- [ ] Milestone 2: house\n",
    )
    .expect("initial plan");

    fabro()
        .args([
            "synth",
            "create",
            "--no-decompose",
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
            "--program",
            "rxmragent",
        ])
        .assert()
        .success();

    fs::write(
        target.join("plans/005-craps-game.md"),
        "# Craps Game\n\n- [ ] Milestone 1: casino-core\n- [ ] Milestone 2: dealer\n",
    )
    .expect("updated plan");

    fabro()
        .args([
            "synth",
            "create",
            "--no-decompose",
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
            "--program",
            "rxmragent",
        ])
        .assert()
        .success();

    let mapping = fs::read_to_string(target.join("malinka/plan-mappings/005-craps-game.yaml"))
        .expect("mapping exists");
    assert!(mapping.contains("id: dealer"));
    assert!(!mapping.contains("id: house"));
}

#[test]
fn synth_create_wipes_existing_fabro_directory_before_regenerating() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("repo");
    fs::create_dir_all(target.join("plans")).expect("plans dir");
    fs::create_dir_all(target.join("malinka/stale")).expect("stale dir");
    fs::write(target.join("README.md"), "# Demo\n").expect("readme");
    fs::write(target.join("SPEC.md"), "# Root Spec\n").expect("spec");
    fs::write(target.join("plans/001-master-plan.md"), "# Master Plan\n").expect("master");
    fs::write(target.join("malinka/stale/old.txt"), "stale\n").expect("stale file");

    fabro()
        .args([
            "synth",
            "create",
            "--target-repo",
            target.to_str().expect("utf-8 target path"),
            "--program",
            "demo",
        ])
        .assert()
        .success();

    assert!(!target.join("malinka/stale/old.txt").exists());
    assert!(target.join("malinka/blueprints/demo.yaml").exists());
    assert!(target.join("malinka/programs/demo.yaml").exists());
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
            "malinka/run-configs/implement/poker.toml",
        ));

    let manifest = fs::read_to_string(target.join("malinka/programs/myosu-update.yaml"))
        .expect("manifest exists");
    assert!(!manifest.contains("id: tutorial"));

    let preview_manifest = fs::read_to_string(preview.join("malinka/programs/myosu-update.yaml"))
        .expect("preview manifest exists");
    assert!(preview_manifest.contains("id: tutorial"));

    let preview_implementation_manifest =
        fs::read_to_string(preview.join("malinka/programs/myosu-games-poker-implementation.yaml"))
            .expect("preview implementation manifest exists");
    assert!(preview_implementation_manifest.contains("program: myosu-games-poker-implementation"));

    assert!(preview
        .join("malinka/run-configs/implement/poker.toml")
        .exists());
    assert!(preview
        .join("malinka/workflows/implement/poker.fabro")
        .exists());

    let original_workflow = fs::read_to_string(fixture(
        "update-myosu/current/fabro/workflows/bootstrap/poker.fabro",
    ))
    .expect("original workflow exists");
    let preview_workflow =
        fs::read_to_string(preview.join("malinka/workflows/bootstrap/poker.fabro"))
            .expect("preview workflow exists");
    assert_eq!(preview_workflow, original_workflow);
    let original_polish = fs::read_to_string(fixture(
        "update-myosu/current/fabro/prompts/bootstrap/poker/polish.md",
    ))
    .expect("original polish prompt exists");
    let preview_polish =
        fs::read_to_string(preview.join("malinka/prompts/bootstrap/poker/polish.md"))
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

    assert!(target.join("malinka/blueprints/myosu-update.yaml").exists());
    assert!(preview.join("malinka/programs/myosu-update.yaml").exists());
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
        .join("malinka/programs/myosu-miner-service-implementation.yaml")
        .exists());
    let workflow =
        fs::read_to_string(preview.join("malinka/workflows/implement/miner-service.fabro"))
            .expect("service workflow exists");
    assert!(workflow.contains("label=\"Health\""));
    assert!(workflow.contains("curl http://{ip}:{port}/health"));

    let review_prompt =
        fs::read_to_string(preview.join("malinka/prompts/implement/miner-service/review.md"))
            .expect("service review prompt exists");
    assert!(review_prompt.contains("First health gate"));
    assert!(review_prompt.contains("Health surfaces to preserve"));
    assert!(review_prompt.contains("Observability surfaces to preserve"));
    assert!(review_prompt.contains("Start: **Start slices 1 and 3 immediately**"));
    assert!(review_prompt.contains("Parallel: **Parallelize**: begin `myosu-miner` CLI skeleton"));
}
