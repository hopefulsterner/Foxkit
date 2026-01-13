//! Accessibility and Screen Reader Support for Foxkit
//!
//! ARIA-compliant accessibility tree, screen reader announcements,
//! keyboard navigation, and high contrast support.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique accessible node ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccessibleId(pub u64);

impl AccessibleId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for AccessibleId {
    fn default() -> Self {
        Self::new()
    }
}

/// ARIA role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    // Document structure
    Document,
    Article,
    Section,
    Navigation,
    Main,
    Complementary,
    Banner,
    ContentInfo,
    // Widgets
    Button,
    Checkbox,
    Radio,
    Textbox,
    Searchbox,
    Listbox,
    Option,
    Combobox,
    Menu,
    MenuBar,
    MenuItem,
    MenuItemCheckbox,
    MenuItemRadio,
    Slider,
    SpinButton,
    Switch,
    Tab,
    TabList,
    TabPanel,
    // Live regions
    Alert,
    Status,
    Log,
    Marquee,
    Timer,
    // Grid
    Grid,
    Row,
    Cell,
    RowHeader,
    ColumnHeader,
    GridCell,
    // Tree
    Tree,
    TreeItem,
    TreeGrid,
    // Other
    Dialog,
    AlertDialog,
    Tooltip,
    Progressbar,
    Separator,
    Group,
    Toolbar,
    Link,
    List,
    ListItem,
    Image,
    Figure,
    Code,
    // Custom
    Editor,
    EditorLine,
    Gutter,
    Minimap,
    Terminal,
}

/// Live region politeness
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LivePoliteness {
    #[default]
    Off,
    Polite,
    Assertive,
}

/// Accessible node state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccessibleState {
    pub checked: Option<bool>,
    pub selected: bool,
    pub expanded: Option<bool>,
    pub disabled: bool,
    pub hidden: bool,
    pub busy: bool,
    pub pressed: Option<bool>,
    pub invalid: bool,
    pub readonly: bool,
    pub required: bool,
    pub multiselectable: bool,
    pub haspopup: Option<PopupType>,
    pub level: Option<u32>,
    pub posinset: Option<u32>,
    pub setsize: Option<u32>,
    pub valuemin: Option<f64>,
    pub valuemax: Option<f64>,
    pub valuenow: Option<f64>,
    pub valuetext: Option<String>,
}

/// Popup type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PopupType {
    Menu,
    Listbox,
    Tree,
    Grid,
    Dialog,
}

/// Accessible node in the accessibility tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibleNode {
    pub id: AccessibleId,
    pub role: Role,
    pub name: Option<String>,
    pub description: Option<String>,
    pub value: Option<String>,
    pub state: AccessibleState,
    pub live: LivePoliteness,
    pub live_atomic: bool,
    pub live_relevant: Vec<LiveRelevant>,
    pub actions: Vec<AccessibleAction>,
    pub relations: HashMap<Relation, Vec<AccessibleId>>,
    pub bounds: Option<AccessibleBounds>,
    pub children: Vec<AccessibleId>,
    pub parent: Option<AccessibleId>,
}

/// Live region change types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LiveRelevant {
    Additions,
    Removals,
    Text,
    All,
}

/// Relationship types between nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Relation {
    LabelledBy,
    DescribedBy,
    Controls,
    FlowsTo,
    ActiveDescendant,
    ErrorMessage,
    Details,
    Owns,
}

/// Accessible action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccessibleAction {
    Click,
    Focus,
    Check,
    Uncheck,
    Select,
    Expand,
    Collapse,
    Activate,
    Dismiss,
    ScrollIntoView,
    SetValue,
    Increment,
    Decrement,
}

/// Accessible bounds
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AccessibleBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl AccessibleNode {
    pub fn new(id: AccessibleId, role: Role) -> Self {
        Self {
            id,
            role,
            name: None,
            description: None,
            value: None,
            state: AccessibleState::default(),
            live: LivePoliteness::Off,
            live_atomic: false,
            live_relevant: Vec::new(),
            actions: Vec::new(),
            relations: HashMap::new(),
            bounds: None,
            children: Vec::new(),
            parent: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn add_action(&mut self, action: AccessibleAction) {
        if !self.actions.contains(&action) {
            self.actions.push(action);
        }
    }

    pub fn add_relation(&mut self, relation: Relation, target: AccessibleId) {
        self.relations.entry(relation).or_default().push(target);
    }
}

/// Announcement for screen readers
#[derive(Debug, Clone)]
pub struct Announcement {
    pub message: String,
    pub politeness: LivePoliteness,
    pub clear_queue: bool,
}

impl Announcement {
    pub fn polite(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            politeness: LivePoliteness::Polite,
            clear_queue: false,
        }
    }

    pub fn assertive(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            politeness: LivePoliteness::Assertive,
            clear_queue: true,
        }
    }
}

/// Focus management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    Next,
    Previous,
    First,
    Last,
    Parent,
    FirstChild,
    Specific(AccessibleId),
}

