use std::fs;
use std::path::{Path, PathBuf};

use fabro_synthesis::{
    load_blueprint, render_blueprint, BlueprintLane, BlueprintUnit, RenderRequest,
    WorkflowTemplate, BOOTSTRAP_REQUIRED_ARTIFACTS,
};

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/bootstrap-verify")
        .join(path)
}

#[test]
fn bootstrap_required_artifacts_constant_has_five_entries() {
    assert_eq!(BOOTSTRAP_REQUIRED_ARTIFACTS.len(), 5);
    assert!(BOOTSTRAP_REQUIRED_ARTIFACTS.contains(&"implementation"));
    assert!(BOOTSTRAP_REQUIRED_ARTIFACTS.contains(&"verification"));
    assert!(BOOTSTRAP_REQUIRED_ARTIFACTS.contains(&"quality"));
    assert!(BOOTSTRAP_REQUIRED_ARTIFACTS.contains(&"promotion"));
    assert!(BOOTSTRAP_REQUIRED_ARTIFACTS.contains(&"integration"));
}

#[test]
fn blueprint_unit_bootstrap_required_artifacts_returns_five_ids() {
    let unit = BlueprintUnit {
        id: "test".to_string(),
        title: "Test".to_string(),
        output_root: PathBuf::from("outputs/test"),
        artifacts: Vec::new(),
        milestones: Vec::new(),
        lanes: Vec::new(),
    };
    let artifacts = unit.bootstrap_required_artifacts();
    assert_eq!(artifacts.len(), 5);
    assert!(artifacts.contains(&"implementation".to_string()));
    assert!(artifacts.contains(&"verification".to_string()));
    assert!(artifacts.contains(&"quality".to_string()));
    assert!(artifacts.contains(&"promotion".to_string()));
    assert!(artifacts.contains(&"integration".to_string()));
}

#[test]
fn blueprint_lane_is_bootstrap_returns_true_for_bootstrap_template() {
    let lane = BlueprintLane {
        id: "test".to_string(),
        kind: raspberry_supervisor::manifest::LaneKind::Platform,
        title: "Test".to_string(),
        family: "bootstrap".to_string(),
        workflow_family: None,
        slug: None,
        template: WorkflowTemplate::Bootstrap,
        goal: "Test goal".to_string(),
        managed_milestone: "implemented".to_string(),
        dependencies: Vec::new(),
        produces: Vec::new(),
        proof_profile: None,
        proof_state_path: None,
        program_manifest: None,
        service_state_path: None,
        orchestration_state_path: None,
        checks: Vec::new(),
        run_dir: None,
        prompt_context: None,
        verify_command: None,
        health_command: None,
    };
    assert!(lane.is_bootstrap());
}

#[test]
fn blueprint_lane_is_bootstrap_returns_true_for_service_bootstrap_template() {
    let lane = BlueprintLane {
        id: "test".to_string(),
        kind: raspberry_supervisor::manifest::LaneKind::Service,
        title: "Test".to_string(),
        family: "bootstrap".to_string(),
        workflow_family: None,
        slug: None,
        template: WorkflowTemplate::ServiceBootstrap,
        goal: "Test goal".to_string(),
        managed_milestone: "implemented".to_string(),
        dependencies: Vec::new(),
        produces: Vec::new(),
        proof_profile: None,
        proof_state_path: None,
        program_manifest: None,
        service_state_path: None,
        orchestration_state_path: None,
        checks: Vec::new(),
        run_dir: None,
        prompt_context: None,
        verify_command: None,
        health_command: None,
    };
    assert!(lane.is_bootstrap());
}

#[test]
fn blueprint_lane_is_bootstrap_returns_false_for_implementation_template() {
    let lane = BlueprintLane {
        id: "test".to_string(),
        kind: raspberry_supervisor::manifest::LaneKind::Platform,
        title: "Test".to_string(),
        family: "implementation".to_string(),
        workflow_family: None,
        slug: None,
        template: WorkflowTemplate::Implementation,
        goal: "Test goal".to_string(),
        managed_milestone: "merge_ready".to_string(),
        dependencies: Vec::new(),
        produces: Vec::new(),
        proof_profile: None,
        proof_state_path: None,
        program_manifest: None,
        service_state_path: None,
        orchestration_state_path: None,
        checks: Vec::new(),
        run_dir: None,
        prompt_context: None,
        verify_command: None,
        health_command: None,
    };
    assert!(!lane.is_bootstrap());
}

