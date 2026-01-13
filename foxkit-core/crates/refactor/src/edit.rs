//! Text edits

use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::Range;

/// A text edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    /// Range to replace
    pub range: (usize, usize), // (start_offset, end_offset)
    /// New text
    pub new_text: String,
}

impl TextEdit {
    /// Create an insertion
    pub fn insert(offset: usize, text: impl Into<String>) -> Self {
        Self {
            range: (offset, offset),
            new_text: text.into(),
        }
    }

    /// Create a deletion
    pub fn delete(start: usize, end: usize) -> Self {
        Self {
            range: (start, end),
            new_text: String::new(),
        }
    }

    /// Create a replacement
    pub fn replace(start: usize, end: usize, text: impl Into<String>) -> Self {
        Self {
            range: (start, end),
            new_text: text.into(),
        }
    }
}

/// Edit for a single file
#[derive(Debug, Clone, Default)]
pub struct FileEdit {
    /// File path
    pub path: PathBuf,
    /// Edits to apply
    pub edits: Vec<TextEdit>,
}

impl FileEdit {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            edits: Vec::new(),
        }
    }

    pub fn add(&mut self, edit: TextEdit) {
        self.edits.push(edit);
    }

    /// Apply edits to source (returns new source)
    pub fn apply(&self, source: &str) -> String {
        let mut result = source.to_string();
        
        // Sort edits by offset (descending) to apply from end to start
        let mut edits = self.edits.clone();
        edits.sort_by_key(|e| std::cmp::Reverse(e.range.0));

        for edit in edits {
            let (start, end) = edit.range;
            if start <= result.len() && end <= result.len() {
                result.replace_range(start..end, &edit.new_text);
            }
        }

        result
    }
}

/// Workspace-wide edit
#[derive(Debug, Clone, Default)]
pub struct WorkspaceEdit {
    /// File edits
    pub changes: HashMap<PathBuf, Vec<TextEdit>>,
    /// File operations (create, delete, rename)
    pub file_operations: Vec<FileOperation>,
}

impl WorkspaceEdit {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add edit for file
    pub fn add_edit(&mut self, file: PathBuf, edit: TextEdit) {
        self.changes.entry(file).or_default().push(edit);
    }

    /// Add file operation
    pub fn add_operation(&mut self, op: FileOperation) {
        self.file_operations.push(op);
    }

    /// Merge another edit
    pub fn merge(&mut self, other: WorkspaceEdit) {
        for (file, edits) in other.changes {
            self.changes.entry(file).or_default().extend(edits);
        }
        self.file_operations.extend(other.file_operations);
    }

    /// Count of affected files
    pub fn file_count(&self) -> usize {
        self.changes.len() + self.file_operations.len()
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty() && self.file_operations.is_empty()
    }
}

/// File operation
#[derive(Debug, Clone)]
pub enum FileOperation {
    Create { path: PathBuf, content: String },
    Delete { path: PathBuf },
    Rename { old_path: PathBuf, new_path: PathBuf },
}
