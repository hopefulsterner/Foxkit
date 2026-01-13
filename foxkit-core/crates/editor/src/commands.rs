//! Editor commands

use crate::Editor;
use anyhow::Result;

/// Command executor
pub struct CommandExecutor;

impl CommandExecutor {
    /// Execute a command on an editor
    pub async fn execute(editor: &mut Editor, command: Command) -> Result<()> {
        match command {
            Command::Insert { text } => {
                editor.insert(&text);
            }
            Command::Delete => {
                editor.backspace();
            }
            Command::Save => {
                editor.save().await?;
            }
            Command::SelectAll => {
                editor.select_all();
            }
            Command::MoveCursor { direction, extend } => {
                editor.move_cursor(direction, extend);
            }
            Command::AddCursor { offset } => {
                editor.add_cursor(offset);
            }
            Command::ClearCursors => {
                editor.clear_secondary_cursors();
            }
        }
        Ok(())
    }
}

/// Editor command
#[derive(Debug, Clone)]
pub enum Command {
    Insert { text: String },
    Delete,
    Save,
    SelectAll,
    MoveCursor { direction: crate::Direction, extend: bool },
    AddCursor { offset: usize },
    ClearCursors,
}
