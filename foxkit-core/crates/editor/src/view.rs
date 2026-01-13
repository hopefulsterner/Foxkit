//! Editor view and viewport management

/// Editor viewport - visible area of the buffer
#[derive(Debug, Clone, Default)]
pub struct Viewport {
    /// First visible line (0-indexed)
    pub first_line: usize,
    /// Number of visible lines
    pub visible_lines: usize,
    /// First visible column (for horizontal scroll)
    pub first_column: usize,
    /// Viewport width in characters
    pub width: usize,
}

impl Viewport {
    /// Create a new viewport
    pub fn new(visible_lines: usize, width: usize) -> Self {
        Self {
            first_line: 0,
            visible_lines,
            first_column: 0,
            width,
        }
    }

    /// Scroll to ensure line is visible
    pub fn scroll_to_line(&mut self, line: usize) {
        if line < self.first_line {
            self.first_line = line;
        } else if line >= self.first_line + self.visible_lines {
            self.first_line = line.saturating_sub(self.visible_lines - 1);
        }
    }

    /// Scroll by delta lines
    pub fn scroll(&mut self, delta: i32, max_line: usize) {
        if delta < 0 {
            self.first_line = self.first_line.saturating_sub((-delta) as usize);
        } else {
            self.first_line = (self.first_line + delta as usize).min(max_line);
        }
    }

    /// Is line visible?
    pub fn is_line_visible(&self, line: usize) -> bool {
        line >= self.first_line && line < self.first_line + self.visible_lines
    }

    /// Last visible line
    pub fn last_line(&self) -> usize {
        self.first_line + self.visible_lines.saturating_sub(1)
    }
}

/// Editor view - presentation layer
pub struct EditorView {
    /// Gutter width (line numbers, etc.)
    pub gutter_width: usize,
    /// Show line numbers?
    pub show_line_numbers: bool,
    /// Show fold markers?
    pub show_fold_markers: bool,
    /// Show minimap?
    pub show_minimap: bool,
    /// Minimap width
    pub minimap_width: usize,
}

impl Default for EditorView {
    fn default() -> Self {
        Self {
            gutter_width: 4,
            show_line_numbers: true,
            show_fold_markers: true,
            show_minimap: true,
            minimap_width: 80,
        }
    }
}

/// Line display info
#[derive(Debug, Clone)]
pub struct DisplayLine {
    /// Line number (1-indexed)
    pub line_number: usize,
    /// Line content
    pub content: String,
    /// Syntax highlighting spans
    pub highlights: Vec<HighlightSpan>,
    /// Diagnostic markers
    pub diagnostics: Vec<DiagnosticMarker>,
    /// Is line folded?
    pub folded: bool,
}

/// Syntax highlight span
#[derive(Debug, Clone)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub style: HighlightStyle,
}

/// Highlight style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightStyle {
    Keyword,
    String,
    Number,
    Comment,
    Function,
    Type,
    Variable,
    Operator,
    Punctuation,
    Constant,
    Parameter,
    Property,
    Label,
}

/// Diagnostic marker on a line
#[derive(Debug, Clone)]
pub struct DiagnosticMarker {
    pub start: usize,
    pub end: usize,
    pub severity: DiagnosticSeverity,
    pub message: String,
}

/// Diagnostic severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}
