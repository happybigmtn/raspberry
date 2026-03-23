use std::fs;
use std::path::{Path, PathBuf};

use fabro_synthesis::{
    import_existing_package, load_blueprint, reconcile_blueprint, render_blueprint, ImportRequest,
    ReconcileRequest, RenderRequest,
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

#[test]
fn render_blueprint_writes_expected_package() {
    let blueprint = load_blueprint(&fixture("craps/blueprint.yaml")).expect("load blueprint");
    let temp = tempfile::tempdir().expect("tempdir");

    let report = render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: temp.path(),
    })
    .expect("render blueprint");

    assert!(report
        .written_files
        .iter()
        .any(|path| path.ends_with("malinka/programs/craps.yaml")));

    let manifest = fs::read_to_string(temp.path().join("malinka/programs/craps.yaml"))
        .expect("manifest exists");
    assert!(manifest.contains("program: craps"));
    assert!(manifest.contains("run_config: ../run-configs/bootstrap/rules.toml"));

    let workflow = fs::read_to_string(temp.path().join("malinka/workflows/bootstrap/rules.fabro"))
        .expect("workflow exists");
    assert!(workflow.contains("digraph Rules"));
    assert!(workflow.contains("goal=\"Bootstrap the craps gameplay lane"));
}

#[test]
fn import_existing_package_reads_current_tree() {
    let temp = tempfile::tempdir().expect("tempdir");
    let fixture_root = temp.path().join("repo");
    copy_dir(&fixture("update-myosu/current"), &fixture_root).expect("copy current fixture");
    let blueprint = import_existing_package(ImportRequest {
        target_repo: &fixture_root,
        program: "myosu-update",
    })
    .expect("import existing package");

    assert_eq!(blueprint.program.id, "myosu-update");
    assert_eq!(
        blueprint.program.state_path,
        Some(PathBuf::from(".raspberry/myosu-update-state.json"))
    );
    assert_eq!(blueprint.units.len(), 1);
    assert_eq!(
        blueprint.units[0].output_root,
        PathBuf::from("outputs/games")
    );
    assert_eq!(blueprint.units[0].lanes.len(), 1);
    assert_eq!(blueprint.units[0].lanes[0].id, "poker");
    assert_eq!(blueprint.units[0].lanes[0].family, "bootstrap");
}

#[test]
fn reconcile_blueprint_reports_drift_and_writes_patch() {
    let temp = tempfile::tempdir().expect("tempdir");
    let current = temp.path().join("repo");
    copy_dir(&fixture("update-myosu/current"), &current).expect("copy current fixture");
    copy_dir(&fixture("update-myosu/doctrine"), &current.join("doctrine"))
        .expect("copy doctrine fixture");
    copy_dir(&fixture("update-myosu/evidence"), &current.join("evidence"))
        .expect("copy evidence fixture");
    copy_dir(
        &fixture("update-myosu/runtime"),
        &current.join(".raspberry"),
    )
    .expect("copy runtime fixture");
    copy_dir(&fixture("update-myosu/outputs"), &current.join("outputs"))
        .expect("copy outputs fixture");
    let blueprint =
        load_blueprint(&fixture("update-myosu/blueprint.yaml")).expect("load update blueprint");

    let report = reconcile_blueprint(ReconcileRequest {
        blueprint: &blueprint,
        current_repo: &current,
        output_repo: &current,
    })
    .expect("reconcile blueprint");

    assert!(report
        .findings
        .iter()
        .any(|finding| finding.contains("tutorial")));
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.contains("doctrine input found")));
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.contains("runtime state found")));
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.contains("supported by doctrine")));
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.contains("runtime state reports lane")));
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.contains("artifact missing")));
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.contains("appears ready for execution")));
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.contains("implementation follow-on is ready")));
    assert!(report
        .recommendations
        .iter()
        .any(|recommendation| recommendation
            .contains("execute the next ready bootstrap lane(s) first: games:tutorial")));
    assert!(report
        .recommendations
        .iter()
        .any(|recommendation| recommendation
            .contains("add implementation-family packages in this order: games:poker")));
    assert!(report.recommendations.iter().any(|recommendation| {
        recommendation.contains("add implementation program")
            && recommendation.contains("myosu-games-poker-implementation.yaml")
            && recommendation.contains("plus an implementation-family package for `games:poker`")
            && recommendation.contains("malinka/run-configs/implement/poker.toml")
    }));

    let manifest = fs::read_to_string(current.join("malinka/programs/myosu-update.yaml"))
        .expect("manifest exists");
    assert!(manifest.contains("id: tutorial"));
    assert!(manifest.contains("tutorial_reviewed"));

    let implementation_manifest =
        fs::read_to_string(current.join("malinka/programs/myosu-games-poker-implementation.yaml"))
            .expect("implementation manifest exists");
    assert!(implementation_manifest.contains("program: myosu-games-poker-implementation"));
    assert!(implementation_manifest.contains("run_config: ../run-configs/implement/poker.toml"));

    let implementation_workflow =
        fs::read_to_string(current.join("malinka/workflows/implement/poker.fabro"))
            .expect("implementation workflow exists");
    assert!(implementation_workflow.contains("digraph Poker"));
}

