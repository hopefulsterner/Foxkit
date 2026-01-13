//! Workspace folder

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Folder identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FolderId(u64);

impl FolderId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

/// A folder in the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFolder {
    /// Folder path
    pub path: PathBuf,
    /// Display name (optional override)
    pub name: Option<String>,
    /// Folder index in workspace
    #[serde(skip)]
    pub index: usize,
}

impl WorkspaceFolder {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            name: None,
            index: 0,
        }
    }

    pub fn with_name(path: PathBuf, name: Option<String>) -> Self {
        Self {
            path,
            name,
            index: 0,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &str {
        if let Some(ref name) = self.name {
            name
        } else {
            self.path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
        }
    }

    /// Check if path is in this folder
    pub fn contains(&self, path: &std::path::Path) -> bool {
        path.starts_with(&self.path)
    }

    /// Get relative path
    pub fn relative(&self, path: &std::path::Path) -> Option<PathBuf> {
        path.strip_prefix(&self.path)
            .ok()
            .map(|p| p.to_path_buf())
    }

    /// Get URI for folder
    pub fn uri(&self) -> String {
        format!("file://{}", self.path.display())
    }
}

impl PartialEq for WorkspaceFolder {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for WorkspaceFolder {}
