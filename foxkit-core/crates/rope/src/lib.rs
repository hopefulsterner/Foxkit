//! # Foxkit Rope
//!
//! A rope data structure for efficient text manipulation.
//! Optimized for large files and frequent edits.

mod chunk;
mod point;
mod cursor;

use std::ops::Range;
use smallvec::SmallVec;

pub use point::{Point, Offset};
pub use cursor::Cursor;

/// Maximum bytes per chunk
const CHUNK_SIZE: usize = 1024;

/// Rope - a tree-based text data structure
#[derive(Clone)]
pub struct Rope {
    root: Node,
}

impl Rope {
    /// Create an empty rope
    pub fn new() -> Self {
        Self {
            root: Node::Leaf(Chunk::new()),
        }
    }

    /// Create a rope from a string
    pub fn from_str(s: &str) -> Self {
        let mut rope = Self::new();
        rope.insert(0, s);
        rope
    }

    /// Get total length in bytes
    pub fn len(&self) -> usize {
        self.root.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get total number of lines
    pub fn line_count(&self) -> usize {
        self.root.line_count() + 1
    }

    /// Insert text at byte offset
    pub fn insert(&mut self, offset: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        self.root.insert(offset, text);
        self.rebalance();
    }

    /// Delete a range of bytes
    pub fn delete(&mut self, range: Range<usize>) {
        if range.is_empty() {
            return;
        }
        self.root.delete(range);
        self.rebalance();
    }

    /// Replace a range with new text
    pub fn replace(&mut self, range: Range<usize>, text: &str) {
        self.delete(range.clone());
        self.insert(range.start, text);
    }

    /// Get a slice of text
    pub fn slice(&self, range: Range<usize>) -> String {
        let mut result = String::new();
        self.root.slice(range, &mut result);
        result
    }

    /// Get entire text
    pub fn to_string(&self) -> String {
        self.slice(0..self.len())
    }

    /// Get line at index (0-based)
    pub fn line(&self, line_idx: usize) -> Option<String> {
        let start = self.line_to_offset(line_idx)?;
        let end = self.line_to_offset(line_idx + 1).unwrap_or(self.len());
        Some(self.slice(start..end))
    }

    /// Convert line index to byte offset
    pub fn line_to_offset(&self, line_idx: usize) -> Option<usize> {
        if line_idx == 0 {
            return Some(0);
        }
        self.root.line_to_offset(line_idx, 0)
    }

    /// Convert byte offset to point (line, column)
    pub fn offset_to_point(&self, offset: usize) -> Point {
        self.root.offset_to_point(offset)
    }

    /// Convert point to byte offset
    pub fn point_to_offset(&self, point: Point) -> usize {
        let line_start = self.line_to_offset(point.line).unwrap_or(0);
        (line_start + point.column).min(self.len())
    }

    /// Create a cursor at offset
    pub fn cursor(&self, offset: usize) -> Cursor {
        Cursor::new(self, offset)
    }

    /// Iterate over chunks
    pub fn chunks(&self) -> impl Iterator<Item = &str> {
        ChunkIterator { stack: vec![&self.root] }
    }

    /// Iterate over lines
    pub fn lines(&self) -> impl Iterator<Item = String> + '_ {
        (0..self.line_count()).filter_map(|i| self.line(i))
    }

    fn rebalance(&mut self) {
        // Simple rebalancing - merge small nodes
        self.root.rebalance();
    }
}

impl Default for Rope {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Rope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rope({} bytes, {} lines)", self.len(), self.line_count())
    }
}

impl From<&str> for Rope {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl From<String> for Rope {
    fn from(s: String) -> Self {
        Self::from_str(&s)
    }
}

/// A text chunk
#[derive(Clone)]
struct Chunk {
    text: String,
    newlines: SmallVec<[usize; 8]>,
}

impl Chunk {
    fn new() -> Self {
        Self {
            text: String::new(),
            newlines: SmallVec::new(),
        }
    }

    fn from_str(s: &str) -> Self {
        let mut chunk = Self::new();
        chunk.text = s.to_string();
        chunk.recompute_newlines();
        chunk
    }

    fn len(&self) -> usize {
        self.text.len()
    }

    fn line_count(&self) -> usize {
        self.newlines.len()
    }

    fn recompute_newlines(&mut self) {
        self.newlines.clear();
        for (i, c) in self.text.char_indices() {
            if c == '\n' {
                self.newlines.push(i);
            }
        }
    }
}

/// Rope node
#[derive(Clone)]
enum Node {
    Leaf(Chunk),
    Branch {
        len: usize,
        line_count: usize,
        left: Box<Node>,
        right: Box<Node>,
    },
}

impl Node {
    fn len(&self) -> usize {
        match self {
            Node::Leaf(chunk) => chunk.len(),
            Node::Branch { len, .. } => *len,
        }
    }

