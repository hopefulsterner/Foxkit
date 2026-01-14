//! Package Detection
//!
//! Detects packages across different languages and build systems.

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use async_trait::async_trait;
use anyhow::Result;

/// Package manager type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PackageManagerType {
    // JavaScript/TypeScript
    Npm,
    Yarn,
    Pnpm,
    Bun,
    // Rust
    Cargo,
    // Python
    Pip,
    Poetry,
    Uv,
    Conda,
    // Go
    GoMod,
    // Java/JVM
    Maven,
    Gradle,
    Sbt,
    // Ruby
    Bundler,
    // PHP
    Composer,
    // .NET
    NuGet,
    Dotnet,
    // Other
    Custom(String),
}

impl PackageManagerType {
    pub fn from_file(filename: &str) -> Option<Self> {
        match filename {
            "package.json" => Some(Self::Npm),
            "yarn.lock" => Some(Self::Yarn),
            "pnpm-lock.yaml" | "pnpm-workspace.yaml" => Some(Self::Pnpm),
            "bun.lockb" => Some(Self::Bun),
            "Cargo.toml" => Some(Self::Cargo),
            "requirements.txt" | "setup.py" | "setup.cfg" => Some(Self::Pip),
            "pyproject.toml" => Some(Self::Poetry), // Could also be uv
            "go.mod" => Some(Self::GoMod),
            "pom.xml" => Some(Self::Maven),
            "build.gradle" | "build.gradle.kts" => Some(Self::Gradle),
            "build.sbt" => Some(Self::Sbt),
            "Gemfile" => Some(Self::Bundler),
            "composer.json" => Some(Self::Composer),
            "packages.config" | "*.csproj" => Some(Self::NuGet),
            _ => None,
        }
    }

    pub fn manifest_file(&self) -> &'static str {
        match self {
            Self::Npm | Self::Yarn | Self::Pnpm | Self::Bun => "package.json",
            Self::Cargo => "Cargo.toml",
            Self::Pip => "requirements.txt",
            Self::Poetry | Self::Uv => "pyproject.toml",
            Self::Conda => "environment.yml",
            Self::GoMod => "go.mod",
            Self::Maven => "pom.xml",
            Self::Gradle => "build.gradle",
            Self::Sbt => "build.sbt",
            Self::Bundler => "Gemfile",
            Self::Composer => "composer.json",
            Self::NuGet => "packages.config",
            Self::Dotnet => "*.csproj",
            Self::Custom(_) => "",
        }
    }

    pub fn lock_file(&self) -> Option<&'static str> {
        match self {
            Self::Npm => Some("package-lock.json"),
            Self::Yarn => Some("yarn.lock"),
            Self::Pnpm => Some("pnpm-lock.yaml"),
            Self::Bun => Some("bun.lockb"),
            Self::Cargo => Some("Cargo.lock"),
            Self::Poetry => Some("poetry.lock"),
            Self::Uv => Some("uv.lock"),
            Self::GoMod => Some("go.sum"),
            Self::Bundler => Some("Gemfile.lock"),
            Self::Composer => Some("composer.lock"),
            _ => None,
        }
    }
}

/// Detected package information
#[derive(Debug, Clone)]
pub struct DetectedPackage {
    pub name: String,
    pub version: Option<String>,
    pub path: PathBuf,
    pub package_manager: PackageManagerType,
    pub dependencies: Vec<PackageDependency>,
    pub dev_dependencies: Vec<PackageDependency>,
    pub scripts: HashMap<String, String>,
    pub workspace_members: Vec<String>,
    pub is_workspace_root: bool,
}

/// Package dependency
#[derive(Debug, Clone)]
pub struct PackageDependency {
    pub name: String,
    pub version_req: String,
    pub is_local: bool,
    pub local_path: Option<PathBuf>,
}

/// Package detector trait
#[async_trait]
pub trait PackageDetector: Send + Sync {
    /// Detect if this detector applies to the given path
    fn detect(&self, path: &Path) -> bool;
    
    /// Parse package information
    async fn parse(&self, path: &Path) -> Result<DetectedPackage>;
    
    /// Get the package manager type
    fn package_manager(&self) -> PackageManagerType;
}

/// Cargo (Rust) detector
pub struct CargoDetector;

#[async_trait]
impl PackageDetector for CargoDetector {
    fn detect(&self, path: &Path) -> bool {
        path.join("Cargo.toml").exists()
    }
    
