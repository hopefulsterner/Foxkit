//! # Foxkit Workspace Trust
//!
//! Workspace security and trust management.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Workspace trust service
pub struct WorkspaceTrustService {
    /// Trusted folders
    trusted: RwLock<HashSet<PathBuf>>,
    /// Untrusted folders
    untrusted: RwLock<HashSet<PathBuf>>,
    /// Current workspace trust state
    state: RwLock<WorkspaceTrustState>,
    /// Events
    events: broadcast::Sender<WorkspaceTrustEvent>,
    /// Configuration
    config: RwLock<WorkspaceTrustConfig>,
}

impl WorkspaceTrustService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            trusted: RwLock::new(HashSet::new()),
            untrusted: RwLock::new(HashSet::new()),
            state: RwLock::new(WorkspaceTrustState::Unknown),
            events,
            config: RwLock::new(WorkspaceTrustConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<WorkspaceTrustEvent> {
        self.events.subscribe()
    }

    /// Configure trust settings
    pub fn configure(&self, config: WorkspaceTrustConfig) {
        *self.config.write() = config;
    }

    /// Check if workspace is trusted
    pub fn is_trusted(&self) -> bool {
        *self.state.read() == WorkspaceTrustState::Trusted
    }

    /// Get current trust state
    pub fn state(&self) -> WorkspaceTrustState {
        *self.state.read()
    }

    /// Check trust for workspace folders
    pub fn check_trust(&self, folders: &[PathBuf]) -> WorkspaceTrustState {
        let config = self.config.read();

        // If trust is disabled, everything is trusted
        if !config.enabled {
            return WorkspaceTrustState::Trusted;
        }

        let trusted = self.trusted.read();
        let untrusted = self.untrusted.read();

        // Check each folder
        for folder in folders {
            // Check if explicitly untrusted
            if untrusted.contains(folder) {
                return WorkspaceTrustState::Untrusted;
            }

            // Check if explicitly trusted (or parent is trusted)
            let is_trusted = trusted.iter().any(|t| folder.starts_with(t));
            
            if !is_trusted {
                // Check if in trusted home folders
                let in_trusted_home = config.trusted_folders.iter()
                    .any(|t| folder.starts_with(t));

                if !in_trusted_home {
                    return WorkspaceTrustState::Unknown;
                }
            }
        }

        WorkspaceTrustState::Trusted
    }

    /// Set workspace folders and evaluate trust
    pub fn set_workspace_folders(&self, folders: Vec<PathBuf>) {
        let state = self.check_trust(&folders);
        let old_state = *self.state.read();
        
        *self.state.write() = state;

        if state != old_state {
            let _ = self.events.send(WorkspaceTrustEvent::StateChanged { state });
        }
    }

    /// Grant trust to folder
    pub fn grant_trust(&self, folder: PathBuf) {
        self.untrusted.write().remove(&folder);
        self.trusted.write().insert(folder.clone());

        let _ = self.events.send(WorkspaceTrustEvent::TrustGranted { folder });

        // Re-evaluate state
        let state = WorkspaceTrustState::Trusted;
        *self.state.write() = state;
        let _ = self.events.send(WorkspaceTrustEvent::StateChanged { state });
    }

    /// Grant trust to parent folder
    pub fn grant_trust_to_parent(&self, folder: &PathBuf) {
        if let Some(parent) = folder.parent() {
            self.grant_trust(parent.to_path_buf());
        }
    }

    /// Revoke trust from folder
    pub fn revoke_trust(&self, folder: PathBuf) {
        self.trusted.write().remove(&folder);
        self.untrusted.write().insert(folder.clone());

        let _ = self.events.send(WorkspaceTrustEvent::TrustRevoked { folder });

        // Re-evaluate state
        let state = WorkspaceTrustState::Untrusted;
        *self.state.write() = state;
        let _ = self.events.send(WorkspaceTrustEvent::StateChanged { state });
    }

    /// Get trusted folders
    pub fn trusted_folders(&self) -> Vec<PathBuf> {
        self.trusted.read().iter().cloned().collect()
    }

    /// Get untrusted folders
    pub fn untrusted_folders(&self) -> Vec<PathBuf> {
        self.untrusted.read().iter().cloned().collect()
    }

    /// Check if feature requires trust
    pub fn requires_trust(&self, feature: &str) -> bool {
        let config = self.config.read();
        config.untrusted_features.contains(&feature.to_string())
    }

    /// Get restricted features in untrusted mode
    pub fn restricted_features(&self) -> Vec<String> {
        if self.is_trusted() {
            Vec::new()
        } else {
            self.config.read().untrusted_features.clone()
        }
    }
}

