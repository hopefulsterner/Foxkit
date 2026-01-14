//! # Monorepo Intelligence
//! 
//! 🦊 Foxkit's unique superpower - understanding entire codebases as one system.
//! 
//! This module provides:
//! - Package detection across all languages
//! - Dependency graph construction
//! - Build order optimization
//! - Cross-package navigation
//! - Impact analysis (what breaks if I change this?)
//! - Workspace-aware code intelligence

pub mod detection;
pub mod detector;
pub mod graph;
pub mod impact;
pub mod package;
pub mod workspace_intel;

pub use detection::{PackageDetector, MultiDetector, DetectedPackage, PackageManagerType, CargoDetector, NpmDetector};

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use async_trait::async_trait;
use anyhow::Result;

pub use package::{Package, PackageKind, PackageManager};
pub use graph::DependencyGraph;
pub use impact::ImpactAnalysis;

/// Monorepo intelligence service
/// 
/// The brain that understands your entire codebase
pub struct MonorepoIntel {
    /// Root path of the monorepo
    root: PathBuf,
    /// Discovered packages
    packages: RwLock<HashMap<String, Arc<Package>>>,
    /// Dependency graph
    graph: RwLock<Option<Arc<DependencyGraph>>>,
    /// File -> Package mapping for fast lookups
    file_to_package: RwLock<HashMap<PathBuf, String>>,
}

impl MonorepoIntel {
    /// Create a new MonorepoIntel instance for a workspace root
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            packages: RwLock::new(HashMap::new()),
            graph: RwLock::new(None),
            file_to_package: RwLock::new(HashMap::new()),
        }
    }

    /// Scan and index the entire monorepo
    pub async fn scan(&self) -> Result<MonorepoSummary> {
        tracing::info!("🦊 Scanning monorepo at {:?}", self.root);
        
        // Detect all packages
        let packages = detector::detect_packages(&self.root).await?;
        
        tracing::info!("Found {} packages", packages.len());
        
        // Build file -> package index
        let mut file_index = HashMap::new();
        for pkg in &packages {
            for file in &pkg.source_files {
                file_index.insert(file.clone(), pkg.name.clone());
            }
        }
        
        // Build dependency graph
        let graph = DependencyGraph::build(&packages)?;
        
        // Store results
        {
            let mut pkgs = self.packages.write();
            for pkg in packages.clone() {
                pkgs.insert(pkg.name.clone(), Arc::new(pkg));
            }
        }
        *self.file_to_package.write() = file_index;
        *self.graph.write() = Some(Arc::new(graph));
        
        Ok(MonorepoSummary {
            root: self.root.clone(),
            package_count: packages.len(),
            total_files: packages.iter().map(|p| p.source_files.len()).sum(),
            detected_managers: packages.iter()
                .filter_map(|p| p.package_manager.clone())
                .collect(),
            detected_build_systems: packages.iter()
                .filter_map(|p| p.build_system.clone())
                .collect(),
        })
    }

    /// Get a package by name
    pub fn get_package(&self, name: &str) -> Option<Arc<Package>> {
        self.packages.read().get(name).cloned()
    }

    /// Get all packages
    pub fn packages(&self) -> Vec<Arc<Package>> {
        self.packages.read().values().cloned().collect()
    }

    /// Find which package owns a file
    pub fn package_for_file(&self, file: &Path) -> Option<Arc<Package>> {
        let pkg_name = self.file_to_package.read().get(file)?.clone();
        self.get_package(&pkg_name)
    }

    /// Get the dependency graph
    pub fn dependency_graph(&self) -> Option<Arc<DependencyGraph>> {
        self.graph.read().clone()
    }

    /// Analyze impact of changing a file
    pub fn analyze_impact(&self, file: &Path) -> Result<ImpactAnalysis> {
        let pkg = self.package_for_file(file)
            .ok_or_else(|| anyhow::anyhow!("File not part of any package"))?;
        
        let graph = self.dependency_graph()
            .ok_or_else(|| anyhow::anyhow!("Dependency graph not built"))?;
        
        impact::analyze(&pkg, &graph)
    }

    /// Get optimal build order
    pub fn build_order(&self) -> Result<Vec<String>> {
        let graph = self.dependency_graph()
            .ok_or_else(|| anyhow::anyhow!("Dependency graph not built"))?;
        
        graph.topological_order()
    }

    /// Get packages affected by changes to given packages
    pub fn affected_packages(&self, changed: &[String]) -> Result<Vec<String>> {
        let graph = self.dependency_graph()
            .ok_or_else(|| anyhow::anyhow!("Dependency graph not built"))?;
        
        graph.dependents(changed)
    }
}

/// Summary of monorepo analysis
#[derive(Debug, Clone)]
pub struct MonorepoSummary {
    pub root: PathBuf,
    pub package_count: usize,
    pub total_files: usize,
    pub detected_managers: Vec<PackageManager>,
    pub detected_build_systems: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monorepo_scan() {
        // Test with the Foxkit repo itself
        let intel = MonorepoIntel::new(".");
        let summary = intel.scan().await.unwrap();
        
        assert!(summary.package_count > 0);
    }
}
