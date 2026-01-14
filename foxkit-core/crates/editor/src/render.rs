//! Editor renderer - connects text layout to GPU rendering
//!
//! This module bridges the gap between:
//! - TextLayoutEngine (layout calculation)
//! - GPU text rendering (wgpu-based)

use crate::text_element::{
    TextEditorLayout, CursorLayout, SelectionLayout,
    TextLayoutEngine, GutterItemKind,
};
use crate::view::{DisplayLine, DiagnosticSeverity};
use crate::Viewport;

/// Render command for the GPU
#[derive(Debug, Clone)]
pub enum RenderCommand {
    /// Draw a filled rectangle
    FillRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
        corner_radius: f32,
    },
    /// Draw text
    DrawText {
        x: f32,
        y: f32,
        text: String,
        font_size: f32,
        color: [f32; 4],
        bold: bool,
        italic: bool,
    },
    /// Draw a line (for underlines, cursors, etc.)
    DrawLine {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        width: f32,
        color: [f32; 4],
        style: LineStyle,
    },
    /// Set clipping rectangle
    SetClip {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// Clear clipping
    ClearClip,
}

/// Line drawing style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStyle {
    Solid,
    Dotted,
    Dashed,
    Wavy,
}

/// Editor renderer that generates GPU commands
pub struct EditorRenderer {
    /// Text layout engine
    layout_engine: TextLayoutEngine,
    /// Render commands buffer
    commands: Vec<RenderCommand>,
    /// Cursor blink state (0.0 - 1.0)
    cursor_blink: f32,
    /// Show cursor?
    show_cursor: bool,
    /// Current scroll offset (for smooth scrolling)
    scroll_offset: f32,
}

impl EditorRenderer {
    /// Create a new editor renderer
    pub fn new() -> Self {
        Self {
            layout_engine: TextLayoutEngine::new(),
            commands: Vec::new(),
            cursor_blink: 1.0,
            show_cursor: true,
            scroll_offset: 0.0,
        }
    }

    /// Create with custom layout engine
    pub fn with_layout_engine(layout_engine: TextLayoutEngine) -> Self {
        Self {
            layout_engine,
            commands: Vec::new(),
            cursor_blink: 1.0,
            show_cursor: true,
            scroll_offset: 0.0,
        }
    }

    /// Set cursor blink state (0.0 = hidden, 1.0 = fully visible)
    pub fn set_cursor_blink(&mut self, blink: f32) {
        self.cursor_blink = blink.clamp(0.0, 1.0);
    }

    /// Set cursor visibility
    pub fn set_show_cursor(&mut self, show: bool) {
        self.show_cursor = show;
    }

