//! # Foxkit Linked Editing
//!
//! Synchronized editing of related ranges (e.g., HTML tags).

use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Linked editing service
pub struct LinkedEditingService {
    /// Active linked ranges by file
    active: RwLock<HashMap<PathBuf, LinkedEditingRanges>>,
    /// Events
    events: broadcast::Sender<LinkedEditingEvent>,
    /// Configuration
    config: RwLock<LinkedEditingConfig>,
}

impl LinkedEditingService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            active: RwLock::new(HashMap::new()),
            events,
            config: RwLock::new(LinkedEditingConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<LinkedEditingEvent> {
        self.events.subscribe()
    }

    /// Configure linked editing
    pub fn configure(&self, config: LinkedEditingConfig) {
        *self.config.write() = config;
    }

    /// Get linked ranges at position
    pub async fn get_linked_ranges(
        &self,
        file: &PathBuf,
        line: u32,
        column: u32,
    ) -> Option<LinkedEditingRanges> {
        let config = self.config.read();

        if !config.enabled {
            return None;
        }

        // Would call LSP linkedEditingRange
        None
    }

    /// Set active linked ranges
    pub fn set_active(&self, file: PathBuf, ranges: LinkedEditingRanges) {
        self.active.write().insert(file.clone(), ranges.clone());
        let _ = self.events.send(LinkedEditingEvent::Activated { file, ranges });
    }

    /// Get active linked ranges
    pub fn get_active(&self, file: &PathBuf) -> Option<LinkedEditingRanges> {
        self.active.read().get(file).cloned()
    }

    /// Clear active linked ranges
    pub fn clear_active(&self, file: &PathBuf) {
        self.active.write().remove(file);
        let _ = self.events.send(LinkedEditingEvent::Deactivated { file: file.clone() });
    }

    /// Check if position is in linked range
    pub fn is_in_linked_range(&self, file: &PathBuf, line: u32, column: u32) -> bool {
        if let Some(ranges) = self.get_active(file) {
            return ranges.contains(line, column);
        }
        false
    }

    /// Apply edit to all linked ranges
    pub fn apply_linked_edit(
        &self,
        file: &PathBuf,
        line: u32,
        column: u32,
        text: &str,
        delete_count: u32,
    ) -> Option<Vec<LinkedEdit>> {
        let ranges = self.get_active(file)?;

        // Find which range contains the edit position
        let source_range = ranges.ranges.iter()
            .find(|r| r.contains(line, column))?;

        // Calculate offset within the range
        let offset = if line == source_range.start_line {
            column - source_range.start_col
        } else {
            // Multi-line ranges not fully supported yet
            return None;
        };

        // Generate edits for all other ranges
        let edits: Vec<LinkedEdit> = ranges.ranges.iter()
            .filter(|r| r != &source_range)
            .map(|r| {
                LinkedEdit {
                    range: LinkedRange {
                        start_line: r.start_line,
                        start_col: r.start_col + offset,
                        end_line: r.start_line,
                        end_col: r.start_col + offset + delete_count,
                    },
                    text: text.to_string(),
                }
            })
            .collect();

        Some(edits)
    }
}

impl Default for LinkedEditingService {
    fn default() -> Self {
        Self::new()
    }
}

/// Linked editing ranges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedEditingRanges {
    /// Linked ranges
    pub ranges: Vec<LinkedRange>,
    /// Word pattern (regex)
    pub word_pattern: Option<String>,
}

impl LinkedEditingRanges {
    pub fn new(ranges: Vec<LinkedRange>) -> Self {
        Self { ranges, word_pattern: None }
    }

    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.word_pattern = Some(pattern.into());
        self
    }

    pub fn contains(&self, line: u32, column: u32) -> bool {
        self.ranges.iter().any(|r| r.contains(line, column))
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
}

/// Linked range
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl LinkedRange {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
    }

    pub fn single_line(line: u32, start_col: u32, end_col: u32) -> Self {
        Self { start_line: line, start_col, end_line: line, end_col }
    }

    pub fn contains(&self, line: u32, column: u32) -> bool {
        if line < self.start_line || line > self.end_line {
            return false;
        }
        if line == self.start_line && column < self.start_col {
            return false;
        }
        if line == self.end_line && column > self.end_col {
            return false;
        }
        true
    }

    pub fn length(&self) -> u32 {
        if self.start_line == self.end_line {
            self.end_col - self.start_col
        } else {
            // Multi-line range
            0
        }
    }
}

/// Linked edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedEdit {
    /// Range to edit
    pub range: LinkedRange,
    /// Text to insert
    pub text: String,
}

/// Linked editing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedEditingConfig {
    /// Enable linked editing
    pub enabled: bool,
    /// Show decorations for linked ranges
    pub show_decorations: bool,
    /// Decoration style
    pub decoration_style: LinkedDecorationStyle,
}

impl Default for LinkedEditingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_decorations: true,
            decoration_style: LinkedDecorationStyle::Underline,
        }
    }
}

/// Linked decoration style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkedDecorationStyle {
    /// Underline linked ranges
    Underline,
    /// Box around linked ranges
    Box,
    /// Background highlight
    Background,
    /// No decoration
    None,
}

/// Linked editing event
#[derive(Debug, Clone)]
pub enum LinkedEditingEvent {
    Activated { file: PathBuf, ranges: LinkedEditingRanges },
    Deactivated { file: PathBuf },
    RangesUpdated { file: PathBuf, ranges: LinkedEditingRanges },
}

/// HTML tag matching (common use case)
pub fn find_html_tag_ranges(content: &str, line: u32, column: u32) -> Option<LinkedEditingRanges> {
    // Simple implementation for matching HTML tags
    // Would need proper HTML parsing for production
    
    let lines: Vec<&str> = content.lines().collect();
    let current_line = lines.get(line as usize)?;
    
    // Check if we're in an opening tag
    let before_cursor = &current_line[..column as usize];
    let after_cursor = &current_line[column as usize..];
    
    // Find tag name
    if let Some(open_start) = before_cursor.rfind('<') {
        let tag_content = &before_cursor[open_start + 1..];
        
        // Skip if it's a closing tag
        if tag_content.starts_with('/') {
            return None;
        }
        
        // Extract tag name
        let tag_name: String = tag_content
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        
        if tag_name.is_empty() {
            return None;
        }
        
        // Find closing tag
        let closing_pattern = format!("</{}", tag_name);
        
        // Search for closing tag in remaining content
        for (i, search_line) in lines.iter().enumerate().skip(line as usize) {
            if let Some(close_pos) = search_line.find(&closing_pattern) {
                let close_line = i as u32;
                let close_col = (close_pos + 2) as u32; // Skip "</"
                
                // Create linked ranges
                let open_range = LinkedRange::single_line(
                    line,
                    (open_start + 1) as u32,
                    (open_start + 1 + tag_name.len()) as u32,
                );
                
                let close_range = LinkedRange::single_line(
                    close_line,
                    close_col,
                    close_col + tag_name.len() as u32,
                );
                
                return Some(LinkedEditingRanges::new(vec![open_range, close_range]));
            }
        }
    }
    
    None
}
