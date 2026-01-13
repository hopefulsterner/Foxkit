//! SCM providers

use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;

use crate::{Repository, CommitInfo, BranchInfo, RemoteInfo};

/// SCM provider trait
#[async_trait]
pub trait ScmProvider: Send + Sync {
    /// Provider ID
    fn id(&self) -> &str;

    /// Provider name
    fn name(&self) -> &str;

    /// Discover repositories
    async fn discover(&self, workspace: &PathBuf) -> anyhow::Result<Vec<PathBuf>>;

    /// Open a repository
    async fn open(&self, path: &PathBuf) -> anyhow::Result<Repository>;

    /// Check if path is a repository
    fn is_repository(&self, path: &PathBuf) -> bool;
}

/// Built-in providers
pub struct BuiltinProviders;

impl BuiltinProviders {
    pub fn all() -> Vec<Arc<dyn ScmProvider>> {
        vec![
            Arc::new(GitProvider),
        ]
    }
}

/// Git provider
pub struct GitProvider;

#[async_trait]
impl ScmProvider for GitProvider {
    fn id(&self) -> &str {
        "git"
    }

    fn name(&self) -> &str {
        "Git"
    }

    async fn discover(&self, workspace: &PathBuf) -> anyhow::Result<Vec<PathBuf>> {
        let mut repos = Vec::new();
        
        // Check if workspace itself is a git repo
        if self.is_repository(workspace) {
            repos.push(workspace.clone());
        }

        // Check subdirectories (for monorepos)
        if let Ok(entries) = std::fs::read_dir(workspace) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && self.is_repository(&path) {
                    repos.push(path);
                }
            }
        }

        Ok(repos)
    }

    async fn open(&self, path: &PathBuf) -> anyhow::Result<Repository> {
        Repository::open(path.clone()).await
    }

    fn is_repository(&self, path: &PathBuf) -> bool {
        path.join(".git").exists() || path.join(".git").is_file()
    }
}

/// Mercurial provider (placeholder)
pub struct MercurialProvider;

#[async_trait]
impl ScmProvider for MercurialProvider {
    fn id(&self) -> &str {
        "hg"
    }

    fn name(&self) -> &str {
        "Mercurial"
    }

    async fn discover(&self, workspace: &PathBuf) -> anyhow::Result<Vec<PathBuf>> {
        let mut repos = Vec::new();
        
        if self.is_repository(workspace) {
            repos.push(workspace.clone());
        }

        Ok(repos)
    }

    async fn open(&self, path: &PathBuf) -> anyhow::Result<Repository> {
        anyhow::bail!("Mercurial not yet implemented")
    }

    fn is_repository(&self, path: &PathBuf) -> bool {
        path.join(".hg").exists()
    }
}

/// SVN provider (placeholder)
pub struct SvnProvider;

#[async_trait]
impl ScmProvider for SvnProvider {
    fn id(&self) -> &str {
        "svn"
    }

    fn name(&self) -> &str {
        "Subversion"
    }

    async fn discover(&self, workspace: &PathBuf) -> anyhow::Result<Vec<PathBuf>> {
        let mut repos = Vec::new();
        
        if self.is_repository(workspace) {
            repos.push(workspace.clone());
        }

        Ok(repos)
    }

    async fn open(&self, path: &PathBuf) -> anyhow::Result<Repository> {
        anyhow::bail!("SVN not yet implemented")
    }

    fn is_repository(&self, path: &PathBuf) -> bool {
        path.join(".svn").exists()
    }
}
