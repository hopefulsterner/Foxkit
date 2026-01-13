//! # Foxkit Extension Host
//!
//! WASM-based extension runtime with:
//! - Sandboxed execution
//! - VS Code API compatibility layer
//! - Permission-based security
//! - Extension marketplace integration
//!
//! Inspired by VS Code/Theia extension system + WASM for safety

pub mod api;
pub mod loader;
pub mod manifest;
pub mod permission;
pub mod runtime;
pub mod sandbox;

use std::sync::Arc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use parking_lot::RwLock;
use anyhow::Result;
use uuid::Uuid;

pub use manifest::{ExtensionManifest, ExtensionKind, Contribution};
pub use permission::{Permission, PermissionSet, PermissionRequest};
pub use runtime::ExtensionRuntime;

/// Extension identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExtensionId {
    pub publisher: String,
    pub name: String,
}

impl ExtensionId {
    pub fn new(publisher: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            publisher: publisher.into(),
            name: name.into(),
        }
    }

    pub fn from_string(id: &str) -> Option<Self> {
        let parts: Vec<&str> = id.split('.').collect();
        if parts.len() >= 2 {
            Some(Self {
                publisher: parts[0].to_string(),
                name: parts[1..].join("."),
            })
        } else {
            None
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}.{}", self.publisher, self.name)
    }
}

impl std::fmt::Display for ExtensionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.publisher, self.name)
    }
}

/// Extension state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionState {
    /// Not loaded
    Unloaded,
    /// Loading
    Loading,
    /// Active and running
    Active,
    /// Disabled by user
    Disabled,
    /// Error during activation
    Error,
}

/// Extension instance
pub struct Extension {
    /// Extension ID
    pub id: ExtensionId,
    /// Manifest
    pub manifest: ExtensionManifest,
    /// Installation path
    pub path: PathBuf,
    /// Current state
    state: RwLock<ExtensionState>,
    /// Granted permissions
    permissions: RwLock<PermissionSet>,
    /// Runtime instance
    runtime: RwLock<Option<ExtensionRuntime>>,
}

impl Extension {
    pub fn new(manifest: ExtensionManifest, path: PathBuf) -> Self {
        let id = ExtensionId::new(&manifest.publisher, &manifest.name);
        
        Self {
            id,
            manifest,
            path,
            state: RwLock::new(ExtensionState::Unloaded),
            permissions: RwLock::new(PermissionSet::new()),
            runtime: RwLock::new(None),
        }
    }

    pub fn state(&self) -> ExtensionState {
        *self.state.read()
    }

    pub fn set_state(&self, state: ExtensionState) {
        *self.state.write() = state;
    }

    pub fn is_active(&self) -> bool {
        self.state() == ExtensionState::Active
    }

    pub fn grant_permission(&self, permission: Permission) {
        self.permissions.write().grant(permission);
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.read().has(permission)
    }

    pub fn required_permissions(&self) -> Vec<Permission> {
        self.manifest.permissions.clone()
    }
}

/// Extension host - manages all extensions
pub struct ExtensionHost {
    /// Loaded extensions
    extensions: RwLock<HashMap<ExtensionId, Arc<Extension>>>,
    /// Extension search paths
    search_paths: Vec<PathBuf>,
    /// Built-in extensions path
    builtin_path: Option<PathBuf>,
    /// User extensions path
    user_path: Option<PathBuf>,
    /// Extension event handlers
    handlers: RwLock<Vec<Box<dyn Fn(&ExtensionEvent) + Send + Sync>>>,
}

impl ExtensionHost {
    pub fn new() -> Self {
        Self {
            extensions: RwLock::new(HashMap::new()),
            search_paths: Vec::new(),
            builtin_path: None,
            user_path: None,
            handlers: RwLock::new(Vec::new()),
        }
    }

    /// Set built-in extensions path
    pub fn set_builtin_path(&mut self, path: PathBuf) {
        self.builtin_path = Some(path.clone());
        self.search_paths.push(path);
    }

    /// Set user extensions path
    pub fn set_user_path(&mut self, path: PathBuf) {
        self.user_path = Some(path.clone());
        self.search_paths.push(path);
    }