#[test]
fn render_blueprint_writes_five_durable_artifact_files() {
    let blueprint = load_blueprint(&fixture("blueprint.yaml")).expect("load blueprint");
    let temp = tempfile::tempdir().expect("tempdir");

    let report = render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: temp.path(),
    })
    .expect("render blueprint");

    // Verify written files include the manifest
    assert!(report
        .written_files
        .iter()
        .any(|path| path.ends_with("malinka/programs/bootstrap-verify-test.yaml")));

    // Verify the five durable artifact files exist
    let artifact_dir = temp.path().join("outputs/demo");
    for artifact_id in BOOTSTRAP_REQUIRED_ARTIFACTS {
        let artifact_path = artifact_dir.join(format!("{artifact_id}.md"));
        assert!(
            artifact_path.exists(),
            "expected {} to exist",
            artifact_path.display()
        );
        let content = fs::read_to_string(&artifact_path).expect("read artifact");
        let artifact_name = (*artifact_id).replace('_', " ");
        assert!(
            content.contains(&artifact_name),
            "artifact {} should contain its name",
            artifact_id
        );
        assert!(
            content.contains("Placeholder"),
            "artifact {} should be a placeholder",
            artifact_id
        );
    }
}

/// Full lifecycle bootstrap verification test covering all acceptance criteria:
/// 1. Render output contains five `.md` files
/// 2. Render output contains run-config
/// 3. Render output contains Graphviz workflow
/// 4. Render output contains program manifest
/// 5. BlueprintLane::is_bootstrap() returns true for bootstrap lanes
/// 6. BlueprintUnit::bootstrap_required_artifacts() returns the five artifact IDs
#[test]
fn bootstrap_verify() {
    // Test helpers
    assert_eq!(BOOTSTRAP_REQUIRED_ARTIFACTS.len(), 5);
    let unit = BlueprintUnit {
        id: "test".to_string(),
        title: "Test".to_string(),
        output_root: PathBuf::from("outputs/test"),
        artifacts: Vec::new(),
        milestones: Vec::new(),
        lanes: Vec::new(),
    };
    assert_eq!(unit.bootstrap_required_artifacts().len(), 5);

    let bootstrap_lane = BlueprintLane {
        id: "test".to_string(),
        kind: raspberry_supervisor::manifest::LaneKind::Platform,
        title: "Test".to_string(),
        family: "bootstrap".to_string(),
        workflow_family: None,
        slug: None,
        template: WorkflowTemplate::Bootstrap,
        goal: "Test goal".to_string(),
        managed_milestone: "implemented".to_string(),
        dependencies: Vec::new(),
        produces: Vec::new(),
        proof_profile: None,
        proof_state_path: None,
        program_manifest: None,
        service_state_path: None,
        orchestration_state_path: None,
        checks: Vec::new(),
        run_dir: None,
        prompt_context: None,
        verify_command: None,
        health_command: None,
    };
    assert!(bootstrap_lane.is_bootstrap());

    // Test rendering with blueprint fixture
    let blueprint = load_blueprint(&fixture("blueprint.yaml")).expect("load blueprint");
    let temp = tempfile::tempdir().expect("tempdir");

    let report = render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: temp.path(),
    })
    .expect("render blueprint");

    // Acceptance criterion 3: Render output contains run-config
    let run_config_path = temp.path().join("malinka/run-configs/bootstrap/bootstrap.toml");
    assert!(run_config_path.exists(), "run config should exist");
    let run_config_content = fs::read_to_string(&run_config_path).expect("read run config");
    assert!(run_config_content.contains("graph = "));
    assert!(run_config_content.contains("goal = "));

    // Acceptance criterion 4: Render output contains Graphviz workflow
    let workflow_path = temp.path().join("malinka/workflows/bootstrap/bootstrap.fabro");
    assert!(workflow_path.exists(), "workflow should exist");
    let workflow_content = fs::read_to_string(&workflow_path).expect("read workflow");
    assert!(workflow_content.contains("digraph"));

    // Acceptance criterion 5: Render output contains program manifest
    assert!(report
        .written_files
        .iter()
        .any(|path| path.ends_with("malinka/programs/bootstrap-verify-test.yaml")));
    let manifest_path = temp.path().join("malinka/programs/bootstrap-verify-test.yaml");
    let manifest_content = fs::read_to_string(&manifest_path).expect("read manifest");
    assert!(manifest_content.contains("program: bootstrap-verify-test"));
    assert!(manifest_content.contains("units:"));
    assert!(manifest_content.contains("lanes:"));

    // Acceptance criteria 1 & 2: Render output contains five `.md` files
    let artifact_dir = temp.path().join("outputs/demo");
    for artifact_id in BOOTSTRAP_REQUIRED_ARTIFACTS {
        let artifact_path = artifact_dir.join(format!("{artifact_id}.md"));
        assert!(
            artifact_path.exists(),
            "expected {} to exist",
            artifact_path.display()
        );
    }
}

#[test]
fn render_output_contains_run_config() {
    let blueprint = load_blueprint(&fixture("blueprint.yaml")).expect("load blueprint");
    let temp = tempfile::tempdir().expect("tempdir");

    render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: temp.path(),
    })
    .expect("render blueprint");

    let run_config_path = temp.path().join("malinka/run-configs/bootstrap/bootstrap.toml");
    assert!(run_config_path.exists(), "run config should exist");
    let content = fs::read_to_string(&run_config_path).expect("read run config");
    assert!(content.contains("graph = "));
    assert!(content.contains("goal = "));
}

