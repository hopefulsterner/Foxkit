//! Editor controller - connects ScrollState, Editor, and SoftWrap
//!
//! This module provides the integration layer that connects:
//! - Editor (buffer and state)
//! - ScrollState (smooth scrolling)
//! - SoftWrapEngine (word wrapping)

use crate::{
    EditorView, Viewport, DisplayLine, HighlightSpan, HighlightStyle,
    ScrollState, EasingFunction,
    SoftWrapConfig, SoftWrapEngine, WrappedLine, WrapSegment,
};
use std::time::Duration;

/// Integrated editor controller
pub struct EditorController {
    /// Scroll state for smooth scrolling
    scroll: ScrollState,
    /// Soft wrap engine
    soft_wrap: SoftWrapEngine,
    /// Whether soft wrap is enabled
    soft_wrap_enabled: bool,
    /// Line height in pixels
    line_height: f32,
    /// Viewport dimensions in pixels
    viewport_width_px: f32,
    viewport_height_px: f32,
    /// Character width (for monospace fonts)
    char_width: f32,
    /// Wrap width in characters
    wrap_width: usize,
}

impl EditorController {
    /// Create a new editor controller
    pub fn new() -> Self {
        let scroll = ScrollState::default();
        let soft_wrap = SoftWrapEngine::new(SoftWrapConfig::default());
        
        Self {
            scroll,
            soft_wrap,
            soft_wrap_enabled: true,
            line_height: 20.0,
            viewport_width_px: 800.0,
            viewport_height_px: 600.0,
            char_width: 8.0,
            wrap_width: 100,
        }
    }

    /// Configure the controller
    pub fn configure(
        &mut self,
        line_height: f32,
        char_width: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        self.line_height = line_height;
        self.char_width = char_width;
        self.viewport_width_px = viewport_width;
        self.viewport_height_px = viewport_height;

        // Update scroll state
        self.scroll.set_viewport_size(viewport_height, viewport_width);
        self.scroll.line_height = line_height;

        // Update wrap width
        self.wrap_width = (viewport_width / char_width) as usize;
    }

    /// Set soft wrap enabled/disabled
    pub fn set_soft_wrap_enabled(&mut self, enabled: bool) {
        self.soft_wrap_enabled = enabled;
        let mut config = self.soft_wrap.config().clone();
        config.enabled = enabled;
        self.soft_wrap.set_config(config);
    }

    /// Get mutable scroll state
    pub fn scroll_mut(&mut self) -> &mut ScrollState {
        &mut self.scroll
    }

    /// Get scroll state
    pub fn scroll(&self) -> &ScrollState {
        &self.scroll
    }

    /// Scroll to a specific line (with animation)
    pub fn scroll_to_line(&mut self, line: usize) {
        self.scroll.scroll_to_line(line);
    }

    /// Scroll by a number of lines (with animation)
    pub fn scroll_by_lines(&mut self, delta: i32) {
        if delta > 0 {
            self.scroll.scroll_down(delta as usize);
        } else if delta < 0 {
            self.scroll.scroll_up((-delta) as usize);
        }
    }

    /// Page up
    pub fn page_up(&mut self) {
        self.scroll.page_up();
    }

    /// Page down
    pub fn page_down(&mut self) {
        self.scroll.page_down();
    }

    /// Ensure a line is visible (scrolling if necessary)
    pub fn ensure_line_visible(&mut self, line: usize) {
        self.scroll.ensure_line_visible(line);
    }

    /// Update scroll animation and return current scroll offset
    pub fn update(&mut self) -> ScrollOffset {
        let (vertical, horizontal) = self.scroll.update();
        ScrollOffset {
            x: horizontal,
            y: vertical,
        }
    }

    /// Get wrapped display lines for visible viewport
    pub fn get_display_lines(
        &self,
        buffer_lines: &[String],
        highlights: &[Vec<HighlightSpan>],
        first_visible_line: usize,
        visible_count: usize,
    ) -> Vec<DisplayLineInfo> {
        let mut result = Vec::new();

        let end_line = (first_visible_line + visible_count + 1).min(buffer_lines.len());

        for line_idx in first_visible_line..end_line {
            let line = &buffer_lines[line_idx];
            let line_highlights = highlights.get(line_idx);

            if self.soft_wrap_enabled && !line.is_empty() {
                // Wrap the line
                let wrapped = self.soft_wrap.wrap_line(line, self.wrap_width);
                
                for (wrap_idx, segment) in wrapped.segments.iter().enumerate() {
                    let is_continuation = wrap_idx > 0;
                    let display_text = self.build_segment_text(line, segment);
                    
                    // Map highlights to wrapped segment
                    let mapped_highlights = if let Some(hl) = line_highlights {
                        self.map_highlights_to_segment(hl, segment)
                    } else {
                        Vec::new()
                    };

                    result.push(DisplayLineInfo {
                        buffer_line: line_idx,
                        wrap_index: wrap_idx,
                        content: display_text,
                        highlights: mapped_highlights,
                        is_continuation,
                        indent_width: segment.indent,
                    });
                }
            } else {
                // No wrapping
                result.push(DisplayLineInfo {
                    buffer_line: line_idx,
                    wrap_index: 0,
                    content: line.clone(),
                    highlights: line_highlights.cloned().unwrap_or_default(),
                    is_continuation: false,
                    indent_width: 0,
                });
            }
        }

        result
    }

    /// Build display text from segment
    fn build_segment_text(&self, original: &str, segment: &WrapSegment) -> String {
        let mut result = String::new();
        
        // Add indent for continuation segments
        if segment.indent > 0 {
            result.push_str(&" ".repeat(segment.indent));
        }

        // Extract segment text
        if segment.end <= original.len() && segment.start <= segment.end {
            result.push_str(&original[segment.start..segment.end]);
        }

        result
    }

