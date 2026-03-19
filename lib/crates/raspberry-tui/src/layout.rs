use std::collections::BTreeMap;

use ratatui::layout::{Constraint, Layout, Rect};

use crate::app::Pane;

pub const MIN_HEIGHT: u16 = 12;
pub const MIN_WIDTH: u16 = 60;

const COLLAPSED_WIDTH: u16 = 7;
const FOOTER_HEIGHT: u16 = 1;
const FOCUS_THRESHOLD_WIDTH: u16 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollapseState {
    pub program: bool,
    pub state: bool,
    pub artifacts: bool,
    pub detail: bool,
}

impl CollapseState {
    pub const fn open() -> Self {
        Self {
            program: false,
            state: false,
            artifacts: false,
            detail: false,
        }
    }

    pub fn is_collapsed(self, pane: Pane) -> bool {
        match pane {
            Pane::Program => self.program,
            Pane::State => self.state,
            Pane::Artifacts => self.artifacts,
            Pane::Detail => self.detail,
        }
    }

    pub fn set(&mut self, pane: Pane, collapsed: bool) {
        match pane {
            Pane::Program => self.program = collapsed,
            Pane::State => self.state = collapsed,
            Pane::Artifacts => self.artifacts = collapsed,
            Pane::Detail => self.detail = collapsed,
        }
    }

    pub fn toggle(&mut self, pane: Pane) {
        let collapsed = self.is_collapsed(pane);
        self.set(pane, !collapsed);
    }

    pub fn open_all(&mut self) {
        *self = Self::open();
    }

    pub fn close_secondary(&mut self) {
        self.program = false;
        self.state = true;
        self.artifacts = true;
        self.detail = true;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    TooSmall,
    Focused,
    Dashboard,
    Split,
}

#[derive(Debug, Clone)]
pub struct PaneRects {
    pub footer: Rect,
    pub mode: LayoutMode,
    pub panes: BTreeMap<Pane, Rect>,
}

pub fn split_screen(area: Rect, focus: Pane, collapsed: CollapseState) -> PaneRects {
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        return PaneRects {
            footer: area,
            mode: LayoutMode::TooSmall,
            panes: BTreeMap::new(),
        };
    }

    let vertical = Layout::vertical([Constraint::Min(1), Constraint::Length(FOOTER_HEIGHT)]);
    let [body, footer] = vertical.areas(area);

    if body.width < FOCUS_THRESHOLD_WIDTH {
        let mut panes = BTreeMap::new();
        panes.insert(focus, body);
        return PaneRects {
            footer,
            mode: LayoutMode::Focused,
            panes,
        };
    }

    if !collapsed.program && !collapsed.state && !collapsed.artifacts && !collapsed.detail {
        return PaneRects {
            footer,
            mode: LayoutMode::Dashboard,
            panes: split_dashboard(body),
        };
    }

    let panes = split_panes(body, collapsed);
    PaneRects {
        footer,
        mode: LayoutMode::Split,
        panes,
    }
}

fn split_dashboard(area: Rect) -> BTreeMap<Pane, Rect> {
    let [left, right] = Layout::horizontal([Constraint::Ratio(34, 100), Constraint::Ratio(66, 100)])
        .areas(area);
    let [state, lower] =
        Layout::vertical([Constraint::Ratio(38, 100), Constraint::Ratio(62, 100)]).areas(right);
    let [artifacts, detail] =
        Layout::horizontal([Constraint::Ratio(28, 100), Constraint::Ratio(72, 100)]).areas(lower);

    let mut output = BTreeMap::new();
    output.insert(Pane::Program, left);
    output.insert(Pane::State, state);
    output.insert(Pane::Artifacts, artifacts);
    output.insert(Pane::Detail, detail);
    output
}

fn split_panes(area: Rect, collapsed: CollapseState) -> BTreeMap<Pane, Rect> {
    let panes = Pane::ALL;
    let mut constraints = Vec::with_capacity(panes.len());
    for pane in panes {
        constraints.push(constraint_for_pane(pane, collapsed));
    }

    let mut output = BTreeMap::new();
    let rects: [Rect; 4] = Layout::horizontal(constraints).areas(area);
    for index in 0..panes.len() {
        output.insert(panes[index], rects[index]);
    }
    output
}

fn constraint_for_pane(pane: Pane, collapsed: CollapseState) -> Constraint {
    if collapsed.is_collapsed(pane) {
        return Constraint::Length(COLLAPSED_WIDTH);
    }
    match pane {
        Pane::Program => Constraint::Ratio(28, 100),
        Pane::State => Constraint::Ratio(22, 100),
        Pane::Artifacts => Constraint::Ratio(20, 100),
        Pane::Detail => Constraint::Ratio(30, 100),
    }
}
