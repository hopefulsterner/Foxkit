//! # Foxkit Workspace Symbols
//!
//! Global symbol search across workspace.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Workspace symbols service
pub struct WorkspaceSymbolsService {
    /// Symbol index
    index: RwLock<SymbolIndex>,
    /// Events
    events: broadcast::Sender<WorkspaceSymbolsEvent>,
    /// Configuration
    config: RwLock<WorkspaceSymbolsConfig>,
}

impl WorkspaceSymbolsService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            index: RwLock::new(SymbolIndex::new()),
            events,
            config: RwLock::new(WorkspaceSymbolsConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<WorkspaceSymbolsEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: WorkspaceSymbolsConfig) {
        *self.config.write() = config;
    }

    /// Search workspace symbols
    pub async fn search(&self, query: &str) -> Vec<WorkspaceSymbol> {
        let config = self.config.read();

        if query.len() < config.min_query_length {
            return Vec::new();
        }

        // Would call LSP workspace/symbol
        // For now, search local index
        let index = self.index.read();
        let mut results = index.search(query, config.max_results);

        // Sort by relevance
        results.sort_by(|a, b| {
            // Exact matches first
            let a_exact = a.name.eq_ignore_ascii_case(query);
            let b_exact = b.name.eq_ignore_ascii_case(query);
            
            if a_exact != b_exact {
                return b_exact.cmp(&a_exact);
            }

            // Then by prefix match
            let a_prefix = a.name.to_lowercase().starts_with(&query.to_lowercase());
            let b_prefix = b.name.to_lowercase().starts_with(&query.to_lowercase());
            
            if a_prefix != b_prefix {
                return b_prefix.cmp(&a_prefix);
            }

            // Then by name length
            a.name.len().cmp(&b.name.len())
        });

        results
    }

    /// Index file symbols
    pub fn index_file(&self, file: PathBuf, symbols: Vec<WorkspaceSymbol>) {
        self.index.write().add_file(file.clone(), symbols);
        let _ = self.events.send(WorkspaceSymbolsEvent::FileIndexed { file });
    }

    /// Remove file from index
    pub fn remove_file(&self, file: &PathBuf) {
        self.index.write().remove_file(file);
    }

    /// Clear index
    pub fn clear(&self) {
        self.index.write().clear();
    }

    /// Get index stats
    pub fn stats(&self) -> IndexStats {
        self.index.read().stats()
    }
}

impl Default for WorkspaceSymbolsService {
    fn default() -> Self {
        Self::new()
    }
}

/// Symbol index
struct SymbolIndex {
    /// Symbols by file
    by_file: HashMap<PathBuf, Vec<WorkspaceSymbol>>,
    /// All symbols (flattened)
    all: Vec<WorkspaceSymbol>,
}

impl SymbolIndex {
    fn new() -> Self {
        Self {
            by_file: HashMap::new(),
            all: Vec::new(),
        }
    }

    fn add_file(&mut self, file: PathBuf, symbols: Vec<WorkspaceSymbol>) {
        // Remove existing
        self.remove_file(&file);

        // Add new
        for symbol in &symbols {
            self.all.push(symbol.clone());
        }
        self.by_file.insert(file, symbols);
    }

    fn remove_file(&mut self, file: &PathBuf) {
        if let Some(symbols) = self.by_file.remove(file) {
            self.all.retain(|s| &s.location.file != file);
        }
    }

    fn clear(&mut self) {
        self.by_file.clear();
        self.all.clear();
    }

    fn search(&self, query: &str, max_results: usize) -> Vec<WorkspaceSymbol> {
        let query_lower = query.to_lowercase();
        
        self.all
            .iter()
            .filter(|s| {
                let name_lower = s.name.to_lowercase();
                
                // Fuzzy match
                name_lower.contains(&query_lower) ||
                    fuzzy_match(&name_lower, &query_lower)
            })
            .take(max_results)
            .cloned()
            .collect()
    }

    fn stats(&self) -> IndexStats {
        IndexStats {
            file_count: self.by_file.len(),
            symbol_count: self.all.len(),
        }
    }
}

/// Simple fuzzy matching
fn fuzzy_match(text: &str, pattern: &str) -> bool {
    let mut pattern_chars = pattern.chars().peekable();
    
    for c in text.chars() {
        if pattern_chars.peek() == Some(&c) {
            pattern_chars.next();
        }
        if pattern_chars.peek().is_none() {
            return true;
        }
    }
    
    pattern_chars.peek().is_none()
}

