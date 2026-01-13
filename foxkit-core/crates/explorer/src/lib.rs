//! # Foxkit Explorer
//!
//! File explorer sidebar.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Explorer service
pub struct ExplorerService {
    /// Workspace roots
    roots: RwLock<Vec<ExplorerRoot>>,
    /// Expanded folders
    expanded: RwLock<HashSet<PathBuf>>,
    /// Selected items
    selected: RwLock<Vec<PathBuf>>,
    /// Events
    events: broadcast::Sender<ExplorerEvent>,
    /// Configuration
    config: RwLock<ExplorerConfig>,
}

impl ExplorerService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            roots: RwLock::new(Vec::new()),
            expanded: RwLock::new(HashSet::new()),
            selected: RwLock::new(Vec::new()),
            events,
            config: RwLock::new(ExplorerConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<ExplorerEvent> {
        self.events.subscribe()
    }

    /// Configure explorer
    pub fn configure(&self, config: ExplorerConfig) {
        *self.config.write() = config;
        let _ = self.events.send(ExplorerEvent::ConfigChanged);
    }

    /// Add workspace root
    pub fn add_root(&self, path: PathBuf) {
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("root")
            .to_string();

        let root = ExplorerRoot {
            path: path.clone(),
            name,
        };

        self.roots.write().push(root);
        self.expanded.write().insert(path.clone());

        let _ = self.events.send(ExplorerEvent::RootAdded { path });
    }

    /// Remove workspace root
    pub fn remove_root(&self, path: &PathBuf) {
        self.roots.write().retain(|r| &r.path != path);
        let _ = self.events.send(ExplorerEvent::RootRemoved { path: path.clone() });
    }

    /// Get roots
    pub fn roots(&self) -> Vec<ExplorerRoot> {
        self.roots.read().clone()
    }

    /// Is folder expanded
    pub fn is_expanded(&self, path: &PathBuf) -> bool {
        self.expanded.read().contains(path)
    }

    /// Expand folder
    pub fn expand(&self, path: PathBuf) {
        self.expanded.write().insert(path.clone());
        let _ = self.events.send(ExplorerEvent::Expanded { path });
    }

    /// Collapse folder
    pub fn collapse(&self, path: &PathBuf) {
        self.expanded.write().remove(path);
        let _ = self.events.send(ExplorerEvent::Collapsed { path: path.clone() });
    }

    /// Toggle expanded
    pub fn toggle_expanded(&self, path: PathBuf) {
        if self.is_expanded(&path) {
            self.collapse(&path);
        } else {
            self.expand(path);
        }
    }

    /// Expand to path
    pub fn expand_to(&self, target: &PathBuf) {
        let mut current = target.clone();
        let mut to_expand = Vec::new();

        while let Some(parent) = current.parent() {
            to_expand.push(parent.to_path_buf());
            current = parent.to_path_buf();
        }

        for path in to_expand.into_iter().rev() {
            self.expand(path);
        }
    }

    /// Select item
    pub fn select(&self, path: PathBuf, add: bool) {
        let mut selected = self.selected.write();
        
        if add {
            if selected.contains(&path) {
                selected.retain(|p| p != &path);
            } else {
                selected.push(path.clone());
            }
        } else {
            selected.clear();
            selected.push(path.clone());
        }

        let _ = self.events.send(ExplorerEvent::SelectionChanged);
    }

    /// Select range
    pub fn select_range(&self, from: &PathBuf, to: &PathBuf) {
        // Would need tree structure to calculate range
    }

    /// Get selected items
    pub fn selected(&self) -> Vec<PathBuf> {
        self.selected.read().clone()
    }

    /// Clear selection
    pub fn clear_selection(&self) {
        self.selected.write().clear();
        let _ = self.events.send(ExplorerEvent::SelectionChanged);
    }

    /// Reveal path
    pub fn reveal(&self, path: &PathBuf) {
        self.expand_to(path);
        self.select(path.clone(), false);
        let _ = self.events.send(ExplorerEvent::Revealed { path: path.clone() });
    }

    /// Refresh
    pub fn refresh(&self) {
        let _ = self.events.send(ExplorerEvent::Refresh);
    }
}

