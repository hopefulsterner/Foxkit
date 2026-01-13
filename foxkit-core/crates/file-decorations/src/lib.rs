//! # Foxkit File Decorations
//!
//! File and folder decorations (badges, colors, icons).

use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// File decorations service
pub struct FileDecorationsService {
    /// Decoration providers
    providers: RwLock<Vec<Box<dyn FileDecorationProvider + Send + Sync>>>,
    /// Cached decorations
    cache: RwLock<HashMap<PathBuf, Vec<FileDecoration>>>,
    /// Event sender
    event_tx: broadcast::Sender<FileDecorationEvent>,
}

impl FileDecorationsService {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(256);

        Self {
            providers: RwLock::new(Vec::new()),
            cache: RwLock::new(HashMap::new()),
            event_tx,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<FileDecorationEvent> {
        self.event_tx.subscribe()
    }

    /// Register decoration provider
    pub fn register_provider(&self, provider: Box<dyn FileDecorationProvider + Send + Sync>) {
        self.providers.write().push(provider);
        let _ = self.event_tx.send(FileDecorationEvent::ProviderRegistered);
    }

    /// Get decorations for path
    pub fn get_decorations(&self, path: &PathBuf) -> Vec<FileDecoration> {
        // Check cache first
        if let Some(cached) = self.cache.read().get(path) {
            return cached.clone();
        }

        // Query providers
        let mut decorations = Vec::new();
        for provider in self.providers.read().iter() {
            if let Some(dec) = provider.provide_decoration(path) {
                decorations.push(dec);
            }
        }

        // Cache results
        self.cache.write().insert(path.clone(), decorations.clone());

        decorations
    }

    /// Get merged decoration
    pub fn get_merged_decoration(&self, path: &PathBuf) -> Option<FileDecoration> {
        let decorations = self.get_decorations(path);
        
        if decorations.is_empty() {
            return None;
        }

        // Merge decorations by priority
        let mut merged = FileDecoration::default();
        let mut has_content = false;

        for dec in decorations.iter().rev() {
            // Higher priority overwrites
            if merged.badge.is_none() {
                merged.badge = dec.badge.clone();
            }
            if merged.badge_tooltip.is_none() {
                merged.badge_tooltip = dec.badge_tooltip.clone();
            }
            if merged.color.is_none() {
                merged.color = dec.color.clone();
            }
            if merged.icon.is_none() {
                merged.icon = dec.icon.clone();
            }
            if merged.strikethrough.is_none() {
                merged.strikethrough = dec.strikethrough;
            }
            if merged.faded.is_none() {
                merged.faded = dec.faded;
            }
            has_content = true;
        }

        if has_content {
            Some(merged)
        } else {
            None
        }
    }

    /// Invalidate cache for path
    pub fn invalidate(&self, path: &PathBuf) {
        self.cache.write().remove(path);
        let _ = self.event_tx.send(FileDecorationEvent::Changed(vec![path.clone()]));
    }

    /// Invalidate all cache
    pub fn invalidate_all(&self) {
        let paths: Vec<_> = self.cache.read().keys().cloned().collect();
        self.cache.write().clear();
        let _ = self.event_tx.send(FileDecorationEvent::Changed(paths));
    }

    /// Notify of file changes
    pub fn notify_changes(&self, paths: Vec<PathBuf>) {
        for path in &paths {
            self.cache.write().remove(path);
        }
        let _ = self.event_tx.send(FileDecorationEvent::Changed(paths));
    }

    /// Set static decorations (for testing or simple use)
    pub fn set_decorations(&self, path: PathBuf, decorations: Vec<FileDecoration>) {
        self.cache.write().insert(path.clone(), decorations);
        let _ = self.event_tx.send(FileDecorationEvent::Changed(vec![path]));
    }
}

impl Default for FileDecorationsService {
    fn default() -> Self {
        Self::new()
    }
}

/// File decoration provider trait
pub trait FileDecorationProvider {
    /// Provide decoration for path
    fn provide_decoration(&self, path: &PathBuf) -> Option<FileDecoration>;
    
    /// Provider ID
    fn id(&self) -> &str;
}

/// File decoration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileDecoration {
    /// Badge text (short, 1-2 chars)
    pub badge: Option<String>,
    /// Badge tooltip
    pub badge_tooltip: Option<String>,
    /// Text color
    pub color: Option<String>,
    /// Custom icon
    pub icon: Option<String>,
    /// Strikethrough text
    pub strikethrough: Option<bool>,
    /// Faded/dimmed
    pub faded: Option<bool>,
    /// Propagate to parent folders
    pub propagate: bool,
    /// Priority (higher = more important)
    pub priority: i32,
}

