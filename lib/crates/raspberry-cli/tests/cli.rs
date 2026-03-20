use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn raspberry() -> Command {
    Command::cargo_bin("raspberry").expect("raspberry binary should build")
}

fn fixture_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures/raspberry-supervisor/program.yaml")
}

fn myosu_fixture_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml")
}

#[test]
fn plan_shows_ready_and_blocked_lanes() {
    raspberry()
        .args([
            "plan",
            "--manifest",
            fixture_manifest().to_str().expect("utf-8 fixture path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Max parallel: 2"))
        .stdout(predicate::str::contains("Ready:"))
        .stdout(predicate::str::contains("runtime:page"))
        .stdout(predicate::str::contains("runtime:proof"))
        .stdout(predicate::str::contains("Blocked:"))
        .stdout(predicate::str::contains("consensus:page"));
}

#[test]
fn status_shows_running_and_failed_lanes() {
    raspberry()
        .args([
            "status",
            "--manifest",
            fixture_manifest().to_str().expect("utf-8 fixture path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Counts:"))
        .stdout(predicate::str::contains("p2p:chapter [running|artifact]"))
        .stdout(predicate::str::contains(
            "consensus:chapter [failed|artifact]",
        ))
        .stdout(predicate::str::contains("stage=Review"))
        .stdout(predicate::str::contains("last_completed_stage=Draft"))
        .stdout(predicate::str::contains(
            "usage: gpt-5.4: 1200 in / 800 out",
        ))
        .stdout(predicate::str::contains("files_written: draft.md"));
}

#[test]
fn watch_single_iteration_renders_status() {
    raspberry()
        .args([
            "watch",
            "--manifest",
            fixture_manifest().to_str().expect("utf-8 fixture path"),
            "--iterations",
            "1",
            "--interval-ms",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Iteration 1:"))
        .stdout(predicate::str::contains(
            "runtime:chapter [complete|artifact]",
        ));
}

#[test]
fn help_shows_tui_subcommand() {
    raspberry()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("tui"));
}

#[test]
fn plan_supports_myouso_shaped_manifest() {
    raspberry()
        .args([
            "plan",
            "--manifest",
            myosu_fixture_manifest()
                .to_str()
                .expect("utf-8 myosu fixture path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Program: myosu-bootstrap"))
        .stdout(predicate::str::contains("validator:oracle [service]"))
        .stdout(predicate::str::contains("operations:scorecard"))
        .stdout(predicate::str::contains("launch:devnet"))
        .stdout(predicate::str::contains("play:tui"));
}

#[test]
fn status_supports_myouso_shaped_manifest() {
    raspberry()
        .args([
            "status",
            "--manifest",
            myosu_fixture_manifest()
                .to_str()
                .expect("utf-8 myosu fixture path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Program: myosu-bootstrap"))
        .stdout(predicate::str::contains("miner:service [running|service]"))
        .stdout(predicate::str::contains("operational=healthy"))
        .stdout(predicate::str::contains(
            "running_checks_passing: miner_http_ok, training_active",
        ))
        .stdout(predicate::str::contains("last_completed_stage=Spec"))
        .stdout(predicate::str::contains(
            "validator:oracle [complete|service]",
        ))
        .stdout(predicate::str::contains("proof_profile=validator_tests"))
        .stdout(predicate::str::contains("preconditions=met"))
        .stdout(predicate::str::contains(
            "ready_checks_passing: chain_ready",
        ))
        .stdout(predicate::str::contains(
            "operations:scorecard [blocked|orchestration]",
        ))
        .stdout(predicate::str::contains(
            "ready_checks_failing: validator_proof_passed",
        ))
        .stdout(predicate::str::contains("proof_state=failed"))
        .stdout(predicate::str::contains(
            "launch:devnet [blocked|orchestration]",
        ))
        .stdout(predicate::str::contains("orchestration=waiting"))
        .stdout(predicate::str::contains("play:tui [failed|interface]"))
        .stdout(predicate::str::contains(
            "error: terminal snapshot mismatch",
        ));
}

#[test]
fn execute_updates_program_state_using_fake_fabro() {
    let fixture_root = fixture_manifest()
        .parent()
        .expect("fixture manifest parent")
        .to_path_buf();
    let temp = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture_root, temp.path()).expect("copy fixture tree");

    let fake_fabro = temp.path().join("fake-fabro.sh");
    fs::write(
        &fake_fabro,
        concat!(
            "#!/usr/bin/env bash\n",
            "set -euo pipefail\n",
            "if [ \"$1\" != \"--no-upgrade-check\" ]; then exit 4; fi\n",
            "if [ \"$2\" != \"run\" ]; then exit 5; fi\n",
            "if [ \"$3\" != \"--detach\" ]; then exit 6; fi\n",
            "RUN_ID=\"01KM244VBG7TF9FB8D53BFTHX7\"\n",
            "RUN_DIR=\"$HOME/.fabro/runs/20260319-$RUN_ID\"\n",
            "mkdir -p \"$RUN_DIR\"\n",
            "cat > \"$RUN_DIR/manifest.json\" <<'EOF'\n",
            "{\"run_id\":\"01KM244VBG7TF9FB8D53BFTHX7\",\"workflow_name\":\"demo\",\"goal\":\"do work\",\"start_time\":\"2026-03-19T00:00:00Z\",\"node_count\":2,\"edge_count\":1,\"labels\":{}}\n",
            "EOF\n",
            "cat > \"$RUN_DIR/status.json\" <<'EOF'\n",
            "{\"status\":\"running\",\"updated_at\":\"2026-03-19T00:00:01Z\"}\n",
            "EOF\n",
            "cat > \"$RUN_DIR/state.json\" <<'EOF'\n",
            "{\"run_id\":\"01KM244VBG7TF9FB8D53BFTHX7\",\"updated_at\":\"2026-03-19T00:00:01Z\",\"status\":\"running\",\"current_stage_label\":\"Review\"}\n",
            "EOF\n",
            "cat > \"$RUN_DIR/progress.jsonl\" <<'EOF'\n",
            "{\"ts\":\"2026-03-19T00:00:01Z\",\"run_id\":\"01KM244VBG7TF9FB8D53BFTHX7\",\"event\":\"StageStarted\",\"node_label\":\"Review\",\"event_seq\":1}\n",
            "EOF\n",
            "printf '%s\\n' \"$RUN_ID\"\n",
        ),
    )
    .expect("write fake fabro");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_fabro).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_fabro, perms).expect("chmod");
    }

    let manifest = temp.path().join("program.yaml");
    raspberry()
        .args([
            "execute",
            "--manifest",
            manifest.to_str().expect("utf-8 manifest path"),
            "--fabro-bin",
            fake_fabro.to_str().expect("utf-8 fake fabro path"),
            "--lane",
            "runtime:page",
            "--lane",
            "runtime:proof",
            "--max-parallel",
            "2",
        ])
        .env("HOME", temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Dispatch parallelism: 2"))
        .stdout(predicate::str::contains("runtime:page [submitted]"))
        .stdout(predicate::str::contains("runtime:proof [submitted]"))
        .stdout(predicate::str::contains(
            "run_id=01KM244VBG7TF9FB8D53BFTHX7",
        ));

    let state_path = temp.path().join(".raspberry/program-state.json");
    let state = fs::read_to_string(&state_path).expect("state file should exist");
    assert!(state.contains("\"runtime:page\""));
    assert!(state.contains("\"runtime:proof\""));
    assert!(state.contains("\"status\": \"running\""));
    assert!(state.contains("\"current_fabro_run_id\": \"01KM244VBG7TF9FB8D53BFTHX7\""));

    raspberry()
        .args([
            "status",
            "--manifest",
            manifest.to_str().expect("utf-8 manifest path"),
        ])
        .env("HOME", temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("runtime:page [running|artifact]"))
        .stdout(predicate::str::contains(
            "fabro_run_id=01KM244VBG7TF9FB8D53BFTHX7",
        ))
        .stdout(predicate::str::contains("stage=Review"));
}

#[test]
fn execute_allows_explicit_rerun_of_failed_lane() {
    let fixture_root = fixture_manifest()
        .parent()
        .expect("fixture manifest parent")
        .to_path_buf();
    let temp = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture_root, temp.path()).expect("copy fixture tree");

    let fake_fabro = temp.path().join("fake-fabro-rerun-failed.sh");
    fs::write(
        &fake_fabro,
        concat!(
            "#!/usr/bin/env bash\n",
            "set -euo pipefail\n",
            "if [ \"$1\" != \"--no-upgrade-check\" ]; then exit 4; fi\n",
            "if [ \"$2\" != \"run\" ]; then exit 5; fi\n",
            "if [ \"$3\" != \"--detach\" ]; then exit 6; fi\n",
            "printf '%s\\n' '01KM244VBG7TF9FB8D53BFTHX7'\n",
        ),
    )
    .expect("write fake fabro");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_fabro).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_fabro, perms).expect("chmod");
    }

    let manifest = temp.path().join("program.yaml");
    raspberry()
        .args([
            "execute",
            "--manifest",
            manifest.to_str().expect("utf-8 manifest path"),
            "--fabro-bin",
            fake_fabro.to_str().expect("utf-8 fake fabro path"),
            "--lane",
            "consensus:chapter",
        ])
        .env("HOME", temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("consensus:chapter [submitted]"))
        .stdout(predicate::str::contains(
            "run_id=01KM244VBG7TF9FB8D53BFTHX7",
        ));
}

