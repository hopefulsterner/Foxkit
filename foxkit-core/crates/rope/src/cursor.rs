//! Cursor for navigating rope

use crate::{Rope, Point};

/// Cursor for navigating a rope
pub struct Cursor<'a> {
    rope: &'a Rope,
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(rope: &'a Rope, offset: usize) -> Self {
        Self {
            rope,
            offset: offset.min(rope.len()),
        }
    }

    /// Get current byte offset
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Get current point (line, column)
    pub fn point(&self) -> Point {
        self.rope.offset_to_point(self.offset)
    }

    /// Move to byte offset
    pub fn seek(&mut self, offset: usize) {
        self.offset = offset.min(self.rope.len());
    }

    /// Move to point
    pub fn seek_point(&mut self, point: Point) {
        self.offset = self.rope.point_to_offset(point);
    }

    /// Move forward by n bytes
    pub fn forward(&mut self, n: usize) {
        self.offset = (self.offset + n).min(self.rope.len());
    }

    /// Move backward by n bytes
    pub fn backward(&mut self, n: usize) {
        self.offset = self.offset.saturating_sub(n);
    }

    /// Move to start of line
    pub fn line_start(&mut self) {
        let point = self.point();
        if let Some(offset) = self.rope.line_to_offset(point.line) {
            self.offset = offset;
        }
    }

    /// Move to end of line
    pub fn line_end(&mut self) {
        let point = self.point();
        if let Some(line) = self.rope.line(point.line) {
            let line_start = self.rope.line_to_offset(point.line).unwrap_or(0);
            let line_len = line.trim_end_matches('\n').len();
            self.offset = line_start + line_len;
        }
    }

    /// Move to next line
    pub fn next_line(&mut self) {
        let point = self.point();
        if let Some(offset) = self.rope.line_to_offset(point.line + 1) {
            self.offset = offset;
        } else {
            self.offset = self.rope.len();
        }
    }

    /// Move to previous line
    pub fn prev_line(&mut self) {
        let point = self.point();
        if point.line > 0 {
            if let Some(offset) = self.rope.line_to_offset(point.line - 1) {
                self.offset = offset;
            }
        } else {
            self.offset = 0;
        }
    }

    /// Get character at cursor
    pub fn char(&self) -> Option<char> {
        self.peek(0)
    }

    /// Peek character at offset from cursor
    pub fn peek(&self, offset: isize) -> Option<char> {
        let pos = if offset >= 0 {
            self.offset + offset as usize
        } else {
            self.offset.checked_sub((-offset) as usize)?
        };
        
        if pos >= self.rope.len() {
            return None;
        }
        
        self.rope.slice(pos..pos + 4).chars().next()
    }

    /// Check if at start of rope
    pub fn at_start(&self) -> bool {
        self.offset == 0
    }

    /// Check if at end of rope
    pub fn at_end(&self) -> bool {
        self.offset >= self.rope.len()
    }
}
