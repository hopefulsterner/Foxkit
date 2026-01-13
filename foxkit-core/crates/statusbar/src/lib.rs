//! # Foxkit Status Bar
//!
//! Bottom status bar management.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Status bar service
pub struct StatusBarService {
    /// Items on the left
    left_items: RwLock<Vec<StatusBarItem>>,
    /// Items on the right
    right_items: RwLock<Vec<StatusBarItem>>,
    /// Items by ID
    items_by_id: RwLock<HashMap<String, StatusBarItem>>,
    /// Events
    events: broadcast::Sender<StatusBarEvent>,
    /// Next item ID
    next_id: AtomicU64,
}

impl StatusBarService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);
        
        Self {
            left_items: RwLock::new(Vec::new()),
            right_items: RwLock::new(Vec::new()),
            items_by_id: RwLock::new(HashMap::new()),
            events,
            next_id: AtomicU64::new(1),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<StatusBarEvent> {
        self.events.subscribe()
    }

    /// Create status bar item
    pub fn create_item(&self, alignment: StatusBarAlignment) -> StatusBarItemBuilder {
        let id = format!("item-{}", self.next_id.fetch_add(1, Ordering::SeqCst));
        StatusBarItemBuilder::new(id, alignment)
    }

    /// Add/update item
    pub fn set_item(&self, item: StatusBarItem) {
        let id = item.id.clone();
        let alignment = item.alignment;

        // Remove from old position if exists
        self.left_items.write().retain(|i| i.id != id);
        self.right_items.write().retain(|i| i.id != id);

        // Add to correct side
        match alignment {
            StatusBarAlignment::Left => {
                let mut items = self.left_items.write();
                let pos = items.iter().position(|i| i.priority > item.priority)
                    .unwrap_or(items.len());
                items.insert(pos, item.clone());
            }
            StatusBarAlignment::Right => {
                let mut items = self.right_items.write();
                let pos = items.iter().position(|i| i.priority > item.priority)
                    .unwrap_or(items.len());
                items.insert(pos, item.clone());
            }
        }

        self.items_by_id.write().insert(id.clone(), item);

        let _ = self.events.send(StatusBarEvent::ItemUpdated { id });
    }

    /// Remove item
    pub fn remove_item(&self, id: &str) {
        self.left_items.write().retain(|i| i.id != id);
        self.right_items.write().retain(|i| i.id != id);
        self.items_by_id.write().remove(id);

        let _ = self.events.send(StatusBarEvent::ItemRemoved { id: id.to_string() });
    }

    /// Get item by ID
    pub fn get_item(&self, id: &str) -> Option<StatusBarItem> {
        self.items_by_id.read().get(id).cloned()
    }

    /// Update item text
    pub fn update_text(&self, id: &str, text: impl Into<String>) {
        if let Some(mut item) = self.get_item(id) {
            item.text = text.into();
            self.set_item(item);
        }
    }

    /// Show/hide item
    pub fn set_visible(&self, id: &str, visible: bool) {
        if let Some(mut item) = self.get_item(id) {
            item.visible = visible;
            self.set_item(item);
        }
    }

    /// Get all left items
    pub fn left_items(&self) -> Vec<StatusBarItem> {
        self.left_items.read()
            .iter()
            .filter(|i| i.visible)
            .cloned()
            .collect()
    }

    /// Get all right items
    pub fn right_items(&self) -> Vec<StatusBarItem> {
        self.right_items.read()
            .iter()
            .filter(|i| i.visible)
            .cloned()
            .collect()
    }
}

impl Default for StatusBarService {
    fn default() -> Self {
        Self::new()
    }
}

/// Status bar item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusBarItem {
    /// Unique ID
    pub id: String,
    /// Alignment
    pub alignment: StatusBarAlignment,
    /// Priority (lower = closer to edge)
    pub priority: i32,
    /// Text content
    pub text: String,
    /// Tooltip
    pub tooltip: Option<String>,
    /// Icon (before text)
    pub icon: Option<String>,
    /// Command to execute on click
    pub command: Option<String>,
    /// Command arguments
    pub command_args: Vec<serde_json::Value>,
    /// Is visible
    pub visible: bool,
    /// Background color
    pub background_color: Option<String>,
    /// Foreground color  
    pub color: Option<String>,
    /// Accessibility label
    pub accessibility_label: Option<String>,
}

impl StatusBarItem {
    /// Create new item
    pub fn new(id: impl Into<String>, alignment: StatusBarAlignment) -> Self {
        Self {
            id: id.into(),
            alignment,
            priority: 0,
            text: String::new(),
            tooltip: None,
            icon: None,
            command: None,
            command_args: Vec::new(),
            visible: true,
            background_color: None,
            color: None,
            accessibility_label: None,
        }
    }
}

/// Status bar item builder
pub struct StatusBarItemBuilder {
    item: StatusBarItem,
}

impl StatusBarItemBuilder {
    pub fn new(id: impl Into<String>, alignment: StatusBarAlignment) -> Self {
        Self {
            item: StatusBarItem::new(id, alignment),
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.item.text = text.into();
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.item.tooltip = Some(tooltip.into());
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.item.icon = Some(icon.into());
        self
    }

    pub fn command(mut self, command: impl Into<String>) -> Self {
        self.item.command = Some(command.into());
        self
    }

    pub fn command_with_args(mut self, command: impl Into<String>, args: Vec<serde_json::Value>) -> Self {
        self.item.command = Some(command.into());
        self.item.command_args = args;
        self
    }

    pub fn priority(mut self, priority: i32) -> Self {
        self.item.priority = priority;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.item.visible = visible;
        self
    }

    pub fn background_color(mut self, color: impl Into<String>) -> Self {
        self.item.background_color = Some(color.into());
        self
    }

    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.item.color = Some(color.into());
        self
    }