#[test]
fn reconcile_blueprint_emits_service_follow_on_with_health_gate() {
    let temp = tempfile::tempdir().expect("tempdir");
    let current = temp.path().join("repo");
    copy_dir(&fixture("service-follow-on/current"), &current).expect("copy current fixture");
    copy_dir(
        &fixture("service-follow-on/outputs"),
        &current.join("outputs"),
    )
    .expect("copy outputs fixture");
    let blueprint = load_blueprint(&fixture("service-follow-on/blueprint.yaml"))
        .expect("load service blueprint");

    let report = reconcile_blueprint(ReconcileRequest {
        blueprint: &blueprint,
        current_repo: &current,
        output_repo: &current,
    })
    .expect("reconcile blueprint");

    assert!(report
        .recommendations
        .iter()
        .any(|recommendation| recommendation.contains("myosu-miner-service-implementation.yaml")));

    if let Ok(implementation_workflow) =
        fs::read_to_string(current.join("malinka/workflows/implement/miner-service.fabro"))
    {
        assert!(implementation_workflow.contains("label=\"Health\""));
        assert!(implementation_workflow.contains("curl http://{ip}:{port}/health"));
    }
}

#[test]
fn reconcile_blueprint_does_not_clobber_files_when_reusing_same_repo() {
    let temp = tempfile::tempdir().expect("tempdir");
    let current = temp.path().join("repo");
    copy_dir(&fixture("update-myosu/current"), &current).expect("copy current fixture");

    let blueprint = import_existing_package(ImportRequest {
        target_repo: &current,
        program: "myosu-update",
    })
    .expect("import existing package");

    let run_config_path = current.join("malinka/run-configs/bootstrap/poker.toml");
    let workflow_path = current.join("malinka/workflows/bootstrap/poker.fabro");
    let workflow_before = fs::read_to_string(&workflow_path).expect("workflow exists");

    let report = reconcile_blueprint(ReconcileRequest {
        blueprint: &blueprint,
        current_repo: &current,
        output_repo: &current,
    })
    .expect("reconcile blueprint");

    assert!(report
        .findings
        .iter()
        .any(|finding| finding.contains("already matches blueprint structure")));
    let run_config_after = fs::read_to_string(&run_config_path).expect("run config after");
    let workflow_after = fs::read_to_string(&workflow_path).expect("workflow after");
    assert!(workflow_before.contains("digraph Poker"));
    assert!(workflow_after.contains("digraph Poker"));
    assert!(workflow_after.contains("test -f ./outputs/games/poker/spec.md"));
    assert!(run_config_after.contains("model = \"MiniMax-M2.7\""));
    assert!(run_config_after.contains("graph = \"../../workflows/bootstrap/poker.fabro\""));
}
