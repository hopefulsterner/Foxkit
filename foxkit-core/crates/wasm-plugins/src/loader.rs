//! Plugin loader

use std::path::{Path, PathBuf};
use tokio::fs;

use crate::PluginManifest;

/// Plugin loader
pub struct PluginLoader {
    /// Cache of loaded manifests
    cache: std::collections::HashMap<PathBuf, PluginManifest>,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
        }
    }

    /// Scan directory for plugins
    pub async fn scan_directory(&self, dir: &Path) -> anyhow::Result<Vec<PluginManifest>> {
        let mut manifests = Vec::new();
        let mut entries = fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_dir() {
                // Look for manifest in directory
                if let Ok(manifest) = self.load_manifest(&path).await {
                    manifests.push(manifest);
                }
            } else if path.extension().map(|e| e == "wasm").unwrap_or(false) {
                // Look for adjacent manifest
                let manifest_path = path.with_extension("json");
                if manifest_path.exists() {
                    if let Ok(manifest) = self.load_manifest_file(&manifest_path).await {
                        manifests.push(manifest);
                    }
                }
            }
        }

        Ok(manifests)
    }

    /// Load plugin manifest from directory or file
    pub async fn load_manifest(&self, path: &Path) -> anyhow::Result<PluginManifest> {
        let manifest_path = if path.is_dir() {
            // Try different manifest names
            let options = [
                path.join("plugin.json"),
                path.join("manifest.json"),
                path.join("package.json"),
            ];
            
            options.into_iter()
                .find(|p| p.exists())
                .ok_or_else(|| anyhow::anyhow!("No manifest found in {}", path.display()))?
        } else if path.is_file() {
            path.to_path_buf()
        } else {
            anyhow::bail!("Path does not exist: {}", path.display());
        };

        self.load_manifest_file(&manifest_path).await
    }

    /// Load manifest from file
    async fn load_manifest_file(&self, path: &Path) -> anyhow::Result<PluginManifest> {
        let content = fs::read_to_string(path).await?;
        let manifest: PluginManifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// Load WASM bytes
    pub async fn load_wasm(&self, manifest: &PluginManifest, base_path: &Path) -> anyhow::Result<Vec<u8>> {
        let wasm_path = if base_path.is_dir() {
            base_path.join(&manifest.wasm)
        } else {
            base_path.parent()
                .map(|p| p.join(&manifest.wasm))
                .unwrap_or_else(|| PathBuf::from(&manifest.wasm))
        };

        let bytes = fs::read(&wasm_path).await?;
        Ok(bytes)
    }

    /// Validate a plugin
    pub fn validate(&self, manifest: &PluginManifest) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check required fields
        if manifest.id.is_empty() {
            errors.push(ValidationError::MissingField("id".to_string()));
        }

        if manifest.name.is_empty() {
            errors.push(ValidationError::MissingField("name".to_string()));
        }

        if manifest.version.is_empty() {
            errors.push(ValidationError::MissingField("version".to_string()));
        }

        if manifest.wasm.is_empty() {
            errors.push(ValidationError::MissingField("wasm".to_string()));
        }

        // Validate ID format (should be valid identifier)
        if !manifest.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
            errors.push(ValidationError::InvalidFormat(
                "id".to_string(),
                "Must contain only alphanumeric characters, dashes, underscores, or dots".to_string(),
            ));
        }

        // Validate version format (semver)
        if !is_valid_semver(&manifest.version) {
            errors.push(ValidationError::InvalidFormat(
                "version".to_string(),
                "Must be valid semver (e.g., 1.0.0)".to_string(),
            ));
        }

        errors
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation error
#[derive(Debug, Clone)]
pub enum ValidationError {
    MissingField(String),
    InvalidFormat(String, String),
    IncompatibleEngine(String),
    MissingDependency(String, String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
            Self::InvalidFormat(field, reason) => write!(f, "Invalid {}: {}", field, reason),
            Self::IncompatibleEngine(required) => write!(f, "Requires engine version: {}", required),
            Self::MissingDependency(dep, version) => write!(f, "Missing dependency: {} {}", dep, version),
        }
    }
}

/// Check if version is valid semver
fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }
    parts.iter().all(|p| p.parse::<u32>().is_ok())
}

/// Plugin registry for discovering plugins from remote sources
pub struct PluginRegistry {
    /// Registry URL
    url: String,
}

impl PluginRegistry {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }

    /// Search for plugins
    pub async fn search(&self, query: &str) -> anyhow::Result<Vec<RegistryEntry>> {
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/search", self.url))
            .query(&[("q", query)])
            .send()
            .await?;

        let entries: Vec<RegistryEntry> = response.json().await?;
        Ok(entries)
    }

    /// Get plugin info
    pub async fn get_plugin(&self, id: &str) -> anyhow::Result<RegistryEntry> {
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/plugins/{}", self.url, id))
            .send()
            .await?;

        let entry: RegistryEntry = response.json().await?;
        Ok(entry)
    }

    /// Download plugin
    pub async fn download(&self, id: &str, version: &str, dest: &Path) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/plugins/{}/versions/{}/download", self.url, id, version))
            .send()
            .await?;

        let bytes = response.bytes().await?;
        fs::write(dest, &bytes).await?;

        Ok(())
    }
}

/// Registry entry
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RegistryEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub downloads: u64,
    pub rating: f32,
}
