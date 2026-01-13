//! # Foxkit File Watcher
//!
//! File system change detection and notification.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// File watcher service
pub struct FileWatcherService {
    /// Watched paths
    watches: RwLock<HashMap<PathBuf, WatchConfig>>,
    /// Exclude patterns
    excludes: RwLock<Vec<GlobPattern>>,
    /// Events
    events: broadcast::Sender<FileWatcherEvent>,
    /// Configuration
    config: RwLock<FileWatcherConfig>,
    /// Is running
    running: RwLock<bool>,
}

impl FileWatcherService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(1024);

        Self {
            watches: RwLock::new(HashMap::new()),
            excludes: RwLock::new(default_excludes()),
            events,
            config: RwLock::new(FileWatcherConfig::default()),
            running: RwLock::new(false),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<FileWatcherEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: FileWatcherConfig) {
        *self.config.write() = config;
    }

    /// Watch a path
    pub fn watch(&self, path: PathBuf, config: WatchConfig) -> anyhow::Result<WatchHandle> {
        let id = WatchId::new();

        // Check if already watching
        if self.watches.read().contains_key(&path) {
            return Ok(WatchHandle {
                id,
                path: path.clone(),
            });
        }

        self.watches.write().insert(path.clone(), config);

        let _ = self.events.send(FileWatcherEvent::WatchStarted {
            path: path.clone(),
        });

        Ok(WatchHandle { id, path })
    }

    /// Unwatch a path
    pub fn unwatch(&self, path: &PathBuf) {
        if self.watches.write().remove(path).is_some() {
            let _ = self.events.send(FileWatcherEvent::WatchStopped {
                path: path.clone(),
            });
        }
    }

    /// Add exclude pattern
    pub fn add_exclude(&self, pattern: impl Into<String>) {
        self.excludes.write().push(GlobPattern::new(pattern.into()));
    }

    /// Check if path should be excluded
    pub fn is_excluded(&self, path: &Path) -> bool {
        let excludes = self.excludes.read();

        for pattern in excludes.iter() {
            if pattern.matches(path) {
                return true;
            }
        }

        false
    }

    /// Handle file change (called by watcher implementation)
    pub fn handle_change(&self, change: FileChange) {
        // Check excludes
        if self.is_excluded(&change.path) {
            return;
        }

        // Check if within watched paths
        let is_watched = self.watches.read().keys().any(|watched| {
            change.path.starts_with(watched)
        });

        if !is_watched {
            return;
        }

        let _ = self.events.send(FileWatcherEvent::FileChanged {
            change: change.clone(),
        });
    }

    /// Get all watched paths
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watches.read().keys().cloned().collect()
    }

    /// Start watching (would initialize native watcher)
    pub fn start(&self) -> anyhow::Result<()> {
        *self.running.write() = true;
        
        let _ = self.events.send(FileWatcherEvent::Started);

        // Would initialize notify::RecommendedWatcher here
        // and start watching all registered paths

        Ok(())
    }

    /// Stop watching
    pub fn stop(&self) {
        *self.running.write() = false;
        let _ = self.events.send(FileWatcherEvent::Stopped);
    }

    /// Is running
    pub fn is_running(&self) -> bool {
        *self.running.read()
    }
}

impl Default for FileWatcherService {
    fn default() -> Self {
        Self::new()
    }
}

/// Default exclude patterns
fn default_excludes() -> Vec<GlobPattern> {
    vec![
        GlobPattern::new("**/.git/**".to_string()),
        GlobPattern::new("**/node_modules/**".to_string()),
        GlobPattern::new("**/target/**".to_string()),
        GlobPattern::new("**/.DS_Store".to_string()),
        GlobPattern::new("**/Thumbs.db".to_string()),
        GlobPattern::new("**/__pycache__/**".to_string()),
        GlobPattern::new("**/.venv/**".to_string()),
        GlobPattern::new("**/dist/**".to_string()),
        GlobPattern::new("**/build/**".to_string()),
    ]
}

/// Watch ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WatchId(String);

impl WatchId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for WatchId {
    fn default() -> Self {
        Self::new()
    }
}

/// Watch handle
#[derive(Debug, Clone)]
pub struct WatchHandle {
    pub id: WatchId,
    pub path: PathBuf,
}

/// Watch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    /// Watch recursively
    pub recursive: bool,
    /// Follow symlinks
    pub follow_symlinks: bool,
    /// Poll interval (for polling mode)
    pub poll_interval_ms: Option<u64>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            recursive: true,
            follow_symlinks: true,
            poll_interval_ms: None,
        }
    }
}

impl WatchConfig {
    pub fn recursive() -> Self {
        Self { recursive: true, ..Default::default() }
    }

    pub fn non_recursive() -> Self {
        Self { recursive: false, ..Default::default() }
    }

