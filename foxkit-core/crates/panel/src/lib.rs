//! # Foxkit Panel
//!
//! Panel and view system for sidebars, bottom panel, etc.

pub mod container;
pub mod view;
pub mod registry;

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub use container::{PanelContainer, PanelPosition};
pub use view::{View, ViewId, ViewState};
pub use registry::ViewRegistry;

/// Panel manager
pub struct PanelManager {
    /// View registry
    registry: ViewRegistry,
    /// Panel containers by position
    containers: HashMap<PanelPosition, PanelContainer>,
    /// Active view per container
    active_views: HashMap<PanelPosition, ViewId>,
    /// Collapsed panels
    collapsed: HashMap<PanelPosition, bool>,
    /// Panel sizes (width or height)
    sizes: HashMap<PanelPosition, f32>,
}

impl PanelManager {
    pub fn new() -> Self {
        let mut containers = HashMap::new();
        containers.insert(PanelPosition::Left, PanelContainer::new(PanelPosition::Left));
        containers.insert(PanelPosition::Right, PanelContainer::new(PanelPosition::Right));
        containers.insert(PanelPosition::Bottom, PanelContainer::new(PanelPosition::Bottom));

        Self {
            registry: ViewRegistry::new(),
            containers,
            active_views: HashMap::new(),
            collapsed: HashMap::new(),
            sizes: HashMap::new(),
        }
    }

    /// Register a view
    pub fn register_view(&mut self, view: Box<dyn View>) {
        let position = view.default_position();
        let id = view.id();
        
        self.registry.register(view);
        
        if let Some(container) = self.containers.get_mut(&position) {
            container.add_view(id.clone());
        }
    }

    /// Show a view
    pub fn show_view(&mut self, id: &ViewId) {
        if let Some(view) = self.registry.get(id) {
            let position = view.default_position();
            self.active_views.insert(position, id.clone());
            self.collapsed.insert(position, false);
        }
    }

    /// Hide a view
    pub fn hide_view(&mut self, id: &ViewId) {
        if let Some(view) = self.registry.get(id) {
            let position = view.default_position();
            if self.active_views.get(&position) == Some(id) {
                self.active_views.remove(&position);
            }
        }
    }

    /// Toggle panel visibility
    pub fn toggle_panel(&mut self, position: PanelPosition) {
        let collapsed = self.collapsed.entry(position).or_insert(false);
        *collapsed = !*collapsed;
    }

    /// Is panel visible?
    pub fn is_visible(&self, position: PanelPosition) -> bool {
        !self.collapsed.get(&position).copied().unwrap_or(false)
    }

    /// Get active view for position
    pub fn active_view(&self, position: PanelPosition) -> Option<&ViewId> {
        self.active_views.get(&position)
    }

    /// Set panel size
    pub fn set_size(&mut self, position: PanelPosition, size: f32) {
        self.sizes.insert(position, size);
    }

    /// Get panel size
    pub fn size(&self, position: PanelPosition) -> f32 {
        *self.sizes.get(&position).unwrap_or(&DEFAULT_PANEL_SIZE)
    }

    /// Get all views for a position
    pub fn views(&self, position: PanelPosition) -> Vec<&ViewId> {
        self.containers
            .get(&position)
            .map(|c| c.views())
            .unwrap_or_default()
    }

    /// Focus a panel
    pub fn focus(&mut self, position: PanelPosition) {
        // Mark as not collapsed and bring to front
        self.collapsed.insert(position, false);
    }

    /// Get view by id
    pub fn get_view(&self, id: &ViewId) -> Option<&dyn View> {
        self.registry.get(id)
    }

    /// Get view mutably
    pub fn get_view_mut(&mut self, id: &ViewId) -> Option<&mut dyn View> {
        self.registry.get_mut(id)
    }
}

impl Default for PanelManager {
    fn default() -> Self {
        Self::new()
    }
}

const DEFAULT_PANEL_SIZE: f32 = 300.0;

/// Built-in view IDs
pub mod views {
    use super::ViewId;

    pub const EXPLORER: ViewId = ViewId::new("workbench.view.explorer");
    pub const SEARCH: ViewId = ViewId::new("workbench.view.search");
    pub const SCM: ViewId = ViewId::new("workbench.view.scm");
    pub const DEBUG: ViewId = ViewId::new("workbench.view.debug");
    pub const EXTENSIONS: ViewId = ViewId::new("workbench.view.extensions");
    pub const PROBLEMS: ViewId = ViewId::new("workbench.panel.markers");
    pub const OUTPUT: ViewId = ViewId::new("workbench.panel.output");
    pub const TERMINAL: ViewId = ViewId::new("workbench.panel.terminal");
    pub const DEBUG_CONSOLE: ViewId = ViewId::new("workbench.panel.repl");
}
