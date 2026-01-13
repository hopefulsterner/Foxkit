//! # Foxkit Selection Range
//!
//! Smart selection expansion based on AST.

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Selection range service
pub struct SelectionRangeService {
    /// Events
    events: broadcast::Sender<SelectionRangeEvent>,
    /// Current expansion stack
    stack: RwLock<Option<SelectionStack>>,
}

impl SelectionRangeService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            events,
            stack: RwLock::new(None),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<SelectionRangeEvent> {
        self.events.subscribe()
    }

    /// Get selection ranges at positions (would call LSP)
    pub async fn get_selection_ranges(
        &self,
        file: &PathBuf,
        positions: Vec<SelectionPosition>,
    ) -> Vec<SelectionRange> {
        // Would call LSP textDocument/selectionRange
        // For now, return empty
        Vec::new()
    }

    /// Start selection expansion
    pub fn start_expansion(&self, file: PathBuf, initial: SelectionRange) {
        let stack = SelectionStack {
            file,
            ranges: vec![initial.clone()],
            current_index: 0,
        };
        *self.stack.write() = Some(stack);

        let _ = self.events.send(SelectionRangeEvent::ExpansionStarted {
            range: initial,
        });
    }

    /// Expand selection
    pub fn expand(&self) -> Option<SelectionRange> {
        let mut stack = self.stack.write();
        let stack = stack.as_mut()?;

        // Get current range
        let current = stack.ranges.get(stack.current_index)?;

        // If there's a parent, expand to it
        if let Some(ref parent) = current.parent {
            // Add parent to stack if not already there
            if stack.current_index + 1 >= stack.ranges.len() {
                stack.ranges.push((**parent).clone());
            }
            stack.current_index += 1;

            let new_range = stack.ranges.get(stack.current_index)?.clone();
            let _ = self.events.send(SelectionRangeEvent::Expanded {
                range: new_range.clone(),
            });
            
            Some(new_range)
        } else {
            None
        }
    }

    /// Shrink selection
    pub fn shrink(&self) -> Option<SelectionRange> {
        let mut stack = self.stack.write();
        let stack = stack.as_mut()?;

        if stack.current_index > 0 {
            stack.current_index -= 1;
            
            let new_range = stack.ranges.get(stack.current_index)?.clone();
            let _ = self.events.send(SelectionRangeEvent::Shrunk {
                range: new_range.clone(),
            });
            
            Some(new_range)
        } else {
            None
        }
    }

    /// Get current range
    pub fn current(&self) -> Option<SelectionRange> {
        let stack = self.stack.read();
        let stack = stack.as_ref()?;
        stack.ranges.get(stack.current_index).cloned()
    }

    /// Clear expansion stack
    pub fn clear(&self) {
        *self.stack.write() = None;
        let _ = self.events.send(SelectionRangeEvent::Cleared);
    }
}

impl Default for SelectionRangeService {
    fn default() -> Self {
        Self::new()
    }
}

/// Selection expansion stack
struct SelectionStack {
    file: PathBuf,
    ranges: Vec<SelectionRange>,
    current_index: usize,
}

/// Selection range with parent chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionRange {
    /// Range
    pub range: SelectionBounds,
    /// Parent range (for expansion)
    #[serde(skip)]
    pub parent: Option<Box<SelectionRange>>,
}

impl SelectionRange {
    pub fn new(range: SelectionBounds) -> Self {
        Self { range, parent: None }
    }

    pub fn with_parent(mut self, parent: SelectionRange) -> Self {
        self.parent = Some(Box::new(parent));
        self
    }

    /// Build from LSP response
    pub fn from_lsp_chain(ranges: Vec<SelectionBounds>) -> Option<Self> {
        if ranges.is_empty() {
            return None;
        }

        // Build chain from innermost to outermost
        let mut iter = ranges.into_iter().rev();
        let mut current = SelectionRange::new(iter.next()?);

        for range in iter {
            current = SelectionRange::new(range).with_parent(current);
        }

        // Reverse so we have innermost first with parent chain
        Some(reverse_selection_range(current))
    }
}

