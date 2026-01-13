//! CRDT operations

use serde::{Deserialize, Serialize};
use crate::{ItemId, ReplicaId};

/// A CRDT operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Unique ID for this operation
    pub id: ItemId,
    /// Operation type
    pub op: TextOperation,
    /// ID of item this is inserted after (for inserts)
    pub origin_left: Option<ItemId>,
    /// ID of item this was inserted before (for inserts)
    pub origin_right: Option<ItemId>,
}

impl Operation {
    pub fn insert(id: ItemId, content: String, left: ItemId, right: Option<ItemId>) -> Self {
        Self {
            id,
            op: TextOperation::Insert { content },
            origin_left: Some(left),
            origin_right: right,
        }
    }

    pub fn delete(id: ItemId, target: ItemId) -> Self {
        Self {
            id,
            op: TextOperation::Delete { target },
            origin_left: None,
            origin_right: None,
        }
    }
}

/// Text-specific operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextOperation {
    /// Insert text content
    Insert {
        content: String,
    },
    /// Delete a character/item
    Delete {
        target: ItemId,
    },
}

/// Operation batch for syncing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationBatch {
    /// Replica that generated these operations
    pub replica: ReplicaId,
    /// Operations in order
    pub operations: Vec<Operation>,
}

impl OperationBatch {
    pub fn new(replica: ReplicaId) -> Self {
        Self {
            replica,
            operations: Vec::new(),
        }
    }

    pub fn push(&mut self, op: Operation) {
        self.operations.push(op);
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub fn len(&self) -> usize {
        self.operations.len()
    }
}
