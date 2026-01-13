//! # Foxkit Formatting
//!
//! Code formatting and indentation.

pub mod provider;
pub mod edit;
pub mod options;

use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub use provider::FormattingProvider;
pub use edit::TextEdit;
pub use options::FormattingOptions;

/// Position in document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// Range in document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}

/// Formatting request
#[derive(Debug, Clone)]
pub struct FormatRequest {
    /// File path
    pub file_path: String,
    /// File content
    pub content: String,
    /// Language ID
    pub language_id: String,
    /// Formatting options
    pub options: FormattingOptions,
    /// Range to format (None = whole document)
    pub range: Option<Range>,
}

impl FormatRequest {
    pub fn document(file_path: &str, content: &str, language_id: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            content: content.to_string(),
            language_id: language_id.to_string(),
            options: FormattingOptions::default(),
            range: None,
        }
    }

    pub fn range(file_path: &str, content: &str, language_id: &str, range: Range) -> Self {
        Self {
            file_path: file_path.to_string(),
            content: content.to_string(),
            language_id: language_id.to_string(),
            options: FormattingOptions::default(),
            range: Some(range),
        }
    }

    pub fn with_options(mut self, options: FormattingOptions) -> Self {
        self.options = options;
        self
    }
}

/// Formatting result
#[derive(Debug, Clone)]
pub struct FormatResult {
    /// Text edits to apply
    pub edits: Vec<TextEdit>,
    /// Formatted text (alternative to edits)
    pub formatted: Option<String>,
}

impl FormatResult {
    pub fn edits(edits: Vec<TextEdit>) -> Self {
        Self {
            edits,
            formatted: None,
        }
    }

    pub fn formatted(text: String) -> Self {
        Self {
            edits: Vec::new(),
            formatted: Some(text),
        }
    }

    pub fn no_changes() -> Self {
        Self {
            edits: Vec::new(),
            formatted: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty() && self.formatted.is_none()
    }
}

/// Formatting service
pub struct FormattingService {
    providers: RwLock<Vec<Arc<dyn FormattingProvider>>>,
    default_options: RwLock<FormattingOptions>,
}

impl FormattingService {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(Vec::new()),
            default_options: RwLock::new(FormattingOptions::default()),
        }
    }

    /// Register a provider
    pub fn register(&self, provider: Arc<dyn FormattingProvider>) {
        self.providers.write().push(provider);
    }

    /// Set default options
    pub fn set_default_options(&self, options: FormattingOptions) {
        *self.default_options.write() = options;
    }

    /// Get default options
    pub fn default_options(&self) -> FormattingOptions {
        self.default_options.read().clone()
    }

    /// Format document
    pub fn format(&self, request: &FormatRequest) -> Option<FormatResult> {
        let providers = self.providers.read();
        
        for provider in providers.iter() {
            if provider.supports(&request.language_id) {
                return provider.format(request);
            }
        }
        
        None
    }

    /// Format on type (after typing a character)
    pub fn format_on_type(&self, request: &FormatRequest, char: char) -> Option<FormatResult> {
        let providers = self.providers.read();
        
        for provider in providers.iter() {
            if provider.supports(&request.language_id) && provider.format_on_type_chars().contains(&char) {
                return provider.format_on_type(request, char);
            }
        }
        
        None
    }

    /// Check if formatting is available for language
    pub fn supports(&self, language_id: &str) -> bool {
        self.providers.read().iter().any(|p| p.supports(language_id))
    }
}

impl Default for FormattingService {
    fn default() -> Self {
        Self::new()
    }
}

/// Apply formatting edits to text
pub fn apply_edits(text: &str, edits: &[TextEdit]) -> String {
    if edits.is_empty() {
        return text.to_string();
    }

    // Sort edits by position (reverse order for safe application)
    let mut sorted_edits: Vec<_> = edits.iter().collect();
    sorted_edits.sort_by(|a, b| {
        let cmp = b.range.start.line.cmp(&a.range.start.line);
        if cmp == std::cmp::Ordering::Equal {
            b.range.start.character.cmp(&a.range.start.character)
        } else {
            cmp
        }
    });

    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    
    // Ensure we have enough lines
    if lines.is_empty() {
        lines.push(String::new());
    }

    for edit in sorted_edits {
        apply_single_edit(&mut lines, edit);
    }

    lines.join("\n")
}

fn apply_single_edit(lines: &mut Vec<String>, edit: &TextEdit) {
    let start = &edit.range.start;
    let end = &edit.range.end;

    // Ensure lines exist
    while lines.len() <= end.line as usize {
        lines.push(String::new());
    }

    if start.line == end.line {
        // Single line edit
        let line = &mut lines[start.line as usize];
        let start_char = start.character as usize;
        let end_char = end.character as usize;
        
        let before = if start_char <= line.len() {
            &line[..start_char]
        } else {
            line.as_str()
        };
        
        let after = if end_char <= line.len() {
            &line[end_char..]
        } else {
            ""
        };
        
        *line = format!("{}{}{}", before, edit.new_text, after);
    } else {
        // Multi-line edit
        let first_line = &lines[start.line as usize];
        let last_line = &lines[end.line as usize];
        
        let before = if start.character as usize <= first_line.len() {
            &first_line[..start.character as usize]
        } else {
            first_line.as_str()
        };
        
        let after = if end.character as usize <= last_line.len() {
            &last_line[end.character as usize..]
        } else {
            ""
        };
        
        let new_content = format!("{}{}{}", before, edit.new_text, after);
        let new_lines: Vec<String> = new_content.lines().map(String::from).collect();
        
        // Remove old lines
        for _ in start.line..=end.line {
            if (start.line as usize) < lines.len() {
                lines.remove(start.line as usize);
            }
        }
        
        // Insert new lines
        for (i, new_line) in new_lines.into_iter().enumerate() {
            lines.insert(start.line as usize + i, new_line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_edits() {
        let text = "hello world";
        let edits = vec![
            TextEdit::new(
                Range::new(Position::new(0, 0), Position::new(0, 5)),
                "hi",
            ),
        ];
        
        assert_eq!(apply_edits(text, &edits), "hi world");
    }
}