    async fn parse(&self, path: &Path) -> Result<DetectedPackage> {
        let manifest_path = path.join("Cargo.toml");
        let content = tokio::fs::read_to_string(&manifest_path).await?;
        let manifest: toml::Value = content.parse()?;
        
        let package = manifest.get("package");
        let name = package
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();
        let version = package
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let mut dependencies = Vec::new();
        if let Some(deps) = manifest.get("dependencies").and_then(|d| d.as_table()) {
            for (dep_name, dep_value) in deps {
                let (version_req, is_local, local_path) = parse_cargo_dep(dep_value);
                dependencies.push(PackageDependency {
                    name: dep_name.clone(),
                    version_req,
                    is_local,
                    local_path,
                });
            }
        }
        
        let mut dev_dependencies = Vec::new();
        if let Some(deps) = manifest.get("dev-dependencies").and_then(|d| d.as_table()) {
            for (dep_name, dep_value) in deps {
                let (version_req, is_local, local_path) = parse_cargo_dep(dep_value);
                dev_dependencies.push(PackageDependency {
                    name: dep_name.clone(),
                    version_req,
                    is_local,
                    local_path,
                });
            }
        }
        
        // Check for workspace
        let workspace_members = manifest
            .get("workspace")
            .and_then(|w| w.get("members"))
            .and_then(|m| m.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        let is_workspace_root = manifest.get("workspace").is_some();
        
        Ok(DetectedPackage {
            name,
            version,
            path: path.to_path_buf(),
            package_manager: PackageManagerType::Cargo,
            dependencies,
            dev_dependencies,
            scripts: HashMap::new(),
            workspace_members,
            is_workspace_root,
        })
    }
    
    fn package_manager(&self) -> PackageManagerType {
        PackageManagerType::Cargo
    }
}

fn parse_cargo_dep(value: &toml::Value) -> (String, bool, Option<PathBuf>) {
    match value {
        toml::Value::String(v) => (v.clone(), false, None),
        toml::Value::Table(t) => {
            let version = t.get("version").and_then(|v| v.as_str()).unwrap_or("*").to_string();
            let path = t.get("path").and_then(|p| p.as_str()).map(PathBuf::from);
            let is_local = path.is_some();
            (version, is_local, path)
        }
        _ => ("*".to_string(), false, None),
    }
}

/// NPM/Node detector
pub struct NpmDetector;

#[async_trait]
impl PackageDetector for NpmDetector {
    fn detect(&self, path: &Path) -> bool {
        path.join("package.json").exists()
    }
    
    async fn parse(&self, path: &Path) -> Result<DetectedPackage> {
        let manifest_path = path.join("package.json");
        let content = tokio::fs::read_to_string(&manifest_path).await?;
        let manifest: serde_json::Value = serde_json::from_str(&content)?;
        
        let name = manifest.get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();
        let version = manifest.get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let mut dependencies = Vec::new();
        if let Some(deps) = manifest.get("dependencies").and_then(|d| d.as_object()) {
            for (dep_name, dep_value) in deps {
                let version_req = dep_value.as_str().unwrap_or("*").to_string();
                let is_local = version_req.starts_with("file:") || version_req.starts_with("link:");
                dependencies.push(PackageDependency {
                    name: dep_name.clone(),
                    version_req,
                    is_local,
                    local_path: None,
                });
            }
        }
        
        let mut dev_dependencies = Vec::new();
        if let Some(deps) = manifest.get("devDependencies").and_then(|d| d.as_object()) {
            for (dep_name, dep_value) in deps {
                let version_req = dep_value.as_str().unwrap_or("*").to_string();
                let is_local = version_req.starts_with("file:") || version_req.starts_with("link:");
                dev_dependencies.push(PackageDependency {
                    name: dep_name.clone(),
                    version_req,
                    is_local,
                    local_path: None,
                });
            }
        }
        
        let mut scripts = HashMap::new();
        if let Some(s) = manifest.get("scripts").and_then(|s| s.as_object()) {
            for (name, cmd) in s {
                if let Some(cmd_str) = cmd.as_str() {
                    scripts.insert(name.clone(), cmd_str.to_string());
                }
            }
        }
        
        let workspace_members = manifest.get("workspaces")
            .and_then(|w| w.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        let is_workspace_root = !workspace_members.is_empty();
        
        // Detect which package manager
        let package_manager = if path.join("pnpm-lock.yaml").exists() {
            PackageManagerType::Pnpm
        } else if path.join("yarn.lock").exists() {
            PackageManagerType::Yarn
        } else if path.join("bun.lockb").exists() {
            PackageManagerType::Bun
        } else {
            PackageManagerType::Npm
        };
        
        Ok(DetectedPackage {
            name,
            version,
            path: path.to_path_buf(),
            package_manager,
            dependencies,
            dev_dependencies,
            scripts,
            workspace_members,
            is_workspace_root,
        })
    }
    
    fn package_manager(&self) -> PackageManagerType {
        PackageManagerType::Npm
    }
}

/// Multi-detector that tries all registered detectors
pub struct MultiDetector {
    detectors: Vec<Box<dyn PackageDetector>>,
}

impl MultiDetector {
    pub fn new() -> Self {
        Self {
            detectors: vec![
                Box::new(CargoDetector),
                Box::new(NpmDetector),
            ],
        }
    }

    pub fn add_detector(&mut self, detector: Box<dyn PackageDetector>) {
        self.detectors.push(detector);
    }

    pub async fn detect(&self, path: &Path) -> Option<DetectedPackage> {
        for detector in &self.detectors {
            if detector.detect(path) {
                if let Ok(pkg) = detector.parse(path).await {
                    return Some(pkg);
                }
            }
        }
        None
    }

    pub async fn detect_all(&self, root: &Path) -> Vec<DetectedPackage> {
        let mut packages = Vec::new();
        self.scan_recursive(root, &mut packages).await;
        packages
    }

    async fn scan_recursive(&self, path: &Path, packages: &mut Vec<DetectedPackage>) {
        if let Some(pkg) = self.detect(path).await {
            packages.push(pkg);
        }
        
        // Scan subdirectories
        if let Ok(mut entries) = tokio::fs::read_dir(path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    let name = entry_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    // Skip common non-package directories
                    if !["node_modules", "target", ".git", "vendor", "__pycache__", ".venv", "venv"].contains(&name) {
                        Box::pin(self.scan_recursive(&entry_path, packages)).await;
                    }
                }
            }
        }
    }
}

impl Default for MultiDetector {
    fn default() -> Self {
        Self::new()
    }
}
