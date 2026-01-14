//! Git worktree management for multi-branch workflows.
//!
//! Worktrees allow checking out multiple branches simultaneously in different
//! directories. This is useful for:
//! - Working on multiple features at once
//! - Testing changes without stashing
//! - Running long builds while continuing development
//! - Code reviews with easy branch switching

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Result type for worktree operations.
pub type WorktreeResult<T> = Result<T, WorktreeError>;

/// Worktree operation errors.
#[derive(Debug, Clone)]
pub enum WorktreeError {
    /// Worktree already exists at path.
    AlreadyExists(PathBuf),
    /// Worktree not found.
    NotFound(PathBuf),
    /// Branch already checked out in another worktree.
    BranchInUse { branch: String, worktree: PathBuf },
    /// Invalid worktree path.
    InvalidPath(String),
    /// Cannot remove main worktree.
    CannotRemoveMain,
    /// Worktree is locked.
    Locked { path: PathBuf, reason: Option<String> },
    /// Worktree has uncommitted changes.
    DirtyWorktree(PathBuf),
    /// IO error.
    Io(String),
    /// Git error.
    Git(String),
}

impl std::fmt::Display for WorktreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyExists(path) => write!(f, "Worktree already exists: {}", path.display()),
            Self::NotFound(path) => write!(f, "Worktree not found: {}", path.display()),
            Self::BranchInUse { branch, worktree } => {
                write!(f, "Branch '{}' is already checked out in {}", branch, worktree.display())
            }
            Self::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            Self::CannotRemoveMain => write!(f, "Cannot remove main worktree"),
            Self::Locked { path, reason } => {
                write!(f, "Worktree {} is locked", path.display())?;
                if let Some(r) = reason {
                    write!(f, ": {}", r)?;
                }
                Ok(())
            }
            Self::DirtyWorktree(path) => {
                write!(f, "Worktree has uncommitted changes: {}", path.display())
            }
            Self::Io(msg) => write!(f, "IO error: {}", msg),
            Self::Git(msg) => write!(f, "Git error: {}", msg),
        }
    }
}

impl std::error::Error for WorktreeError {}

/// Information about a git worktree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worktree {
    /// Absolute path to the worktree directory.
    pub path: PathBuf,
    /// HEAD commit hash.
    pub head: String,
    /// Branch name (if on a branch).
    pub branch: Option<String>,
    /// Whether this is the main worktree.
    pub is_main: bool,
    /// Whether the worktree is bare.
    pub is_bare: bool,
    /// Whether the worktree is detached HEAD.
    pub is_detached: bool,
    /// Whether the worktree is locked.
    pub locked: bool,
    /// Lock reason (if locked).
    pub lock_reason: Option<String>,
    /// Whether the worktree directory is prunable (missing).
    pub prunable: bool,
}

impl Worktree {
    /// Check if this worktree is on the given branch.
    pub fn is_on_branch(&self, branch: &str) -> bool {
        self.branch.as_deref() == Some(branch)
    }

    /// Get the short branch name (without refs/heads/).
    pub fn short_branch(&self) -> Option<&str> {
        self.branch.as_ref().map(|b| {
            b.strip_prefix("refs/heads/").unwrap_or(b)
        })
    }

    /// Get the worktree name (directory name).
    pub fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
    }
}

/// Options for creating a worktree.
#[derive(Debug, Clone, Default)]
pub struct AddWorktreeOptions {
    /// Branch to checkout (creates new if doesn't exist).
    pub branch: Option<String>,
    /// Create a new branch with this name.
    pub new_branch: Option<String>,
    /// Start point for new branch.
    pub start_point: Option<String>,
    /// Create in detached HEAD mode.
    pub detach: bool,
    /// Force creation even if branch is checked out elsewhere.
    pub force: bool,
    /// Track a remote branch.
    pub track: Option<String>,
    /// Don't checkout (create worktree but leave it empty).
    pub no_checkout: bool,
    /// Lock the worktree after creation.
    pub lock: bool,
    /// Reason for locking.
    pub lock_reason: Option<String>,
}

impl AddWorktreeOptions {
    /// Create options for a new branch.
    pub fn new_branch(name: impl Into<String>) -> Self {
        Self {
            new_branch: Some(name.into()),
            ..Default::default()
        }
    }

