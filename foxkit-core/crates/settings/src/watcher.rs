//! Settings file watcher

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// Settings change event
#[derive(Debug, Clone)]
pub enum SettingsEvent {
    /// Settings file changed
    Changed(PathBuf),
    /// Settings file deleted
    Deleted(PathBuf),
    /// Error watching file
    Error(String),
}

/// Watch settings files for changes
pub struct SettingsWatcher {
    paths: Vec<PathBuf>,
    // In real impl, would use notify crate
}

impl SettingsWatcher {
    pub fn new() -> Self {
        Self {
            paths: Vec::new(),
        }
    }

    /// Watch a settings file
    pub fn watch(&mut self, path: PathBuf) {
        if !self.paths.contains(&path) {
            self.paths.push(path);
        }
    }

    /// Stop watching a file
    pub fn unwatch(&mut self, path: &Path) {
        self.paths.retain(|p| p != path);
    }

    /// Get watched paths
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
}

impl Default for SettingsWatcher {
    fn default() -> Self {
        Self::new()
    }
}