    fn line_count(&self) -> usize {
        match self {
            Node::Leaf(chunk) => chunk.line_count(),
            Node::Branch { line_count, .. } => *line_count,
        }
    }

    fn insert(&mut self, offset: usize, text: &str) {
        match self {
            Node::Leaf(chunk) => {
                chunk.text.insert_str(offset.min(chunk.len()), text);
                chunk.recompute_newlines();
                
                // Split if too large
                if chunk.len() > CHUNK_SIZE * 2 {
                    let mid = chunk.len() / 2;
                    let right_text = chunk.text.split_off(mid);
                    chunk.recompute_newlines();
                    
                    let left = Box::new(Node::Leaf(chunk.clone()));
                    let right = Box::new(Node::Leaf(Chunk::from_str(&right_text)));
                    
                    *self = Node::Branch {
                        len: left.len() + right.len(),
                        line_count: left.line_count() + right.line_count(),
                        left,
                        right,
                    };
                }
            }
            Node::Branch { len, line_count, left, right } => {
                let left_len = left.len();
                if offset <= left_len {
                    left.insert(offset, text);
                } else {
                    right.insert(offset - left_len, text);
                }
                *len = left.len() + right.len();
                *line_count = left.line_count() + right.line_count();
            }
        }
    }

    fn delete(&mut self, range: Range<usize>) {
        match self {
            Node::Leaf(chunk) => {
                let start = range.start.min(chunk.len());
                let end = range.end.min(chunk.len());
                chunk.text.replace_range(start..end, "");
                chunk.recompute_newlines();
            }
            Node::Branch { len, line_count, left, right } => {
                let left_len = left.len();
                
                if range.end <= left_len {
                    left.delete(range);
                } else if range.start >= left_len {
                    right.delete(range.start - left_len..range.end - left_len);
                } else {
                    left.delete(range.start..left_len);
                    right.delete(0..range.end - left_len);
                }
                
                *len = left.len() + right.len();
                *line_count = left.line_count() + right.line_count();
            }
        }
    }

    fn slice(&self, range: Range<usize>, result: &mut String) {
        match self {
            Node::Leaf(chunk) => {
                let start = range.start.min(chunk.len());
                let end = range.end.min(chunk.len());
                result.push_str(&chunk.text[start..end]);
            }
            Node::Branch { left, right, .. } => {
                let left_len = left.len();
                
                if range.start < left_len {
                    left.slice(range.start..range.end.min(left_len), result);
                }
                if range.end > left_len {
                    right.slice(range.start.saturating_sub(left_len)..range.end - left_len, result);
                }
            }
        }
    }

    fn line_to_offset(&self, target_line: usize, current_offset: usize) -> Option<usize> {
        match self {
            Node::Leaf(chunk) => {
                if target_line == 0 {
                    return Some(current_offset);
                }
                chunk.newlines.get(target_line - 1).map(|&nl| current_offset + nl + 1)
            }
            Node::Branch { left, right, .. } => {
                let left_lines = left.line_count();
                if target_line <= left_lines {
                    left.line_to_offset(target_line, current_offset)
                } else {
                    right.line_to_offset(target_line - left_lines, current_offset + left.len())
                }
            }
        }
    }

    fn offset_to_point(&self, offset: usize) -> Point {
        let mut line = 0;
        let mut col = 0;
        let mut pos = 0;

        for chunk_str in ChunkIterator { stack: vec![self] } {
            for c in chunk_str.chars() {
                if pos == offset {
                    return Point { line, column: col };
                }
                if c == '\n' {
                    line += 1;
                    col = 0;
                } else {
                    col += c.len_utf8();
                }
                pos += c.len_utf8();
            }
        }

        Point { line, column: col }
    }

    fn rebalance(&mut self) {
        if let Node::Branch { left, right, len, line_count } = self {
            left.rebalance();
            right.rebalance();
            
            // Merge if both children are small leaves
            if let (Node::Leaf(l), Node::Leaf(r)) = (left.as_ref(), right.as_ref()) {
                if l.len() + r.len() <= CHUNK_SIZE {
                    let mut merged = l.text.clone();
                    merged.push_str(&r.text);
                    *self = Node::Leaf(Chunk::from_str(&merged));
                }
            }
        }
    }
}

/// Iterator over chunks
struct ChunkIterator<'a> {
    stack: Vec<&'a Node>,
}

impl<'a> Iterator for ChunkIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            match node {
                Node::Leaf(chunk) => return Some(&chunk.text),
                Node::Branch { left, right, .. } => {
                    self.stack.push(right);
                    self.stack.push(left);
                }
            }
        }
        None
    }
}
