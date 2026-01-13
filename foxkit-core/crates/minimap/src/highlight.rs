//! Minimap highlights

use serde::{Deserialize, Serialize};

/// Minimap highlight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimapHighlight {
    /// Start line
    pub line: usize,
    /// End line
    pub end_line: usize,
    /// Color
    pub color: String,
    /// Kind
    pub kind: HighlightKind,
    /// Z-index for layering
    pub z_index: i32,
}

impl MinimapHighlight {
    pub fn new(line: usize, kind: HighlightKind) -> Self {
        Self {
            line,
            end_line: line,
            color: kind.default_color().to_string(),
            kind,
            z_index: 0,
        }
    }

    pub fn with_range(line: usize, end_line: usize, kind: HighlightKind) -> Self {
        Self {
            line,
            end_line,
            color: kind.default_color().to_string(),
            kind,
            z_index: 0,
        }
    }

    pub fn with_color(mut self, color: &str) -> Self {
        self.color = color.to_string();
        self
    }

    pub fn with_z_index(mut self, z_index: i32) -> Self {
        self.z_index = z_index;
        self
    }

    /// Line count
    pub fn line_count(&self) -> usize {
        self.end_line - self.line + 1
    }
}

/// Highlight kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HighlightKind {
    /// Error
    Error,
    /// Warning
    Warning,
    /// Info
    Info,
    /// Hint
    Hint,
    /// Search result
    Search,
    /// Selection
    Selection,
    /// Modified line
    Modified,
    /// Added line
    Added,
    /// Deleted line
    Deleted,
    /// Bookmark
    Bookmark,
    /// Breakpoint
    Breakpoint,
    /// Current line
    CurrentLine,
    /// Find result
    FindResult,
    /// Custom
    Custom,
}

impl HighlightKind {
    pub fn default_color(&self) -> &'static str {
        match self {
            HighlightKind::Error => "rgba(244, 67, 54, 0.6)",
            HighlightKind::Warning => "rgba(255, 193, 7, 0.6)",
            HighlightKind::Info => "rgba(33, 150, 243, 0.6)",
            HighlightKind::Hint => "rgba(76, 175, 80, 0.6)",
            HighlightKind::Search => "rgba(255, 235, 59, 0.5)",
            HighlightKind::Selection => "rgba(33, 150, 243, 0.3)",
            HighlightKind::Modified => "rgba(33, 150, 243, 0.6)",
            HighlightKind::Added => "rgba(76, 175, 80, 0.6)",
            HighlightKind::Deleted => "rgba(244, 67, 54, 0.6)",
            HighlightKind::Bookmark => "rgba(103, 58, 183, 0.6)",
            HighlightKind::Breakpoint => "rgba(244, 67, 54, 0.8)",
            HighlightKind::CurrentLine => "rgba(255, 255, 255, 0.1)",
            HighlightKind::FindResult => "rgba(255, 152, 0, 0.5)",
            HighlightKind::Custom => "rgba(128, 128, 128, 0.5)",
        }
    }
}

/// Highlight builder
pub struct HighlightBuilder {
    highlights: Vec<MinimapHighlight>,
}

impl HighlightBuilder {
    pub fn new() -> Self {
        Self {
            highlights: Vec::new(),
        }
    }

    pub fn add(&mut self, line: usize, kind: HighlightKind) -> &mut Self {
        self.highlights.push(MinimapHighlight::new(line, kind));
        self
    }

    pub fn add_range(&mut self, line: usize, end_line: usize, kind: HighlightKind) -> &mut Self {
        self.highlights
            .push(MinimapHighlight::with_range(line, end_line, kind));
        self
    }

    pub fn add_error(&mut self, line: usize) -> &mut Self {
        self.add(line, HighlightKind::Error)
    }

    pub fn add_warning(&mut self, line: usize) -> &mut Self {
        self.add(line, HighlightKind::Warning)
    }

    pub fn add_search(&mut self, line: usize) -> &mut Self {
        self.add(line, HighlightKind::Search)
    }

    pub fn add_modified(&mut self, line: usize) -> &mut Self {
        self.add(line, HighlightKind::Modified)
    }

    pub fn add_bookmark(&mut self, line: usize) -> &mut Self {
        self.add(line, HighlightKind::Bookmark)
    }