#[test]
fn execute_allows_explicit_rerun_of_complete_lane() {
    let fixture_root = fixture_manifest()
        .parent()
        .expect("fixture manifest parent")
        .to_path_buf();
    let temp = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture_root, temp.path()).expect("copy fixture tree");

    let fake_fabro = temp.path().join("fake-fabro-rerun-complete.sh");
    fs::write(
        &fake_fabro,
        concat!(
            "#!/usr/bin/env bash\n",
            "set -euo pipefail\n",
            "if [ \"$1\" != \"--no-upgrade-check\" ]; then exit 4; fi\n",
            "if [ \"$2\" != \"run\" ]; then exit 5; fi\n",
            "if [ \"$3\" != \"--detach\" ]; then exit 6; fi\n",
            "printf '%s\\n' '01KM244VBG7TF9FB8D53BFTHX8'\n",
        ),
    )
    .expect("write fake fabro");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_fabro).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_fabro, perms).expect("chmod");
    }

    let manifest = temp.path().join("program.yaml");
    raspberry()
        .args([
            "execute",
            "--manifest",
            manifest.to_str().expect("utf-8 manifest path"),
            "--fabro-bin",
            fake_fabro.to_str().expect("utf-8 fake fabro path"),
            "--lane",
            "runtime:chapter",
        ])
        .env("HOME", temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("runtime:chapter [submitted]"))
        .stdout(predicate::str::contains(
            "run_id=01KM244VBG7TF9FB8D53BFTHX8",
        ));
}

