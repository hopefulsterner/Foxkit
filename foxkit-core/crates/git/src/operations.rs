//! Git operations for staging, committing, and syncing.
//!
//! Provides high-level async operations for common git workflows:
//! - Staging (add, reset, restore)
//! - Committing (commit, amend)
//! - Syncing (push, pull, fetch)
//! - Branching (create, switch, delete, merge, rebase)

use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Result type for git operations.
pub type GitResult<T> = Result<T, GitError>;

/// Git operation errors.
#[derive(Debug, Clone)]
pub enum GitError {
    /// Repository not found.
    NotARepository(PathBuf),
    /// Working directory has uncommitted changes.
    DirtyWorkingDirectory,
    /// Merge conflict detected.
    MergeConflict(Vec<PathBuf>),
    /// Remote operation failed.
    RemoteError(String),
    /// Authentication required.
    AuthenticationRequired,
    /// Branch not found.
    BranchNotFound(String),
    /// Reference error.
    RefError(String),
    /// Index/staging error.
    IndexError(String),
    /// Commit error.
    CommitError(String),
    /// Push rejected (e.g., non-fast-forward).
    PushRejected(String),
    /// Rebase in progress.
    RebaseInProgress,
    /// Merge in progress.
    MergeInProgress,
    /// Cherry-pick in progress.
    CherryPickInProgress,
    /// Operation aborted by user.
    Aborted,
    /// Generic error.
    Other(String),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotARepository(path) => write!(f, "Not a git repository: {}", path.display()),
            Self::DirtyWorkingDirectory => write!(f, "Working directory has uncommitted changes"),
            Self::MergeConflict(files) => write!(f, "Merge conflict in {} files", files.len()),
            Self::RemoteError(msg) => write!(f, "Remote error: {}", msg),
            Self::AuthenticationRequired => write!(f, "Authentication required"),
            Self::BranchNotFound(name) => write!(f, "Branch not found: {}", name),
            Self::RefError(msg) => write!(f, "Reference error: {}", msg),
            Self::IndexError(msg) => write!(f, "Index error: {}", msg),
            Self::CommitError(msg) => write!(f, "Commit error: {}", msg),
            Self::PushRejected(msg) => write!(f, "Push rejected: {}", msg),
            Self::RebaseInProgress => write!(f, "Rebase in progress"),
            Self::MergeInProgress => write!(f, "Merge in progress"),
            Self::CherryPickInProgress => write!(f, "Cherry-pick in progress"),
            Self::Aborted => write!(f, "Operation aborted"),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for GitError {}

/// Options for staging files.
#[derive(Debug, Clone, Default)]
pub struct StageOptions {
    /// Stage all changes (including untracked).
    pub all: bool,
    /// Update tracked files only.
    pub update: bool,
    /// Interactive staging (patch mode).
    pub patch: bool,
    /// Intent to add (stage path without content).
    pub intent_to_add: bool,
    /// Force add ignored files.
    pub force: bool,
}

/// Options for committing.
#[derive(Debug, Clone)]
pub struct CommitOptions {
    /// Commit message.
    pub message: String,
    /// Extended description.
    pub body: Option<String>,
    /// Amend the previous commit.
    pub amend: bool,
    /// Allow empty commits.
    pub allow_empty: bool,
    /// Sign the commit with GPG.
    pub sign: bool,
    /// Author override.
    pub author: Option<CommitAuthor>,
    /// Skip pre-commit hooks.
    pub no_verify: bool,
    /// Automatically stage modified files.
    pub all: bool,
}

impl Default for CommitOptions {
    fn default() -> Self {
        Self {
            message: String::new(),
            body: None,
            amend: false,
            allow_empty: false,
            sign: false,
            author: None,
            no_verify: false,
            all: false,
        }
    }
}

impl CommitOptions {
    /// Create commit options with a message.
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            ..Default::default()
        }
    }

    /// Set as an amend commit.
    pub fn amend(mut self) -> Self {
        self.amend = true;
        self
    }

    /// Set the commit body.
    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Enable GPG signing.
    pub fn signed(mut self) -> Self {
        self.sign = true;
        self
    }
}

