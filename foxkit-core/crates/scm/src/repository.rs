//! Repository management

use std::path::PathBuf;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{Change, ChangeKind, CommitInfo, BranchInfo, RemoteInfo, StashEntry};

/// Repository
pub struct Repository {
    /// Repository root path
    pub path: PathBuf,
    /// Current state
    state: RwLock<RepositoryState>,
}

impl Repository {
    /// Open a repository
    pub async fn open(path: PathBuf) -> anyhow::Result<Self> {
        let repo = Self {
            path: path.clone(),
            state: RwLock::new(RepositoryState::default()),
        };
        
        repo.refresh().await?;
        
        Ok(repo)
    }

    /// Get current state
    pub fn state(&self) -> RepositoryState {
        self.state.read().clone()
    }

    /// Refresh repository state
    pub async fn refresh(&self) -> anyhow::Result<()> {
        // Get current branch
        let branch = self.get_current_branch().await?;
        
        // Get changes
        let changes = self.get_changes().await?;
        
        // Update state
        let mut state = self.state.write();
        state.branch = branch;
        state.changes = changes;
        state.is_dirty = !state.changes.is_empty();
        
        Ok(())
    }

    /// Get current branch
    async fn get_current_branch(&self) -> anyhow::Result<String> {
        let output = tokio::process::Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&self.path)
            .output()
            .await?;
        
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get changes
    async fn get_changes(&self) -> anyhow::Result<Vec<Change>> {
        let output = tokio::process::Command::new("git")
            .args(["status", "--porcelain", "-uall"])
            .current_dir(&self.path)
            .output()
            .await?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut changes = Vec::new();

        for line in stdout.lines() {
            if line.len() < 4 {
                continue;
            }

            let index_status = line.chars().next().unwrap_or(' ');
            let worktree_status = line.chars().nth(1).unwrap_or(' ');
            let path = PathBuf::from(line[3..].trim());

            let kind = match (index_status, worktree_status) {
                ('?', '?') => ChangeKind::Untracked,
                ('A', _) | (_, 'A') => ChangeKind::Added,
                ('M', _) | (_, 'M') => ChangeKind::Modified,
                ('D', _) | (_, 'D') => ChangeKind::Deleted,
                ('R', _) => ChangeKind::Renamed,
                ('C', _) => ChangeKind::Copied,
                ('U', _) | (_, 'U') => ChangeKind::Conflicted,
                _ => ChangeKind::Modified,
            };

            let staged = index_status != ' ' && index_status != '?';

            changes.push(Change {
                path: self.path.join(&path),
                relative_path: path,
                kind,
                staged,
                original_path: None,
            });
        }

        Ok(changes)
    }

    /// Stage files
    pub async fn stage(&self, paths: &[PathBuf]) -> anyhow::Result<()> {
        let mut cmd = tokio::process::Command::new("git");
        cmd.arg("add").current_dir(&self.path);
        
        for path in paths {
            cmd.arg(path);
        }

        cmd.output().await?;
        self.refresh().await?;
        
        Ok(())
    }

    /// Unstage files
    pub async fn unstage(&self, paths: &[PathBuf]) -> anyhow::Result<()> {
        let mut cmd = tokio::process::Command::new("git");
        cmd.args(["restore", "--staged"]).current_dir(&self.path);
        
        for path in paths {
            cmd.arg(path);
        }

        cmd.output().await?;
        self.refresh().await?;
        
        Ok(())
    }

    /// Commit changes
    pub async fn commit(&self, message: &str) -> anyhow::Result<String> {
        let output = tokio::process::Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.path)
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("Commit failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        // Get the commit hash
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.path)
            .output()
            .await?;

        let commit_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        self.refresh().await?;
        
