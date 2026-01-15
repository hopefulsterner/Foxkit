//! Host functions for WASM plugins

use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;

use crate::PluginManifest;

/// Host context passed to WASM instances
pub struct HostContext {
    /// Plugin manifest
    pub manifest: PluginManifest,
    /// Plugin data directory
    pub data_dir: PathBuf,
    /// Registered callbacks
    callbacks: RwLock<HashMap<String, Box<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>>>,
    /// Environment variables
    env: HashMap<String, String>,
}

impl HostContext {
    pub fn new(manifest: PluginManifest) -> Self {
        let data_dir = std::env::temp_dir()
            .join("foxkit-plugins")
            .join(&manifest.id);
        
        std::fs::create_dir_all(&data_dir).ok();

        Self {
            manifest,
            data_dir,
            callbacks: RwLock::new(HashMap::new()),
            env: HashMap::new(),
        }
    }

    /// Set environment variable for plugin
    pub fn set_env(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.env.insert(key.into(), value.into());
    }

    /// Get environment variable
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env.get(key)
    }

    /// Register a callback
    pub fn register_callback<F>(&self, name: impl Into<String>, callback: F)
    where
        F: Fn(&[u8]) -> Vec<u8> + Send + Sync + 'static,
    {
        self.callbacks.write().insert(name.into(), Box::new(callback));
    }

    /// Invoke a callback
    pub fn invoke_callback(&self, name: &str, args: &[u8]) -> Option<Vec<u8>> {
        self.callbacks.read().get(name).map(|cb| cb(args))
    }
}

/// Host functions exposed to plugins
pub trait HostFunctions {
    /// Log a message
    fn log(&self, level: LogLevel, message: &str);

    /// Read a file
    fn read_file(&self, path: &str) -> Result<Vec<u8>, HostError>;

    /// Write a file
    fn write_file(&self, path: &str, data: &[u8]) -> Result<(), HostError>;

    /// List directory
    fn list_dir(&self, path: &str) -> Result<Vec<String>, HostError>;

    /// Get configuration value
    fn get_config(&self, key: &str) -> Option<String>;

    /// Set configuration value
    fn set_config(&self, key: &str, value: &str) -> Result<(), HostError>;

    /// Show notification
    fn show_notification(&self, message: &str, kind: NotificationKind);

    /// Get active editor path
    fn get_active_file(&self) -> Option<String>;

    /// Insert text at cursor
    fn insert_text(&self, text: &str) -> Result<(), HostError>;

    /// Get selection
    fn get_selection(&self) -> Option<String>;

    /// Execute command
    fn execute_command(&self, command: &str, args: &[&str]) -> Result<(), HostError>;

    /// HTTP GET request
    fn http_get(&self, url: &str) -> Result<Vec<u8>, HostError>;

    /// HTTP POST request
    fn http_post(&self, url: &str, body: &[u8]) -> Result<Vec<u8>, HostError>;
}

/// Log level for host logging
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Notification kind
#[derive(Debug, Clone, Copy)]
pub enum NotificationKind {
    Info,
    Warning,
    Error,
}

/// Host error
#[derive(Debug, Clone, thiserror::Error)]
pub enum HostError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("File not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

/// Host function implementation for Foxkit
pub struct FoxkitHostFunctions {
    context: HostContext,
}

impl FoxkitHostFunctions {
    pub fn new(context: HostContext) -> Self {
        Self { context }
    }

    /// Check if path is allowed
    fn check_path_permission(&self, path: &str) -> Result<PathBuf, HostError> {
        let path = PathBuf::from(path);
        
        // Only allow access within plugin data directory or workspace
        if path.starts_with(&self.context.data_dir) {
            Ok(path)
        } else {
            Err(HostError::PermissionDenied(format!(
                "Access to {} not allowed",
                path.display()
            )))
        }
    }
}

impl HostFunctions for FoxkitHostFunctions {
    fn log(&self, level: LogLevel, message: &str) {
        let plugin_id = &self.context.manifest.id;
        match level {
            LogLevel::Trace => tracing::trace!(plugin_id = %plugin_id, "{}", message),
            LogLevel::Debug => tracing::debug!(plugin_id = %plugin_id, "{}", message),
            LogLevel::Info => tracing::info!(plugin_id = %plugin_id, "{}", message),
            LogLevel::Warn => tracing::warn!(plugin_id = %plugin_id, "{}", message),
            LogLevel::Error => tracing::error!(plugin_id = %plugin_id, "{}", message),
        }
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, HostError> {
        let path = self.check_path_permission(path)?;
        std::fs::read(&path).map_err(|e| HostError::Io(e.to_string()))
    }

    fn write_file(&self, path: &str, data: &[u8]) -> Result<(), HostError> {
        let path = self.check_path_permission(path)?;
        std::fs::write(&path, data).map_err(|e| HostError::Io(e.to_string()))
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, HostError> {
        let path = self.check_path_permission(path)?;
        
        let entries = std::fs::read_dir(&path)
            .map_err(|e| HostError::Io(e.to_string()))?
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        
        Ok(entries)
    }

    fn get_config(&self, key: &str) -> Option<String> {
        self.context.get_env(key).cloned()
    }

    fn set_config(&self, _key: &str, _value: &str) -> Result<(), HostError> {
        // Config is read-only for plugins
        Err(HostError::PermissionDenied("Config is read-only".to_string()))
    }

    fn show_notification(&self, message: &str, kind: NotificationKind) {
        tracing::info!(
            plugin = %self.context.manifest.id,
            kind = ?kind,
            "notification: {}",
            message
        );
    }

    fn get_active_file(&self) -> Option<String> {
        // Would get from editor service
        None
    }

    fn insert_text(&self, _text: &str) -> Result<(), HostError> {
        // Would insert via editor service
        Ok(())
    }

    fn get_selection(&self) -> Option<String> {
        // Would get from editor service
        None
    }

    fn execute_command(&self, _command: &str, _args: &[&str]) -> Result<(), HostError> {
        // Would execute via command service
        Ok(())
    }

    fn http_get(&self, _url: &str) -> Result<Vec<u8>, HostError> {
        // Would need network permission check
        Err(HostError::PermissionDenied("Network access requires permission".to_string()))
    }

    fn http_post(&self, _url: &str, _body: &[u8]) -> Result<Vec<u8>, HostError> {
        Err(HostError::PermissionDenied("Network access requires permission".to_string()))
    }
}
