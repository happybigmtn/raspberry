pub mod app;
pub mod files;
pub mod keys;
pub mod layout;
mod narration;
pub mod render;
mod runs;

use std::io::{self, IsTerminal};
use std::path::Path;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::App;

/// Runs the Raspberry observer TUI for a manifest path.
pub fn run(manifest_path: &Path) -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!("raspberry tui requires an interactive terminal");
    }

    let _session = TerminalSession::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).context("failed to initialize terminal backend")?;
    let mut app = App::load(manifest_path)?;

    loop {
        app.tick();
        terminal
            .draw(|frame| render::render(frame, &app))
            .context("failed to render Raspberry TUI")?;
        if app.should_quit() {
            break;
        }
        if event::poll(Duration::from_millis(100)).context("failed to poll terminal events")? {
            match event::read().context("failed to read terminal event")? {
                Event::Key(key) if should_handle_key(key.kind) => app.handle_key_event(key)?,
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    terminal
        .show_cursor()
        .context("failed to restore terminal cursor")?;
    Ok(())
}

struct TerminalSession;

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("failed to enable raw terminal mode")?;
        execute!(io::stdout(), EnterAlternateScreen).context("failed to enter alternate screen")?;
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

fn should_handle_key(kind: KeyEventKind) -> bool {
    matches!(kind, KeyEventKind::Press | KeyEventKind::Repeat)
}
