//! Collaboration room management

use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::{UserId, User};
use crate::presence::Presence;

/// Room identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomId(pub Uuid);

impl RoomId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RoomId {
    fn default() -> Self {
        Self::new()
    }
}

/// Room information (for joining)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub id: RoomId,
    pub name: String,
    pub users: Vec<UserInfo>,
    pub shared_files: Vec<String>,
}

/// Basic user info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: UserId,
    pub name: String,
}

/// Collaboration room
pub struct Room {
    /// Room ID
    pub id: RoomId,
    /// Room name
    pub name: String,
    /// Users in room
    users: RwLock<HashMap<UserId, User>>,
    /// Shared files
    shared_files: RwLock<HashSet<String>>,
    /// User presence
    presence: RwLock<Presence>,
    /// File versions
    file_versions: RwLock<HashMap<String, u64>>,
}

impl Room {
    pub fn new(id: RoomId, name: String) -> Self {
        Self {
            id,
            name,
            users: RwLock::new(HashMap::new()),
            shared_files: RwLock::new(HashSet::new()),
            presence: RwLock::new(Presence::new()),
            file_versions: RwLock::new(HashMap::new()),
        }
    }

    pub fn from_info(info: RoomInfo) -> Self {
        let room = Self::new(info.id, info.name);
        
        // Add users
        for user_info in info.users {
            let user = User {
                id: user_info.id,
                name: user_info.name,
                email: None,
                avatar_url: None,
                color: crate::UserColor::from_user_id(user_info.id),
            };
            room.add_user(user);
        }
        
        // Add shared files
        for file in info.shared_files {
            room.shared_files.write().insert(file);
        }
        
        room
    }

    /// Add a user to the room
    pub fn add_user(&self, user: User) {
        self.presence.write().add_user(user.clone());
        self.users.write().insert(user.id, user);
    }

    /// Remove a user from the room
    pub fn remove_user(&self, user_id: UserId) {
        self.users.write().remove(&user_id);
        self.presence.write().remove_user(user_id);
    }

    /// Get a user
    pub fn get_user(&self, user_id: UserId) -> Option<User> {
        self.users.read().get(&user_id).cloned()
    }

    /// Get all users
    pub fn users(&self) -> Vec<User> {
        self.users.read().values().cloned().collect()
    }

    /// Number of users
    pub fn user_count(&self) -> usize {
        self.users.read().len()
    }

    /// Share a file
    pub fn share_file(&self, path: &str) {
        self.shared_files.write().insert(path.to_string());
        self.file_versions.write().insert(path.to_string(), 0);
    }

    /// Unshare a file
    pub fn unshare_file(&self, path: &str) {
        self.shared_files.write().remove(path);
        self.file_versions.write().remove(path);
    }

    /// Is file shared?
    pub fn is_file_shared(&self, path: &str) -> bool {
        self.shared_files.read().contains(path)
    }

    /// Get shared files
    pub fn shared_files(&self) -> Vec<String> {
        self.shared_files.read().iter().cloned().collect()
    }

    /// Get presence
    pub fn presence(&self) -> &RwLock<Presence> {
        &self.presence
    }

    /// Get file version
    pub fn file_version(&self, path: &str) -> Option<u64> {
        self.file_versions.read().get(path).copied()
    }

    /// Increment file version
    pub fn increment_file_version(&self, path: &str) -> u64 {
        let mut versions = self.file_versions.write();
        let version = versions.entry(path.to_string()).or_insert(0);
        *version += 1;
        *version
    }

    /// Get room info
    pub fn info(&self) -> RoomInfo {
        RoomInfo {
            id: self.id,
            name: self.name.clone(),
            users: self.users.read()
                .values()
                .map(|u| UserInfo { id: u.id, name: u.name.clone() })
                .collect(),
            shared_files: self.shared_files(),
        }
    }
}