    /// Create options for checking out existing branch.
    pub fn checkout_branch(name: impl Into<String>) -> Self {
        Self {
            branch: Some(name.into()),
            ..Default::default()
        }
    }

    /// Create in detached HEAD mode.
    pub fn detached(commit: impl Into<String>) -> Self {
        Self {
            start_point: Some(commit.into()),
            detach: true,
            ..Default::default()
        }
    }

    /// Set to track remote branch.
    pub fn tracking(mut self, remote_branch: impl Into<String>) -> Self {
        self.track = Some(remote_branch.into());
        self
    }

    /// Force creation.
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }
}

/// Options for removing a worktree.
#[derive(Debug, Clone, Default)]
pub struct RemoveWorktreeOptions {
    /// Force removal even if dirty.
    pub force: bool,
}

/// Options for listing worktrees.
#[derive(Debug, Clone, Default)]
pub struct ListWorktreeOptions {
    /// Include prunable worktrees.
    pub porcelain: bool,
}

/// Git worktree manager.
pub struct WorktreeManager {
    /// Path to the main repository.
    repo_path: PathBuf,
    /// Cached worktree list.
    cache: Option<Vec<Worktree>>,
}

impl WorktreeManager {
    /// Create a new worktree manager for a repository.
    pub fn new(repo_path: impl Into<PathBuf>) -> Self {
        Self {
            repo_path: repo_path.into(),
            cache: None,
        }
    }

    /// Get the main repository path.
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// List all worktrees.
    pub async fn list(&mut self) -> WorktreeResult<Vec<Worktree>> {
        // In a real implementation, this would run `git worktree list --porcelain`
        // and parse the output
        
        let main_worktree = Worktree {
            path: self.repo_path.clone(),
            head: "abc1234".to_string(),
            branch: Some("refs/heads/main".to_string()),
            is_main: true,
            is_bare: false,
            is_detached: false,
            locked: false,
            lock_reason: None,
            prunable: false,
        };

        let worktrees = vec![main_worktree];
        self.cache = Some(worktrees.clone());
        Ok(worktrees)
    }

    /// Get a specific worktree by path.
    pub async fn get(&mut self, path: &Path) -> WorktreeResult<Option<Worktree>> {
        let worktrees = self.list().await?;
        Ok(worktrees.into_iter().find(|w| w.path == path))
    }

    /// Find worktree by branch name.
    pub async fn find_by_branch(&mut self, branch: &str) -> WorktreeResult<Option<Worktree>> {
        let worktrees = self.list().await?;
        let normalized = if branch.starts_with("refs/heads/") {
            branch.to_string()
        } else {
            format!("refs/heads/{}", branch)
        };
        
        Ok(worktrees.into_iter().find(|w| {
            w.branch.as_ref() == Some(&normalized)
        }))
    }

    /// Add a new worktree.
    pub async fn add(
        &mut self,
        path: impl Into<PathBuf>,
        options: &AddWorktreeOptions,
    ) -> WorktreeResult<Worktree> {
        let path = path.into();
        
        // Check if path already exists as a worktree
        if let Some(existing) = self.get(&path).await? {
            return Err(WorktreeError::AlreadyExists(existing.path));
        }

        // Check if branch is already checked out
        if let Some(ref branch) = options.branch {
            if !options.force {
                if let Some(existing) = self.find_by_branch(branch).await? {
                    return Err(WorktreeError::BranchInUse {
                        branch: branch.clone(),
                        worktree: existing.path,
                    });
                }
            }
        }

        // Determine the branch name
        let branch = options.new_branch.clone()
            .or_else(|| options.branch.clone())
            .map(|b| {
                if b.starts_with("refs/heads/") {
                    b
                } else {
                    format!("refs/heads/{}", b)
                }
            });

        // Create the worktree
        let worktree = Worktree {
            path: path.clone(),
            head: options.start_point.clone().unwrap_or_else(|| "HEAD".to_string()),
            branch,
            is_main: false,
            is_bare: false,
            is_detached: options.detach,
            locked: options.lock,
            lock_reason: options.lock_reason.clone(),
            prunable: false,
        };

        // In a real implementation, this would:
        // 1. Create the directory
        // 2. Run `git worktree add <path> <branch>`
        // 3. Set up tracking if specified

        // Invalidate cache
        self.cache = None;

        Ok(worktree)
    }