impl Default for ExplorerService {
    fn default() -> Self {
        Self::new()
    }
}

/// Explorer root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerRoot {
    pub path: PathBuf,
    pub name: String,
}

/// Explorer item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerItem {
    /// File path
    pub path: PathBuf,
    /// Display name
    pub name: String,
    /// Item kind
    pub kind: ExplorerItemKind,
    /// Icon
    pub icon: Option<String>,
    /// Is hidden
    pub is_hidden: bool,
    /// Is symlink
    pub is_symlink: bool,
    /// Size (for files)
    pub size: Option<u64>,
    /// Modified time
    pub modified: Option<u64>,
}

impl ExplorerItem {
    pub fn file(path: PathBuf) -> Self {
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        Self {
            path,
            name: name.clone(),
            kind: ExplorerItemKind::File,
            icon: Some(Self::icon_for_file(&name)),
            is_hidden: name.starts_with('.'),
            is_symlink: false,
            size: None,
            modified: None,
        }
    }

    pub fn folder(path: PathBuf) -> Self {
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        Self {
            path,
            name: name.clone(),
            kind: ExplorerItemKind::Folder,
            icon: Some(Self::icon_for_folder(&name)),
            is_hidden: name.starts_with('.'),
            is_symlink: false,
            size: None,
            modified: None,
        }
    }

    fn icon_for_file(name: &str) -> String {
        let extension = name.rsplit('.').next().unwrap_or("");
        
        match extension {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "py" => "python",
            "go" => "go",
            "java" => "java",
            "c" | "h" => "c",
            "cpp" | "cc" | "cxx" | "hpp" => "cpp",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            "md" => "markdown",
            "html" => "html",
            "css" => "css",
            "scss" | "sass" => "sass",
            "svg" => "svg",
            "png" | "jpg" | "jpeg" | "gif" | "webp" => "image",
            "pdf" => "pdf",
            "zip" | "tar" | "gz" => "archive",
            _ => "file",
        }.to_string()
    }

    fn icon_for_folder(name: &str) -> String {
        match name {
            "src" => "folder-src",
            "lib" => "folder-lib",
            "test" | "tests" | "__tests__" => "folder-test",
            "doc" | "docs" => "folder-docs",
            "node_modules" => "folder-node",
            "target" => "folder-target",
            ".git" => "folder-git",
            ".github" => "folder-github",
            ".vscode" => "folder-vscode",
            "dist" | "build" | "out" => "folder-dist",
            "public" | "static" | "assets" => "folder-public",
            "components" => "folder-components",
            "hooks" => "folder-hooks",
            "utils" | "helpers" => "folder-utils",
            "config" => "folder-config",
            "scripts" => "folder-scripts",
            _ => "folder",
        }.to_string()
    }
}

/// Explorer item kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExplorerItemKind {
    File,
    Folder,
    Root,
}

/// Explorer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerConfig {
    /// Show hidden files
    pub show_hidden: bool,
    /// Sort order
    pub sort_order: SortOrder,
    /// Sort direction
    pub sort_direction: SortDirection,
    /// Compact folders (single child collapse)
    pub compact_folders: bool,
    /// Auto reveal active file
    pub auto_reveal: bool,
    /// Auto reveal delay (ms)
    pub auto_reveal_delay: u32,
    /// Exclude patterns
    pub exclude: Vec<String>,
    /// File nesting rules
    pub file_nesting: FileNestingConfig,
}

impl Default for ExplorerConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            sort_order: SortOrder::Type,
            sort_direction: SortDirection::Ascending,
            compact_folders: true,
            auto_reveal: true,
            auto_reveal_delay: 500,
            exclude: vec![
                "**/.git".to_string(),
                "**/node_modules".to_string(),
                "**/target".to_string(),
            ],
            file_nesting: FileNestingConfig::default(),
        }
    }
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Name,
    Type,
    Modified,
    Size,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// File nesting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNestingConfig {
    /// Enable file nesting
    pub enabled: bool,
    /// Nesting patterns
    pub patterns: HashMap<String, Vec<String>>,
}

