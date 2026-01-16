//! # Foxkit Project
//!
//! Project detection and configuration management.

pub mod detect;
pub mod config;
pub mod tasks;

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub use detect::{ProjectType, detect_project};
pub use config::ProjectConfig;
pub use tasks::{Task, TaskGroup, TaskRunner};

/// A project
#[derive(Debug, Clone)]
pub struct Project {
    /// Project root directory
    pub root: PathBuf,
    /// Project type
    pub project_type: ProjectType,
    /// Project name
    pub name: String,
    /// Project info from detection
    pub info: ProjectInfo,
    /// Project configuration
    pub config: ProjectConfig,
}

impl Project {
    /// Detect project from directory
    pub fn from_directory(path: &Path) -> Option<Self> {
        let info = detect_project(path)?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Some(Self {
            root: path.to_path_buf(),
            project_type: info.project_type,
            name,
            info,
            config: ProjectConfig::default(),
        })
    }

    /// Get package manager command
    pub fn package_manager(&self) -> Option<&str> {
        self.info.package_manager.as_deref()
    }

    /// Get available tasks
    pub fn tasks(&self) -> Vec<Task> {
        let mut tasks = Vec::new();

        // Add tasks from project type
        match self.project_type {
            ProjectType::Node | ProjectType::TypeScript => {
                if let Some(ref scripts) = self.info.scripts {
                    for (name, _cmd) in scripts {
                        let mut task = Task::new(name, &format!("npm run {}", name), TaskGroup::Build);
                        if name == "build" {
                            task = task.with_default();
                        }
                        tasks.push(task);
                    }
                }
            }
            ProjectType::Rust => {
                tasks.push(Task::new("build", "cargo build", TaskGroup::Build));
                tasks.push(Task::new("run", "cargo run", TaskGroup::Build));
                tasks.push(Task::new("test", "cargo test", TaskGroup::Test));
                tasks.push(Task::new("check", "cargo check", TaskGroup::Build));
                tasks.push(Task::new("clippy", "cargo clippy", TaskGroup::Build));
            }
            ProjectType::Python => {
                tasks.push(Task::new("run", "python main.py", TaskGroup::Build));
                tasks.push(Task::new("test", "pytest", TaskGroup::Test));
            }
            ProjectType::Go => {
                tasks.push(Task::new("build", "go build", TaskGroup::Build));
                tasks.push(Task::new("run", "go run .", TaskGroup::Build));
                tasks.push(Task::new("test", "go test ./...", TaskGroup::Test));
            }
            _ => {}
        }

        tasks
    }

    /// Get main entry file
    pub fn main_file(&self) -> Option<PathBuf> {
        self.info.main_file.clone()
    }

    /// Is monorepo?
    pub fn is_monorepo(&self) -> bool {
        self.info.is_monorepo
    }

    /// Get dependencies
    pub fn dependencies(&self) -> &HashMap<String, String> {
        &self.info.dependencies
    }
}

/// Project detection result
#[derive(Debug, Clone, Default)]
pub struct ProjectInfo {
    pub project_type: ProjectType,
    pub package_manager: Option<String>,
    pub main_file: Option<PathBuf>,
    pub config_file: Option<PathBuf>,
    pub is_monorepo: bool,
    pub scripts: Option<HashMap<String, String>>,
    pub dependencies: HashMap<String, String>,
    pub dev_dependencies: HashMap<String, String>,
}

impl ProjectInfo {
    pub fn new(project_type: ProjectType) -> Self {
        Self {
            project_type,
            ..Default::default()
        }
    }
}
