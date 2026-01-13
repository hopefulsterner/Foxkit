//! # Foxkit CRDT
//!
//! Conflict-free replicated data types for real-time collaboration.
//! Implements a simplified CRDT for text editing (similar to Yjs/Automerge).

pub mod id;
pub mod text;
pub mod operation;
pub mod state;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub use id::{ItemId, ReplicaId};
pub use text::TextCrdt;
pub use operation::{Operation, TextOperation};
pub use state::{CrdtState, StateVector};

/// A CRDT document
pub struct Document {
    /// Document ID
    pub id: String,
    /// Local replica ID
    pub replica_id: ReplicaId,
    /// Text content CRDT
    pub text: TextCrdt,
    /// State vector (for sync)
    pub state: StateVector,
    /// Pending operations (not yet synced)
    pending: Vec<Operation>,
}

impl Document {
    /// Create new document
    pub fn new(id: &str, replica_id: ReplicaId) -> Self {
        Self {
            id: id.to_string(),
            replica_id,
            text: TextCrdt::new(replica_id),
            state: StateVector::new(),
            pending: Vec::new(),
        }
    }

    /// Insert text at position
    pub fn insert(&mut self, position: usize, text: &str) -> Operation {
        let op = self.text.insert(position, text);
        self.state.increment(self.replica_id);
        self.pending.push(op.clone());
        op
    }

    /// Delete text at range
    pub fn delete(&mut self, start: usize, end: usize) -> Operation {
        let op = self.text.delete(start, end);
        self.state.increment(self.replica_id);
        self.pending.push(op.clone());
        op
    }

    /// Apply remote operation
    pub fn apply(&mut self, op: Operation) -> bool {
        if self.state.has_seen(&op.id) {
            return false; // Already applied
        }
        
        self.text.apply(&op);
        self.state.update_from_op(&op);
        true
    }

    /// Get document text
    pub fn content(&self) -> String {
        self.text.to_string()
    }

    /// Get pending operations (for sync)
    pub fn take_pending(&mut self) -> Vec<Operation> {
        std::mem::take(&mut self.pending)
    }

    /// Get state vector for sync
    pub fn state_vector(&self) -> &StateVector {
        &self.state
    }

    /// Generate operations since state vector
    pub fn diff(&self, since: &StateVector) -> Vec<Operation> {
        self.text.operations_since(since)
    }

    /// Merge another document's state
    pub fn merge(&mut self, ops: Vec<Operation>) {
        for op in ops {
            self.apply(op);
        }
    }
}

/// Awareness information (cursor, selection, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Awareness {
    /// Replica ID
    pub replica_id: ReplicaId,
    /// User info
    pub user: Option<UserInfo>,
    /// Cursor position
    pub cursor: Option<CursorState>,
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub name: String,
    pub color: String,
}

/// Cursor state for awareness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorState {
    /// Anchor position (start of selection)
    pub anchor: usize,
    /// Head position (cursor position)
    pub head: usize,
}

impl CursorState {
    pub fn new(position: usize) -> Self {
        Self {
            anchor: position,
            head: position,
        }
    }

    pub fn selection(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    pub fn has_selection(&self) -> bool {
        self.anchor != self.head
    }

    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concurrent_edits() {
        let replica1 = ReplicaId::new();
        let replica2 = ReplicaId::new();
        
        let mut doc1 = Document::new("test", replica1);
        let mut doc2 = Document::new("test", replica2);
        
        // Initial content
        let op1 = doc1.insert(0, "Hello");
        doc2.apply(op1);
        
        // Concurrent edits
        let op2 = doc1.insert(5, " World");
        let op3 = doc2.insert(5, "!");
        
        // Cross-apply
        doc1.apply(op3.clone());
        doc2.apply(op2.clone());
        
        // Both should have same content (convergence)
        assert_eq!(doc1.content(), doc2.content());
    }
}
