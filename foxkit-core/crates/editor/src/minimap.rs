//! Minimap rendering for the editor
//!
//! Provides a zoomed-out view of the entire document for navigation.

use crate::view::{DisplayLine, HighlightSpan, HighlightStyle};

/// Minimap configuration
#[derive(Debug, Clone)]
pub struct MinimapConfig {
    /// Enable minimap
    pub enabled: bool,
    /// Width in pixels
    pub width: f32,
    /// Character scale (how many chars per pixel)
    pub char_scale: f32,
    /// Line height in minimap
    pub line_height: f32,
    /// Show slider
    pub show_slider: bool,
    /// Max lines to render
    pub max_lines: usize,
    /// Show search highlights
    pub show_search_highlights: bool,
    /// Show git changes
    pub show_git_changes: bool,
}

impl Default for MinimapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            width: 80.0,
            char_scale: 0.5,
            line_height: 2.0,
            show_slider: true,
            max_lines: 10000,
            show_search_highlights: true,
            show_git_changes: true,
        }
    }
}

/// Minimap line data
#[derive(Debug, Clone)]
pub struct MinimapLine {
    /// Display row in minimap
    pub row: usize,
    /// Buffer line number
    pub line_number: usize,
    /// Color blocks representing syntax
    pub blocks: Vec<MinimapBlock>,
}

/// A colored block in the minimap
#[derive(Debug, Clone, Copy)]
pub struct MinimapBlock {
    /// Start position (0.0 - 1.0, relative to line width)
    pub start: f32,
    /// End position (0.0 - 1.0)
    pub end: f32,
    /// Color (RGBA)
    pub color: [f32; 4],
}

/// Minimap slider (viewport indicator)
#[derive(Debug, Clone)]
pub struct MinimapSlider {
    /// Top position in minimap
    pub top: f32,
    /// Height
    pub height: f32,
    /// Background color
    pub color: [f32; 4],
}

/// Minimap marker (search result, error, etc.)
#[derive(Debug, Clone)]
pub struct MinimapMarker {
    /// Line number
    pub line: usize,
    /// Marker type
    pub kind: MinimapMarkerKind,
    /// Color
    pub color: [f32; 4],
}

/// Types of minimap markers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinimapMarkerKind {
    SearchResult,
    Error,
    Warning,
    GitAdded,
    GitModified,
    GitDeleted,
    Bookmark,
}

/// Complete minimap layout
#[derive(Debug, Clone)]
pub struct MinimapLayout {
    /// X position (from right edge of editor)
    pub x: f32,
    /// Y position
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Lines to render
    pub lines: Vec<MinimapLine>,
    /// Slider
    pub slider: Option<MinimapSlider>,
    /// Markers
    pub markers: Vec<MinimapMarker>,
    /// Scale factor (content height / minimap height)
    pub scale: f32,
}

/// Minimap renderer
pub struct MinimapRenderer {
    config: MinimapConfig,
    /// Theme colors for minimap
    theme: MinimapTheme,
}

/// Minimap theme colors
#[derive(Debug, Clone)]
pub struct MinimapTheme {
    pub background: [f32; 4],
    pub slider_background: [f32; 4],
    pub slider_hover: [f32; 4],
    pub text_default: [f32; 4],
    pub search_highlight: [f32; 4],
    pub error_marker: [f32; 4],
    pub warning_marker: [f32; 4],
    pub git_added: [f32; 4],
    pub git_modified: [f32; 4],
    pub git_deleted: [f32; 4],
}

impl Default for MinimapTheme {
    fn default() -> Self {
        Self {
            background: [0.15, 0.15, 0.15, 0.9],
            slider_background: [0.3, 0.3, 0.3, 0.5],
            slider_hover: [0.4, 0.4, 0.4, 0.7],
            text_default: [0.6, 0.6, 0.6, 0.8],
            search_highlight: [0.9, 0.7, 0.2, 0.6],
            error_marker: [0.9, 0.2, 0.2, 0.8],
            warning_marker: [0.9, 0.7, 0.2, 0.8],
            git_added: [0.3, 0.8, 0.3, 0.8],
            git_modified: [0.3, 0.6, 0.9, 0.8],
            git_deleted: [0.9, 0.3, 0.3, 0.8],
        }
    }
}

