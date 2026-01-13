//! # Foxkit FS
//!
//! File system abstraction with watching, search, and ignore support.

pub mod entry;
pub mod ignore;
pub mod tree;
pub mod watcher;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;

pub use entry::{Entry, EntryKind, Metadata};
pub use tree::{FileTree, FileNode};
pub use watcher::{Watcher, WatchEvent, WatchEventKind};

/// File system abstraction
pub struct Fs {
    /// Root path
    root: PathBuf,
    /// File tree
    tree: Arc<RwLock<FileTree>>,
    /// Ignore rules
    ignore: ignore::IgnoreRules,
    /// File watcher
    watcher: Option<Watcher>,
}

impl Fs {
    /// Create a new file system rooted at path
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            tree: Arc::new(RwLock::new(FileTree::new(&root))),
            root,
            ignore: ignore::IgnoreRules::default(),
            watcher: None,
        }
    }

    /// Get root path
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Load ignore files
    pub fn load_ignores(&mut self) -> anyhow::Result<()> {
        self.ignore = ignore::IgnoreRules::load(&self.root)?;
        Ok(())
    }

    /// Check if path should be ignored
    pub fn is_ignored(&self, path: &Path) -> bool {
        self.ignore.is_ignored(path)
    }

    /// Scan directory tree
    pub fn scan(&self) -> anyhow::Result<()> {
        let tree = FileTree::scan(&self.root, &self.ignore)?;
        *self.tree.write() = tree;
        Ok(())
    }

    /// Get file tree
    pub fn tree(&self) -> Arc<RwLock<FileTree>> {
        Arc::clone(&self.tree)
    }

    /// Read file contents
    pub async fn read(&self, path: &Path) -> anyhow::Result<String> {
        let full_path = self.resolve(path);
        let content = tokio::fs::read_to_string(&full_path).await?;
        Ok(content)
    }

    /// Read file as bytes
    pub async fn read_bytes(&self, path: &Path) -> anyhow::Result<Vec<u8>> {
        let full_path = self.resolve(path);
        let content = tokio::fs::read(&full_path).await?;
        Ok(content)
    }

    /// Write file contents
    pub async fn write(&self, path: &Path, content: &str) -> anyhow::Result<()> {
        let full_path = self.resolve(path);
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&full_path, content).await?;
        Ok(())
    }

    /// Write file as bytes
    pub async fn write_bytes(&self, path: &Path, content: &[u8]) -> anyhow::Result<()> {
        let full_path = self.resolve(path);
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&full_path, content).await?;
        Ok(())
    }

    /// Create directory
    pub async fn create_dir(&self, path: &Path) -> anyhow::Result<()> {
        let full_path = self.resolve(path);
        tokio::fs::create_dir_all(&full_path).await?;
        Ok(())
    }

    /// Remove file
    pub async fn remove_file(&self, path: &Path) -> anyhow::Result<()> {
        let full_path = self.resolve(path);
        tokio::fs::remove_file(&full_path).await?;
        Ok(())
    }

    /// Remove directory
    pub async fn remove_dir(&self, path: &Path) -> anyhow::Result<()> {
        let full_path = self.resolve(path);
        tokio::fs::remove_dir_all(&full_path).await?;
        Ok(())
    }

    /// Rename/move file
    pub async fn rename(&self, from: &Path, to: &Path) -> anyhow::Result<()> {
        let from_full = self.resolve(from);
        let to_full = self.resolve(to);
        tokio::fs::rename(&from_full, &to_full).await?;
        Ok(())
    }

    /// Copy file
    pub async fn copy(&self, from: &Path, to: &Path) -> anyhow::Result<()> {
        let from_full = self.resolve(from);
        let to_full = self.resolve(to);
        tokio::fs::copy(&from_full, &to_full).await?;
        Ok(())
    }

    /// Check if path exists
    pub fn exists(&self, path: &Path) -> bool {
        self.resolve(path).exists()
    }

    /// Check if path is file
    pub fn is_file(&self, path: &Path) -> bool {
        self.resolve(path).is_file()
    }

    /// Check if path is directory
    pub fn is_dir(&self, path: &Path) -> bool {
        self.resolve(path).is_dir()
    }

    /// Get metadata
    pub async fn metadata(&self, path: &Path) -> anyhow::Result<Metadata> {
        let full_path = self.resolve(path);
        let meta = tokio::fs::metadata(&full_path).await?;
        Ok(Metadata::from_std(meta))
    }

    /// Resolve relative path to absolute
    pub fn resolve(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }

    /// Make path relative to root
    pub fn relativize(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.root)
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|_| path.to_path_buf())
    }

    /// Start watching for changes
    pub fn watch(&mut self) -> anyhow::Result<mpsc::Receiver<WatchEvent>> {
        let (tx, rx) = mpsc::channel(100);
        let watcher = Watcher::new(&self.root, tx)?;
        self.watcher = Some(watcher);
        Ok(rx)
    }

    /// Stop watching
    pub fn unwatch(&mut self) {
        self.watcher = None;
    }

    /// List directory contents
    pub async fn list_dir(&self, path: &Path) -> anyhow::Result<Vec<Entry>> {
        let full_path = self.resolve(path);
        let mut entries = Vec::new();
        
        let mut read_dir = tokio::fs::read_dir(&full_path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            let relative = self.relativize(&path);
            
            if !self.is_ignored(&relative) {
                entries.push(Entry::from_path(&path)?);
            }
        }
        
        entries.sort_by(|a, b| {
            // Directories first, then by name
            match (a.kind, b.kind) {
                (EntryKind::Directory, EntryKind::File) => std::cmp::Ordering::Less,
                (EntryKind::File, EntryKind::Directory) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        
        Ok(entries)
    }

    /// Find files matching pattern
    pub fn glob(&self, pattern: &str) -> anyhow::Result<Vec<PathBuf>> {
        let glob = globset::Glob::new(pattern)?.compile_matcher();
        let mut matches = Vec::new();

        for entry in walkdir::WalkDir::new(&self.root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let relative = self.relativize(entry.path());
            if !self.is_ignored(&relative) && glob.is_match(&relative) {
                matches.push(relative);
            }
        }

        Ok(matches)
    }
}

impl Default for Fs {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}
