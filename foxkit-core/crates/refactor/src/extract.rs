//! Extract refactorings

use std::path::PathBuf;
use async_trait::async_trait;

use crate::{CodeAction, CodeActionKind, CodeActionProvider, WorkspaceEdit, TextEdit};

/// Extract provider trait
#[async_trait]
pub trait ExtractProvider: Send + Sync {
    /// Extract selection to function
    async fn extract_function(
        &self,
        file: &PathBuf,
        start: usize,
        end: usize,
        name: &str,
        source: &str,
    ) -> anyhow::Result<ExtractResult>;

    /// Extract selection to variable
    async fn extract_variable(
        &self,
        file: &PathBuf,
        start: usize,
        end: usize,
        name: &str,
        source: &str,
    ) -> anyhow::Result<ExtractResult>;

    /// Extract selection to constant
    async fn extract_constant(
        &self,
        file: &PathBuf,
        start: usize,
        end: usize,
        name: &str,
        source: &str,
    ) -> anyhow::Result<ExtractResult>;
}

/// Extract result
#[derive(Debug, Clone)]
pub struct ExtractResult {
    /// Edit to apply
    pub edit: WorkspaceEdit,
    /// Extracted symbol name
    pub name: String,
}

/// Built-in extract provider
pub struct BuiltinExtractProvider;

#[async_trait]
impl CodeActionProvider for BuiltinExtractProvider {
    async fn provide_actions(
        &self,
        file: &PathBuf,
        start: usize,
        end: usize,
        source: &str,
    ) -> anyhow::Result<Vec<CodeAction>> {
        let mut actions = Vec::new();

        // Only show extract actions if there's a selection
        if start < end {
            let selected = &source[start..end.min(source.len())];
            
            // Don't offer for whitespace-only or very short selections
            if selected.trim().len() > 2 {
                actions.push(CodeAction::new(
                    "Extract to function",
                    CodeActionKind::RefactorExtract,
                ));

                actions.push(CodeAction::new(
                    "Extract to variable",
                    CodeActionKind::RefactorExtract,
                ));

                actions.push(CodeAction::new(
                    "Extract to constant",
                    CodeActionKind::RefactorExtract,
                ));
            }
        }

        Ok(actions)
    }

    fn can_handle(&self, kind: &CodeActionKind) -> bool {
        matches!(kind, CodeActionKind::RefactorExtract)
    }

    async fn execute(&self, action: &CodeAction) -> anyhow::Result<WorkspaceEdit> {
        // Would need additional context to implement
        Ok(WorkspaceEdit::new())
    }
}

#[async_trait]
impl ExtractProvider for BuiltinExtractProvider {
    async fn extract_function(
        &self,
        file: &PathBuf,
        start: usize,
        end: usize,
        name: &str,
        source: &str,
    ) -> anyhow::Result<ExtractResult> {
        let selected = &source[start..end.min(source.len())];
        let mut edit = WorkspaceEdit::new();

        // Find insertion point (before current function or at file level)
        // This is a simplified implementation
        let insert_point = find_function_insert_point(source, start);
        
        // Detect language and generate appropriate function
        let func_def = format!(
            "\nfn {}() {{\n    {}\n}}\n",
            name,
            selected.trim()
        );

        // Insert function definition
        edit.add_edit(file.clone(), TextEdit::insert(insert_point, &func_def));

        // Replace selection with function call
        edit.add_edit(file.clone(), TextEdit::replace(start, end, &format!("{}()", name)));

        Ok(ExtractResult {
            edit,
            name: name.to_string(),
        })
    }

    async fn extract_variable(
        &self,
        file: &PathBuf,
        start: usize,
        end: usize,
        name: &str,
        source: &str,
    ) -> anyhow::Result<ExtractResult> {
        let selected = &source[start..end.min(source.len())];
        let mut edit = WorkspaceEdit::new();

        // Find the start of the current line for insertion
        let line_start = source[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
        
        // Get indentation
        let indent = &source[line_start..start]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect::<String>();

        // Create variable declaration
        let var_decl = format!("{}let {} = {};\n", indent, name, selected.trim());

        // Insert variable declaration
        edit.add_edit(file.clone(), TextEdit::insert(line_start, &var_decl));

        // Replace selection with variable name
        let adjusted_start = start + var_decl.len();
        let adjusted_end = end + var_decl.len();
        edit.add_edit(file.clone(), TextEdit::replace(adjusted_start, adjusted_end, name));

        Ok(ExtractResult {
            edit,
            name: name.to_string(),
        })
    }

    async fn extract_constant(
        &self,
        file: &PathBuf,
        start: usize,
        end: usize,
        name: &str,
        source: &str,
    ) -> anyhow::Result<ExtractResult> {
        let selected = &source[start..end.min(source.len())];
        let mut edit = WorkspaceEdit::new();

        // Insert at module level (beginning of file or after imports)
        let insert_point = find_module_level_insert_point(source);
        
        let const_name = name.to_uppercase();
        let const_decl = format!("const {}: _ = {};\n\n", const_name, selected.trim());

        // Insert constant declaration
        edit.add_edit(file.clone(), TextEdit::insert(insert_point, &const_decl));

        // Replace selection with constant name
        let adjusted_start = start + const_decl.len();
        let adjusted_end = end + const_decl.len();
        edit.add_edit(file.clone(), TextEdit::replace(adjusted_start, adjusted_end, &const_name));

        Ok(ExtractResult {
            edit,
            name: const_name,
        })
    }
}

/// Find a good insertion point for a new function
fn find_function_insert_point(source: &str, offset: usize) -> usize {
    // Find the start of the current function
    // This is a simplified heuristic
    let before = &source[..offset];
    
    // Look for "fn " going backwards
    if let Some(fn_pos) = before.rfind("\nfn ") {
        fn_pos + 1
    } else if let Some(fn_pos) = before.rfind("fn ") {
        fn_pos
    } else {
        // Insert at end of file
        source.len()
    }
}

/// Find insertion point at module level
fn find_module_level_insert_point(source: &str) -> usize {
    // After last use/import statement
    let mut last_import = 0;
    
    for (i, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("use ") || trimmed.starts_with("import ") {
            if let Some(pos) = source.match_indices(line).nth(0) {
                last_import = pos.0 + line.len() + 1;
            }
        }
    }

    if last_import > 0 {
        // Add a blank line after imports
        last_import
    } else {
        0
    }
}