impl FileDecoration {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn badge(mut self, text: impl Into<String>) -> Self {
        self.badge = Some(text.into());
        self
    }

    pub fn badge_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.badge_tooltip = Some(tooltip.into());
        self
    }

    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = Some(true);
        self
    }

    pub fn faded(mut self) -> Self {
        self.faded = Some(true);
        self
    }

    pub fn propagate(mut self) -> Self {
        self.propagate = true;
        self
    }

    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// Git status decoration provider
pub struct GitStatusDecorationProvider;

impl FileDecorationProvider for GitStatusDecorationProvider {
    fn provide_decoration(&self, _path: &PathBuf) -> Option<FileDecoration> {
        // Would integrate with git service
        None
    }

    fn id(&self) -> &str {
        "git-status"
    }
}

/// Common git decorations
pub mod git {
    use super::*;

    pub fn modified() -> FileDecoration {
        FileDecoration::new()
            .badge("M")
            .badge_tooltip("Modified")
            .color("gitDecoration.modifiedResourceForeground")
            .propagate()
    }

    pub fn added() -> FileDecoration {
        FileDecoration::new()
            .badge("A")
            .badge_tooltip("Added")
            .color("gitDecoration.addedResourceForeground")
            .propagate()
    }

    pub fn deleted() -> FileDecoration {
        FileDecoration::new()
            .badge("D")
            .badge_tooltip("Deleted")
            .color("gitDecoration.deletedResourceForeground")
            .strikethrough()
    }

    pub fn renamed() -> FileDecoration {
        FileDecoration::new()
            .badge("R")
            .badge_tooltip("Renamed")
            .color("gitDecoration.renamedResourceForeground")
    }

    pub fn untracked() -> FileDecoration {
        FileDecoration::new()
            .badge("U")
            .badge_tooltip("Untracked")
            .color("gitDecoration.untrackedResourceForeground")
    }

    pub fn ignored() -> FileDecoration {
        FileDecoration::new()
            .color("gitDecoration.ignoredResourceForeground")
            .faded()
    }

    pub fn conflict() -> FileDecoration {
        FileDecoration::new()
            .badge("!")
            .badge_tooltip("Conflict")
            .color("gitDecoration.conflictingResourceForeground")
            .priority(100)
    }

    pub fn submodule() -> FileDecoration {
        FileDecoration::new()
            .badge("S")
            .badge_tooltip("Submodule")
            .color("gitDecoration.submoduleResourceForeground")
    }
}

/// Error decoration provider
pub struct ErrorDecorationProvider {
    /// Files with errors
    error_files: RwLock<HashMap<PathBuf, ErrorDecorationData>>,
}

impl ErrorDecorationProvider {
    pub fn new() -> Self {
        Self {
            error_files: RwLock::new(HashMap::new()),
        }
    }

    pub fn set_errors(&self, path: PathBuf, errors: u32, warnings: u32) {
        self.error_files.write().insert(path, ErrorDecorationData { errors, warnings });
    }

    pub fn clear_errors(&self, path: &PathBuf) {
        self.error_files.write().remove(path);
    }
}

impl Default for ErrorDecorationProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct ErrorDecorationData {
    errors: u32,
    warnings: u32,
}

impl FileDecorationProvider for ErrorDecorationProvider {
    fn provide_decoration(&self, path: &PathBuf) -> Option<FileDecoration> {
        let files = self.error_files.read();
        let data = files.get(path)?;

        if data.errors > 0 {
            Some(FileDecoration::new()
                .badge(data.errors.to_string())
                .badge_tooltip(format!("{} error(s)", data.errors))
                .color("list.errorForeground")
                .priority(50))
        } else if data.warnings > 0 {
            Some(FileDecoration::new()
                .badge(data.warnings.to_string())
                .badge_tooltip(format!("{} warning(s)", data.warnings))
                .color("list.warningForeground")
                .priority(40))
        } else {
            None
        }
    }

    fn id(&self) -> &str {
        "diagnostics"
    }
}

/// File decoration event
#[derive(Debug, Clone)]
pub enum FileDecorationEvent {
    Changed(Vec<PathBuf>),
    ProviderRegistered,
}
