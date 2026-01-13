//! Change types

use crate::Position;
use serde::{Deserialize, Serialize};

/// A change in the document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Change {
    /// Text change
    Text(TextChange),
    /// Cursor change
    Cursor(CursorChange),
    /// Selection change
    Selection(SelectionChange),
    /// Multiple changes (atomic)
    Batch(Vec<Change>),
}

impl Change {
    pub fn text(change: TextChange) -> Self {
        Change::Text(change)
    }

    pub fn cursor(change: CursorChange) -> Self {
        Change::Cursor(change)
    }

    pub fn batch(changes: Vec<Change>) -> Self {
        Change::Batch(changes)
    }

    /// Get inverse change (for undo)
    pub fn inverse(&self) -> Change {
        match self {
            Change::Text(tc) => Change::Text(tc.inverse()),
            Change::Cursor(cc) => Change::Cursor(cc.inverse()),
            Change::Selection(sc) => Change::Selection(sc.inverse()),
            Change::Batch(changes) => {
                Change::Batch(changes.iter().rev().map(|c| c.inverse()).collect())
            }
        }
    }
}

/// Text change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChange {
    /// Kind of change
    pub kind: TextChangeKind,
    /// Position
    pub position: Position,
    /// Text involved
    pub text: String,
    /// End position (for delete/replace)
    pub end_position: Option<Position>,
    /// Original text (for replace)
    pub original: Option<String>,
}

/// Text change kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextChangeKind {
    Insert,
    Delete,
    Replace,
}

impl TextChange {
    pub fn insert(position: Position, text: &str) -> Self {
        Self {
            kind: TextChangeKind::Insert,
            position,
            text: text.to_string(),
            end_position: None,
            original: None,
        }
    }

    pub fn delete(start: Position, end: Position, deleted_text: &str) -> Self {
        Self {
            kind: TextChangeKind::Delete,
            position: start,
            text: deleted_text.to_string(),
            end_position: Some(end),
            original: None,
        }
    }

    pub fn replace(start: Position, end: Position, original: &str, new_text: &str) -> Self {
        Self {
            kind: TextChangeKind::Replace,
            position: start,
            text: new_text.to_string(),
            end_position: Some(end),
            original: Some(original.to_string()),
        }
    }

    /// Get inverse change
    pub fn inverse(&self) -> TextChange {
        match self.kind {
            TextChangeKind::Insert => {
                // Inverse of insert is delete
                let end = calculate_end_position(self.position, &self.text);
                TextChange::delete(self.position, end, &self.text)
            }
            TextChangeKind::Delete => {
                // Inverse of delete is insert
                TextChange::insert(self.position, &self.text)
            }
            TextChangeKind::Replace => {
                // Inverse of replace is replace with original
                let original = self.original.clone().unwrap_or_default();
                let new_end = calculate_end_position(self.position, &self.text);
                TextChange::replace(
                    self.position,
                    new_end,
                    &self.text,
                    &original,
                )
            }
        }
    }
}

fn calculate_end_position(start: Position, text: &str) -> Position {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return start;
    }

    if lines.len() == 1 {
        Position::new(start.line, start.column + text.len() as u32)
    } else {
        let last_line_len = lines.last().map(|l| l.len()).unwrap_or(0);
        Position::new(
            start.line + (lines.len() - 1) as u32,
            last_line_len as u32,
        )
    }
}

/// Cursor change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorChange {
    /// Old position
    pub old: Position,
    /// New position
    pub new: Position,
}

impl CursorChange {
    pub fn new(old: Position, new: Position) -> Self {
        Self { old, new }
    }

    pub fn inverse(&self) -> CursorChange {
        CursorChange {
            old: self.new,
            new: self.old,
        }
    }
}

/// Selection change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionChange {
    /// Old selection
    pub old: Option<(Position, Position)>,
    /// New selection
    pub new: Option<(Position, Position)>,
}

impl SelectionChange {
    pub fn new(
        old: Option<(Position, Position)>,
        new: Option<(Position, Position)>,
    ) -> Self {
        Self { old, new }
    }

    pub fn inverse(&self) -> SelectionChange {
        SelectionChange {
            old: self.new,
            new: self.old,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_inverse() {
        let insert = TextChange::insert(Position::new(0, 0), "hello");
        let inverse = insert.inverse();
        
        assert_eq!(inverse.kind, TextChangeKind::Delete);
        assert_eq!(inverse.text, "hello");
    }

    #[test]
    fn test_delete_inverse() {
        let delete = TextChange::delete(
            Position::new(0, 0),
            Position::new(0, 5),
            "hello",
        );
        let inverse = delete.inverse();
        
        assert_eq!(inverse.kind, TextChangeKind::Insert);
        assert_eq!(inverse.text, "hello");
    }
}
