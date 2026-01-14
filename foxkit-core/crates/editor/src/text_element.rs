//! Editor text element - GPU-accelerated text rendering for the editor
//!
//! This module provides the text rendering layer that connects:
//! - Buffer content (rope-based storage)
//! - Syntax highlighting (tree-sitter)
//! - GPU rendering (wgpu)

use std::ops::Range;

use crate::view::{DisplayLine, HighlightStyle, DiagnosticSeverity};
use crate::Viewport;

/// Text layout for a single line
#[derive(Debug, Clone)]
pub struct LineLayout {
    /// The display row (0-indexed within viewport)
    pub display_row: usize,
    /// The buffer line number (1-indexed)
    pub line_number: usize,
    /// Text content
    pub text: String,
    /// Character positions (x-coordinates for each character)
    pub char_positions: Vec<f32>,
    /// Width of the line in pixels
    pub width: f32,
    /// Height of the line in pixels  
    pub height: f32,
    /// Runs of styled text
    pub runs: Vec<TextRun>,
}

/// A run of styled text within a line
#[derive(Debug, Clone)]
pub struct TextRun {
    /// Byte range within the line
    pub range: Range<usize>,
    /// Style for this run
    pub style: TextStyle,
}

/// Text styling
#[derive(Debug, Clone, Copy)]
pub struct TextStyle {
    /// Foreground color (RGBA)
    pub color: [f32; 4],
    /// Background color (optional, RGBA)
    pub background: Option<[f32; 4]>,
    /// Is bold?
    pub bold: bool,
    /// Is italic?
    pub italic: bool,
    /// Underline style
    pub underline: UnderlineStyle,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: [0.9, 0.9, 0.9, 1.0], // Light gray
            background: None,
            bold: false,
            italic: false,
            underline: UnderlineStyle::None,
        }
    }
}

/// Underline styles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderlineStyle {
    None,
    Solid,
    Dotted,
    Wavy,
}

/// Cursor rendering info
#[derive(Debug, Clone)]
pub struct CursorLayout {
    /// X position in pixels
    pub x: f32,
    /// Y position in pixels (top of line)
    pub y: f32,
    /// Width (typically 2px for line cursor)
    pub width: f32,
    /// Height (line height)
    pub height: f32,
    /// Cursor color (RGBA)
    pub color: [f32; 4],
    /// Is primary cursor?
    pub is_primary: bool,
}

/// Selection rendering info  
#[derive(Debug, Clone)]
pub struct SelectionLayout {
    /// Bounds for each row the selection spans
    pub row_bounds: Vec<SelectionRowBound>,
    /// Selection color (RGBA)
    pub color: [f32; 4],
}

/// Selection bounds for a single row
#[derive(Debug, Clone)]
pub struct SelectionRowBound {
    /// Display row
    pub row: usize,
    /// Start X position
    pub start_x: f32,
    /// End X position
    pub end_x: f32,
    /// Y position
    pub y: f32,
    /// Height
    pub height: f32,
}

/// Gutter item (line number, fold marker, etc.)
#[derive(Debug, Clone)]
pub struct GutterItem {
    /// Display row
    pub row: usize,
    /// Item type
    pub kind: GutterItemKind,
    /// Bounds
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Types of gutter items
#[derive(Debug, Clone)]
pub enum GutterItemKind {
    LineNumber(usize),
    FoldMarker { collapsed: bool },
    Breakpoint,
    DiagnosticIcon(DiagnosticSeverity),
    GitDiff(GitDiffKind),
}

/// Git diff marker types
#[derive(Debug, Clone, Copy)]
pub enum GitDiffKind {
    Added,
    Modified,
    Deleted,
}

/// Complete layout for the text editor area
pub struct TextEditorLayout {
    /// Line layouts for visible lines
    pub lines: Vec<LineLayout>,
    /// Cursor layouts
    pub cursors: Vec<CursorLayout>,
    /// Selection layouts
    pub selections: Vec<SelectionLayout>,
    /// Gutter items
    pub gutter_items: Vec<GutterItem>,
    /// Content origin (where text starts after gutter)
    pub content_origin: (f32, f32),
    /// Gutter width in pixels
    pub gutter_width: f32,
    /// Line height in pixels
    pub line_height: f32,
    /// Character width (for monospace)
    pub char_width: f32,
    /// Total content height
    pub content_height: f32,
    /// Viewport bounds
    pub viewport_bounds: (f32, f32, f32, f32), // x, y, width, height
}

impl TextEditorLayout {
    /// Create a new empty layout
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            cursors: Vec::new(),
            selections: Vec::new(),
            gutter_items: Vec::new(),
            content_origin: (0.0, 0.0),
            gutter_width: 50.0,
            line_height: 20.0,
            char_width: 8.0,
            content_height: 0.0,
            viewport_bounds: (0.0, 0.0, 800.0, 600.0),
        }
    }
}