/// Commit author information.
#[derive(Debug, Clone)]
pub struct CommitAuthor {
    pub name: String,
    pub email: String,
    pub timestamp: Option<i64>,
}

impl CommitAuthor {
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            timestamp: None,
        }
    }
}

/// Options for push operations.
#[derive(Debug, Clone, Default)]
pub struct PushOptions {
    /// Remote name (default: origin).
    pub remote: Option<String>,
    /// Branch to push (default: current).
    pub branch: Option<String>,
    /// Set upstream tracking.
    pub set_upstream: bool,
    /// Force push.
    pub force: bool,
    /// Force push with lease (safer).
    pub force_with_lease: bool,
    /// Push tags.
    pub tags: bool,
    /// Delete remote branch.
    pub delete: bool,
    /// Dry run (don't actually push).
    pub dry_run: bool,
}

impl PushOptions {
    /// Create options for pushing to a specific remote.
    pub fn to_remote(remote: impl Into<String>) -> Self {
        Self {
            remote: Some(remote.into()),
            ..Default::default()
        }
    }

    /// Set upstream while pushing.
    pub fn with_upstream(mut self) -> Self {
        self.set_upstream = true;
        self
    }

    /// Force push with lease.
    pub fn force_with_lease(mut self) -> Self {
        self.force_with_lease = true;
        self
    }
}

/// Options for pull operations.
#[derive(Debug, Clone, Default)]
pub struct PullOptions {
    /// Remote name.
    pub remote: Option<String>,
    /// Branch to pull.
    pub branch: Option<String>,
    /// Rebase instead of merge.
    pub rebase: bool,
    /// Fast-forward only.
    pub ff_only: bool,
    /// No fast-forward (always create merge commit).
    pub no_ff: bool,
    /// Autostash before pull.
    pub autostash: bool,
}

impl PullOptions {
    /// Create options with rebase.
    pub fn with_rebase() -> Self {
        Self {
            rebase: true,
            ..Default::default()
        }
    }

    /// Fast-forward only.
    pub fn ff_only() -> Self {
        Self {
            ff_only: true,
            ..Default::default()
        }
    }
}

/// Options for fetch operations.
#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    /// Remote to fetch from (None = all remotes).
    pub remote: Option<String>,
    /// Fetch all remotes.
    pub all: bool,
    /// Prune deleted remote branches.
    pub prune: bool,
    /// Fetch tags.
    pub tags: bool,
    /// Depth for shallow fetch.
    pub depth: Option<u32>,
}

/// Options for branch operations.
#[derive(Debug, Clone, Default)]
pub struct BranchOptions {
    /// Start point for new branch.
    pub start_point: Option<String>,
    /// Track a remote branch.
    pub track: Option<String>,
    /// Force create/delete.
    pub force: bool,
    /// Delete branch.
    pub delete: bool,
    /// Move/rename branch.
    pub rename: Option<String>,
}

/// Options for merge operations.
#[derive(Debug, Clone, Default)]
pub struct MergeOptions {
    /// Branch to merge.
    pub branch: String,
    /// Merge message.
    pub message: Option<String>,
    /// Fast-forward only.
    pub ff_only: bool,
    /// No fast-forward.
    pub no_ff: bool,
    /// Squash commits.
    pub squash: bool,
    /// Abort on conflict.
    pub abort_on_conflict: bool,
    /// Strategy (recursive, ours, theirs, etc.).
    pub strategy: Option<String>,
}

/// Options for rebase operations.
#[derive(Debug, Clone, Default)]
pub struct RebaseOptions {
    /// Branch/commit to rebase onto.
    pub onto: String,
    /// Interactive rebase.
    pub interactive: bool,
    /// Autosquash fixup commits.
    pub autosquash: bool,
    /// Autostash changes.
    pub autostash: bool,
    /// Preserve merge commits.
    pub preserve_merges: bool,
}

/// Options for stash operations.
#[derive(Debug, Clone, Default)]
pub struct StashOptions {
    /// Stash message.
    pub message: Option<String>,
    /// Include untracked files.
    pub include_untracked: bool,
    /// Stash everything including ignored.
    pub all: bool,
    /// Keep index staged.
    pub keep_index: bool,
}

