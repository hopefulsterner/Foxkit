//! Remote SSH Connections for Foxkit
//!
//! Connect to remote machines via SSH for remote development.

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Connection identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub Uuid);
impl ConnectionId { pub fn new() -> Self { Self(Uuid::new_v4()) } }
impl Default for ConnectionId { fn default() -> Self { Self::new() } }

/// SSH host configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshHostConfig {
    pub name: String,
    pub hostname: String,
    pub port: u16,
    pub user: String,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
    pub forward_agent: bool,
    pub remote_platform: Platform,
}

impl Default for SshHostConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            hostname: String::new(),
            port: 22,
            user: String::new(),
            identity_file: None,
            proxy_jump: None,
            forward_agent: false,
            remote_platform: Platform::Linux,
        }
    }
}

/// Remote platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Platform { #[default] Linux, MacOS, Windows }

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState { Disconnected, Connecting, Connected, Reconnecting, Error }

/// SSH connection
#[derive(Debug)]
pub struct SshConnection {
    pub id: ConnectionId,
    pub config: SshHostConfig,
    pub state: ConnectionState,
    pub remote_home: Option<String>,
}

impl SshConnection {
    pub fn new(config: SshHostConfig) -> Self {
        Self { id: ConnectionId::new(), config, state: ConnectionState::Disconnected, remote_home: None }
    }
}

/// Port forwarding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForward {
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub label: Option<String>,
}

/// SSH connection trait
#[async_trait]
pub trait SshClient: Send + Sync {
    async fn connect(&self, config: &SshHostConfig) -> Result<ConnectionId, SshError>;
    async fn disconnect(&self, id: ConnectionId) -> Result<(), SshError>;
    async fn exec(&self, id: ConnectionId, command: &str) -> Result<ExecResult, SshError>;
    async fn forward_port(&self, id: ConnectionId, forward: PortForward) -> Result<(), SshError>;
}

/// Execution result
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// SSH error
#[derive(Debug, Clone)]
pub enum SshError {
    ConnectionFailed(String),
    AuthFailed,
    Timeout,
    HostKeyMismatch,
    ChannelError(String),
    Disconnected,
}

impl std::fmt::Display for SshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed(e) => write!(f, "Connection failed: {}", e),
            Self::AuthFailed => write!(f, "Authentication failed"),
            Self::Timeout => write!(f, "Connection timeout"),
            Self::HostKeyMismatch => write!(f, "Host key mismatch"),
            Self::ChannelError(e) => write!(f, "Channel error: {}", e),
            Self::Disconnected => write!(f, "Disconnected"),
        }
    }
}

impl std::error::Error for SshError {}

/// Remote SSH service
pub struct RemoteSshService {
    connections: RwLock<HashMap<ConnectionId, SshConnection>>,
    hosts: RwLock<Vec<SshHostConfig>>,
    forwards: RwLock<HashMap<ConnectionId, Vec<PortForward>>>,
}

impl RemoteSshService {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            hosts: RwLock::new(Vec::new()),
            forwards: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_host(&self, config: SshHostConfig) { self.hosts.write().push(config); }
    pub fn list_hosts(&self) -> Vec<SshHostConfig> { self.hosts.read().clone() }
    
    pub fn register_connection(&self, conn: SshConnection) -> ConnectionId {
        let id = conn.id;
        self.connections.write().insert(id, conn);
        id
    }

    pub fn get_connection(&self, id: ConnectionId) -> Option<ConnectionState> {
        self.connections.read().get(&id).map(|c| c.state)
    }

    pub fn add_port_forward(&self, conn_id: ConnectionId, forward: PortForward) {
        self.forwards.write().entry(conn_id).or_default().push(forward);
    }

    pub fn list_connections(&self) -> Vec<ConnectionId> {
        self.connections.read().keys().copied().collect()
    }
}

impl Default for RemoteSshService { fn default() -> Self { Self::new() } }
