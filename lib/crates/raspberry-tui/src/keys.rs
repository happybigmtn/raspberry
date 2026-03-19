use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingSequence {
    None,
    LowerG,
    Fold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    FocusLeft,
    FocusRight,
    CycleFocus,
    MoveUp,
    MoveDown,
    MoveTop,
    MoveBottom,
    Activate,
    ToggleCollapse,
    OpenPane,
    ClosePane,
    OpenAllPanes,
    CloseSecondaryPanes,
    Refresh,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    None,
    Command(Command),
    AppendSearch(char),
    BackspaceSearch,
    SubmitSearch,
    CancelSearch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyResolution {
    pub action: KeyAction,
    pub input_mode: InputMode,
    pub pending: PendingSequence,
}

pub fn interpret_key(mode: InputMode, pending: PendingSequence, key: KeyEvent) -> KeyResolution {
    match mode {
        InputMode::Search => interpret_search_key(key),
        InputMode::Normal => interpret_normal_key(pending, key),
    }
}

fn interpret_search_key(key: KeyEvent) -> KeyResolution {
    match key.code {
        KeyCode::Esc => resolution(KeyAction::CancelSearch, InputMode::Normal),
        KeyCode::Enter => resolution(KeyAction::SubmitSearch, InputMode::Normal),
        KeyCode::Backspace => resolution(KeyAction::BackspaceSearch, InputMode::Search),
        KeyCode::Char(character) => {
            resolution(KeyAction::AppendSearch(character), InputMode::Search)
        }
        _ => resolution(KeyAction::None, InputMode::Search),
    }
}

fn interpret_normal_key(pending: PendingSequence, key: KeyEvent) -> KeyResolution {
    match pending {
        PendingSequence::LowerG => {
            if key.code == KeyCode::Char('g') {
                return resolution(KeyAction::Command(Command::MoveTop), InputMode::Normal);
            }
            interpret_base_normal_key(key)
        }
        PendingSequence::Fold => interpret_fold_key(key),
        PendingSequence::None => interpret_base_normal_key(key),
    }
}

fn interpret_fold_key(key: KeyEvent) -> KeyResolution {
    let action = match key.code {
        KeyCode::Char('a') => KeyAction::Command(Command::ToggleCollapse),
        KeyCode::Char('o') => KeyAction::Command(Command::OpenPane),
        KeyCode::Char('c') => KeyAction::Command(Command::ClosePane),
        KeyCode::Char('R') => KeyAction::Command(Command::OpenAllPanes),
        KeyCode::Char('M') => KeyAction::Command(Command::CloseSecondaryPanes),
        _ => KeyAction::None,
    };
    resolution(action, InputMode::Normal)
}

fn interpret_base_normal_key(key: KeyEvent) -> KeyResolution {
    let action = match key.code {
        KeyCode::Char('q') => KeyAction::Command(Command::Quit),
        KeyCode::Char('h') | KeyCode::Left => KeyAction::Command(Command::FocusLeft),
        KeyCode::Char('l') | KeyCode::Right => KeyAction::Command(Command::FocusRight),
        KeyCode::Char('j') | KeyCode::Down => KeyAction::Command(Command::MoveDown),
        KeyCode::Char('k') | KeyCode::Up => KeyAction::Command(Command::MoveUp),
        KeyCode::Char('G') => KeyAction::Command(Command::MoveBottom),
        KeyCode::Char('g') => KeyAction::None,
        KeyCode::Char('z') => KeyAction::None,
        KeyCode::Char('r') => KeyAction::Command(Command::Refresh),
        KeyCode::Char('/') => KeyAction::None,
        KeyCode::Enter => KeyAction::Command(Command::Activate),
        KeyCode::Tab => KeyAction::Command(Command::CycleFocus),
        _ => KeyAction::None,
    };

    let input_mode = if key.code == KeyCode::Char('/') {
        InputMode::Search
    } else {
        InputMode::Normal
    };
    let pending = match key.code {
        KeyCode::Char('g') => PendingSequence::LowerG,
        KeyCode::Char('z') => PendingSequence::Fold,
        _ => PendingSequence::None,
    };

    KeyResolution {
        action,
        input_mode,
        pending,
    }
}

fn resolution(action: KeyAction, input_mode: InputMode) -> KeyResolution {
    KeyResolution {
        action,
        input_mode,
        pending: PendingSequence::None,
    }
}