    /// Add extension search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Discover extensions in search paths
    pub async fn discover(&self) -> Result<Vec<ExtensionManifest>> {
        let mut manifests = Vec::new();
        
        for search_path in &self.search_paths {
            if search_path.exists() {
                let discovered = loader::discover_extensions(search_path).await?;
                manifests.extend(discovered);
            }
        }
        
        Ok(manifests)
    }

    /// Load an extension
    pub async fn load(&self, manifest: ExtensionManifest, path: PathBuf) -> Result<Arc<Extension>> {
        let extension = Arc::new(Extension::new(manifest, path));
        let id = extension.id.clone();
        
        self.extensions.write().insert(id.clone(), Arc::clone(&extension));
        
        self.emit(ExtensionEvent::Loaded { id });
        
        Ok(extension)
    }

    /// Activate an extension
    pub async fn activate(&self, id: &ExtensionId) -> Result<()> {
        let extension = self.get(id)
            .ok_or_else(|| anyhow::anyhow!("Extension not found: {}", id))?;
        
        extension.set_state(ExtensionState::Loading);
        
        // Check permissions
        let required = extension.required_permissions();
        for perm in required {
            if !extension.has_permission(&perm) {
                self.emit(ExtensionEvent::PermissionRequest {
                    id: id.clone(),
                    permission: perm,
                });
            }
        }
        
        // Create runtime
        let runtime = runtime::ExtensionRuntime::new(&extension).await?;
        *extension.runtime.write() = Some(runtime);
        
        // Activate
        if let Some(runtime) = extension.runtime.read().as_ref() {
            runtime.activate().await?;
        }
        
        extension.set_state(ExtensionState::Active);
        
        self.emit(ExtensionEvent::Activated { id: id.clone() });
        
        Ok(())
    }

    /// Deactivate an extension
    pub async fn deactivate(&self, id: &ExtensionId) -> Result<()> {
        let extension = self.get(id)
            .ok_or_else(|| anyhow::anyhow!("Extension not found: {}", id))?;
        
        // Deactivate runtime
        if let Some(runtime) = extension.runtime.read().as_ref() {
            runtime.deactivate().await?;
        }
        
        *extension.runtime.write() = None;
        extension.set_state(ExtensionState::Unloaded);
        
        self.emit(ExtensionEvent::Deactivated { id: id.clone() });
        
        Ok(())
    }

    /// Unload an extension
    pub async fn unload(&self, id: &ExtensionId) -> Result<()> {
        // Deactivate first if active
        if let Some(ext) = self.get(id) {
            if ext.is_active() {
                self.deactivate(id).await?;
            }
        }
        
        self.extensions.write().remove(id);
        
        self.emit(ExtensionEvent::Unloaded { id: id.clone() });
        
        Ok(())
    }

    /// Get an extension
    pub fn get(&self, id: &ExtensionId) -> Option<Arc<Extension>> {
        self.extensions.read().get(id).cloned()
    }

    /// Get all extensions
    pub fn all(&self) -> Vec<Arc<Extension>> {
        self.extensions.read().values().cloned().collect()
    }

    /// Get active extensions
    pub fn active(&self) -> Vec<Arc<Extension>> {
        self.extensions.read()
            .values()
            .filter(|e| e.is_active())
            .cloned()
            .collect()
    }

    /// Register event handler
    pub fn on_event(&self, handler: impl Fn(&ExtensionEvent) + Send + Sync + 'static) {
        self.handlers.write().push(Box::new(handler));
    }

    fn emit(&self, event: ExtensionEvent) {
        for handler in self.handlers.read().iter() {
            handler(&event);
        }
    }

    /// Get extensions by contribution type
    pub fn with_contribution(&self, kind: ContributionKind) -> Vec<Arc<Extension>> {
        self.extensions.read()
            .values()
            .filter(|e| e.manifest.has_contribution(kind))
            .cloned()
            .collect()
    }
}

impl Default for ExtensionHost {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension event
#[derive(Debug, Clone)]
pub enum ExtensionEvent {
    Loaded { id: ExtensionId },
    Activated { id: ExtensionId },
    Deactivated { id: ExtensionId },
    Unloaded { id: ExtensionId },
    Error { id: ExtensionId, message: String },
    PermissionRequest { id: ExtensionId, permission: Permission },
}

/// Contribution kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContributionKind {
    Commands,
    Languages,
    Grammars,
    Themes,
    Snippets,
    Keybindings,
    Views,
    Debuggers,
    TaskProviders,
}
