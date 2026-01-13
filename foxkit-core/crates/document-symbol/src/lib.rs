//! # Foxkit Document Symbol
//!
//! Document symbols and structure provider.

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Document symbol service
pub struct DocumentSymbolService {
    /// Cached symbols per file
    cache: RwLock<std::collections::HashMap<PathBuf, CachedSymbols>>,
    /// Events
    events: broadcast::Sender<DocumentSymbolEvent>,
    /// Configuration
    config: RwLock<DocumentSymbolConfig>,
}

impl DocumentSymbolService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            cache: RwLock::new(std::collections::HashMap::new()),
            events,
            config: RwLock::new(DocumentSymbolConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<DocumentSymbolEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: DocumentSymbolConfig) {
        *self.config.write() = config;
    }

    /// Get symbols for file (would call LSP)
    pub async fn get_symbols(&self, file: &PathBuf) -> Vec<DocumentSymbol> {
        // Check cache first
        if let Some(cached) = self.cache.read().get(file) {
            if !cached.is_stale() {
                return cached.symbols.clone();
            }
        }

        // Would call LSP textDocument/documentSymbol
        let symbols = Vec::new();

        // Update cache
        self.cache.write().insert(file.clone(), CachedSymbols {
            symbols: symbols.clone(),
            timestamp: std::time::Instant::now(),
        });

        symbols
    }

    /// Set symbols for file
    pub fn set_symbols(&self, file: PathBuf, symbols: Vec<DocumentSymbol>) {
        self.cache.write().insert(file.clone(), CachedSymbols {
            symbols: symbols.clone(),
            timestamp: std::time::Instant::now(),
        });

        let _ = self.events.send(DocumentSymbolEvent::SymbolsUpdated {
            file,
            count: symbols.len(),
        });
    }

    /// Invalidate cache for file
    pub fn invalidate(&self, file: &PathBuf) {
        self.cache.write().remove(file);
    }

    /// Clear all cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }

    /// Get flat list of all symbols
    pub fn flatten_symbols(symbols: &[DocumentSymbol]) -> Vec<FlatSymbol> {
        let mut result = Vec::new();
        Self::flatten_recursive(symbols, &mut result, 0, None);
        result
    }

    fn flatten_recursive(
        symbols: &[DocumentSymbol],
        result: &mut Vec<FlatSymbol>,
        depth: u32,
        parent: Option<String>,
    ) {
        for symbol in symbols {
            result.push(FlatSymbol {
                name: symbol.name.clone(),
                kind: symbol.kind,
                range: symbol.range.clone(),
                selection_range: symbol.selection_range.clone(),
                depth,
                parent: parent.clone(),
            });

            Self::flatten_recursive(
                &symbol.children,
                result,
                depth + 1,
                Some(symbol.name.clone()),
            );
        }
    }

    /// Find symbol at position
    pub fn symbol_at_position(
        symbols: &[DocumentSymbol],
        line: u32,
        col: u32,
    ) -> Option<&DocumentSymbol> {
        for symbol in symbols {
            if symbol.range.contains(line, col) {
                // Check children first (more specific)
                if let Some(child) = Self::symbol_at_position(&symbol.children, line, col) {
                    return Some(child);
                }
                return Some(symbol);
            }
        }
        None
    }

    /// Get breadcrumb path to position
    pub fn breadcrumb_path(
        symbols: &[DocumentSymbol],
        line: u32,
        col: u32,
    ) -> Vec<BreadcrumbItem> {
        let mut path = Vec::new();
        Self::breadcrumb_recursive(symbols, line, col, &mut path);
        path
    }

    fn breadcrumb_recursive(
        symbols: &[DocumentSymbol],
        line: u32,
        col: u32,
        path: &mut Vec<BreadcrumbItem>,
    ) {
        for symbol in symbols {
            if symbol.range.contains(line, col) {
                path.push(BreadcrumbItem {
                    name: symbol.name.clone(),
                    kind: symbol.kind,
                    range: symbol.selection_range.clone(),
                });
                Self::breadcrumb_recursive(&symbol.children, line, col, path);
                break;
            }
        }
    }
}

