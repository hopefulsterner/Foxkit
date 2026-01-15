//! # Foxkit Window Management
//!
//! Editor window splits, tabs, and layout management.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Window management service
pub struct WindowManagementService {
    /// Root layout
    layout: RwLock<WindowLayout>,
    /// Events
    events: broadcast::Sender<WindowEvent>,
    /// Configuration
    config: RwLock<WindowConfig>,
}

impl WindowManagementService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            layout: RwLock::new(WindowLayout::single()),
            events,
            config: RwLock::new(WindowConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<WindowEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: WindowConfig) {
        *self.config.write() = config;
    }

    /// Get current layout
    pub fn layout(&self) -> WindowLayout {
        self.layout.read().clone()
    }

    /// Split editor
    pub fn split(&self, direction: SplitDirection) -> EditorGroupId {
        let mut layout = self.layout.write();
        let active_id = layout.active_group.clone();
        let new_id = EditorGroupId::new();

        // Create new group
        let new_group = EditorGroup::new(new_id.clone());

        // Add split
        layout.groups.insert(new_id.clone(), new_group);

        // Update split structure
        let split = Split {
            direction,
            first: active_id.clone(),
            second: new_id.clone(),
            ratio: 0.5,
        };

        layout.splits.push(split);
        layout.active_group = new_id.clone();

        let _ = self.events.send(WindowEvent::Split {
            direction,
            new_group: new_id.clone(),
        });

        new_id
    }

    /// Close editor group
    pub fn close_group(&self, group_id: &EditorGroupId) -> bool {
        let mut layout = self.layout.write();

        if layout.groups.len() <= 1 {
            return false; // Can't close last group
        }

        layout.groups.remove(group_id);
        layout.splits.retain(|s| &s.first != group_id && &s.second != group_id);

        // Update active group if needed
        if &layout.active_group == group_id {
            if let Some(id) = layout.groups.keys().next().cloned() {
                layout.active_group = id;
            }
        }

        let _ = self.events.send(WindowEvent::GroupClosed {
            group_id: group_id.clone(),
        });

        true
    }

    /// Set active group
    pub fn set_active_group(&self, group_id: EditorGroupId) {
        self.layout.write().active_group = group_id.clone();

        let _ = self.events.send(WindowEvent::ActiveGroupChanged {
            group_id,
        });
    }

    /// Get active group
    pub fn active_group(&self) -> EditorGroupId {
        self.layout.read().active_group.clone()
    }

    /// Open tab in group
    pub fn open_tab(&self, group_id: &EditorGroupId, tab: EditorTab) -> bool {
        let mut layout = self.layout.write();

        if let Some(group) = layout.groups.get_mut(group_id) {
            // Check if already open
            if let Some(idx) = group.tabs.iter().position(|t| t.file == tab.file) {
                group.active_tab = idx;
            } else {
                group.tabs.push(tab.clone());
                group.active_tab = group.tabs.len() - 1;
            }

            let _ = self.events.send(WindowEvent::TabOpened {
                group_id: group_id.clone(),
                tab,
            });

            true
        } else {
            false
        }
    }

    /// Close tab
    pub fn close_tab(&self, group_id: &EditorGroupId, tab_index: usize) -> bool {
        let mut layout = self.layout.write();

        if let Some(group) = layout.groups.get_mut(group_id) {
            if tab_index < group.tabs.len() {
                let tab = group.tabs.remove(tab_index);

                // Adjust active tab
                if group.active_tab >= group.tabs.len() && !group.tabs.is_empty() {
                    group.active_tab = group.tabs.len() - 1;
                }

                let _ = self.events.send(WindowEvent::TabClosed {
                    group_id: group_id.clone(),
                    file: tab.file,
                });

                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Move tab to another group
    pub fn move_tab(
        &self,
        from_group: &EditorGroupId,
        tab_index: usize,
        to_group: &EditorGroupId,
    ) -> bool {
        let mut layout = self.layout.write();

        // Get tab from source
        let tab = {
            let group = match layout.groups.get_mut(from_group) {
                Some(g) => g,
                None => return false,
            };
            if tab_index >= group.tabs.len() {
                return false;
            }
            group.tabs.remove(tab_index)
        };

        // Add to destination
        if let Some(group) = layout.groups.get_mut(to_group) {
            group.tabs.push(tab.clone());
            group.active_tab = group.tabs.len() - 1;

            let _ = self.events.send(WindowEvent::TabMoved {
                from: from_group.clone(),
                to: to_group.clone(),
                file: tab.file,
            });

            true
        } else {
            // Restore if destination doesn't exist
            if let Some(group) = layout.groups.get_mut(from_group) {
                group.tabs.insert(tab_index, tab);
            }
            false
        }
    }

    /// Get all open files
    pub fn open_files(&self) -> Vec<PathBuf> {
        let layout = self.layout.read();
        
        layout.groups.values()
            .flat_map(|g| g.tabs.iter().map(|t| t.file.clone()))
            .collect()
    }

    /// Find file in groups
    pub fn find_file(&self, file: &PathBuf) -> Option<(EditorGroupId, usize)> {
        let layout = self.layout.read();
        
        for (group_id, group) in &layout.groups {
            if let Some(idx) = group.tabs.iter().position(|t| &t.file == file) {
                return Some((group_id.clone(), idx));
            }
        }
        
        None
    }

    /// Resize split
    pub fn resize_split(&self, split_index: usize, ratio: f32) {
        let mut layout = self.layout.write();
        
        if let Some(split) = layout.splits.get_mut(split_index) {
            split.ratio = ratio.clamp(0.1, 0.9);
            
            let _ = self.events.send(WindowEvent::SplitResized {
                index: split_index,
                ratio: split.ratio,
            });
        }
    }

    /// Reset layout to single group
    pub fn reset_layout(&self) {
        let mut layout = self.layout.write();
        
        // Keep tabs from all groups in a single group
        let all_tabs: Vec<EditorTab> = layout.groups.values()
            .flat_map(|g| g.tabs.clone())
            .collect();

        *layout = WindowLayout::single();
        
        if let Some(group) = layout.groups.values_mut().next() {
            group.tabs = all_tabs;
        }

        let _ = self.events.send(WindowEvent::LayoutReset);
    }
}

impl Default for WindowManagementService {
    fn default() -> Self {
        Self::new()
    }
}

/// Editor group ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorGroupId(String);

impl EditorGroupId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn default_id() -> Self {
        Self("default".to_string())
    }
}

impl Default for EditorGroupId {
    fn default() -> Self {
        Self::default_id()
    }
}

/// Window layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowLayout {
    /// Editor groups
    pub groups: HashMap<EditorGroupId, EditorGroup>,
    /// Active group
    pub active_group: EditorGroupId,
    /// Splits
    pub splits: Vec<Split>,
}