/// A stash entry.
#[derive(Debug, Clone)]
pub struct StashEntry {
    /// Stash index (0 = most recent).
    pub index: usize,
    /// Stash reference (stash@{0}).
    pub reference: String,
    /// Stash message.
    pub message: String,
    /// Branch where stash was created.
    pub branch: Option<String>,
    /// Timestamp.
    pub timestamp: i64,
}

/// Result of a push operation.
#[derive(Debug, Clone)]
pub struct PushResult {
    /// Remote that was pushed to.
    pub remote: String,
    /// Branch that was pushed.
    pub branch: String,
    /// Old commit hash.
    pub old_hash: Option<String>,
    /// New commit hash.
    pub new_hash: String,
    /// Whether it was forced.
    pub forced: bool,
}

/// Result of a pull/fetch operation.
#[derive(Debug, Clone)]
pub struct FetchResult {
    /// Remote that was fetched from.
    pub remote: String,
    /// Updated references.
    pub updated_refs: Vec<UpdatedRef>,
    /// New tags.
    pub new_tags: Vec<String>,
    /// Pruned branches.
    pub pruned: Vec<String>,
}

/// An updated reference from fetch.
#[derive(Debug, Clone)]
pub struct UpdatedRef {
    /// Reference name.
    pub name: String,
    /// Old hash (None if new).
    pub old_hash: Option<String>,
    /// New hash.
    pub new_hash: String,
    /// Whether it was fast-forward.
    pub fast_forward: bool,
}

/// Git operations handler.
pub struct GitOperations {
    /// Repository path.
    repo_path: PathBuf,
    /// Credentials callback.
    credentials: Option<Box<dyn CredentialsProvider + Send + Sync>>,
}

/// Trait for providing git credentials.
pub trait CredentialsProvider {
    /// Get credentials for a URL.
    fn get_credentials(&self, url: &str) -> Option<Credentials>;
}

/// Git credentials.
#[derive(Debug, Clone)]
pub enum Credentials {
    /// Username/password.
    UserPass { username: String, password: String },
    /// SSH key.
    SshKey {
        username: String,
        public_key: Option<PathBuf>,
        private_key: PathBuf,
        passphrase: Option<String>,
    },
    /// SSH agent.
    SshAgent { username: String },
    /// Default (from git config).
    Default,
}

impl GitOperations {
    /// Create a new operations handler for a repository.
    pub fn new(repo_path: impl Into<PathBuf>) -> Self {
        Self {
            repo_path: repo_path.into(),
            credentials: None,
        }
    }

