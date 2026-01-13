//! # Foxkit Outline Panel
//!
//! Document structure outline view.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Outline panel service
pub struct OutlineService {
    /// Cached outlines
    cache: RwLock<HashMap<PathBuf, OutlineTree>>,
    /// Events
    events: broadcast::Sender<OutlineEvent>,
    /// Configuration
    config: RwLock<OutlineConfig>,
}

impl OutlineService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            cache: RwLock::new(HashMap::new()),
            events,
            config: RwLock::new(OutlineConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<OutlineEvent> {
        self.events.subscribe()
    }

    /// Configure outline
    pub fn configure(&self, config: OutlineConfig) {
        *self.config.write() = config;
    }

    /// Get outline for file
    pub async fn get_outline(&self, file: &PathBuf) -> Option<OutlineTree> {
        // Check cache
        if let Some(cached) = self.cache.read().get(file) {
            return Some(cached.clone());
        }

        // Would request from LSP
        None
    }

    /// Set outline (from LSP response)
    pub fn set_outline(&self, file: PathBuf, tree: OutlineTree) {
        self.cache.write().insert(file.clone(), tree);
        let _ = self.events.send(OutlineEvent::Updated { file });
    }

    /// Invalidate outline
    pub fn invalidate(&self, file: &PathBuf) {
        self.cache.write().remove(file);
        let _ = self.events.send(OutlineEvent::Invalidated { file: file.clone() });
    }

    /// Clear cache
    pub fn clear(&self) {
        self.cache.write().clear();
    }
}

impl Default for OutlineService {
    fn default() -> Self {
        Self::new()
    }
}

/// Outline tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineTree {
    /// Root nodes
    pub roots: Vec<OutlineNode>,
}

impl OutlineTree {
    pub fn new(roots: Vec<OutlineNode>) -> Self {
        Self { roots }
    }

    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    /// Flatten tree to list
    pub fn flatten(&self) -> Vec<FlatOutlineNode> {
        let mut result = Vec::new();
        for root in &self.roots {
            Self::flatten_node(root, 0, &mut result);
        }
        result
    }

    fn flatten_node(node: &OutlineNode, depth: usize, result: &mut Vec<FlatOutlineNode>) {
        result.push(FlatOutlineNode {
            node: node.clone(),
            depth,
        });
        for child in &node.children {
            Self::flatten_node(child, depth + 1, result);
        }
    }

    /// Filter by kind
    pub fn filter_by_kind(&self, kinds: &[OutlineKind]) -> OutlineTree {
        let roots = self.roots.iter()
            .filter_map(|n| Self::filter_node(n, kinds))
            .collect();
        OutlineTree { roots }
    }

    fn filter_node(node: &OutlineNode, kinds: &[OutlineKind]) -> Option<OutlineNode> {
        let children: Vec<_> = node.children.iter()
            .filter_map(|c| Self::filter_node(c, kinds))
            .collect();

        if kinds.contains(&node.kind) || !children.is_empty() {
            Some(OutlineNode {
                children,
                ..node.clone()
            })
        } else {
            None
        }
    }

    /// Sort tree
    pub fn sort(&mut self, order: OutlineSortOrder) {
        Self::sort_nodes(&mut self.roots, order);
    }

    fn sort_nodes(nodes: &mut [OutlineNode], order: OutlineSortOrder) {
        match order {
            OutlineSortOrder::Position => {
                nodes.sort_by_key(|n| n.range.start_line);
            }
            OutlineSortOrder::Name => {
                nodes.sort_by(|a, b| a.name.cmp(&b.name));
            }
            OutlineSortOrder::Kind => {
                nodes.sort_by_key(|n| n.kind as u8);
            }
        }

        for node in nodes {
            Self::sort_nodes(&mut node.children, order);
        }
    }

    /// Find node at position
    pub fn find_at_position(&self, line: u32, column: u32) -> Option<&OutlineNode> {
        for root in &self.roots {
            if let Some(node) = Self::find_in_node(root, line, column) {
                return Some(node);
            }
        }
        None
    }

    fn find_in_node(node: &OutlineNode, line: u32, column: u32) -> Option<&OutlineNode> {
        if !node.range.contains(line, column) {
            return None;
        }

        // Check children first (more specific)
        for child in &node.children {
            if let Some(found) = Self::find_in_node(child, line, column) {
                return Some(found);
            }
        }

        Some(node)
    }
}

/// Outline node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineNode {
    /// Symbol name
    pub name: String,
    /// Detail (e.g., type signature)
    pub detail: Option<String>,
    /// Kind
    pub kind: OutlineKind,
    /// Range in document
    pub range: OutlineRange,
    /// Selection range
    pub selection_range: OutlineRange,
    /// Children
    pub children: Vec<OutlineNode>,
    /// Tags
    pub tags: Vec<OutlineTag>,
}

