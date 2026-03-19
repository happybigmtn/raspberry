use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crossterm::event::KeyEvent;
use raspberry_supervisor::{
    load_optional_autodev_report, AutodevReport, AutodevStopReason, EvaluatedLane,
    EvaluatedProgram, LaneExecutionStatus, ProgramManifest,
};

use crate::files::{collect_lane_artifacts, preview_artifact, ArtifactEntry};
use crate::keys::{interpret_key, Command, InputMode, KeyAction, PendingSequence};
use crate::layout::CollapseState;
use crate::runs::{build_recent_run_index, RecentLaneRun};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pane {
    Program,
    State,
    Artifacts,
    Detail,
}

impl Pane {
    pub const ALL: [Self; 4] = [Self::Program, Self::State, Self::Artifacts, Self::Detail];

    pub fn title(self) -> &'static str {
        match self {
            Self::Program => "Program",
            Self::State => "State",
            Self::Artifacts => "Artifacts",
            Self::Detail => "Detail",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Program => Self::State,
            Self::State => Self::Artifacts,
            Self::Artifacts => Self::Detail,
            Self::Detail => Self::Program,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Program => Self::Detail,
            Self::State => Self::Program,
            Self::Artifacts => Self::State,
            Self::Detail => Self::Artifacts,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProgramRowKind {
    Summary,
    StatusHeader,
    Lane,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProgramRow {
    pub kind: ProgramRowKind,
    pub primary: String,
    pub secondary: Option<String>,
    pub lane_key: Option<String>,
    pub status: Option<LaneExecutionStatus>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ProgramCounts {
    running: usize,
    ready: usize,
    blocked: usize,
    failed: usize,
    complete: usize,
}

/// Application state for the read-only Raspberry run observer.
pub struct App {
    collapsed: CollapseState,
    detail_scroll: u16,
    filter_query: String,
    focus: Pane,
    input_mode: InputMode,
    manifest: ProgramManifest,
    manifest_path: PathBuf,
    pending_sequence: PendingSequence,
    program: EvaluatedProgram,
    autodev_report: Option<AutodevReport>,
    recent_runs: BTreeMap<String, Vec<RecentLaneRun>>,
    selected_artifact_index: usize,
    selected_lane_key: Option<String>,
    should_quit: bool,
    state_scroll: u16,
}

impl App {
    /// Loads the manifest and evaluated program used by the TUI.
    pub fn load(manifest_path: &Path) -> Result<Self> {
        Self::load_with_default_runs(manifest_path)
    }

    fn load_with_default_runs(manifest_path: &Path) -> Result<Self> {
        let manifest_path = manifest_path.to_path_buf();
        let manifest = ProgramManifest::load(&manifest_path).with_context(|| {
            format!(
                "failed to load Raspberry manifest {}",
                manifest_path.display()
            )
        })?;
        let program =
            raspberry_supervisor::evaluate_program(&manifest_path).with_context(|| {
                format!(
                    "failed to evaluate Raspberry manifest {}",
                    manifest_path.display()
                )
            })?;
        let autodev_report =
            load_optional_autodev_report(&manifest_path, &manifest).with_context(|| {
                format!(
                    "failed to load Raspberry autodev report for {}",
                    manifest_path.display()
                )
            })?;
        let recent_runs = build_recent_run_index(&manifest_path, &manifest, &program);
        let selected_lane_key = program.lanes.first().map(|lane| lane.lane_key.clone());

        Ok(Self {
            collapsed: CollapseState::open(),
            detail_scroll: 0,
            filter_query: String::new(),
            focus: Pane::Program,
            input_mode: InputMode::Normal,
            manifest,
            manifest_path,
            pending_sequence: PendingSequence::None,
            program,
            autodev_report,
            recent_runs,
            selected_artifact_index: 0,
            selected_lane_key,
            should_quit: false,
            state_scroll: 0,
        })
    }

    /// Applies one keyboard event to the TUI state machine.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        let resolution = interpret_key(self.input_mode, self.pending_sequence, key);
        self.input_mode = resolution.input_mode;
        self.pending_sequence = resolution.pending;

        match resolution.action {
            KeyAction::None => {}
            KeyAction::Command(command) => self.handle_command(command)?,
            KeyAction::AppendSearch(character) => {
                self.filter_query.push(character);
                self.sync_after_filter_change();
            }
            KeyAction::BackspaceSearch => {
                self.filter_query.pop();
                self.sync_after_filter_change();
            }
            KeyAction::SubmitSearch => self.pending_sequence = PendingSequence::None,
            KeyAction::CancelSearch => {
                self.input_mode = InputMode::Normal;
                self.pending_sequence = PendingSequence::None;
            }
        }
        Ok(())
    }

    /// Reloads the manifest and evaluated lane state from disk.
    pub fn refresh(&mut self) -> Result<()> {
        let selected_lane_key = self.selected_lane_key.clone();
        let selected_artifact = self.selected_artifact().map(|artifact| artifact.id);

        self.manifest = ProgramManifest::load(&self.manifest_path).with_context(|| {
            format!(
                "failed to reload Raspberry manifest {}",
                self.manifest_path.display()
            )
        })?;
        self.program =
            raspberry_supervisor::evaluate_program(&self.manifest_path).with_context(|| {
                format!(
                    "failed to refresh Raspberry manifest {}",
                    self.manifest_path.display()
                )
            })?;
        self.autodev_report = load_optional_autodev_report(&self.manifest_path, &self.manifest)
            .with_context(|| {
                format!(
                    "failed to refresh Raspberry autodev report for {}",
                    self.manifest_path.display()
                )
            })?;
        self.recent_runs =
            build_recent_run_index(&self.manifest_path, &self.manifest, &self.program);

        self.selected_lane_key = selected_lane_key;
        self.sync_lane_selection();
        self.restore_selected_artifact(selected_artifact.as_deref());
        Ok(())
    }

    pub fn selected_lane_key(&self) -> &str {
        self.selected_lane()
            .map(|lane| lane.lane_key.as_str())
            .unwrap_or("")
    }

    pub fn artifact_rows(&self) -> Vec<String> {
        let mut rows = Vec::new();
        for artifact in self.artifacts_for_selected_lane() {
            rows.push(format!(
                "{} [{}]",
                artifact.id,
                if artifact.exists {
                    "present"
                } else {
                    "missing"
                }
            ));
        }
        rows
    }

    pub fn state_text(&self) -> String {
        let Some(lane) = self.selected_lane() else {
            return "No lanes match the current filter.".to_string();
        };

        let counts = self.program_counts();
        let mut lines = vec![
            "Overview".to_string(),
            format!(
                "  RUN {} | RDY {} | BLK {} | FAIL {} | DONE {}",
                counts.running, counts.ready, counts.blocked, counts.failed, counts.complete
            ),
            String::new(),
            "Selected lane".to_string(),
            format!("  Status: {} ({})", status_label(lane.status), lane.lane_kind),
            format!("  Key: {}", lane.lane_key),
            format!("  Title: {}", lane.lane_title),
            format!("  Managed milestone: {}", lane.managed_milestone),
            format!("  Summary: {}", lane.detail),
        ];
        if let Some(proof_profile) = lane.proof_profile.as_ref() {
            lines.push(format!("  Proof profile: {}", proof_profile));
        }
        push_optional_prefixed_line(
            &mut lines,
            "  Preconditions",
            lane.precondition_state.map(|value| value.to_string()),
        );
        push_optional_prefixed_line(
            &mut lines,
            "  Proof state",
            lane.proof_state.map(|value| value.to_string()),
        );
        push_optional_prefixed_line(
            &mut lines,
            "  Operational state",
            lane.operational_state.map(|value| value.to_string()),
        );
        push_optional_prefixed_line(
            &mut lines,
            "  Orchestration state",
            lane.orchestration_state.map(|value| value.to_string()),
        );
        push_optional_prefixed_line(
            &mut lines,
            "  Current stage",
            lane.current_stage.clone(),
        );
        push_optional_prefixed_line(
            &mut lines,
            "  Last run id",
            lane.last_run_id.clone().or_else(|| lane.run_id.clone()),
        );
        push_optional_prefixed_line(
            &mut lines,
            "  Started at",
            lane.last_started_at.map(|value| value.to_rfc3339()),
        );
        push_optional_prefixed_line(
            &mut lines,
            "  Finished at",
            lane.last_finished_at.map(|value| value.to_rfc3339()),
        );
        if !lane.ready_checks_passing.is_empty()
            || !lane.ready_checks_failing.is_empty()
            || !lane.running_checks_passing.is_empty()
            || !lane.running_checks_failing.is_empty()
        {
            lines.push(String::new());
            lines.push("Checks".to_string());
            if !lane.ready_checks_passing.is_empty() {
                lines.push(format!(
                    "  Ready passing: {}",
                    lane.ready_checks_passing.join(", ")
                ));
            }
            if !lane.ready_checks_failing.is_empty() {
                lines.push(format!(
                    "  Ready failing: {}",
                    lane.ready_checks_failing.join(", ")
                ));
            }
            if !lane.running_checks_passing.is_empty() {
                lines.push(format!(
                    "  Running passing: {}",
                    lane.running_checks_passing.join(", ")
                ));
            }
            if !lane.running_checks_failing.is_empty() {
                lines.push(format!(
                    "  Running failing: {}",
                    lane.running_checks_failing.join(", ")
                ));
            }
        }
        if let Some(summary) = self.autodev_lane_summary(lane) {
            lines.push(String::new());
            lines.push(summary);
        }
        lines.join("\n")
    }

    pub fn detail_text(&self) -> String {
        let Some(lane) = self.selected_lane() else {
            return "No lanes match the current filter.".to_string();
        };

        let artifacts = self.artifacts_for_selected_lane();
        let mut sections = Vec::new();
        if lane.status == LaneExecutionStatus::Complete {
            sections.push(self.completed_result_text(lane, &artifacts));
        }
        if let Some(autodev) = self.autodev_detail_text(lane) {
            sections.push(autodev);
        }
        sections.push(self.live_detail_text(lane));
        if let Some(runs) = self.recent_runs.get(&lane.lane_key) {
            if !runs.is_empty() {
                sections.push(self.recent_runs_text(runs));
            }
        }
        if let Some(artifact) = artifacts.get(self.selected_artifact_index).cloned() {
            sections.push(preview_artifact(&artifact).unwrap_or_else(|error| error.to_string()));
        } else {
            sections.push("No curated artifacts are associated with this lane.".to_string());
        }
        sections.join("\n\n")
    }

    pub fn focus(&self) -> Pane {
        self.focus
    }

    pub fn is_collapsed(&self, pane: Pane) -> bool {
        self.collapsed.is_collapsed(pane)
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub(crate) fn collapse_state(&self) -> CollapseState {
        self.collapsed
    }

    pub(crate) fn detail_scroll(&self) -> u16 {
        self.detail_scroll
    }

    pub(crate) fn footer_text(&self) -> String {
        let mode = match self.input_mode {
            InputMode::Normal => "normal".to_string(),
            InputMode::Search => format!("search /{}", self.filter_query),
        };
        format!(
            "{} | focus={} | h/l panes j/k move gg/G jump za fold / filter r refresh q quit",
            mode,
            self.focus.title()
        )
    }

    pub(crate) fn has_visible_lanes(&self) -> bool {
        !self.visible_lanes().is_empty()
    }

    pub(crate) fn program_rows(&self) -> Vec<ProgramRow> {
        let mut rows = Vec::new();
        let counts = self.program_counts();
        rows.push(ProgramRow {
            kind: ProgramRowKind::Summary,
            primary: format!(
                "RUN {} | RDY {} | BLK {} | FAIL {} | DONE {}",
                counts.running, counts.ready, counts.blocked, counts.failed, counts.complete
            ),
            secondary: Some(format!(
                "{} visible lane(s) in {}",
                self.visible_lanes().len(),
                self.program.program
            )),
            lane_key: None,
            status: None,
        });

        let groups = [
            LaneExecutionStatus::Running,
            LaneExecutionStatus::Ready,
            LaneExecutionStatus::Blocked,
            LaneExecutionStatus::Failed,
            LaneExecutionStatus::Complete,
        ];

        for status in groups {
            let lanes = self
                .visible_lanes()
                .into_iter()
                .filter(|lane| lane.status == status)
                .collect::<Vec<_>>();
            if lanes.is_empty() {
                continue;
            }
            rows.push(ProgramRow {
                kind: ProgramRowKind::StatusHeader,
                primary: format!("{} ({})", status_label(status), lanes.len()),
                secondary: None,
                lane_key: None,
                status: Some(status),
            });
            for lane in lanes {
                rows.push(ProgramRow {
                    kind: ProgramRowKind::Lane,
                    primary: format!("{}  {}", status_badge(status), lane.lane_key),
                    secondary: Some(format!("{} | {}", lane.lane_title, lane.detail)),
                    lane_key: Some(lane.lane_key.clone()),
                    status: Some(status),
                });
            }
        }
        rows
    }

    pub(crate) fn selected_artifact_index(&self) -> Option<usize> {
        if self.artifacts_for_selected_lane().is_empty() {
            return None;
        }
        Some(self.selected_artifact_index)
    }

    pub(crate) fn selected_program_row_index(&self) -> Option<usize> {
        let selected_lane_key = self.selected_lane_key();
        self.program_rows().iter().position(|row| {
            row.kind == ProgramRowKind::Lane
                && row.lane_key.as_deref() == Some(selected_lane_key)
        })
    }

    pub(crate) fn state_scroll(&self) -> u16 {
        self.state_scroll
    }

    fn artifacts_for_selected_lane(&self) -> Vec<ArtifactEntry> {
        let Some(lane) = self.selected_lane() else {
            return Vec::new();
        };
        collect_lane_artifacts(&self.manifest_path, &self.manifest, lane)
    }

    fn handle_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::FocusLeft => self.focus = self.focus.previous(),
            Command::FocusRight | Command::CycleFocus => self.focus = self.focus.next(),
            Command::MoveUp => self.move_up(),
            Command::MoveDown => self.move_down(),
            Command::MoveTop => self.move_top(),
            Command::MoveBottom => self.move_bottom(),
            Command::Activate => self.activate(),
            Command::ToggleCollapse => self.collapsed.toggle(self.focus),
            Command::OpenPane => self.collapsed.set(self.focus, false),
            Command::ClosePane => self.collapsed.set(self.focus, true),
            Command::OpenAllPanes => self.collapsed.open_all(),
            Command::CloseSecondaryPanes => self.collapsed.close_secondary(),
            Command::Refresh => self.refresh()?,
            Command::Quit => self.should_quit = true,
        }
        Ok(())
    }

    fn selected_lane(&self) -> Option<&EvaluatedLane> {
        let visible = self.visible_lanes();
        if visible.is_empty() {
            return None;
        }
        let Some(selected_lane_key) = self.selected_lane_key.as_deref() else {
            return visible.into_iter().next();
        };
        visible
            .into_iter()
            .find(|lane| lane.lane_key == selected_lane_key)
            .or_else(|| self.program.lanes.first())
    }

    fn selected_artifact(&self) -> Option<ArtifactEntry> {
        let artifacts = self.artifacts_for_selected_lane();
        artifacts.get(self.selected_artifact_index).cloned()
    }

    fn visible_lanes(&self) -> Vec<&EvaluatedLane> {
        let query = self.filter_query.to_ascii_lowercase();
        self.program
            .lanes
            .iter()
            .filter(|lane| lane_matches(lane, &query))
            .collect()
    }

    fn move_down(&mut self) {
        match self.focus {
            Pane::Program => self.move_lane_selection(1),
            Pane::Artifacts => self.move_artifact_selection(1),
            Pane::State => self.state_scroll = self.state_scroll.saturating_add(1),
            Pane::Detail => self.detail_scroll = self.detail_scroll.saturating_add(1),
        }
    }

    fn move_up(&mut self) {
        match self.focus {
            Pane::Program => self.move_lane_selection(-1),
            Pane::Artifacts => self.move_artifact_selection(-1),
            Pane::State => self.state_scroll = self.state_scroll.saturating_sub(1),
            Pane::Detail => self.detail_scroll = self.detail_scroll.saturating_sub(1),
        }
    }

    fn move_top(&mut self) {
        match self.focus {
            Pane::Program => self.select_lane_at(0),
            Pane::Artifacts => self.selected_artifact_index = 0,
            Pane::State => self.state_scroll = 0,
            Pane::Detail => self.detail_scroll = 0,
        }
    }

    fn move_bottom(&mut self) {
        match self.focus {
            Pane::Program => {
                let visible = self.visible_lanes();
                if !visible.is_empty() {
                    self.select_lane_at(visible.len() - 1);
                }
            }
            Pane::Artifacts => {
                let artifacts = self.artifacts_for_selected_lane();
                if !artifacts.is_empty() {
                    self.selected_artifact_index = artifacts.len() - 1;
                }
            }
            Pane::State => self.state_scroll = u16::MAX,
            Pane::Detail => self.detail_scroll = u16::MAX,
        }
    }

    fn activate(&mut self) {
        self.focus = match self.focus {
            Pane::Program => Pane::Artifacts,
            Pane::State => Pane::Detail,
            Pane::Artifacts => Pane::Detail,
            Pane::Detail => Pane::Detail,
        };
    }

    fn move_artifact_selection(&mut self, delta: isize) {
        let artifacts = self.artifacts_for_selected_lane();
        if artifacts.is_empty() {
            return;
        }
        let next = offset_index(self.selected_artifact_index, delta, artifacts.len());
        self.selected_artifact_index = next;
        self.detail_scroll = 0;
    }

    fn move_lane_selection(&mut self, delta: isize) {
        let visible = self.visible_lane_keys();
        if visible.is_empty() {
            return;
        }
        let current = visible
            .iter()
            .position(|lane_key| Some(lane_key.as_str()) == self.selected_lane_key.as_deref())
            .unwrap_or(0);
        let next = offset_index(current, delta, visible.len());
        self.selected_lane_key = Some(visible[next].clone());
        self.reset_lane_derived_state();
    }

    fn reset_lane_derived_state(&mut self) {
        self.selected_artifact_index = 0;
        self.state_scroll = 0;
        self.detail_scroll = 0;
    }

    fn restore_selected_artifact(&mut self, artifact_id: Option<&str>) {
        self.sync_artifact_selection();
        let Some(artifact_id) = artifact_id else {
            return;
        };
        let artifacts = self.artifacts_for_selected_lane();
        if let Some(index) = artifacts
            .iter()
            .position(|artifact| artifact.id == artifact_id)
        {
            self.selected_artifact_index = index;
        }
    }

    fn select_lane_at(&mut self, index: usize) {
        let visible = self.visible_lane_keys();
        if visible.is_empty() {
            return;
        }
        let index = index.min(visible.len() - 1);
        self.selected_lane_key = Some(visible[index].clone());
        self.reset_lane_derived_state();
    }

    fn sync_after_filter_change(&mut self) {
        self.sync_lane_selection();
        self.sync_artifact_selection();
    }

    fn sync_artifact_selection(&mut self) {
        let artifacts = self.artifacts_for_selected_lane();
        if artifacts.is_empty() {
            self.selected_artifact_index = 0;
            return;
        }
        if self.selected_artifact_index >= artifacts.len() {
            self.selected_artifact_index = artifacts.len() - 1;
        }
    }

    fn sync_lane_selection(&mut self) {
        let visible = self.visible_lane_keys();
        if visible.is_empty() {
            self.selected_lane_key = None;
            self.reset_lane_derived_state();
            return;
        }
        if let Some(selected) = self.selected_lane_key.as_ref() {
            if visible.iter().any(|lane_key| lane_key == selected) {
                return;
            }
        }
        self.selected_lane_key = Some(visible[0].clone());
        self.reset_lane_derived_state();
    }

    fn visible_lane_keys(&self) -> Vec<String> {
        self.visible_lanes()
            .into_iter()
            .map(|lane| lane.lane_key.clone())
            .collect()
    }

    fn program_counts(&self) -> ProgramCounts {
        let mut counts = ProgramCounts::default();
        for lane in self.visible_lanes() {
            match lane.status {
                LaneExecutionStatus::Running => counts.running += 1,
                LaneExecutionStatus::Ready => counts.ready += 1,
                LaneExecutionStatus::Blocked => counts.blocked += 1,
                LaneExecutionStatus::Failed => counts.failed += 1,
                LaneExecutionStatus::Complete => counts.complete += 1,
            }
        }
        counts
    }

    fn live_detail_text(&self, lane: &EvaluatedLane) -> String {
        let freshness = live_detail_freshness(lane);
        let mut lines = vec!["Live run detail".to_string()];
        lines.push(format!("  Freshness: {}", freshness));
        push_optional_prefixed_line(
            &mut lines,
            "  Run id",
            lane.current_fabro_run_id
                .clone()
                .or_else(|| lane.run_id.clone())
                .or_else(|| lane.last_run_id.clone()),
        );
        push_optional_prefixed_line(&mut lines, "  Current stage", lane.current_stage.clone());
        push_optional_prefixed_line(
            &mut lines,
            "  Last completed stage",
            lane.last_completed_stage_label.clone(),
        );
        push_optional_prefixed_line(&mut lines, "  Usage", lane.last_usage_summary.clone());
        push_optional_prefixed_line(&mut lines, "  Error", lane.last_error.clone());
        if !lane.last_files_read.is_empty() {
            lines.push(format!("  Files read: {}", lane.last_files_read.join(", ")));
        }
        if !lane.last_files_written.is_empty() {
            lines.push(format!(
                "  Files written: {}",
                lane.last_files_written.join(", ")
            ));
        }
        lines.join("\n")
    }

    fn completed_result_text(&self, lane: &EvaluatedLane, artifacts: &[ArtifactEntry]) -> String {
        let mut lines = vec![
            "Completed result".to_string(),
            format!("Managed milestone: {}", lane.managed_milestone),
        ];

        let present = artifacts
            .iter()
            .filter(|artifact| artifact.exists)
            .map(|artifact| artifact.id.as_str())
            .collect::<Vec<_>>();
        let missing = artifacts
            .iter()
            .filter(|artifact| !artifact.exists)
            .map(|artifact| artifact.id.as_str())
            .collect::<Vec<_>>();

        if !present.is_empty() {
            lines.push(format!("Artifacts present: {}", present.join(", ")));
        }
        if !missing.is_empty() {
            lines.push(format!("Artifacts missing: {}", missing.join(", ")));
        }
        push_optional_line(
            &mut lines,
            "Last completed stage",
            lane.last_completed_stage_label.clone(),
        );
        push_optional_line(
            &mut lines,
            "Finished at",
            lane.last_finished_at.map(|value| value.to_rfc3339()),
        );
        push_optional_line(
            &mut lines,
            "Exit status",
            lane.last_exit_status.map(|value| value.to_string()),
        );
        push_optional_line(&mut lines, "Usage", lane.last_usage_summary.clone());
        if !lane.last_files_written.is_empty() {
            lines.push(format!(
                "Files written: {}",
                lane.last_files_written.join(", ")
            ));
        }
        if let Some(stdout) = lane.last_stdout_snippet.as_ref() {
            lines.push(format!("stdout: {}", stdout));
        }
        if let Some(stderr) = lane.last_stderr_snippet.as_ref() {
            lines.push(format!("stderr: {}", stderr));
        }
        if lane.last_run_id.is_none() && lane.last_exit_status.is_none() {
            lines.push(
                "No persisted completed-run record was found, so this summary is derived from \
                 curated artifacts and milestone state."
                    .to_string(),
            );
        }
        lines.join("\n")
    }

    fn recent_runs_text(&self, runs: &[RecentLaneRun]) -> String {
        let mut lines = vec!["Recent successful runs".to_string()];
        for run in runs {
            lines.push(format!(
                "{} [{}] {}",
                run.run_id, run.status, run.workflow_name
            ));
            if let Some(finished_at) = run.finished_at {
                lines.push(format!("Finished at: {}", finished_at.to_rfc3339()));
            }
            push_optional_line(
                &mut lines,
                "Last completed stage",
                run.last_completed_stage_label.clone(),
            );
            push_optional_line(&mut lines, "Usage", run.usage_summary.clone());
            if !run.matched_files.is_empty() {
                lines.push(format!(
                    "Matched artifacts: {}",
                    run.matched_files.join(", ")
                ));
            }
        }
        lines.join("\n")
    }

    fn autodev_lane_summary(&self, lane: &EvaluatedLane) -> Option<String> {
        let report = self.autodev_report.as_ref()?;
        let last_cycle = report.cycles.last()?;
        let mut summary = format!(
            "Autodev: cycles={} stop={}",
            report.cycles.len(),
            autodev_stop_reason(report.stop_reason)
        );
        if last_cycle
            .ready_lanes
            .iter()
            .any(|key| key == &lane.lane_key)
        {
            summary.push_str(" | ready_in_last_cycle=yes");
        }
        if last_cycle
            .dispatched
            .iter()
            .any(|outcome| outcome.lane_key == lane.lane_key)
        {
            summary.push_str(" | dispatched_in_last_cycle=yes");
        }
        Some(summary)
    }

    fn autodev_detail_text(&self, lane: &EvaluatedLane) -> Option<String> {
        let report = self.autodev_report.as_ref()?;
        let last_cycle = report.cycles.last()?;
        let mut lines = vec![
            "Autodev".to_string(),
            format!("  Stop reason: {}", autodev_stop_reason(report.stop_reason)),
            format!("  Cycles recorded: {}", report.cycles.len()),
            format!("  Last cycle: {}", last_cycle.cycle),
        ];
        if last_cycle.evolved {
            lines.push(format!(
                "  Evolve target: {}",
                last_cycle.evolve_target.as_deref().unwrap_or("live repo")
            ));
        }
        if last_cycle
            .ready_lanes
            .iter()
            .any(|key| key == &lane.lane_key)
        {
            lines.push("  Selected lane was ready in the last autodev cycle.".to_string());
        }
        if let Some(outcome) = last_cycle
            .dispatched
            .iter()
            .find(|outcome| outcome.lane_key == lane.lane_key)
        {
            lines.push(format!(
                "  Selected lane was dispatched in the last cycle: run_id={} exit_status={}",
                outcome.fabro_run_id.as_deref().unwrap_or("unknown"),
                outcome.exit_status
            ));
        }
        Some(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use fabro_workflows::manifest::Manifest as RunManifest;
    use fabro_workflows::run_status::{RunStatus, RunStatusRecord};

    use super::*;

    #[test]
    fn detail_text_surfaces_recent_successful_runs_for_complete_lanes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo_root = temp.path().join("repo");
        let runs_base = temp.path().join("runs");
        std::fs::create_dir_all(repo_root.join("outputs/validator/oracle")).expect("outputs");
        std::fs::create_dir_all(repo_root.join("run-configs")).expect("run-configs");
        std::fs::write(
            repo_root.join("outputs/validator/oracle/spec.md"),
            "validator spec",
        )
        .expect("spec");
        std::fs::write(
            repo_root.join("outputs/validator/oracle/review.md"),
            "validator review",
        )
        .expect("review");
        let manifest_path = repo_root.join("program.yaml");
        std::fs::write(
            &manifest_path,
            indoc::indoc! {r#"
                version: 1
                program: demo
                target_repo: .
                state_path: .raspberry/program-state.json
                units:
                  - id: validator
                    title: Validator Oracle
                    output_root: outputs/validator/oracle
                    artifacts:
                      - id: spec
                        path: spec.md
                      - id: review
                        path: review.md
                    milestones:
                      - id: reviewed
                        requires: [spec, review]
                    lanes:
                      - id: oracle
                        kind: service
                        title: Oracle
                        run_config: run-configs/oracle.toml
                        managed_milestone: reviewed
                        produces: [spec, review]
            "#},
        )
        .expect("manifest");

        let run_dir = runs_base.join("20260319-01TESTRECENTAPP000000000000");
        std::fs::create_dir_all(&run_dir).expect("run dir");
        RunManifest {
            run_id: "01TESTRECENTAPP000000000000".to_string(),
            workflow_name: "BootstrapValidatorOracle".to_string(),
            goal: "Bootstrap the Myosu `validator:oracle` lane.".to_string(),
            start_time: Utc::now(),
            node_count: 3,
            edge_count: 2,
            run_branch: None,
            base_sha: None,
            labels: HashMap::new(),
            base_branch: None,
            workflow_slug: Some("services".to_string()),
            host_repo_path: Some(repo_root.display().to_string()),
        }
        .save(&run_dir.join("manifest.json"))
        .expect("run manifest");
        RunStatusRecord::new(RunStatus::Succeeded, None)
            .save(&run_dir.join("status.json"))
            .expect("status");
        std::fs::write(
            run_dir.join("progress.jsonl"),
            indoc::indoc! {r#"
                {"ts":"2026-03-19T06:39:36Z","run_id":"01TESTRECENTAPP000000000000","event":"StageCompleted","node_label":"Inventory","duration_ms":1000,"files_written":["outputs/validator/oracle/spec.md","outputs/validator/oracle/review.md"],"usage":{"model":"gpt-5.4","input_tokens":100,"output_tokens":80}}
                {"ts":"2026-03-19T06:39:37Z","run_id":"01TESTRECENTAPP000000000000","event":"WorkflowRunCompleted","duration_ms":1200,"status":"success","usage":{"model":"gpt-5.4","input_tokens":100,"output_tokens":80}}
            "#},
        )
        .expect("progress");

        let manifest = ProgramManifest::load(&manifest_path).expect("manifest loads");
        let program =
            raspberry_supervisor::evaluate_program(&manifest_path).expect("program evaluates");
        let recent_runs = crate::runs::build_recent_run_index_in_base(
            &manifest_path,
            &manifest,
            &program,
            &runs_base,
        );
        let app = App {
            collapsed: CollapseState::open(),
            detail_scroll: 0,
            filter_query: String::new(),
            focus: Pane::Program,
            input_mode: InputMode::Normal,
            manifest,
            manifest_path,
            pending_sequence: PendingSequence::None,
            program,
            autodev_report: None,
            recent_runs,
            selected_artifact_index: 0,
            selected_lane_key: Some("validator:oracle".to_string()),
            should_quit: false,
            state_scroll: 0,
        };

        let detail = app.detail_text();
        assert!(detail.contains("Recent successful runs"));
        assert!(detail.contains("01TESTRECENTAPP000000000000"));
        assert!(detail.contains("BootstrapValidatorOracle"));
        assert!(detail.contains("Matched artifacts: spec, review"));
    }
}

fn lane_matches(lane: &EvaluatedLane, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let haystacks = [
        lane.lane_key.to_ascii_lowercase(),
        lane.lane_title.to_ascii_lowercase(),
        lane.unit_id.to_ascii_lowercase(),
        lane.unit_title.to_ascii_lowercase(),
        lane.detail.to_ascii_lowercase(),
    ];
    haystacks.iter().any(|value| value.contains(query))
}

fn live_detail_freshness(lane: &EvaluatedLane) -> &'static str {
    if lane.status == LaneExecutionStatus::Running {
        return "live";
    }
    if lane.run_id.is_some() || lane.last_run_id.is_some() || lane.last_error.is_some() {
        return "stale";
    }
    "unavailable"
}

fn offset_index(current: usize, delta: isize, len: usize) -> usize {
    let next = current.saturating_add_signed(delta);
    next.min(len.saturating_sub(1))
}

fn push_optional_line(lines: &mut Vec<String>, label: &str, value: Option<String>) {
    let Some(value) = value else {
        return;
    };
    lines.push(format!("{}: {}", label, value));
}

fn push_optional_prefixed_line(lines: &mut Vec<String>, label: &str, value: Option<String>) {
    let Some(value) = value else {
        return;
    };
    lines.push(format!("{}: {}", label, value));
}

fn status_label(status: LaneExecutionStatus) -> &'static str {
    match status {
        LaneExecutionStatus::Running => "RUNNING",
        LaneExecutionStatus::Ready => "READY",
        LaneExecutionStatus::Blocked => "BLOCKED",
        LaneExecutionStatus::Failed => "FAILED",
        LaneExecutionStatus::Complete => "COMPLETE",
    }
}

fn status_badge(status: LaneExecutionStatus) -> &'static str {
    match status {
        LaneExecutionStatus::Running => "RUN",
        LaneExecutionStatus::Ready => "RDY",
        LaneExecutionStatus::Blocked => "BLK",
        LaneExecutionStatus::Failed => "BAD",
        LaneExecutionStatus::Complete => "OK ",
    }
}

fn autodev_stop_reason(reason: AutodevStopReason) -> &'static str {
    match reason {
        AutodevStopReason::Settled => "settled",
        AutodevStopReason::CycleLimit => "cycle_limit",
    }
}
