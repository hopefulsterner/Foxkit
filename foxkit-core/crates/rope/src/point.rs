//! Point and offset types

/// A point in text (line, column)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point {
    pub line: usize,
    pub column: usize,
}

impl Point {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    pub fn zero() -> Self {
        Self { line: 0, column: 0 }
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.line.cmp(&other.line) {
            std::cmp::Ordering::Equal => self.column.cmp(&other.column),
            ord => ord,
        }
    }
}

/// Byte offset in text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct Offset(pub usize);

impl Offset {
    pub fn new(offset: usize) -> Self {
        Self(offset)
    }
}

impl From<usize> for Offset {
    fn from(offset: usize) -> Self {
        Self(offset)
    }
}

impl From<Offset> for usize {
    fn from(offset: Offset) -> Self {
        offset.0
    }
}
