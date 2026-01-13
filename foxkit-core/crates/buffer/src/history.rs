//! Edit history (undo/redo)

use crate::Edit;

/// Edit history with undo/redo support
pub struct History {
    /// Undo stack
    undo_stack: Vec<HistoryEntry>,
    /// Redo stack
    redo_stack: Vec<HistoryEntry>,
    /// Maximum history size
    max_size: usize,
    /// Group ID for grouping edits
    current_group: Option<u64>,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size: 1000,
            current_group: None,
        }
    }

    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            max_size,
            ..Self::new()
        }
    }

    /// Push an edit to history
    pub fn push(&mut self, edit: Edit, inverse: Edit) {
        // Clear redo stack on new edit
        self.redo_stack.clear();

        let entry = HistoryEntry {
            edit,
            inverse,
            group: self.current_group,
        };

        self.undo_stack.push(entry);

        // Trim if too large
        while self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    /// Start a group of edits (treated as single undo unit)
    pub fn begin_group(&mut self) -> u64 {
        let group_id = timestamp();
        self.current_group = Some(group_id);
        group_id
    }

    /// End current group
    pub fn end_group(&mut self) {
        self.current_group = None;
    }

    /// Undo last edit (returns the edit to apply)
    pub fn undo(&mut self) -> Option<Edit> {
        let entry = self.undo_stack.pop()?;
        let inverse = entry.inverse.clone();
        
        self.redo_stack.push(HistoryEntry {
            edit: entry.inverse,
            inverse: entry.edit,
            group: entry.group,
        });

        Some(inverse)
    }

    /// Redo last undone edit
    pub fn redo(&mut self) -> Option<Edit> {
        let entry = self.redo_stack.pop()?;
        let edit = entry.inverse.clone();
        
        self.undo_stack.push(HistoryEntry {
            edit: entry.inverse,
            inverse: entry.edit,
            group: entry.group,
        });

        Some(edit)
    }

    /// Can undo?
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Can redo?
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Get undo stack size
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get redo stack size
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

/// A history entry
struct HistoryEntry {
    edit: Edit,
    inverse: Edit,
    group: Option<u64>,
}

fn timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
