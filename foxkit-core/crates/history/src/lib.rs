//! # Foxkit History
//!
//! Undo/redo and change history management.

pub mod change;
pub mod stack;

use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

pub use change::{Change, TextChange};
pub use stack::UndoStack;

/// Transaction for grouping changes
#[derive(Debug, Clone)]
pub struct Transaction {
    /// Transaction ID
    pub id: u64,
    /// Changes in this transaction
    pub changes: Vec<Change>,
    /// Timestamp
    pub timestamp: std::time::SystemTime,
    /// Description
    pub description: Option<String>,
    /// Cursor position before
    pub cursor_before: Option<CursorState>,
    /// Cursor position after
    pub cursor_after: Option<CursorState>,
}

impl Transaction {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            changes: Vec::new(),
            timestamp: std::time::SystemTime::now(),
            description: None,
            cursor_before: None,
            cursor_after: None,
        }
    }

    pub fn with_change(mut self, change: Change) -> Self {
        self.changes.push(change);
        self
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn add(&mut self, change: Change) {
        self.changes.push(change);
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Get inverse transaction (for undo)
    pub fn inverse(&self) -> Transaction {
        Transaction {
            id: self.id,
            changes: self.changes.iter().rev().map(|c| c.inverse()).collect(),
            timestamp: self.timestamp,
            description: self.description.clone(),
            cursor_before: self.cursor_after.clone(),
            cursor_after: self.cursor_before.clone(),
        }
    }
}

/// Cursor state for restoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorState {
    /// Cursor position
    pub position: Position,
    /// Selection anchor (if any)
    pub anchor: Option<Position>,
    /// Multiple cursors
    pub secondary: Vec<(Position, Option<Position>)>,
}

impl CursorState {
    pub fn new(position: Position) -> Self {
        Self {
            position,
            anchor: None,
            secondary: Vec::new(),
        }
    }

    pub fn with_selection(mut self, anchor: Position) -> Self {
        self.anchor = Some(anchor);
        self
    }

    pub fn has_selection(&self) -> bool {
        self.anchor.is_some()
    }
}

/// Position in document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

impl Position {
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

/// History manager
pub struct HistoryManager {
    /// Undo stack
    undo_stack: Vec<Transaction>,
    /// Redo stack
    redo_stack: Vec<Transaction>,
    /// Current transaction (being built)
    current: Option<Transaction>,
    /// Next transaction ID
    next_id: u64,
    /// Maximum history size
    max_size: usize,
    /// Merge timeout (merge consecutive edits within this time)
    merge_timeout: Duration,
    /// Last edit time
    last_edit: Option<Instant>,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current: None,
            next_id: 1,
            max_size: 1000,
            merge_timeout: Duration::from_millis(500),
            last_edit: None,
        }
    }

    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    /// Begin a new transaction
    pub fn begin_transaction(&mut self) -> u64 {
        self.commit_current();
        let id = self.next_id;
        self.next_id += 1;
        self.current = Some(Transaction::new(id));
        id
    }

    /// Add change to current transaction
    pub fn add_change(&mut self, change: Change) {
        let now = Instant::now();
        
        // Check if we should merge with current or start new
        let should_merge = self.last_edit
            .map(|last| now.duration_since(last) < self.merge_timeout)
            .unwrap_or(false);

        if self.current.is_none() && !should_merge {
            self.begin_transaction();
        } else if self.current.is_none() && should_merge && !self.undo_stack.is_empty() {
            // Merge with last transaction
            if let Some(mut last) = self.undo_stack.pop() {
                last.add(change);
                self.undo_stack.push(last);
                self.last_edit = Some(now);
                return;
            }
        }

        if let Some(ref mut tx) = self.current {
            tx.add(change);
        }

        self.last_edit = Some(now);
        
        // Clear redo stack on new changes
        self.redo_stack.clear();
    }

    /// Commit current transaction
    pub fn commit_current(&mut self) {
        if let Some(tx) = self.current.take() {
            if !tx.is_empty() {
                self.undo_stack.push(tx);
                
                // Trim if over max size
                while self.undo_stack.len() > self.max_size {
                    self.undo_stack.remove(0);
                }
            }
        }
    }

    /// Undo last transaction
    pub fn undo(&mut self) -> Option<Transaction> {
        self.commit_current();
        
        if let Some(tx) = self.undo_stack.pop() {
            let inverse = tx.inverse();
            self.redo_stack.push(tx);
            Some(inverse)
        } else {
            None
        }
    }

    /// Redo last undone transaction
    pub fn redo(&mut self) -> Option<Transaction> {
        self.commit_current();
        
        if let Some(tx) = self.redo_stack.pop() {
            self.undo_stack.push(tx.clone());
            Some(tx)
        } else {
            None
        }
    }

    /// Can undo?
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty() || self.current.as_ref().map(|t| !t.is_empty()).unwrap_or(false)
    }

    /// Can redo?
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.current = None;
    }

    /// Get undo stack size
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len() + if self.current.as_ref().map(|t| !t.is_empty()).unwrap_or(false) { 1 } else { 0 }
    }

    /// Get redo stack size
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_redo() {
        let mut history = HistoryManager::new();
        
        history.begin_transaction();
        history.add_change(Change::text(TextChange::insert(Position::new(0, 0), "hello")));
        history.commit_current();
        
        assert!(history.can_undo());
        assert!(!history.can_redo());
        
        let undo_tx = history.undo().unwrap();
        assert_eq!(undo_tx.changes.len(), 1);
        
        assert!(!history.can_undo());
        assert!(history.can_redo());
        
        let redo_tx = history.redo().unwrap();
        assert_eq!(redo_tx.changes.len(), 1);
    }
}
