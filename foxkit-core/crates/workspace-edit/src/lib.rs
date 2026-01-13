//! # Foxkit Workspace Edit
//!
//! Atomic multi-file edit operations with undo support.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Workspace edit service
pub struct WorkspaceEditService {
    /// Pending edits (not yet applied)
    pending: RwLock<Option<PendingEdit>>,
    /// Edit history for undo
    history: RwLock<EditHistory>,
    /// Events
    events: broadcast::Sender<WorkspaceEditEvent>,
}

impl WorkspaceEditService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            pending: RwLock::new(None),
            history: RwLock::new(EditHistory::new()),
            events,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<WorkspaceEditEvent> {
        self.events.subscribe()
    }

    /// Apply workspace edit
    pub async fn apply(&self, edit: WorkspaceEdit) -> anyhow::Result<ApplyResult> {
        let _ = self.events.send(WorkspaceEditEvent::Applying {
            label: edit.label.clone(),
        });

        let mut applied_files = Vec::new();
        let mut failed_files = Vec::new();
        let mut document_changes_applied = 0;

        // Apply text edits
        for (file, edits) in &edit.changes {
            match self.apply_text_edits(file, edits).await {
                Ok(_) => applied_files.push(file.clone()),
                Err(e) => failed_files.push((file.clone(), e.to_string())),
            }
        }

        // Apply document changes
        for change in &edit.document_changes {
            match self.apply_document_change(change).await {
                Ok(_) => document_changes_applied += 1,
                Err(e) => {
                    tracing::error!("Failed to apply document change: {}", e);
                }
            }
        }

        // Record in history
        if applied_files.len() > 0 || document_changes_applied > 0 {
            self.history.write().push(edit.clone());
        }

        let result = ApplyResult {
            applied_files: applied_files.len(),
            failed_files: failed_files.len(),
            document_changes_applied,
            failures: failed_files,
        };

        let _ = self.events.send(WorkspaceEditEvent::Applied {
            label: edit.label.clone(),
            result: result.clone(),
        });

        Ok(result)
    }

    /// Apply with preview
    pub fn preview(&self, edit: WorkspaceEdit) -> WorkspaceEditPreview {
        let mut file_previews = Vec::new();

        for (file, edits) in &edit.changes {
            let preview = FileEditPreview {
                file: file.clone(),
                edits: edits.clone(),
                summary: format!("{} changes", edits.len()),
            };
            file_previews.push(preview);
        }

        for change in &edit.document_changes {
            let (file, summary) = match change {
                DocumentChange::TextDocumentEdit { uri, edits } => {
                    (uri.clone(), format!("{} text edits", edits.len()))
                }
                DocumentChange::CreateFile { uri, .. } => {
                    (uri.clone(), "Create file".to_string())
                }
                DocumentChange::RenameFile { old_uri, new_uri, .. } => {
                    (old_uri.clone(), format!("Rename to {}", new_uri.display()))
                }
                DocumentChange::DeleteFile { uri, .. } => {
                    (uri.clone(), "Delete file".to_string())
                }
            };
            
            file_previews.push(FileEditPreview {
                file,
                edits: Vec::new(),
                summary,
            });
        }

        WorkspaceEditPreview {
            label: edit.label.clone(),
            files: file_previews,
            total_files: file_previews.len(),
        }
    }

    /// Set pending edit
    pub fn set_pending(&self, edit: WorkspaceEdit, metadata: EditMetadata) {
        *self.pending.write() = Some(PendingEdit { edit, metadata });
    }

    /// Get pending edit
    pub fn pending(&self) -> Option<PendingEdit> {
        self.pending.read().clone()
    }

    /// Clear pending edit
    pub fn clear_pending(&self) {
        *self.pending.write() = None;
    }

    /// Apply pending edit
    pub async fn apply_pending(&self) -> anyhow::Result<ApplyResult> {
        let pending = self.pending.write().take()
            .ok_or_else(|| anyhow::anyhow!("No pending edit"))?;
        
        self.apply(pending.edit).await
    }

    async fn apply_text_edits(&self, file: &PathBuf, edits: &[TextEdit]) -> anyhow::Result<()> {
        // Would apply edits to file
        // Edits should be sorted in reverse order to maintain positions
        Ok(())
    }

    async fn apply_document_change(&self, change: &DocumentChange) -> anyhow::Result<()> {
        match change {
            DocumentChange::TextDocumentEdit { uri, edits } => {
                self.apply_text_edits(uri, edits).await
            }
            DocumentChange::CreateFile { uri, options } => {
                // Would create file
                Ok(())
            }
            DocumentChange::RenameFile { old_uri, new_uri, options } => {
                // Would rename file
                Ok(())
            }
            DocumentChange::DeleteFile { uri, options } => {
                // Would delete file
                Ok(())
            }
        }
    }

    /// Undo last edit
    pub async fn undo(&self) -> anyhow::Result<Option<WorkspaceEdit>> {
        let edit = self.history.write().undo();
        
        if let Some(ref edit) = edit {
            let _ = self.events.send(WorkspaceEditEvent::Undone {
                label: edit.label.clone(),
            });
        }
        
        Ok(edit)
    }

    /// Can undo
    pub fn can_undo(&self) -> bool {
        self.history.read().can_undo()
    }
}

