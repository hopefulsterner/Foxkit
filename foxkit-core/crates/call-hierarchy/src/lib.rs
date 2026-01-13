//! # Foxkit Call Hierarchy
//!
//! Incoming and outgoing call analysis.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Call hierarchy service
pub struct CallHierarchyService {
    /// Cache
    cache: RwLock<HashMap<CallHierarchyId, CallHierarchyItem>>,
    /// Events
    events: broadcast::Sender<CallHierarchyEvent>,
}

impl CallHierarchyService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            cache: RwLock::new(HashMap::new()),
            events,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<CallHierarchyEvent> {
        self.events.subscribe()
    }

    /// Prepare call hierarchy
    pub async fn prepare(
        &self,
        file: &PathBuf,
        line: u32,
        column: u32,
    ) -> Option<Vec<CallHierarchyItem>> {
        // Would request from LSP
        None
    }

    /// Get incoming calls
    pub async fn get_incoming_calls(
        &self,
        item: &CallHierarchyItem,
    ) -> anyhow::Result<Vec<IncomingCall>> {
        // Would request from LSP
        Ok(Vec::new())
    }

    /// Get outgoing calls
    pub async fn get_outgoing_calls(
        &self,
        item: &CallHierarchyItem,
    ) -> anyhow::Result<Vec<OutgoingCall>> {
        // Would request from LSP
        Ok(Vec::new())
    }
}

impl Default for CallHierarchyService {
    fn default() -> Self {
        Self::new()
    }
}

/// Call hierarchy item ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallHierarchyId(pub String);

/// Call hierarchy item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyItem {
    /// ID (for caching/reference)
    pub id: CallHierarchyId,
    /// Name
    pub name: String,
    /// Kind
    pub kind: SymbolKind,
    /// Tags
    pub tags: Vec<SymbolTag>,
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

impl CallHierarchyItem {
    pub fn new(
        name: impl Into<String>,
        kind: SymbolKind,
        uri: PathBuf,
        range: Range,
    ) -> Self {
        let name = name.into();
        let id = CallHierarchyId(format!("{}:{}:{}", uri.display(), range.start.line, name));

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
        self.tags.contains(&SymbolTag::Deprecated)
    }
}

/// Incoming call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingCall {
    /// The item that makes the call
    pub from: CallHierarchyItem,
    /// Call ranges
    pub from_ranges: Vec<Range>,
}

impl IncomingCall {
    pub fn new(from: CallHierarchyItem, from_ranges: Vec<Range>) -> Self {
        Self { from, from_ranges }
    }

    pub fn call_count(&self) -> usize {
        self.from_ranges.len()
    }
}

/// Outgoing call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingCall {
    /// The item being called
    pub to: CallHierarchyItem,
    /// Call ranges
    pub from_ranges: Vec<Range>,
}

impl OutgoingCall {
    pub fn new(to: CallHierarchyItem, from_ranges: Vec<Range>) -> Self {
        Self { to, from_ranges }
    }

    pub fn call_count(&self) -> usize {
        self.from_ranges.len()
    }
}

/// Symbol kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// Symbol tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolTag {
    Deprecated,
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

    pub fn point(line: u32, character: u32) -> Self {
        let pos = Position::new(line, character);
        Self { start: pos.clone(), end: pos }
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

/// Call hierarchy event
#[derive(Debug, Clone)]
pub enum CallHierarchyEvent {
    Prepared { items: Vec<CallHierarchyItem> },
    IncomingLoaded { item: CallHierarchyId, calls: Vec<IncomingCall> },
    OutgoingLoaded { item: CallHierarchyId, calls: Vec<OutgoingCall> },
}

/// Call hierarchy direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallHierarchyDirection {
    Incoming,
    Outgoing,
}

/// Call hierarchy view model
pub struct CallHierarchyViewModel {
    service: Arc<CallHierarchyService>,
    /// Current direction
    direction: RwLock<CallHierarchyDirection>,
    /// Root item
    root: RwLock<Option<CallHierarchyItem>>,
    /// Expanded items
    expanded: RwLock<HashMap<CallHierarchyId, bool>>,
    /// Loaded children
    children: RwLock<HashMap<CallHierarchyId, Vec<CallTreeNode>>>,
    /// Selected item
    selected: RwLock<Option<CallHierarchyId>>,
}

impl CallHierarchyViewModel {
    pub fn new(service: Arc<CallHierarchyService>) -> Self {
        Self {
            service,
            direction: RwLock::new(CallHierarchyDirection::Incoming),
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

    pub fn root(&self) -> Option<CallHierarchyItem> {
        self.root.read().clone()
    }

    pub fn direction(&self) -> CallHierarchyDirection {
        *self.direction.read()
    }

    pub fn set_direction(&self, direction: CallHierarchyDirection) {
        *self.direction.write() = direction;
        // Clear cached children when direction changes
        self.children.write().clear();
        self.expanded.write().clear();
    }

    pub fn is_expanded(&self, id: &CallHierarchyId) -> bool {
        *self.expanded.read().get(id).unwrap_or(&false)
    }

    pub async fn toggle_expanded(&self, item: &CallHierarchyItem) {
        let id = &item.id;
        let is_expanded = self.is_expanded(id);

        if is_expanded {
            self.expanded.write().insert(id.clone(), false);
        } else {
            // Load children if needed
            if !self.children.read().contains_key(id) {
                self.load_children(item).await;
            }
            self.expanded.write().insert(id.clone(), true);
        }
    }

    async fn load_children(&self, item: &CallHierarchyItem) {
        let direction = *self.direction.read();
        let nodes = match direction {
            CallHierarchyDirection::Incoming => {
                match self.service.get_incoming_calls(item).await {
                    Ok(calls) => calls.into_iter()
                        .map(|c| CallTreeNode::Incoming(c))
                        .collect(),
                    Err(_) => Vec::new(),
                }
            }
            CallHierarchyDirection::Outgoing => {
                match self.service.get_outgoing_calls(item).await {
                    Ok(calls) => calls.into_iter()
                        .map(|c| CallTreeNode::Outgoing(c))
                        .collect(),
                    Err(_) => Vec::new(),
                }
            }
        };

        self.children.write().insert(item.id.clone(), nodes);
    }

    pub fn children(&self, id: &CallHierarchyId) -> Vec<CallTreeNode> {
        self.children.read().get(id).cloned().unwrap_or_default()
    }

    pub fn select(&self, id: CallHierarchyId) {
        *self.selected.write() = Some(id);
    }

    pub fn selected(&self) -> Option<CallHierarchyId> {
        self.selected.read().clone()
    }

    pub fn clear(&self) {
        *self.root.write() = None;
        self.expanded.write().clear();
        self.children.write().clear();
        *self.selected.write() = None;
    }
}

/// Call tree node
#[derive(Debug, Clone)]
pub enum CallTreeNode {
    Incoming(IncomingCall),
    Outgoing(OutgoingCall),
}

impl CallTreeNode {
    pub fn item(&self) -> &CallHierarchyItem {
        match self {
            Self::Incoming(call) => &call.from,
            Self::Outgoing(call) => &call.to,
        }
    }

    pub fn call_count(&self) -> usize {
        match self {
            Self::Incoming(call) => call.call_count(),
            Self::Outgoing(call) => call.call_count(),
        }
    }

    pub fn ranges(&self) -> &[Range] {
        match self {
            Self::Incoming(call) => &call.from_ranges,
            Self::Outgoing(call) => &call.from_ranges,
        }
    }
}
