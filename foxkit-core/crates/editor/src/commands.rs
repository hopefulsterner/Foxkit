//! Editor commands - comprehensive editing operations
//!
//! This module provides all the commands that can be executed on an editor,
//! from basic text manipulation to advanced multi-cursor operations.

use crate::{Editor, Direction, EditorMode};
use anyhow::Result;

/// Command executor - dispatches commands to editor
pub struct CommandExecutor;

impl CommandExecutor {
    /// Execute a command on an editor
    pub async fn execute(editor: &mut Editor, command: EditorCommand) -> Result<()> {
        match command {
            // === Text Input ===
            EditorCommand::Insert { text } => {
                editor.insert(&text);
            }
            EditorCommand::InsertNewline => {
                editor.insert("\n");
            }
            EditorCommand::InsertTab => {
                editor.insert_tab();
            }
            
            // === Deletion ===
            EditorCommand::Backspace => {
                editor.backspace();
            }
            EditorCommand::Delete => {
                editor.delete_forward();
            }
            EditorCommand::DeleteLine => {
                editor.delete_line();
            }
            EditorCommand::DeleteWord => {
                editor.delete_word();
            }
            EditorCommand::DeleteToLineEnd => {
                editor.delete_to_line_end();
            }
            EditorCommand::DeleteToLineStart => {
                editor.delete_to_line_start();
            }
            
            // === Cursor Movement ===
            EditorCommand::MoveCursor { direction, extend } => {
                editor.move_cursor(direction, extend);
            }
            EditorCommand::MoveWord { direction, extend } => {
                editor.move_word(direction, extend);
            }
            EditorCommand::MoveToLineStart { extend } => {
                editor.move_to_line_start(extend);
            }
            EditorCommand::MoveToLineEnd { extend } => {
                editor.move_to_line_end(extend);
            }
            EditorCommand::MoveToDocumentStart { extend } => {
                editor.move_to_document_start(extend);
            }
            EditorCommand::MoveToDocumentEnd { extend } => {
                editor.move_to_document_end(extend);
            }
            EditorCommand::PageUp { extend } => {
                editor.page_up(extend);
            }
            EditorCommand::PageDown { extend } => {
                editor.page_down(extend);
            }
            
            // === Selection ===
            EditorCommand::SelectAll => {
                editor.select_all();
            }
            EditorCommand::SelectLine => {
                editor.select_line();
            }
            EditorCommand::SelectWord => {
                editor.select_word();
            }
            EditorCommand::ExpandSelection => {
                editor.expand_selection();
            }
            EditorCommand::ShrinkSelection => {
                editor.shrink_selection();
            }
            
            // === Multi-cursor ===
            EditorCommand::AddCursor { offset } => {
                editor.add_cursor(offset);
            }
            EditorCommand::AddCursorAbove => {
                editor.add_cursor_above();
            }
            EditorCommand::AddCursorBelow => {
                editor.add_cursor_below();
            }
            EditorCommand::AddCursorsToLineEnds => {
                editor.add_cursors_to_line_ends();
            }
            EditorCommand::ClearSecondary => {
                editor.clear_secondary_cursors();
            }
            
            // === Clipboard ===
            EditorCommand::Copy => {
                editor.copy();
            }
            EditorCommand::Cut => {
                editor.cut();
            }
            EditorCommand::Paste { text } => {
                editor.paste(&text);
            }
            
            // === Undo/Redo ===
            EditorCommand::Undo => {
                editor.undo();
            }
            EditorCommand::Redo => {
                editor.redo();
            }
            
            // === Line Operations ===
            EditorCommand::DuplicateLine => {
                editor.duplicate_line();
            }
            EditorCommand::MoveLineUp => {
                editor.move_line_up();
            }
            EditorCommand::MoveLineDown => {
                editor.move_line_down();
            }
            EditorCommand::JoinLines => {
                editor.join_lines();
            }
            EditorCommand::ToggleComment => {
                editor.toggle_comment();
            }
            
            // === Indentation ===
            EditorCommand::Indent => {
                editor.indent();
            }
            EditorCommand::Outdent => {
                editor.outdent();
            }
            
            // === Case Transforms ===
            EditorCommand::UpperCase => {
                editor.transform_case(CaseTransform::Upper);
            }
            EditorCommand::LowerCase => {
                editor.transform_case(CaseTransform::Lower);
            }
            EditorCommand::TitleCase => {
                editor.transform_case(CaseTransform::Title);
            }
            
            // === Find/Replace ===
            EditorCommand::Find { query } => {
                editor.find(&query);
            }
            EditorCommand::FindNext => {
                editor.find_next();
            }
            EditorCommand::FindPrevious => {
                editor.find_previous();
            }
            EditorCommand::Replace { replacement } => {
                editor.replace(&replacement);
            }
            EditorCommand::ReplaceAll { replacement } => {
                editor.replace_all(&replacement);
            }
            
            // === File Operations ===
            EditorCommand::Save => {
                editor.save().await?;
            }
            EditorCommand::SaveAs { path } => {
                editor.save_as(&path).await?;
            }
            
            // === View ===
            EditorCommand::CenterCursor => {
                editor.center_cursor();
            }
            EditorCommand::ScrollUp { lines } => {
                editor.scroll_up(lines);
            }
            EditorCommand::ScrollDown { lines } => {
                editor.scroll_down(lines);
            }
            
            // === Mode ===
            EditorCommand::SetMode { mode } => {
                editor.set_mode(mode);
            }
        }
        Ok(())
    }
}

