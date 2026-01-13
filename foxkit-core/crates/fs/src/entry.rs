//! File entry types

use std::path::{Path, PathBuf};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

/// A file system entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

impl Entry {
    /// Create entry from path
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let meta = std::fs::metadata(path)?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        Ok(Self {
            name,
            path: path.to_path_buf(),
            kind: if meta.is_dir() { EntryKind::Directory } else { EntryKind::File },
            size: meta.len(),
            modified: meta.modified().ok(),
        })
    }

    /// Get file extension
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|e| e.to_str())
    }

    /// Is hidden file?
    pub fn is_hidden(&self) -> bool {
        self.name.starts_with('.')
    }
}

/// Entry kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
}

/// File metadata
#[derive(Debug, Clone)]
pub struct Metadata {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub readonly: bool,
    pub modified: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
}

impl Metadata {
    pub fn from_std(meta: std::fs::Metadata) -> Self {
        Self {
            size: meta.len(),
            is_file: meta.is_file(),
            is_dir: meta.is_dir(),
            is_symlink: meta.is_symlink(),
            readonly: meta.permissions().readonly(),
            modified: meta.modified().ok(),
            created: meta.created().ok(),
            accessed: meta.accessed().ok(),
        }
    }
}
