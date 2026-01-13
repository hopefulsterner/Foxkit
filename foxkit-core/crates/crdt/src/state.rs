//! CRDT state management

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::{ItemId, ReplicaId};

/// State vector for sync
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateVector {
    /// Latest sequence seen from each replica
    clocks: HashMap<ReplicaId, u64>,
}

impl StateVector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment clock for replica
    pub fn increment(&mut self, replica: ReplicaId) {
        let clock = self.clocks.entry(replica).or_insert(0);
        *clock += 1;
    }

    /// Get clock for replica
    pub fn get(&self, replica: ReplicaId) -> u64 {
        self.clocks.get(&replica).copied().unwrap_or(0)
    }

    /// Update from operation
    pub fn update_from_op(&mut self, op: &crate::Operation) {
        let current = self.clocks.entry(op.id.replica).or_insert(0);
        *current = (*current).max(op.id.seq);
    }

    /// Check if operation has been seen
    pub fn has_seen(&self, id: &ItemId) -> bool {
        self.get(id.replica) >= id.seq
    }

    /// Merge with another state vector
    pub fn merge(&mut self, other: &StateVector) {
        for (&replica, &seq) in &other.clocks {
            let current = self.clocks.entry(replica).or_insert(0);
            *current = (*current).max(seq);
        }
    }

    /// Calculate difference (what other is missing)
    pub fn diff(&self, other: &StateVector) -> StateVector {
        let mut diff = StateVector::new();
        
        for (&replica, &seq) in &self.clocks {
            let other_seq = other.get(replica);
            if seq > other_seq {
                diff.clocks.insert(replica, seq);
            }
        }
        
        diff
    }

    /// Get all replicas
    pub fn replicas(&self) -> Vec<ReplicaId> {
        self.clocks.keys().copied().collect()
    }
}

/// CRDT document state (for serialization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtState {
    /// Document ID
    pub doc_id: String,
    /// State vector
    pub state_vector: StateVector,
    /// Encoded operations
    pub operations: Vec<u8>,
}

impl CrdtState {
    pub fn new(doc_id: &str) -> Self {
        Self {
            doc_id: doc_id.to_string(),
            state_vector: StateVector::new(),
            operations: Vec::new(),
        }
    }
}

/// Sync message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Request sync (send state vector)
    SyncRequest {
        state_vector: StateVector,
    },
    /// Sync response (operations since state)
    SyncResponse {
        operations: Vec<crate::Operation>,
    },
    /// Awareness update
    Awareness {
        replica: ReplicaId,
        state: Vec<u8>,
    },
}

impl SyncMessage {
    pub fn request(sv: StateVector) -> Self {
        SyncMessage::SyncRequest { state_vector: sv }
    }

    pub fn response(ops: Vec<crate::Operation>) -> Self {
        SyncMessage::SyncResponse { operations: ops }
    }
}
