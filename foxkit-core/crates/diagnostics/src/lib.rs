//! # Foxkit Diagnostics
//!
//! Problems, errors, and warnings management (LSP-compatible).

pub mod collection;
pub mod quickfix;
pub mod source;

pub use quickfix::{QuickFix, QuickFixKind, QuickFixRegistry, DiagnosticFilter, DiagnosticGrouper};

use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use url::Url;

pub use collection::DiagnosticCollection;
pub use source::DiagnosticSource;

/// Diagnostic severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Error => "Error",
            Severity::Warning => "Warning",
            Severity::Information => "Info",
            Severity::Hint => "Hint",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Information => "info",
            Severity::Hint => "light-bulb",
        }
    }
}

/// A diagnostic (error, warning, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Message
    pub message: String,
    /// Severity
    pub severity: Severity,
    /// Source (e.g., "rustc", "eslint")
    pub source: Option<String>,
    /// Error code
    pub code: Option<DiagnosticCode>,
    /// Range in file
    pub range: Range,
    /// Related information
    #[serde(default)]
    pub related: Vec<RelatedInformation>,
    /// Tags
    #[serde(default)]
    pub tags: Vec<DiagnosticTag>,
    /// Quick fix suggestions
    #[serde(default)]
    pub suggestions: Vec<CodeAction>,
}

impl Diagnostic {
    pub fn error(message: &str, range: Range) -> Self {
        Self {
            message: message.to_string(),
            severity: Severity::Error,
            source: None,
            code: None,
            range,
            related: Vec::new(),
            tags: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn warning(message: &str, range: Range) -> Self {
        Self {
            message: message.to_string(),
            severity: Severity::Warning,
            source: None,
            code: None,
            range,
            related: Vec::new(),
            tags: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn info(message: &str, range: Range) -> Self {
        Self {
            message: message.to_string(),
            severity: Severity::Information,
            source: None,
            code: None,
            range,
            related: Vec::new(),
            tags: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn hint(message: &str, range: Range) -> Self {
        Self {
            message: message.to_string(),
            severity: Severity::Hint,
            source: None,
            code: None,
            range,
            related: Vec::new(),
            tags: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn with_source(mut self, source: &str) -> Self {
        self.source = Some(source.to_string());
        self
    }

    pub fn with_code(mut self, code: impl Into<DiagnosticCode>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_tag(mut self, tag: DiagnosticTag) -> Self {
        self.tags.push(tag);
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    pub fn is_warning(&self) -> bool {
        self.severity == Severity::Warning
    }
}

/// Diagnostic code
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DiagnosticCode {
    Number(i32),
    String(String),
}

impl From<i32> for DiagnosticCode {
    fn from(n: i32) -> Self {
        DiagnosticCode::Number(n)
    }
}

impl From<String> for DiagnosticCode {
    fn from(s: String) -> Self {
        DiagnosticCode::String(s)
    }
}

impl From<&str> for DiagnosticCode {
    fn from(s: &str) -> Self {
        DiagnosticCode::String(s.to_string())
    }
}

/// Position in file
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

/// Range in file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn at_line(line: u32) -> Self {
        Self {
            start: Position::new(line, 0),
            end: Position::new(line, u32::MAX),
        }
    }

    pub fn contains(&self, pos: Position) -> bool {
        if pos.line < self.start.line || pos.line > self.end.line {
            return false;
        }
        if pos.line == self.start.line && pos.character < self.start.character {
            return false;
        }
        if pos.line == self.end.line && pos.character > self.end.character {
            return false;
        }
        true
    }
}

/// Related diagnostic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedInformation {
    pub location: Location,
    pub message: String,
}

/// Location in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

impl Location {
    pub fn new(uri: &str, range: Range) -> Self {
        Self {
            uri: uri.to_string(),
            range,
        }
    }
}

/// Diagnostic tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticTag {
    Unnecessary = 1,
    Deprecated = 2,
}

/// Code action (quick fix)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAction {
    pub title: String,
    pub kind: CodeActionKind,
    pub edit: Option<WorkspaceEdit>,
    pub command: Option<String>,
    pub is_preferred: bool,
}

impl CodeAction {
    pub fn quick_fix(title: &str, edit: WorkspaceEdit) -> Self {
        Self {
            title: title.to_string(),
            kind: CodeActionKind::QuickFix,
            edit: Some(edit),
            command: None,
            is_preferred: false,
        }
    }
}

/// Code action kind
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeActionKind {
    QuickFix,
    Refactor,
    RefactorExtract,
    RefactorInline,
    RefactorRewrite,
    Source,
    SourceOrganizeImports,
    SourceFixAll,
}

/// Workspace edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEdit {
    pub changes: HashMap<String, Vec<TextEdit>>,
}

impl WorkspaceEdit {
    pub fn new() -> Self {
        Self {
            changes: HashMap::new(),
        }
    }

    pub fn add_edit(&mut self, uri: &str, edit: TextEdit) {
        self.changes
            .entry(uri.to_string())
            .or_default()
            .push(edit);
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
    pub range: Range,
    pub new_text: String,
}

impl TextEdit {
    pub fn new(range: Range, new_text: &str) -> Self {
        Self {
            range,
            new_text: new_text.to_string(),
        }
    }

    pub fn insert(position: Position, text: &str) -> Self {
        Self {
            range: Range::new(position, position),
            new_text: text.to_string(),
        }
    }

    pub fn delete(range: Range) -> Self {
        Self {
            range,
            new_text: String::new(),
        }
    }
}

/// Diagnostics manager
pub struct DiagnosticsManager {
    /// Collections by source
    collections: HashMap<String, DiagnosticCollection>,
}

impl DiagnosticsManager {
    pub fn new() -> Self {
        Self {
            collections: HashMap::new(),
        }
    }

    /// Get or create collection
    pub fn collection(&mut self, name: &str) -> &mut DiagnosticCollection {
        self.collections
            .entry(name.to_string())
            .or_insert_with(|| DiagnosticCollection::new(name))
    }

    /// Get all diagnostics for a file
    pub fn get(&self, uri: &str) -> Vec<&Diagnostic> {
        self.collections
            .values()
            .flat_map(|c| c.get(uri))
            .collect()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.collections.values().map(|c| c.error_count()).sum()
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.collections.values().map(|c| c.warning_count()).sum()
    }

    /// Clear all diagnostics
    pub fn clear_all(&mut self) {
        for collection in self.collections.values_mut() {
            collection.clear();
        }
    }
}

impl Default for DiagnosticsManager {
    fn default() -> Self {
        Self::new()
    }
}
