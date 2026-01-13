//! User presence tracking

use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::{UserId, User, UserColor};

/// Cursor position in a file
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct CursorPosition {
    /// Line number (0-indexed)
    pub line: usize,
    /// Column number (0-indexed)
    pub column: usize,
    /// Byte offset
    pub offset: usize,
}

/// User selection in a file
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Selection {
    /// Start position
    pub start: CursorPosition,
    /// End position
    pub end: CursorPosition,
}

/// Per-user presence information
#[derive(Debug, Clone)]
pub struct UserPresence {
    /// User info
    pub user: User,
    /// Current file being edited
    pub active_file: Option<String>,
    /// Cursor position in active file
    pub cursor: Option<CursorPosition>,
    /// Selection in active file
    pub selection: Option<Selection>,
    /// User's activity status
    pub status: ActivityStatus,
    /// Last activity timestamp
    pub last_activity: Instant,
}

/// User activity status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityStatus {
    /// Actively editing
    Active,
    /// Idle (no recent activity)
    Idle,
    /// Away (explicitly set or long idle)
    Away,
    /// Offline
    Offline,
}

impl UserPresence {
    pub fn new(user: User) -> Self {
        Self {
            user,
            active_file: None,
            cursor: None,
            selection: None,
            status: ActivityStatus::Active,
            last_activity: Instant::now(),
        }
    }

    /// Update last activity
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
        if self.status != ActivityStatus::Away {
            self.status = ActivityStatus::Active;
        }
    }

    /// Check and update idle status
    pub fn check_idle(&mut self, idle_threshold: Duration, away_threshold: Duration) {
        let elapsed = self.last_activity.elapsed();
        
        if elapsed > away_threshold {
            self.status = ActivityStatus::Away;
        } else if elapsed > idle_threshold {
            self.status = ActivityStatus::Idle;
        }
    }

    /// Set cursor position
    pub fn set_cursor(&mut self, file: &str, cursor: CursorPosition) {
        self.active_file = Some(file.to_string());
        self.cursor = Some(cursor);
        self.selection = None;
        self.touch();
    }

    /// Set selection
    pub fn set_selection(&mut self, file: &str, selection: Selection) {
        self.active_file = Some(file.to_string());
        self.selection = Some(selection);
        self.cursor = Some(selection.end);
        self.touch();
    }
}

/// Presence manager - tracks all users in a room
pub struct Presence {
    /// All user presences
    users: HashMap<UserId, UserPresence>,
    /// Idle threshold
    idle_threshold: Duration,
    /// Away threshold
    away_threshold: Duration,
}

impl Presence {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            idle_threshold: Duration::from_secs(60),      // 1 minute
            away_threshold: Duration::from_secs(300),     // 5 minutes
        }
    }

    /// Add a user
    pub fn add_user(&mut self, user: User) {
        let presence = UserPresence::new(user.clone());
        self.users.insert(user.id, presence);
    }

    /// Remove a user
    pub fn remove_user(&mut self, user_id: UserId) {
        self.users.remove(&user_id);
    }

    /// Get user presence
    pub fn get(&self, user_id: UserId) -> Option<&UserPresence> {
        self.users.get(&user_id)
    }

    /// Get mutable user presence
    pub fn get_mut(&mut self, user_id: UserId) -> Option<&mut UserPresence> {
        self.users.get_mut(&user_id)
    }

    /// Update cursor for user
    pub fn update_cursor(&mut self, user_id: UserId, file: &str, cursor: CursorPosition) {
        if let Some(presence) = self.users.get_mut(&user_id) {
            presence.set_cursor(file, cursor);
        }
    }

    /// Update selection for user
    pub fn update_selection(&mut self, user_id: UserId, file: &str, selection: Selection) {
        if let Some(presence) = self.users.get_mut(&user_id) {
            presence.set_selection(file, selection);
        }
    }

    /// Get all users in a specific file
    pub fn users_in_file(&self, file: &str) -> Vec<&UserPresence> {
        self.users
            .values()
            .filter(|p| p.active_file.as_deref() == Some(file))
            .collect()
    }

    /// Get all active users
    pub fn active_users(&self) -> Vec<&UserPresence> {
        self.users
            .values()
            .filter(|p| p.status == ActivityStatus::Active)
            .collect()
    }

    /// Update idle status for all users
    pub fn tick(&mut self) {
        for presence in self.users.values_mut() {
            presence.check_idle(self.idle_threshold, self.away_threshold);
        }
    }

    /// Get all users
    pub fn all_users(&self) -> impl Iterator<Item = &UserPresence> {
        self.users.values()
    }

    /// Number of users
    pub fn count(&self) -> usize {
        self.users.len()
    }
}

impl Default for Presence {
    fn default() -> Self {
        Self::new()
    }
}
