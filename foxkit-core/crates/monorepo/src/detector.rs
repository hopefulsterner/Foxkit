//! Package detection across all languages and build systems

use std::path::{Path, PathBuf};
use anyhow::Result;
use walkdir::WalkDir;
use crate::package::{Package, PackageKind, PackageManager};

/// Detect all packages in a monorepo
pub async fn detect_packages(root: &Path) -> Result<Vec<Package>> {
    let mut packages = Vec::new();
    
    // Walk the directory tree
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_ignored(e.path()))
    {
        let entry = entry?;
        let path = entry.path();
        
        // Check for package markers
        if let Some(pkg) = detect_package_at(path).await? {
            packages.push(pkg);
        }
    }
    
    Ok(packages)
}

/// Check if a directory contains a package
async fn detect_package_at(path: &Path) -> Result<Option<Package>> {
    // JavaScript/TypeScript (package.json)
    if path.join("package.json").exists() {
        return detect_npm_package(path).await;
    }
    
    // Rust (Cargo.toml)
    if path.join("Cargo.toml").exists() {
        return detect_cargo_package(path).await;
    }
    
    // Python (pyproject.toml, setup.py, setup.cfg)
    if path.join("pyproject.toml").exists() {
        return detect_python_package(path).await;
    }
    
    // Go (go.mod)
    if path.join("go.mod").exists() {
        return detect_go_package(path).await;
    }
    
    // Java/Kotlin (pom.xml, build.gradle, build.gradle.kts)
    if path.join("pom.xml").exists() {
        return detect_maven_package(path).await;
    }
    if path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() {
        return detect_gradle_package(path).await;
    }
    
    Ok(None)
}

async fn detect_npm_package(path: &Path) -> Result<Option<Package>> {
    let content = tokio::fs::read_to_string(path.join("package.json")).await?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    
    let name = json["name"]
        .as_str()
        .unwrap_or_else(|| path.file_name().unwrap().to_str().unwrap())
        .to_string();
    
    let version = json["version"]
        .as_str()
        .map(String::from);
    
    // Detect package manager
    let package_manager = if path.join("pnpm-lock.yaml").exists() {
        Some(PackageManager::Pnpm)
    } else if path.join("yarn.lock").exists() {
        Some(PackageManager::Yarn)
    } else if path.join("bun.lockb").exists() {
        Some(PackageManager::Bun)
    } else if path.join("package-lock.json").exists() {
        Some(PackageManager::Npm)
    } else {
        None
    };
    
    // Determine package kind
    let kind = if json.get("workspaces").is_some() {
        PackageKind::WorkspaceRoot
    } else if json.get("private").and_then(|v| v.as_bool()).unwrap_or(false) {
        PackageKind::App
    } else {
        PackageKind::Library
    };
    
    // Extract dependencies
    let deps = extract_json_deps(&json, "dependencies");
    let dev_deps = extract_json_deps(&json, "devDependencies");
    let peer_deps = extract_json_deps(&json, "peerDependencies");
    
    // Find source files
    let source_files = find_source_files(path, &["js", "jsx", "ts", "tsx", "mjs", "cjs"]).await?;
    
    Ok(Some(Package {
        name,
        version,
        path: path.to_path_buf(),
        kind,
        package_manager,
        build_system: detect_build_system(path),
        dependencies: deps,
        dev_dependencies: dev_deps,
        peer_dependencies: peer_deps,
        source_files,
        entry_points: find_entry_points(&json),
    }))
}

async fn detect_cargo_package(path: &Path) -> Result<Option<Package>> {
    let content = tokio::fs::read_to_string(path.join("Cargo.toml")).await?;
    let cargo: toml::Value = toml::from_str(&content)?;
    
    let name = cargo
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or_else(|| path.file_name().unwrap().to_str().unwrap())
        .to_string();
    
    let version = cargo
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from);
    
    // Check if workspace root
    let kind = if cargo.get("workspace").is_some() {
        PackageKind::WorkspaceRoot
    } else if cargo.get("bin").is_some() {
        PackageKind::App
    } else {
        PackageKind::Library
    };
    
    // Extract dependencies
    let deps = extract_cargo_deps(&cargo, "dependencies");
    let dev_deps = extract_cargo_deps(&cargo, "dev-dependencies");
    
    let source_files = find_source_files(path, &["rs"]).await?;
    
    Ok(Some(Package {
        name,
        version,
        path: path.to_path_buf(),
        kind,
        package_manager: Some(PackageManager::Cargo),
        build_system: Some("cargo".into()),
        dependencies: deps,
        dev_dependencies: dev_deps,
        peer_dependencies: Vec::new(),
        source_files,
        entry_points: vec![],
    }))
}