#[test]
fn execute_sets_dedicated_autodev_cargo_target_dir_for_fabro() {
    let fixture_root = fixture_manifest()
        .parent()
        .expect("fixture manifest parent")
        .to_path_buf();
    let temp = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture_root, temp.path()).expect("copy fixture tree");

    let fake_fabro = temp.path().join("fake-fabro-cargo-target.sh");
    fs::write(
        &fake_fabro,
        concat!(
            "#!/usr/bin/env bash\n",
            "set -euo pipefail\n",
            "printf '%s\\n' \"${CARGO_TARGET_DIR:-unset}\" > \"$HOME/cargo-target-dir.log\"\n",
            "if [ \"$1\" != \"--no-upgrade-check\" ]; then exit 4; fi\n",
            "if [ \"$2\" != \"run\" ]; then exit 5; fi\n",
            "if [ \"$3\" != \"--detach\" ]; then exit 6; fi\n",
            "printf '%s\\n' '01KM244VBG7TF9FB8D53BFTHX7'\n",
        ),
    )
    .expect("write fake fabro");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_fabro).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_fabro, perms).expect("chmod");
    }

    let manifest = temp.path().join("program.yaml");
    raspberry()
        .args([
            "execute",
            "--manifest",
            manifest.to_str().expect("utf-8 manifest path"),
            "--fabro-bin",
            fake_fabro.to_str().expect("utf-8 fake fabro path"),
            "--lane",
            "runtime:page",
        ])
        .env("HOME", temp.path())
        .assert()
        .success();

    let logged =
        fs::read_to_string(temp.path().join("cargo-target-dir.log")).expect("cargo target log");
    assert_eq!(
        logged.trim(),
        temp.path()
            .join(".raspberry/cargo-target")
            .display()
            .to_string()
    );
}