    pub fn add_breakpoint(&mut self, line: usize) -> &mut Self {
        self.add(line, HighlightKind::Breakpoint)
    }

    pub fn build(self) -> Vec<MinimapHighlight> {
        self.highlights
    }
}

impl Default for HighlightBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Decorator for minimap
pub trait MinimapDecorator: Send + Sync {
    /// Get highlights for document
    fn get_highlights(&self, doc_uri: &str, total_lines: usize) -> Vec<MinimapHighlight>;
}

/// Diagnostics decorator
pub struct DiagnosticsDecorator {
    /// Errors by document
    errors: std::collections::HashMap<String, Vec<usize>>,
    /// Warnings by document
    warnings: std::collections::HashMap<String, Vec<usize>>,
}

impl DiagnosticsDecorator {
    pub fn new() -> Self {
        Self {
            errors: std::collections::HashMap::new(),
            warnings: std::collections::HashMap::new(),
        }
    }

    pub fn set_errors(&mut self, doc_uri: &str, lines: Vec<usize>) {
        self.errors.insert(doc_uri.to_string(), lines);
    }

    pub fn set_warnings(&mut self, doc_uri: &str, lines: Vec<usize>) {
        self.warnings.insert(doc_uri.to_string(), lines);
    }

    pub fn clear(&mut self, doc_uri: &str) {
        self.errors.remove(doc_uri);
        self.warnings.remove(doc_uri);
    }
}

impl Default for DiagnosticsDecorator {
    fn default() -> Self {
        Self::new()
    }
}

impl MinimapDecorator for DiagnosticsDecorator {
    fn get_highlights(&self, doc_uri: &str, _total_lines: usize) -> Vec<MinimapHighlight> {
        let mut highlights = Vec::new();

        if let Some(errors) = self.errors.get(doc_uri) {
            for &line in errors {
                highlights.push(MinimapHighlight::new(line, HighlightKind::Error));
            }
        }

        if let Some(warnings) = self.warnings.get(doc_uri) {
            for &line in warnings {
                highlights.push(MinimapHighlight::new(line, HighlightKind::Warning));
            }
        }

        highlights
    }
}

/// Git decorator
pub struct GitDecorator {
    /// Modified lines by document
    modified: std::collections::HashMap<String, Vec<usize>>,
    /// Added lines by document
    added: std::collections::HashMap<String, Vec<usize>>,
}

impl GitDecorator {
    pub fn new() -> Self {
        Self {
            modified: std::collections::HashMap::new(),
            added: std::collections::HashMap::new(),
        }
    }

    pub fn set_changes(&mut self, doc_uri: &str, modified: Vec<usize>, added: Vec<usize>) {
        self.modified.insert(doc_uri.to_string(), modified);
        self.added.insert(doc_uri.to_string(), added);
    }

    pub fn clear(&mut self, doc_uri: &str) {
        self.modified.remove(doc_uri);
        self.added.remove(doc_uri);
    }
}

impl Default for GitDecorator {
    fn default() -> Self {
        Self::new()
    }
}

impl MinimapDecorator for GitDecorator {
    fn get_highlights(&self, doc_uri: &str, _total_lines: usize) -> Vec<MinimapHighlight> {
        let mut highlights = Vec::new();

        if let Some(modified) = self.modified.get(doc_uri) {
            for &line in modified {
                highlights.push(MinimapHighlight::new(line, HighlightKind::Modified));
            }
        }

        if let Some(added) = self.added.get(doc_uri) {
            for &line in added {
                highlights.push(MinimapHighlight::new(line, HighlightKind::Added));
            }
        }

        highlights
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight() {
        let highlight = MinimapHighlight::new(10, HighlightKind::Error);
        assert_eq!(highlight.line, 10);
        assert_eq!(highlight.kind, HighlightKind::Error);
    }

    #[test]
    fn test_builder() {
        let highlights = HighlightBuilder::new()
            .add_error(5)
            .add_warning(10)
            .add_search(15)
            .build();
        
        assert_eq!(highlights.len(), 3);
    }

    #[test]
    fn test_diagnostics_decorator() {
        let mut decorator = DiagnosticsDecorator::new();
        decorator.set_errors("test.rs", vec![5, 10, 15]);
        
        let highlights = decorator.get_highlights("test.rs", 100);
        assert_eq!(highlights.len(), 3);
    }
}
