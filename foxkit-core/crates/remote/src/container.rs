//! Container connections

use std::sync::Arc;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{Connection, ConnectionState, ConnectionInfo, ConnectionType, ExecResult, RemoteFs};
use crate::fs::ContainerFs;

/// Container configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Container ID or name
    pub container: String,
    /// Container runtime
    #[serde(default)]
    pub runtime: ContainerRuntime,
    /// Working directory
    pub workdir: Option<String>,
    /// User to run as
    pub user: Option<String>,
}

/// Container runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ContainerRuntime {
    #[default]
    Docker,
    Podman,
    Containerd,
}

impl ContainerRuntime {
    pub fn command(&self) -> &str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
            Self::Containerd => "ctr",
        }
    }
}

/// Container connection
pub struct ContainerConnection {
    id: String,
    config: ContainerConfig,
    state: RwLock<ConnectionState>,
    fs: ContainerFs,
}

impl ContainerConnection {
    /// Connect to container
    pub async fn connect(config: ContainerConfig) -> anyhow::Result<Self> {
        let id = format!("container-{}", config.container);
        
        tracing::info!("Connecting to container: {}", config.container);

        // Verify container is running
        let output = tokio::process::Command::new(config.runtime.command())
            .args(["inspect", "--format", "{{.State.Running}}", &config.container])
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("Container not found: {}", config.container);
        }

        let running = String::from_utf8_lossy(&output.stdout).trim() == "true";
        if !running {
            anyhow::bail!("Container is not running: {}", config.container);
        }

        let conn = Self {
            id,
            config: config.clone(),
            state: RwLock::new(ConnectionState::Connected),
            fs: ContainerFs::new(&config.container, config.runtime),
        };

        Ok(conn)
    }
}

#[async_trait]
impl Connection for ContainerConnection {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.config.container
    }

    fn state(&self) -> ConnectionState {
        *self.state.read()
    }

    async fn disconnect(&self) -> anyhow::Result<()> {
        *self.state.write() = ConnectionState::Disconnected;
        tracing::info!("Disconnected from container: {}", self.config.container);
        Ok(())
    }

    async fn exec(&self, command: &str) -> anyhow::Result<ExecResult> {
        let mut args = vec!["exec"];
        
        if let Some(ref user) = self.config.user {
            args.push("-u");
            args.push(user);
        }
        
        if let Some(ref workdir) = self.config.workdir {
            args.push("-w");
            args.push(workdir);
        }
        
        args.push(&self.config.container);
        args.push("sh");
        args.push("-c");
        args.push(command);

        let output = tokio::process::Command::new(self.config.runtime.command())
            .args(&args)
            .output()
            .await?;

        Ok(ExecResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    fn fs(&self) -> &dyn RemoteFs {
        &self.fs
    }

    async fn forward_port(&self, local: u16, remote: u16) -> anyhow::Result<()> {
        // Containers use network directly, not port forwarding
        tracing::warn!("Port forwarding not applicable for containers");
        Ok(())
    }

    fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            host: self.config.container.clone(),
            port: 0,
            username: self.config.user.clone(),
            connection_type: ConnectionType::Container,
            latency_ms: Some(0),
        }
    }
}

/// List available containers
pub async fn list_containers(runtime: ContainerRuntime) -> anyhow::Result<Vec<ContainerInfo>> {
    let output = tokio::process::Command::new(runtime.command())
        .args(["ps", "--format", "{{.ID}}|{{.Names}}|{{.Image}}|{{.Status}}"])
        .output()
        .await?;

    if !output.status.success() {
        anyhow::bail!("Failed to list containers");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let containers = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                Some(ContainerInfo {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    image: parts[2].to_string(),
                    status: parts[3].to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(containers)
}

/// Container info
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
}