/// Reverse selection range chain
fn reverse_selection_range(range: SelectionRange) -> SelectionRange {
    let mut ranges = Vec::new();
    let mut current = Some(range);
    
    while let Some(r) = current {
        ranges.push(r.range.clone());
        current = r.parent.map(|p| *p);
    }
    
    ranges.reverse();
    
    let mut iter = ranges.into_iter();
    let mut result = SelectionRange::new(iter.next().unwrap());
    
    for range in iter {
        result = result.with_parent(SelectionRange::new(range));
    }
    
    result
}

/// Selection bounds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionBounds {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl SelectionBounds {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
    }

    pub fn single_line(line: u32, start_col: u32, end_col: u32) -> Self {
        Self { start_line: line, start_col, end_line: line, end_col }
    }

    pub fn point(line: u32, col: u32) -> Self {
        Self { start_line: line, start_col: col, end_line: line, end_col: col }
    }

    /// Check if this range contains another
    pub fn contains(&self, other: &Self) -> bool {
        let start_before = self.start_line < other.start_line ||
            (self.start_line == other.start_line && self.start_col <= other.start_col);
        let end_after = self.end_line > other.end_line ||
            (self.end_line == other.end_line && self.end_col >= other.end_col);
        start_before && end_after
    }

    /// Calculate character count (approximate)
    pub fn char_count(&self) -> u32 {
        if self.start_line == self.end_line {
            self.end_col - self.start_col
        } else {
            // Rough estimate
            (self.end_line - self.start_line) * 80 + self.end_col - self.start_col
        }
    }
}

/// Selection position
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SelectionPosition {
    pub line: u32,
    pub col: u32,
}

impl SelectionPosition {
    pub fn new(line: u32, col: u32) -> Self {
        Self { line, col }
    }
}

/// Selection range event
#[derive(Debug, Clone)]
pub enum SelectionRangeEvent {
    ExpansionStarted { range: SelectionRange },
    Expanded { range: SelectionRange },
    Shrunk { range: SelectionRange },
    Cleared,
}

/// Selection expansion patterns (for languages without LSP)
pub mod patterns {
    use super::*;

    /// Bracket-based expansion
    pub fn bracket_expansion(content: &str, position: SelectionPosition) -> Vec<SelectionBounds> {
        let mut ranges = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        
        let line = position.line as usize;
        let col = position.col as usize;
        
        if line >= lines.len() {
            return ranges;
        }

        // Find word at cursor
        if let Some(word_range) = find_word_at(lines[line], col) {
            ranges.push(SelectionBounds::single_line(
                position.line,
                word_range.0 as u32,
                word_range.1 as u32,
            ));
        }

        // Find matching brackets
        let brackets = [('(', ')'), ('[', ']'), ('{', '}'), ('<', '>')];
        
        for (open, close) in brackets {
            if let Some(range) = find_bracket_pair(content, position, open, close) {
                ranges.push(range);
            }
        }

        // Sort by size
        ranges.sort_by_key(|r| r.char_count());

        ranges
    }

    fn find_word_at(line: &str, col: usize) -> Option<(usize, usize)> {
        let chars: Vec<char> = line.chars().collect();
        
        if col >= chars.len() {
            return None;
        }

        if !chars[col].is_alphanumeric() && chars[col] != '_' {
            return None;
        }

        let mut start = col;
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }

        let mut end = col;
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        Some((start, end))
    }

    fn find_bracket_pair(
        content: &str,
        position: SelectionPosition,
        open: char,
        close: char,
    ) -> Option<SelectionBounds> {
        // Simplified bracket matching
        // Would need proper implementation with nesting
        None
    }
}

/// View model for expand selection UI
pub struct ExpandSelectionViewModel {
    service: Arc<SelectionRangeService>,
}

impl ExpandSelectionViewModel {
    pub fn new(service: Arc<SelectionRangeService>) -> Self {
        Self { service }
    }

    pub async fn expand(&self) -> Option<SelectionBounds> {
        self.service.expand().map(|r| r.range)
    }

    pub fn shrink(&self) -> Option<SelectionBounds> {
        self.service.shrink().map(|r| r.range)
    }

    pub fn current(&self) -> Option<SelectionBounds> {
        self.service.current().map(|r| r.range)
    }
}
