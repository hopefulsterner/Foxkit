//! # Foxkit WASM Plugins
//!
//! WebAssembly-based plugin system for safe, sandboxed extensions.

pub mod runtime;
pub mod host;
pub mod manifest;
pub mod loader;
pub mod api;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;

pub use runtime::{WasmRuntime, PluginInstance};
pub use host::{HostFunctions, HostContext};
pub use manifest::{PluginManifest, PluginPermission};
pub use loader::PluginLoader;
pub use api::PluginApi;

/// WASM plugin service
pub struct WasmPluginService {
    /// Plugin runtime
    runtime: Arc<WasmRuntime>,
    /// Loaded plugins
    plugins: RwLock<HashMap<String, Arc<Plugin>>>,
    /// Plugin loader
    loader: PluginLoader,
    /// Plugin directories
    plugin_dirs: Vec<PathBuf>,
}

impl WasmPluginService {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            runtime: Arc::new(WasmRuntime::new()?),
            plugins: RwLock::new(HashMap::new()),
            loader: PluginLoader::new(),
            plugin_dirs: Vec::new(),
        })
    }

    /// Add plugin search directory
    pub fn add_plugin_dir(&mut self, dir: PathBuf) {
        self.plugin_dirs.push(dir);
    }

    /// Discover plugins in search directories
    pub async fn discover(&self) -> anyhow::Result<Vec<PluginManifest>> {
        let mut manifests = Vec::new();

        for dir in &self.plugin_dirs {
            if dir.exists() {
                let found = self.loader.scan_directory(dir).await?;
                manifests.extend(found);
            }
        }

        Ok(manifests)
    }

    /// Load a plugin
    pub async fn load(&self, path: &Path) -> anyhow::Result<Arc<Plugin>> {
        let manifest = self.loader.load_manifest(path).await?;
        let wasm_bytes = self.loader.load_wasm(&manifest, path).await?;

        let instance = self.runtime.instantiate(&wasm_bytes, &manifest).await?;

        let plugin = Arc::new(Plugin {
            manifest: manifest.clone(),
            instance: RwLock::new(Some(instance)),
            state: RwLock::new(PluginState::Loaded),
        });

        self.plugins.write().insert(manifest.id.clone(), plugin.clone());

        tracing::info!("Loaded plugin: {} v{}", manifest.name, manifest.version);
        Ok(plugin)
    }

    /// Unload a plugin
    pub async fn unload(&self, plugin_id: &str) -> anyhow::Result<()> {
        let plugin = self.plugins.write().remove(plugin_id);
        
        if let Some(plugin) = plugin {
            plugin.deactivate().await?;
            tracing::info!("Unloaded plugin: {}", plugin_id);
        }

        Ok(())
    }

    /// Get a loaded plugin
    pub fn get(&self, plugin_id: &str) -> Option<Arc<Plugin>> {
        self.plugins.read().get(plugin_id).cloned()
    }

    /// List loaded plugins
    pub fn list(&self) -> Vec<PluginManifest> {
        self.plugins.read().values().map(|p| p.manifest.clone()).collect()
    }

    /// Call a plugin function
    pub async fn call<T: serde::de::DeserializeOwned>(
        &self,
        plugin_id: &str,
        function: &str,
        args: &impl serde::Serialize,
    ) -> anyhow::Result<T> {
        let plugin = self.get(plugin_id)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_id))?;
        
        plugin.call(function, args).await
    }
}

impl Default for WasmPluginService {
    fn default() -> Self {
        Self::new().expect("Failed to create WASM plugin service")
    }
}

/// A loaded plugin
pub struct Plugin {
    /// Plugin manifest
    pub manifest: PluginManifest,
    /// WASM instance
    instance: RwLock<Option<PluginInstance>>,
    /// Current state
    state: RwLock<PluginState>,
}

impl Plugin {
    /// Get plugin ID
    pub fn id(&self) -> &str {
        &self.manifest.id
    }

    /// Get plugin name
    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    /// Get current state
    pub fn state(&self) -> PluginState {
        *self.state.read()
    }

    /// Activate the plugin
    pub async fn activate(&self) -> anyhow::Result<()> {
        if *self.state.read() == PluginState::Active {
            return Ok(());
        }

        if let Some(ref instance) = *self.instance.read() {
            instance.call_activate().await?;
        }

        *self.state.write() = PluginState::Active;
        tracing::debug!("Activated plugin: {}", self.manifest.id);
        Ok(())
    }

    /// Deactivate the plugin
    pub async fn deactivate(&self) -> anyhow::Result<()> {
        if *self.state.read() != PluginState::Active {
            return Ok(());
        }

        if let Some(ref instance) = *self.instance.read() {
            instance.call_deactivate().await?;
        }

        *self.state.write() = PluginState::Inactive;
        tracing::debug!("Deactivated plugin: {}", self.manifest.id);
        Ok(())
    }

    /// Call a function on the plugin
    pub async fn call<T: serde::de::DeserializeOwned>(
        &self,
        function: &str,
        args: &impl serde::Serialize,
    ) -> anyhow::Result<T> {
        let instance = self.instance.read();
        let instance = instance.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Plugin not loaded"))?;
        
        instance.call_function(function, args).await
    }
}

/// Plugin state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    Loaded,
    Active,
    Inactive,
    Error,
}
