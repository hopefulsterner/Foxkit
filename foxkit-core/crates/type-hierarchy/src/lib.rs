//! # Foxkit Type Hierarchy
//!
//! Supertypes and subtypes visualization.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Type hierarchy service
pub struct TypeHierarchyService {
    /// Cache
    cache: RwLock<HashMap<TypeHierarchyId, TypeHierarchyItem>>,
    /// Events
    events: broadcast::Sender<TypeHierarchyEvent>,
}

impl TypeHierarchyService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            cache: RwLock::new(HashMap::new()),
            events,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<TypeHierarchyEvent> {
        self.events.subscribe()
    }

    /// Prepare type hierarchy
    pub async fn prepare(
        &self,
        file: &PathBuf,
        line: u32,
        column: u32,
    ) -> Option<Vec<TypeHierarchyItem>> {
        // Would request from LSP
        None
    }

    /// Get supertypes
    pub async fn get_supertypes(
        &self,
        item: &TypeHierarchyItem,
    ) -> anyhow::Result<Vec<TypeHierarchyItem>> {
        // Would request from LSP
        Ok(Vec::new())
    }

    /// Get subtypes
    pub async fn get_subtypes(
        &self,
        item: &TypeHierarchyItem,
    ) -> anyhow::Result<Vec<TypeHierarchyItem>> {
        // Would request from LSP
        Ok(Vec::new())
    }
}

impl Default for TypeHierarchyService {
    fn default() -> Self {
        Self::new()
    }
}

/// Type hierarchy item ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeHierarchyId(pub String);

/// Type hierarchy item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeHierarchyItem {
    /// ID
    pub id: TypeHierarchyId,
    /// Name
    pub name: String,
    /// Kind
    pub kind: TypeKind,
    /// Tags
    pub tags: Vec<TypeTag>,
    /// Detail
    pub detail: Option<String>,
    /// URI
    pub uri: PathBuf,
    /// Range
    pub range: Range,
    /// Selection range
    pub selection_range: Range,
    /// Data (for provider)
    #[serde(skip)]
    pub data: Option<serde_json::Value>,
}

impl TypeHierarchyItem {
    pub fn new(
        name: impl Into<String>,
        kind: TypeKind,
        uri: PathBuf,
        range: Range,
    ) -> Self {
        let name = name.into();
        let id = TypeHierarchyId(format!("{}:{}:{}", uri.display(), range.start.line, name));

        Self {
            id,
            name,
            kind,
            tags: Vec::new(),
            detail: None,
            uri,
            range: range.clone(),
            selection_range: range,
            data: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn icon(&self) -> &'static str {
        self.kind.icon()
    }

    pub fn is_deprecated(&self) -> bool {
        self.tags.contains(&TypeTag::Deprecated)
    }
}

/// Type kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeKind {
    Class,
    Interface,
    Struct,
    Enum,
    TypeParameter,
    TypeAlias,
    Module,
    Namespace,
    Protocol,
    Trait,
}

impl TypeKind {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Class => "$(symbol-class)",
            Self::Interface => "$(symbol-interface)",
            Self::Struct => "$(symbol-struct)",
            Self::Enum => "$(symbol-enum)",
            Self::TypeParameter => "$(symbol-type-parameter)",
            Self::TypeAlias => "$(symbol-type-parameter)",
            Self::Module => "$(package)",
            Self::Namespace => "$(symbol-namespace)",
            Self::Protocol => "$(symbol-interface)",
            Self::Trait => "$(symbol-interface)",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::TypeParameter => "type parameter",
            Self::TypeAlias => "type alias",
            Self::Module => "module",
            Self::Namespace => "namespace",
            Self::Protocol => "protocol",
            Self::Trait => "trait",
        }
    }
}

/// Type tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeTag {
    Deprecated,
    Abstract,
    Final,
    Static,
}

/// Range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}

/// Position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// Type hierarchy event
#[derive(Debug, Clone)]
pub enum TypeHierarchyEvent {
    Prepared { items: Vec<TypeHierarchyItem> },
    SupertypesLoaded { item: TypeHierarchyId, supertypes: Vec<TypeHierarchyItem> },
    SubtypesLoaded { item: TypeHierarchyId, subtypes: Vec<TypeHierarchyItem> },
}

/// Type hierarchy direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeHierarchyDirection {
    Supertypes,
    Subtypes,
}

/// Type hierarchy view model
pub struct TypeHierarchyViewModel {
    service: Arc<TypeHierarchyService>,
    /// Current direction
    direction: RwLock<TypeHierarchyDirection>,
    /// Root item
    root: RwLock<Option<TypeHierarchyItem>>,
    /// Expanded items
    expanded: RwLock<HashMap<TypeHierarchyId, bool>>,
    /// Loaded children
    children: RwLock<HashMap<TypeHierarchyId, Vec<TypeHierarchyItem>>>,
    /// Selected item
    selected: RwLock<Option<TypeHierarchyId>>,
}

