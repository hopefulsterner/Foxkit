//! # Foxkit Git
//!
//! Git integration for version control.

pub mod blame;
pub mod diff;
pub mod repository;
pub mod status;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;

pub use blame::{Blame, BlameLine};
pub use diff::{Diff, DiffHunk, DiffLine, DiffLineKind};
pub use repository::{Repository, Commit, Branch, Remote};
pub use status::{Status, FileStatus, StatusKind};

/// Git manager
pub struct GitManager {
    repositories: Vec<Arc<RwLock<Repository>>>,
}

impl GitManager {
    pub fn new() -> Self {
        Self {
            repositories: Vec::new(),
        }
    }

    /// Open a repository
    pub fn open(&mut self, path: impl AsRef<Path>) -> anyhow::Result<Arc<RwLock<Repository>>> {
        let repo = Repository::open(path)?;
        let shared = Arc::new(RwLock::new(repo));
        self.repositories.push(Arc::clone(&shared));
        Ok(shared)
    }

    /// Find repository for a file
    pub fn repository_for(&self, path: &Path) -> Option<Arc<RwLock<Repository>>> {
        for repo in &self.repositories {
            let repo_read = repo.read();
            if path.starts_with(&repo_read.path) {
                return Some(Arc::clone(repo));
            }
        }
        None
    }

    /// Discover repository containing path
    pub fn discover(path: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
        Repository::discover(path)
    }
}

impl Default for GitManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Git file change kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    Conflicted,
}

impl ChangeKind {
    pub fn symbol(&self) -> char {
        match self {
            ChangeKind::Added => 'A',
            ChangeKind::Modified => 'M',
            ChangeKind::Deleted => 'D',
            ChangeKind::Renamed => 'R',
            ChangeKind::Copied => 'C',
            ChangeKind::Untracked => '?',
            ChangeKind::Ignored => '!',
            ChangeKind::Conflicted => 'U',
        }
    }
}