impl WindowLayout {
    pub fn single() -> Self {
        let id = EditorGroupId::default_id();
        let mut groups = HashMap::new();
        groups.insert(id.clone(), EditorGroup::new(id.clone()));

        Self {
            groups,
            active_group: id,
            splits: Vec::new(),
        }
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }
}

/// Editor group (collection of tabs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorGroup {
    /// Group ID
    pub id: EditorGroupId,
    /// Tabs in this group
    pub tabs: Vec<EditorTab>,
    /// Active tab index
    pub active_tab: usize,
}

impl EditorGroup {
    pub fn new(id: EditorGroupId) -> Self {
        Self {
            id,
            tabs: Vec::new(),
            active_tab: 0,
        }
    }

    pub fn active(&self) -> Option<&EditorTab> {
        self.tabs.get(self.active_tab)
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }
}

/// Editor tab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorTab {
    /// File path
    pub file: PathBuf,
    /// Tab label
    pub label: String,
    /// Is dirty (unsaved changes)
    pub dirty: bool,
    /// Is pinned
    pub pinned: bool,
    /// Is preview (will be replaced when opening another file)
    pub preview: bool,
}

impl EditorTab {
    pub fn new(file: PathBuf) -> Self {
        let label = file.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
            .to_string();

        Self {
            file,
            label,
            dirty: false,
            pinned: false,
            preview: false,
        }
    }

