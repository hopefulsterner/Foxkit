//! # Foxkit Selection
//!
//! Cursor and selection management with multi-cursor support.

pub mod cursor;
pub mod range;

use serde::{Deserialize, Serialize};

pub use cursor::{Cursor, CursorShape, CursorBlink};
pub use range::{Selection, SelectionRange};

/// Position in document
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Position {
    /// Line number (0-indexed)
    pub line: u32,
    /// Column (character offset, 0-indexed)
    pub column: u32,
}

impl Position {
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }

    pub fn zero() -> Self {
        Self { line: 0, column: 0 }
    }

    pub fn is_zero(&self) -> bool {
        self.line == 0 && self.column == 0
    }

    /// Move position
    pub fn offset(&self, line_delta: i32, column_delta: i32) -> Self {
        Self {
            line: (self.line as i32 + line_delta).max(0) as u32,
            column: (self.column as i32 + column_delta).max(0) as u32,
        }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::zero()
    }
}

/// Selection mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionMode {
    /// Normal character-based selection
    Normal,
    /// Line-based selection (select whole lines)
    Line,
    /// Block/column selection
    Block,
    /// Word selection
    Word,
}

impl Default for SelectionMode {
    fn default() -> Self {
        SelectionMode::Normal
    }
}

/// Selection manager for a document
#[derive(Debug, Clone)]
pub struct SelectionManager {
    /// Primary selection
    primary: Selection,
    /// Secondary selections (multi-cursor)
    secondary: Vec<Selection>,
    /// Selection mode
    mode: SelectionMode,
    /// Preserve column when moving vertically
    preferred_column: Option<u32>,
}

impl SelectionManager {
    pub fn new() -> Self {
        Self {
            primary: Selection::caret(Position::zero()),
            secondary: Vec::new(),
            mode: SelectionMode::Normal,
            preferred_column: None,
        }
    }

    /// Get primary selection
    pub fn primary(&self) -> &Selection {
        &self.primary
    }

    /// Get all selections (primary + secondary)
    pub fn all(&self) -> impl Iterator<Item = &Selection> {
        std::iter::once(&self.primary).chain(self.secondary.iter())
    }

    /// Get cursor position (head of primary selection)
    pub fn cursor(&self) -> Position {
        self.primary.head
    }

    /// Set single cursor
    pub fn set_cursor(&mut self, position: Position) {
        self.primary = Selection::caret(position);
        self.secondary.clear();
        self.preferred_column = Some(position.column);
    }

    /// Set selection
    pub fn set_selection(&mut self, anchor: Position, head: Position) {
        self.primary = Selection::new(anchor, head);
        self.secondary.clear();
        self.preferred_column = Some(head.column);
    }

    /// Add cursor
    pub fn add_cursor(&mut self, position: Position) {
        self.secondary.push(Selection::caret(position));
    }

    /// Add selection
    pub fn add_selection(&mut self, anchor: Position, head: Position) {
        self.secondary.push(Selection::new(anchor, head));
    }

    /// Remove all secondary cursors
    pub fn single_cursor(&mut self) {
        self.secondary.clear();
    }

    /// Has multiple cursors?
    pub fn has_multiple_cursors(&self) -> bool {
        !self.secondary.is_empty()
    }

    /// Cursor count
    pub fn cursor_count(&self) -> usize {
        1 + self.secondary.len()
    }

    /// Has selection (non-empty)?
    pub fn has_selection(&self) -> bool {
        self.primary.is_selection() || self.secondary.iter().any(|s| s.is_selection())
    }

    /// Get selection mode
    pub fn mode(&self) -> SelectionMode {
        self.mode
    }

    /// Set selection mode
    pub fn set_mode(&mut self, mode: SelectionMode) {
        self.mode = mode;
    }

    /// Move cursor (with or without selection)
    pub fn move_cursor(&mut self, new_head: Position, extend_selection: bool) {
        if extend_selection {
            self.primary.head = new_head;
        } else {
            self.primary = Selection::caret(new_head);
        }
        self.preferred_column = Some(new_head.column);
    }

    /// Move all cursors
    pub fn move_all(&mut self, delta: (i32, i32), extend_selection: bool) {
        let (line_delta, col_delta) = delta;
        
        let move_selection = |s: &mut Selection| {
            let new_head = s.head.offset(line_delta, col_delta);
            if extend_selection {
                s.head = new_head;
            } else {
                *s = Selection::caret(new_head);
            }
        };

        move_selection(&mut self.primary);
        for s in &mut self.secondary {
            move_selection(s);
        }
    }

    /// Select all
    pub fn select_all(&mut self, end: Position) {
        self.primary = Selection::new(Position::zero(), end);
        self.secondary.clear();
    }

    /// Merge overlapping selections
    pub fn merge_overlapping(&mut self) {
        // Sort all selections
        let mut all: Vec<Selection> = std::iter::once(self.primary.clone())
            .chain(self.secondary.drain(..))
            .collect();
        
        all.sort_by_key(|s| s.start());

        // Merge overlapping
        let mut merged = Vec::new();
        for sel in all {
            if let Some(last) = merged.last_mut() {
                if overlaps(last, &sel) {
                    // Merge
                    let new_start = last.start().min(sel.start());
                    let new_end = last.end().max(sel.end());
                    *last = Selection::new(new_start, new_end);
                    continue;
                }
            }
            merged.push(sel);
        }

        // Restore
        self.primary = merged.remove(0);
        self.secondary = merged;
    }

    /// Get preferred column (for vertical movement)
    pub fn preferred_column(&self) -> Option<u32> {
        self.preferred_column
    }

    /// Clear preferred column
    pub fn clear_preferred_column(&mut self) {
        self.preferred_column = None;
    }
}

fn overlaps(a: &Selection, b: &Selection) -> bool {
    let a_end = a.end();
    let b_start = b.start();
    a_end >= b_start
}

impl Default for SelectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_manager() {
        let mut manager = SelectionManager::new();
        
        manager.set_cursor(Position::new(5, 10));
        assert_eq!(manager.cursor(), Position::new(5, 10));
        assert!(!manager.has_selection());
        
        manager.set_selection(Position::new(5, 0), Position::new(5, 10));
        assert!(manager.has_selection());
    }

    #[test]
    fn test_multi_cursor() {
        let mut manager = SelectionManager::new();
        
        manager.set_cursor(Position::new(0, 0));
        manager.add_cursor(Position::new(1, 0));
        manager.add_cursor(Position::new(2, 0));
        
        assert!(manager.has_multiple_cursors());
        assert_eq!(manager.cursor_count(), 3);
        
        manager.single_cursor();
        assert!(!manager.has_multiple_cursors());
    }
}