impl MinimapRenderer {
    /// Create a new minimap renderer
    pub fn new(config: MinimapConfig) -> Self {
        Self {
            config,
            theme: MinimapTheme::default(),
        }
    }

    /// Create with custom theme
    pub fn with_theme(config: MinimapConfig, theme: MinimapTheme) -> Self {
        Self { config, theme }
    }

    /// Get configuration
    pub fn config(&self) -> &MinimapConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: MinimapConfig) {
        self.config = config;
    }

    /// Get theme
    pub fn theme(&self) -> &MinimapTheme {
        &self.theme
    }

    /// Set theme
    pub fn set_theme(&mut self, theme: MinimapTheme) {
        self.theme = theme;
    }

    /// Layout the minimap
    pub fn layout(
        &self,
        lines: &[DisplayLine],
        viewport_height: f32,
        first_visible_line: usize,
        visible_line_count: usize,
        editor_height: f32,
    ) -> MinimapLayout {
        if !self.config.enabled || lines.is_empty() {
            return MinimapLayout {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
                lines: vec![],
                slider: None,
                markers: vec![],
                scale: 1.0,
            };
        }

        let total_lines = lines.len().min(self.config.max_lines);
        let minimap_content_height = total_lines as f32 * self.config.line_height;
        let scale = if minimap_content_height > editor_height {
            minimap_content_height / editor_height
        } else {
            1.0
        };

        // Calculate which lines to show in minimap
        let start_line = if scale > 1.0 {
            // Need to scroll minimap
            let scroll_ratio = first_visible_line as f32 / total_lines as f32;
            (scroll_ratio * total_lines as f32) as usize
        } else {
            0
        };

        let lines_to_render = (editor_height / self.config.line_height) as usize;
        let end_line = (start_line + lines_to_render).min(total_lines);

        // Build minimap lines
        let minimap_lines: Vec<MinimapLine> = (start_line..end_line)
            .filter_map(|i| lines.get(i).map(|line| self.build_minimap_line(i, line)))
            .collect();

        // Build slider
        let slider = if self.config.show_slider {
            let slider_top = (first_visible_line as f32 / total_lines as f32) * editor_height;
            let slider_height = (visible_line_count as f32 / total_lines as f32) * editor_height;
            Some(MinimapSlider {
                top: slider_top,
                height: slider_height.max(20.0), // Minimum slider height
                color: self.theme.slider_background,
            })
        } else {
            None
        };

        // Collect markers
        let markers = self.collect_markers(lines);

        MinimapLayout {
            x: viewport_height - self.config.width,
            y: 0.0,
            width: self.config.width,
            height: editor_height,
            lines: minimap_lines,
            slider,
            markers,
            scale,
        }
    }

    /// Build a single minimap line
    fn build_minimap_line(&self, row: usize, line: &DisplayLine) -> MinimapLine {
        let mut blocks = Vec::new();
        let line_len = line.content.len().max(1) as f32;
        let max_chars = (self.config.width / self.config.char_scale) as usize;

        if line.highlights.is_empty() {
            // No syntax highlighting, render as single block
            let end = (line.content.len().min(max_chars) as f32 / max_chars as f32).min(1.0);
            if end > 0.0 {
                blocks.push(MinimapBlock {
                    start: 0.0,
                    end,
                    color: self.theme.text_default,
                });
            }
        } else {
            // Render syntax-highlighted blocks
            for span in &line.highlights {
                let start = (span.start as f32 / max_chars as f32).min(1.0);
                let end = (span.end as f32 / max_chars as f32).min(1.0);
                
                if end > start {
                    blocks.push(MinimapBlock {
                        start,
                        end,
                        color: self.color_for_highlight(span.style),
                    });
                }
            }
        }

        MinimapLine {
            row,
            line_number: line.line_number,
            blocks,
        }
    }

    /// Get color for a highlight style
    fn color_for_highlight(&self, style: HighlightStyle) -> [f32; 4] {
        // Return slightly muted versions of syntax colors for minimap
        let base = match style {
            HighlightStyle::Keyword => [0.78, 0.47, 0.81, 0.7],
            HighlightStyle::String => [0.81, 0.56, 0.47, 0.7],
            HighlightStyle::Number => [0.71, 0.82, 0.60, 0.7],
            HighlightStyle::Comment => [0.5, 0.5, 0.5, 0.5],
            HighlightStyle::Function => [0.56, 0.74, 0.86, 0.7],
            HighlightStyle::Type => [0.31, 0.78, 0.76, 0.7],
            _ => self.theme.text_default,
        };
        base
    }

    /// Collect markers from lines
    fn collect_markers(&self, lines: &[DisplayLine]) -> Vec<MinimapMarker> {
        let mut markers = Vec::new();

        for line in lines {
            // Add diagnostic markers
            for diag in &line.diagnostics {
                let (kind, color) = match diag.severity {
                    crate::view::DiagnosticSeverity::Error => {
                        (MinimapMarkerKind::Error, self.theme.error_marker)
                    }
                    crate::view::DiagnosticSeverity::Warning => {
                        (MinimapMarkerKind::Warning, self.theme.warning_marker)
                    }
                    _ => continue,
                };
                
                markers.push(MinimapMarker {
                    line: line.line_number,
                    kind,
                    color,
                });
            }
        }

        markers
    }

    /// Convert minimap Y coordinate to buffer line
    pub fn y_to_line(&self, y: f32, total_lines: usize, minimap_height: f32) -> usize {
        let ratio = y / minimap_height;
        ((ratio * total_lines as f32) as usize).min(total_lines.saturating_sub(1))
    }

    /// Check if a point is within the slider
    pub fn is_in_slider(&self, y: f32, slider: &MinimapSlider) -> bool {
        y >= slider.top && y <= slider.top + slider.height
    }
}

