//! Extension loader - discovers and loads extensions

use std::path::{Path, PathBuf};
use anyhow::Result;
use tokio::fs;

use crate::ExtensionManifest;

/// Discover extensions in a directory
pub async fn discover_extensions(path: &Path) -> Result<Vec<ExtensionManifest>> {
    let mut manifests = Vec::new();
    
    if !path.exists() {
        return Ok(manifests);
    }
    
    let mut entries = fs::read_dir(path).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let entry_path = entry.path();
        
        if entry_path.is_dir() {
            // Look for package.json or foxkit.json
            let manifest_path = entry_path.join("package.json");
            let foxkit_manifest = entry_path.join("foxkit.json");
            
            let manifest_file = if foxkit_manifest.exists() {
                foxkit_manifest
            } else if manifest_path.exists() {
                manifest_path
            } else {
                continue;
            };
            
            match load_manifest(&manifest_file).await {
                Ok(manifest) => {
                    tracing::info!("Discovered extension: {}.{}", manifest.publisher, manifest.name);
                    manifests.push(manifest);
                }
                Err(e) => {
                    tracing::warn!("Failed to load manifest {:?}: {}", manifest_file, e);
                }
            }
        }
    }
    
    Ok(manifests)
}

/// Load extension manifest from file
pub async fn load_manifest(path: &Path) -> Result<ExtensionManifest> {
    let content = fs::read_to_string(path).await?;
    let manifest: ExtensionManifest = serde_json::from_str(&content)?;
    Ok(manifest)
}

/// Load WASM module from extension
pub async fn load_wasm_module(extension_path: &Path, main: &str) -> Result<Vec<u8>> {
    let wasm_path = extension_path.join(main);
    let bytes = fs::read(&wasm_path).await?;
    Ok(bytes)
}

/// Extension installer
pub struct ExtensionInstaller {
    /// Installation directory
    install_dir: PathBuf,
}

impl ExtensionInstaller {
    pub fn new(install_dir: PathBuf) -> Self {
        Self { install_dir }
    }

    /// Install extension from VSIX or archive
    pub async fn install_from_file(&self, path: &Path) -> Result<PathBuf> {
        // TODO: Implement VSIX extraction
        // 1. Verify signature
        // 2. Extract contents
        // 3. Validate manifest
        // 4. Copy to install dir
        
        let file_name = path.file_stem()
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?
            .to_string_lossy();
        
        let install_path = self.install_dir.join(file_name.as_ref());
        
        // For now, just copy directory
        if path.is_dir() {
            copy_dir_recursive(path, &install_path).await?;
        }
        
        Ok(install_path)
    }

    /// Install from marketplace
    pub async fn install_from_marketplace(&self, id: &str, version: Option<&str>) -> Result<PathBuf> {
        // TODO: Implement marketplace download
        // 1. Query marketplace API
        // 2. Download VSIX
        // 3. Install from file
        
        anyhow::bail!("Marketplace installation not yet implemented")
    }

    /// Uninstall extension
    pub async fn uninstall(&self, extension_dir: &Path) -> Result<()> {
        if extension_dir.starts_with(&self.install_dir) && extension_dir.exists() {
            fs::remove_dir_all(extension_dir).await?;
        }
        Ok(())
    }
}

async fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).await?;
    
    let mut entries = fs::read_dir(src).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            Box::pin(copy_dir_recursive(&src_path, &dst_path)).await?;
        } else {
            fs::copy(&src_path, &dst_path).await?;
        }
    }
    
    Ok(())
}

/// Check if a path is a valid extension directory
pub fn is_valid_extension(path: &Path) -> bool {
    path.is_dir() && (
        path.join("package.json").exists() ||
        path.join("foxkit.json").exists()
    )
}
