//! Edit operations

use std::ops::Range;
use rope::Rope;

/// An edit operation
#[derive(Debug, Clone)]
pub struct Edit {
    pub kind: EditKind,
    pub timestamp: u64,
}

impl Edit {
    pub fn insert(offset: usize, text: impl Into<String>) -> Self {
        Self {
            kind: EditKind::Insert {
                offset,
                text: text.into(),
            },
            timestamp: timestamp(),
        }
    }

    pub fn delete(range: Range<usize>) -> Self {
        Self {
            kind: EditKind::Delete { range },
            timestamp: timestamp(),
        }
    }

    pub fn replace(range: Range<usize>, text: impl Into<String>) -> Self {
        Self {
            kind: EditKind::Replace {
                range,
                text: text.into(),
            },
            timestamp: timestamp(),
        }
    }

    /// Get the inverse of this edit (for undo)
    pub fn inverse(&self, rope: &Rope) -> Edit {
        match &self.kind {
            EditKind::Insert { offset, text } => {
                Edit::delete(*offset..*offset + text.len())
            }
            EditKind::Delete { range } => {
                let deleted_text = rope.slice(range.clone());
                Edit::insert(range.start, deleted_text)
            }
            EditKind::Replace { range, text } => {
                let old_text = rope.slice(range.clone());
                Edit::replace(range.start..range.start + text.len(), old_text)
            }
        }
    }

    /// Get the range affected by this edit
    pub fn range(&self) -> Range<usize> {
        match &self.kind {
            EditKind::Insert { offset, text } => *offset..*offset + text.len(),
            EditKind::Delete { range } => range.clone(),
            EditKind::Replace { range, text } => range.start..range.start + text.len(),
        }
    }
}

/// Edit kind
#[derive(Debug, Clone)]
pub enum EditKind {
    Insert { offset: usize, text: String },
    Delete { range: Range<usize> },
    Replace { range: Range<usize>, text: String },
}

fn timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
