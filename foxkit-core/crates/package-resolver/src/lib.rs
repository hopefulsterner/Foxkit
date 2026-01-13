//! Package resolution for monorepos.
//!
//! This crate provides package discovery and resolution for various
//! package managers: npm/yarn/pnpm, cargo, go modules, etc.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use dependency_graph::{PackageId, PackageNode, PackageType, DependencyEdge, DependencyType};

/// Package manager type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Cargo,
    Go,
    Maven,
    Gradle,
    Pip,
    Poetry,
}

/// Discovered package information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPackage {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub manifest_path: PathBuf,
    pub package_manager: PackageManager,
    pub dependencies: HashMap<String, DependencyInfo>,
    pub dev_dependencies: HashMap<String, DependencyInfo>,
}

/// Dependency information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub name: String,
    pub version_req: String,
    pub is_workspace: bool,
    pub is_optional: bool,
}

/// Workspace configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub root: PathBuf,
    pub package_manager: PackageManager,
    pub packages: Vec<String>, // Glob patterns
}

/// Trait for package resolvers.
#[async_trait]
pub trait PackageResolver: Send + Sync {
    /// Get the package manager type.
    fn package_manager(&self) -> PackageManager;

    /// Check if this resolver can handle the given path.
    async fn can_resolve(&self, path: &Path) -> bool;

    /// Discover packages in a directory.
    async fn discover(&self, root: &Path) -> anyhow::Result<Vec<DiscoveredPackage>>;

    /// Parse a manifest file.
    async fn parse_manifest(&self, path: &Path) -> anyhow::Result<DiscoveredPackage>;
}

/// Cargo package resolver.
pub struct CargoResolver;

#[async_trait]
impl PackageResolver for CargoResolver {
    fn package_manager(&self) -> PackageManager {
        PackageManager::Cargo
    }

    async fn can_resolve(&self, path: &Path) -> bool {
        path.join("Cargo.toml").exists()
    }

    async fn discover(&self, root: &Path) -> anyhow::Result<Vec<DiscoveredPackage>> {
        let mut packages = Vec::new();
        
        // Check for workspace Cargo.toml
        let root_manifest = root.join("Cargo.toml");
        if root_manifest.exists() {
            // For now, just return the root package
            if let Ok(pkg) = self.parse_manifest(&root_manifest).await {
                packages.push(pkg);
            }
        }
        
        Ok(packages)
    }

    async fn parse_manifest(&self, path: &Path) -> anyhow::Result<DiscoveredPackage> {
        let content = tokio::fs::read_to_string(path).await?;
        let manifest: toml::Value = toml::from_str(&content)?;
        
        let package = manifest.get("package").ok_or_else(|| {
            anyhow::anyhow!("No [package] section in Cargo.toml")
        })?;
        
        let name = package.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        
        let version = package.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();
        
        Ok(DiscoveredPackage {
            name,
            version,
            path: path.parent().unwrap_or(path).to_path_buf(),
            manifest_path: path.to_path_buf(),
            package_manager: PackageManager::Cargo,
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
        })
    }
}

/// NPM/Yarn/PNPM package resolver.
pub struct NodeResolver {
    manager: PackageManager,
}

impl NodeResolver {
    pub fn npm() -> Self {
        Self { manager: PackageManager::Npm }
    }

    pub fn yarn() -> Self {
        Self { manager: PackageManager::Yarn }
    }

    pub fn pnpm() -> Self {
        Self { manager: PackageManager::Pnpm }
    }
}

#[async_trait]
impl PackageResolver for NodeResolver {
    fn package_manager(&self) -> PackageManager {
        self.manager
    }

    async fn can_resolve(&self, path: &Path) -> bool {
        path.join("package.json").exists()
    }

    async fn discover(&self, root: &Path) -> anyhow::Result<Vec<DiscoveredPackage>> {
        let mut packages = Vec::new();
        
        let root_manifest = root.join("package.json");
        if root_manifest.exists() {
            if let Ok(pkg) = self.parse_manifest(&root_manifest).await {
                packages.push(pkg);
            }
        }
        
        Ok(packages)
    }

    async fn parse_manifest(&self, path: &Path) -> anyhow::Result<DiscoveredPackage> {
        let content = tokio::fs::read_to_string(path).await?;
        let manifest: serde_json::Value = serde_json::from_str(&content)?;
        
        let name = manifest.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        
        let version = manifest.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();
        
        Ok(DiscoveredPackage {
            name,
            version,
            path: path.parent().unwrap_or(path).to_path_buf(),
            manifest_path: path.to_path_buf(),
            package_manager: self.manager,
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
        })
    }
}

/// Multi-resolver that tries multiple package resolvers.
pub struct MultiResolver {
    resolvers: Vec<Box<dyn PackageResolver>>,
}

impl MultiResolver {
    pub fn new() -> Self {
        Self {
            resolvers: vec![
                Box::new(CargoResolver),
                Box::new(NodeResolver::npm()),
            ],
        }
    }

    pub fn add_resolver(&mut self, resolver: Box<dyn PackageResolver>) {
        self.resolvers.push(resolver);
    }

    pub async fn discover_all(&self, root: &Path) -> anyhow::Result<Vec<DiscoveredPackage>> {
        let mut all_packages = Vec::new();
        
        for resolver in &self.resolvers {
            if resolver.can_resolve(root).await {
                let packages = resolver.discover(root).await?;
                all_packages.extend(packages);
            }
        }
        
        Ok(all_packages)
    }
}

impl Default for MultiResolver {
    fn default() -> Self {
        Self::new()
    }
}
