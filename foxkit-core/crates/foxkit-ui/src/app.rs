//! Foxkit Application Shell
//!
//! Main application layout with all panels and components.

use crate::components::*;
use crate::theme::Theme;

/// Main application state
pub struct App {
    /// Current theme
    pub theme: Theme,
    /// Layout state
    pub layout: AppLayout,
    /// Activity bar state
    pub activity_bar: ActivityBarState,
    /// Sidebar state
    pub sidebar: SidebarState,
    /// Editor area state
    pub editor_area: EditorAreaState,
    /// Bottom panel state
    pub bottom_panel: BottomPanelState,
    /// Status bar state
    pub status_bar: StatusBarState,
}

impl App {
    pub fn new() -> Self {
        Self {
            theme: Theme::dark(),
            layout: AppLayout::default(),
            activity_bar: ActivityBarState::new(),
            sidebar: SidebarState::new(),
            editor_area: EditorAreaState::new(),
            bottom_panel: BottomPanelState::new(),
            status_bar: StatusBarState::new(),
        }
    }

    /// Render the entire application
    pub fn render(&self) -> AppView {
        AppView {
            theme: &self.theme,
            layout: &self.layout,
            activity_bar: self.activity_bar.render(&self.theme),
            sidebar: self.sidebar.render(&self.theme),
            editor_area: self.editor_area.render(&self.theme),
            bottom_panel: self.bottom_panel.render(&self.theme),
            status_bar: self.status_bar.render(&self.theme),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Application layout configuration
#[derive(Debug, Clone)]
pub struct AppLayout {
    /// Show activity bar
    pub show_activity_bar: bool,
    /// Show sidebar
    pub show_sidebar: bool,
    /// Show bottom panel
    pub show_bottom_panel: bool,
    /// Show status bar
    pub show_status_bar: bool,
    /// Sidebar width
    pub sidebar_width: f32,
    /// Bottom panel height
    pub bottom_panel_height: f32,
    /// Activity bar position (left or right)
    pub activity_bar_position: Position,
    /// Sidebar position (left or right)
    pub sidebar_position: Position,
    /// Bottom panel position
    pub bottom_panel_position: BottomPanelPosition,
}

impl Default for AppLayout {
    fn default() -> Self {
        Self {
            show_activity_bar: true,
            show_sidebar: true,
            show_bottom_panel: true,
            show_status_bar: true,
            sidebar_width: 260.0,
            bottom_panel_height: 300.0,
            activity_bar_position: Position::Left,
            sidebar_position: Position::Left,
            bottom_panel_position: BottomPanelPosition::Bottom,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomPanelPosition {
    Bottom,
    Left,
    Right,
}

/// The rendered application view
pub struct AppView<'a> {
    pub theme: &'a Theme,
    pub layout: &'a AppLayout,
    pub activity_bar: ActivityBarView,
    pub sidebar: SidebarView,
    pub editor_area: EditorAreaView,
    pub bottom_panel: BottomPanelView,
    pub status_bar: StatusBarView,
}

impl<'a> AppView<'a> {
    /// Calculate the bounds for each component
    pub fn layout_bounds(&self, window_width: f32, window_height: f32) -> LayoutBounds {
        let activity_bar_width = if self.layout.show_activity_bar { 48.0 } else { 0.0 };
        let sidebar_width = if self.layout.show_sidebar { self.layout.sidebar_width } else { 0.0 };
        let status_bar_height = if self.layout.show_status_bar { 22.0 } else { 0.0 };
        let bottom_panel_height = if self.layout.show_bottom_panel { self.layout.bottom_panel_height } else { 0.0 };

        let content_height = window_height - status_bar_height;
        let editor_height = content_height - bottom_panel_height;
        let editor_width = window_width - activity_bar_width - sidebar_width;

        LayoutBounds {
            activity_bar: Rect {
                x: 0.0,
                y: 0.0,
                width: activity_bar_width,
                height: content_height,
            },
            sidebar: Rect {
                x: activity_bar_width,
                y: 0.0,
                width: sidebar_width,
                height: content_height,
            },
            editor_area: Rect {
                x: activity_bar_width + sidebar_width,
                y: 0.0,
                width: editor_width,
                height: editor_height,
            },
            bottom_panel: Rect {
                x: activity_bar_width + sidebar_width,
                y: editor_height,
                width: editor_width,
                height: bottom_panel_height,
            },
            status_bar: Rect {
                x: 0.0,
                y: content_height,
                width: window_width,
                height: status_bar_height,
            },
        }
    }
}

/// Calculated layout bounds for all components
#[derive(Debug, Clone)]
pub struct LayoutBounds {
    pub activity_bar: Rect,
    pub sidebar: Rect,
    pub editor_area: Rect,
    pub bottom_panel: Rect,
    pub status_bar: Rect,
}

/// A simple rectangle
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}