impl Default for TextEditorLayout {
    fn default() -> Self {
        Self::new()
    }
}

/// Layout engine for the text editor
pub struct TextLayoutEngine {
    /// Font size in points
    pub font_size: f32,
    /// Line height multiplier
    pub line_height_multiplier: f32,
    /// Tab width in spaces
    pub tab_width: usize,
    /// Show line numbers
    pub show_line_numbers: bool,
    /// Show fold markers
    pub show_fold_markers: bool,
    /// Theme colors
    pub theme: EditorTheme,
}

/// Editor color theme
#[derive(Debug, Clone)]
pub struct EditorTheme {
    /// Background color
    pub background: [f32; 4],
    /// Gutter background
    pub gutter_background: [f32; 4],
    /// Default text color
    pub foreground: [f32; 4],
    /// Line number color
    pub line_number: [f32; 4],
    /// Active line number color
    pub line_number_active: [f32; 4],
    /// Cursor color
    pub cursor: [f32; 4],
    /// Selection color
    pub selection: [f32; 4],
    /// Syntax colors
    pub syntax: SyntaxColors,
}

/// Syntax highlighting colors
#[derive(Debug, Clone)]
pub struct SyntaxColors {
    pub keyword: [f32; 4],
    pub string: [f32; 4],
    pub number: [f32; 4],
    pub comment: [f32; 4],
    pub function: [f32; 4],
    pub r#type: [f32; 4],
    pub variable: [f32; 4],
    pub operator: [f32; 4],
    pub punctuation: [f32; 4],
    pub constant: [f32; 4],
}

impl Default for EditorTheme {
    fn default() -> Self {
        // Default dark theme (similar to VS Code Dark+)
        Self {
            background: [0.12, 0.12, 0.12, 1.0],
            gutter_background: [0.10, 0.10, 0.10, 1.0],
            foreground: [0.85, 0.85, 0.85, 1.0],
            line_number: [0.5, 0.5, 0.5, 1.0],
            line_number_active: [0.9, 0.9, 0.9, 1.0],
            cursor: [1.0, 1.0, 1.0, 1.0],
            selection: [0.26, 0.4, 0.6, 0.4],
            syntax: SyntaxColors::default(),
        }
    }
}

impl Default for SyntaxColors {
    fn default() -> Self {
        Self {
            keyword: [0.78, 0.47, 0.81, 1.0],   // Purple
            string: [0.81, 0.56, 0.47, 1.0],    // Orange
            number: [0.71, 0.82, 0.60, 1.0],    // Green
            comment: [0.5, 0.5, 0.5, 1.0],      // Gray
            function: [0.56, 0.74, 0.86, 1.0],  // Blue
            r#type: [0.31, 0.78, 0.76, 1.0],    // Cyan
            variable: [0.61, 0.76, 0.93, 1.0],  // Light blue
            operator: [0.85, 0.85, 0.85, 1.0],  // White
            punctuation: [0.85, 0.85, 0.85, 1.0], // White
            constant: [0.71, 0.82, 0.60, 1.0],  // Green
        }
    }
}

impl TextLayoutEngine {
    /// Create a new layout engine with default settings
    pub fn new() -> Self {
        Self {
            font_size: 14.0,
            line_height_multiplier: 1.5,
            tab_width: 4,
            show_line_numbers: true,
            show_fold_markers: true,
            theme: EditorTheme::default(),
        }
    }

    /// Calculate line height
    pub fn line_height(&self) -> f32 {
        self.font_size * self.line_height_multiplier
    }

    /// Estimate character width (for monospace fonts)
    pub fn char_width(&self) -> f32 {
        // Approximate: 0.6 * font_size for most monospace fonts
        self.font_size * 0.6
    }

    /// Calculate gutter width based on line count
    pub fn gutter_width(&self, max_line_number: usize) -> f32 {
        let digits = max_line_number.max(1).ilog10() as usize + 1;
        let line_number_width = digits as f32 * self.char_width();
        let padding = self.char_width() * 2.0; // Padding on each side
        let fold_marker_width = if self.show_fold_markers {
            self.char_width() * 2.0
        } else {
            0.0
        };
        
        if self.show_line_numbers {
            line_number_width + padding + fold_marker_width
        } else {
            fold_marker_width + padding
        }
    }

