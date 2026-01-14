//! # Foxkit Collaboration
//! 
//! Real-time multi-user editing based on:
//! - CRDT (Conflict-free Replicated Data Types) for sync
//! - WebSocket for real-time communication
//! - Presence awareness (cursors, selections, activity)
//! 
//! Inspired by Zed's collaboration system

pub mod client;
pub mod presence;
pub mod protocol;
pub mod room;
pub mod session;
pub mod sync;

pub use session::{Session, SessionId, SessionManager, Participant, ParticipantId, ParticipantColor, ParticipantRole, SessionEvent};

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use anyhow::Result;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

pub use client::CollabClient;
pub use presence::{Presence, UserPresence, CursorPosition};
pub use protocol::{Message, Operation};
pub use room::{Room, RoomId};

/// Unique user identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

/// User information
#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub color: UserColor,
}

/// User's assigned color (for cursor, selection highlights)
#[derive(Debug, Clone, Copy)]
pub struct UserColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl UserColor {
    /// Generate a color from user ID
    pub fn from_user_id(id: UserId) -> Self {
        let bytes = id.0.as_bytes();
        // Use bytes to generate a pleasant color
        let hue = ((bytes[0] as u16 * 256 + bytes[1] as u16) % 360) as f32;
        let (r, g, b) = hsl_to_rgb(hue, 0.7, 0.5);
        Self { r, g, b }
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    
    let (r, g, b) = match (h as u32) / 60 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    
    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

/// Collaboration service
pub struct CollabService {
    /// Current user
    current_user: Option<User>,
    /// Active rooms
    rooms: RwLock<HashMap<RoomId, Arc<Room>>>,
    /// Collaboration client
    client: Option<CollabClient>,
}

impl CollabService {
    pub fn new() -> Self {
        Self {
            current_user: None,
            rooms: RwLock::new(HashMap::new()),
            client: None,
        }
    }

    /// Set current user
    pub fn set_user(&mut self, user: User) {
        self.current_user = Some(user);
    }

    /// Get current user
    pub fn current_user(&self) -> Option<&User> {
        self.current_user.as_ref()
    }

    /// Connect to collaboration server
    pub async fn connect(&mut self, server_url: &str) -> Result<()> {
        let user = self.current_user.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No user set"))?;
        
        let client = CollabClient::connect(server_url, user.id).await?;
        self.client = Some(client);
        
        Ok(())
    }

    /// Create a new room
    pub async fn create_room(&self, name: &str) -> Result<RoomId> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        
        let room_id = client.create_room(name).await?;
        
        let room = Room::new(room_id, name.to_string());
        self.rooms.write().insert(room_id, Arc::new(room));
        
        Ok(room_id)
    }

    /// Join an existing room
    pub async fn join_room(&self, room_id: RoomId) -> Result<Arc<Room>> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        
        let room_info = client.join_room(room_id).await?;
        
        let room = Arc::new(Room::from_info(room_info));
        self.rooms.write().insert(room_id, Arc::clone(&room));
        
        Ok(room)
    }

    /// Leave a room
    pub async fn leave_room(&self, room_id: RoomId) -> Result<()> {
        if let Some(client) = &self.client {
            client.leave_room(room_id).await?;
        }
        
        self.rooms.write().remove(&room_id);
        
        Ok(())
    }

    /// Get a room by ID
    pub fn get_room(&self, room_id: RoomId) -> Option<Arc<Room>> {
        self.rooms.read().get(&room_id).cloned()
    }

    /// List active rooms
    pub fn rooms(&self) -> Vec<Arc<Room>> {
        self.rooms.read().values().cloned().collect()
    }

    /// Share current file with room
    pub async fn share_file(&self, room_id: RoomId, file_path: &str) -> Result<()> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        
        client.share_file(room_id, file_path).await
    }

    /// Update cursor position
    pub async fn update_cursor(&self, room_id: RoomId, file: &str, position: CursorPosition) -> Result<()> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;
        
        client.update_cursor(room_id, file, position).await
    }

    /// Disconnect from server
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(client) = self.client.take() {
            client.disconnect().await?;
        }
        self.rooms.write().clear();
        Ok(())
    }

    /// Is connected?
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }
}

impl Default for CollabService {
    fn default() -> Self {
        Self::new()
    }
}

/// Collaboration event
#[derive(Debug, Clone)]
pub enum CollabEvent {
    /// User joined room
    UserJoined { room_id: RoomId, user: User },
    /// User left room
    UserLeft { room_id: RoomId, user_id: UserId },
    /// User cursor moved
    CursorMoved { room_id: RoomId, user_id: UserId, file: String, position: CursorPosition },
    /// User selection changed
    SelectionChanged { room_id: RoomId, user_id: UserId, file: String, start: usize, end: usize },
    /// File content changed
    FileChanged { room_id: RoomId, file: String, operations: Vec<Operation> },
    /// File shared
    FileShared { room_id: RoomId, file: String },
    /// Connection lost
    Disconnected { reason: String },
}