impl Default for DocumentSymbolService {
    fn default() -> Self {
        Self::new()
    }
}

/// Cached symbols
struct CachedSymbols {
    symbols: Vec<DocumentSymbol>,
    timestamp: std::time::Instant,
}

impl CachedSymbols {
    fn is_stale(&self) -> bool {
        self.timestamp.elapsed().as_secs() > 30
    }
}

/// Document symbol (hierarchical)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSymbol {
    /// Symbol name
    pub name: String,
    /// Detail (e.g., signature)
    pub detail: Option<String>,
    /// Symbol kind
    pub kind: SymbolKind,
    /// Tags
    pub tags: Vec<SymbolTag>,
    /// Full range
    pub range: SymbolRange,
    /// Selection range (for navigation)
    pub selection_range: SymbolRange,
    /// Children
    pub children: Vec<DocumentSymbol>,
}

impl DocumentSymbol {
    pub fn new(name: impl Into<String>, kind: SymbolKind, range: SymbolRange) -> Self {
        Self {
            name: name.into(),
            detail: None,
            kind,
            tags: Vec::new(),
            range: range.clone(),
            selection_range: range,
            children: Vec::new(),
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_selection_range(mut self, range: SymbolRange) -> Self {
        self.selection_range = range;
        self
    }

    pub fn with_children(mut self, children: Vec<DocumentSymbol>) -> Self {
        self.children = children;
        self
    }

    pub fn deprecated(mut self) -> Self {
        self.tags.push(SymbolTag::Deprecated);
        self
    }

    pub fn is_deprecated(&self) -> bool {
        self.tags.contains(&SymbolTag::Deprecated)
    }

    pub fn icon(&self) -> &'static str {
        self.kind.icon()
    }

    pub fn label(&self) -> String {
        if let Some(ref detail) = self.detail {
            format!("{} {}", self.name, detail)
        } else {
            self.name.clone()
        }
    }
}

/// Symbol kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

impl SymbolKind {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::File => "$(file)",
            Self::Module => "$(package)",
            Self::Namespace => "$(symbol-namespace)",
            Self::Package => "$(package)",
            Self::Class => "$(symbol-class)",
            Self::Method => "$(symbol-method)",
            Self::Property => "$(symbol-property)",
            Self::Field => "$(symbol-field)",
            Self::Constructor => "$(symbol-constructor)",
            Self::Enum => "$(symbol-enum)",
            Self::Interface => "$(symbol-interface)",
            Self::Function => "$(symbol-function)",
            Self::Variable => "$(symbol-variable)",
            Self::Constant => "$(symbol-constant)",
            Self::String => "$(symbol-string)",
            Self::Number => "$(symbol-number)",
            Self::Boolean => "$(symbol-boolean)",
            Self::Array => "$(symbol-array)",
            Self::Object => "$(symbol-object)",
            Self::Key => "$(symbol-key)",
            Self::Null => "$(symbol-null)",
            Self::EnumMember => "$(symbol-enum-member)",
            Self::Struct => "$(symbol-struct)",
            Self::Event => "$(symbol-event)",
            Self::Operator => "$(symbol-operator)",
            Self::TypeParameter => "$(symbol-type-parameter)",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Module => "module",
            Self::Namespace => "namespace",
            Self::Package => "package",
            Self::Class => "class",
            Self::Method => "method",
            Self::Property => "property",
            Self::Field => "field",
            Self::Constructor => "constructor",
            Self::Enum => "enum",
            Self::Interface => "interface",
            Self::Function => "function",
            Self::Variable => "variable",
            Self::Constant => "constant",
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Array => "array",
            Self::Object => "object",
            Self::Key => "key",
            Self::Null => "null",
            Self::EnumMember => "enum member",
            Self::Struct => "struct",
            Self::Event => "event",
            Self::Operator => "operator",
            Self::TypeParameter => "type parameter",
        }
    }
}