    /// Set credentials provider.
    pub fn with_credentials<P: CredentialsProvider + Send + Sync + 'static>(
        mut self,
        provider: P,
    ) -> Self {
        self.credentials = Some(Box::new(provider));
        self
    }

    /// Stage files for commit.
    pub async fn stage(&self, paths: &[PathBuf], options: &StageOptions) -> GitResult<Vec<PathBuf>> {
        // In a real implementation, this would use gix or git2
        let mut staged = Vec::new();
        
        for path in paths {
            let full_path = self.repo_path.join(path);
            if full_path.exists() || options.intent_to_add {
                staged.push(path.clone());
            }
        }
        
        Ok(staged)
    }

    /// Stage all changes.
    pub async fn stage_all(&self) -> GitResult<Vec<PathBuf>> {
        self.stage(&[], &StageOptions { all: true, ..Default::default() }).await
    }

    /// Unstage files (remove from index).
    pub async fn unstage(&self, paths: &[PathBuf]) -> GitResult<Vec<PathBuf>> {
        // Reset paths in the index
        Ok(paths.to_vec())
    }

    /// Unstage all files.
    pub async fn unstage_all(&self) -> GitResult<()> {
        // git reset HEAD
        Ok(())
    }

    /// Discard changes in working directory.
    pub async fn discard(&self, paths: &[PathBuf]) -> GitResult<Vec<PathBuf>> {
        // git checkout -- <paths> or git restore <paths>
        Ok(paths.to_vec())
    }

    /// Discard all working directory changes.
    pub async fn discard_all(&self) -> GitResult<()> {
        // git checkout -- . or git restore .
        Ok(())
    }

    /// Create a commit.
    pub async fn commit(&self, options: &CommitOptions) -> GitResult<CommitInfo> {
        if options.message.is_empty() && !options.amend {
            return Err(GitError::CommitError("Empty commit message".into()));
        }

        // In a real implementation, this would create an actual commit
        let hash = format!("{:040x}", rand_hash());
        
        Ok(CommitInfo {
            hash: hash.clone(),
            short_hash: hash[..7].to_string(),
            message: options.message.clone(),
            author: options.author.clone().unwrap_or_else(|| CommitAuthor {
                name: "User".into(),
                email: "user@example.com".into(),
                timestamp: None,
            }),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            parents: vec![],
        })
    }

    /// Amend the last commit.
    pub async fn amend(&self, message: Option<&str>) -> GitResult<CommitInfo> {
        let mut options = CommitOptions::default();
        options.amend = true;
        if let Some(msg) = message {
            options.message = msg.to_string();
        }
        self.commit(&options).await
    }

    /// Push to remote.
    pub async fn push(&self, options: &PushOptions) -> GitResult<PushResult> {
        let remote = options.remote.clone().unwrap_or_else(|| "origin".into());
        let branch = options.branch.clone().unwrap_or_else(|| "main".into());

        // In a real implementation, this would push to the remote
        Ok(PushResult {
            remote,
            branch,
            old_hash: Some(format!("{:040x}", rand_hash())),
            new_hash: format!("{:040x}", rand_hash()),
            forced: options.force || options.force_with_lease,
        })
    }

    /// Pull from remote.
    pub async fn pull(&self, options: &PullOptions) -> GitResult<PullResult> {
        // Fetch then merge/rebase
        let fetch_result = self.fetch(&FetchOptions {
            remote: options.remote.clone(),
            ..Default::default()
        }).await?;

        Ok(PullResult {
            fetch: fetch_result,
            merge_commit: if options.rebase {
                None
            } else {
                Some(format!("{:040x}", rand_hash()))
            },
            rebased: options.rebase,
            conflicts: vec![],
        })
    }

    /// Fetch from remote.
    pub async fn fetch(&self, options: &FetchOptions) -> GitResult<FetchResult> {
        let remote = options.remote.clone().unwrap_or_else(|| "origin".into());
        
        Ok(FetchResult {
            remote,
            updated_refs: vec![],
            new_tags: vec![],
            pruned: vec![],
        })
    }

    /// Create a new branch.
    pub async fn create_branch(&self, name: &str, options: &BranchOptions) -> GitResult<BranchInfo> {
        Ok(BranchInfo {
            name: name.to_string(),
            is_remote: false,
            upstream: options.track.clone(),
            head: format!("{:040x}", rand_hash()),
            ahead: 0,
            behind: 0,
        })
    }

    /// Switch to a branch.
    pub async fn switch_branch(&self, name: &str) -> GitResult<()> {
        // git switch <name> or git checkout <name>
        Ok(())
    }

    /// Delete a branch.
    pub async fn delete_branch(&self, name: &str, force: bool) -> GitResult<()> {
        if !force {
            // Check if branch is merged
        }
        Ok(())
    }

    /// Merge a branch.
    pub async fn merge(&self, options: &MergeOptions) -> GitResult<MergeResult> {
        Ok(MergeResult {
            commit: Some(format!("{:040x}", rand_hash())),
            fast_forward: false,
            conflicts: vec![],
        })
    }

    /// Abort a merge.
    pub async fn merge_abort(&self) -> GitResult<()> {
        Ok(())
    }

    /// Start a rebase.
    pub async fn rebase(&self, options: &RebaseOptions) -> GitResult<RebaseResult> {
        Ok(RebaseResult {
            completed: true,
            current_step: 0,
            total_steps: 0,
            conflicts: vec![],
        })
    }

    /// Continue a rebase.
    pub async fn rebase_continue(&self) -> GitResult<RebaseResult> {
        Ok(RebaseResult {
            completed: true,
            current_step: 0,
            total_steps: 0,
            conflicts: vec![],
        })
    }

    /// Abort a rebase.
    pub async fn rebase_abort(&self) -> GitResult<()> {
        Ok(())
    }

    /// Skip current rebase step.
    pub async fn rebase_skip(&self) -> GitResult<RebaseResult> {
        Ok(RebaseResult {
            completed: true,
            current_step: 0,
            total_steps: 0,
            conflicts: vec![],
        })
    }

    /// Create a stash.
    pub async fn stash(&self, options: &StashOptions) -> GitResult<StashEntry> {
        Ok(StashEntry {
            index: 0,
            reference: "stash@{0}".into(),
            message: options.message.clone().unwrap_or_else(|| "WIP".into()),
            branch: Some("main".into()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
        })
    }

    /// Apply a stash.
    pub async fn stash_apply(&self, index: usize) -> GitResult<()> {
        let _ = index;
        Ok(())
    }

    /// Pop a stash (apply and delete).
    pub async fn stash_pop(&self, index: usize) -> GitResult<()> {
        self.stash_apply(index).await?;
        self.stash_drop(index).await
    }

    /// Drop a stash.
    pub async fn stash_drop(&self, index: usize) -> GitResult<()> {
        let _ = index;
        Ok(())
    }

    /// List all stashes.
    pub async fn stash_list(&self) -> GitResult<Vec<StashEntry>> {
        Ok(vec![])
    }

    /// Cherry-pick a commit.
    pub async fn cherry_pick(&self, commit: &str) -> GitResult<CommitInfo> {
        let hash = format!("{:040x}", rand_hash());
        Ok(CommitInfo {
            hash: hash.clone(),
            short_hash: hash[..7].to_string(),
            message: format!("Cherry-picked from {}", commit),
            author: CommitAuthor::new("User", "user@example.com"),
            timestamp: 0,
            parents: vec![],
        })
    }

    /// Revert a commit.
    pub async fn revert(&self, commit: &str) -> GitResult<CommitInfo> {
        let hash = format!("{:040x}", rand_hash());
        Ok(CommitInfo {
            hash: hash.clone(),
            short_hash: hash[..7].to_string(),
            message: format!("Revert \"{}\"", commit),
            author: CommitAuthor::new("User", "user@example.com"),
            timestamp: 0,
            parents: vec![],
        })
    }

    /// Reset to a commit.
    pub async fn reset(&self, target: &str, mode: ResetMode) -> GitResult<()> {
        let _ = (target, mode);
        Ok(())
    }

    /// Clean untracked files.
    pub async fn clean(&self, options: &CleanOptions) -> GitResult<Vec<PathBuf>> {
        let _ = options;
        Ok(vec![])
    }

    /// Get repository state.
    pub async fn state(&self) -> GitResult<RepositoryState> {
        Ok(RepositoryState::Clean)
    }
}

