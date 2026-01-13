//! # Foxkit Indent Guides
//!
//! Visual indentation guides for code structure.

use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Indent guides service
pub struct IndentGuidesService {
    /// Configuration
    config: RwLock<IndentGuidesConfig>,
    /// Cache of computed guides
    cache: RwLock<HashMap<String, Vec<IndentGuide>>>,
}

impl IndentGuidesService {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(IndentGuidesConfig::default()),
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Configure service
    pub fn configure(&self, config: IndentGuidesConfig) {
        *self.config.write() = config;
        self.cache.write().clear();
    }

    /// Get configuration
    pub fn config(&self) -> IndentGuidesConfig {
        self.config.read().clone()
    }

    /// Compute indent guides for content
    pub fn compute_guides(&self, content: &str, tab_size: u32) -> Vec<IndentGuide> {
        let config = self.config.read();
        
        if !config.enabled {
            return Vec::new();
        }

        let mut guides = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Compute indent for each line
        let indents: Vec<u32> = lines
            .iter()
            .map(|line| compute_visual_indent(line, tab_size))
            .collect();

        // Find guide columns
        let max_indent = indents.iter().cloned().max().unwrap_or(0);
        
        for col in (0..max_indent).step_by(tab_size as usize) {
            if col == 0 && !config.show_first_indent {
                continue;
            }

            let mut start_line: Option<u32> = None;

            for (line_num, &indent) in indents.iter().enumerate() {
                let line = lines[line_num];
                let is_blank = line.trim().is_empty();

                if indent > col || is_blank {
                    // Guide should be here
                    if start_line.is_none() {
                        start_line = Some(line_num as u32);
                    }
                } else {
                    // Guide ends here
                    if let Some(start) = start_line {
                        let end = line_num as u32;
                        if end > start {
                            guides.push(IndentGuide {
                                column: col,
                                start_line: start,
                                end_line: end - 1,
                                level: col / tab_size,
                                is_active: false,
                            });
                        }
                        start_line = None;
                    }
                }
            }

            // Handle guide that extends to end
            if let Some(start) = start_line {
                guides.push(IndentGuide {
                    column: col,
                    start_line: start,
                    end_line: lines.len() as u32 - 1,
                    level: col / tab_size,
                    is_active: false,
                });
            }
        }

        guides
    }

    /// Get active guide at cursor position
    pub fn get_active_guide(
        &self,
        guides: &[IndentGuide],
        cursor_line: u32,
        cursor_col: u32,
    ) -> Option<usize> {
        let config = self.config.read();
        
        if !config.highlight_active {
            return None;
        }

        // Find the deepest guide that contains the cursor
        let mut best: Option<(usize, u32)> = None;

        for (i, guide) in guides.iter().enumerate() {
            if cursor_line >= guide.start_line 
                && cursor_line <= guide.end_line
                && cursor_col >= guide.column
            {
                if best.is_none() || guide.column > best.unwrap().1 {
                    best = Some((i, guide.column));
                }
            }
        }

        best.map(|(i, _)| i)
    }

    /// Mark active guide
    pub fn with_active_guide(
        &self,
        mut guides: Vec<IndentGuide>,
        cursor_line: u32,
        cursor_col: u32,
    ) -> Vec<IndentGuide> {
        if let Some(active_idx) = self.get_active_guide(&guides, cursor_line, cursor_col) {
            if let Some(guide) = guides.get_mut(active_idx) {
                guide.is_active = true;
            }
        }
        guides
    }

    /// Get bracket-scoped guides
    pub fn compute_bracket_guides(
        &self,
        content: &str,
        tab_size: u32,
        brackets: &[(u32, u32)], // (open_line, close_line) pairs
    ) -> Vec<IndentGuide> {
        let config = self.config.read();
        
        if !config.bracket_guides {
            return Vec::new();
        }

        let lines: Vec<&str> = content.lines().collect();
        let mut guides = Vec::new();

        for &(open_line, close_line) in brackets {
            if open_line >= lines.len() as u32 {
                continue;
            }

            let indent = compute_visual_indent(lines[open_line as usize], tab_size);

            guides.push(IndentGuide {
                column: indent,
                start_line: open_line,
                end_line: close_line,
                level: indent / tab_size,
                is_active: false,
            });
        }

        guides
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}

impl Default for IndentGuidesService {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_visual_indent(line: &str, tab_size: u32) -> u32 {
    let mut indent = 0;
    
    for c in line.chars() {
        match c {
            ' ' => indent += 1,
            '\t' => indent += tab_size - (indent % tab_size),
            _ => break,
        }
    }

    indent
}

/// Indent guide
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndentGuide {
    /// Column position
    pub column: u32,
    /// Start line
    pub start_line: u32,
    /// End line
    pub end_line: u32,
    /// Nesting level
    pub level: u32,
    /// Is active (cursor is in this scope)
    pub is_active: bool,
}

impl IndentGuide {
    /// Get decoration style
    pub fn style(&self, config: &IndentGuidesConfig) -> GuideStyle {
        if self.is_active {
            GuideStyle {
                color: config.active_color.clone(),
                width: 2,
                style: GuideLineStyle::Solid,
            }
        } else {
            GuideStyle {
                color: config.color.clone(),
                width: 1,
                style: GuideLineStyle::Solid,
            }
        }
    }

    /// Line count
    pub fn line_count(&self) -> u32 {
        self.end_line - self.start_line + 1
    }
}

/// Guide decoration style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideStyle {
    pub color: String,
    pub width: u32,
    pub style: GuideLineStyle,
}

/// Guide line style
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GuideLineStyle {
    Solid,
    Dashed,
    Dotted,
}

/// Indent guides configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndentGuidesConfig {
    /// Enable indent guides
    pub enabled: bool,
    /// Guide color
    pub color: String,
    /// Active guide color
    pub active_color: String,
    /// Highlight active guide
    pub highlight_active: bool,
    /// Show first indent guide
    pub show_first_indent: bool,
    /// Bracket pair guides
    pub bracket_guides: bool,
    /// Maximum indent level
    pub max_indent_level: u32,
}

impl Default for IndentGuidesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            color: "editorIndentGuide.background".to_string(),
            active_color: "editorIndentGuide.activeBackground".to_string(),
            highlight_active: true,
            show_first_indent: true,
            bracket_guides: true,
            max_indent_level: 20,
        }
    }
}
