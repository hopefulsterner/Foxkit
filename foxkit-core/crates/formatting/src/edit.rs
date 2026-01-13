//! Text edit

use crate::Range;
use serde::{Deserialize, Serialize};

/// Text edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    /// Range to replace
    pub range: Range,
    /// New text
    pub new_text: String,
}

impl TextEdit {
    pub fn new(range: Range, new_text: &str) -> Self {
        Self {
            range,
            new_text: new_text.to_string(),
        }
    }

    /// Insert at position
    pub fn insert(position: crate::Position, text: &str) -> Self {
        Self {
            range: Range::new(position, position),
            new_text: text.to_string(),
        }
    }

    /// Delete range
    pub fn delete(range: Range) -> Self {
        Self {
            range,
            new_text: String::new(),
        }
    }

    /// Replace entire line
    pub fn replace_line(line: u32, new_text: &str) -> Self {
        Self {
            range: Range::new(
                crate::Position::new(line, 0),
                crate::Position::new(line, u32::MAX),
            ),
            new_text: new_text.to_string(),
        }
    }
}

/// Annotated text edit (with annotation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotatedTextEdit {
    /// Base edit
    pub edit: TextEdit,
    /// Annotation ID
    pub annotation_id: String,
}

/// Text edit group (related edits)
#[derive(Debug, Clone)]
pub struct TextEditGroup {
    /// Group label
    pub label: String,
    /// Edits in group
    pub edits: Vec<TextEdit>,
}

impl TextEditGroup {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            edits: Vec::new(),
        }
    }

    pub fn with_edit(mut self, edit: TextEdit) -> Self {
        self.edits.push(edit);
        self
    }

    pub fn add(&mut self, edit: TextEdit) {
        self.edits.push(edit);
    }
}
