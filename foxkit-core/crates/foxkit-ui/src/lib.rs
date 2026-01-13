//! Foxkit UI - High-level UI components and widgets.
//!
//! This crate provides reusable UI components built on top of foxkit-gpui.

use serde::{Deserialize, Serialize};
use foxkit_gpui::{Color, Corners, Edges, Size};

/// Button variant styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Danger,
}

/// Button component properties.
#[derive(Debug, Clone)]
pub struct ButtonProps {
    pub label: String,
    pub variant: ButtonVariant,
    pub disabled: bool,
    pub icon: Option<String>,
}

/// Input field properties.
#[derive(Debug, Clone)]
pub struct InputProps {
    pub value: String,
    pub placeholder: String,
    pub disabled: bool,
    pub password: bool,
}

/// Checkbox properties.
#[derive(Debug, Clone)]
pub struct CheckboxProps {
    pub checked: bool,
    pub label: Option<String>,
    pub disabled: bool,
}

/// Icon button properties.
#[derive(Debug, Clone)]
pub struct IconButtonProps {
    pub icon: String,
    pub tooltip: Option<String>,
    pub disabled: bool,
}

/// Label properties.
#[derive(Debug, Clone)]
pub struct LabelProps {
    pub text: String,
    pub color: Option<Color>,
    pub size: LabelSize,
}

/// Label size variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelSize {
    Small,
    #[default]
    Medium,
    Large,
}

/// Divider orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DividerOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Panel properties.
#[derive(Debug, Clone)]
pub struct PanelProps {
    pub title: Option<String>,
    pub collapsible: bool,
    pub collapsed: bool,
}

/// Scrollbar properties.
#[derive(Debug, Clone)]
pub struct ScrollbarProps {
    pub orientation: DividerOrientation,
    pub thumb_size: f32,
    pub thumb_position: f32,
}

/// Tooltip properties.
#[derive(Debug, Clone)]
pub struct TooltipProps {
    pub text: String,
    pub delay_ms: u32,
}

/// Badge properties.
#[derive(Debug, Clone)]
pub struct BadgeProps {
    pub text: String,
    pub variant: BadgeVariant,
}

/// Badge variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeVariant {
    #[default]
    Default,
    Info,
    Success,
    Warning,
    Error,
}

/// Progress bar properties.
#[derive(Debug, Clone)]
pub struct ProgressBarProps {
    pub value: f32, // 0.0 to 1.0
    pub indeterminate: bool,
}

/// Tab item.
#[derive(Debug, Clone)]
pub struct TabItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub closeable: bool,
}

/// Tab bar properties.
#[derive(Debug, Clone)]
pub struct TabBarProps {
    pub tabs: Vec<TabItem>,
    pub active_tab: Option<String>,
}

/// Modal dialog properties.
#[derive(Debug, Clone)]
pub struct ModalProps {
    pub title: String,
    pub closeable: bool,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

/// Context menu item.
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
    pub icon: Option<String>,
    pub disabled: bool,
    pub submenu: Option<Vec<MenuItem>>,
}

/// Context menu properties.
#[derive(Debug, Clone)]
pub struct ContextMenuProps {
    pub items: Vec<MenuItem>,
}

/// Tree view item.
#[derive(Debug, Clone)]
pub struct TreeItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub expanded: bool,
    pub children: Vec<TreeItem>,
}

/// Tree view properties.
#[derive(Debug, Clone)]
pub struct TreeViewProps {
    pub items: Vec<TreeItem>,
    pub selected: Option<String>,
}

/// List item.
#[derive(Debug, Clone)]
pub struct ListItem {
    pub id: String,
    pub primary_text: String,
    pub secondary_text: Option<String>,
    pub icon: Option<String>,
}

/// List view properties.
#[derive(Debug, Clone)]
pub struct ListViewProps {
    pub items: Vec<ListItem>,
    pub selected: Option<String>,
}