/// Workspace symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSymbol {
    /// Symbol name
    pub name: String,
    /// Symbol kind
    pub kind: SymbolKind,
    /// Tags
    pub tags: Vec<SymbolTag>,
    /// Container name
    pub container_name: Option<String>,
    /// Location
    pub location: SymbolLocation,
    /// Data for resolution
    #[serde(skip)]
    pub data: Option<serde_json::Value>,
}

impl WorkspaceSymbol {
    pub fn new(name: impl Into<String>, kind: SymbolKind, location: SymbolLocation) -> Self {
        Self {
            name: name.into(),
            kind,
            tags: Vec::new(),
            container_name: None,
            location,
            data: None,
        }
    }

    pub fn with_container(mut self, container: impl Into<String>) -> Self {
        self.container_name = Some(container.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<SymbolTag>) -> Self {
        self.tags = tags;
        self
    }

    pub fn icon(&self) -> &'static str {
        self.kind.icon()
    }

    pub fn is_deprecated(&self) -> bool {
        self.tags.contains(&SymbolTag::Deprecated)
    }

    /// Format for display
    pub fn display_label(&self) -> String {
        if let Some(ref container) = self.container_name {
            format!("{}.{}", container, self.name)
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

/// Symbol location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolLocation {
    /// File path
    pub file: PathBuf,
    /// Range (optional for workspace symbols)
    pub range: Option<SymbolRange>,
}

impl SymbolLocation {
    pub fn file(path: PathBuf) -> Self {
        Self { file: path, range: None }
    }

    pub fn with_range(mut self, range: SymbolRange) -> Self {
        self.range = Some(range);
        self
    }
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
}

/// Index stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub file_count: usize,
    pub symbol_count: usize,
}

/// Workspace symbols configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSymbolsConfig {
    /// Minimum query length
    pub min_query_length: usize,
    /// Maximum results
    pub max_results: usize,
    /// Include deprecated
    pub include_deprecated: bool,
}

impl Default for WorkspaceSymbolsConfig {
    fn default() -> Self {
        Self {
            min_query_length: 1,
            max_results: 100,
            include_deprecated: true,
        }
    }
}

/// Workspace symbols event
#[derive(Debug, Clone)]
pub enum WorkspaceSymbolsEvent {
    FileIndexed { file: PathBuf },
    IndexCleared,
    SearchCompleted { query: String, count: usize },
}

/// Symbol picker view model
pub struct SymbolPickerViewModel {
    service: Arc<WorkspaceSymbolsService>,
    /// Current query
    query: RwLock<String>,
    /// Results
    results: RwLock<Vec<WorkspaceSymbol>>,
    /// Selected index
    selected: RwLock<usize>,
    /// Is loading
    loading: RwLock<bool>,
}

impl SymbolPickerViewModel {
    pub fn new(service: Arc<WorkspaceSymbolsService>) -> Self {
        Self {
            service,
            query: RwLock::new(String::new()),
            results: RwLock::new(Vec::new()),
            selected: RwLock::new(0),
            loading: RwLock::new(false),
        }
    }

    pub async fn search(&self, query: &str) {
        *self.query.write() = query.to_string();
        *self.loading.write() = true;

        let results = self.service.search(query).await;

        *self.results.write() = results;
        *self.selected.write() = 0;
        *self.loading.write() = false;
    }

    pub fn results(&self) -> Vec<WorkspaceSymbol> {
        self.results.read().clone()
    }

    pub fn select(&self, index: usize) {
        let len = self.results.read().len();
        if index < len {
            *self.selected.write() = index;
        }
    }

    pub fn selected(&self) -> Option<WorkspaceSymbol> {
        let index = *self.selected.read();
        self.results.read().get(index).cloned()
    }

    pub fn select_next(&self) {
        let mut selected = self.selected.write();
        let len = self.results.read().len();
        if *selected + 1 < len {
            *selected += 1;
        }
    }

    pub fn select_previous(&self) {
        let mut selected = self.selected.write();
        if *selected > 0 {
            *selected -= 1;
        }
    }

    pub fn is_loading(&self) -> bool {
        *self.loading.read()
    }
}
