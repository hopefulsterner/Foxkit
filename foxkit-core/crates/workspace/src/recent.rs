//! Recent workspaces

use std::path::PathBuf;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

/// Recent workspaces manager
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentWorkspaces {
    entries: Vec<RecentEntry>,
    max_entries: usize,
}

impl RecentWorkspaces {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 50,
        }
    }

    /// Add a recent entry
    pub fn add(&mut self, entry: RecentEntry) {
        // Remove existing entry with same path
        self.entries.retain(|e| e.path() != entry.path());
        
        // Add to front
        self.entries.insert(0, entry);
        
        // Trim to max
        self.entries.truncate(self.max_entries);
    }

    /// Add folder
    pub fn add_folder(&mut self, path: PathBuf) {
        self.add(RecentEntry::Folder {
            folder_path: path,
            label: None,
        });
    }

    /// Add workspace file
    pub fn add_workspace(&mut self, path: PathBuf) {
        self.add(RecentEntry::Workspace {
            workspace_path: path,
            label: None,
        });
    }

    /// Add file
    pub fn add_file(&mut self, path: PathBuf) {
        self.add(RecentEntry::File { file_path: path });
    }

    /// Get all entries
    pub fn entries(&self) -> &[RecentEntry] {
        &self.entries
    }

    /// Get folders only
    pub fn folders(&self) -> impl Iterator<Item = &PathBuf> {
        self.entries.iter().filter_map(|e| {
            if let RecentEntry::Folder { folder_path, .. } = e {
                Some(folder_path)
            } else {
                None
            }
        })
    }

    /// Get workspaces only
    pub fn workspaces(&self) -> impl Iterator<Item = &PathBuf> {
        self.entries.iter().filter_map(|e| {
            if let RecentEntry::Workspace { workspace_path, .. } = e {
                Some(workspace_path)
            } else {
                None
            }
        })
    }

    /// Remove entry
    pub fn remove(&mut self, path: &PathBuf) {
        self.entries.retain(|e| e.path() != path);
    }

    /// Clear all
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Load from file
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let recent: RecentWorkspaces = serde_json::from_str(&content)?;
        Ok(recent)
    }

    /// Save to file
    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// A recent entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RecentEntry {
    Folder {
        #[serde(rename = "folderUri")]
        folder_path: PathBuf,
        label: Option<String>,
    },
    Workspace {
        #[serde(rename = "workspace")]
        workspace_path: PathBuf,
        label: Option<String>,
    },
    File {
        #[serde(rename = "fileUri")]
        file_path: PathBuf,
    },
}

impl RecentEntry {
    pub fn path(&self) -> &PathBuf {
        match self {
            RecentEntry::Folder { folder_path, .. } => folder_path,
            RecentEntry::Workspace { workspace_path, .. } => workspace_path,
            RecentEntry::File { file_path } => file_path,
        }
    }

    pub fn label(&self) -> Option<&str> {
        match self {
            RecentEntry::Folder { label, .. } => label.as_deref(),
            RecentEntry::Workspace { label, .. } => label.as_deref(),
            RecentEntry::File { .. } => None,
        }
    }

    pub fn display_name(&self) -> String {
        if let Some(label) = self.label() {
            return label.to_string();
        }
        
        self.path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string()
    }

    pub fn is_folder(&self) -> bool {
        matches!(self, RecentEntry::Folder { .. })
    }

    pub fn is_workspace(&self) -> bool {
        matches!(self, RecentEntry::Workspace { .. })
    }

    pub fn is_file(&self) -> bool {
        matches!(self, RecentEntry::File { .. })
    }
}