impl TypeHierarchyViewModel {
    pub fn new(service: Arc<TypeHierarchyService>) -> Self {
        Self {
            service,
            direction: RwLock::new(TypeHierarchyDirection::Supertypes),
            root: RwLock::new(None),
            expanded: RwLock::new(HashMap::new()),
            children: RwLock::new(HashMap::new()),
            selected: RwLock::new(None),
        }
    }

    pub async fn prepare(&self, file: &PathBuf, line: u32, column: u32) -> anyhow::Result<()> {
        if let Some(items) = self.service.prepare(file, line, column).await {
            if let Some(first) = items.into_iter().next() {
                *self.root.write() = Some(first);
            }
        }
        Ok(())
    }

    pub fn root(&self) -> Option<TypeHierarchyItem> {
        self.root.read().clone()
    }

    pub fn direction(&self) -> TypeHierarchyDirection {
        *self.direction.read()
    }

    pub fn set_direction(&self, direction: TypeHierarchyDirection) {
        *self.direction.write() = direction;
        self.children.write().clear();
        self.expanded.write().clear();
    }

    pub fn is_expanded(&self, id: &TypeHierarchyId) -> bool {
        *self.expanded.read().get(id).unwrap_or(&false)
    }

    pub async fn toggle_expanded(&self, item: &TypeHierarchyItem) {
        let id = &item.id;
        let is_expanded = self.is_expanded(id);

        if is_expanded {
            self.expanded.write().insert(id.clone(), false);
        } else {
            if !self.children.read().contains_key(id) {
                self.load_children(item).await;
            }
            self.expanded.write().insert(id.clone(), true);
        }
    }

    async fn load_children(&self, item: &TypeHierarchyItem) {
        let direction = *self.direction.read();
        let items = match direction {
            TypeHierarchyDirection::Supertypes => {
                self.service.get_supertypes(item).await.unwrap_or_default()
            }
            TypeHierarchyDirection::Subtypes => {
                self.service.get_subtypes(item).await.unwrap_or_default()
            }
        };

        self.children.write().insert(item.id.clone(), items);
    }

    pub fn children(&self, id: &TypeHierarchyId) -> Vec<TypeHierarchyItem> {
        self.children.read().get(id).cloned().unwrap_or_default()
    }

    pub fn select(&self, id: TypeHierarchyId) {
        *self.selected.write() = Some(id);
    }

    pub fn selected(&self) -> Option<TypeHierarchyId> {
        self.selected.read().clone()
    }

    pub fn clear(&self) {
        *self.root.write() = None;
        self.expanded.write().clear();
        self.children.write().clear();
        *self.selected.write() = None;
    }
}

/// Build full hierarchy tree
pub fn build_hierarchy_tree(
    root: &TypeHierarchyItem,
    supertypes: &HashMap<TypeHierarchyId, Vec<TypeHierarchyItem>>,
    subtypes: &HashMap<TypeHierarchyId, Vec<TypeHierarchyItem>>,
) -> HierarchyTree {
    let supertype_chain = build_chain(&root.id, supertypes);
    let subtype_tree = build_subtree(&root.id, subtypes);

    HierarchyTree {
        root: root.clone(),
        supertypes: supertype_chain,
        subtypes: subtype_tree,
    }
}

fn build_chain(
    id: &TypeHierarchyId,
    map: &HashMap<TypeHierarchyId, Vec<TypeHierarchyItem>>,
) -> Vec<TypeHierarchyItem> {
    let mut chain = Vec::new();
    let mut current = id.clone();

    while let Some(items) = map.get(&current) {
        if let Some(first) = items.first() {
            chain.push(first.clone());
            current = first.id.clone();
        } else {
            break;
        }
    }

    chain
}

fn build_subtree(
    id: &TypeHierarchyId,
    map: &HashMap<TypeHierarchyId, Vec<TypeHierarchyItem>>,
) -> Vec<HierarchyNode> {
    map.get(id)
        .map(|items| {
            items.iter()
                .map(|item| HierarchyNode {
                    item: item.clone(),
                    children: build_subtree(&item.id, map),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Hierarchy tree
#[derive(Debug, Clone)]
pub struct HierarchyTree {
    pub root: TypeHierarchyItem,
    pub supertypes: Vec<TypeHierarchyItem>,
    pub subtypes: Vec<HierarchyNode>,
}

/// Hierarchy node
#[derive(Debug, Clone)]
pub struct HierarchyNode {
    pub item: TypeHierarchyItem,
    pub children: Vec<HierarchyNode>,
}