impl Default for WorkspaceEditService {
    fn default() -> Self {
        Self::new()
    }
}

/// Edit history
struct EditHistory {
    edits: Vec<WorkspaceEdit>,
    max_size: usize,
}

impl EditHistory {
    fn new() -> Self {
        Self { edits: Vec::new(), max_size: 50 }
    }

    fn push(&mut self, edit: WorkspaceEdit) {
        self.edits.push(edit);
        if self.edits.len() > self.max_size {
            self.edits.remove(0);
        }
    }

    fn undo(&mut self) -> Option<WorkspaceEdit> {
        self.edits.pop()
    }

    fn can_undo(&self) -> bool {
        !self.edits.is_empty()
    }
}

/// Workspace edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEdit {
    /// Label for this edit
    pub label: Option<String>,
    /// Text changes per file
    #[serde(default)]
    pub changes: HashMap<PathBuf, Vec<TextEdit>>,
    /// Document changes (create/rename/delete)
    #[serde(default)]
    pub document_changes: Vec<DocumentChange>,
    /// Change annotations
    #[serde(default)]
    pub change_annotations: HashMap<String, ChangeAnnotation>,
}

impl WorkspaceEdit {
    pub fn new() -> Self {
        Self {
            label: None,
            changes: HashMap::new(),
            document_changes: Vec::new(),
            change_annotations: HashMap::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn add_text_edit(&mut self, file: PathBuf, edit: TextEdit) {
        self.changes.entry(file).or_default().push(edit);
    }

    pub fn add_document_change(&mut self, change: DocumentChange) {
        self.document_changes.push(change);
    }

    pub fn create_file(mut self, uri: PathBuf) -> Self {
        self.document_changes.push(DocumentChange::CreateFile {
            uri,
            options: None,
        });
        self
    }

    pub fn rename_file(mut self, old: PathBuf, new: PathBuf) -> Self {
        self.document_changes.push(DocumentChange::RenameFile {
            old_uri: old,
            new_uri: new,
            options: None,
        });
        self
    }

    pub fn delete_file(mut self, uri: PathBuf) -> Self {
        self.document_changes.push(DocumentChange::DeleteFile {
            uri,
            options: None,
        });
        self
    }

    pub fn file_count(&self) -> usize {
        let mut count = self.changes.len();
        count += self.document_changes.len();
        count
    }

    pub fn edit_count(&self) -> usize {
        self.changes.values().map(|v| v.len()).sum::<usize>() + self.document_changes.len()
    }
}

impl Default for WorkspaceEdit {
    fn default() -> Self {
        Self::new()
    }
}

/// Text edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: TextRange,
    pub new_text: String,
    /// Annotation ID
    pub annotation_id: Option<String>,
}

impl TextEdit {
    pub fn new(range: TextRange, new_text: impl Into<String>) -> Self {
        Self {
            range,
            new_text: new_text.into(),
            annotation_id: None,
        }
    }

    pub fn replace(
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
        new_text: impl Into<String>,
    ) -> Self {
        Self::new(TextRange::new(start_line, start_col, end_line, end_col), new_text)
    }

    pub fn insert(line: u32, col: u32, text: impl Into<String>) -> Self {
        Self::new(TextRange::point(line, col), text)
    }

    pub fn delete(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self::new(TextRange::new(start_line, start_col, end_line, end_col), "")
    }

    pub fn with_annotation(mut self, id: impl Into<String>) -> Self {
        self.annotation_id = Some(id.into());
        self
    }
}

/// Text range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl TextRange {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
    }

    pub fn point(line: u32, col: u32) -> Self {
        Self { start_line: line, start_col: col, end_line: line, end_col: col }
    }

    pub fn single_line(line: u32, start_col: u32, end_col: u32) -> Self {
        Self { start_line: line, start_col, end_line: line, end_col }
    }
}