impl Default for FileNestingConfig {
    fn default() -> Self {
        let mut patterns = HashMap::new();
        
        // Common nesting patterns
        patterns.insert("*.ts".to_string(), vec![
            "${capture}.js".to_string(),
            "${capture}.d.ts".to_string(),
            "${capture}.js.map".to_string(),
        ]);
        patterns.insert("package.json".to_string(), vec![
            "package-lock.json".to_string(),
            "yarn.lock".to_string(),
            "pnpm-lock.yaml".to_string(),
            ".npmrc".to_string(),
        ]);
        patterns.insert("Cargo.toml".to_string(), vec![
            "Cargo.lock".to_string(),
        ]);
        patterns.insert("*.rs".to_string(), vec![
            "${capture}.generated.rs".to_string(),
        ]);

        Self {
            enabled: true,
            patterns,
        }
    }
}

/// Explorer event
#[derive(Debug, Clone)]
pub enum ExplorerEvent {
    RootAdded { path: PathBuf },
    RootRemoved { path: PathBuf },
    Expanded { path: PathBuf },
    Collapsed { path: PathBuf },
    SelectionChanged,
    Revealed { path: PathBuf },
    ConfigChanged,
    Refresh,
    FileCreated { path: PathBuf },
    FileDeleted { path: PathBuf },
    FileRenamed { old: PathBuf, new: PathBuf },
}

/// Explorer view model
pub struct ExplorerViewModel {
    service: Arc<ExplorerService>,
    /// Filter text
    filter: RwLock<String>,
    /// Focused item
    focused: RwLock<Option<PathBuf>>,
    /// Cut items
    cut: RwLock<Vec<PathBuf>>,
    /// Copied items
    copied: RwLock<Vec<PathBuf>>,
}

impl ExplorerViewModel {
    pub fn new(service: Arc<ExplorerService>) -> Self {
        Self {
            service,
            filter: RwLock::new(String::new()),
            focused: RwLock::new(None),
            cut: RwLock::new(Vec::new()),
            copied: RwLock::new(Vec::new()),
        }
    }

    pub fn roots(&self) -> Vec<ExplorerRoot> {
        self.service.roots()
    }

    pub fn is_expanded(&self, path: &PathBuf) -> bool {
        self.service.is_expanded(path)
    }

    pub fn toggle(&self, path: PathBuf) {
        self.service.toggle_expanded(path);
    }

    pub fn select(&self, path: PathBuf, extend: bool) {
        self.service.select(path.clone(), extend);
        *self.focused.write() = Some(path);
    }

    pub fn selected(&self) -> Vec<PathBuf> {
        self.service.selected()
    }

    pub fn focused(&self) -> Option<PathBuf> {
        self.focused.read().clone()
    }

    pub fn cut(&self) {
        let selected = self.selected();
        *self.cut.write() = selected;
        *self.copied.write() = Vec::new();
    }

    pub fn copy(&self) {
        let selected = self.selected();
        *self.copied.write() = selected;
        *self.cut.write() = Vec::new();
    }

    pub async fn paste(&self, target: &PathBuf) -> anyhow::Result<()> {
        // Would implement paste logic
        Ok(())
    }

    pub async fn delete(&self) -> anyhow::Result<()> {
        // Would implement delete logic
        Ok(())
    }

    pub async fn rename(&self, path: &PathBuf, new_name: &str) -> anyhow::Result<()> {
        // Would implement rename logic
        Ok(())
    }

    pub async fn create_file(&self, parent: &PathBuf, name: &str) -> anyhow::Result<PathBuf> {
        // Would implement file creation
        Ok(parent.join(name))
    }

    pub async fn create_folder(&self, parent: &PathBuf, name: &str) -> anyhow::Result<PathBuf> {
        // Would implement folder creation
        Ok(parent.join(name))
    }

    pub fn set_filter(&self, filter: String) {
        *self.filter.write() = filter;
    }

    pub fn filter(&self) -> String {
        self.filter.read().clone()
    }

    pub fn reveal(&self, path: &PathBuf) {
        self.service.reveal(path);
    }

    pub fn collapse_all(&self) {
        // Would collapse all folders
    }

    pub fn refresh(&self) {
        self.service.refresh();
    }
}
