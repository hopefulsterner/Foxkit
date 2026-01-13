//! # Foxkit Minimap
//!
//! Code minimap/overview for navigation.

pub mod renderer;
pub mod highlight;

use serde::{Deserialize, Serialize};

pub use renderer::{MinimapRenderer, MinimapConfig};
pub use highlight::{MinimapHighlight, HighlightKind};

/// Minimap data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Minimap {
    /// Total lines
    pub total_lines: usize,
    /// Visible range start
    pub visible_start: usize,
    /// Visible range end
    pub visible_end: usize,
    /// Line data
    pub lines: Vec<MinimapLine>,
    /// Highlights
    pub highlights: Vec<MinimapHighlight>,
    /// Configuration
    pub config: MinimapConfig,
}

impl Minimap {
    pub fn new(total_lines: usize, config: MinimapConfig) -> Self {
        Self {
            total_lines,
            visible_start: 0,
            visible_end: 0,
            lines: Vec::new(),
            highlights: Vec::new(),
            config,
        }
    }

    /// Update from document content
    pub fn update_from_content(&mut self, content: &str) {
        self.lines.clear();
        
        for (idx, line) in content.lines().enumerate() {
            self.lines.push(MinimapLine {
                index: idx,
                indent: Self::calculate_indent(line),
                length: line.len(),
                is_blank: line.trim().is_empty(),
                tokens: Vec::new(),
            });
        }
        
        self.total_lines = self.lines.len();
    }

    fn calculate_indent(line: &str) -> usize {
        line.len() - line.trim_start().len()
    }

    /// Set visible range
    pub fn set_visible_range(&mut self, start: usize, end: usize) {
        self.visible_start = start;
        self.visible_end = end.min(self.total_lines);
    }

    /// Add highlight
    pub fn add_highlight(&mut self, highlight: MinimapHighlight) {
        self.highlights.push(highlight);
    }

    /// Clear highlights of kind
    pub fn clear_highlights(&mut self, kind: HighlightKind) {
        self.highlights.retain(|h| h.kind != kind);
    }

    /// Clear all highlights
    pub fn clear_all_highlights(&mut self) {
        self.highlights.clear();
    }

    /// Get line at y position
    pub fn line_at_y(&self, y: f32, minimap_height: f32) -> Option<usize> {
        if self.total_lines == 0 || minimap_height <= 0.0 {
            return None;
        }

        let line_height = minimap_height / self.total_lines as f32;
        let line = (y / line_height) as usize;
        
        if line < self.total_lines {
            Some(line)
        } else {
            None
        }
    }

    /// Get y position for line
    pub fn y_for_line(&self, line: usize, minimap_height: f32) -> f32 {
        if self.total_lines == 0 {
            return 0.0;
        }

        let line_height = minimap_height / self.total_lines as f32;
        line as f32 * line_height
    }

    /// Get visible region bounds
    pub fn visible_region_bounds(&self, minimap_height: f32) -> (f32, f32) {
        let start_y = self.y_for_line(self.visible_start, minimap_height);
        let end_y = self.y_for_line(self.visible_end, minimap_height);
        (start_y, end_y)
    }

    /// Is line in visible range?
    pub fn is_line_visible(&self, line: usize) -> bool {
        line >= self.visible_start && line < self.visible_end
    }
}

/// Minimap line data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimapLine {
    /// Line index
    pub index: usize,
    /// Indentation level
    pub indent: usize,
    /// Line length
    pub length: usize,
    /// Is blank line
    pub is_blank: bool,
    /// Token information for coloring
    pub tokens: Vec<MinimapToken>,
}

/// Minimap token for syntax coloring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimapToken {
    /// Start column
    pub start: usize,
    /// End column
    pub end: usize,
    /// Token type/scope
    pub scope: String,
}

/// Minimap service
pub struct MinimapService {
    /// Enabled?
    enabled: bool,
    /// Configuration
    config: MinimapConfig,
    /// Minimaps per document
    minimaps: std::collections::HashMap<String, Minimap>,
}

impl MinimapService {
    pub fn new(config: MinimapConfig) -> Self {
        Self {
            enabled: true,
            config,
            minimaps: std::collections::HashMap::new(),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get or create minimap for document
    pub fn get_or_create(&mut self, doc_uri: &str, content: &str) -> &mut Minimap {
        if !self.minimaps.contains_key(doc_uri) {
            let mut minimap = Minimap::new(0, self.config.clone());
            minimap.update_from_content(content);
            self.minimaps.insert(doc_uri.to_string(), minimap);
        }
        self.minimaps.get_mut(doc_uri).unwrap()
    }

    /// Update minimap for document
    pub fn update(&mut self, doc_uri: &str, content: &str) {
        if let Some(minimap) = self.minimaps.get_mut(doc_uri) {
            minimap.update_from_content(content);
        }
    }

    /// Remove minimap for document
    pub fn remove(&mut self, doc_uri: &str) {
        self.minimaps.remove(doc_uri);
    }

    /// Get minimap for document
    pub fn get(&self, doc_uri: &str) -> Option<&Minimap> {
        self.minimaps.get(doc_uri)
    }

    /// Get mutable minimap for document
    pub fn get_mut(&mut self, doc_uri: &str) -> Option<&mut Minimap> {
        self.minimaps.get_mut(doc_uri)
    }
}

impl Default for MinimapService {
    fn default() -> Self {
        Self::new(MinimapConfig::default())
    }
}

/// Minimap position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MinimapPosition {
    /// On the right side
    Right,
    /// On the left side
    Left,
}

impl Default for MinimapPosition {
    fn default() -> Self {
        MinimapPosition::Right
    }
}

/// Minimap rendering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MinimapRenderMode {
    /// Show actual characters
    Characters,
    /// Show colored blocks
    Blocks,
    /// Show colored dots
    Dots,
}

impl Default for MinimapRenderMode {
    fn default() -> Self {
        MinimapRenderMode::Blocks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimap_creation() {
        let mut minimap = Minimap::new(100, MinimapConfig::default());
        minimap.update_from_content("line1\n  line2\n    line3\n");
        
        assert_eq!(minimap.total_lines, 3);
        assert_eq!(minimap.lines[1].indent, 2);
        assert_eq!(minimap.lines[2].indent, 4);
    }

    #[test]
    fn test_visible_range() {
        let mut minimap = Minimap::new(100, MinimapConfig::default());
        minimap.set_visible_range(10, 30);
        
        assert!(minimap.is_line_visible(15));
        assert!(!minimap.is_line_visible(5));
        assert!(!minimap.is_line_visible(35));
    }

    #[test]
    fn test_line_at_y() {
        let mut minimap = Minimap::new(0, MinimapConfig::default());
        minimap.total_lines = 100;
        
        let line = minimap.line_at_y(50.0, 200.0);
        assert_eq!(line, Some(25));
    }
}