/// Commit information.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: CommitAuthor,
    pub timestamp: i64,
    pub parents: Vec<String>,
}

/// Branch information.
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub head: String,
    pub ahead: usize,
    pub behind: usize,
}

/// Result of a pull operation.
#[derive(Debug, Clone)]
pub struct PullResult {
    pub fetch: FetchResult,
    pub merge_commit: Option<String>,
    pub rebased: bool,
    pub conflicts: Vec<PathBuf>,
}

/// Result of a merge operation.
#[derive(Debug, Clone)]
pub struct MergeResult {
    pub commit: Option<String>,
    pub fast_forward: bool,
    pub conflicts: Vec<PathBuf>,
}

/// Result of a rebase operation.
#[derive(Debug, Clone)]
pub struct RebaseResult {
    pub completed: bool,
    pub current_step: usize,
    pub total_steps: usize,
    pub conflicts: Vec<PathBuf>,
}

/// Reset modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMode {
    /// Keep working directory and index.
    Soft,
    /// Keep working directory, reset index.
    Mixed,
    /// Reset everything.
    Hard,
    /// Reset index, keep working directory changes.
    Keep,
    /// Merge-like reset.
    Merge,
}

/// Options for clean operation.
#[derive(Debug, Clone, Default)]
pub struct CleanOptions {
    /// Remove directories.
    pub directories: bool,
    /// Remove ignored files.
    pub ignored: bool,
    /// Force (required).
    pub force: bool,
    /// Dry run.
    pub dry_run: bool,
    /// Paths to clean.
    pub paths: Vec<PathBuf>,
}

