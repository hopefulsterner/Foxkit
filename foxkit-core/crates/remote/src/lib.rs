//! # Foxkit Remote
//!
//! Remote development support with SSH, containers, and cloud.

pub mod connection;
pub mod container;
pub mod fs;
pub mod protocol;
pub mod ssh;
pub mod tunnel;

pub use protocol::{RemoteMessage, RemoteCodec, RequestTracker, Keepalive, ServerInfo, DirEntry};

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;

pub use connection::{Connection, ConnectionConfig, ConnectionState};
pub use ssh::{SshConnection, SshConfig};
pub use container::{ContainerConnection, ContainerConfig};
pub use tunnel::{Tunnel, TunnelConfig, PortForward};
pub use fs::RemoteFs;

/// Remote service
pub struct RemoteService {
    /// Active connections
    connections: RwLock<Vec<Arc<dyn Connection>>>,
    /// Event callbacks
    listeners: RwLock<Vec<Box<dyn Fn(&RemoteEvent) + Send + Sync>>>,
}

impl RemoteService {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(Vec::new()),
            listeners: RwLock::new(Vec::new()),
        }
    }

    /// Connect to a remote host
    pub async fn connect(&self, config: ConnectionConfig) -> anyhow::Result<Arc<dyn Connection>> {
        let conn: Arc<dyn Connection> = match config {
            ConnectionConfig::Ssh(cfg) => Arc::new(SshConnection::connect(cfg).await?),
            ConnectionConfig::Container(cfg) => Arc::new(ContainerConnection::connect(cfg).await?),
            ConnectionConfig::Tunnel(cfg) => Arc::new(Tunnel::connect(cfg).await?),
        };

        self.connections.write().push(conn.clone());
        self.emit(RemoteEvent::Connected(conn.id()));

        Ok(conn)
    }

    /// Disconnect from remote
    pub async fn disconnect(&self, connection_id: &str) -> anyhow::Result<()> {
        let mut connections = self.connections.write();
        
        if let Some(pos) = connections.iter().position(|c| c.id() == connection_id) {
            let conn = connections.remove(pos);
            conn.disconnect().await?;
            drop(connections);
            self.emit(RemoteEvent::Disconnected(connection_id.to_string()));
        }

        Ok(())
    }

    /// Get active connections
    pub fn connections(&self) -> Vec<Arc<dyn Connection>> {
        self.connections.read().clone()
    }

    /// Get connection by ID
    pub fn get(&self, id: &str) -> Option<Arc<dyn Connection>> {
        self.connections.read().iter().find(|c| c.id() == id).cloned()
    }

    /// Subscribe to events
    pub fn subscribe<F>(&self, callback: F)
    where
        F: Fn(&RemoteEvent) + Send + Sync + 'static,
    {
        self.listeners.write().push(Box::new(callback));
    }

    fn emit(&self, event: RemoteEvent) {
        for listener in self.listeners.read().iter() {
            listener(&event);
        }
    }
}

impl Default for RemoteService {
    fn default() -> Self {
        Self::new()
    }
}

/// Remote events
#[derive(Debug, Clone)]
pub enum RemoteEvent {
    Connected(String),
    Disconnected(String),
    Error(String, String),
    PortForwarded { local: u16, remote: u16 },
}

/// Remote host info
#[derive(Debug, Clone)]
pub struct RemoteHost {
    pub name: String,
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub os: Option<String>,
    pub arch: Option<String>,
}

impl RemoteHost {
    pub fn display_name(&self) -> String {
        format!("{}@{}:{}", self.username, self.hostname, self.port)
    }
}

/// Remote workspace
#[derive(Debug, Clone)]
pub struct RemoteWorkspace {
    pub connection_id: String,
    pub path: PathBuf,
    pub name: String,
}