#[test]
fn autodev_runs_synth_and_dispatch_cycle() {
    let fixture_root = fixture_manifest()
        .parent()
        .expect("fixture manifest parent")
        .to_path_buf();
    let temp = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture_root, temp.path()).expect("copy fixture tree");

    let fake_fabro = temp.path().join("fake-fabro-autodev.sh");
    fs::write(
        &fake_fabro,
        concat!(
            "#!/usr/bin/env bash\n",
            "set -euo pipefail\n",
            "LOG=\"$HOME/autodev-fabro.log\"\n",
            "printf '%s\\n' \"$*\" >> \"$LOG\"\n",
            "if [ \"$1\" = \"--no-upgrade-check\" ]; then shift; fi\n",
            "case \"$1\" in\n",
            "  synth)\n",
            "    case \"$2\" in\n",
            "      import)\n",
            "        OUTPUT=\"\"\n",
            "        PROGRAM=\"\"\n",
            "        while [ $# -gt 0 ]; do\n",
            "          case \"$1\" in\n",
            "            --output) OUTPUT=\"$2\"; shift 2 ;;\n",
            "            --program) PROGRAM=\"$2\"; shift 2 ;;\n",
            "            *) shift ;;\n",
            "          esac\n",
            "        done\n",
            "        cat > \"$OUTPUT\" <<EOF\n",
            "version: 1\n",
            "program:\n",
            "  id: ${PROGRAM}\n",
            "  max_parallel: 2\n",
            "inputs:\n",
            "  doctrine_files: []\n",
            "  evidence_paths: []\n",
            "package:\n",
            "  fabro_root: fabro\n",
            "units: []\n",
            "EOF\n",
            "        printf 'Program: %s\\nMode: import\\nBlueprint: %s\\n' \"$PROGRAM\" \"$OUTPUT\"\n",
            "        ;;\n",
            "      evolve)\n",
            "        TARGET=\"\"\n",
            "        while [ $# -gt 0 ]; do\n",
            "          case \"$1\" in\n",
            "            --target-repo) TARGET=\"$2\"; shift 2 ;;\n",
            "            --preview-root) shift 2 ;;\n",
            "            *) shift ;;\n",
            "          esac\n",
            "        done\n",
            "        touch \"$TARGET/.autodev-evolved\"\n",
            "        printf 'Mode: evolve\\n'\n",
            "        ;;\n",
            "      *) exit 11 ;;\n",
            "    esac\n",
            "    ;;\n",
            "  run)\n",
            "    if [ \"$2\" != \"--detach\" ]; then exit 12; fi\n",
            "    RUN_ID=\"01KM244VBG7TF9FB8D53BFTHX7\"\n",
            "    RUN_DIR=\"$HOME/.fabro/runs/20260319-$RUN_ID\"\n",
            "    mkdir -p \"$RUN_DIR\"\n",
            "    cat > \"$RUN_DIR/manifest.json\" <<'EOF'\n",
            "{\"run_id\":\"01KM244VBG7TF9FB8D53BFTHX7\",\"workflow_name\":\"demo\",\"goal\":\"do work\",\"start_time\":\"2026-03-19T00:00:00Z\",\"node_count\":2,\"edge_count\":1,\"labels\":{}}\n",
            "EOF\n",
            "    cat > \"$RUN_DIR/status.json\" <<'EOF'\n",
            "{\"status\":\"running\",\"updated_at\":\"2026-03-19T00:00:01Z\"}\n",
            "EOF\n",
            "    cat > \"$RUN_DIR/state.json\" <<'EOF'\n",
            "{\"run_id\":\"01KM244VBG7TF9FB8D53BFTHX7\",\"updated_at\":\"2026-03-19T00:00:01Z\",\"status\":\"running\",\"current_stage_label\":\"Review\"}\n",
            "EOF\n",
            "    cat > \"$RUN_DIR/progress.jsonl\" <<'EOF'\n",
            "{\"ts\":\"2026-03-19T00:00:01Z\",\"run_id\":\"01KM244VBG7TF9FB8D53BFTHX7\",\"event\":\"StageStarted\",\"node_label\":\"Review\",\"event_seq\":1}\n",
            "EOF\n",
            "    printf '%s\\n' \"$RUN_ID\"\n",
            "    ;;\n",
            "  *) exit 13 ;;\n",
            "esac\n",
        ),
    )
    .expect("write fake fabro");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_fabro).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_fabro, perms).expect("chmod");
    }

    let manifest = temp.path().join("program.yaml");
    raspberry()
        .args([
            "autodev",
            "--manifest",
            manifest.to_str().expect("utf-8 manifest path"),
            "--fabro-bin",
            fake_fabro.to_str().expect("utf-8 fake fabro path"),
            "--max-cycles",
            "1",
            "--poll-interval-ms",
            "1",
            "--evolve-every-seconds",
            "0",
        ])
        .env("HOME", temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Program: raspberry-demo"))
        .stdout(predicate::str::contains("Cycle 1:"))
        .stdout(predicate::str::contains("evolve: skipped"))
        .stdout(predicate::str::contains(
            "ready: runtime:page, runtime:proof",
        ))
        .stdout(predicate::str::contains("dispatched: runtime:page"))
        .stdout(predicate::str::contains("dispatched: runtime:proof"))
        .stdout(predicate::str::contains("Stop reason: cycle_limit"));

    let log = fs::read_to_string(temp.path().join("autodev-fabro.log")).expect("fabro log");
    assert!(!log.contains("synth import"));
    assert!(!log.contains("synth evolve"));
    let run_count = log
        .lines()
        .filter(|line| line.contains("run --detach"))
        .count();
    assert_eq!(run_count, 2, "expected two dispatched runs");
    assert!(!temp.path().join(".autodev-evolved").exists());
}

