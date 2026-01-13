//! # Foxkit Workspace Layout
//!
//! Window and view layout management.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

static VIEW_ID: AtomicU64 = AtomicU64::new(1);

/// Workspace layout service
pub struct WorkspaceLayoutService {
    /// Current layout
    layout: RwLock<WorkspaceLayout>,
    /// Saved layouts
    saved_layouts: RwLock<HashMap<String, WorkspaceLayout>>,
    /// Configuration
    config: RwLock<LayoutConfig>,
    /// Event sender
    event_tx: broadcast::Sender<LayoutEvent>,
}

impl WorkspaceLayoutService {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(64);

        Self {
            layout: RwLock::new(WorkspaceLayout::default()),
            saved_layouts: RwLock::new(HashMap::new()),
            config: RwLock::new(LayoutConfig::default()),
            event_tx,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<LayoutEvent> {
        self.event_tx.subscribe()
    }

    /// Get current layout
    pub fn layout(&self) -> WorkspaceLayout {
        self.layout.read().clone()
    }

    /// Set layout
    pub fn set_layout(&self, layout: WorkspaceLayout) {
        *self.layout.write() = layout.clone();
        let _ = self.event_tx.send(LayoutEvent::Changed(layout));
    }

    /// Toggle sidebar
    pub fn toggle_sidebar(&self) {
        let mut layout = self.layout.write();
        layout.sidebar.visible = !layout.sidebar.visible;
        let _ = self.event_tx.send(LayoutEvent::SidebarToggled(layout.sidebar.visible));
    }

    /// Set sidebar position
    pub fn set_sidebar_position(&self, position: SidebarPosition) {
        let mut layout = self.layout.write();
        layout.sidebar.position = position;
        let _ = self.event_tx.send(LayoutEvent::SidebarPositionChanged(position));
    }

    /// Toggle panel
    pub fn toggle_panel(&self) {
        let mut layout = self.layout.write();
        layout.panel.visible = !layout.panel.visible;
        let _ = self.event_tx.send(LayoutEvent::PanelToggled(layout.panel.visible));
    }

    /// Set panel position
    pub fn set_panel_position(&self, position: PanelPosition) {
        let mut layout = self.layout.write();
        layout.panel.position = position;
        let _ = self.event_tx.send(LayoutEvent::PanelPositionChanged(position));
    }

    /// Toggle activity bar
    pub fn toggle_activity_bar(&self) {
        let mut layout = self.layout.write();
        layout.activity_bar.visible = !layout.activity_bar.visible;
        let _ = self.event_tx.send(LayoutEvent::ActivityBarToggled(layout.activity_bar.visible));
    }

    /// Toggle status bar
    pub fn toggle_status_bar(&self) {
        let mut layout = self.layout.write();
        layout.status_bar.visible = !layout.status_bar.visible;
        let _ = self.event_tx.send(LayoutEvent::StatusBarToggled(layout.status_bar.visible));
    }

    /// Set sidebar width
    pub fn set_sidebar_width(&self, width: u32) {
        let config = self.config.read();
        let clamped = width.clamp(config.min_sidebar_width, config.max_sidebar_width);
        
        let mut layout = self.layout.write();
        layout.sidebar.width = clamped;
    }

    /// Set panel height
    pub fn set_panel_height(&self, height: u32) {
        let config = self.config.read();
        let clamped = height.clamp(config.min_panel_height, config.max_panel_height);
        
        let mut layout = self.layout.write();
        layout.panel.height = clamped;
    }

    /// Save current layout
    pub fn save_layout(&self, name: impl Into<String>) {
        let layout = self.layout.read().clone();
        self.saved_layouts.write().insert(name.into(), layout);
    }

    /// Load saved layout
    pub fn load_layout(&self, name: &str) -> bool {
        if let Some(layout) = self.saved_layouts.read().get(name).cloned() {
            *self.layout.write() = layout.clone();
            let _ = self.event_tx.send(LayoutEvent::Changed(layout));
            true
        } else {
            false
        }
    }

    /// Delete saved layout
    pub fn delete_layout(&self, name: &str) {
        self.saved_layouts.write().remove(name);
    }

    /// List saved layouts
    pub fn saved_layouts(&self) -> Vec<String> {
        self.saved_layouts.read().keys().cloned().collect()
    }

    /// Reset to default layout
    pub fn reset(&self) {
        let layout = WorkspaceLayout::default();
        *self.layout.write() = layout.clone();
        let _ = self.event_tx.send(LayoutEvent::Changed(layout));
    }

    /// Center editor layout
    pub fn center_editor(&self, enabled: bool, width_ratio: f32) {
        let mut layout = self.layout.write();
        layout.editor_area.centered = enabled;
        layout.editor_area.center_width_ratio = width_ratio.clamp(0.3, 1.0);
        let _ = self.event_tx.send(LayoutEvent::EditorCentered { enabled, width_ratio });
    }

    /// Maximize editor (hide everything else)
    pub fn maximize_editor(&self) {
        let mut layout = self.layout.write();
        layout.sidebar.visible = false;
        layout.panel.visible = false;
        layout.activity_bar.visible = false;
        let _ = self.event_tx.send(LayoutEvent::EditorMaximized);
    }

    /// Configure
    pub fn configure(&self, config: LayoutConfig) {
        *self.config.write() = config;
    }

    /// Add auxiliary bar
    pub fn add_auxiliary_bar(&self, position: AuxiliaryBarPosition) {
        let mut layout = self.layout.write();
        layout.auxiliary_bar = Some(AuxiliaryBar {
            visible: true,
            position,
            width: 300,
        });
    }

    /// Toggle auxiliary bar
    pub fn toggle_auxiliary_bar(&self) {
        let mut layout = self.layout.write();
        if let Some(ref mut bar) = layout.auxiliary_bar {
            bar.visible = !bar.visible;
        }
    }
}

impl Default for WorkspaceLayoutService {
    fn default() -> Self {
        Self::new()
    }
}

/// Workspace layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    /// Sidebar
    pub sidebar: SidebarLayout,
    /// Panel
    pub panel: PanelLayout,
    /// Activity bar
    pub activity_bar: ActivityBarLayout,
    /// Status bar
    pub status_bar: StatusBarLayout,
    /// Editor area
    pub editor_area: EditorAreaLayout,
    /// Auxiliary bar
    pub auxiliary_bar: Option<AuxiliaryBar>,
}

