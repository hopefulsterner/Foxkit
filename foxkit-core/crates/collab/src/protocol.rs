//! Collaboration protocol messages

use serde::{Deserialize, Serialize};
use crate::{UserId, RoomId, CursorPosition};
use crate::room::RoomInfo;

/// Message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Client(ClientMessage),
    Server(ServerMessage),
}

/// Client -> Server messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Authenticate with server
    Auth { user_id: UserId },
    /// Create a new room
    CreateRoom { name: String },
    /// Join an existing room
    JoinRoom { room_id: RoomId },
    /// Leave a room
    LeaveRoom { room_id: RoomId },
    /// Share a file with room
    ShareFile { room_id: RoomId, file_path: String },
    /// Update cursor position
    CursorUpdate { room_id: RoomId, file: String, position: CursorPosition },
    /// Send an operation
    Operation { room_id: RoomId, file: String, operation: Operation },
    /// Request file sync
    RequestSync { room_id: RoomId, file: String },
    /// Disconnect
    Disconnect,
}

/// Server -> Client messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Authentication succeeded
    AuthSuccess,
    /// Room created
    RoomCreated { room_id: RoomId },
    /// Joined room
    RoomJoined { room_info: RoomInfo },
    /// User joined room
    UserJoined { room_id: RoomId, user_id: UserId, name: String },
    /// User left room
    UserLeft { room_id: RoomId, user_id: UserId },
    /// Cursor update from another user
    CursorUpdate { room_id: RoomId, user_id: UserId, file: String, position: CursorPosition },
    /// Operation from another user
    Operation { room_id: RoomId, user_id: UserId, file: String, operation: Operation },
    /// File sync response
    FileSync { room_id: RoomId, file: String, content: String, version: u64 },
    /// Error
    Error { message: String },
}

/// Text operation (for CRDT)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    /// Insert text at position
    Insert {
        /// Position in the document
        position: usize,
        /// Text to insert
        text: String,
        /// Lamport timestamp
        timestamp: u64,
        /// Author ID
        author: UserId,
    },
    /// Delete text
    Delete {
        /// Start position
        start: usize,
        /// End position
        end: usize,
        /// Lamport timestamp
        timestamp: u64,
        /// Author ID
        author: UserId,
    },
    /// Replace text
    Replace {
        /// Start position
        start: usize,
        /// End position
        end: usize,
        /// New text
        text: String,
        /// Lamport timestamp
        timestamp: u64,
        /// Author ID
        author: UserId,
    },
}

impl Operation {
    /// Get the timestamp of this operation
    pub fn timestamp(&self) -> u64 {
        match self {
            Operation::Insert { timestamp, .. } => *timestamp,
            Operation::Delete { timestamp, .. } => *timestamp,
            Operation::Replace { timestamp, .. } => *timestamp,
        }
    }

    /// Get the author of this operation
    pub fn author(&self) -> UserId {
        match self {
            Operation::Insert { author, .. } => *author,
            Operation::Delete { author, .. } => *author,
            Operation::Replace { author, .. } => *author,
        }
    }

    /// Transform this operation against another
    /// (for operational transformation)
    pub fn transform(&self, other: &Operation) -> Operation {
        // Simplified OT - real implementation would be more complex
        match (self, other) {
            (Operation::Insert { position, text, timestamp, author },
             Operation::Insert { position: other_pos, text: other_text, .. }) => {
                let new_pos = if *other_pos <= *position {
                    position + other_text.len()
                } else {
                    *position
                };
                Operation::Insert {
                    position: new_pos,
                    text: text.clone(),
                    timestamp: *timestamp,
                    author: *author,
                }
            }
            // ... other transformations
            _ => self.clone(),
        }
    }
}
