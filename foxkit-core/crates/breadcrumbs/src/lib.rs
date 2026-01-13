//! # Foxkit Breadcrumbs
//!
//! Navigation breadcrumb trail for editor.

use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Breadcrumb service
pub struct BreadcrumbService {
    /// Cache by file
    cache: RwLock<HashMap<PathBuf, Vec<BreadcrumbItem>>>,
    /// Events
    events: broadcast::Sender<BreadcrumbEvent>,
    /// Configuration
    config: RwLock<BreadcrumbConfig>,
}

impl BreadcrumbService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);
        
        Self {
            cache: RwLock::new(HashMap::new()),
            events,
            config: RwLock::new(BreadcrumbConfig::default()),
        }
    }

    /// Configure breadcrumbs
    pub fn configure(&self, config: BreadcrumbConfig) {
        *self.config.write() = config;
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<BreadcrumbEvent> {
        self.events.subscribe()
    }

    /// Get breadcrumbs for position
    pub async fn get_breadcrumbs(
        &self,
        file: &PathBuf,
        line: u32,
        column: u32,
    ) -> Vec<BreadcrumbItem> {
        let config = self.config.read().clone();
        let mut items = Vec::new();

        // Add file path breadcrumbs
        if config.show_files {
            items.extend(self.get_path_breadcrumbs(file));
        }

        // Add symbol breadcrumbs
        if config.show_symbols {
            items.extend(self.get_symbol_breadcrumbs(file, line, column).await);
        }

        let _ = self.events.send(BreadcrumbEvent::Updated {
            items: items.clone(),
        });

        items
    }

    /// Get path breadcrumbs
    fn get_path_breadcrumbs(&self, file: &PathBuf) -> Vec<BreadcrumbItem> {
        let mut items = Vec::new();
        let mut current = PathBuf::new();

        for component in file.components() {
            current.push(component);
            
            let name = component.as_os_str()
                .to_string_lossy()
                .to_string();

            // Skip root and common prefixes
            if name == "/" || name.is_empty() {
                continue;
            }

            let kind = if current.is_dir() || !current.exists() && !name.contains('.') {
                BreadcrumbKind::Folder
            } else {
                BreadcrumbKind::File
            };

            items.push(BreadcrumbItem {
                label: name,
                kind,
                icon: kind.icon().to_string(),
                detail: None,
                path: Some(current.clone()),
                range: None,
                children: Vec::new(),
            });
        }

        items
    }

    /// Get symbol breadcrumbs
    async fn get_symbol_breadcrumbs(
        &self,
        file: &PathBuf,
        line: u32,
        column: u32,
    ) -> Vec<BreadcrumbItem> {
        // Would query LSP for document symbols and find containing symbols
        // For now, return empty
        Vec::new()
    }

    /// Invalidate cache
    pub fn invalidate(&self, file: &PathBuf) {
        self.cache.write().remove(file);
    }

    /// Clear all cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}

impl Default for BreadcrumbService {
    fn default() -> Self {
        Self::new()
    }
}

/// Breadcrumb item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreadcrumbItem {
    /// Display label
    pub label: String,
    /// Item kind
    pub kind: BreadcrumbKind,
    /// Icon ID
    pub icon: String,
    /// Optional detail
    pub detail: Option<String>,
    /// Path for file/folder items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// Range for symbol items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<BreadcrumbRange>,
    /// Child items (for picker)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<BreadcrumbItem>,
}

impl BreadcrumbItem {
    pub fn folder(name: impl Into<String>, path: PathBuf) -> Self {
        Self {
            label: name.into(),
            kind: BreadcrumbKind::Folder,
            icon: "folder".to_string(),
            detail: None,
            path: Some(path),
            range: None,
            children: Vec::new(),
        }
    }

    pub fn file(name: impl Into<String>, path: PathBuf) -> Self {
        Self {
            label: name.into(),
            kind: BreadcrumbKind::File,
            icon: "file".to_string(),
            detail: None,
            path: Some(path),
            range: None,
            children: Vec::new(),
        }
    }

    pub fn symbol(name: impl Into<String>, kind: BreadcrumbKind, range: BreadcrumbRange) -> Self {
        Self {
            label: name.into(),
            kind,
            icon: kind.icon().to_string(),
            detail: None,
            path: None,
            range: Some(range),
            children: Vec::new(),
        }
    }

    pub fn with_children(mut self, children: Vec<BreadcrumbItem>) -> Self {
        self.children = children;
        self
    }
}

/// Breadcrumb kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreadcrumbKind {
    File,
    Folder,
    Module,
    Namespace,
    Class,
    Method,
    Function,
    Constructor,
    Interface,
    Property,
    Field,
    Enum,
    EnumMember,
    Struct,
    Constant,
    Variable,
    TypeParameter,
}

impl BreadcrumbKind {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Folder => "folder",
            Self::Module => "module",
            Self::Namespace => "namespace",
            Self::Class => "class",
            Self::Method => "method",
            Self::Function => "function",
            Self::Constructor => "constructor",
            Self::Interface => "interface",
            Self::Property => "property",
            Self::Field => "field",
            Self::Enum => "enum",
            Self::EnumMember => "enum-member",
            Self::Struct => "struct",
            Self::Constant => "constant",
            Self::Variable => "variable",
            Self::TypeParameter => "type-parameter",
        }
    }
}

/// Breadcrumb range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreadcrumbRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl BreadcrumbRange {
    pub fn contains(&self, line: u32, col: u32) -> bool {
        if line < self.start_line || line > self.end_line {
            return false;
        }
        if line == self.start_line && col < self.start_col {
            return false;
        }
        if line == self.end_line && col > self.end_col {
            return false;
        }
        true
    }
}

/// Breadcrumb configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreadcrumbConfig {
    /// Show breadcrumbs
    pub enabled: bool,
    /// Show file path
    pub show_files: bool,
    /// Show symbols
    pub show_symbols: bool,
    /// Show icons
    pub show_icons: bool,
    /// File path style
    pub file_path: FilePathStyle,
    /// Symbol sort order
    pub symbol_sort: SymbolSortOrder,
}

impl Default for BreadcrumbConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_files: true,
            show_symbols: true,
            show_icons: true,
            file_path: FilePathStyle::On,
            symbol_sort: SymbolSortOrder::Position,
        }
    }
}

/// File path style
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FilePathStyle {
    /// Always show
    On,
    /// Never show
    Off,
    /// Show only last segment
    Last,
}

/// Symbol sort order
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SymbolSortOrder {
    /// By position in file
    Position,
    /// Alphabetically
    Name,
    /// By kind then name
    Kind,
}

/// Breadcrumb event
#[derive(Debug, Clone)]
pub enum BreadcrumbEvent {
    Updated { items: Vec<BreadcrumbItem> },
}

/// Breadcrumb picker
pub struct BreadcrumbPicker {
    /// Items to pick from
    items: Vec<BreadcrumbItem>,
    /// Selected index
    selected: usize,
}

impl BreadcrumbPicker {
    pub fn new(items: Vec<BreadcrumbItem>) -> Self {
        Self {
            items,
            selected: 0,
        }
    }

    pub fn items(&self) -> &[BreadcrumbItem] {
        &self.items
    }

    pub fn select(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected = index;
        }
    }

    pub fn selected(&self) -> Option<&BreadcrumbItem> {
        self.items.get(self.selected)
    }

    pub fn select_next(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
        }
    }

    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
}