    pub fn build(self) -> StatusBarItem {
        self.item
    }
}

/// Status bar alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusBarAlignment {
    Left,
    Right,
}

/// Status bar event
#[derive(Debug, Clone)]
pub enum StatusBarEvent {
    ItemUpdated { id: String },
    ItemRemoved { id: String },
}

/// Built-in status items
pub mod builtin {
    use super::*;

    /// Create language mode item
    pub fn language_mode(language: &str) -> StatusBarItem {
        StatusBarItemBuilder::new("foxkit.languageMode", StatusBarAlignment::Right)
            .text(language)
            .tooltip(format!("Language: {}", language))
            .command("foxkit.changeLanguageMode")
            .priority(100)
            .build()
    }

    /// Create encoding item
    pub fn encoding(encoding: &str) -> StatusBarItem {
        StatusBarItemBuilder::new("foxkit.encoding", StatusBarAlignment::Right)
            .text(encoding)
            .tooltip(format!("Encoding: {}", encoding))
            .command("foxkit.changeEncoding")
            .priority(90)
            .build()
    }

    /// Create line ending item
    pub fn line_ending(ending: &str) -> StatusBarItem {
        StatusBarItemBuilder::new("foxkit.lineEnding", StatusBarAlignment::Right)
            .text(ending)
            .tooltip(format!("Line Ending: {}", ending))
            .command("foxkit.changeLineEnding")
            .priority(80)
            .build()
    }

    /// Create cursor position item
    pub fn cursor_position(line: u32, column: u32) -> StatusBarItem {
        StatusBarItemBuilder::new("foxkit.cursorPosition", StatusBarAlignment::Right)
            .text(format!("Ln {}, Col {}", line, column))
            .tooltip("Go to Line")
            .command("foxkit.goToLine")
            .priority(70)
            .build()
    }

    /// Create selection item
    pub fn selection(lines: usize, chars: usize) -> StatusBarItem {
        StatusBarItemBuilder::new("foxkit.selection", StatusBarAlignment::Right)
            .text(format!("{} selected ({} lines)", chars, lines))
            .priority(65)
            .visible(chars > 0)
            .build()
    }

    /// Create indentation item
    pub fn indentation(spaces: bool, size: u32) -> StatusBarItem {
        let text = if spaces {
            format!("Spaces: {}", size)
        } else {
            format!("Tab Size: {}", size)
        };

        StatusBarItemBuilder::new("foxkit.indentation", StatusBarAlignment::Right)
            .text(text)
            .tooltip("Select Indentation")
            .command("foxkit.changeIndentation")
            .priority(60)
            .build()
    }

    /// Create git branch item
    pub fn git_branch(branch: &str, is_dirty: bool) -> StatusBarItem {
        let icon = if is_dirty { "$(git-branch)*" } else { "$(git-branch)" };
        
        StatusBarItemBuilder::new("foxkit.gitBranch", StatusBarAlignment::Left)
            .icon(icon)
            .text(branch)
            .tooltip(format!("Git: {}", branch))
            .command("foxkit.git.checkout")
            .priority(100)
            .build()
    }

    /// Create sync status item
    pub fn sync_status(ahead: usize, behind: usize) -> StatusBarItem {
        let text = if ahead > 0 && behind > 0 {
            format!("{}↑ {}↓", ahead, behind)
        } else if ahead > 0 {
            format!("{}↑", ahead)
        } else if behind > 0 {
            format!("{}↓", behind)
        } else {
            "".to_string()
        };

        StatusBarItemBuilder::new("foxkit.syncStatus", StatusBarAlignment::Left)
            .text(text)
            .tooltip("Sync Changes")
            .command("foxkit.git.sync")
            .priority(90)
            .visible(ahead > 0 || behind > 0)
            .build()
    }

    /// Create problems item
    pub fn problems(errors: usize, warnings: usize) -> StatusBarItem {
        let text = format!("$(error) {} $(warning) {}", errors, warnings);
        
        StatusBarItemBuilder::new("foxkit.problems", StatusBarAlignment::Left)
            .text(text)
            .tooltip("Problems")
            .command("foxkit.showProblems")
            .priority(50)
            .build()
    }

    /// Create notification item
    pub fn notifications(count: usize) -> StatusBarItem {
        StatusBarItemBuilder::new("foxkit.notifications", StatusBarAlignment::Right)
            .icon("$(bell)")
            .text(if count > 0 { count.to_string() } else { String::new() })
            .tooltip("Notifications")
            .command("foxkit.showNotifications")
            .priority(200)
            .build()
    }

    /// Create feedback item
    pub fn feedback() -> StatusBarItem {
        StatusBarItemBuilder::new("foxkit.feedback", StatusBarAlignment::Right)
            .icon("$(feedback)")
            .tooltip("Send Feedback")
            .command("foxkit.sendFeedback")
            .priority(300)
            .build()
    }
}