#[test]
fn autodev_evolves_when_program_is_locally_settled() {
    let fixture_root = fixture_manifest()
        .parent()
        .expect("fixture manifest parent")
        .to_path_buf();
    let temp = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture_root, temp.path()).expect("copy fixture tree");

    let fake_fabro = temp.path().join("fake-fabro-autodev-settled.sh");
    fs::write(
        &fake_fabro,
        concat!(
            "#!/usr/bin/env bash\n",
            "set -euo pipefail\n",
            "LOG=\"$HOME/autodev-settled.log\"\n",
            "printf '%s\\n' \"$*\" >> \"$LOG\"\n",
            "if [ \"$1\" = \"--no-upgrade-check\" ]; then shift; fi\n",
            "case \"$1\" in\n",
            "  synth)\n",
            "    case \"$2\" in\n",
            "      import)\n",
            "        OUTPUT=\"\"\n",
            "        PROGRAM=\"\"\n",
            "        while [ $# -gt 0 ]; do\n",
            "          case \"$1\" in\n",
            "            --output) OUTPUT=\"$2\"; shift 2 ;;\n",
            "            --program) PROGRAM=\"$2\"; shift 2 ;;\n",
            "            *) shift ;;\n",
            "          esac\n",
            "        done\n",
            "        cat > \"$OUTPUT\" <<EOF\n",
            "version: 1\n",
            "program:\n",
            "  id: ${PROGRAM}\n",
            "  max_parallel: 1\n",
            "inputs:\n",
            "  doctrine_files: []\n",
            "  evidence_paths: []\n",
            "package:\n",
            "  fabro_root: fabro\n",
            "units: []\n",
            "EOF\n",
            "        ;;\n",
            "      evolve)\n",
            "        TARGET=\"\"\n",
            "        while [ $# -gt 0 ]; do\n",
            "          case \"$1\" in\n",
            "            --target-repo) TARGET=\"$2\"; shift 2 ;;\n",
            "            --preview-root) shift 2 ;;\n",
            "            *) shift ;;\n",
            "          esac\n",
            "        done\n",
            "        touch \"$TARGET/.autodev-evolved\"\n",
            "        ;;\n",
            "      *) exit 21 ;;\n",
            "    esac\n",
            "    ;;\n",
            "  run)\n",
            "    exit 22\n",
            "    ;;\n",
            "  *) exit 23 ;;\n",
            "esac\n",
        ),
    )
    .expect("write fake fabro");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_fabro).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_fabro, perms).expect("chmod");
    }

    let manifest = temp.path().join("complete-program.yaml");
    raspberry()
        .args([
            "autodev",
            "--manifest",
            manifest.to_str().expect("utf-8 manifest path"),
            "--fabro-bin",
            fake_fabro.to_str().expect("utf-8 fake fabro path"),
            "--max-cycles",
            "1",
            "--poll-interval-ms",
            "1",
            "--evolve-every-seconds",
            "0",
        ])
        .env("HOME", temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("evolve: applied"))
        .stdout(predicate::str::contains("ready: none"));

    let log = fs::read_to_string(temp.path().join("autodev-settled.log"))
        .expect("fabro log should exist");
    assert!(log.contains("synth import"));
    assert!(log.contains("synth evolve"));
    assert!(temp.path().join(".autodev-evolved").exists());
}

