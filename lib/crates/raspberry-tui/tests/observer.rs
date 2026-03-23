use std::path::{Path, PathBuf};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use raspberry_supervisor::{
    autodev_report_path, AutodevCycleReport, AutodevReport, AutodevStopReason, DispatchOutcome,
    ProgramManifest,
};
use raspberry_tui::app::{App, Pane};
use raspberry_tui::render;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn myosu_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures/raspberry-supervisor/myosu-program.yaml")
}

fn portfolio_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures/raspberry-supervisor/portfolio-program.yaml")
}

fn press_char(app: &mut App, character: char) -> Result<()> {
    let event = KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
    app.handle_key_event(event)
}

fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer().clone();
    let mut lines = Vec::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            line.push_str(buffer[(x, y)].symbol());
        }
        lines.push(line);
    }
    lines.join("\n")
}

#[test]
fn myosu_fixture_exposes_present_artifacts() -> Result<()> {
    let app = App::load(&myosu_manifest())?;

    assert_eq!(app.selected_lane_key(), "chain:runtime");
    assert!(app
        .artifact_rows()
        .iter()
        .any(|row| row.contains("runtime_spec [present]")));
    assert!(app.state_text().contains("Proof profile: cargo_workspace"));
    assert!(app
        .detail_text()
        .contains("Chain runtime specification exists."));
    Ok(())
}

#[test]
fn complete_lane_detail_surfaces_completed_result_summary() -> Result<()> {
    let app = App::load(&myosu_manifest())?;

    let detail = app.detail_text();
    assert!(detail.contains("Completed result"));
    assert!(detail.contains("Managed milestone: reviewed"));
    assert!(detail.contains("Artifacts present: runtime_spec, runtime_review"));
    Ok(())
}

#[test]
fn failed_lane_shows_missing_artifact_and_stale_detail() -> Result<()> {
    let mut app = App::load(&myosu_manifest())?;
    press_char(&mut app, 'k')?;
    assert_eq!(app.selected_lane_key(), "play:tui");

    assert!(app
        .artifact_rows()
        .iter()
        .any(|row| row.contains("tui_spec [missing]")));
    let detail = app.detail_text();
    assert!(detail.contains("Missing artifact"));
    assert!(detail.contains("Live run detail"));
    assert!(detail.contains("Freshness: stale"));
    assert!(detail.contains("terminal snapshot mismatch"));
    Ok(())
}

#[test]
fn detail_text_surfaces_autodev_summary_when_present() -> Result<()> {
    let fixture_root = myosu_manifest()
        .parent()
        .expect("fixture manifest parent")
        .to_path_buf();
    let temp = tempfile::tempdir().expect("tempdir");
    copy_dir(&fixture_root, temp.path())?;
    let manifest_path = temp.path().join("myosu-program.yaml");
    let manifest = ProgramManifest::load(&manifest_path)?;
    let report_path = autodev_report_path(&manifest_path, &manifest);
    std::fs::create_dir_all(
        report_path
            .parent()
            .expect("autodev report should have parent"),
    )?;
    let report = AutodevReport {
        program: manifest.program.clone(),
        stop_reason: AutodevStopReason::CycleLimit,
        updated_at: chrono::Utc::now(),
        provenance: None,
        current: None,
        cycles: vec![AutodevCycleReport {
            cycle: 1,
            evolved: true,
            evolve_target: Some("/tmp/preview".to_string()),
            ready_lanes: vec!["chain:runtime".to_string()],
            replayed_lanes: vec![],
            regenerate_noop_lanes: vec![],
            dispatched: vec![DispatchOutcome {
                lane_key: "chain:runtime".to_string(),
                exit_status: 0,
                fabro_run_id: Some("01KM244VBG7TF9FB8D53BFTHX7".to_string()),
                stdout: String::new(),
                stderr: String::new(),
            }],
            running_after: 1,
            complete_after: 0,
        }],
    };
    std::fs::write(&report_path, serde_json::to_string_pretty(&report)?)?;

    let app = App::load(&manifest_path)?;
    let detail = app.detail_text();
    let state = app.state_text();

    assert!(detail.contains("Autodev"));
    assert!(detail.contains("Stop reason: cycle_limit"));
    assert!(detail.contains("Selected lane was ready in the last autodev cycle."));
    assert!(detail.contains("run_id=01KM244VBG7TF9FB8D53BFTHX7"));
    assert!(state.contains("Autodev: cycles=1 stop=cycle_limit"));
    Ok(())
}

#[test]
fn state_text_surfaces_child_program_digest_for_orchestration_lane() -> Result<()> {
    let app = App::load(&portfolio_manifest())?;
    let state = app.state_text();

    assert!(state.contains("Child program"));
    assert!(state.contains("RUN"));
    Ok(())
}

#[test]
fn fold_commands_collapse_and_expand_secondary_panes() -> Result<()> {
    let mut app = App::load(&myosu_manifest())?;

    press_char(&mut app, 'l')?;
    press_char(&mut app, 'z')?;
    press_char(&mut app, 'a')?;
    assert!(app.is_collapsed(Pane::State));

    press_char(&mut app, 'z')?;
    press_char(&mut app, 'o')?;
    assert!(!app.is_collapsed(Pane::State));

    press_char(&mut app, 'z')?;
    press_char(&mut app, 'M')?;
    assert!(!app.is_collapsed(Pane::Program));
    assert!(app.is_collapsed(Pane::State));
    assert!(app.is_collapsed(Pane::Artifacts));
    assert!(app.is_collapsed(Pane::Detail));
    Ok(())
}

#[test]
fn render_draws_all_pane_titles_on_wide_terminal() -> Result<()> {
    let app = App::load(&myosu_manifest())?;
    let backend = TestBackend::new(160, 40);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|frame| render::render(frame, &app))?;
    let contents = buffer_text(&terminal);

    assert!(contents.contains("Program"));
    assert!(contents.contains("State"));
    assert!(contents.contains("Artifacts"));
    assert!(contents.contains("Detail"));
    Ok(())
}

fn copy_dir(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    for entry in walk(source)? {
        let relative = entry.strip_prefix(source).expect("prefix");
        let destination = target.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&destination)?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&entry, &destination)?;
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
    let mut entries = std::fs::read_dir(root)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    for entry in entries {
        visit(&entry, paths)?;
    }
    Ok(())
}
