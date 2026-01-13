//! SSH connections

use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{Connection, ConnectionState, ConnectionInfo, ConnectionType, ExecResult, RemoteFs};
use crate::fs::SftpFs;

/// SSH configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    /// Host to connect to
    pub host: String,
    /// Port (default 22)
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    /// Username
    pub username: String,
    /// Authentication method
    #[serde(default)]
    pub auth: SshAuth,
    /// Keep alive interval in seconds
    #[serde(default = "default_keepalive")]
    pub keepalive: u32,
    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u32,
}

fn default_ssh_port() -> u16 { 22 }
fn default_keepalive() -> u32 { 30 }
fn default_timeout() -> u32 { 10 }

/// SSH authentication
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "method")]
pub enum SshAuth {
    /// Use SSH agent
    #[default]
    Agent,
    /// Password authentication
    Password { password: String },
    /// Private key
    PrivateKey { 
        path: PathBuf,
        #[serde(default)]
        passphrase: Option<String>,
    },
}

/// SSH connection
pub struct SshConnection {
    id: String,
    config: SshConfig,
    state: RwLock<ConnectionState>,
    fs: SftpFs,
    // session: Arc<RwLock<Option<russh::client::Handle<SshHandler>>>>,
}

impl SshConnection {
    /// Connect to SSH server
    pub async fn connect(config: SshConfig) -> anyhow::Result<Self> {
        let id = format!("ssh-{}-{}", config.host, config.port);
        
        tracing::info!("Connecting to SSH: {}@{}:{}", config.username, config.host, config.port);

        // Create connection (simplified - real impl would use russh)
        let conn = Self {
            id,
            config: config.clone(),
            state: RwLock::new(ConnectionState::Connected),
            fs: SftpFs::new(&config.host, config.port),
        };

        Ok(conn)
    }

    /// Reconnect
    pub async fn reconnect(&self) -> anyhow::Result<()> {
        *self.state.write() = ConnectionState::Reconnecting;
        
        // Implement reconnection logic
        
        *self.state.write() = ConnectionState::Connected;
        Ok(())
    }
}

#[async_trait]
impl Connection for SshConnection {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.config.host
    }

    fn state(&self) -> ConnectionState {
        *self.state.read()
    }

    async fn disconnect(&self) -> anyhow::Result<()> {
        *self.state.write() = ConnectionState::Disconnected;
        tracing::info!("Disconnected from SSH: {}", self.config.host);
        Ok(())
    }

    async fn exec(&self, command: &str) -> anyhow::Result<ExecResult> {
        // Execute command over SSH
        // Simplified - real impl would use russh channel
        tracing::debug!("Executing: {}", command);
        
        Ok(ExecResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
        })
    }

    fn fs(&self) -> &dyn RemoteFs {
        &self.fs
    }

    async fn forward_port(&self, local: u16, remote: u16) -> anyhow::Result<()> {
        tracing::info!("Forwarding port {} -> {}", local, remote);
        // Implement port forwarding
        Ok(())
    }

    fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            host: self.config.host.clone(),
            port: self.config.port,
            username: Some(self.config.username.clone()),
            connection_type: ConnectionType::Ssh,
            latency_ms: None,
        }
    }
}

/// Parse SSH config file
pub fn parse_ssh_config(content: &str) -> Vec<SshConfig> {
    let mut configs = Vec::new();
    let mut current: Option<SshConfig> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() != 2 {
            continue;
        }

        let key = parts[0].to_lowercase();
        let value = parts[1].trim();

        match key.as_str() {
            "host" => {
                if let Some(cfg) = current.take() {
                    configs.push(cfg);
                }
                current = Some(SshConfig {
                    host: value.to_string(),
                    port: 22,
                    username: String::new(),
                    auth: SshAuth::Agent,
                    keepalive: 30,
                    timeout: 10,
                });
            }
            "hostname" => {
                if let Some(ref mut cfg) = current {
                    cfg.host = value.to_string();
                }
            }
            "port" => {
                if let Some(ref mut cfg) = current {
                    cfg.port = value.parse().unwrap_or(22);
                }
            }
            "user" => {
                if let Some(ref mut cfg) = current {
                    cfg.username = value.to_string();
                }
            }
            "identityfile" => {
                if let Some(ref mut cfg) = current {
                    cfg.auth = SshAuth::PrivateKey {
                        path: PathBuf::from(value),
                        passphrase: None,
                    };
                }
            }
            _ => {}
        }
    }

    if let Some(cfg) = current {
        configs.push(cfg);
    }

    configs
}
