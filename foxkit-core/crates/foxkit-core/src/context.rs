//! Global application context
//! 
//! The context provides access to shared services and state
//! throughout the application. Inspired by React's Context API
//! but for a Rust application.

use std::sync::Arc;
use parking_lot::RwLock;

use crate::{EventEmitter, ServiceRegistry, Settings};

/// Global application context
/// 
/// This is the central nervous system of Foxkit - every component
/// can access shared services through this context.
pub struct Context {
    /// Application settings
    settings: Arc<RwLock<Settings>>,
    /// Event bus for pub/sub messaging
    event_bus: Arc<EventEmitter>,
    /// Service registry for dependency injection
    services: Arc<ServiceRegistry>,
    /// Current workspace path (if any)
    workspace_path: RwLock<Option<std::path::PathBuf>>,
}

impl Context {
    pub fn new(
        settings: Arc<RwLock<Settings>>,
        event_bus: Arc<EventEmitter>,
        services: Arc<ServiceRegistry>,
    ) -> Self {
        Self {
            settings,
            event_bus,
            services,
            workspace_path: RwLock::new(None),
        }
    }

    /// Get application settings
    pub fn settings(&self) -> &Arc<RwLock<Settings>> {
        &self.settings
    }

    /// Get the event bus
    pub fn events(&self) -> &Arc<EventEmitter> {
        &self.event_bus
    }

    /// Get the service registry
    pub fn services(&self) -> &Arc<ServiceRegistry> {
        &self.services
    }

    /// Get the current workspace path
    pub fn workspace_path(&self) -> Option<std::path::PathBuf> {
        self.workspace_path.read().clone()
    }

    /// Set the current workspace path
    pub fn set_workspace_path(&self, path: Option<std::path::PathBuf>) {
        *self.workspace_path.write() = path;
    }
}