    /// Remove a worktree.
    pub async fn remove(
        &mut self,
        path: &Path,
        options: &RemoveWorktreeOptions,
    ) -> WorktreeResult<()> {
        let worktree = self.get(path).await?
            .ok_or_else(|| WorktreeError::NotFound(path.to_path_buf()))?;

        if worktree.is_main {
            return Err(WorktreeError::CannotRemoveMain);
        }

        if worktree.locked && !options.force {
            return Err(WorktreeError::Locked {
                path: worktree.path,
                reason: worktree.lock_reason,
            });
        }

        // In a real implementation, this would:
        // 1. Check for uncommitted changes (unless force)
        // 2. Run `git worktree remove <path>`
        // 3. Optionally delete the directory

        // Invalidate cache
        self.cache = None;

        Ok(())
    }

    /// Lock a worktree to prevent accidental pruning.
    pub async fn lock(&mut self, path: &Path, reason: Option<&str>) -> WorktreeResult<()> {
        let worktree = self.get(path).await?
            .ok_or_else(|| WorktreeError::NotFound(path.to_path_buf()))?;

        if worktree.is_main {
            return Err(WorktreeError::InvalidPath("Cannot lock main worktree".into()));
        }

        // In a real implementation: git worktree lock <path> [--reason <reason>]
        self.cache = None;
        Ok(())
    }

    /// Unlock a worktree.
    pub async fn unlock(&mut self, path: &Path) -> WorktreeResult<()> {
        let worktree = self.get(path).await?
            .ok_or_else(|| WorktreeError::NotFound(path.to_path_buf()))?;

        if !worktree.locked {
            return Ok(()); // Already unlocked
        }

        // In a real implementation: git worktree unlock <path>
        self.cache = None;
        Ok(())
    }

    /// Move a worktree to a new location.
    pub async fn move_worktree(
        &mut self,
        from: &Path,
        to: impl Into<PathBuf>,
    ) -> WorktreeResult<Worktree> {
        let to = to.into();
        let worktree = self.get(from).await?
            .ok_or_else(|| WorktreeError::NotFound(from.to_path_buf()))?;

        if worktree.is_main {
            return Err(WorktreeError::InvalidPath("Cannot move main worktree".into()));
        }

        if worktree.locked {
            return Err(WorktreeError::Locked {
                path: worktree.path.clone(),
                reason: worktree.lock_reason.clone(),
            });
        }

        // In a real implementation: git worktree move <worktree> <new-path>
        
        let moved = Worktree {
            path: to,
            ..worktree
        };

        self.cache = None;
        Ok(moved)
    }

    /// Prune stale worktree information.
    pub async fn prune(&mut self, dry_run: bool) -> WorktreeResult<Vec<PathBuf>> {
        let worktrees = self.list().await?;
        let prunable: Vec<PathBuf> = worktrees
            .into_iter()
            .filter(|w| w.prunable)
            .map(|w| w.path)
            .collect();

        if !dry_run {
            // In a real implementation: git worktree prune
            self.cache = None;
        }

        Ok(prunable)
    }

    /// Repair worktree administrative files.
    pub async fn repair(&mut self, paths: &[PathBuf]) -> WorktreeResult<()> {
        // In a real implementation: git worktree repair [<path>...]
        let _ = paths;
        self.cache = None;
        Ok(())
    }
}

/// Worktree session for managing work across multiple worktrees.
pub struct WorktreeSession {
    /// Worktree manager.
    manager: WorktreeManager,
    /// Active worktree paths.
    active_worktrees: HashMap<String, PathBuf>,
}

impl WorktreeSession {
    /// Create a new worktree session.
    pub fn new(repo_path: impl Into<PathBuf>) -> Self {
        Self {
            manager: WorktreeManager::new(repo_path),
            active_worktrees: HashMap::new(),
        }
    }