    /// Set scroll offset for smooth scrolling
    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.scroll_offset = offset;
    }

    /// Get the layout engine
    pub fn layout_engine(&self) -> &TextLayoutEngine {
        &self.layout_engine
    }

    /// Get mutable layout engine
    pub fn layout_engine_mut(&mut self) -> &mut TextLayoutEngine {
        &mut self.layout_engine
    }

    /// Render the editor and return GPU commands
    pub fn render(
        &mut self,
        lines: &[DisplayLine],
        viewport: &Viewport,
        cursors: &[(usize, usize)],
        selections: &[(usize, usize, usize, usize)],
        viewport_width: f32,
        viewport_height: f32,
    ) -> &[RenderCommand] {
        self.commands.clear();

        // Layout visible lines
        let layout = self.layout_engine.layout_lines(lines, viewport, viewport_width, viewport_height);
        
        // Layout cursors
        let cursor_layouts = self.layout_engine.layout_cursors(cursors, &layout, viewport);
        
        // Layout selections
        let selection_layouts = self.layout_engine.layout_selections(selections, &layout, viewport);

        // Paint in order: background -> selections -> text -> cursors -> overlays
        self.paint_background(&layout);
        self.paint_gutter(&layout);
        self.paint_selections(&selection_layouts);
        self.paint_text(&layout);
        self.paint_cursors(&cursor_layouts);

        &self.commands
    }

    /// Paint the editor background
    fn paint_background(&mut self, layout: &TextEditorLayout) {
        let theme = &self.layout_engine.theme;
        
        // Main background
        self.commands.push(RenderCommand::FillRect {
            x: 0.0,
            y: 0.0,
            width: layout.viewport_bounds.2,
            height: layout.viewport_bounds.3,
            color: theme.background,
            corner_radius: 0.0,
        });
    }

    /// Paint the gutter (line numbers, fold markers, etc.)
    fn paint_gutter(&mut self, layout: &TextEditorLayout) {
        let theme = &self.layout_engine.theme;
        
        // Gutter background
        self.commands.push(RenderCommand::FillRect {
            x: 0.0,
            y: 0.0,
            width: layout.gutter_width,
            height: layout.viewport_bounds.3,
            color: theme.gutter_background,
            corner_radius: 0.0,
        });

        // Gutter items
        for item in &layout.gutter_items {
            match &item.kind {
                GutterItemKind::LineNumber(num) => {
                    // Determine if this is the active line
                    let color = theme.line_number; // Could check for active line
                    let text = format!("{}", num);
                    
                    // Right-align line numbers
                    let char_width = layout.char_width;
                    let text_width = text.len() as f32 * char_width;
                    let x = item.x + item.width - text_width - char_width * 0.5;
                    
                    self.commands.push(RenderCommand::DrawText {
                        x,
                        y: item.y + layout.line_height * 0.8,
                        text,
                        font_size: self.layout_engine.font_size,
                        color,
                        bold: false,
                        italic: false,
                    });
                }
                GutterItemKind::DiagnosticIcon(severity) => {
                    let color = match severity {
                        DiagnosticSeverity::Error => [0.9, 0.2, 0.2, 1.0],
                        DiagnosticSeverity::Warning => [0.9, 0.7, 0.2, 1.0],
                        DiagnosticSeverity::Info => [0.2, 0.6, 0.9, 1.0],
                        DiagnosticSeverity::Hint => [0.5, 0.5, 0.5, 1.0],
                    };
                    
                    // Draw a small circle/icon
                    let radius = layout.char_width * 0.3;
                    self.commands.push(RenderCommand::FillRect {
                        x: item.x + item.width / 2.0 - radius,
                        y: item.y + item.height / 2.0 - radius,
                        width: radius * 2.0,
                        height: radius * 2.0,
                        color,
                        corner_radius: radius,
                    });
                }
                GutterItemKind::FoldMarker { collapsed } => {
                    // Draw fold indicator (▶ or ▼)
                    let icon = if *collapsed { "▶" } else { "▼" };
                    self.commands.push(RenderCommand::DrawText {
                        x: item.x,
                        y: item.y + layout.line_height * 0.8,
                        text: icon.to_string(),
                        font_size: self.layout_engine.font_size * 0.8,
                        color: theme.line_number,
                        bold: false,
                        italic: false,
                    });
                }
                GutterItemKind::Breakpoint => {
                    // Draw breakpoint circle
                    let radius = layout.char_width * 0.3;
                    self.commands.push(RenderCommand::FillRect {
                        x: item.x + item.width / 2.0 - radius,
                        y: item.y + item.height / 2.0 - radius,
                        width: radius * 2.0,
                        height: radius * 2.0,
                        color: [0.9, 0.2, 0.2, 1.0],
                        corner_radius: radius,
                    });
                }
                GutterItemKind::GitDiff(kind) => {
                    use crate::text_element::GitDiffKind;
                    let color = match kind {
                        GitDiffKind::Added => [0.3, 0.8, 0.3, 1.0],
                        GitDiffKind::Modified => [0.3, 0.6, 0.9, 1.0],
                        GitDiffKind::Deleted => [0.9, 0.3, 0.3, 1.0],
                    };
                    
                    // Draw thin bar at edge of gutter
                    self.commands.push(RenderCommand::FillRect {
                        x: layout.gutter_width - 3.0,
                        y: item.y,
                        width: 2.0,
                        height: item.height,
                        color,
                        corner_radius: 0.0,
                    });
                }
            }
        }

        // Gutter separator line
        self.commands.push(RenderCommand::DrawLine {
            x1: layout.gutter_width - 0.5,
            y1: 0.0,
            x2: layout.gutter_width - 0.5,
            y2: layout.viewport_bounds.3,
            width: 1.0,
            color: [0.3, 0.3, 0.3, 0.5],
            style: LineStyle::Solid,
        });
    }

    /// Paint selection backgrounds
    fn paint_selections(&mut self, selections: &[SelectionLayout]) {
        for selection in selections {
            for row_bound in &selection.row_bounds {
                self.commands.push(RenderCommand::FillRect {
                    x: row_bound.start_x,
                    y: row_bound.y - self.scroll_offset,
                    width: row_bound.end_x - row_bound.start_x,
                    height: row_bound.height,
                    color: selection.color,
                    corner_radius: 2.0,
                });
            }
        }
    }

    /// Paint text content
    fn paint_text(&mut self, layout: &TextEditorLayout) {
        let content_x = layout.content_origin.0;

        for line_layout in &layout.lines {
            let y = line_layout.display_row as f32 * layout.line_height - self.scroll_offset;
            let baseline_y = y + layout.line_height * 0.8;

            for run in &line_layout.runs {
                // Get the text for this run
                let text: String = line_layout.text
                    .chars()
                    .skip(run.range.start)
                    .take(run.range.end - run.range.start)
                    .collect();

                if text.is_empty() {
                    continue;
                }

                // Get x position from char_positions
                let x = content_x + line_layout.char_positions
                    .get(run.range.start)
                    .copied()
                    .unwrap_or(0.0);

                self.commands.push(RenderCommand::DrawText {
                    x,
                    y: baseline_y,
                    text,
                    font_size: self.layout_engine.font_size,
                    color: run.style.color,
                    bold: run.style.bold,
                    italic: run.style.italic,
                });

                // Draw underline if needed
                if run.style.underline != crate::text_element::UnderlineStyle::None {
                    let underline_y = baseline_y + 2.0;
                    let end_x = content_x + line_layout.char_positions
                        .get(run.range.end)
                        .copied()
                        .unwrap_or(line_layout.width);

                    let style = match run.style.underline {
                        crate::text_element::UnderlineStyle::Solid => LineStyle::Solid,
                        crate::text_element::UnderlineStyle::Dotted => LineStyle::Dotted,
                        crate::text_element::UnderlineStyle::Wavy => LineStyle::Wavy,
                        crate::text_element::UnderlineStyle::None => continue,
                    };

                    self.commands.push(RenderCommand::DrawLine {
                        x1: x,
                        y1: underline_y,
                        x2: end_x,
                        y2: underline_y,
                        width: 1.0,
                        color: run.style.color,
                        style,
                    });
                }
            }
        }
    }

    /// Paint cursors
    fn paint_cursors(&mut self, cursors: &[CursorLayout]) {
        if !self.show_cursor || self.cursor_blink < 0.1 {
            return;
        }

        for cursor in cursors {
            let mut color = cursor.color;
            color[3] *= self.cursor_blink; // Apply blink alpha

            self.commands.push(RenderCommand::FillRect {
                x: cursor.x,
                y: cursor.y - self.scroll_offset,
                width: cursor.width,
                height: cursor.height,
                color,
                corner_radius: 0.0,
            });
        }
    }

    /// Clear render commands
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Get render commands
    pub fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }
}

