//! Text CRDT implementation

use std::collections::HashMap;
use crate::{ItemId, ReplicaId, Operation, TextOperation, StateVector};

/// A text CRDT (simplified RGA/YATA-style)
pub struct TextCrdt {
    /// Local replica ID
    replica_id: ReplicaId,
    /// Current sequence number
    seq: u64,
    /// Items in the document
    items: Vec<Item>,
    /// Index by ItemId
    index: HashMap<ItemId, usize>,
    /// Stored operations (for sync)
    operations: Vec<Operation>,
}

impl TextCrdt {
    pub fn new(replica_id: ReplicaId) -> Self {
        Self {
            replica_id,
            seq: 0,
            items: Vec::new(),
            index: HashMap::new(),
            operations: Vec::new(),
        }
    }

    /// Insert text at position
    pub fn insert(&mut self, position: usize, text: &str) -> Operation {
        // Find the item at position
        let left_id = if position == 0 {
            ItemId::root()
        } else {
            self.item_at_position(position - 1)
                .map(|i| i.id)
                .unwrap_or(ItemId::root())
        };

        let right_id = self.item_at_position(position).map(|i| i.id);

        // Create new item for each character
        // (In a real impl, we might batch characters)
        self.seq += 1;
        let id = ItemId::new(self.replica_id, self.seq);

        let item = Item {
            id,
            content: text.to_string(),
            origin_left: left_id,
            origin_right: right_id,
            deleted: false,
        };

        // Insert into items list
        let insert_pos = self.find_insert_position(left_id, right_id);
        self.items.insert(insert_pos, item);
        self.rebuild_index();

        let op = Operation::insert(id, text.to_string(), left_id, right_id);
        self.operations.push(op.clone());
        op
    }

    /// Delete text in range
    pub fn delete(&mut self, start: usize, end: usize) -> Operation {
        // Mark items as deleted (tombstones)
        let mut deleted_ids = Vec::new();
        let mut visible_pos = 0;

        for item in &mut self.items {
            if item.deleted {
                continue;
            }

            if visible_pos >= start && visible_pos < end {
                item.deleted = true;
                deleted_ids.push(item.id);
            }

            visible_pos += item.content.chars().count();
            if visible_pos >= end {
                break;
            }
        }

        // Create delete operation for first deleted item
        // (In real impl, would create operation for each)
        self.seq += 1;
        let id = ItemId::new(self.replica_id, self.seq);
        let target = deleted_ids.first().copied().unwrap_or(ItemId::root());

        let op = Operation::delete(id, target);
        self.operations.push(op.clone());
        op
    }

    /// Apply a remote operation
    pub fn apply(&mut self, op: &Operation) {
        match &op.op {
            TextOperation::Insert { content } => {
                // Check if already exists
                if self.index.contains_key(&op.id) {
                    return;
                }

                let item = Item {
                    id: op.id,
                    content: content.clone(),
                    origin_left: op.origin_left.unwrap_or(ItemId::root()),
                    origin_right: op.origin_right,
                    deleted: false,
                };

                let insert_pos = self.find_insert_position(item.origin_left, item.origin_right);
                self.items.insert(insert_pos, item);
                self.rebuild_index();
            }
            TextOperation::Delete { target } => {
                if let Some(&idx) = self.index.get(target) {
                    self.items[idx].deleted = true;
                }
            }
        }
    }

    /// Get operations since state vector
    pub fn operations_since(&self, since: &StateVector) -> Vec<Operation> {
        self.operations
            .iter()
            .filter(|op| !since.has_seen(&op.id))
            .cloned()
            .collect()
    }

    /// Get document as string
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        for item in &self.items {
            if !item.deleted {
                result.push_str(&item.content);
            }
        }
        result
    }

    /// Get length (visible characters)
    pub fn len(&self) -> usize {
        self.items
            .iter()
            .filter(|i| !i.deleted)
            .map(|i| i.content.chars().count())
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn item_at_position(&self, position: usize) -> Option<&Item> {
        let mut visible_pos = 0;
        for item in &self.items {
            if item.deleted {
                continue;
            }
            let item_len = item.content.chars().count();
            if visible_pos + item_len > position {
                return Some(item);
            }
            visible_pos += item_len;
        }
        None
    }

    fn find_insert_position(&self, left: ItemId, right: Option<ItemId>) -> usize {
        // Find position after left, before right
        let mut left_pos = None;
        let mut right_pos = None;

        for (i, item) in self.items.iter().enumerate() {
            if item.id == left {
                left_pos = Some(i);
            }
            if Some(item.id) == right {
                right_pos = Some(i);
            }
        }

        // Insert after left
        match (left_pos, right_pos) {
            (Some(l), Some(r)) if r > l => l + 1,
            (Some(l), _) => l + 1,
            (None, Some(r)) => r,
            (None, None) => self.items.len(),
        }
    }

    fn rebuild_index(&mut self) {
        self.index.clear();
        for (i, item) in self.items.iter().enumerate() {
            self.index.insert(item.id, i);
        }
    }
}

/// An item in the CRDT
#[derive(Debug, Clone)]
struct Item {
    id: ItemId,
    content: String,
    origin_left: ItemId,
    origin_right: Option<ItemId>,
    deleted: bool,
}
