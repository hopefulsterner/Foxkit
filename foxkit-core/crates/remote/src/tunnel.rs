//! Tunnel connections

use std::net::SocketAddr;
use std::sync::Arc;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use crate::{Connection, ConnectionState, ConnectionInfo, ConnectionType, ExecResult, RemoteFs};
use crate::fs::HttpFs;

/// Tunnel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    /// Tunnel name
    pub name: String,
    /// Remote server URL
    pub url: String,
    /// Authentication token
    pub token: Option<String>,
    /// Port forwards
    #[serde(default)]
    pub forwards: Vec<PortForward>,
}

/// Port forward configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForward {
    /// Local port to bind
    pub local: u16,
    /// Remote port to forward to
    pub remote: u16,
    /// Bind address
    #[serde(default = "default_bind")]
    pub bind: String,
}

fn default_bind() -> String {
    "127.0.0.1".to_string()
}

/// Tunnel connection
pub struct Tunnel {
    id: String,
    config: TunnelConfig,
    state: RwLock<ConnectionState>,
    fs: HttpFs,
    forwards: RwLock<Vec<ActiveForward>>,
}

/// Active port forward
struct ActiveForward {
    config: PortForward,
    // handle: tokio::task::JoinHandle<()>,
}

impl Tunnel {
    /// Connect tunnel
    pub async fn connect(config: TunnelConfig) -> anyhow::Result<Self> {
        let id = format!("tunnel-{}", config.name);
        
        tracing::info!("Connecting tunnel: {}", config.name);

        let tunnel = Self {
            id,
            config: config.clone(),
            state: RwLock::new(ConnectionState::Connected),
            fs: HttpFs::new(&config.url),
            forwards: RwLock::new(Vec::new()),
        };

        // Start configured port forwards
        for forward in &config.forwards {
            tunnel.start_forward(forward.clone()).await?;
        }

        Ok(tunnel)
    }

    /// Start a port forward
    async fn start_forward(&self, forward: PortForward) -> anyhow::Result<()> {
        let addr: SocketAddr = format!("{}:{}", forward.bind, forward.local).parse()?;
        
        tracing::info!("Starting port forward: {} -> {}", addr, forward.remote);

        // Would spawn a task to handle forwarding
        self.forwards.write().push(ActiveForward {
            config: forward,
        });

        Ok(())
    }
}

#[async_trait]
impl Connection for Tunnel {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn state(&self) -> ConnectionState {
        *self.state.read()
    }

    async fn disconnect(&self) -> anyhow::Result<()> {
        *self.state.write() = ConnectionState::Disconnected;
        
        // Stop all forwards
        self.forwards.write().clear();
        
        tracing::info!("Disconnected tunnel: {}", self.config.name);
        Ok(())
    }

    async fn exec(&self, command: &str) -> anyhow::Result<ExecResult> {
        // Execute over HTTP API
        let client = reqwest::Client::new();
        
        let response = client
            .post(format!("{}/exec", self.config.url))
            .json(&serde_json::json!({ "command": command }))
            .send()
            .await?;

        if response.status().is_success() {
            let result: ExecResult = response.json().await?;
            Ok(result)
        } else {
            Ok(ExecResult {
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("HTTP error: {}", response.status()),
            })
        }
    }

    fn fs(&self) -> &dyn RemoteFs {
        &self.fs
    }

    async fn forward_port(&self, local: u16, remote: u16) -> anyhow::Result<()> {
        self.start_forward(PortForward {
            local,
            remote,
            bind: "127.0.0.1".to_string(),
        }).await
    }

    fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            host: self.config.url.clone(),
            port: 0,
            username: None,
            connection_type: ConnectionType::Tunnel,
            latency_ms: None,
        }
    }
}

/// Cloud development environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEnvironment {
    pub id: String,
    pub name: String,
    pub provider: CloudProvider,
    pub status: EnvironmentStatus,
    pub machine_type: Option<String>,
    pub region: Option<String>,
}

/// Cloud provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloudProvider {
    GitHubCodespaces,
    GitpodWorkspace,
    CloudflareWorkers,
    Custom,
}

/// Environment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}
