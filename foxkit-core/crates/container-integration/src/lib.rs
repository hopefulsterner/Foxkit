//! Docker and Devcontainer Integration for Foxkit
//!
//! Development containers, Docker management, and container-based workflows.

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Container identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContainerId(pub Uuid);
impl ContainerId { pub fn new() -> Self { Self(Uuid::new_v4()) } }
impl Default for ContainerId { fn default() -> Self { Self::new() } }

/// Devcontainer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevcontainerConfig {
    pub name: Option<String>,
    pub image: Option<String>,
    pub dockerfile: Option<String>,
    pub docker_compose_file: Option<String>,
    pub service: Option<String>,
    pub workspace_folder: Option<String>,
    pub remote_user: Option<String>,
    pub features: HashMap<String, serde_json::Value>,
    pub customizations: HashMap<String, serde_json::Value>,
    pub forward_ports: Vec<u16>,
    pub post_create_command: Option<String>,
    pub post_start_command: Option<String>,
}

impl Default for DevcontainerConfig {
    fn default() -> Self {
        Self {
            name: None, image: None, dockerfile: None, docker_compose_file: None,
            service: None, workspace_folder: None, remote_user: None,
            features: HashMap::new(), customizations: HashMap::new(),
            forward_ports: Vec::new(), post_create_command: None, post_start_command: None,
        }
    }
}

/// Container state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerState { Created, Running, Paused, Stopped, Removing, Dead }

/// Container info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: ContainerState,
    pub ports: Vec<PortBinding>,
    pub mounts: Vec<Mount>,
    pub labels: HashMap<String, String>,
}

/// Port binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortBinding {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: String,
}

/// Volume mount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    pub source: String,
    pub destination: String,
    pub read_only: bool,
}

/// Docker image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub id: String,
    pub tags: Vec<String>,
    pub size: u64,
    pub created: String,
}

/// Container runtime trait
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerInfo>, ContainerError>;
    async fn create_container(&self, config: &CreateContainerConfig) -> Result<String, ContainerError>;
    async fn start_container(&self, id: &str) -> Result<(), ContainerError>;
    async fn stop_container(&self, id: &str, timeout: Option<u32>) -> Result<(), ContainerError>;
    async fn remove_container(&self, id: &str, force: bool) -> Result<(), ContainerError>;
    async fn exec_in_container(&self, id: &str, cmd: &[&str]) -> Result<ExecResult, ContainerError>;
    async fn list_images(&self) -> Result<Vec<ImageInfo>, ContainerError>;
    async fn pull_image(&self, image: &str) -> Result<(), ContainerError>;
}

/// Container creation config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateContainerConfig {
    pub image: String,
    pub name: Option<String>,
    pub cmd: Vec<String>,
    pub env: HashMap<String, String>,
    pub ports: Vec<PortBinding>,
    pub mounts: Vec<Mount>,
    pub working_dir: Option<String>,
    pub labels: HashMap<String, String>,
}

/// Exec result
#[derive(Debug, Clone)]
pub struct ExecResult { pub stdout: String, pub stderr: String, pub exit_code: i32 }

/// Container error
#[derive(Debug, Clone)]
pub enum ContainerError {
    NotFound(String), AlreadyExists(String), RuntimeError(String),
    ImageNotFound(String), PermissionDenied, Timeout, NotRunning,
}

impl std::fmt::Display for ContainerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "Container not found: {}", id),
            Self::AlreadyExists(n) => write!(f, "Container exists: {}", n),
            Self::RuntimeError(e) => write!(f, "Runtime error: {}", e),
            Self::ImageNotFound(i) => write!(f, "Image not found: {}", i),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::Timeout => write!(f, "Operation timeout"),
            Self::NotRunning => write!(f, "Container not running"),
        }
    }
}

impl std::error::Error for ContainerError {}

/// Container integration service
pub struct ContainerService {
    devcontainer_configs: RwLock<HashMap<String, DevcontainerConfig>>,
    active_container: RwLock<Option<String>>,
}

impl ContainerService {
    pub fn new() -> Self {
        Self { devcontainer_configs: RwLock::new(HashMap::new()), active_container: RwLock::new(None) }
    }

    pub fn load_devcontainer(&self, workspace: &str, config: DevcontainerConfig) {
        self.devcontainer_configs.write().insert(workspace.to_string(), config);
    }

    pub fn get_devcontainer(&self, workspace: &str) -> Option<DevcontainerConfig> {
        self.devcontainer_configs.read().get(workspace).cloned()
    }

    pub fn set_active_container(&self, id: Option<String>) { *self.active_container.write() = id; }
    pub fn active_container(&self) -> Option<String> { self.active_container.read().clone() }
}

impl Default for ContainerService { fn default() -> Self { Self::new() } }