/// Accessibility configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityConfig {
    pub screen_reader_enabled: bool,
    pub reduced_motion: bool,
    pub high_contrast: bool,
    pub large_text: bool,
    pub keyboard_navigation: bool,
    pub focus_visible: bool,
    pub announcement_delay_ms: u64,
    pub min_target_size: f64,
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            screen_reader_enabled: false,
            reduced_motion: false,
            high_contrast: false,
            large_text: false,
            keyboard_navigation: true,
            focus_visible: true,
            announcement_delay_ms: 150,
            min_target_size: 44.0,
        }
    }
}

/// Accessibility service
pub struct AccessibilityService {
    nodes: RwLock<HashMap<AccessibleId, AccessibleNode>>,
    root: RwLock<Option<AccessibleId>>,
    focus: RwLock<Option<AccessibleId>>,
    announcements: RwLock<Vec<Announcement>>,
    config: RwLock<AccessibilityConfig>,
}

impl AccessibilityService {
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            root: RwLock::new(None),
            focus: RwLock::new(None),
            announcements: RwLock::new(Vec::new()),
            config: RwLock::new(AccessibilityConfig::default()),
        }
    }

    pub fn set_root(&self, node: AccessibleNode) {
        let id = node.id;
        self.nodes.write().insert(id, node);
        *self.root.write() = Some(id);
    }

    pub fn add_node(&self, node: AccessibleNode) {
        self.nodes.write().insert(node.id, node);
    }

    pub fn remove_node(&self, id: AccessibleId) {
        self.nodes.write().remove(&id);
    }

    pub fn update_node(&self, id: AccessibleId, update: impl FnOnce(&mut AccessibleNode)) {
        if let Some(node) = self.nodes.write().get_mut(&id) {
            update(node);
        }
    }

    pub fn get_node(&self, id: AccessibleId) -> Option<AccessibleNode> {
        self.nodes.read().get(&id).cloned()
    }

    pub fn announce(&self, announcement: Announcement) {
        let mut queue = self.announcements.write();
        if announcement.clear_queue {
            queue.clear();
        }
        queue.push(announcement);
    }

    pub fn drain_announcements(&self) -> Vec<Announcement> {
        std::mem::take(&mut *self.announcements.write())
    }

    pub fn set_focus(&self, id: AccessibleId) {
        *self.focus.write() = Some(id);
    }

    pub fn get_focus(&self) -> Option<AccessibleId> {
        *self.focus.read()
    }

    pub fn move_focus(&self, target: FocusTarget) -> Option<AccessibleId> {
        let nodes = self.nodes.read();
        let current = *self.focus.read();

        let new_focus = match target {
            FocusTarget::Specific(id) => Some(id),
            FocusTarget::Next | FocusTarget::Previous => {
                // Simplified - would need full tree traversal
                current
            }
            FocusTarget::First => self.root.read().and_then(|root| {
                nodes.get(&root).and_then(|n| n.children.first().copied())
            }),
            FocusTarget::Last => self.root.read().and_then(|root| {
                nodes.get(&root).and_then(|n| n.children.last().copied())
            }),
            FocusTarget::Parent => {
                current.and_then(|c| nodes.get(&c).and_then(|n| n.parent))
            }
            FocusTarget::FirstChild => {
                current.and_then(|c| nodes.get(&c).and_then(|n| n.children.first().copied()))
            }
        };

        if new_focus.is_some() {
            *self.focus.write() = new_focus;
        }
        new_focus
    }

    pub fn config(&self) -> AccessibilityConfig {
        self.config.read().clone()
    }

    pub fn update_config(&self, update: impl FnOnce(&mut AccessibilityConfig)) {
        update(&mut self.config.write());
    }

    pub fn is_screen_reader_enabled(&self) -> bool {
        self.config.read().screen_reader_enabled
    }

    pub fn prefers_reduced_motion(&self) -> bool {
        self.config.read().reduced_motion
    }

    pub fn is_high_contrast(&self) -> bool {
        self.config.read().high_contrast
    }
}

impl Default for AccessibilityService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accessible_node() {
        let node = AccessibleNode::new(AccessibleId::new(), Role::Button)
            .with_name("Submit")
            .with_description("Submit the form");
        
        assert_eq!(node.name, Some("Submit".to_string()));
        assert_eq!(node.role, Role::Button);
    }

    #[test]
    fn test_announcements() {
        let service = AccessibilityService::new();
        service.announce(Announcement::polite("File saved"));
        service.announce(Announcement::assertive("Error occurred"));
        
        let announcements = service.drain_announcements();
        assert_eq!(announcements.len(), 1); // assertive clears queue
    }

    #[test]
    fn test_focus_management() {
        let service = AccessibilityService::new();
        let id = AccessibleId::new();
        service.add_node(AccessibleNode::new(id, Role::Button));
        service.set_focus(id);
        assert_eq!(service.get_focus(), Some(id));
    }
}
