//! Buffer snapshots

use crate::BufferId;

/// An immutable snapshot of a buffer
#[derive(Clone)]
pub struct Snapshot {
    /// Buffer ID
    pub id: BufferId,
    /// Version at snapshot time
    pub version: u64,
    /// Rope snapshot
    pub rope: rope::Rope,
}

impl Snapshot {
    /// Get text
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Get slice
    pub fn slice(&self, start: usize, end: usize) -> String {
        self.rope.slice(start..end).to_string()
    }

    /// Get line
    pub fn line(&self, line_idx: usize) -> Option<String> {
        self.rope.line(line_idx).map(|l| l.to_string())
    }

    /// Get line count
    pub fn line_count(&self) -> usize {
        self.rope.line_count()
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.rope.len()
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.rope.is_empty()
    }

    /// Get byte at offset
    pub fn byte(&self, offset: usize) -> Option<u8> {
        if offset < self.len() {
            Some(self.rope.byte(offset))
        } else {
            None
        }
    }

    /// Get char at offset
    pub fn char(&self, offset: usize) -> Option<char> {
        self.rope.char(offset)
    }

    /// Convert offset to line/column
    pub fn offset_to_line_col(&self, offset: usize) -> (usize, usize) {
        let line = self.rope.byte_to_line(offset);
        let line_start = self.rope.line_to_byte(line);
        let col = offset - line_start;
        (line, col)
    }

    /// Convert line/column to offset
    pub fn line_col_to_offset(&self, line: usize, col: usize) -> usize {
        let line_start = self.rope.line_to_byte(line);
        line_start + col
    }

    /// Iterate over lines
    pub fn lines(&self) -> impl Iterator<Item = String> + '_ {
        (0..self.line_count()).map(|i| self.line(i).unwrap_or_default())
    }
}