/// Symbol tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolTag {
    Deprecated,
}

/// Symbol range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl SymbolRange {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
    }

    pub fn single_line(line: u32, start_col: u32, end_col: u32) -> Self {
        Self { start_line: line, start_col, end_line: line, end_col }
    }

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

/// Flat symbol (for list view)
#[derive(Debug, Clone)]
pub struct FlatSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: SymbolRange,
    pub selection_range: SymbolRange,
    pub depth: u32,
    pub parent: Option<String>,
}

impl FlatSymbol {
    pub fn indent(&self) -> String {
        "  ".repeat(self.depth as usize)
    }
}

/// Breadcrumb item
#[derive(Debug, Clone)]
pub struct BreadcrumbItem {
    pub name: String,
    pub kind: SymbolKind,
    pub range: SymbolRange,
}

/// Document symbol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSymbolConfig {
    /// Show deprecated symbols
    pub show_deprecated: bool,
    /// Symbol sort order
    pub sort_by: SymbolSortOrder,
}

impl Default for DocumentSymbolConfig {
    fn default() -> Self {
        Self {
            show_deprecated: true,
            sort_by: SymbolSortOrder::Position,
        }
    }
}

/// Symbol sort order
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SymbolSortOrder {
    Position,
    Name,
    Kind,
}

/// Document symbol event
#[derive(Debug, Clone)]
pub enum DocumentSymbolEvent {
    SymbolsUpdated { file: PathBuf, count: usize },
    CacheInvalidated { file: PathBuf },
}

/// Symbol tree view model
pub struct SymbolTreeViewModel {
    service: Arc<DocumentSymbolService>,
    /// Expanded nodes
    expanded: RwLock<std::collections::HashSet<String>>,
    /// Selected symbol
    selected: RwLock<Option<String>>,
    /// Current file
    file: RwLock<Option<PathBuf>>,
}

impl SymbolTreeViewModel {
    pub fn new(service: Arc<DocumentSymbolService>) -> Self {
        Self {
            service,
            expanded: RwLock::new(std::collections::HashSet::new()),
            selected: RwLock::new(None),
            file: RwLock::new(None),
        }
    }

    pub fn set_file(&self, file: PathBuf) {
        *self.file.write() = Some(file);
        self.expanded.write().clear();
        *self.selected.write() = None;
    }

    pub async fn symbols(&self) -> Vec<DocumentSymbol> {
        if let Some(ref file) = *self.file.read() {
            self.service.get_symbols(file).await
        } else {
            Vec::new()
        }
    }

    pub fn toggle_expand(&self, name: &str) {
        let mut expanded = self.expanded.write();
        if expanded.contains(name) {
            expanded.remove(name);
        } else {
            expanded.insert(name.to_string());
        }
    }

    pub fn is_expanded(&self, name: &str) -> bool {
        self.expanded.read().contains(name)
    }

    pub fn expand_all(&self, symbols: &[DocumentSymbol]) {
        let mut expanded = self.expanded.write();
        Self::expand_recursive(symbols, &mut expanded);
    }

    fn expand_recursive(symbols: &[DocumentSymbol], expanded: &mut std::collections::HashSet<String>) {
        for symbol in symbols {
            if !symbol.children.is_empty() {
                expanded.insert(symbol.name.clone());
                Self::expand_recursive(&symbol.children, expanded);
            }
        }
    }

    pub fn collapse_all(&self) {
        self.expanded.write().clear();
    }

    pub fn select(&self, name: &str) {
        *self.selected.write() = Some(name.to_string());
    }

    pub fn selected(&self) -> Option<String> {
        self.selected.read().clone()
    }
}