    pub fn with_polling(mut self, interval_ms: u64) -> Self {
        self.poll_interval_ms = Some(interval_ms);
        self
    }
}

/// File change
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Changed path
    pub path: PathBuf,
    /// Change kind
    pub kind: FileChangeKind,
    /// Timestamp
    pub timestamp: std::time::SystemTime,
}

impl FileChange {
    pub fn created(path: PathBuf) -> Self {
        Self {
            path,
            kind: FileChangeKind::Created,
            timestamp: std::time::SystemTime::now(),
        }
    }

    pub fn modified(path: PathBuf) -> Self {
        Self {
            path,
            kind: FileChangeKind::Modified,
            timestamp: std::time::SystemTime::now(),
        }
    }

    pub fn deleted(path: PathBuf) -> Self {
        Self {
            path,
            kind: FileChangeKind::Deleted,
            timestamp: std::time::SystemTime::now(),
        }
    }

    pub fn renamed(old_path: PathBuf, new_path: PathBuf) -> Self {
        Self {
            path: new_path,
            kind: FileChangeKind::Renamed { from: old_path },
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// File change kind
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf },
}

impl FileChangeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::Renamed { .. } => "renamed",
        }
    }
}

/// Glob pattern
#[derive(Debug, Clone)]
pub struct GlobPattern {
    pattern: String,
}

impl GlobPattern {
    pub fn new(pattern: String) -> Self {
        Self { pattern }
    }

    pub fn matches(&self, path: &Path) -> bool {
        // Simplified glob matching
        let path_str = path.to_string_lossy();
        let pattern = &self.pattern;

        // Handle ** for recursive matching
        if pattern.contains("**") {
            let parts: Vec<&str> = pattern.split("**").collect();
            if parts.len() == 2 {
                let prefix = parts[0].trim_end_matches('/');
                let suffix = parts[1].trim_start_matches('/');

                if !prefix.is_empty() && !path_str.contains(prefix) {
                    return false;
                }
                if !suffix.is_empty() && !path_str.ends_with(suffix) {
                    return false;
                }
                return true;
            }
        }

        // Simple contains check
        path_str.contains(&self.pattern.replace("*", ""))
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }
}

/// File watcher configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWatcherConfig {
    /// Use polling instead of native events
    pub use_polling: bool,
    /// Poll interval in milliseconds
    pub poll_interval_ms: u64,
    /// Debounce delay in milliseconds
    pub debounce_ms: u64,
    /// Maximum watched paths
    pub max_watches: usize,
}

impl Default for FileWatcherConfig {
    fn default() -> Self {
        Self {
            use_polling: false,
            poll_interval_ms: 1000,
            debounce_ms: 100,
            max_watches: 10000,
        }
    }
}

/// File watcher event
#[derive(Debug, Clone)]
pub enum FileWatcherEvent {
    Started,
    Stopped,
    WatchStarted { path: PathBuf },
    WatchStopped { path: PathBuf },
    FileChanged { change: FileChange },
    Error { message: String },
}

/// Debouncer for file changes
pub struct FileChangeDebouncer {
    /// Pending changes
    pending: RwLock<HashMap<PathBuf, FileChange>>,
    /// Debounce duration
    duration: Duration,
}

impl FileChangeDebouncer {
    pub fn new(duration: Duration) -> Self {
        Self {
            pending: RwLock::new(HashMap::new()),
            duration,
        }
    }

    /// Add change to debounce queue
    pub fn add(&self, change: FileChange) {
        let mut pending = self.pending.write();

        // Merge with existing change if present
        if let Some(existing) = pending.get_mut(&change.path) {
            // Deleted takes precedence
            if matches!(change.kind, FileChangeKind::Deleted) {
                existing.kind = FileChangeKind::Deleted;
            }
            existing.timestamp = change.timestamp;
        } else {
            pending.insert(change.path.clone(), change);
        }
    }

    /// Flush pending changes that have been debounced
    pub fn flush(&self) -> Vec<FileChange> {
        let now = std::time::SystemTime::now();
        let mut pending = self.pending.write();
        let mut flushed = Vec::new();

        pending.retain(|_, change| {
            if let Ok(elapsed) = now.duration_since(change.timestamp) {
                if elapsed >= self.duration {
                    flushed.push(change.clone());
                    return false;
                }
            }
            true
        });

        flushed
    }

    /// Clear all pending
    pub fn clear(&self) {
        self.pending.write().clear();
    }
}

/// File watcher statistics
#[derive(Debug, Clone, Default)]
pub struct FileWatcherStats {
    pub watched_paths: usize,
    pub events_received: u64,
    pub events_processed: u64,
    pub events_excluded: u64,
}

impl FileWatcherStats {
    pub fn new() -> Self {
        Self::default()
    }
}