/// All editor commands
#[derive(Debug, Clone)]
pub enum EditorCommand {
    // Text Input
    Insert { text: String },
    InsertNewline,
    InsertTab,
    
    // Deletion
    Backspace,
    Delete,
    DeleteLine,
    DeleteWord,
    DeleteToLineEnd,
    DeleteToLineStart,
    
    // Cursor Movement
    MoveCursor { direction: Direction, extend: bool },
    MoveWord { direction: Direction, extend: bool },
    MoveToLineStart { extend: bool },
    MoveToLineEnd { extend: bool },
    MoveToDocumentStart { extend: bool },
    MoveToDocumentEnd { extend: bool },
    PageUp { extend: bool },
    PageDown { extend: bool },
    
    // Selection
    SelectAll,
    SelectLine,
    SelectWord,
    ExpandSelection,
    ShrinkSelection,
    
    // Multi-cursor
    AddCursor { offset: usize },
    AddCursorAbove,
    AddCursorBelow,
    AddCursorsToLineEnds,
    ClearSecondary,
    
    // Clipboard
    Copy,
    Cut,
    Paste { text: String },
    
    // Undo/Redo
    Undo,
    Redo,
    
    // Line Operations
    DuplicateLine,
    MoveLineUp,
    MoveLineDown,
    JoinLines,
    ToggleComment,
    
    // Indentation
    Indent,
    Outdent,
    
    // Case Transforms
    UpperCase,
    LowerCase,
    TitleCase,
    
    // Find/Replace
    Find { query: String },
    FindNext,
    FindPrevious,
    Replace { replacement: String },
    ReplaceAll { replacement: String },
    
    // File Operations
    Save,
    SaveAs { path: String },
    
    // View
    CenterCursor,
    ScrollUp { lines: usize },
    ScrollDown { lines: usize },
    
    // Mode
    SetMode { mode: EditorMode },
}

/// Case transformation type
#[derive(Debug, Clone, Copy)]
pub enum CaseTransform {
    Upper,
    Lower,
    Title,
    Snake,
    Camel,
    Pascal,
    Kebab,
}

/// Command for command palette
#[derive(Debug, Clone)]
pub struct CommandPaletteItem {
    pub id: &'static str,
    pub label: &'static str,
    pub keybinding: Option<&'static str>,
    pub category: CommandCategory,
}

/// Command categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    File,
    Edit,
    Selection,
    View,
    Go,
    Search,
    Debug,
    Terminal,
}

/// Built-in command palette items
pub const COMMAND_PALETTE: &[CommandPaletteItem] = &[
    CommandPaletteItem {
        id: "file.save",
        label: "File: Save",
        keybinding: Some("Ctrl+S"),
        category: CommandCategory::File,
    },
    CommandPaletteItem {
        id: "file.saveAs",
        label: "File: Save As...",
        keybinding: Some("Ctrl+Shift+S"),
        category: CommandCategory::File,
    },
    CommandPaletteItem {
        id: "edit.undo",
        label: "Edit: Undo",
        keybinding: Some("Ctrl+Z"),
        category: CommandCategory::Edit,
    },
    CommandPaletteItem {
        id: "edit.redo",
        label: "Edit: Redo",
        keybinding: Some("Ctrl+Shift+Z"),
        category: CommandCategory::Edit,
    },
    CommandPaletteItem {
        id: "edit.cut",
        label: "Edit: Cut",
        keybinding: Some("Ctrl+X"),
        category: CommandCategory::Edit,
    },
    CommandPaletteItem {
        id: "edit.copy",
        label: "Edit: Copy",
        keybinding: Some("Ctrl+C"),
        category: CommandCategory::Edit,
    },
    CommandPaletteItem {
        id: "edit.paste",
        label: "Edit: Paste",
        keybinding: Some("Ctrl+V"),
        category: CommandCategory::Edit,
    },
    CommandPaletteItem {
        id: "selection.selectAll",
        label: "Selection: Select All",
        keybinding: Some("Ctrl+A"),
        category: CommandCategory::Selection,
    },
    CommandPaletteItem {
        id: "editor.addCursorAbove",
        label: "Add Cursor Above",
        keybinding: Some("Ctrl+Alt+Up"),
        category: CommandCategory::Selection,
    },
    CommandPaletteItem {
        id: "editor.addCursorBelow",
        label: "Add Cursor Below",
        keybinding: Some("Ctrl+Alt+Down"),
        category: CommandCategory::Selection,
    },
];