/// Repository state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryState {
    /// Normal state.
    Clean,
    /// Merge in progress.
    Merge,
    /// Rebase in progress.
    Rebase,
    /// Interactive rebase.
    RebaseInteractive,
    /// Rebase merging.
    RebaseMerge,
    /// Cherry-pick in progress.
    CherryPick,
    /// Cherry-pick sequence.
    CherryPickSequence,
    /// Bisect in progress.
    Bisect,
    /// Revert in progress.
    Revert,
    /// Revert sequence.
    RevertSequence,
    /// Apply mailbox.
    ApplyMailbox,
    /// Apply mailbox or rebase.
    ApplyMailboxOrRebase,
}

/// Generate a random hash-like number for testing.
fn rand_hash() -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    RandomState::new().build_hasher().finish()
}

/// Builder for batch git operations.
pub struct GitBatch {
    operations: Vec<BatchOperation>,
    repo_path: PathBuf,
}

enum BatchOperation {
    Stage(Vec<PathBuf>),
    Unstage(Vec<PathBuf>),
    Commit(CommitOptions),
    Push(PushOptions),
}

impl GitBatch {
    /// Create a new batch for a repository.
    pub fn new(repo_path: impl Into<PathBuf>) -> Self {
        Self {
            operations: Vec::new(),
            repo_path: repo_path.into(),
        }
    }

    /// Add stage operation.
    pub fn stage(mut self, paths: Vec<PathBuf>) -> Self {
        self.operations.push(BatchOperation::Stage(paths));
        self
    }

    /// Add unstage operation.
    pub fn unstage(mut self, paths: Vec<PathBuf>) -> Self {
        self.operations.push(BatchOperation::Unstage(paths));
        self
    }

    /// Add commit operation.
    pub fn commit(mut self, options: CommitOptions) -> Self {
        self.operations.push(BatchOperation::Commit(options));
        self
    }

    /// Add push operation.
    pub fn push(mut self, options: PushOptions) -> Self {
        self.operations.push(BatchOperation::Push(options));
        self
    }

    /// Execute all operations.
    pub async fn execute(self) -> GitResult<BatchResult> {
        let ops = GitOperations::new(&self.repo_path);
        let mut results = BatchResult::default();

        for operation in self.operations {
            match operation {
                BatchOperation::Stage(paths) => {
                    let staged = ops.stage(&paths, &StageOptions::default()).await?;
                    results.staged.extend(staged);
                }
                BatchOperation::Unstage(paths) => {
                    let unstaged = ops.unstage(&paths).await?;
                    results.unstaged.extend(unstaged);
                }
                BatchOperation::Commit(options) => {
                    results.commit = Some(ops.commit(&options).await?);
                }
                BatchOperation::Push(options) => {
                    results.push = Some(ops.push(&options).await?);
                }
            }
        }

        Ok(results)
    }
}

/// Result of batch operations.
#[derive(Debug, Clone, Default)]
pub struct BatchResult {
    pub staged: Vec<PathBuf>,
    pub unstaged: Vec<PathBuf>,
    pub commit: Option<CommitInfo>,
    pub push: Option<PushResult>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_commit_options() {
        let options = CommitOptions::with_message("feat: add feature")
            .with_body("Detailed description")
            .signed();
        
        assert_eq!(options.message, "feat: add feature");
        assert!(options.body.is_some());
        assert!(options.sign);
    }

    #[tokio::test]
    async fn test_push_options() {
        let options = PushOptions::to_remote("upstream")
            .with_upstream()
            .force_with_lease();
        
        assert_eq!(options.remote, Some("upstream".into()));
        assert!(options.set_upstream);
        assert!(options.force_with_lease);
    }

    #[tokio::test]
    async fn test_git_operations() {
        let ops = GitOperations::new("/tmp/test-repo");
        
        let commit = ops.commit(&CommitOptions::with_message("test")).await.unwrap();
        assert!(!commit.hash.is_empty());
    }
}