        Ok(commit_id)
    }

    /// Discard changes
    pub async fn discard(&self, paths: &[PathBuf]) -> anyhow::Result<()> {
        let mut cmd = tokio::process::Command::new("git");
        cmd.args(["checkout", "--"]).current_dir(&self.path);
        
        for path in paths {
            cmd.arg(path);
        }

        cmd.output().await?;
        self.refresh().await?;
        
        Ok(())
    }

    /// Get branches
    pub async fn branches(&self) -> anyhow::Result<Vec<BranchInfo>> {
        let output = tokio::process::Command::new("git")
            .args(["branch", "-a", "-v", "--format=%(refname:short)|%(upstream:short)|%(upstream:track)"])
            .current_dir(&self.path)
            .output()
            .await?;

        let current = self.get_current_branch().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut branches = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<_> = line.split('|').collect();
            if parts.is_empty() {
                continue;
            }

            let name = parts[0].to_string();
            let is_remote = name.contains('/');
            let upstream = parts.get(1).filter(|s| !s.is_empty()).map(|s| s.to_string());
            
            // Parse ahead/behind from track info
            let (ahead, behind) = if let Some(track) = parts.get(2) {
                parse_track_info(track)
            } else {
                (0, 0)
            };

            branches.push(BranchInfo {
                name: name.clone(),
                is_current: name == current,
                is_remote,
                upstream,
                ahead,
                behind,
            });
        }

        Ok(branches)
    }

    /// Checkout branch
    pub async fn checkout(&self, branch: &str) -> anyhow::Result<()> {
        let output = tokio::process::Command::new("git")
            .args(["checkout", branch])
            .current_dir(&self.path)
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("Checkout failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        self.refresh().await?;
        Ok(())
    }

    /// Create new branch
    pub async fn create_branch(&self, name: &str, checkout: bool) -> anyhow::Result<()> {
        let args = if checkout {
            vec!["checkout", "-b", name]
        } else {
            vec!["branch", name]
        };

        let output = tokio::process::Command::new("git")
            .args(&args)
            .current_dir(&self.path)
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("Branch creation failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        self.refresh().await?;
        Ok(())
    }

    /// Get remotes
    pub async fn remotes(&self) -> anyhow::Result<Vec<RemoteInfo>> {
        let output = tokio::process::Command::new("git")
            .args(["remote", "-v"])
            .current_dir(&self.path)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut remotes = std::collections::HashMap::new();

        for line in stdout.lines() {
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let url = parts[1].to_string();
                let is_push = parts.get(2).map(|s| s.contains("push")).unwrap_or(false);

                let remote = remotes.entry(name.clone()).or_insert(RemoteInfo {
                    name,
                    url: url.clone(),
                    fetch_url: None,
                    push_url: None,
                });

                if is_push {
                    remote.push_url = Some(url);
                } else {
                    remote.fetch_url = Some(url);
                }
            }
        }

        Ok(remotes.into_values().collect())
    }

    /// Fetch from remote
    pub async fn fetch(&self, remote: Option<&str>) -> anyhow::Result<()> {
        let mut cmd = tokio::process::Command::new("git");
        cmd.arg("fetch").current_dir(&self.path);
        
        if let Some(r) = remote {
            cmd.arg(r);
        } else {
            cmd.arg("--all");
        }

        cmd.output().await?;
        self.refresh().await?;
        
        Ok(())
    }

    /// Pull from remote
    pub async fn pull(&self) -> anyhow::Result<()> {
        let output = tokio::process::Command::new("git")
            .arg("pull")
            .current_dir(&self.path)
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("Pull failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        self.refresh().await?;
        Ok(())
    }

    /// Push to remote
    pub async fn push(&self, remote: Option<&str>, branch: Option<&str>) -> anyhow::Result<()> {
        let mut cmd = tokio::process::Command::new("git");
        cmd.arg("push").current_dir(&self.path);
        
        if let Some(r) = remote {
            cmd.arg(r);
        }
        if let Some(b) = branch {
            cmd.arg(b);
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            anyhow::bail!("Push failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    /// Get stashes
    pub async fn stashes(&self) -> anyhow::Result<Vec<StashEntry>> {
        let output = tokio::process::Command::new("git")
            .args(["stash", "list", "--format=%gd|%s|%ci"])
            .current_dir(&self.path)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut stashes = Vec::new();

        for (index, line) in stdout.lines().enumerate() {
            let parts: Vec<_> = line.split('|').collect();
            if parts.len() >= 2 {
                stashes.push(StashEntry {
                    index,
                    message: parts[1].to_string(),
                    branch: String::new(), // Would need additional parsing
                    timestamp: 0, // Would need date parsing
                });
            }
        }

        Ok(stashes)
    }

    /// Create stash
    pub async fn stash(&self, message: Option<&str>) -> anyhow::Result<()> {
        let mut cmd = tokio::process::Command::new("git");
        cmd.args(["stash", "push"]).current_dir(&self.path);
        
        if let Some(msg) = message {
            cmd.args(["-m", msg]);
        }

        cmd.output().await?;
        self.refresh().await?;
        
        Ok(())
    }

    /// Pop stash
    pub async fn stash_pop(&self, index: Option<usize>) -> anyhow::Result<()> {
        let mut cmd = tokio::process::Command::new("git");
        cmd.args(["stash", "pop"]).current_dir(&self.path);
        
        if let Some(idx) = index {
            cmd.arg(format!("stash@{{{}}}", idx));
        }

        cmd.output().await?;
        self.refresh().await?;
        
        Ok(())
    }
}

fn parse_track_info(track: &str) -> (usize, usize) {
    let mut ahead = 0;
    let mut behind = 0;

    if track.contains("ahead") {
        if let Some(n) = track.split("ahead ").nth(1) {
            ahead = n.split(|c: char| !c.is_numeric())
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }

    if track.contains("behind") {
        if let Some(n) = track.split("behind ").nth(1) {
            behind = n.split(|c: char| !c.is_numeric())
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }

    (ahead, behind)
}

/// Repository state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepositoryState {
    /// Current branch
    pub branch: String,
    /// Is repository dirty
    pub is_dirty: bool,
    /// Pending changes
    pub changes: Vec<Change>,
    /// Head commit
    pub head: Option<String>,
    /// Upstream tracking
    pub upstream: Option<String>,
    /// Commits ahead of upstream
    pub ahead: usize,
    /// Commits behind upstream
    pub behind: usize,
    /// Active merge/rebase
    pub merge_state: Option<MergeState>,
}

/// Merge state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeState {
    Merging { branch: String },
    Rebasing { branch: String, step: usize, total: usize },
    CherryPicking { commit: String },
    Reverting { commit: String },
}
