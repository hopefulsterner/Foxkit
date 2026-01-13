//! Inline refactorings

use std::path::PathBuf;
use async_trait::async_trait;

use crate::{CodeAction, CodeActionKind, CodeActionProvider, WorkspaceEdit, TextEdit};

/// Inline provider trait
#[async_trait]
pub trait InlineProvider: Send + Sync {
    /// Inline a variable
    async fn inline_variable(
        &self,
        file: &PathBuf,
        offset: usize,
        source: &str,
    ) -> anyhow::Result<WorkspaceEdit>;

    /// Inline a function call
    async fn inline_function(
        &self,
        file: &PathBuf,
        offset: usize,
        source: &str,
    ) -> anyhow::Result<WorkspaceEdit>;
}

/// Built-in inline provider
pub struct BuiltinInlineProvider;

#[async_trait]
impl CodeActionProvider for BuiltinInlineProvider {
    async fn provide_actions(
        &self,
        file: &PathBuf,
        start: usize,
        end: usize,
        source: &str,
    ) -> anyhow::Result<Vec<CodeAction>> {
        let mut actions = Vec::new();

        // Check if cursor is on a variable that could be inlined
        if let Some(var_info) = detect_inline_candidate(source, start) {
            match var_info {
                InlineCandidate::Variable(name) => {
                    actions.push(CodeAction::new(
                        format!("Inline variable '{}'", name),
                        CodeActionKind::RefactorInline,
                    ));
                }
                InlineCandidate::Function(name) => {
                    actions.push(CodeAction::new(
                        format!("Inline function '{}'", name),
                        CodeActionKind::RefactorInline,
                    ));
                }
            }
        }

        Ok(actions)
    }

    fn can_handle(&self, kind: &CodeActionKind) -> bool {
        matches!(kind, CodeActionKind::RefactorInline)
    }

    async fn execute(&self, action: &CodeAction) -> anyhow::Result<WorkspaceEdit> {
        Ok(WorkspaceEdit::new())
    }
}

#[async_trait]
impl InlineProvider for BuiltinInlineProvider {
    async fn inline_variable(
        &self,
        file: &PathBuf,
        offset: usize,
        source: &str,
    ) -> anyhow::Result<WorkspaceEdit> {
        let mut edit = WorkspaceEdit::new();

        // Find the variable declaration
        if let Some((name, value, decl_start, decl_end)) = find_variable_declaration(source, offset) {
            // Find all usages of the variable
            let usages = find_identifier_usages(source, &name, decl_end);

            // Replace each usage with the value
            for (usage_start, usage_end) in usages.iter().rev() {
                edit.add_edit(file.clone(), TextEdit::replace(*usage_start, *usage_end, &value));
            }

            // Remove the declaration
            edit.add_edit(file.clone(), TextEdit::delete(decl_start, decl_end));
        }

        Ok(edit)
    }

    async fn inline_function(
        &self,
        file: &PathBuf,
        offset: usize,
        source: &str,
    ) -> anyhow::Result<WorkspaceEdit> {
        // This would require more complex analysis
        // For now, return empty edit
        Ok(WorkspaceEdit::new())
    }
}

/// Inline candidate type
enum InlineCandidate {
    Variable(String),
    Function(String),
}

/// Detect if cursor is on something that can be inlined
fn detect_inline_candidate(source: &str, offset: usize) -> Option<InlineCandidate> {
    // Get the word at offset
    let word = get_word_at(source, offset)?;

    // Check if it's a variable declaration
    let line_start = source[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line = &source[line_start..source[offset..].find('\n').map(|i| offset + i).unwrap_or(source.len())];

    if line.contains("let ") && line.contains(&word) && line.contains(" = ") {
        return Some(InlineCandidate::Variable(word.to_string()));
    }

    // Check if it's a function definition
    if line.starts_with("fn ") && line.contains(&word) {
        return Some(InlineCandidate::Function(word.to_string()));
    }

    None
}

/// Get word at offset
fn get_word_at(source: &str, offset: usize) -> Option<&str> {
    if offset >= source.len() {
        return None;
    }

    let bytes = source.as_bytes();
    if !bytes[offset].is_ascii_alphanumeric() && bytes[offset] != b'_' {
        return None;
    }

    let mut start = offset;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }

    let mut end = offset;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }

    Some(&source[start..end])
}

/// Find variable declaration
fn find_variable_declaration(source: &str, offset: usize) -> Option<(String, String, usize, usize)> {
    // Find the line containing the offset
    let line_start = source[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = source[offset..].find('\n').map(|i| offset + i).unwrap_or(source.len());
    let line = &source[line_start..line_end];

    // Parse "let name = value;"
    if let Some(let_pos) = line.find("let ") {
        let rest = &line[let_pos + 4..];
        if let Some(eq_pos) = rest.find(" = ") {
            let name = rest[..eq_pos].trim();
            let value_start = eq_pos + 3;
            let value_end = rest[value_start..].find(';').unwrap_or(rest.len() - value_start);
            let value = rest[value_start..value_start + value_end].trim();

            return Some((
                name.to_string(),
                value.to_string(),
                line_start,
                line_end + 1, // Include newline
            ));
        }
    }

    None
}

/// Find all usages of an identifier after a given offset
fn find_identifier_usages(source: &str, name: &str, after: usize) -> Vec<(usize, usize)> {
    let mut usages = Vec::new();
    let search = &source[after..];

    let mut pos = 0;
    while let Some(found) = search[pos..].find(name) {
        let abs_pos = after + pos + found;
        
        // Check it's a word boundary
        let before_ok = abs_pos == 0 || {
            let c = source.as_bytes()[abs_pos - 1];
            !c.is_ascii_alphanumeric() && c != b'_'
        };
        
        let after_ok = abs_pos + name.len() >= source.len() || {
            let c = source.as_bytes()[abs_pos + name.len()];
            !c.is_ascii_alphanumeric() && c != b'_'
        };

        if before_ok && after_ok {
            usages.push((abs_pos, abs_pos + name.len()));
        }

        pos += found + name.len();
    }

    usages
}
