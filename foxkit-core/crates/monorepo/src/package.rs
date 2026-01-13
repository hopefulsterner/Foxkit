//! Package representation

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// A package within the monorepo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Package name (e.g., "@myorg/utils", "my-crate")
    pub name: String,
    /// Package version
    pub version: Option<String>,
    /// Path to package root
    pub path: PathBuf,
    /// Type of package
    pub kind: PackageKind,
    /// Package manager used
    pub package_manager: Option<PackageManager>,
    /// Build system (nx, turbo, etc.)
    pub build_system: Option<String>,
    /// Production dependencies
    pub dependencies: Vec<String>,
    /// Development dependencies
    pub dev_dependencies: Vec<String>,
    /// Peer dependencies
    pub peer_dependencies: Vec<String>,
    /// All source files in this package
    pub source_files: Vec<PathBuf>,
    /// Entry points (main, bin, etc.)
    pub entry_points: Vec<PathBuf>,
}

/// Type of package
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageKind {
    /// Workspace root (contains other packages)
    WorkspaceRoot,
    /// Standalone application
    App,
    /// Reusable library
    Library,
    /// Build tooling/dev package
    Tool,
    /// Documentation package
    Docs,
    /// Test/spec package
    Test,
}

/// Package manager
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PackageManager {
    // JavaScript/TypeScript
    Npm,
    Yarn,
    Pnpm,
    Bun,
    // Rust
    Cargo,
    // Go
    Go,
    // Python
    Pip,
    Poetry,
    Uv,
    // Java/Kotlin
    Maven,
    Gradle,
    // .NET
    Nuget,
    // Ruby
    Bundler,
    // PHP
    Composer,
}

impl Package {
    /// Check if this package depends on another
    pub fn depends_on(&self, other: &str) -> bool {
        self.dependencies.contains(&other.to_string())
            || self.dev_dependencies.contains(&other.to_string())
            || self.peer_dependencies.contains(&other.to_string())
    }

    /// Get all dependencies (combined)
    pub fn all_dependencies(&self) -> Vec<&str> {
        self.dependencies
            .iter()
            .chain(self.dev_dependencies.iter())
            .chain(self.peer_dependencies.iter())
            .map(|s| s.as_str())
            .collect()
    }

    /// Check if this package is a workspace root
    pub fn is_workspace_root(&self) -> bool {
        matches!(self.kind, PackageKind::WorkspaceRoot)
    }

    /// Get the language(s) used in this package
    pub fn languages(&self) -> Vec<&'static str> {
        let mut langs = Vec::new();
        
        for file in &self.source_files {
            if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
                let lang = match ext {
                    "rs" => "Rust",
                    "js" | "mjs" | "cjs" => "JavaScript",
                    "ts" | "mts" | "cts" => "TypeScript",
                    "jsx" | "tsx" => "React",
                    "py" => "Python",
                    "go" => "Go",
                    "java" => "Java",
                    "kt" | "kts" => "Kotlin",
                    "rb" => "Ruby",
                    "php" => "PHP",
                    "cs" => "C#",
                    "cpp" | "cc" | "cxx" => "C++",
                    "c" | "h" => "C",
                    _ => continue,
                };
                if !langs.contains(&lang) {
                    langs.push(lang);
                }
            }
        }
        
        langs
    }
}

impl std::fmt::Display for PackageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageManager::Npm => write!(f, "npm"),
            PackageManager::Yarn => write!(f, "yarn"),
            PackageManager::Pnpm => write!(f, "pnpm"),
            PackageManager::Bun => write!(f, "bun"),
            PackageManager::Cargo => write!(f, "cargo"),
            PackageManager::Go => write!(f, "go"),
            PackageManager::Pip => write!(f, "pip"),
            PackageManager::Poetry => write!(f, "poetry"),
            PackageManager::Uv => write!(f, "uv"),
            PackageManager::Maven => write!(f, "maven"),
            PackageManager::Gradle => write!(f, "gradle"),
            PackageManager::Nuget => write!(f, "nuget"),
            PackageManager::Bundler => write!(f, "bundler"),
            PackageManager::Composer => write!(f, "composer"),
        }
    }
}
