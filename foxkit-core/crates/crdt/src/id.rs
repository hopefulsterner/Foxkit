//! CRDT identifiers

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a replica (client)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplicaId(u64);

impl ReplicaId {
    pub fn new() -> Self {
        // Use random u64 as replica ID
        let uuid = Uuid::new_v4();
        let bytes = uuid.as_bytes();
        let id = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        Self(id)
    }

    pub fn from_u64(id: u64) -> Self {
        Self(id)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl Default for ReplicaId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for an item/character in the CRDT
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ItemId {
    /// Replica that created this item
    pub replica: ReplicaId,
    /// Sequence number within replica
    pub seq: u64,
}

impl ItemId {
    pub fn new(replica: ReplicaId, seq: u64) -> Self {
        Self { replica, seq }
    }

    /// Root ID (before first character)
    pub fn root() -> Self {
        Self {
            replica: ReplicaId(0),
            seq: 0,
        }
    }

    pub fn is_root(&self) -> bool {
        self.replica.0 == 0 && self.seq == 0
    }
}

impl PartialEq for ItemId {
    fn eq(&self, other: &Self) -> bool {
        self.replica == other.replica && self.seq == other.seq
    }
}

impl Eq for ItemId {}

impl Hash for ItemId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.replica.hash(state);
        self.seq.hash(state);
    }
}

impl Ord for ItemId {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare by sequence first, then by replica ID for tie-breaking
        match self.seq.cmp(&other.seq) {
            Ordering::Equal => self.replica.0.cmp(&other.replica.0),
            ord => ord,
        }
    }
}

impl PartialOrd for ItemId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Position identifier (for fractional indexing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Parent item
    pub parent: ItemId,
    /// Position relative to parent (left = true, right = false)
    pub side: Side,
}

impl Position {
    pub fn after(id: ItemId) -> Self {
        Self {
            parent: id,
            side: Side::Right,
        }
    }

    pub fn before(id: ItemId) -> Self {
        Self {
            parent: id,
            side: Side::Left,
        }
    }

    pub fn start() -> Self {
        Self {
            parent: ItemId::root(),
            side: Side::Right,
        }
    }
}

/// Side relative to parent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Left,
    Right,
}
