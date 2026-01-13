//! File watcher for watch mode

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use notify::{Watcher, RecursiveMode, Event, EventKind};

/// File watcher
pub struct FileWatcher {
    /// Watcher instance
    watcher: Option<notify::RecommendedWatcher>,
    /// Watched paths
    watched: RwLock<Vec<PathBuf>>,
    /// Debounce duration
    debounce: Duration,
}

impl FileWatcher {
    pub fn new() -> Self {
        Self {
            watcher: None,
            watched: RwLock::new(Vec::new()),
            debounce: Duration::from_millis(100),
        }
    }

    /// Set debounce duration
    pub fn with_debounce(mut self, duration: Duration) -> Self {
        self.debounce = duration;
        self
    }

    /// Start watching with a callback
    pub fn watch<F>(&mut self, paths: Vec<PathBuf>, callback: F) -> anyhow::Result<()>
    where
        F: Fn(Vec<PathBuf>) + Send + Sync + 'static,
    {
        let callback = Arc::new(callback);
        let debounce = self.debounce;

        let (tx, mut rx) = mpsc::channel::<PathBuf>(100);

        // Create watcher
        let watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        for path in event.paths {
                            let _ = tx.blocking_send(path);
                        }
                    }
                    _ => {}
                }
            }
        })?;

        self.watcher = Some(watcher);

        // Add paths to watch
        if let Some(ref mut w) = self.watcher {
            for path in &paths {
                w.watch(path, RecursiveMode::Recursive)?;
            }
        }

        *self.watched.write() = paths;

        // Debounce and process events
        tokio::spawn(async move {
            let mut pending: Vec<PathBuf> = Vec::new();
            let mut timer = tokio::time::interval(debounce);
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    Some(path) = rx.recv() => {
                        if !pending.contains(&path) {
                            pending.push(path);
                        }
                    }
                    _ = timer.tick() => {
                        if !pending.is_empty() {
                            let paths = std::mem::take(&mut pending);
                            callback(paths);
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop watching
    pub fn stop(&mut self) {
        self.watcher = None;
        self.watched.write().clear();
    }

    /// Get watched paths
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched.read().clone()
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Glob pattern matcher for file watching
pub struct GlobMatcher {
    patterns: Vec<glob::Pattern>,
    ignore_patterns: Vec<glob::Pattern>,
}

impl GlobMatcher {
    pub fn new(patterns: Vec<&str>, ignore: Vec<&str>) -> Self {
        let patterns = patterns.iter()
            .filter_map(|p| glob::Pattern::new(p).ok())
            .collect();
        
        let ignore_patterns = ignore.iter()
            .filter_map(|p| glob::Pattern::new(p).ok())
            .collect();

        Self { patterns, ignore_patterns }
    }

    /// Check if path matches
    pub fn matches(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        // Check ignore patterns first
        for pattern in &self.ignore_patterns {
            if pattern.matches(&path_str) {
                return false;
            }
        }

        // Check match patterns
        for pattern in &self.patterns {
            if pattern.matches(&path_str) {
                return true;
            }
        }

        false
    }

    /// Default patterns for source files
    pub fn source_files() -> Self {
        Self::new(
            vec!["**/*.rs", "**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx", "**/*.py", "**/*.go"],
            vec!["**/node_modules/**", "**/target/**", "**/.git/**", "**/dist/**"],
        )
    }
}
