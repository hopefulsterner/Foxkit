//! File tree structure

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::ignore::IgnoreRules;

/// A file tree
#[derive(Debug, Clone)]
pub struct FileTree {
    pub root: PathBuf,
    pub nodes: HashMap<PathBuf, FileNode>,
}

impl FileTree {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            nodes: HashMap::new(),
        }
    }

    /// Scan directory and build tree
    pub fn scan(root: &Path, ignore: &IgnoreRules) -> anyhow::Result<Self> {
        let mut tree = Self::new(root);
        tree.scan_dir(root, ignore)?;
        Ok(tree)
    }

    fn scan_dir(&mut self, dir: &Path, ignore: &IgnoreRules) -> anyhow::Result<()> {
        let relative = dir.strip_prefix(&self.root).unwrap_or(dir);
        
        let mut children = Vec::new();
        
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let entry_relative = path.strip_prefix(&self.root).unwrap_or(&path);

            if ignore.is_ignored(entry_relative) {
                continue;
            }

            let is_dir = entry.file_type()?.is_dir();
            let size = if is_dir { 0 } else { entry.metadata()?.len() };

            let node = FileNode {
                name: name.clone(),
                path: entry_relative.to_path_buf(),
                is_dir,
                size,
                children: Vec::new(),
            };

            children.push(entry_relative.to_path_buf());
            self.nodes.insert(entry_relative.to_path_buf(), node);

            if is_dir {
                self.scan_dir(&path, ignore)?;
            }
        }

        // Update parent's children
        if let Some(parent) = self.nodes.get_mut(relative) {
            parent.children = children;
        }

        Ok(())
    }

    /// Get node by path
    pub fn get(&self, path: &Path) -> Option<&FileNode> {
        self.nodes.get(path)
    }

    /// Get root children
    pub fn root_children(&self) -> Vec<&FileNode> {
        self.nodes
            .values()
            .filter(|n| n.path.parent().map(|p| p == Path::new("")).unwrap_or(true))
            .collect()
    }

    /// Get children of a directory
    pub fn children(&self, path: &Path) -> Vec<&FileNode> {
        self.get(path)
            .map(|n| {
                n.children
                    .iter()
                    .filter_map(|p| self.nodes.get(p))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Count all files
    pub fn file_count(&self) -> usize {
        self.nodes.values().filter(|n| !n.is_dir).count()
    }

    /// Count all directories
    pub fn dir_count(&self) -> usize {
        self.nodes.values().filter(|n| n.is_dir).count()
    }

    /// Total size of all files
    pub fn total_size(&self) -> u64 {
        self.nodes.values().map(|n| n.size).sum()
    }

    /// Find files by extension
    pub fn find_by_extension(&self, ext: &str) -> Vec<&FileNode> {
        self.nodes
            .values()
            .filter(|n| !n.is_dir && n.path.extension().map(|e| e == ext).unwrap_or(false))
            .collect()
    }

    /// Find files by name pattern
    pub fn find_by_name(&self, pattern: &str) -> Vec<&FileNode> {
        let pattern = pattern.to_lowercase();
        self.nodes
            .values()
            .filter(|n| n.name.to_lowercase().contains(&pattern))
            .collect()
    }
}

/// A file tree node
#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub children: Vec<PathBuf>,
}

impl FileNode {
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|e| e.to_str())
    }

    pub fn is_hidden(&self) -> bool {
        self.name.starts_with('.')
    }
}