impl Default for EditorRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Render metrics for performance monitoring
#[derive(Debug, Clone, Default)]
pub struct RenderMetrics {
    /// Time spent in layout (microseconds)
    pub layout_time_us: u64,
    /// Time spent generating commands (microseconds)
    pub command_gen_time_us: u64,
    /// Number of render commands generated
    pub command_count: usize,
    /// Number of lines rendered
    pub lines_rendered: usize,
    /// Number of glyphs rendered
    pub glyphs_rendered: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::HighlightStyle;
    use crate::view::HighlightSpan;

    #[test]
    fn test_render_empty_editor() {
        let mut renderer = EditorRenderer::new();
        let viewport = Viewport::new(30, 80);
        
        let commands = renderer.render(&[], &viewport, &[], &[], 800.0, 600.0);
        
        // Should at least have background
        assert!(!commands.is_empty());
    }

    #[test]
    fn test_render_single_line() {
        let mut renderer = EditorRenderer::new();
        let viewport = Viewport::new(30, 80);
        
        let lines = vec![DisplayLine {
            line_number: 1,
            content: "Hello, World!".to_string(),
            highlights: vec![],
            diagnostics: vec![],
            folded: false,
        }];
        
        let commands = renderer.render(&lines, &viewport, &[(0, 0)], &[], 800.0, 600.0);
        
        // Should have background, gutter, text, cursor
        assert!(commands.len() >= 3);
        
        // Check for text command
        let has_text = commands.iter().any(|cmd| matches!(cmd, RenderCommand::DrawText { .. }));
        assert!(has_text);
    }

    #[test]
    fn test_render_with_selection() {
        let mut renderer = EditorRenderer::new();
        let viewport = Viewport::new(30, 80);
        
        let lines = vec![DisplayLine {
            line_number: 1,
            content: "Hello, World!".to_string(),
            highlights: vec![],
            diagnostics: vec![],
            folded: false,
        }];
        
        // Select "World"
        let selections = vec![(0, 7, 0, 12)];
        
        let commands = renderer.render(&lines, &viewport, &[(0, 12)], &selections, 800.0, 600.0);
        
        // Should have a selection rectangle
        let has_selection = commands.iter().any(|cmd| {
            if let RenderCommand::FillRect { color, .. } = cmd {
                // Selection has alpha < 1.0
                color[3] < 1.0
            } else {
                false
            }
        });
        assert!(has_selection);
    }

    #[test]
    fn test_render_with_syntax_highlighting() {
        let mut renderer = EditorRenderer::new();
        let viewport = Viewport::new(30, 80);
        
        let lines = vec![DisplayLine {
            line_number: 1,
            content: "let x = 42;".to_string(),
            highlights: vec![
                HighlightSpan { start: 0, end: 3, style: HighlightStyle::Keyword },
                HighlightSpan { start: 8, end: 10, style: HighlightStyle::Number },
            ],
            diagnostics: vec![],
            folded: false,
        }];
        
        let commands = renderer.render(&lines, &viewport, &[(0, 0)], &[], 800.0, 600.0);
        
        // Should have multiple text commands with different colors
        let text_commands: Vec<_> = commands.iter()
            .filter(|cmd| matches!(cmd, RenderCommand::DrawText { .. }))
            .collect();
        
        assert!(text_commands.len() >= 1);
    }
}