    /// Layout visible lines for rendering
    pub fn layout_lines(
        &self,
        lines: &[DisplayLine],
        viewport: &Viewport,
        viewport_width: f32,
        viewport_height: f32,
    ) -> TextEditorLayout {
        let line_height = self.line_height();
        let char_width = self.char_width();
        let max_line_number = lines.last().map(|l| l.line_number).unwrap_or(1);
        let gutter_width = self.gutter_width(max_line_number);
        let content_x = gutter_width;
        
        let mut layout = TextEditorLayout {
            lines: Vec::with_capacity(lines.len()),
            cursors: Vec::new(),
            selections: Vec::new(),
            gutter_items: Vec::new(),
            content_origin: (content_x, 0.0),
            gutter_width,
            line_height,
            char_width,
            content_height: lines.len() as f32 * line_height,
            viewport_bounds: (0.0, 0.0, viewport_width, viewport_height),
        };

        for (display_row, line) in lines.iter().enumerate() {
            let y = display_row as f32 * line_height;
            
            // Layout line text
            let line_layout = self.layout_line(line, display_row, y, char_width, line_height);
            layout.lines.push(line_layout);
            
            // Add line number gutter item
            if self.show_line_numbers {
                layout.gutter_items.push(GutterItem {
                    row: display_row,
                    kind: GutterItemKind::LineNumber(line.line_number),
                    x: 0.0,
                    y,
                    width: gutter_width - char_width,
                    height: line_height,
                });
            }
            
            // Add diagnostic icons if any
            if let Some(diag) = line.diagnostics.first() {
                layout.gutter_items.push(GutterItem {
                    row: display_row,
                    kind: GutterItemKind::DiagnosticIcon(diag.severity),
                    x: gutter_width - char_width * 1.5,
                    y,
                    width: char_width,
                    height: line_height,
                });
            }
        }

        layout
    }

    /// Layout a single line
    fn layout_line(
        &self,
        line: &DisplayLine,
        display_row: usize,
        y: f32,
        char_width: f32,
        line_height: f32,
    ) -> LineLayout {
        let text = &line.content;
        
        // Calculate character positions (assuming monospace)
        let mut char_positions = Vec::with_capacity(text.len() + 1);
        let mut x = 0.0;
        
        for ch in text.chars() {
            char_positions.push(x);
            x += if ch == '\t' {
                char_width * self.tab_width as f32
            } else {
                char_width
            };
        }
        char_positions.push(x); // End position
        
        let width = x;
        
        // Convert highlight spans to text runs
        let runs = self.create_text_runs(line, text.len());
        
        LineLayout {
            display_row,
            line_number: line.line_number,
            text: text.clone(),
            char_positions,
            width,
            height: line_height,
            runs,
        }
    }

    /// Create text runs from highlight spans
    fn create_text_runs(&self, line: &DisplayLine, text_len: usize) -> Vec<TextRun> {
        if line.highlights.is_empty() {
            // No highlighting - single run with default style
            return vec![TextRun {
                range: 0..text_len,
                style: TextStyle {
                    color: self.theme.foreground,
                    ..Default::default()
                },
            }];
        }

        let mut runs = Vec::new();
        let mut last_end = 0;

        for span in &line.highlights {
            // Add default run before this span if there's a gap
            if span.start > last_end {
                runs.push(TextRun {
                    range: last_end..span.start,
                    style: TextStyle {
                        color: self.theme.foreground,
                        ..Default::default()
                    },
                });
            }

            // Add the highlighted run
            runs.push(TextRun {
                range: span.start..span.end,
                style: self.style_for_highlight(span.style),
            });

            last_end = span.end;
        }

        // Add trailing default run if needed
        if last_end < text_len {
            runs.push(TextRun {
                range: last_end..text_len,
                style: TextStyle {
                    color: self.theme.foreground,
                    ..Default::default()
                },
            });
        }

        runs
    }

    /// Get text style for a highlight type
    fn style_for_highlight(&self, style: HighlightStyle) -> TextStyle {
        let color = match style {
            HighlightStyle::Keyword => self.theme.syntax.keyword,
            HighlightStyle::String => self.theme.syntax.string,
            HighlightStyle::Number => self.theme.syntax.number,
            HighlightStyle::Comment => self.theme.syntax.comment,
            HighlightStyle::Function => self.theme.syntax.function,
            HighlightStyle::Type => self.theme.syntax.r#type,
            HighlightStyle::Variable => self.theme.syntax.variable,
            HighlightStyle::Operator => self.theme.syntax.operator,
            HighlightStyle::Punctuation => self.theme.syntax.punctuation,
            HighlightStyle::Constant => self.theme.syntax.constant,
            HighlightStyle::Parameter => self.theme.syntax.variable,
            HighlightStyle::Property => self.theme.syntax.variable,
            HighlightStyle::Label => self.theme.syntax.function,
        };

        TextStyle {
            color,
            italic: matches!(style, HighlightStyle::Comment),
            ..Default::default()
        }
    }

