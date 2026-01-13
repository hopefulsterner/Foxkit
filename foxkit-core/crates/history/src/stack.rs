//! Undo stack implementation

use crate::{Change, Transaction, Position, CursorState};
use std::collections::VecDeque;

/// Undo stack with branching support
#[derive(Debug)]
pub struct UndoStack {
    /// Stack entries
    entries: VecDeque<StackEntry>,
    /// Current position in stack
    position: usize,
    /// Maximum size
    max_size: usize,
    /// Save point (for dirty tracking)
    save_point: Option<usize>,
}

/// Stack entry
#[derive(Debug, Clone)]
pub struct StackEntry {
    /// Transaction
    pub transaction: Transaction,
    /// Branch (for branching undo)
    pub branch: Option<Vec<StackEntry>>,
}

impl StackEntry {
    pub fn new(transaction: Transaction) -> Self {
        Self {
            transaction,
            branch: None,
        }
    }
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            position: 0,
            max_size: 1000,
            save_point: None,
        }
    }

    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    /// Push a transaction
    pub fn push(&mut self, transaction: Transaction) {
        // Remove any entries after current position (standard undo behavior)
        while self.entries.len() > self.position {
            self.entries.pop_back();
        }

        self.entries.push_back(StackEntry::new(transaction));
        self.position = self.entries.len();

        // Trim if over max size
        while self.entries.len() > self.max_size {
            self.entries.pop_front();
            self.position = self.position.saturating_sub(1);
            if let Some(sp) = self.save_point {
                self.save_point = if sp > 0 { Some(sp - 1) } else { None };
            }
        }
    }

    /// Undo and return the transaction to apply
    pub fn undo(&mut self) -> Option<Transaction> {
        if self.position == 0 {
            return None;
        }

        self.position -= 1;
        let entry = self.entries.get(self.position)?;
        Some(entry.transaction.inverse())
    }

    /// Redo and return the transaction to apply
    pub fn redo(&mut self) -> Option<Transaction> {
        if self.position >= self.entries.len() {
            return None;
        }

        let entry = self.entries.get(self.position)?;
        self.position += 1;
        Some(entry.transaction.clone())
    }

    /// Can undo?
    pub fn can_undo(&self) -> bool {
        self.position > 0
    }

    /// Can redo?
    pub fn can_redo(&self) -> bool {
        self.position < self.entries.len()
    }

    /// Mark current position as save point
    pub fn mark_saved(&mut self) {
        self.save_point = Some(self.position);
    }

    /// Is the document dirty (has changes since save)?
    pub fn is_dirty(&self) -> bool {
        self.save_point != Some(self.position)
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.entries.clear();
        self.position = 0;
        self.save_point = None;
    }

    /// Get number of undo steps available
    pub fn undo_count(&self) -> usize {
        self.position
    }

    /// Get number of redo steps available
    pub fn redo_count(&self) -> usize {
        self.entries.len() - self.position
    }

    /// Get total history size
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is stack empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get entries for display
    pub fn entries(&self) -> impl Iterator<Item = &Transaction> {
        self.entries.iter().map(|e| &e.transaction)
    }

    /// Get current position
    pub fn position(&self) -> usize {
        self.position
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

/// Undo group for batching related undos
pub struct UndoGroup {
    changes: Vec<Change>,
    description: Option<String>,
    cursor_before: Option<CursorState>,
}

impl UndoGroup {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
            description: None,
            cursor_before: None,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn with_cursor(mut self, cursor: CursorState) -> Self {
        self.cursor_before = Some(cursor);
        self
    }

    pub fn add(&mut self, change: Change) {
        self.changes.push(change);
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn into_transaction(self, id: u64) -> Transaction {
        let mut tx = Transaction::new(id);
        tx.changes = self.changes;
        tx.description = self.description;
        tx.cursor_before = self.cursor_before;
        tx
    }
}

impl Default for UndoGroup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change::TextChange;

    #[test]
    fn test_undo_stack() {
        let mut stack = UndoStack::new();
        
        let mut tx = Transaction::new(1);
        tx.add(Change::Text(TextChange::insert(Position::new(0, 0), "hello")));
        stack.push(tx);
        
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
        
        stack.undo();
        assert!(!stack.can_undo());
        assert!(stack.can_redo());
        
        stack.redo();
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_dirty_tracking() {
        let mut stack = UndoStack::new();
        
        assert!(!stack.is_dirty()); // Empty is not dirty
        
        let tx = Transaction::new(1);
        stack.push(tx);
        
        assert!(stack.is_dirty());
        
        stack.mark_saved();
        assert!(!stack.is_dirty());
        
        stack.undo();
        assert!(stack.is_dirty());
    }
}