impl Default for WorkspaceTrustService {
    fn default() -> Self {
        Self::new()
    }
}

/// Workspace trust state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceTrustState {
    /// Workspace is trusted
    Trusted,
    /// Workspace is not trusted
    Untrusted,
    /// Trust state unknown (needs user decision)
    Unknown,
}

impl WorkspaceTrustState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trusted => "trusted",
            Self::Untrusted => "untrusted",
            Self::Unknown => "unknown",
        }
    }
}

/// Workspace trust configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceTrustConfig {
    /// Enable workspace trust
    pub enabled: bool,
    /// Trusted folders (always trusted)
    pub trusted_folders: Vec<PathBuf>,
    /// Empty window trust state
    pub empty_window: EmptyWindowTrust,
    /// Features restricted in untrusted mode
    pub untrusted_features: Vec<String>,
    /// Show trust banner
    pub show_banner: bool,
    /// Startup prompt
    pub startup_prompt: StartupPrompt,
}

impl Default for WorkspaceTrustConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            trusted_folders: Vec::new(),
            empty_window: EmptyWindowTrust::Inherit,
            untrusted_features: vec![
                "terminal".to_string(),
                "tasks".to_string(),
                "debugging".to_string(),
                "extensions".to_string(),
                "git.autofetch".to_string(),
            ],
            show_banner: true,
            startup_prompt: StartupPrompt::Once,
        }
    }
}

/// Empty window trust behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmptyWindowTrust {
    /// Trust empty windows
    Trust,
    /// Don't trust empty windows
    Untrust,
    /// Inherit from last workspace
    Inherit,
}

/// Startup prompt behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartupPrompt {
    /// Always prompt
    Always,
    /// Prompt once per workspace
    Once,
    /// Never prompt
    Never,
}

/// Workspace trust event
#[derive(Debug, Clone)]
pub enum WorkspaceTrustEvent {
    StateChanged { state: WorkspaceTrustState },
    TrustGranted { folder: PathBuf },
    TrustRevoked { folder: PathBuf },
}

/// Trust request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustRequest {
    /// Folders requiring trust
    pub folders: Vec<PathBuf>,
    /// Request message
    pub message: String,
    /// Restricted features
    pub restricted: Vec<RestrictedFeature>,
}

impl TrustRequest {
    pub fn new(folders: Vec<PathBuf>) -> Self {
        Self {
            folders,
            message: String::new(),
            restricted: Vec::new(),
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn with_restricted(mut self, restricted: Vec<RestrictedFeature>) -> Self {
        self.restricted = restricted;
        self
    }
}

/// Restricted feature info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestrictedFeature {
    /// Feature ID
    pub id: String,
    /// Display label
    pub label: String,
    /// Description
    pub description: String,
    /// Icon
    pub icon: Option<String>,
}

impl RestrictedFeature {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: String::new(),
            icon: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

/// Trust editor info
pub struct TrustEditorInfo {
    /// Current trust state
    pub state: WorkspaceTrustState,
    /// Workspace folders
    pub folders: Vec<TrustFolderInfo>,
    /// Restricted features
    pub restricted: Vec<RestrictedFeature>,
}

/// Trust folder info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustFolderInfo {
    /// Folder path
    pub path: PathBuf,
    /// Folder name
    pub name: String,
    /// Is trusted
    pub trusted: bool,
    /// Trust origin
    pub origin: TrustOrigin,
}

/// Trust origin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustOrigin {
    /// Explicitly trusted
    Explicit,
    /// Trusted via parent
    Parent,
    /// Trusted via config
    Config,
    /// Not trusted
    None,
}

/// Builtin restricted features
pub fn builtin_restricted_features() -> Vec<RestrictedFeature> {
    vec![
        RestrictedFeature::new("terminal", "Terminal")
            .with_description("Running terminal commands"),
        RestrictedFeature::new("tasks", "Tasks")
            .with_description("Running build and debug tasks"),
        RestrictedFeature::new("debugging", "Debugging")
            .with_description("Starting debug sessions"),
        RestrictedFeature::new("extensions", "Extensions")
            .with_description("Running extension code"),
        RestrictedFeature::new("git.autofetch", "Git Auto Fetch")
            .with_description("Automatic git fetch operations"),
    ]
}