impl Default for MinimapRenderer {
    fn default() -> Self {
        Self::new(MinimapConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimap_layout_empty() {
        let renderer = MinimapRenderer::new(MinimapConfig::default());
        let layout = renderer.layout(&[], 800.0, 0, 30, 600.0);
        
        assert!(layout.lines.is_empty());
        assert!(layout.slider.is_none());
    }

    #[test]
    fn test_minimap_layout_with_lines() {
        let renderer = MinimapRenderer::new(MinimapConfig::default());
        let lines = vec![
            DisplayLine {
                line_number: 1,
                content: "fn main() {".to_string(),
                highlights: vec![],
                diagnostics: vec![],
                folded: false,
            },
            DisplayLine {
                line_number: 2,
                content: "    println!(\"Hello\");".to_string(),
                highlights: vec![],
                diagnostics: vec![],
                folded: false,
            },
        ];
        
        let layout = renderer.layout(&lines, 800.0, 0, 30, 600.0);
        
        assert!(!layout.lines.is_empty());
        assert!(layout.slider.is_some());
    }

    #[test]
    fn test_y_to_line_conversion() {
        let renderer = MinimapRenderer::new(MinimapConfig::default());
        
        // At top of minimap
        assert_eq!(renderer.y_to_line(0.0, 100, 200.0), 0);
        
        // At middle of minimap
        assert_eq!(renderer.y_to_line(100.0, 100, 200.0), 50);
        
        // At bottom of minimap
        assert_eq!(renderer.y_to_line(200.0, 100, 200.0), 99);
    }

    #[test]
    fn test_minimap_disabled() {
        let config = MinimapConfig {
            enabled: false,
            ..Default::default()
        };
        let renderer = MinimapRenderer::new(config);
        let lines = vec![DisplayLine {
            line_number: 1,
            content: "test".to_string(),
            highlights: vec![],
            diagnostics: vec![],
            folded: false,
        }];
        
        let layout = renderer.layout(&lines, 800.0, 0, 30, 600.0);
        
        assert_eq!(layout.width, 0.0);
        assert!(layout.lines.is_empty());
    }
}
