//! Debug views

use serde::{Deserialize, Serialize};

/// Debug view ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DebugViewId {
    Breakpoints,
    Variables,
    CallStack,
    Watch,
    Console,
    Loaded,
    Threads,
}

impl DebugViewId {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Breakpoints => "debug.breakpoints",
            Self::Variables => "debug.variables",
            Self::CallStack => "debug.callstack",
            Self::Watch => "debug.watch",
            Self::Console => "debug.console",
            Self::Loaded => "debug.loaded",
            Self::Threads => "debug.threads",
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Breakpoints => "Breakpoints",
            Self::Variables => "Variables",
            Self::CallStack => "Call Stack",
            Self::Watch => "Watch",
            Self::Console => "Debug Console",
            Self::Loaded => "Loaded Scripts",
            Self::Threads => "Threads",
        }
    }
}

/// Debug view trait
pub trait DebugView {
    /// View ID
    fn id(&self) -> DebugViewId;

    /// View title
    fn title(&self) -> &str;

    /// Is view visible?
    fn is_visible(&self) -> bool;

    /// Show view
    fn show(&mut self);

    /// Hide view
    fn hide(&mut self);

    /// Refresh view content
    fn refresh(&mut self);

    /// Clear view content
    fn clear(&mut self);
}

/// View container for debug views
pub struct DebugViewContainer {
    /// Views in container
    views: Vec<Box<dyn DebugView + Send + Sync>>,
    /// Active view index
    active: usize,
    /// Container visibility
    visible: bool,
}

impl DebugViewContainer {
    pub fn new() -> Self {
        Self {
            views: Vec::new(),
            active: 0,
            visible: false,
        }
    }

    /// Add view to container
    pub fn add_view<V: DebugView + Send + Sync + 'static>(&mut self, view: V) {
        self.views.push(Box::new(view));
    }

    /// Get active view
    pub fn active_view(&self) -> Option<&(dyn DebugView + Send + Sync)> {
        self.views.get(self.active).map(|v| &**v)
    }

    /// Set active view by ID
    pub fn set_active(&mut self, id: DebugViewId) {
        if let Some(pos) = self.views.iter().position(|v| v.id() == id) {
            self.active = pos;
        }
    }

    /// Show container
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide container
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Refresh all views
    pub fn refresh_all(&mut self) {
        for view in &mut self.views {
            view.refresh();
        }
    }
}

impl Default for DebugViewContainer {
    fn default() -> Self {
        Self::new()
    }
}
