//! # Foxkit Editor Tabs
//!
//! Tab management and tab bar functionality.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

static TAB_ID: AtomicU64 = AtomicU64::new(1);
static GROUP_ID: AtomicU64 = AtomicU64::new(1);

/// Editor tabs service
pub struct EditorTabsService {
    /// Tab groups
    groups: RwLock<HashMap<TabGroupId, TabGroup>>,
    /// Active group
    active_group: RwLock<Option<TabGroupId>>,
    /// Configuration
    config: RwLock<TabsConfig>,
    /// Event sender
    event_tx: broadcast::Sender<TabEvent>,
}

impl EditorTabsService {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(256);
        
        // Create default group
        let mut groups = HashMap::new();
        let default_group = TabGroup::new();
        let default_id = default_group.id.clone();
        groups.insert(default_id.clone(), default_group);

        Self {
            groups: RwLock::new(groups),
            active_group: RwLock::new(Some(default_id)),
            config: RwLock::new(TabsConfig::default()),
            event_tx,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<TabEvent> {
        self.event_tx.subscribe()
    }

    /// Open file in tab
    pub fn open(&self, file: PathBuf, preview: bool) -> TabId {
        let mut groups = self.groups.write();
        let active_id = self.active_group.read().clone();

        let group = active_id
            .and_then(|id| groups.get_mut(&id))
            .or_else(|| groups.values_mut().next())
            .expect("No tab group available");

        // Check if already open
        if let Some(existing) = group.tabs.iter().find(|t| t.file == Some(file.clone())) {
            let id = existing.id.clone();
            group.active = Some(id.clone());
            let _ = self.event_tx.send(TabEvent::Activated(id.clone()));
            return id;
        }

        // Create new tab
        let tab = Tab::new_file(file.clone(), preview);
        let id = tab.id.clone();

        // Handle preview mode
        if preview {
            // Replace existing preview tab
            if let Some(pos) = group.tabs.iter().position(|t| t.preview) {
                group.tabs.remove(pos);
            }
        }

        // Insert tab
        let insert_pos = match self.config.read().open_position {
            TabOpenPosition::End => group.tabs.len(),
            TabOpenPosition::AfterActive => {
                group.active
                    .as_ref()
                    .and_then(|id| group.tabs.iter().position(|t| &t.id == id))
                    .map(|p| p + 1)
                    .unwrap_or(group.tabs.len())
            }
            TabOpenPosition::First => 0,
        };

        group.tabs.insert(insert_pos, tab);
        group.active = Some(id.clone());

        let _ = self.event_tx.send(TabEvent::Opened(id.clone()));
        id
    }

    /// Open untitled tab
    pub fn open_untitled(&self) -> TabId {
        let mut groups = self.groups.write();
        let active_id = self.active_group.read().clone();

        let group = active_id
            .and_then(|id| groups.get_mut(&id))
            .or_else(|| groups.values_mut().next())
            .expect("No tab group available");

        let tab = Tab::new_untitled();
        let id = tab.id.clone();

        group.tabs.push(tab);
        group.active = Some(id.clone());

        let _ = self.event_tx.send(TabEvent::Opened(id.clone()));
        id
    }

    /// Close tab
    pub fn close(&self, id: &TabId) -> Option<Tab> {
        let mut groups = self.groups.write();

        for group in groups.values_mut() {
            if let Some(pos) = group.tabs.iter().position(|t| &t.id == id) {
                let tab = group.tabs.remove(pos);

                // Update active tab
                if group.active.as_ref() == Some(id) {
                    group.active = if pos > 0 {
                        group.tabs.get(pos - 1).map(|t| t.id.clone())
                    } else {
                        group.tabs.first().map(|t| t.id.clone())
                    };
                }

                let _ = self.event_tx.send(TabEvent::Closed(id.clone()));
                return Some(tab);
            }
        }

        None
    }

    /// Close all tabs in group
    pub fn close_all(&self, group_id: &TabGroupId) {
        let mut groups = self.groups.write();
        
        if let Some(group) = groups.get_mut(group_id) {
            let ids: Vec<_> = group.tabs.iter().map(|t| t.id.clone()).collect();
            group.tabs.clear();
            group.active = None;

            for id in ids {
                let _ = self.event_tx.send(TabEvent::Closed(id));
            }
        }
    }

    /// Close other tabs
    pub fn close_others(&self, keep_id: &TabId) {
        let mut groups = self.groups.write();

        for group in groups.values_mut() {
            let to_close: Vec<_> = group.tabs
                .iter()
                .filter(|t| &t.id != keep_id)
                .map(|t| t.id.clone())
                .collect();

            group.tabs.retain(|t| &t.id == keep_id);

            for id in to_close {
                let _ = self.event_tx.send(TabEvent::Closed(id));
            }
        }
    }

    /// Close tabs to the right
    pub fn close_to_right(&self, id: &TabId) {
        let mut groups = self.groups.write();

        for group in groups.values_mut() {
            if let Some(pos) = group.tabs.iter().position(|t| &t.id == id) {
                let to_close: Vec<_> = group.tabs
                    .iter()
                    .skip(pos + 1)
                    .map(|t| t.id.clone())
                    .collect();

                group.tabs.truncate(pos + 1);

                for id in to_close {
                    let _ = self.event_tx.send(TabEvent::Closed(id));
                }
            }
        }
    }

    /// Activate tab
    pub fn activate(&self, id: &TabId) {
        let mut groups = self.groups.write();

        for (group_id, group) in groups.iter_mut() {
            if group.tabs.iter().any(|t| &t.id == id) {
                group.active = Some(id.clone());
                *self.active_group.write() = Some(group_id.clone());
                let _ = self.event_tx.send(TabEvent::Activated(id.clone()));
                return;
            }
        }
    }

    /// Pin tab
    pub fn pin(&self, id: &TabId) {
        let mut groups = self.groups.write();

        for group in groups.values_mut() {
            if let Some(tab) = group.tabs.iter_mut().find(|t| &t.id == id) {
                tab.pinned = true;
                tab.preview = false;

                // Move to front with other pinned tabs
                let pos = group.tabs.iter().position(|t| &t.id == id).unwrap();
                let tab = group.tabs.remove(pos);
                let insert_pos = group.tabs.iter().take_while(|t| t.pinned).count();
                group.tabs.insert(insert_pos, tab);

                let _ = self.event_tx.send(TabEvent::Pinned(id.clone()));
                return;
            }
        }
    }

    /// Unpin tab
    pub fn unpin(&self, id: &TabId) {
        let mut groups = self.groups.write();

        for group in groups.values_mut() {
            if let Some(tab) = group.tabs.iter_mut().find(|t| &t.id == id) {
                tab.pinned = false;
                let _ = self.event_tx.send(TabEvent::Unpinned(id.clone()));
                return;
            }
        }
    }

    /// Mark tab as dirty
    pub fn set_dirty(&self, id: &TabId, dirty: bool) {
        let mut groups = self.groups.write();

        for group in groups.values_mut() {
            if let Some(tab) = group.tabs.iter_mut().find(|t| &t.id == id) {
                if tab.dirty != dirty {
                    tab.dirty = dirty;
                    let _ = self.event_tx.send(TabEvent::DirtyChanged(id.clone(), dirty));
                }
                return;
            }
        }
    }

    /// Promote preview to permanent
    pub fn promote_preview(&self, id: &TabId) {
        let mut groups = self.groups.write();

        for group in groups.values_mut() {
            if let Some(tab) = group.tabs.iter_mut().find(|t| &t.id == id) {
                if tab.preview {
                    tab.preview = false;
                    let _ = self.event_tx.send(TabEvent::PreviewPromoted(id.clone()));
                }
                return;
            }
        }
    }

    /// Move tab
    pub fn move_tab(&self, id: &TabId, to_group: &TabGroupId, index: usize) {
        let mut groups = self.groups.write();

        // Find and remove tab
        let mut tab = None;
        for group in groups.values_mut() {
            if let Some(pos) = group.tabs.iter().position(|t| &t.id == id) {
                tab = Some(group.tabs.remove(pos));
                break;
            }
        }

        // Insert in new position
        if let Some(tab) = tab {
            if let Some(group) = groups.get_mut(to_group) {
                let insert_pos = index.min(group.tabs.len());
                group.tabs.insert(insert_pos, tab);
                let _ = self.event_tx.send(TabEvent::Moved(id.clone()));
            }
        }
    }

    /// Get active tab
    pub fn active_tab(&self) -> Option<Tab> {
        let groups = self.groups.read();
        let active_group = self.active_group.read();

        active_group
            .as_ref()
            .and_then(|gid| groups.get(gid))
            .and_then(|g| g.active.as_ref())
            .and_then(|tid| groups.values().flat_map(|g| &g.tabs).find(|t| &t.id == tid))
            .cloned()
    }

    /// Get tab by ID
    pub fn get_tab(&self, id: &TabId) -> Option<Tab> {
        self.groups
            .read()
            .values()
            .flat_map(|g| &g.tabs)
            .find(|t| &t.id == id)
            .cloned()
    }

    /// Get all tabs
    pub fn all_tabs(&self) -> Vec<Tab> {
        self.groups
            .read()
            .values()
            .flat_map(|g| g.tabs.clone())
            .collect()
    }

    /// Get dirty tabs
    pub fn dirty_tabs(&self) -> Vec<Tab> {
        self.groups
            .read()
            .values()
            .flat_map(|g| &g.tabs)
            .filter(|t| t.dirty)
            .cloned()
            .collect()
    }

    /// Create new tab group
    pub fn create_group(&self) -> TabGroupId {
        let group = TabGroup::new();
        let id = group.id.clone();
        self.groups.write().insert(id.clone(), group);
        id
    }

    /// Remove tab group
    pub fn remove_group(&self, id: &TabGroupId) {
        let mut groups = self.groups.write();
        groups.remove(id);

        // Ensure at least one group exists
        if groups.is_empty() {
            let group = TabGroup::new();
            let new_id = group.id.clone();
            groups.insert(new_id.clone(), group);
            *self.active_group.write() = Some(new_id);
        }
    }

    /// Get tab groups
    pub fn groups(&self) -> Vec<TabGroup> {
        self.groups.read().values().cloned().collect()
    }
}

impl Default for EditorTabsService {
    fn default() -> Self {
        Self::new()
    }
}

/// Tab ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(u64);

impl TabId {
    fn new() -> Self {
        Self(TAB_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Tab group ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabGroupId(u64);

impl TabGroupId {
    fn new() -> Self {
        Self(GROUP_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Tab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    /// Unique ID
    pub id: TabId,
    /// File path
    pub file: Option<PathBuf>,
    /// Display label
    pub label: String,
    /// Is dirty
    pub dirty: bool,
    /// Is preview
    pub preview: bool,
    /// Is pinned
    pub pinned: bool,
    /// Icon
    pub icon: Option<String>,
    /// Description
    pub description: Option<String>,
}

impl Tab {
    pub fn new_file(file: PathBuf, preview: bool) -> Self {
        let label = file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        Self {
            id: TabId::new(),
            file: Some(file),
            label,
            dirty: false,
            preview,
            pinned: false,
            icon: None,
            description: None,
        }
    }

    pub fn new_untitled() -> Self {
        Self {
            id: TabId::new(),
            file: None,
            label: "Untitled".to_string(),
            dirty: false,
            preview: false,
            pinned: false,
            icon: None,
            description: None,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// Tab group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabGroup {
    /// Group ID
    pub id: TabGroupId,
    /// Tabs
    pub tabs: Vec<Tab>,
    /// Active tab
    pub active: Option<TabId>,
}

impl TabGroup {
    pub fn new() -> Self {
        Self {
            id: TabGroupId::new(),
            tabs: Vec::new(),
            active: None,
        }
    }
}

impl Default for TabGroup {
    fn default() -> Self {
        Self::new()
    }
}

/// Tab event
#[derive(Debug, Clone)]
pub enum TabEvent {
    Opened(TabId),
    Closed(TabId),
    Activated(TabId),
    Pinned(TabId),
    Unpinned(TabId),
    DirtyChanged(TabId, bool),
    PreviewPromoted(TabId),
    Moved(TabId),
}

/// Tabs configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabsConfig {
    /// Show tabs
    pub show_tabs: bool,
    /// Tab sizing
    pub tab_sizing: TabSizing,
    /// Close button position
    pub close_button: CloseButtonPosition,
    /// Open position
    pub open_position: TabOpenPosition,
    /// Show icons
    pub show_icons: bool,
    /// Preview mode
    pub enable_preview: bool,
    /// Wrap tabs
    pub wrap_tabs: bool,
    /// Max tabs per group
    pub max_tabs: Option<u32>,
}

impl Default for TabsConfig {
    fn default() -> Self {
        Self {
            show_tabs: true,
            tab_sizing: TabSizing::Fit,
            close_button: CloseButtonPosition::Right,
            open_position: TabOpenPosition::End,
            show_icons: true,
            enable_preview: true,
            wrap_tabs: true,
            max_tabs: None,
        }
    }
}

/// Tab sizing
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TabSizing {
    /// Fit content
    Fit,
    /// Fixed width
    Fixed,
    /// Shrink to fit
    Shrink,
}

/// Close button position
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CloseButtonPosition {
    Left,
    Right,
    Off,
}

/// Tab open position
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TabOpenPosition {
    End,
    AfterActive,
    First,
}
