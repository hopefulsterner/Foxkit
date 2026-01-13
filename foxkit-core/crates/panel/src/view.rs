//! View trait and types

use std::any::Any;
use crate::PanelPosition;

/// View identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewId(String);

impl ViewId {
    pub const fn new(id: &'static str) -> Self {
        // Note: const fn with String::from not available, this is pseudo-code
        // In real impl would use Cow<'static, str>
        Self(String::new()) // Placeholder
    }

    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// View state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewState {
    /// View is hidden
    Hidden,
    /// View is visible but not focused
    Visible,
    /// View is visible and focused
    Focused,
}

/// A view in a panel
pub trait View: Send + Sync {
    /// Get view ID
    fn id(&self) -> ViewId;

    /// Get display name
    fn name(&self) -> &str;

    /// Get icon (codicon name)
    fn icon(&self) -> &str {
        "symbol-misc"
    }

    /// Default panel position
    fn default_position(&self) -> PanelPosition {
        PanelPosition::Left
    }

    /// Priority for ordering in panel
    fn priority(&self) -> i32 {
        100
    }

    /// Can this view be closed?
    fn closeable(&self) -> bool {
        true
    }

    /// Render the view (returns element tree)
    fn render(&self) -> Box<dyn Any>;

    /// Handle view becoming visible
    fn on_show(&mut self) {}

    /// Handle view being hidden
    fn on_hide(&mut self) {}

    /// Handle view receiving focus
    fn on_focus(&mut self) {}

    /// Handle view losing focus
    fn on_blur(&mut self) {}

    /// Get context for keybindings
    fn context(&self) -> Vec<(&str, &str)> {
        Vec::new()
    }
}

/// View configuration
#[derive(Debug, Clone)]
pub struct ViewConfig {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub position: PanelPosition,
    pub priority: i32,
    pub closeable: bool,
    pub when: Option<String>,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            icon: "symbol-misc".to_string(),
            position: PanelPosition::Left,
            priority: 100,
            closeable: true,
            when: None,
        }
    }
}

/// Activity bar item (left sidebar icons)
#[derive(Debug, Clone)]
pub struct ActivityBarItem {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub view_container: String,
    pub priority: i32,
}

/// View container (group of views)
#[derive(Debug, Clone)]
pub struct ViewContainer {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub views: Vec<String>,
}