    /// Map highlights from original line to segment
    fn map_highlights_to_segment(
        &self,
        highlights: &[HighlightSpan],
        segment: &WrapSegment,
    ) -> Vec<HighlightSpan> {
        let mut result = Vec::new();
        let indent_offset = segment.indent;

        for hl in highlights {
            // Check if highlight intersects with this segment
            if hl.end <= segment.start || hl.start >= segment.end {
                continue;
            }

            // Calculate adjusted positions
            let adj_start = hl.start.saturating_sub(segment.start) + indent_offset;
            let adj_end = hl.end.min(segment.end).saturating_sub(segment.start) + indent_offset;

            if adj_end > adj_start {
                result.push(HighlightSpan {
                    start: adj_start,
                    end: adj_end,
                    style: hl.style,
                });
            }
        }

        result
    }

    /// Convert display row (with wrapping) to buffer line
    pub fn display_row_to_buffer_line(&self, display_row: usize, buffer_lines: &[String]) -> (usize, usize) {
        if !self.soft_wrap_enabled {
            return (display_row, 0);
        }

        let mut current_row = 0;
        for (line_idx, line) in buffer_lines.iter().enumerate() {
            let wrapped = self.soft_wrap.wrap_line(line, self.wrap_width);
            let wrap_count = wrapped.segments.len().max(1);

            if current_row + wrap_count > display_row {
                return (line_idx, display_row - current_row);
            }
            current_row += wrap_count;
        }

        (buffer_lines.len().saturating_sub(1), 0)
    }

    /// Convert buffer line to display row
    pub fn buffer_line_to_display_row(&self, buffer_line: usize, buffer_lines: &[String]) -> usize {
        if !self.soft_wrap_enabled {
            return buffer_line;
        }

        let mut display_row = 0;
        for (line_idx, line) in buffer_lines.iter().enumerate() {
            if line_idx >= buffer_line {
                break;
            }
            let wrapped = self.soft_wrap.wrap_line(line, self.wrap_width);
            display_row += wrapped.segments.len().max(1);
        }

        display_row
    }

    /// Get total display rows (accounting for wrapping)
    pub fn total_display_rows(&self, buffer_lines: &[String]) -> usize {
        if !self.soft_wrap_enabled {
            return buffer_lines.len();
        }

        let mut total = 0;
        for line in buffer_lines {
            let wrapped = self.soft_wrap.wrap_line(line, self.wrap_width);
            total += wrapped.segments.len().max(1);
        }
        total
    }

    /// Update content size for scroll bounds
    pub fn update_content_size(&mut self, buffer_lines: &[String]) {
        let total_rows = self.total_display_rows(buffer_lines);
        let content_height = total_rows as f32 * self.line_height;
        self.scroll.set_content_size(content_height, self.viewport_width_px);
    }

    /// Set scroll animation duration
    pub fn set_scroll_duration(&mut self, duration: Duration) {
        self.scroll.vertical.set_duration(duration);
        self.scroll.horizontal.set_duration(duration);
    }

    /// Set scroll easing function
    pub fn set_scroll_easing(&mut self, easing: EasingFunction) {
        self.scroll.vertical.set_easing(easing);
        self.scroll.horizontal.set_easing(easing);
    }

    /// Get line height
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// Get character width
    pub fn char_width(&self) -> f32 {
        self.char_width
    }

    /// Is scroll animating?
    pub fn is_scrolling(&self) -> bool {
        self.scroll.is_animating()
    }
}

impl Default for EditorController {
    fn default() -> Self {
        Self::new()
    }
}

/// Current scroll offset
#[derive(Debug, Clone, Copy, Default)]
pub struct ScrollOffset {
    pub x: f32,
    pub y: f32,
}

/// Display line info with wrapping metadata
#[derive(Debug, Clone)]
pub struct DisplayLineInfo {
    /// Original buffer line index
    pub buffer_line: usize,
    /// Wrap index within the line (0 for first/only segment)
    pub wrap_index: usize,
    /// Display content (may include indent)
    pub content: String,
    /// Highlights adjusted for this wrapped segment
    pub highlights: Vec<HighlightSpan>,
    /// Is this a continuation of the previous line?
    pub is_continuation: bool,
    /// Indent width for continuation lines
    pub indent_width: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_controller_creation() {
        let controller = EditorController::new();
        assert!(controller.soft_wrap_enabled);
        assert_eq!(controller.line_height, 20.0);
    }

    #[test]
    fn test_configure() {
        let mut controller = EditorController::new();
        controller.configure(16.0, 8.0, 640.0, 480.0);
        
        assert_eq!(controller.line_height, 16.0);
        assert_eq!(controller.char_width, 8.0);
        assert_eq!(controller.wrap_width, 80);
    }

    #[test]
    fn test_display_row_to_buffer_line() {
        let controller = EditorController::new();
        
        let lines = vec![
            "short".to_string(),
            "also short".to_string(),
        ];

        let (line, wrap) = controller.display_row_to_buffer_line(0, &lines);
        assert_eq!(line, 0);
        assert_eq!(wrap, 0);

        let (line, wrap) = controller.display_row_to_buffer_line(1, &lines);
        assert_eq!(line, 1);
        assert_eq!(wrap, 0);
    }

    #[test]
    fn test_scroll_offset() {
        let offset = ScrollOffset { x: 10.0, y: 20.0 };
        assert_eq!(offset.x, 10.0);
        assert_eq!(offset.y, 20.0);
    }
}