impl OutlineNode {
    pub fn new(name: impl Into<String>, kind: OutlineKind, range: OutlineRange) -> Self {
        Self {
            name: name.into(),
            detail: None,
            kind,
            range: range.clone(),
            selection_range: range,
            children: Vec::new(),
            tags: Vec::new(),
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_children(mut self, children: Vec<OutlineNode>) -> Self {
        self.children = children;
        self
    }

    pub fn with_tags(mut self, tags: Vec<OutlineTag>) -> Self {
        self.tags = tags;
        self
    }

    pub fn icon(&self) -> &'static str {
        self.kind.icon()
    }

    pub fn is_deprecated(&self) -> bool {
        self.tags.contains(&OutlineTag::Deprecated)
    }
}

/// Flat outline node (for list view)
#[derive(Debug, Clone)]
pub struct FlatOutlineNode {
    pub node: OutlineNode,
    pub depth: usize,
}

/// Outline kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OutlineKind {
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

impl OutlineKind {
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
            Self::File => "File",
            Self::Module => "Module",
            Self::Namespace => "Namespace",
            Self::Package => "Package",
            Self::Class => "Class",
            Self::Method => "Method",
            Self::Property => "Property",
            Self::Field => "Field",
            Self::Constructor => "Constructor",
            Self::Enum => "Enum",
            Self::Interface => "Interface",
            Self::Function => "Function",
            Self::Variable => "Variable",
            Self::Constant => "Constant",
            Self::String => "String",
            Self::Number => "Number",
            Self::Boolean => "Boolean",
            Self::Array => "Array",
            Self::Object => "Object",
            Self::Key => "Key",
            Self::Null => "Null",
            Self::EnumMember => "Enum Member",
            Self::Struct => "Struct",
            Self::Event => "Event",
            Self::Operator => "Operator",
            Self::TypeParameter => "Type Parameter",
        }
    }
}

/// Outline range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl OutlineRange {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
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

/// Outline tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutlineTag {
    Deprecated,
}

/// Outline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineConfig {
    /// Follow cursor
    pub follow_cursor: bool,
    /// Sort order
    pub sort_order: OutlineSortOrder,
    /// Filter by kind
    pub filter_kinds: Vec<OutlineKind>,
    /// Show icons
    pub show_icons: bool,
    /// Show details
    pub show_details: bool,
}

impl Default for OutlineConfig {
    fn default() -> Self {
        Self {
            follow_cursor: true,
            sort_order: OutlineSortOrder::Position,
            filter_kinds: Vec::new(), // All kinds
            show_icons: true,
            show_details: true,
        }
    }
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutlineSortOrder {
    Position,
    Name,
    Kind,
}

/// Outline event
#[derive(Debug, Clone)]
pub enum OutlineEvent {
    Updated { file: PathBuf },
    Invalidated { file: PathBuf },
}

/// Outline view model
pub struct OutlineViewModel {
    service: Arc<OutlineService>,
    /// Current file
    file: RwLock<Option<PathBuf>>,
    /// Current tree
    tree: RwLock<Option<OutlineTree>>,
    /// Expanded nodes
    expanded: RwLock<HashMap<String, bool>>,
    /// Selected node path
    selected: RwLock<Option<Vec<usize>>>,
    /// Filter text
    filter: RwLock<String>,
}

impl OutlineViewModel {
    pub fn new(service: Arc<OutlineService>) -> Self {
        Self {
            service,
            file: RwLock::new(None),
            tree: RwLock::new(None),
            expanded: RwLock::new(HashMap::new()),
            selected: RwLock::new(None),
            filter: RwLock::new(String::new()),
        }
    }

    pub async fn load(&self, file: PathBuf) -> anyhow::Result<()> {
        *self.file.write() = Some(file.clone());
        let tree = self.service.get_outline(&file).await;
        *self.tree.write() = tree;
        Ok(())
    }

    pub fn tree(&self) -> Option<OutlineTree> {
        self.tree.read().clone()
    }

    pub fn flat_items(&self) -> Vec<FlatOutlineNode> {
        self.tree.read().as_ref().map(|t| t.flatten()).unwrap_or_default()
    }

    pub fn is_expanded(&self, name: &str) -> bool {
        *self.expanded.read().get(name).unwrap_or(&true)
    }

    pub fn toggle_expanded(&self, name: &str) {
        let mut expanded = self.expanded.write();
        let current = *expanded.get(name).unwrap_or(&true);
        expanded.insert(name.to_string(), !current);
    }

    pub fn expand_all(&self) {
        self.expanded.write().clear();
    }

    pub fn collapse_all(&self) {
        if let Some(tree) = self.tree.read().as_ref() {
            let mut expanded = self.expanded.write();
            for node in tree.flatten() {
                expanded.insert(node.node.name.clone(), false);
            }
        }
    }

    pub fn set_filter(&self, filter: String) {
        *self.filter.write() = filter;
    }

    pub fn reveal(&self, line: u32, column: u32) {
        if let Some(tree) = self.tree.read().as_ref() {
            if let Some(node) = tree.find_at_position(line, column) {
                // Expand parents and select node
                self.expanded.write().insert(node.name.clone(), true);
            }
        }
    }

    pub fn refresh(&self) {
        if let Some(file) = self.file.read().clone() {
            self.service.invalidate(&file);
        }
    }
}
