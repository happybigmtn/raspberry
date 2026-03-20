use raspberry_supervisor::LaneExecutionStatus;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Pane, ProgramRow, ProgramRowKind};
use crate::layout::{split_screen, LayoutMode};

/// Renders one frame of the Raspberry observer TUI.
pub fn render(frame: &mut Frame<'_>, app: &App) {
    let layout = split_screen(frame.area(), app.focus(), app.collapse_state());
    match layout.mode {
        LayoutMode::TooSmall => render_too_small(frame),
        LayoutMode::Focused | LayoutMode::Dashboard | LayoutMode::Split => {
            for pane in Pane::ALL {
                let Some(area) = layout.panes.get(&pane).copied() else {
                    continue;
                };
                render_pane(frame, app, pane, area);
            }
            let footer =
                Paragraph::new(app.footer_text()).style(Style::default().fg(Color::DarkGray));
            frame.render_widget(footer, layout.footer);
        }
    }
}

fn render_too_small(frame: &mut Frame<'_>) {
    let warning = Paragraph::new(
        "Terminal is too small for the Raspberry observer.\nResize to at least 60x12.",
    )
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Raspberry TUI"),
    );
    frame.render_widget(warning, frame.area());
}

fn render_pane(frame: &mut Frame<'_>, app: &App, pane: Pane, area: Rect) {
    if app.is_collapsed(pane) && area.width <= 8 {
        let title = abbreviate_title(pane);
        let widget = Paragraph::new(title).block(block_for_pane(app, pane));
        frame.render_widget(widget, area);
        return;
    }

    match pane {
        Pane::Program => render_program_pane(frame, app, area),
        Pane::State => render_text_pane(
            frame,
            app.state_text(),
            app.state_scroll(),
            block_for_pane(app, pane),
            area,
        ),
        Pane::Artifacts => render_artifacts_pane(frame, app, area),
        Pane::Detail => render_text_pane(
            frame,
            app.detail_text(),
            app.detail_scroll(),
            block_for_pane(app, pane),
            area,
        ),
    }
}

fn render_artifacts_pane(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let rows = app.artifact_rows();
    if rows.is_empty() {
        let empty = Paragraph::new("No curated artifacts for the selected lane.")
            .block(block_for_pane(app, Pane::Artifacts));
        frame.render_widget(empty, area);
        return;
    }

    let items = rows.into_iter().map(ListItem::new).collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(app.selected_artifact_index());
    let list = List::new(items)
        .block(block_for_pane(app, Pane::Artifacts))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_program_pane(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if !app.has_visible_lanes() {
        let empty = Paragraph::new("No lanes match the current filter.")
            .block(block_for_pane(app, Pane::Program));
        frame.render_widget(empty, area);
        return;
    }

    let rows = app.program_rows();
    let items = rows.iter().map(program_row_item).collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(app.selected_program_row_index());
    let list = List::new(items)
        .block(block_for_pane(app, Pane::Program))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_text_pane(
    frame: &mut Frame<'_>,
    text: String,
    scroll: u16,
    block: Block<'static>,
    area: Rect,
) {
    let paragraph = Paragraph::new(text)
        .block(block)
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn block_for_pane(app: &App, pane: Pane) -> Block<'static> {
    let mut title = match pane {
        Pane::Program => "Dashboard".to_string(),
        _ => pane.title().to_string(),
    };
    if app.is_collapsed(pane) {
        title.push_str(" (folded)");
    }
    let border_style = if app.focus() == pane {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
}

fn program_row_item(row: &ProgramRow) -> ListItem<'static> {
    match row.kind {
        ProgramRowKind::Summary => ListItem::new(vec![
            Line::styled(
                row.primary.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::styled(
                row.secondary.clone().unwrap_or_default(),
                Style::default().fg(Color::Gray),
            ),
        ]),
        ProgramRowKind::StatusHeader => ListItem::new(Line::styled(
            row.primary.clone(),
            Style::default()
                .fg(status_color(
                    row.status.unwrap_or(LaneExecutionStatus::Blocked),
                ))
                .add_modifier(Modifier::BOLD),
        )),
        ProgramRowKind::Lane => ListItem::new(vec![
            Line::styled(
                row.primary.clone(),
                Style::default()
                    .fg(status_color(
                        row.status.unwrap_or(LaneExecutionStatus::Blocked),
                    ))
                    .add_modifier(Modifier::BOLD),
            ),
            Line::styled(
                row.secondary.clone().unwrap_or_default(),
                Style::default().fg(Color::Gray),
            ),
        ]),
    }
}

fn status_color(status: LaneExecutionStatus) -> Color {
    match status {
        LaneExecutionStatus::Running => Color::Cyan,
        LaneExecutionStatus::Ready => Color::Green,
        LaneExecutionStatus::Blocked => Color::Yellow,
        LaneExecutionStatus::Failed => Color::Red,
        LaneExecutionStatus::Complete => Color::Blue,
    }
}

fn abbreviate_title(pane: Pane) -> &'static str {
    match pane {
        Pane::Program => "P",
        Pane::State => "S",
        Pane::Artifacts => "A",
        Pane::Detail => "D",
    }
}
