//! Collaboration Session Management
//!
//! Manages collaborative editing sessions.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Session identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Participant in a session
#[derive(Debug, Clone)]
pub struct Participant {
    pub id: ParticipantId,
    pub name: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub color: ParticipantColor,
    pub role: ParticipantRole,
    pub status: ParticipantStatus,
}

/// Participant identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParticipantId(pub Uuid);

impl ParticipantId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ParticipantId {
    fn default() -> Self {
        Self::new()
    }
}

/// Participant assigned color
#[derive(Debug, Clone, Copy)]
pub struct ParticipantColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ParticipantColor {
    pub fn from_index(index: usize) -> Self {
        const COLORS: [(u8, u8, u8); 10] = [
            (66, 133, 244),   // Blue
            (234, 67, 53),    // Red
            (251, 188, 4),    // Yellow
            (52, 168, 83),    // Green
            (156, 39, 176),   // Purple
            (255, 87, 34),    // Orange
            (0, 188, 212),    // Cyan
            (233, 30, 99),    // Pink
            (63, 81, 181),    // Indigo
            (139, 195, 74),   // Light Green
        ];
        let (r, g, b) = COLORS[index % COLORS.len()];
        Self { r, g, b }
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn to_rgba(&self, alpha: f32) -> String {
        format!("rgba({}, {}, {}, {})", self.r, self.g, self.b, alpha)
    }
}

/// Participant role
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantRole {
    Owner,
    Editor,
    Viewer,
}

impl ParticipantRole {
    pub fn can_edit(&self) -> bool {
        matches!(self, Self::Owner | Self::Editor)
    }

    pub fn can_manage(&self) -> bool {
        matches!(self, Self::Owner)
    }
}

/// Participant status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantStatus {
    Active,
    Idle,
    Away,
    Offline,
}

/// Collaboration session
pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    participants: RwLock<HashMap<ParticipantId, Participant>>,
    shared_files: RwLock<Vec<SharedFile>>,
    event_tx: broadcast::Sender<SessionEvent>,
}

/// Shared file in a session
#[derive(Debug, Clone)]
pub struct SharedFile {
    pub path: String,
    pub language: String,
    pub shared_by: ParticipantId,
    pub shared_at: chrono::DateTime<chrono::Utc>,
}

/// Session events
#[derive(Debug, Clone)]
pub enum SessionEvent {
    ParticipantJoined(Participant),
    ParticipantLeft(ParticipantId),
    ParticipantUpdated(Participant),
    FileShared(SharedFile),
    FileUnshared(String),
    CursorMoved { participant: ParticipantId, file: String, line: u32, column: u32 },
    SelectionChanged { participant: ParticipantId, file: String, ranges: Vec<SelectionRange> },
    ChatMessage { participant: ParticipantId, message: String },
}

/// Selection range
#[derive(Debug, Clone)]
pub struct SelectionRange {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

impl Session {
    pub fn new(name: &str) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            id: SessionId::new(),
            name: name.to_string(),
            created_at: chrono::Utc::now(),
            participants: RwLock::new(HashMap::new()),
            shared_files: RwLock::new(Vec::new()),
            event_tx,
        }
    }

    /// Add a participant
    pub fn add_participant(&self, participant: Participant) {
        let event = SessionEvent::ParticipantJoined(participant.clone());
        self.participants.write().insert(participant.id, participant);
        let _ = self.event_tx.send(event);
    }

    /// Remove a participant
    pub fn remove_participant(&self, id: ParticipantId) {
        self.participants.write().remove(&id);
        let _ = self.event_tx.send(SessionEvent::ParticipantLeft(id));
    }

    /// Get participant by ID
    pub fn get_participant(&self, id: ParticipantId) -> Option<Participant> {
        self.participants.read().get(&id).cloned()
    }

    /// Get all participants
    pub fn participants(&self) -> Vec<Participant> {
        self.participants.read().values().cloned().collect()
    }

    /// Share a file
    pub fn share_file(&self, file: SharedFile) {
        let event = SessionEvent::FileShared(file.clone());
        self.shared_files.write().push(file);
        let _ = self.event_tx.send(event);
    }

    /// Unshare a file
    pub fn unshare_file(&self, path: &str) {
        self.shared_files.write().retain(|f| f.path != path);
        let _ = self.event_tx.send(SessionEvent::FileUnshared(path.to_string()));
    }

    /// Get shared files
    pub fn shared_files(&self) -> Vec<SharedFile> {
        self.shared_files.read().clone()
    }

    /// Subscribe to session events
    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.event_tx.subscribe()
    }

    /// Broadcast cursor movement
    pub fn broadcast_cursor(&self, participant: ParticipantId, file: &str, line: u32, column: u32) {
        let _ = self.event_tx.send(SessionEvent::CursorMoved {
            participant,
            file: file.to_string(),
            line,
            column,
        });
    }

    /// Broadcast selection change
    pub fn broadcast_selection(&self, participant: ParticipantId, file: &str, ranges: Vec<SelectionRange>) {
        let _ = self.event_tx.send(SessionEvent::SelectionChanged {
            participant,
            file: file.to_string(),
            ranges,
        });
    }

    /// Send chat message
    pub fn send_chat(&self, participant: ParticipantId, message: &str) {
        let _ = self.event_tx.send(SessionEvent::ChatMessage {
            participant,
            message: message.to_string(),
        });
    }
}

/// Session manager
pub struct SessionManager {
    sessions: RwLock<HashMap<SessionId, Arc<Session>>>,
    participant_sessions: RwLock<HashMap<ParticipantId, Vec<SessionId>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            participant_sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new session
    pub fn create_session(&self, name: &str, owner: Participant) -> Arc<Session> {
        let session = Arc::new(Session::new(name));
        session.add_participant(owner.clone());
        
        self.sessions.write().insert(session.id, Arc::clone(&session));
        self.participant_sessions.write()
            .entry(owner.id)
            .or_default()
            .push(session.id);
        
        session
    }

    /// Get session by ID
    pub fn get_session(&self, id: SessionId) -> Option<Arc<Session>> {
        self.sessions.read().get(&id).cloned()
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Vec<Arc<Session>> {
        self.sessions.read().values().cloned().collect()
    }

    /// Get sessions for a participant
    pub fn sessions_for_participant(&self, id: ParticipantId) -> Vec<Arc<Session>> {
        self.participant_sessions.read()
            .get(&id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.sessions.read().get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Close a session
    pub fn close_session(&self, id: SessionId) {
        if let Some(session) = self.sessions.write().remove(&id) {
            // Remove from participant mappings
            for participant in session.participants() {
                if let Some(sessions) = self.participant_sessions.write().get_mut(&participant.id) {
                    sessions.retain(|&sid| sid != id);
                }
            }
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
