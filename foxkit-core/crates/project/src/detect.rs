//! Project type detection

use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::ProjectInfo;

/// Project type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ProjectType {
    #[default]
    Unknown,
    Rust,
    Node,
    TypeScript,
    Python,
    Go,
    Java,
    CSharp,
    Cpp,
    Ruby,
    PHP,
    Swift,
    Kotlin,
}

impl ProjectType {
    pub fn name(&self) -> &'static str {
        match self {
            ProjectType::Unknown => "Unknown",
            ProjectType::Rust => "Rust",
            ProjectType::Node => "Node.js",
            ProjectType::TypeScript => "TypeScript",
            ProjectType::Python => "Python",
            ProjectType::Go => "Go",
            ProjectType::Java => "Java",
            ProjectType::CSharp => "C#",
            ProjectType::Cpp => "C++",
            ProjectType::Ruby => "Ruby",
            ProjectType::PHP => "PHP",
            ProjectType::Swift => "Swift",
            ProjectType::Kotlin => "Kotlin",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ProjectType::Rust => "🦀",
            ProjectType::Node | ProjectType::TypeScript => "📦",
            ProjectType::Python => "🐍",
            ProjectType::Go => "🐹",
            ProjectType::Java => "☕",
            _ => "📁",
        }
    }
}

/// Detect project type from directory
pub fn detect_project(path: &Path) -> Option<ProjectInfo> {
    // Rust
    if path.join("Cargo.toml").exists() {
        return Some(detect_rust(path));
    }

    // Node/TypeScript
    if path.join("package.json").exists() {
        return Some(detect_node(path));
    }

    // Python
    if path.join("pyproject.toml").exists() 
        || path.join("setup.py").exists()
        || path.join("requirements.txt").exists() 
    {
        return Some(detect_python(path));
    }

    // Go
    if path.join("go.mod").exists() {
        return Some(detect_go(path));
    }

    // Java
    if path.join("pom.xml").exists() || path.join("build.gradle").exists() {
        return Some(detect_java(path));
    }

    // C#
    if has_extension(path, "csproj") || has_extension(path, "sln") {
        return Some(ProjectInfo::new(ProjectType::CSharp));
    }

    None
}

fn detect_rust(path: &Path) -> ProjectInfo {
    let mut info = ProjectInfo::new(ProjectType::Rust);
    info.config_file = Some(path.join("Cargo.toml"));
    
    // Check for workspace
    if let Ok(content) = std::fs::read_to_string(path.join("Cargo.toml")) {
        if content.contains("[workspace]") {
            info.is_monorepo = true;
        }
    }

    // Find main.rs or lib.rs
    if path.join("src/main.rs").exists() {
        info.main_file = Some(path.join("src/main.rs"));
    } else if path.join("src/lib.rs").exists() {
        info.main_file = Some(path.join("src/lib.rs"));
    }

    info
}

fn detect_node(path: &Path) -> ProjectInfo {
    let mut info = ProjectInfo::new(ProjectType::Node);
    info.config_file = Some(path.join("package.json"));

    // Determine package manager
    if path.join("pnpm-lock.yaml").exists() {
        info.package_manager = Some("pnpm".to_string());
    } else if path.join("yarn.lock").exists() {
        info.package_manager = Some("yarn".to_string());
    } else if path.join("bun.lockb").exists() {
        info.package_manager = Some("bun".to_string());
    } else {
        info.package_manager = Some("npm".to_string());
    }

    // Check for TypeScript
    if path.join("tsconfig.json").exists() {
        info.project_type = ProjectType::TypeScript;
    }

    // Check for monorepo
    if path.join("pnpm-workspace.yaml").exists()
        || path.join("lerna.json").exists()
        || path.join("nx.json").exists()
        || path.join("turbo.json").exists()
    {
        info.is_monorepo = true;
    }

    // Parse package.json
    if let Ok(content) = std::fs::read_to_string(path.join("package.json")) {
        if let Ok(pkg) = serde_json::from_str::<PackageJson>(&content) {
            info.scripts = pkg.scripts;
            info.dependencies = pkg.dependencies.unwrap_or_default();
            info.dev_dependencies = pkg.dev_dependencies.unwrap_or_default();
            
            if let Some(main) = pkg.main {
                info.main_file = Some(path.join(main));
            }

            // Check workspaces
            if pkg.workspaces.is_some() {
                info.is_monorepo = true;
            }
        }
    }

    info
}

fn detect_python(path: &Path) -> ProjectInfo {
    let mut info = ProjectInfo::new(ProjectType::Python);

    if path.join("pyproject.toml").exists() {
        info.config_file = Some(path.join("pyproject.toml"));
    } else if path.join("setup.py").exists() {
        info.config_file = Some(path.join("setup.py"));
    }

    // Check for package manager
    if path.join("poetry.lock").exists() {
        info.package_manager = Some("poetry".to_string());
    } else if path.join("Pipfile.lock").exists() {
        info.package_manager = Some("pipenv".to_string());
    } else if path.join("pdm.lock").exists() {
        info.package_manager = Some("pdm".to_string());
    } else {
        info.package_manager = Some("pip".to_string());
    }

    // Find main file
    for name in &["main.py", "app.py", "__main__.py"] {
        if path.join(name).exists() {
            info.main_file = Some(path.join(name));
            break;
        }
    }

    info
}

fn detect_go(path: &Path) -> ProjectInfo {
    let mut info = ProjectInfo::new(ProjectType::Go);
    info.config_file = Some(path.join("go.mod"));
    info.package_manager = Some("go".to_string());

    // Find main.go
    if path.join("main.go").exists() {
        info.main_file = Some(path.join("main.go"));
    } else if path.join("cmd/main.go").exists() {
        info.main_file = Some(path.join("cmd/main.go"));
    }

    info
}

fn detect_java(path: &Path) -> ProjectInfo {
    let mut info = ProjectInfo::new(ProjectType::Java);

    if path.join("pom.xml").exists() {
        info.config_file = Some(path.join("pom.xml"));
        info.package_manager = Some("maven".to_string());
    } else if path.join("build.gradle").exists() {
        info.config_file = Some(path.join("build.gradle"));
        info.package_manager = Some("gradle".to_string());
    } else if path.join("build.gradle.kts").exists() {
        info.config_file = Some(path.join("build.gradle.kts"));
        info.package_manager = Some("gradle".to_string());
        info.project_type = ProjectType::Kotlin;
    }

    info
}

fn has_extension(path: &Path, ext: &str) -> bool {
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Some(e) = entry.path().extension() {
                if e == ext {
                    return true;
                }
            }
        }
    }
    false
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    name: Option<String>,
    main: Option<String>,
    scripts: Option<HashMap<String, String>>,
    dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
    workspaces: Option<serde_json::Value>,
}
