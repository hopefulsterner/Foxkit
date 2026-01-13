//! # Foxkit Rename
//!
//! Symbol renaming with preview and validation.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Rename service
pub struct RenameService {
    /// Events
    events: broadcast::Sender<RenameEvent>,
    /// Configuration
    config: RwLock<RenameConfig>,
}

impl RenameService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            events,
            config: RwLock::new(RenameConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<RenameEvent> {
        self.events.subscribe()
    }

    /// Configure rename
    pub fn configure(&self, config: RenameConfig) {
        *self.config.write() = config;
    }

    /// Prepare rename at position
    pub async fn prepare_rename(
        &self,
        file: &PathBuf,
        line: u32,
        column: u32,
    ) -> anyhow::Result<PrepareRenameResult> {
        // Would call LSP prepareRename
        Ok(PrepareRenameResult {
            range: RenameRange::new(line, column, line, column + 5),
            placeholder: "symbol".to_string(),
        })
    }

    /// Execute rename
    pub async fn rename(
        &self,
        file: &PathBuf,
        line: u32,
        column: u32,
        new_name: &str,
    ) -> anyhow::Result<WorkspaceEdit> {
        // Validate name
        self.validate_name(new_name)?;

        // Would call LSP rename
        let _ = self.events.send(RenameEvent::Started {
            file: file.clone(),
            new_name: new_name.to_string(),
        });

        // Placeholder result
        Ok(WorkspaceEdit::new())
    }

    /// Preview rename changes
    pub async fn preview_rename(
        &self,
        file: &PathBuf,
        line: u32,
        column: u32,
        new_name: &str,
    ) -> anyhow::Result<RenamePreview> {
        let edit = self.rename(file, line, column, new_name).await?;
        
        Ok(RenamePreview {
            old_name: "symbol".to_string(),
            new_name: new_name.to_string(),
            edit,
            affected_files: Vec::new(),
        })
    }

    /// Validate rename
    fn validate_name(&self, name: &str) -> anyhow::Result<()> {
        if name.is_empty() {
            anyhow::bail!("Name cannot be empty");
        }

        if name.contains(char::is_whitespace) {
            anyhow::bail!("Name cannot contain whitespace");
        }

        // Check for invalid characters
        let invalid_chars = ['/', '\\', '<', '>', ':', '"', '|', '?', '*'];
        for c in invalid_chars {
            if name.contains(c) {
                anyhow::bail!("Name contains invalid character: {}", c);
            }
        }

        Ok(())
    }
}

impl Default for RenameService {
    fn default() -> Self {
        Self::new()
    }
}

/// Prepare rename result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareRenameResult {
    /// Range of symbol to rename
    pub range: RenameRange,
    /// Placeholder text
    pub placeholder: String,
}

/// Rename range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl RenameRange {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
    }
}

/// Workspace edit
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceEdit {
    /// Text edits by file
    pub changes: HashMap<PathBuf, Vec<TextEdit>>,
    /// Document changes (for rename/create/delete)
    pub document_changes: Vec<DocumentChange>,
}

impl WorkspaceEdit {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_edit(&mut self, file: PathBuf, edit: TextEdit) {
        self.changes.entry(file).or_default().push(edit);
    }

    pub fn file_count(&self) -> usize {
        self.changes.len()
    }

    pub fn edit_count(&self) -> usize {
        self.changes.values().map(|v| v.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty() && self.document_changes.is_empty()
    }
}

/// Text edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    /// Range to replace
    pub range: RenameRange,
    /// New text
    pub new_text: String,
}

impl TextEdit {
    pub fn new(range: RenameRange, new_text: impl Into<String>) -> Self {
        Self { range, new_text: new_text.into() }
    }

    pub fn replace(
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
        new_text: impl Into<String>,
    ) -> Self {
        Self {
            range: RenameRange::new(start_line, start_col, end_line, end_col),
            new_text: new_text.into(),
        }
    }
}

/// Document change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentChange {
    /// Text document edit
    Edit {
        file: PathBuf,
        edits: Vec<TextEdit>,
    },
    /// Create file
    Create {
        path: PathBuf,
        overwrite: bool,
        ignore_if_exists: bool,
    },
    /// Rename file
    Rename {
        old_path: PathBuf,
        new_path: PathBuf,
        overwrite: bool,
        ignore_if_exists: bool,
    },
    /// Delete file
    Delete {
        path: PathBuf,
        recursive: bool,
        ignore_if_not_exists: bool,
    },
}

/// Rename preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenamePreview {
    /// Old name
    pub old_name: String,
    /// New name
    pub new_name: String,
    /// Workspace edit
    pub edit: WorkspaceEdit,
    /// Affected files summary
    pub affected_files: Vec<AffectedFile>,
}

impl RenamePreview {
    pub fn file_count(&self) -> usize {
        self.edit.file_count()
    }

    pub fn edit_count(&self) -> usize {
        self.edit.edit_count()
    }
}

/// Affected file info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedFile {
    /// File path
    pub path: PathBuf,
    /// Number of occurrences
    pub occurrences: usize,
    /// Preview of changes
    pub preview: Vec<ChangePreview>,
}

/// Change preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePreview {
    /// Line number
    pub line: u32,
    /// Line content before
    pub before: String,
    /// Line content after
    pub after: String,
}

/// Rename configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameConfig {
    /// Enable rename preview
    pub enable_preview: bool,
    /// Confirm before rename
    pub confirm: bool,
    /// Enable file renaming
    pub enable_file_rename: bool,
}

impl Default for RenameConfig {
    fn default() -> Self {
        Self {
            enable_preview: true,
            confirm: true,
            enable_file_rename: true,
        }
    }
}

/// Rename event
#[derive(Debug, Clone)]
pub enum RenameEvent {
    Started { file: PathBuf, new_name: String },
    Completed { edit: WorkspaceEdit },
    Failed { error: String },
}

/// Rename input widget state
pub struct RenameInputWidget {
    /// Current value
    value: String,
    /// Placeholder
    placeholder: String,
    /// Position
    position: (u32, u32),
    /// Validation error
    error: Option<String>,
    /// Is visible
    visible: bool,
}

impl RenameInputWidget {
    pub fn new(placeholder: impl Into<String>, position: (u32, u32)) -> Self {
        let placeholder = placeholder.into();
        Self {
            value: placeholder.clone(),
            placeholder,
            position,
            error: None,
            visible: true,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.error = None;
    }

    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn position(&self) -> (u32, u32) {
        self.position
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn is_valid(&self) -> bool {
        self.error.is_none() && !self.value.is_empty()
    }

    pub fn select_all(&mut self) {
        // Would select all text
    }
}
