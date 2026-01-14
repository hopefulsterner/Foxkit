//! Input handling

use crate::{Editor, Direction, EditorMode};

/// Key event
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
}

/// Key codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Tab,
    Backspace,
    Delete,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
}

/// Modifier keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool, // Cmd on Mac, Win on Windows
}

impl Modifiers {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn ctrl() -> Self {
        Self { ctrl: true, ..Default::default() }
    }

    pub fn shift() -> Self {
        Self { shift: true, ..Default::default() }
    }

    pub fn alt() -> Self {
        Self { alt: true, ..Default::default() }
    }

    pub fn ctrl_shift() -> Self {
        Self { ctrl: true, shift: true, ..Default::default() }
    }
}

/// Input handler - processes key events
pub struct InputHandler;

impl InputHandler {
    /// Handle a key event for an editor
    pub fn handle(editor: &mut Editor, event: KeyEvent) -> InputResult {
        match editor.mode() {
            EditorMode::Insert => Self::handle_insert_mode(editor, event),
            EditorMode::Normal => Self::handle_normal_mode(editor, event),
            EditorMode::Visual | EditorMode::VisualLine | EditorMode::VisualBlock => {
                Self::handle_visual_mode(editor, event)
            }
            EditorMode::Replace => Self::handle_replace_mode(editor, event),
        }
    }

    fn handle_insert_mode(editor: &mut Editor, event: KeyEvent) -> InputResult {
        match (&event.key, &event.modifiers) {
            // Escape -> Normal mode
            (Key::Escape, _) => {
                editor.set_mode(EditorMode::Normal);
                InputResult::Handled
            }
            
            // Basic text input
            (Key::Char(c), m) if !m.ctrl && !m.alt && !m.meta => {
                editor.insert(&c.to_string());
                InputResult::Handled
            }
            
            // Enter
            (Key::Enter, _) => {
                editor.insert("\n");
                InputResult::Handled
            }
            
            // Tab
            (Key::Tab, _) => {
                editor.insert("    "); // or \t based on settings
                InputResult::Handled
            }
            
            // Backspace
            (Key::Backspace, _) => {
                editor.backspace();
                InputResult::Handled
            }
            
            // Arrow keys
            (Key::Left, m) => {
                editor.move_cursor(Direction::Left, m.shift);
                InputResult::Handled
            }
            (Key::Right, m) => {
                editor.move_cursor(Direction::Right, m.shift);
                InputResult::Handled
            }
            (Key::Up, m) => {
                editor.move_cursor(Direction::Up, m.shift);
                InputResult::Handled
            }
            (Key::Down, m) => {
                editor.move_cursor(Direction::Down, m.shift);
                InputResult::Handled
            }
            
            // Ctrl+S - Save
            (Key::Char('s'), m) if m.ctrl => {
                InputResult::Command(EditorCommand::Save)
            }
            
            // Ctrl+A - Select all
            (Key::Char('a'), m) if m.ctrl => {
                editor.select_all();
                InputResult::Handled
            }
            
            _ => InputResult::Unhandled,
        }
    }

    fn handle_normal_mode(editor: &mut Editor, event: KeyEvent) -> InputResult {
        match (&event.key, &event.modifiers) {
            // i -> Insert mode
            (Key::Char('i'), _) => {
                editor.set_mode(EditorMode::Insert);
                InputResult::Handled
            }
            
            // a -> Insert after cursor
            (Key::Char('a'), _) => {
                editor.move_cursor(Direction::Right, false);
                editor.set_mode(EditorMode::Insert);
                InputResult::Handled
            }
            
            // v -> Visual mode
            (Key::Char('v'), m) if !m.shift => {
                editor.set_mode(EditorMode::Visual);
                InputResult::Handled
            }
            
            // V -> Visual line mode
            (Key::Char('V'), m) | (Key::Char('v'), m) if m.shift => {
                editor.set_mode(EditorMode::VisualLine);
                InputResult::Handled
            }
            
            // Movement: h, j, k, l
            (Key::Char('h'), _) | (Key::Left, _) => {
                editor.move_cursor(Direction::Left, false);
                InputResult::Handled
            }
            (Key::Char('l'), _) | (Key::Right, _) => {
                editor.move_cursor(Direction::Right, false);
                InputResult::Handled
            }
            (Key::Char('j'), _) | (Key::Down, _) => {
                editor.move_cursor(Direction::Down, false);
                InputResult::Handled
            }
            (Key::Char('k'), _) | (Key::Up, _) => {
                editor.move_cursor(Direction::Up, false);
                InputResult::Handled
            }
            
            // : -> Command mode (not fully implemented)
            (Key::Char(':'), _) => {
                InputResult::Command(EditorCommand::OpenCommandPalette)
            }
            
            // / -> Search
            (Key::Char('/'), _) => {
                InputResult::Command(EditorCommand::StartSearch)
            }
            
            _ => InputResult::Unhandled,
        }
    }

    fn handle_visual_mode(editor: &mut Editor, event: KeyEvent) -> InputResult {
        match (&event.key, &event.modifiers) {
            // Escape -> Normal mode
            (Key::Escape, _) => {
                editor.set_mode(EditorMode::Normal);
                editor.selections_mut().primary_mut().collapse();
                InputResult::Handled
            }
            
            // Movement with selection extension
            (Key::Char('h'), _) | (Key::Left, _) => {
                editor.move_cursor(Direction::Left, true);
                InputResult::Handled
            }
            (Key::Char('l'), _) | (Key::Right, _) => {
                editor.move_cursor(Direction::Right, true);
                InputResult::Handled
            }
            (Key::Char('j'), _) | (Key::Down, _) => {
                editor.move_cursor(Direction::Down, true);
                InputResult::Handled
            }
            (Key::Char('k'), _) | (Key::Up, _) => {
                editor.move_cursor(Direction::Up, true);
                InputResult::Handled
            }
            
            _ => InputResult::Unhandled,
        }
    }

    fn handle_replace_mode(editor: &mut Editor, event: KeyEvent) -> InputResult {
        match &event.key {
            Key::Escape => {
                editor.set_mode(EditorMode::Normal);
                InputResult::Handled
            }
            Key::Char(c) => {
                // Replace character under cursor
                editor.backspace();
                editor.insert(&c.to_string());
                editor.set_mode(EditorMode::Normal);
                InputResult::Handled
            }
            _ => InputResult::Unhandled,
        }
    }
}

/// Result of input handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputResult {
    /// Input was handled
    Handled,
    /// Input was not handled
    Unhandled,
    /// Input triggered a command
    Command(EditorCommand),
}

/// High-level editor commands
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorCommand {
    Save,
    SaveAs,
    Close,
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    Find,
    Replace,
    GoToLine,
    OpenCommandPalette,
    StartSearch,
}
