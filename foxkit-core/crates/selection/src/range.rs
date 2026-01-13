//! Selection range

use crate::Position;
use serde::{Deserialize, Serialize};

/// A selection in the document
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    /// Anchor position (where selection started)
    pub anchor: Position,
    /// Head position (where cursor is, extends to)
    pub head: Position,
}

impl Selection {
    pub fn new(anchor: Position, head: Position) -> Self {
        Self { anchor, head }
    }

    /// Create a caret (no selection, cursor only)
    pub fn caret(position: Position) -> Self {
        Self {
            anchor: position,
            head: position,
        }
    }

    /// Is this just a caret (no selection)?
    pub fn is_caret(&self) -> bool {
        self.anchor == self.head
    }

    /// Is this a selection (non-empty)?
    pub fn is_selection(&self) -> bool {
        !self.is_caret()
    }

    /// Get start position (min of anchor/head)
    pub fn start(&self) -> Position {
        if self.anchor <= self.head {
            self.anchor
        } else {
            self.head
        }
    }

    /// Get end position (max of anchor/head)
    pub fn end(&self) -> Position {
        if self.anchor >= self.head {
            self.anchor
        } else {
            self.head
        }
    }

    /// Is selection reversed (head before anchor)?
    pub fn is_reversed(&self) -> bool {
        self.head < self.anchor
    }

    /// Normalize to forward selection
    pub fn normalize(&self) -> Selection {
        Selection {
            anchor: self.start(),
            head: self.end(),
        }
    }

    /// Get as range (start, end)
    pub fn range(&self) -> (Position, Position) {
        (self.start(), self.end())
    }

    /// Check if position is within selection
    pub fn contains(&self, pos: Position) -> bool {
        let start = self.start();
        let end = self.end();
        pos >= start && pos <= end
    }

    /// Check if selection spans multiple lines
    pub fn is_multiline(&self) -> bool {
        self.start().line != self.end().line
    }

    /// Collapse to head
    pub fn collapse_to_head(&self) -> Selection {
        Selection::caret(self.head)
    }

    /// Collapse to anchor
    pub fn collapse_to_anchor(&self) -> Selection {
        Selection::caret(self.anchor)
    }

    /// Collapse to start
    pub fn collapse_to_start(&self) -> Selection {
        Selection::caret(self.start())
    }

    /// Collapse to end
    pub fn collapse_to_end(&self) -> Selection {
        Selection::caret(self.end())
    }

    /// Extend selection to position
    pub fn extend_to(&self, pos: Position) -> Selection {
        Selection::new(self.anchor, pos)
    }

    /// Union with another selection
    pub fn union(&self, other: &Selection) -> Selection {
        let start = self.start().min(other.start());
        let end = self.end().max(other.end());
        Selection::new(start, end)
    }

    /// Intersection with another selection
    pub fn intersection(&self, other: &Selection) -> Option<Selection> {
        let start = self.start().max(other.start());
        let end = self.end().min(other.end());
        
        if start <= end {
            Some(Selection::new(start, end))
        } else {
            None
        }
    }

    /// Check if overlaps with another selection
    pub fn overlaps(&self, other: &Selection) -> bool {
        self.intersection(other).is_some()
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::caret(Position::zero())
    }
}

/// Selection range (more explicit representation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionRange {
    /// Start line
    pub start_line: u32,
    /// Start column
    pub start_column: u32,
    /// End line
    pub end_line: u32,
    /// End column
    pub end_column: u32,
}

impl SelectionRange {
    pub fn new(start_line: u32, start_column: u32, end_line: u32, end_column: u32) -> Self {
        Self {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    pub fn from_selection(sel: &Selection) -> Self {
        let start = sel.start();
        let end = sel.end();
        Self {
            start_line: start.line,
            start_column: start.column,
            end_line: end.line,
            end_column: end.column,
        }
    }

    pub fn to_selection(&self) -> Selection {
        Selection::new(
            Position::new(self.start_line, self.start_column),
            Position::new(self.end_line, self.end_column),
        )
    }

    pub fn is_empty(&self) -> bool {
        self.start_line == self.end_line && self.start_column == self.end_column
    }

    pub fn line_count(&self) -> u32 {
        self.end_line - self.start_line + 1
    }
}

/// Block (column) selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSelection {
    /// Start position
    pub start: Position,
    /// End position
    pub end: Position,
}

impl BlockSelection {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Get column range
    pub fn columns(&self) -> (u32, u32) {
        let min_col = self.start.column.min(self.end.column);
        let max_col = self.start.column.max(self.end.column);
        (min_col, max_col)
    }

    /// Get line range
    pub fn lines(&self) -> (u32, u32) {
        let min_line = self.start.line.min(self.end.line);
        let max_line = self.start.line.max(self.end.line);
        (min_line, max_line)
    }

    /// Get all ranges (one per line)
    pub fn ranges(&self) -> Vec<SelectionRange> {
        let (start_col, end_col) = self.columns();
        let (start_line, end_line) = self.lines();
        
        (start_line..=end_line)
            .map(|line| SelectionRange::new(line, start_col, line, end_col))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection() {
        let sel = Selection::new(Position::new(0, 5), Position::new(0, 10));
        
        assert!(sel.is_selection());
        assert!(!sel.is_caret());
        assert_eq!(sel.start(), Position::new(0, 5));
        assert_eq!(sel.end(), Position::new(0, 10));
    }

    #[test]
    fn test_reversed_selection() {
        let sel = Selection::new(Position::new(0, 10), Position::new(0, 5));
        
        assert!(sel.is_reversed());
        assert_eq!(sel.start(), Position::new(0, 5));
        assert_eq!(sel.end(), Position::new(0, 10));
    }

    #[test]
    fn test_block_selection() {
        let block = BlockSelection::new(
            Position::new(0, 5),
            Position::new(3, 10),
        );
        
        let ranges = block.ranges();
        assert_eq!(ranges.len(), 4);
        assert_eq!(ranges[0].start_line, 0);
        assert_eq!(ranges[3].start_line, 3);
    }
}
