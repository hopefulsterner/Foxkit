//! Remote Development Protocols
//!
//! Protocol definitions for remote development.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Remote protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RemoteMessage {
    // Connection
    Hello { version: String, capabilities: Vec<String> },
    HelloAck { session_id: String, server_info: ServerInfo },
    Ping { seq: u64 },
    Pong { seq: u64 },
    Bye { reason: String },

    // File operations
    FileRead { path: String, request_id: u64 },
    FileReadResponse { request_id: u64, content: Option<String>, error: Option<String> },
    FileWrite { path: String, content: String, request_id: u64 },
    FileWriteResponse { request_id: u64, success: bool, error: Option<String> },
    FileWatch { paths: Vec<String>, request_id: u64 },
    FileWatchResponse { request_id: u64, watch_id: u64 },
    FileChange { watch_id: u64, path: String, change_type: FileChangeType },
    
    // Directory operations
    DirList { path: String, request_id: u64 },
    DirListResponse { request_id: u64, entries: Vec<DirEntry>, error: Option<String> },
    DirCreate { path: String, request_id: u64 },
    DirCreateResponse { request_id: u64, success: bool, error: Option<String> },
    
    // Process operations
    ProcessSpawn { command: String, args: Vec<String>, cwd: Option<String>, env: HashMap<String, String>, request_id: u64 },
    ProcessSpawnResponse { request_id: u64, process_id: Option<u64>, error: Option<String> },
    ProcessStdin { process_id: u64, data: Vec<u8> },
    ProcessStdout { process_id: u64, data: Vec<u8> },
    ProcessStderr { process_id: u64, data: Vec<u8> },
    ProcessExit { process_id: u64, code: Option<i32> },
    ProcessKill { process_id: u64 },

    // Terminal
    TerminalCreate { shell: Option<String>, cwd: Option<String>, env: HashMap<String, String>, request_id: u64 },
    TerminalCreateResponse { request_id: u64, terminal_id: Option<u64>, error: Option<String> },
    TerminalResize { terminal_id: u64, rows: u16, cols: u16 },
    TerminalInput { terminal_id: u64, data: Vec<u8> },
    TerminalOutput { terminal_id: u64, data: Vec<u8> },
    TerminalClose { terminal_id: u64 },

    // Port forwarding
    PortForwardRequest { remote_port: u16, local_port: Option<u16>, request_id: u64 },
    PortForwardResponse { request_id: u64, local_port: Option<u16>, error: Option<String> },
    PortForwardClose { local_port: u16 },
    PortForwardData { local_port: u16, data: Vec<u8> },
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub version: String,
    pub home_dir: String,
}

/// File change type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FileChangeType {
    Created,
    Changed,
    Deleted,
}

/// Directory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub size: Option<u64>,
    pub modified: Option<u64>,
}

/// Protocol codec for framing messages
pub struct RemoteCodec {
    max_frame_size: usize,
}

impl RemoteCodec {
    pub fn new() -> Self {
        Self {
            max_frame_size: 16 * 1024 * 1024, // 16MB max
        }
    }

    /// Encode a message to bytes
    pub fn encode(&self, msg: &RemoteMessage) -> anyhow::Result<Vec<u8>> {
        let json = serde_json::to_vec(msg)?;
        if json.len() > self.max_frame_size {
            anyhow::bail!("Message too large: {} bytes", json.len());
        }
        
        // Frame format: [length: 4 bytes][json payload]
        let mut frame = Vec::with_capacity(4 + json.len());
        frame.extend_from_slice(&(json.len() as u32).to_be_bytes());
        frame.extend_from_slice(&json);
        Ok(frame)
    }

    /// Decode a message from bytes
    pub fn decode(&self, data: &[u8]) -> anyhow::Result<(RemoteMessage, usize)> {
        if data.len() < 4 {
            anyhow::bail!("Incomplete frame header");
        }
        
        let length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        
        if length > self.max_frame_size {
            anyhow::bail!("Frame too large: {} bytes", length);
        }
        
        if data.len() < 4 + length {
            anyhow::bail!("Incomplete frame body");
        }
        
        let msg: RemoteMessage = serde_json::from_slice(&data[4..4 + length])?;
        Ok((msg, 4 + length))
    }
}

impl Default for RemoteCodec {
    fn default() -> Self {
        Self::new()
    }
}

/// Request/response tracker
pub struct RequestTracker {
    next_id: std::sync::atomic::AtomicU64,
    pending: parking_lot::RwLock<HashMap<u64, tokio::sync::oneshot::Sender<RemoteMessage>>>,
}

impl RequestTracker {
    pub fn new() -> Self {
        Self {
            next_id: std::sync::atomic::AtomicU64::new(1),
            pending: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Get next request ID
    pub fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Register a pending request
    pub fn register(&self, id: u64) -> tokio::sync::oneshot::Receiver<RemoteMessage> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.write().insert(id, tx);
        rx
    }

    /// Complete a pending request
    pub fn complete(&self, id: u64, response: RemoteMessage) {
        if let Some(tx) = self.pending.write().remove(&id) {
            let _ = tx.send(response);
        }
    }

    /// Cancel all pending requests
    pub fn cancel_all(&self) {
        self.pending.write().clear();
    }
}

impl Default for RequestTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection keepalive
pub struct Keepalive {
    interval_ms: u64,
    timeout_ms: u64,
    last_ping: std::sync::atomic::AtomicU64,
    last_pong: std::sync::atomic::AtomicU64,
}

impl Keepalive {
    pub fn new(interval_ms: u64, timeout_ms: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        
        Self {
            interval_ms,
            timeout_ms,
            last_ping: std::sync::atomic::AtomicU64::new(now),
            last_pong: std::sync::atomic::AtomicU64::new(now),
        }
    }

    /// Check if we should send a ping
    pub fn should_ping(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let last = self.last_ping.load(std::sync::atomic::Ordering::SeqCst);
        now - last >= self.interval_ms
    }

    /// Record that we sent a ping
    pub fn record_ping(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.last_ping.store(now, std::sync::atomic::Ordering::SeqCst);
    }

    /// Record that we received a pong
    pub fn record_pong(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.last_pong.store(now, std::sync::atomic::Ordering::SeqCst);
    }

    /// Check if connection has timed out
    pub fn is_timed_out(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let last_pong = self.last_pong.load(std::sync::atomic::Ordering::SeqCst);
        now - last_pong > self.timeout_ms
    }
}