async fn detect_python_package(path: &Path) -> Result<Option<Package>> {
    let content = tokio::fs::read_to_string(path.join("pyproject.toml")).await?;
    let pyproject: toml::Value = toml::from_str(&content)?;
    
    let name = pyproject
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .or_else(|| {
            pyproject.get("tool")
                .and_then(|t| t.get("poetry"))
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
        })
        .unwrap_or_else(|| path.file_name().unwrap().to_str().unwrap())
        .to_string();
    
    let version = pyproject
        .get("project")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from);
    
    let package_manager = if pyproject.get("tool").and_then(|t| t.get("poetry")).is_some() {
        Some(PackageManager::Poetry)
    } else {
        Some(PackageManager::Pip)
    };
    
    let source_files = find_source_files(path, &["py"]).await?;
    
    Ok(Some(Package {
        name,
        version,
        path: path.to_path_buf(),
        kind: PackageKind::Library,
        package_manager,
        build_system: None,
        dependencies: vec![],
        dev_dependencies: vec![],
        peer_dependencies: vec![],
        source_files,
        entry_points: vec![],
    }))
}

async fn detect_go_package(path: &Path) -> Result<Option<Package>> {
    let content = tokio::fs::read_to_string(path.join("go.mod")).await?;
    
    let name = content
        .lines()
        .find(|l| l.starts_with("module "))
        .map(|l| l.trim_start_matches("module ").trim().to_string())
        .unwrap_or_else(|| path.file_name().unwrap().to_str().unwrap().to_string());
    
    let source_files = find_source_files(path, &["go"]).await?;
    
    Ok(Some(Package {
        name,
        version: None,
        path: path.to_path_buf(),
        kind: PackageKind::Library,
        package_manager: Some(PackageManager::Go),
        build_system: Some("go".into()),
        dependencies: vec![],
        dev_dependencies: vec![],
        peer_dependencies: vec![],
        source_files,
        entry_points: vec![],
    }))
}

async fn detect_maven_package(path: &Path) -> Result<Option<Package>> {
    // Simplified Maven detection
    let source_files = find_source_files(path, &["java", "kt"]).await?;
    
    Ok(Some(Package {
        name: path.file_name().unwrap().to_str().unwrap().to_string(),
        version: None,
        path: path.to_path_buf(),
        kind: PackageKind::Library,
        package_manager: Some(PackageManager::Maven),
        build_system: Some("maven".into()),
        dependencies: vec![],
        dev_dependencies: vec![],
        peer_dependencies: vec![],
        source_files,
        entry_points: vec![],
    }))
}

async fn detect_gradle_package(path: &Path) -> Result<Option<Package>> {
    let source_files = find_source_files(path, &["java", "kt"]).await?;
    
    Ok(Some(Package {
        name: path.file_name().unwrap().to_str().unwrap().to_string(),
        version: None,
        path: path.to_path_buf(),
        kind: PackageKind::Library,
        package_manager: Some(PackageManager::Gradle),
        build_system: Some("gradle".into()),
        dependencies: vec![],
        dev_dependencies: vec![],
        peer_dependencies: vec![],
        source_files,
        entry_points: vec![],
    }))
}

// Helper functions

fn is_ignored(path: &Path) -> bool {
    let ignored = [
        "node_modules", "target", "dist", "build", ".git", 
        "__pycache__", ".pytest_cache", "venv", ".venv",
        "vendor", ".next", ".nuxt", "coverage",
    ];
    
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| ignored.contains(&n) || n.starts_with('.'))
        .unwrap_or(false)
}

fn extract_json_deps(json: &serde_json::Value, field: &str) -> Vec<String> {
    json.get(field)
        .and_then(|d| d.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

fn extract_cargo_deps(cargo: &toml::Value, field: &str) -> Vec<String> {
    cargo.get(field)
        .and_then(|d| d.as_table())
        .map(|table| table.keys().cloned().collect())
        .unwrap_or_default()
}

fn detect_build_system(path: &Path) -> Option<String> {
    if path.join("nx.json").exists() {
        Some("nx".into())
    } else if path.join("turbo.json").exists() {
        Some("turborepo".into())
    } else if path.join("lerna.json").exists() {
        Some("lerna".into())
    } else if path.join("BUILD").exists() || path.join("BUILD.bazel").exists() {
        Some("bazel".into())
    } else if path.join("BUCK").exists() {
        Some("buck".into())
    } else {
        None
    }
}

async fn find_source_files(path: &Path, extensions: &[&str]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_ignored(e.path()))
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension() {
                if extensions.iter().any(|e| ext == *e) {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }
    
    Ok(files)
}

fn find_entry_points(json: &serde_json::Value) -> Vec<PathBuf> {
    let mut entries = Vec::new();
    
    if let Some(main) = json.get("main").and_then(|v| v.as_str()) {
        entries.push(PathBuf::from(main));
    }
    if let Some(module) = json.get("module").and_then(|v| v.as_str()) {
        entries.push(PathBuf::from(module));
    }
    if let Some(bin) = json.get("bin") {
        if let Some(s) = bin.as_str() {
            entries.push(PathBuf::from(s));
        } else if let Some(obj) = bin.as_object() {
            for v in obj.values() {
                if let Some(s) = v.as_str() {
                    entries.push(PathBuf::from(s));
                }
            }
        }
    }
    
    entries
}
