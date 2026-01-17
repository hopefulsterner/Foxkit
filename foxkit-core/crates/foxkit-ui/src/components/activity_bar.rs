//! Activity Bar Component
//!
//! The leftmost vertical bar with icon buttons for switching views.

use crate::theme::Theme;
use std::collections::HashMap;

/// Activity bar state
#[derive(Debug, Clone)]
pub struct ActivityBarState {
    /// Activities (icon buttons)
    pub items: Vec<ActivityItem>,
    /// Currently active item
    pub active: Option<ActivityId>,
    /// Badge counts per item
    pub badges: HashMap<ActivityId, u32>,
}

impl ActivityBarState {
    pub fn new() -> Self {
        Self {
            items: vec![
                ActivityItem {
                    id: ActivityId::Explorer,
                    icon: "files",
                    tooltip: "Explorer (Ctrl+Shift+E)",
                    order: 0,
                },
                ActivityItem {
                    id: ActivityId::Search,
                    icon: "search",
                    tooltip: "Search (Ctrl+Shift+F)",
                    order: 1,
                },
                ActivityItem {
                    id: ActivityId::SourceControl,
                    icon: "git-branch",
                    tooltip: "Source Control (Ctrl+Shift+G)",
                    order: 2,
                },
                ActivityItem {
                    id: ActivityId::Debug,
                    icon: "debug",
                    tooltip: "Run and Debug (Ctrl+Shift+D)",
                    order: 3,
                },
                ActivityItem {
                    id: ActivityId::Extensions,
                    icon: "extensions",
                    tooltip: "Extensions (Ctrl+Shift+X)",
                    order: 4,
                },
                ActivityItem {
                    id: ActivityId::AiAgent,
                    icon: "robot",
                    tooltip: "AI Agent (Ctrl+Shift+A)",
                    order: 5,
                },
            ],
            active: Some(ActivityId::Explorer),
            badges: HashMap::new(),
        }
    }

    /// Set active item
    pub fn set_active(&mut self, id: ActivityId) {
        self.active = Some(id);
    }

    /// Toggle active item (clicking same item hides sidebar)
    pub fn toggle(&mut self, id: ActivityId) -> bool {
        if self.active == Some(id) {
            self.active = None;
            false
        } else {
            self.active = Some(id);
            true
        }
    }

    /// Set badge count for an item
    pub fn set_badge(&mut self, id: ActivityId, count: u32) {
        if count > 0 {
            self.badges.insert(id, count);
        } else {
            self.badges.remove(&id);
        }
    }

    /// Render the activity bar
    pub fn render(&self, theme: &Theme) -> ActivityBarView {
        ActivityBarView {
            items: self.items.iter().map(|item| {
                ActivityItemView {
                    id: item.id,
                    icon: item.icon,
                    tooltip: item.tooltip,
                    is_active: self.active == Some(item.id),
                    badge: self.badges.get(&item.id).map(|n| n.to_string()),
                    colors: ActivityItemColors {
                        background: theme.colors.activity_bar_bg,
                        foreground: if self.active == Some(item.id) {
                            theme.colors.activity_bar_fg
                        } else {
                            theme.colors.activity_bar_inactive_fg
                        },
                        indicator: theme.colors.activity_bar_active_border,
                        badge_bg: theme.colors.activity_bar_badge_bg,
                        badge_fg: theme.colors.activity_bar_badge_fg,
                    },
                }
            }).collect(),
            colors: ActivityBarColors {
                background: theme.colors.activity_bar_bg,
                foreground: theme.colors.activity_bar_fg,
                inactive_foreground: theme.colors.activity_bar_inactive_fg,
                active_background: theme.colors.sidebar_item_active_bg,
                active_border: theme.colors.activity_bar_active_border,
                badge_background: theme.colors.activity_bar_badge_bg,
                badge_foreground: theme.colors.activity_bar_badge_fg,
            },
        }
    }
}

impl Default for ActivityBarState {
    fn default() -> Self {
        Self::new()
    }
}

/// Activity item identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityId {
    Explorer,
    Search,
    SourceControl,
    Debug,
    Extensions,
    AiAgent,
    Custom(u32),
}

/// An item in the activity bar
#[derive(Debug, Clone)]
pub struct ActivityItem {
    pub id: ActivityId,
    pub icon: &'static str,
    pub tooltip: &'static str,
    pub order: u32,
}

/// Rendered activity bar view
#[derive(Debug, Clone)]
pub struct ActivityBarView {
    pub items: Vec<ActivityItemView>,
    pub colors: ActivityBarColors,
}

/// Rendered activity item
#[derive(Debug, Clone)]
pub struct ActivityItemView {
    pub id: ActivityId,
    pub icon: &'static str,
    pub tooltip: &'static str,
    pub is_active: bool,
    pub badge: Option<String>,
    pub colors: ActivityItemColors,
}

/// Activity bar colors
#[derive(Debug, Clone, Copy)]
pub struct ActivityBarColors {
    pub background: Color,
    pub foreground: Color,
    pub inactive_foreground: Color,
    pub active_background: Color,
    pub active_border: Color,
    pub badge_background: Color,
    pub badge_foreground: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct ActivityItemColors {
    pub background: Color,
    pub foreground: Color,
    pub indicator: Color,
    pub badge_bg: Color,
    pub badge_fg: Color,
}

/// Simple RGBA color
#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self { 
            r: r as f32 / 255.0, 
            g: g as f32 / 255.0, 
            b: b as f32 / 255.0, 
            a 
        }
    }

    pub const fn hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as f32 / 255.0,
            g: ((hex >> 8) & 0xFF) as f32 / 255.0,
            b: (hex & 0xFF) as f32 / 255.0,
            a: 1.0,
        }
    }

    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
}