impl Default for WorkspaceLayout {
    fn default() -> Self {
        Self {
            sidebar: SidebarLayout::default(),
            panel: PanelLayout::default(),
            activity_bar: ActivityBarLayout::default(),
            status_bar: StatusBarLayout::default(),
            editor_area: EditorAreaLayout::default(),
            auxiliary_bar: None,
        }
    }
}

/// Sidebar layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarLayout {
    pub visible: bool,
    pub position: SidebarPosition,
    pub width: u32,
    pub active_view: Option<String>,
}

impl Default for SidebarLayout {
    fn default() -> Self {
        Self {
            visible: true,
            position: SidebarPosition::Left,
            width: 300,
            active_view: Some("explorer".to_string()),
        }
    }
}

/// Sidebar position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SidebarPosition {
    Left,
    Right,
}

/// Panel layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelLayout {
    pub visible: bool,
    pub position: PanelPosition,
    pub height: u32,
    pub maximized: bool,
    pub active_view: Option<String>,
}

impl Default for PanelLayout {
    fn default() -> Self {
        Self {
            visible: true,
            position: PanelPosition::Bottom,
            height: 300,
            maximized: false,
            active_view: Some("terminal".to_string()),
        }
    }
}

/// Panel position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelPosition {
    Bottom,
    Left,
    Right,
}

/// Activity bar layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityBarLayout {
    pub visible: bool,
    pub position: ActivityBarPosition,
}

impl Default for ActivityBarLayout {
    fn default() -> Self {
        Self {
            visible: true,
            position: ActivityBarPosition::Side,
        }
    }
}

/// Activity bar position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityBarPosition {
    Side,
    Top,
    Hidden,
}

/// Status bar layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusBarLayout {
    pub visible: bool,
}

impl Default for StatusBarLayout {
    fn default() -> Self {
        Self { visible: true }
    }
}

/// Editor area layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorAreaLayout {
    pub centered: bool,
    pub center_width_ratio: f32,
}

impl Default for EditorAreaLayout {
    fn default() -> Self {
        Self {
            centered: false,
            center_width_ratio: 0.6,
        }
    }
}

/// Auxiliary bar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuxiliaryBar {
    pub visible: bool,
    pub position: AuxiliaryBarPosition,
    pub width: u32,
}

/// Auxiliary bar position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuxiliaryBarPosition {
    Left,
    Right,
}

/// Layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub min_sidebar_width: u32,
    pub max_sidebar_width: u32,
    pub min_panel_height: u32,
    pub max_panel_height: u32,
    pub restore_on_startup: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            min_sidebar_width: 170,
            max_sidebar_width: 800,
            min_panel_height: 100,
            max_panel_height: 800,
            restore_on_startup: true,
        }
    }
}

/// View ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViewId(u64);

impl ViewId {
    pub fn new() -> Self {
        Self(VIEW_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for ViewId {
    fn default() -> Self {
        Self::new()
    }
}

/// Layout event
#[derive(Debug, Clone)]
pub enum LayoutEvent {
    Changed(WorkspaceLayout),
    SidebarToggled(bool),
    SidebarPositionChanged(SidebarPosition),
    PanelToggled(bool),
    PanelPositionChanged(PanelPosition),
    ActivityBarToggled(bool),
    StatusBarToggled(bool),
    EditorCentered { enabled: bool, width_ratio: f32 },
    EditorMaximized,
}

/// Preset layouts
pub mod presets {
    use super::*;

    pub fn default_layout() -> WorkspaceLayout {
        WorkspaceLayout::default()
    }

    pub fn minimal() -> WorkspaceLayout {
        WorkspaceLayout {
            sidebar: SidebarLayout { visible: false, ..Default::default() },
            panel: PanelLayout { visible: false, ..Default::default() },
            activity_bar: ActivityBarLayout { visible: false, ..Default::default() },
            status_bar: StatusBarLayout { visible: true },
            editor_area: EditorAreaLayout::default(),
            auxiliary_bar: None,
        }
    }

    pub fn focus() -> WorkspaceLayout {
        WorkspaceLayout {
            sidebar: SidebarLayout { visible: false, ..Default::default() },
            panel: PanelLayout { visible: false, ..Default::default() },
            activity_bar: ActivityBarLayout { visible: false, ..Default::default() },
            status_bar: StatusBarLayout { visible: false },
            editor_area: EditorAreaLayout {
                centered: true,
                center_width_ratio: 0.6,
            },
            auxiliary_bar: None,
        }
    }

    pub fn presentation() -> WorkspaceLayout {
        WorkspaceLayout {
            sidebar: SidebarLayout { visible: false, ..Default::default() },
            panel: PanelLayout { visible: false, ..Default::default() },
            activity_bar: ActivityBarLayout { visible: false, ..Default::default() },
            status_bar: StatusBarLayout { visible: false },
            editor_area: EditorAreaLayout {
                centered: true,
                center_width_ratio: 0.8,
            },
            auxiliary_bar: None,
        }
    }
}
