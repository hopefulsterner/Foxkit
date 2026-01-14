//! Document synchronization using CRDT

use std::collections::HashMap;
use crate::{UserId, RoomId};
use crate::protocol::Operation;

/// Document sync state
pub struct DocumentSync {
    /// Document content
    content: String,
    /// Current version
    version: u64,
    /// Pending local operations
    pending: Vec<Operation>,
    /// Applied operations log
    history: Vec<Operation>,
    /// Local Lamport timestamp
    timestamp: u64,
    /// Local user ID
    user_id: UserId,
}

impl DocumentSync {
    pub fn new(user_id: UserId) -> Self {
        Self {
            content: String::new(),
            version: 0,
            pending: Vec::new(),
            history: Vec::new(),
            timestamp: 0,
            user_id,
        }
    }

    pub fn with_content(user_id: UserId, content: String) -> Self {
        Self {
            content,
            version: 0,
            pending: Vec::new(),
            history: Vec::new(),
            timestamp: 0,
            user_id,
        }
    }

    /// Get current content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get current version
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Apply a local insert
    pub fn local_insert(&mut self, position: usize, text: &str) -> Operation {
        self.timestamp += 1;
        
        let op = Operation::Insert {
            position,
            text: text.to_string(),
            timestamp: self.timestamp,
            author: self.user_id,
        };
        
        // Apply locally
        self.apply_operation(&op);
        
        // Add to pending
        self.pending.push(op.clone());
        
        op
    }

    /// Apply a local delete
    pub fn local_delete(&mut self, start: usize, end: usize) -> Operation {
        self.timestamp += 1;
        
        let op = Operation::Delete {
            start,
            end,
            timestamp: self.timestamp,
            author: self.user_id,
        };
        
        // Apply locally
        self.apply_operation(&op);
        
        // Add to pending
        self.pending.push(op.clone());
        
        op
    }

    /// Receive a remote operation
    pub fn receive_operation(&mut self, op: Operation) {
        // Update local timestamp
        self.timestamp = self.timestamp.max(op.timestamp()) + 1;
        
        // Transform against pending operations
        let mut transformed_op = op;
        for pending in &self.pending {
            transformed_op = transformed_op.transform(pending);
        }
        
        // Apply the transformed operation
        self.apply_operation(&transformed_op);
        
        // Add to history
        self.history.push(transformed_op);
    }

    /// Acknowledge that server received our operations
    pub fn acknowledge(&mut self, count: usize) {
        // Move acknowledged operations from pending to history
        for _ in 0..count.min(self.pending.len()) {
            let op = self.pending.remove(0);
            self.history.push(op);
        }
    }

    fn apply_operation(&mut self, op: &Operation) {
        match op {
            Operation::Insert { position, text, .. } => {
                let pos = (*position).min(self.content.len());
                self.content.insert_str(pos, text);
            }
            Operation::Delete { start, end, .. } => {
                let start = (*start).min(self.content.len());
                let end = (*end).min(self.content.len());
                if start < end {
                    self.content.replace_range(start..end, "");
                }
            }
            Operation::Replace { start, end, text, .. } => {
                let start = (*start).min(self.content.len());
                let end = (*end).min(self.content.len());
                if start <= end {
                    self.content.replace_range(start..end, text);
                }
            }
        }
        
        self.version += 1;
    }

    /// Sync with server state
    pub fn sync(&mut self, content: String, version: u64) {
        self.content = content;
        self.version = version;
        self.pending.clear();
    }

    /// Check if there are pending operations
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Get pending operations
    pub fn pending_operations(&self) -> &[Operation] {
        &self.pending
    }
}

/// Multi-document sync manager
pub struct SyncManager {
    /// Per-document sync state
    documents: HashMap<String, DocumentSync>,
    /// User ID
    user_id: UserId,
}

impl SyncManager {
    pub fn new(user_id: UserId) -> Self {
        Self {
            documents: HashMap::new(),
            user_id,
        }
    }

    /// Get or create document sync
    pub fn get_or_create(&mut self, file: &str) -> &mut DocumentSync {
        self.documents
            .entry(file.to_string())
            .or_insert_with(|| DocumentSync::new(self.user_id))
    }

    /// Get document sync
    pub fn get(&self, file: &str) -> Option<&DocumentSync> {
        self.documents.get(file)
    }

    /// Get mutable document sync
    pub fn get_mut(&mut self, file: &str) -> Option<&mut DocumentSync> {
        self.documents.get_mut(file)
    }

    /// Remove document
    pub fn remove(&mut self, file: &str) {
        self.documents.remove(file);
    }

    /// List all documents
    pub fn documents(&self) -> Vec<&str> {
        self.documents.keys().map(|s| s.as_str()).collect()
    }
}
