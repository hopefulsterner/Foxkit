//! # Foxkit Workspace
//!
//! Workspace and multi-root folder management.

pub mod folder;
pub mod config;
pub mod recent;

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use url::Url;

pub use folder::{WorkspaceFolder, FolderId};
pub use config::WorkspaceConfig;
pub use recent::RecentWorkspaces;

/// Workspace manager
pub struct Workspace {
    /// Workspace folders
    folders: Vec<WorkspaceFolder>,
    /// Workspace file path (if saved)
    workspace_file: Option<PathBuf>,
    /// Workspace configuration
    config: WorkspaceConfig,
    /// Is trusted workspace
    trusted: bool,
    /// Workspace name
    name: Option<String>,
}

impl Workspace {
    /// Create empty workspace
    pub fn empty() -> Self {
        Self {
            folders: Vec::new(),
            workspace_file: None,
            config: WorkspaceConfig::default(),
            trusted: false,
            name: None,
        }
    }

    /// Create from single folder
    pub fn from_folder(path: PathBuf) -> Self {
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());
        
        let folder = WorkspaceFolder::new(path);
        
        Self {
            folders: vec![folder],
            workspace_file: None,
            config: WorkspaceConfig::default(),
            trusted: false,
            name,
        }
    }

    /// Load from workspace file
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let file: WorkspaceFile = serde_json::from_str(&content)?;
        
        let base = path.parent().unwrap_or(Path::new("."));
        let folders = file.folders
            .into_iter()
            .map(|f| {
                let folder_path = if f.path.is_absolute() {
                    f.path
                } else {
                    base.join(&f.path)
                };
                WorkspaceFolder::with_name(folder_path, f.name)
            })
            .collect();

        Ok(Self {
            folders,
            workspace_file: Some(path.to_path_buf()),
            config: file.settings.unwrap_or_default(),
            trusted: false,
            name: path.file_stem().and_then(|n| n.to_str()).map(|s| s.to_string()),
        })
    }

    /// Save workspace file
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let base = path.parent().unwrap_or(Path::new("."));
        
        let folders: Vec<FolderEntry> = self.folders
            .iter()
            .map(|f| {
                let relative = pathdiff::diff_paths(&f.path, base)
                    .unwrap_or_else(|| f.path.clone());
                FolderEntry {
                    path: relative,
                    name: f.name.clone(),
                }
            })
            .collect();

        let file = WorkspaceFile {
            folders,
            settings: Some(self.config.clone()),
        };

        let content = serde_json::to_string_pretty(&file)?;
        std::fs::write(path, content)?;
        
        Ok(())
    }

    /// Add a folder
    pub fn add_folder(&mut self, path: PathBuf) {
        if !self.contains_folder(&path) {
            self.folders.push(WorkspaceFolder::new(path));
        }
    }

    /// Remove a folder
    pub fn remove_folder(&mut self, path: &Path) {
        self.folders.retain(|f| f.path != path);
    }

    /// Check if folder exists
    pub fn contains_folder(&self, path: &Path) -> bool {
        self.folders.iter().any(|f| f.path == path)
    }

    /// Get folders
    pub fn folders(&self) -> &[WorkspaceFolder] {
        &self.folders
    }

    /// Get folder count
    pub fn folder_count(&self) -> usize {
        self.folders.len()
    }

    /// Is multi-root?
    pub fn is_multi_root(&self) -> bool {
        self.folders.len() > 1
    }

    /// Get workspace name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get workspace root (first folder or workspace file dir)
    pub fn root(&self) -> Option<&Path> {
        if let Some(ref file) = self.workspace_file {
            file.parent()
        } else {
            self.folders.first().map(|f| f.path.as_path())
        }
    }

    /// Find folder containing path
    pub fn folder_for_path(&self, path: &Path) -> Option<&WorkspaceFolder> {
        self.folders.iter().find(|f| path.starts_with(&f.path))
    }

    /// Get relative path from workspace
    pub fn relative_path(&self, path: &Path) -> Option<PathBuf> {
        for folder in &self.folders {
            if let Ok(relative) = path.strip_prefix(&folder.path) {
                return Some(relative.to_path_buf());
            }
        }
        None
    }

    /// Is trusted?
    pub fn is_trusted(&self) -> bool {
        self.trusted
    }

    /// Trust the workspace
    pub fn trust(&mut self) {
        self.trusted = true;
    }

    /// Get configuration
    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }

    /// Get configuration mutably
    pub fn config_mut(&mut self) -> &mut WorkspaceConfig {
        &mut self.config
    }

    /// Get workspace file path
    pub fn workspace_file(&self) -> Option<&Path> {
        self.workspace_file.as_deref()
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::empty()
    }
}

/// Workspace file format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceFile {
    folders: Vec<FolderEntry>,
    settings: Option<WorkspaceConfig>,
}

/// Folder entry in workspace file
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FolderEntry {
    path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

/// URI for workspace resources
pub fn workspace_uri(folder: &WorkspaceFolder, relative: &Path) -> Url {
    let full_path = folder.path.join(relative);
    Url::from_file_path(&full_path).unwrap_or_else(|_| {
        Url::parse(&format!("file://{}", full_path.display())).unwrap()
    })
}
