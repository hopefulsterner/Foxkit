//! # Foxkit SCM
//!
//! Source Control Management UI and providers.

pub mod provider;
pub mod repository;
pub mod changes;
pub mod history;
pub mod blame;
pub mod views;

use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

pub use provider::{ScmProvider, BuiltinProviders};
pub use repository::{Repository, RepositoryState};
pub use changes::{Change, ChangeKind, ResourceState};

/// SCM service
pub struct ScmService {
    /// Registered providers
    providers: RwLock<Vec<Arc<dyn ScmProvider>>>,
    /// Active repositories
    repositories: RwLock<HashMap<PathBuf, Arc<Repository>>>,
    /// Event channel
    events: broadcast::Sender<ScmEvent>,
    /// Configuration
    config: RwLock<ScmConfig>,
}

impl ScmService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(256);
        
        Self {
            providers: RwLock::new(Vec::new()),
            repositories: RwLock::new(HashMap::new()),
            events,
            config: RwLock::new(ScmConfig::default()),
        }
    }

    /// Configure SCM
    pub fn configure(&self, config: ScmConfig) {
        *self.config.write() = config;
    }

    /// Register a provider
    pub fn register<P: ScmProvider + 'static>(&self, provider: P) {
        self.providers.write().push(Arc::new(provider));
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<ScmEvent> {
        self.events.subscribe()
    }

    /// Discover repositories in workspace
    pub async fn discover(&self, workspace: &PathBuf) -> anyhow::Result<Vec<Arc<Repository>>> {
        let mut repos = Vec::new();

        for provider in self.providers.read().iter() {
            if let Ok(discovered) = provider.discover(workspace).await {
                for repo_path in discovered {
                    if let Ok(repo) = provider.open(&repo_path).await {
                        let repo = Arc::new(repo);
                        repos.push(repo.clone());
                        self.repositories.write().insert(repo_path, repo);
                    }
                }
            }
        }

        let _ = self.events.send(ScmEvent::RepositoriesDiscovered {
            count: repos.len(),
        });

        Ok(repos)
    }

    /// Get repository for path
    pub fn get_repository(&self, path: &PathBuf) -> Option<Arc<Repository>> {
        // Find repository that contains this path
        for (repo_path, repo) in self.repositories.read().iter() {
            if path.starts_with(repo_path) {
                return Some(repo.clone());
            }
        }
        None
    }

    /// Get all repositories
    pub fn repositories(&self) -> Vec<Arc<Repository>> {
        self.repositories.read().values().cloned().collect()
    }

    /// Stage changes
    pub async fn stage(&self, repo: &Repository, paths: &[PathBuf]) -> anyhow::Result<()> {
        repo.stage(paths).await?;
        let _ = self.events.send(ScmEvent::ChangesStaged {
            paths: paths.to_vec(),
        });
        Ok(())
    }

    /// Unstage changes
    pub async fn unstage(&self, repo: &Repository, paths: &[PathBuf]) -> anyhow::Result<()> {
        repo.unstage(paths).await?;
        let _ = self.events.send(ScmEvent::ChangesUnstaged {
            paths: paths.to_vec(),
        });
        Ok(())
    }

    /// Commit staged changes
    pub async fn commit(&self, repo: &Repository, message: &str) -> anyhow::Result<String> {
        let commit_id = repo.commit(message).await?;
        let _ = self.events.send(ScmEvent::Committed {
            commit_id: commit_id.clone(),
        });
        Ok(commit_id)
    }

    /// Discard changes
    pub async fn discard(&self, repo: &Repository, paths: &[PathBuf]) -> anyhow::Result<()> {
        repo.discard(paths).await?;
        let _ = self.events.send(ScmEvent::ChangesDiscarded {
            paths: paths.to_vec(),
        });
        Ok(())
    }

    /// Refresh repository state
    pub async fn refresh(&self, repo: &Repository) -> anyhow::Result<()> {
        repo.refresh().await?;
        let _ = self.events.send(ScmEvent::Refreshed);
        Ok(())
    }
}

impl Default for ScmService {
    fn default() -> Self {
        let service = Self::new();
        
        // Register built-in providers
        for provider in BuiltinProviders::all() {
            service.providers.write().push(provider);
        }
        
        service
    }
}

/// SCM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScmConfig {
    /// Auto-refresh on file changes
    pub auto_refresh: bool,
    /// Show inline blame
    pub show_inline_blame: bool,
    /// Count badge limit
    pub count_badge_limit: usize,
    /// Always show actions
    pub always_show_actions: bool,
    /// Default view mode
    pub default_view_mode: ViewMode,
}

impl Default for ScmConfig {
    fn default() -> Self {
        Self {
            auto_refresh: true,
            show_inline_blame: false,
            count_badge_limit: 100,
            always_show_actions: false,
            default_view_mode: ViewMode::Tree,
        }
    }
}

/// View mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ViewMode {
    Tree,
    List,
}

/// SCM event
#[derive(Debug, Clone)]
pub enum ScmEvent {
    RepositoriesDiscovered { count: usize },
    ChangesStaged { paths: Vec<PathBuf> },
    ChangesUnstaged { paths: Vec<PathBuf> },
    Committed { commit_id: String },
    ChangesDiscarded { paths: Vec<PathBuf> },
    Refreshed,
    BranchChanged { branch: String },
    ConflictDetected { paths: Vec<PathBuf> },
}

/// Commit info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author: String,
    pub author_email: String,
    pub timestamp: i64,
    pub parent_ids: Vec<String>,
}

/// Branch info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
}

/// Remote info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub fetch_url: Option<String>,
    pub push_url: Option<String>,
}

/// Stash entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
    pub branch: String,
    pub timestamp: i64,
}