    pub fn with_preview(mut self) -> Self {
        self.preview = true;
        self
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.preview = false;
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    pub fn pin(&mut self) {
        self.pinned = true;
        self.preview = false;
    }

    pub fn unpin(&mut self) {
        self.pinned = false;
    }
}

/// Split
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Split {
    /// Split direction
    pub direction: SplitDirection,
    /// First group
    pub first: EditorGroupId,
    /// Second group
    pub second: EditorGroupId,
    /// Split ratio (0.0 to 1.0)
    pub ratio: f32,
}

/// Split direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

impl SplitDirection {
    pub fn opposite(&self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Enable preview tabs
    pub preview_tabs: bool,
    /// Tab close button position
    pub tab_close_button: TabCloseButtonPosition,
    /// Show tab icons
    pub show_tab_icons: bool,
    /// Tab sizing mode
    pub tab_sizing: TabSizing,
    /// Wrap tabs
    pub wrap_tabs: bool,
    /// Maximum split count
    pub max_splits: usize,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            preview_tabs: true,
            tab_close_button: TabCloseButtonPosition::Right,
            show_tab_icons: true,
            tab_sizing: TabSizing::Fit,
            wrap_tabs: true,
            max_splits: 8,
        }
    }
}

/// Tab close button position
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TabCloseButtonPosition {
    Left,
    Right,
    Off,
}

/// Tab sizing
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TabSizing {
    Fit,
    Shrink,
    Fixed,
}

/// Window event
#[derive(Debug, Clone)]
pub enum WindowEvent {
    Split { direction: SplitDirection, new_group: EditorGroupId },
    GroupClosed { group_id: EditorGroupId },
    ActiveGroupChanged { group_id: EditorGroupId },
    TabOpened { group_id: EditorGroupId, tab: EditorTab },
    TabClosed { group_id: EditorGroupId, file: PathBuf },
    TabMoved { from: EditorGroupId, to: EditorGroupId, file: PathBuf },
    SplitResized { index: usize, ratio: f32 },
    LayoutReset,
}

/// Tab bar view model
pub struct TabBarViewModel {
    group: EditorGroup,
}

impl TabBarViewModel {
    pub fn new(group: EditorGroup) -> Self {
        Self { group }
    }

    pub fn tabs(&self) -> &[EditorTab] {
        &self.group.tabs
    }

    pub fn active_index(&self) -> usize {
        self.group.active_tab
    }

    pub fn is_tab_active(&self, index: usize) -> bool {
        self.group.active_tab == index
    }

    pub fn dirty_tabs(&self) -> Vec<usize> {
        self.group.tabs.iter()
            .enumerate()
            .filter(|(_, t)| t.dirty)
            .map(|(i, _)| i)
            .collect()
    }
}

/// Layout serialization for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedLayout {
    pub groups: Vec<PersistedGroup>,
    pub active_group: String,
    pub splits: Vec<Split>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedGroup {
    pub id: String,
    pub files: Vec<PathBuf>,
    pub active_index: usize,
}

impl From<&WindowLayout> for PersistedLayout {
    fn from(layout: &WindowLayout) -> Self {
        Self {
            groups: layout.groups.iter().map(|(id, g)| PersistedGroup {
                id: id.0.clone(),
                files: g.tabs.iter().map(|t| t.file.clone()).collect(),
                active_index: g.active_tab,
            }).collect(),
            active_group: layout.active_group.0.clone(),
            splits: layout.splits.clone(),
        }
    }
}