#[test]
fn render_output_contains_graphviz_workflow() {
    let blueprint = load_blueprint(&fixture("blueprint.yaml")).expect("load blueprint");
    let temp = tempfile::tempdir().expect("tempdir");

    render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: temp.path(),
    })
    .expect("render blueprint");

    let workflow_path = temp.path().join("malinka/workflows/bootstrap/bootstrap.fabro");
    assert!(workflow_path.exists(), "workflow should exist");
    let content = fs::read_to_string(&workflow_path).expect("read workflow");
    assert!(content.contains("digraph"));
    assert!(content.contains("Bootstrap the demo feature from scratch"));
}

#[test]
fn render_output_contains_program_manifest() {
    let blueprint = load_blueprint(&fixture("blueprint.yaml")).expect("load blueprint");
    let temp = tempfile::tempdir().expect("tempdir");

    render_blueprint(RenderRequest {
        blueprint: &blueprint,
        target_repo: temp.path(),
    })
    .expect("render blueprint");

    let manifest_path = temp.path().join("malinka/programs/bootstrap-verify-test.yaml");
    assert!(manifest_path.exists(), "program manifest should exist");
    let content = fs::read_to_string(&manifest_path).expect("read manifest");
    assert!(content.contains("program: bootstrap-verify-test"));
    assert!(content.contains("units:"));
    assert!(content.contains("lanes:"));
}

#[test]
fn render_lane_writes_artifact_files_for_implementation_lane() {
    let unit = BlueprintUnit {
        id: "test-impl".to_string(),
        title: "Test Implementation".to_string(),
        output_root: PathBuf::from("outputs/test-impl"),
        artifacts: vec![
            fabro_synthesis::BlueprintArtifact {
                id: "implementation".to_string(),
                path: PathBuf::from("implementation.md"),
            },
            fabro_synthesis::BlueprintArtifact {
                id: "verification".to_string(),
                path: PathBuf::from("verification.md"),
            },
            fabro_synthesis::BlueprintArtifact {
                id: "quality".to_string(),
                path: PathBuf::from("quality.md"),
            },
            fabro_synthesis::BlueprintArtifact {
                id: "promotion".to_string(),
                path: PathBuf::from("promotion.md"),
            },
            fabro_synthesis::BlueprintArtifact {
                id: "integration".to_string(),
                path: PathBuf::from("integration.md"),
            },
        ],
        milestones: vec![
            raspberry_supervisor::manifest::MilestoneManifest {
                id: "implemented".to_string(),
                requires: vec!["implementation".to_string()],
            },
            raspberry_supervisor::manifest::MilestoneManifest {
                id: "verified".to_string(),
                requires: vec![
                    "implementation".to_string(),
                    "verification".to_string(),
                    "quality".to_string(),
                ],
            },
            raspberry_supervisor::manifest::MilestoneManifest {
                id: "merge_ready".to_string(),
                requires: vec![
                    "implementation".to_string(),
                    "verification".to_string(),
                    "quality".to_string(),
                    "promotion".to_string(),
                ],
            },
        ],
        lanes: vec![BlueprintLane {
            id: "test-impl".to_string(),
            kind: raspberry_supervisor::manifest::LaneKind::Platform,
            title: "Test Implementation Lane".to_string(),
            family: "implementation".to_string(),
            workflow_family: Some("implementation".to_string()),
            slug: Some("test-impl".to_string()),
            template: WorkflowTemplate::Implementation,
            goal: "Implement the test feature.".to_string(),
            managed_milestone: "merge_ready".to_string(),
            dependencies: Vec::new(),
            produces: vec![
                "implementation".to_string(),
                "verification".to_string(),
                "quality".to_string(),
                "promotion".to_string(),
                "integration".to_string(),
            ],
            proof_profile: None,
            proof_state_path: None,
            program_manifest: None,
            service_state_path: None,
            orchestration_state_path: None,
            checks: Vec::new(),
            run_dir: None,
            prompt_context: None,
            verify_command: Some("cargo check".to_string()),
            health_command: None,
        }],
    };

    let blueprint = fabro_synthesis::ProgramBlueprint {
        version: 1,
        program: fabro_synthesis::BlueprintProgram {
            id: "test-impl-program".to_string(),
            max_parallel: 1,
            state_path: None,
            run_dir: None,
        },
        inputs: fabro_synthesis::BlueprintInputs::default(),
        package: fabro_synthesis::BlueprintPackage::default(),
        units: vec![unit],
        protocols: vec![],
    };

    let temp = tempfile::tempdir().expect("tempdir");

    let _report = render_blueprint(fabro_synthesis::RenderRequest {
        blueprint: &blueprint,
        target_repo: temp.path(),
    })
    .expect("render blueprint");

    // Verify the five durable artifact files exist for implementation lane too
    let artifact_dir = temp.path().join("outputs/test-impl");
    for artifact_id in BOOTSTRAP_REQUIRED_ARTIFACTS {
        let artifact_path = artifact_dir.join(format!("{artifact_id}.md"));
        assert!(
            artifact_path.exists(),
            "implementation lane should write {}",
            artifact_path.display()
        );
    }
}