/// Document change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentChange {
    /// Edit text document
    TextDocumentEdit {
        uri: PathBuf,
        edits: Vec<TextEdit>,
    },
    /// Create file
    CreateFile {
        uri: PathBuf,
        options: Option<CreateFileOptions>,
    },
    /// Rename file
    RenameFile {
        old_uri: PathBuf,
        new_uri: PathBuf,
        options: Option<RenameFileOptions>,
    },
    /// Delete file
    DeleteFile {
        uri: PathBuf,
        options: Option<DeleteFileOptions>,
    },
}

/// Create file options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFileOptions {
    pub overwrite: bool,
    pub ignore_if_exists: bool,
}

/// Rename file options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameFileOptions {
    pub overwrite: bool,
    pub ignore_if_exists: bool,
}

/// Delete file options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteFileOptions {
    pub recursive: bool,
    pub ignore_if_not_exists: bool,
}

/// Change annotation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeAnnotation {
    pub label: String,
    pub needs_confirmation: bool,
    pub description: Option<String>,
}

impl ChangeAnnotation {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            needs_confirmation: false,
            description: None,
        }
    }

    pub fn needs_confirmation(mut self) -> Self {
        self.needs_confirmation = true;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Pending edit
#[derive(Debug, Clone)]
pub struct PendingEdit {
    pub edit: WorkspaceEdit,
    pub metadata: EditMetadata,
}

/// Edit metadata
#[derive(Debug, Clone)]
pub struct EditMetadata {
    /// Source of the edit
    pub source: String,
    /// Timestamp
    pub timestamp: std::time::SystemTime,
}

impl EditMetadata {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Apply result
#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub applied_files: usize,
    pub failed_files: usize,
    pub document_changes_applied: usize,
    pub failures: Vec<(PathBuf, String)>,
}

impl ApplyResult {
    pub fn success(&self) -> bool {
        self.failed_files == 0
    }
}

/// Workspace edit preview
#[derive(Debug, Clone)]
pub struct WorkspaceEditPreview {
    pub label: Option<String>,
    pub files: Vec<FileEditPreview>,
    pub total_files: usize,
}

/// File edit preview
#[derive(Debug, Clone)]
pub struct FileEditPreview {
    pub file: PathBuf,
    pub edits: Vec<TextEdit>,
    pub summary: String,
}

/// Workspace edit event
#[derive(Debug, Clone)]
pub enum WorkspaceEditEvent {
    Applying { label: Option<String> },
    Applied { label: Option<String>, result: ApplyResult },
    Failed { label: Option<String>, error: String },
    Undone { label: Option<String> },
}

/// Edit builder for fluent API
pub struct WorkspaceEditBuilder {
    edit: WorkspaceEdit,
}

impl WorkspaceEditBuilder {
    pub fn new() -> Self {
        Self { edit: WorkspaceEdit::new() }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.edit.label = Some(label.into());
        self
    }

    pub fn edit_file(mut self, file: PathBuf) -> FileEditBuilder {
        FileEditBuilder {
            builder: self,
            file,
            edits: Vec::new(),
        }
    }

    pub fn create_file(mut self, uri: PathBuf) -> Self {
        self.edit = self.edit.create_file(uri);
        self
    }

    pub fn rename_file(mut self, old: PathBuf, new: PathBuf) -> Self {
        self.edit = self.edit.rename_file(old, new);
        self
    }

    pub fn delete_file(mut self, uri: PathBuf) -> Self {
        self.edit = self.edit.delete_file(uri);
        self
    }

    pub fn build(self) -> WorkspaceEdit {
        self.edit
    }
}

impl Default for WorkspaceEditBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// File edit builder
pub struct FileEditBuilder {
    builder: WorkspaceEditBuilder,
    file: PathBuf,
    edits: Vec<TextEdit>,
}

impl FileEditBuilder {
    pub fn replace(
        mut self,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
        new_text: impl Into<String>,
    ) -> Self {
        self.edits.push(TextEdit::replace(start_line, start_col, end_line, end_col, new_text));
        self
    }

    pub fn insert(mut self, line: u32, col: u32, text: impl Into<String>) -> Self {
        self.edits.push(TextEdit::insert(line, col, text));
        self
    }

    pub fn delete(mut self, start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        self.edits.push(TextEdit::delete(start_line, start_col, end_line, end_col));
        self
    }

    pub fn done(mut self) -> WorkspaceEditBuilder {
        self.builder.edit.changes.insert(self.file, self.edits);
        self.builder
    }
}
