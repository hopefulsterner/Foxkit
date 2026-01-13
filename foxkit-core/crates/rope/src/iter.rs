//! Iterators

use crate::Rope;

/// Byte iterator
pub struct Bytes<'a> {
    rope: &'a Rope,
    offset: usize,
}

impl<'a> Bytes<'a> {
    pub fn new(rope: &'a Rope) -> Self {
        Self { rope, offset: 0 }
    }
}

impl Iterator for Bytes<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.rope.len() {
            return None;
        }
        let byte = self.rope.byte(self.offset);
        self.offset += 1;
        Some(byte)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.rope.len() - self.offset;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Bytes<'_> {}

/// Character iterator
pub struct Chars<'a> {
    rope: &'a Rope,
    offset: usize,
}

impl<'a> Chars<'a> {
    pub fn new(rope: &'a Rope) -> Self {
        Self { rope, offset: 0 }
    }
}

impl Iterator for Chars<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.rope.len() {
            return None;
        }
        
        let c = self.rope.char(self.offset)?;
        self.offset += c.len_utf8();
        Some(c)
    }
}

/// Line iterator
pub struct Lines<'a> {
    rope: &'a Rope,
    line_idx: usize,
}

impl<'a> Lines<'a> {
    pub fn new(rope: &'a Rope) -> Self {
        Self { rope, line_idx: 0 }
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.rope.line(self.line_idx)?;
        self.line_idx += 1;
        Some(line.to_string())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.rope.line_count() - self.line_idx;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Lines<'_> {}
