//! LSP manager - coordinates multiple language servers

use std::sync::Arc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use parking_lot::RwLock;
use tokio::sync::mpsc;
use anyhow::Result;

use crate::{ServerConfig, LspClient, LspEvent, servers};

/// Manages multiple language servers
pub struct LspManager {
    /// Registered server configurations
    configs: RwLock<HashMap<String, ServerConfig>>,
    /// Active clients (by language ID)
    clients: RwLock<HashMap<String, Arc<tokio::sync::RwLock<LspClient>>>>,
    /// Event channel
    event_tx: mpsc::UnboundedSender<LspEvent>,
    /// Event receiver (for consumers)
    event_rx: RwLock<Option<mpsc::UnboundedReceiver<LspEvent>>>,
    /// Workspace roots
    roots: RwLock<Vec<PathBuf>>,
}

impl LspManager {
    /// Create new manager
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let manager = Self {
            configs: RwLock::new(HashMap::new()),
            clients: RwLock::new(HashMap::new()),
            event_tx,
            event_rx: RwLock::new(Some(event_rx)),
            roots: RwLock::new(Vec::new()),
        };

        // Register built-in servers
        for config in servers::all() {
            manager.register_server(config);
        }

        manager
    }

    /// Register a server configuration
    pub fn register_server(&self, config: ServerConfig) {
        self.configs.write().insert(config.language_id.clone(), config);
    }

    /// Add a workspace root
    pub fn add_root(&self, path: PathBuf) {
        self.roots.write().push(path);
    }

    /// Get event receiver (can only be called once)
    pub fn take_event_receiver(&self) -> Option<mpsc::UnboundedReceiver<LspEvent>> {
        self.event_rx.write().take()
    }

    /// Start a language server
    pub async fn start_server(&self, language_id: &str) -> Result<()> {
        let config = self.configs.read().get(language_id).cloned()
            .ok_or_else(|| anyhow::anyhow!("Unknown language: {}", language_id))?;

        let root = self.find_root_for_language(&config)?;

        let mut client = LspClient::new(config, self.event_tx.clone());
        client.start(&root).await?;

        let client = Arc::new(tokio::sync::RwLock::new(client));
        self.clients.write().insert(language_id.to_string(), client);

        Ok(())
    }

    /// Stop a language server
    pub async fn stop_server(&self, language_id: &str) -> Result<()> {
        if let Some(client) = self.clients.write().remove(language_id) {
            client.write().await.stop().await?;
        }
        Ok(())
    }

    /// Stop all servers
    pub async fn stop_all(&self) -> Result<()> {
        let language_ids: Vec<_> = self.clients.read().keys().cloned().collect();
        for id in language_ids {
            self.stop_server(&id).await?;
        }
        Ok(())
    }

    /// Get client for a language
    pub fn client(&self, language_id: &str) -> Option<Arc<tokio::sync::RwLock<LspClient>>> {
        self.clients.read().get(language_id).cloned()
    }

    /// Get client for a file
    pub fn client_for_file(&self, path: &Path) -> Option<Arc<tokio::sync::RwLock<LspClient>>> {
        let extension = path.extension()?.to_str()?;

        for (language_id, config) in self.configs.read().iter() {
            for pattern in &config.file_patterns {
                if matches_pattern(pattern, extension) {
                    return self.clients.read().get(language_id).cloned();
                }
            }
        }

        None
    }

    /// Get or start client for a file
    pub async fn get_or_start_for_file(&self, path: &Path) -> Result<Arc<tokio::sync::RwLock<LspClient>>> {
        if let Some(client) = self.client_for_file(path) {
            return Ok(client);
        }

        // Find matching language
        let extension = path.extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension"))?;

        let language_id = self.language_for_extension(extension)
            .ok_or_else(|| anyhow::anyhow!("No language server for extension: {}", extension))?;

        self.start_server(&language_id).await?;

        self.client(&language_id)
            .ok_or_else(|| anyhow::anyhow!("Failed to get client"))
    }

    /// Get language ID for file extension
    pub fn language_for_extension(&self, extension: &str) -> Option<String> {
        for (language_id, config) in self.configs.read().iter() {
            for pattern in &config.file_patterns {
                if matches_pattern(pattern, extension) {
                    return Some(language_id.clone());
                }
            }
        }
        None
    }

    /// Find root directory for a language
    fn find_root_for_language(&self, config: &ServerConfig) -> Result<PathBuf> {
        let roots = self.roots.read();
        
        // Try to find a root with matching root patterns
        for root in roots.iter() {
            for pattern in &config.root_patterns {
                if root.join(pattern).exists() {
                    return Ok(root.clone());
                }
            }
        }

        // Fall back to first root
        roots.first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No workspace root"))
    }

    /// List registered servers
    pub fn registered_servers(&self) -> Vec<String> {
        self.configs.read().keys().cloned().collect()
    }

    /// List running servers
    pub fn running_servers(&self) -> Vec<String> {
        self.clients.read().keys().cloned().collect()
    }
}

impl Default for LspManager {
    fn default() -> Self {
        Self::new()
    }
}

fn matches_pattern(pattern: &str, extension: &str) -> bool {
    if pattern.starts_with("*.") {
        pattern[2..] == *extension
    } else {
        pattern == extension
    }
}