    /// Layout cursors for rendering
    pub fn layout_cursors(
        &self,
        cursors: &[(usize, usize)], // (line, column) pairs
        layout: &TextEditorLayout,
        viewport: &Viewport,
    ) -> Vec<CursorLayout> {
        let mut cursor_layouts = Vec::with_capacity(cursors.len());
        
        for (idx, (line, column)) in cursors.iter().enumerate() {
            // Check if cursor's line is visible
            if *line < viewport.first_line || *line >= viewport.first_line + viewport.visible_lines {
                continue;
            }
            
            let display_row = line - viewport.first_line;
            
            // Get x position from line layout
            let x = if let Some(line_layout) = layout.lines.get(display_row) {
                let col = (*column).min(line_layout.char_positions.len().saturating_sub(1));
                layout.content_origin.0 + line_layout.char_positions.get(col).copied().unwrap_or(0.0)
            } else {
                layout.content_origin.0
            };
            
            let y = display_row as f32 * layout.line_height;
            
            cursor_layouts.push(CursorLayout {
                x,
                y,
                width: 2.0, // Standard cursor width
                height: layout.line_height,
                color: self.theme.cursor,
                is_primary: idx == 0,
            });
        }
        
        cursor_layouts
    }

    /// Layout selections for rendering
    pub fn layout_selections(
        &self,
        selections: &[(usize, usize, usize, usize)], // (start_line, start_col, end_line, end_col)
        layout: &TextEditorLayout,
        viewport: &Viewport,
    ) -> Vec<SelectionLayout> {
        let mut selection_layouts = Vec::new();
        
        for (start_line, start_col, end_line, end_col) in selections {
            let mut row_bounds = Vec::new();
            
            for line in *start_line..=*end_line {
                // Check if line is visible
                if line < viewport.first_line || line >= viewport.first_line + viewport.visible_lines {
                    continue;
                }
                
                let display_row = line - viewport.first_line;
                let y = display_row as f32 * layout.line_height;
                
                let (start_x, end_x) = if let Some(line_layout) = layout.lines.get(display_row) {
                    let col_start = if line == *start_line { *start_col } else { 0 };
                    let col_end = if line == *end_line { 
                        *end_col 
                    } else { 
                        line_layout.char_positions.len().saturating_sub(1)
                    };
                    
                    let start_x = layout.content_origin.0 + 
                        line_layout.char_positions.get(col_start).copied().unwrap_or(0.0);
                    let end_x = layout.content_origin.0 + 
                        line_layout.char_positions.get(col_end).copied().unwrap_or(line_layout.width);
                    
                    (start_x, end_x)
                } else {
                    (layout.content_origin.0, layout.content_origin.0 + layout.char_width)
                };
                
                row_bounds.push(SelectionRowBound {
                    row: display_row,
                    start_x,
                    end_x,
                    y,
                    height: layout.line_height,
                });
            }
            
            if !row_bounds.is_empty() {
                selection_layouts.push(SelectionLayout {
                    row_bounds,
                    color: self.theme.selection,
                });
            }
        }
        
        selection_layouts
    }
}

impl Default for TextLayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::HighlightSpan;

    #[test]
    fn test_gutter_width() {
        let engine = TextLayoutEngine::new();
        
        // Should accommodate different line number widths
        let w1 = engine.gutter_width(9);     // 1 digit
        let w2 = engine.gutter_width(99);    // 2 digits
        let w3 = engine.gutter_width(999);   // 3 digits
        
        assert!(w2 > w1);
        assert!(w3 > w2);
    }

    #[test]
    fn test_layout_empty_line() {
        let engine = TextLayoutEngine::new();
        let line = DisplayLine {
            line_number: 1,
            content: String::new(),
            highlights: vec![],
            diagnostics: vec![],
            folded: false,
        };
        
        let viewport = Viewport::new(30, 80);
        let layout = engine.layout_lines(&[line], &viewport, 800.0, 600.0);
        
        assert_eq!(layout.lines.len(), 1);
        assert_eq!(layout.lines[0].runs.len(), 1); // Default run
    }

    #[test]
    fn test_layout_with_highlights() {
        let engine = TextLayoutEngine::new();
        let line = DisplayLine {
            line_number: 1,
            content: "let x = 42;".to_string(),
            highlights: vec![
                HighlightSpan { start: 0, end: 3, style: HighlightStyle::Keyword },
                HighlightSpan { start: 8, end: 10, style: HighlightStyle::Number },
            ],
            diagnostics: vec![],
            folded: false,
        };
        
        let viewport = Viewport::new(30, 80);
        let layout = engine.layout_lines(&[line], &viewport, 800.0, 600.0);
        
        assert_eq!(layout.lines.len(), 1);
        // Should have: "let" (keyword) + " x = " (default) + "42" (number) + ";" (default)
        assert!(layout.lines[0].runs.len() >= 3);
    }
}