#[test]
fn autodev_respects_parallel_slots_when_dispatching_ready_lanes() {
    let fixture_root = fixture_manifest()
        .parent()
        .expect("fixture manifest parent")
        .to_path_buf();
    let temp = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture_root, temp.path()).expect("copy fixture tree");

    let fake_fabro = temp.path().join("fake-fabro-autodev-slots.sh");
    fs::write(
        &fake_fabro,
        concat!(
            "#!/usr/bin/env bash\n",
            "set -euo pipefail\n",
            "LOG=\"$HOME/autodev-slots.log\"\n",
            "printf '%s\\n' \"$*\" >> \"$LOG\"\n",
            "if [ \"$1\" = \"--no-upgrade-check\" ]; then shift; fi\n",
            "case \"$1\" in\n",
            "  synth)\n",
            "    case \"$2\" in\n",
            "      import)\n",
            "        OUTPUT=\"\"\n",
            "        PROGRAM=\"\"\n",
            "        while [ $# -gt 0 ]; do\n",
            "          case \"$1\" in\n",
            "            --output) OUTPUT=\"$2\"; shift 2 ;;\n",
            "            --program) PROGRAM=\"$2\"; shift 2 ;;\n",
            "            *) shift ;;\n",
            "          esac\n",
            "        done\n",
            "        cat > \"$OUTPUT\" <<EOF\n",
            "version: 1\n",
            "program:\n",
            "  id: ${PROGRAM}\n",
            "  max_parallel: 2\n",
            "inputs:\n",
            "  doctrine_files: []\n",
            "  evidence_paths: []\n",
            "package:\n",
            "  fabro_root: fabro\n",
            "units: []\n",
            "EOF\n",
            "        ;;\n",
            "      evolve)\n",
            "        exit 0\n",
            "        ;;\n",
            "      *) exit 11 ;;\n",
            "    esac\n",
            "    ;;\n",
            "  run)\n",
            "    if [ \"$2\" != \"--detach\" ]; then exit 12; fi\n",
            "    printf '%s\\n' '01KM244VBG7TF9FB8D53BFTHX7'\n",
            "    ;;\n",
            "  *) exit 13 ;;\n",
            "esac\n",
        ),
    )
    .expect("write fake fabro");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_fabro).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_fabro, perms).expect("chmod");
    }

    let manifest = temp.path().join("program.yaml");
    raspberry()
        .args([
            "autodev",
            "--manifest",
            manifest.to_str().expect("utf-8 manifest path"),
            "--fabro-bin",
            fake_fabro.to_str().expect("utf-8 fake fabro path"),
            "--max-cycles",
            "1",
            "--max-parallel",
            "2",
            "--poll-interval-ms",
            "1",
            "--evolve-every-seconds",
            "0",
        ])
        .env("HOME", temp.path())
        .assert()
        .success();

    let log = fs::read_to_string(temp.path().join("autodev-slots.log")).expect("fabro log");
    let run_count = log
        .lines()
        .filter(|line| line.contains("run --detach"))
        .count();
    assert_eq!(run_count, 2, "expected both ready lanes to dispatch");
}

