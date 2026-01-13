//! Connection abstraction

use std::path::PathBuf;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{SshConfig, ContainerConfig, TunnelConfig, RemoteFs};

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConnectionConfig {
    Ssh(SshConfig),
    Container(ContainerConfig),
    Tunnel(TunnelConfig),
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

/// Connection trait
#[async_trait]
pub trait Connection: Send + Sync {
    /// Unique connection ID
    fn id(&self) -> &str;

    /// Connection name
    fn name(&self) -> &str;

    /// Current state
    fn state(&self) -> ConnectionState;

    /// Disconnect
    async fn disconnect(&self) -> anyhow::Result<()>;

    /// Execute a command
    async fn exec(&self, command: &str) -> anyhow::Result<ExecResult>;

    /// Get filesystem access
    fn fs(&self) -> &dyn RemoteFs;

    /// Forward a port
    async fn forward_port(&self, local: u16, remote: u16) -> anyhow::Result<()>;

    /// Get connection info
    fn info(&self) -> ConnectionInfo;
}

/// Command execution result
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl ExecResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Connection info
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub connection_type: ConnectionType,
    pub latency_ms: Option<u32>,
}

/// Connection type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    Ssh,
    Container,
    Tunnel,
    Wsl,
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ssh => write!(f, "SSH"),
            Self::Container => write!(f, "Container"),
            Self::Tunnel => write!(f, "Tunnel"),
            Self::Wsl => write!(f, "WSL"),
        }
    }
}
