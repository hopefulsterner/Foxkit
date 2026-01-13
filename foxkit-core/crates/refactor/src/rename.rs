//! Rename refactoring

use std::path::PathBuf;
use std::collections::HashMap;
use async_trait::async_trait;

use crate::{CodeAction, CodeActionKind, CodeActionProvider, WorkspaceEdit, TextEdit};

/// Rename provider trait
#[async_trait]
pub trait RenameProvider: Send + Sync {
    /// Prepare rename (validate and get range)
    async fn prepare(&self, file: &PathBuf, offset: usize, source: &str) -> Option<RenameResult>;

    /// Perform rename
    async fn rename(
        &self,
        file: &PathBuf,
        offset: usize,
        new_name: &str,
        source: &str,
    ) -> anyhow::Result<WorkspaceEdit>;
}

/// Rename result
#[derive(Debug, Clone)]
pub struct RenameResult {
    /// Current name range
    pub range: (usize, usize),
    /// Placeholder text
    pub placeholder: String,
}

/// Built-in rename provider
pub struct BuiltinRenameProvider;

#[async_trait]
impl CodeActionProvider for BuiltinRenameProvider {
    async fn provide_actions(
        &self,
        file: &PathBuf,
        start: usize,
        end: usize,
        source: &str,
    ) -> anyhow::Result<Vec<CodeAction>> {
        // Check if we're on an identifier
        if let Some(word) = get_word_at(source, start) {
            let action = CodeAction::new(
                format!("Rename '{}'", word),
                CodeActionKind::Refactor,
            );
            return Ok(vec![action]);
        }
        Ok(vec![])
    }

    fn can_handle(&self, kind: &CodeActionKind) -> bool {
        matches!(kind, CodeActionKind::Refactor)
    }

    async fn execute(&self, action: &CodeAction) -> anyhow::Result<WorkspaceEdit> {
        // This would be implemented with LSP rename request
        Ok(WorkspaceEdit::new())
    }
}

#[async_trait]
impl RenameProvider for BuiltinRenameProvider {
    async fn prepare(&self, _file: &PathBuf, offset: usize, source: &str) -> Option<RenameResult> {
        let (start, end, word) = get_word_range_at(source, offset)?;
        
        Some(RenameResult {
            range: (start, end),
            placeholder: word.to_string(),
        })
    }

    async fn rename(
        &self,
        file: &PathBuf,
        offset: usize,
        new_name: &str,
        source: &str,
    ) -> anyhow::Result<WorkspaceEdit> {
        let mut edit = WorkspaceEdit::new();

        // Simple local rename - just replace current occurrence
        // Real implementation would use LSP or treesitter for multi-file rename
        if let Some((start, end, _)) = get_word_range_at(source, offset) {
            edit.add_edit(file.clone(), TextEdit::replace(start, end, new_name));
        }

        Ok(edit)
    }
}

/// Get word at offset
fn get_word_at(source: &str, offset: usize) -> Option<&str> {
    get_word_range_at(source, offset).map(|(_, _, w)| w)
}

/// Get word range at offset
fn get_word_range_at(source: &str, offset: usize) -> Option<(usize, usize, &str)> {
    if offset >= source.len() {
        return None;
    }

    let bytes = source.as_bytes();
    
    // Check if we're in an identifier character
    if !is_identifier_char(bytes[offset]) {
        return None;
    }

    // Find start
    let mut start = offset;
    while start > 0 && is_identifier_char(bytes[start - 1]) {
        start -= 1;
    }

    // Find end
    let mut end = offset;
    while end < bytes.len() && is_identifier_char(bytes[end]) {
        end += 1;
    }

    Some((start, end, &source[start..end]))
}

fn is_identifier_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}