#[test]
fn execute_can_tick_a_child_program_lane() {
    let fixture_root = fixture_manifest()
        .parent()
        .expect("fixture manifest parent")
        .to_path_buf();
    let temp = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture_root, temp.path()).expect("copy fixture tree");

    let fake_fabro = temp.path().join("fake-fabro-child-program.sh");
    fs::write(
        &fake_fabro,
        concat!(
            "#!/usr/bin/env bash\n",
            "set -euo pipefail\n",
            "if [ \"$1\" = \"--no-upgrade-check\" ]; then shift; fi\n",
            "case \"$1\" in\n",
            "  synth)\n",
            "    case \"$2\" in\n",
            "      import)\n",
            "        OUTPUT=\"\"\n",
            "        PROGRAM=\"\"\n",
            "        while [ $# -gt 0 ]; do\n",
            "          case \"$1\" in\n",
            "            --output) OUTPUT=\"$2\"; shift 2 ;;\n",
            "            --program) PROGRAM=\"$2\"; shift 2 ;;\n",
            "            *) shift ;;\n",
            "          esac\n",
            "        done\n",
            "        cat > \"$OUTPUT\" <<EOF\n",
            "version: 1\n",
            "program:\n",
            "  id: ${PROGRAM}\n",
            "  max_parallel: 1\n",
            "inputs:\n",
            "  doctrine_files: []\n",
            "  evidence_paths: []\n",
            "package:\n",
            "  fabro_root: fabro\n",
            "units: []\n",
            "EOF\n",
            "        ;;\n",
            "      evolve)\n",
            "        exit 0\n",
            "        ;;\n",
            "      *) exit 11 ;;\n",
            "    esac\n",
            "    ;;\n",
            "  run)\n",
            "    if [ \"$2\" != \"--detach\" ]; then exit 12; fi\n",
            "    RUN_ID=\"01KM244VBG7TF9FB8D53BFTHX7\"\n",
            "    RUN_DIR=\"$HOME/.fabro/runs/20260319-$RUN_ID\"\n",
            "    mkdir -p \"$RUN_DIR\"\n",
            "    cat > \"$RUN_DIR/status.json\" <<'EOF'\n",
            "{\"status\":\"running\",\"updated_at\":\"2026-03-19T00:00:01Z\"}\n",
            "EOF\n",
            "    cat > \"$RUN_DIR/state.json\" <<'EOF'\n",
            "{\"run_id\":\"01KM244VBG7TF9FB8D53BFTHX7\",\"updated_at\":\"2026-03-19T00:00:01Z\",\"status\":\"running\",\"current_stage_label\":\"Review\"}\n",
            "EOF\n",
            "    cat > \"$RUN_DIR/progress.jsonl\" <<'EOF'\n",
            "{\"ts\":\"2026-03-19T00:00:01Z\",\"run_id\":\"01KM244VBG7TF9FB8D53BFTHX7\",\"event\":\"StageStarted\",\"node_label\":\"Review\",\"event_seq\":1}\n",
            "EOF\n",
            "    printf '%s\\n' \"$RUN_ID\"\n",
            "    ;;\n",
            "  *) exit 13 ;;\n",
            "esac\n",
        ),
    )
    .expect("write fake fabro");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_fabro).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_fabro, perms).expect("chmod");
    }

    let manifest = temp.path().join("portfolio-program.yaml");
    raspberry()
        .args([
            "execute",
            "--manifest",
            manifest.to_str().expect("utf-8 manifest path"),
            "--fabro-bin",
            fake_fabro.to_str().expect("utf-8 fake fabro path"),
            "--lane",
            "ready:program",
        ])
        .env("HOME", temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Program: portfolio-demo"))
        .stdout(predicate::str::contains("ready:program [submitted]"));

    raspberry()
        .args([
            "status",
            "--manifest",
            manifest.to_str().expect("utf-8 manifest path"),
        ])
        .env("HOME", temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "ready:program [running|orchestration]",
        ))
        .stdout(predicate::str::contains("child program `ready-program`"))
        .stdout(predicate::str::contains("running=1"));

    assert!(temp
        .path()
        .join(".raspberry/ready-program-autodev.json")
        .exists());
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