    /// Open or create a worktree for a branch.
    pub async fn open_branch(&mut self, branch: &str) -> WorktreeResult<&Path> {
        // Check if already tracked
        if let Some(path) = self.active_worktrees.get(branch) {
            return Ok(path);
        }

        // Check if worktree exists
        if let Some(existing) = self.manager.find_by_branch(branch).await? {
            self.active_worktrees.insert(branch.to_string(), existing.path.clone());
            return Ok(self.active_worktrees.get(branch).unwrap());
        }

        // Create new worktree
        let worktree_path = self.default_worktree_path(branch);
        let options = AddWorktreeOptions::checkout_branch(branch);
        let worktree = self.manager.add(&worktree_path, &options).await?;
        
        self.active_worktrees.insert(branch.to_string(), worktree.path.clone());
        Ok(self.active_worktrees.get(branch).unwrap())
    }

    /// Close a worktree session (doesn't remove the worktree).
    pub fn close_branch(&mut self, branch: &str) {
        self.active_worktrees.remove(branch);
    }

    /// Generate default worktree path for a branch.
    fn default_worktree_path(&self, branch: &str) -> PathBuf {
        let repo_path = self.manager.repo_path();
        let parent = repo_path.parent().unwrap_or(repo_path);
        let repo_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("repo");
        
        // Sanitize branch name for filesystem
        let safe_branch = branch
            .replace('/', "-")
            .replace('\\', "-")
            .replace(':', "-");
        
        parent.join(format!("{}-{}", repo_name, safe_branch))
    }

    /// List all tracked worktrees in this session.
    pub fn tracked_worktrees(&self) -> impl Iterator<Item = (&str, &Path)> {
        self.active_worktrees.iter().map(|(k, v)| (k.as_str(), v.as_path()))
    }
}

/// Worktree-aware file operations.
pub struct WorktreeFileOps {
    worktree_path: PathBuf,
}

impl WorktreeFileOps {
    /// Create file operations for a worktree.
    pub fn new(worktree_path: impl Into<PathBuf>) -> Self {
        Self {
            worktree_path: worktree_path.into(),
        }
    }

    /// Resolve a path relative to the worktree.
    pub fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        self.worktree_path.join(path)
    }

    /// Check if a file exists in the worktree.
    pub fn exists(&self, path: impl AsRef<Path>) -> bool {
        self.resolve(path).exists()
    }

    /// Read a file from the worktree.
    pub async fn read(&self, path: impl AsRef<Path>) -> std::io::Result<String> {
        tokio::fs::read_to_string(self.resolve(path)).await
    }

    /// Write a file to the worktree.
    pub async fn write(&self, path: impl AsRef<Path>, contents: &str) -> std::io::Result<()> {
        let full_path = self.resolve(path);
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(full_path, contents).await
    }

    /// Copy a file between worktrees.
    pub async fn copy_from(
        &self,
        source_worktree: &WorktreeFileOps,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let content = source_worktree.read(&path).await?;
        self.write(path, &content).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worktree_manager() {
        let mut manager = WorktreeManager::new("/tmp/test-repo");
        let worktrees = manager.list().await.unwrap();
        
        assert!(!worktrees.is_empty());
        assert!(worktrees[0].is_main);
    }

    #[tokio::test]
    async fn test_add_worktree_options() {
        let options = AddWorktreeOptions::new_branch("feature/test")
            .tracking("origin/main")
            .force();
        
        assert_eq!(options.new_branch, Some("feature/test".to_string()));
        assert!(options.force);
        assert!(options.track.is_some());
    }

    #[test]
    fn test_worktree_short_branch() {
        let worktree = Worktree {
            path: PathBuf::from("/test"),
            head: "abc123".to_string(),
            branch: Some("refs/heads/feature/test".to_string()),
            is_main: false,
            is_bare: false,
            is_detached: false,
            locked: false,
            lock_reason: None,
            prunable: false,
        };

        assert_eq!(worktree.short_branch(), Some("feature/test"));
    }

    #[test]
    fn test_worktree_session_path() {
        let session = WorktreeSession::new("/home/user/projects/myrepo");
        let path = session.default_worktree_path("feature/new-feature");
        
        assert!(path.to_string_lossy().contains("myrepo-feature-new-feature"));
    }
}
