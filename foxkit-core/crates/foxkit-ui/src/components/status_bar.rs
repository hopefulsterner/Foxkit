//! Status Bar Component
//!
//! Bottom status bar showing editor info, git status, notifications, etc.

use crate::theme::Theme;
use crate::components::activity_bar::Color;

/// Status bar state
#[derive(Debug, Clone)]
pub struct StatusBarState {
    /// Left side items
    pub left_items: Vec<StatusBarItem>,
    /// Right side items
    pub right_items: Vec<StatusBarItem>,
    /// Remote indicator
    pub remote: Option<RemoteStatus>,
    /// Background mode (changes color based on state)
    pub mode: StatusBarMode,
}

impl StatusBarState {
    pub fn new() -> Self {
        Self {
            left_items: vec![
                StatusBarItem {
                    id: "git-branch".into(),
                    text: "main".into(),
                    icon: Some("git-branch"),
                    tooltip: Some("Git Branch".into()),
                    command: Some("git.checkout".into()),
                    priority: 100,
                },
                StatusBarItem {
                    id: "git-sync".into(),
                    text: "↑0 ↓0".into(),
                    icon: Some("sync"),
                    tooltip: Some("Synchronize Changes".into()),
                    command: Some("git.sync".into()),
                    priority: 99,
                },
                StatusBarItem {
                    id: "problems".into(),
                    text: "0".into(),
                    icon: Some("error"),
                    tooltip: Some("No Problems".into()),
                    command: Some("workbench.actions.view.problems".into()),
                    priority: 98,
                },
                StatusBarItem {
                    id: "warnings".into(),
                    text: "0".into(),
                    icon: Some("warning"),
                    tooltip: Some("No Warnings".into()),
                    command: Some("workbench.actions.view.problems".into()),
                    priority: 97,
                },
            ],
            right_items: vec![
                StatusBarItem {
                    id: "cursor-position".into(),
                    text: "Ln 1, Col 1".into(),
                    icon: None,
                    tooltip: Some("Go to Line".into()),
                    command: Some("workbench.action.gotoLine".into()),
                    priority: 100,
                },
                StatusBarItem {
                    id: "indentation".into(),
                    text: "Spaces: 4".into(),
                    icon: None,
                    tooltip: Some("Select Indentation".into()),
                    command: Some("changeEditorIndentation".into()),
                    priority: 99,
                },
                StatusBarItem {
                    id: "encoding".into(),
                    text: "UTF-8".into(),
                    icon: None,
                    tooltip: Some("Select Encoding".into()),
                    command: Some("workbench.action.editor.changeEncoding".into()),
                    priority: 98,
                },
                StatusBarItem {
                    id: "eol".into(),
                    text: "LF".into(),
                    icon: None,
                    tooltip: Some("Select End of Line Sequence".into()),
                    command: Some("workbench.action.editor.changeEOL".into()),
                    priority: 97,
                },
                StatusBarItem {
                    id: "language".into(),
                    text: "Rust".into(),
                    icon: None,
                    tooltip: Some("Select Language Mode".into()),
                    command: Some("workbench.action.editor.changeLanguageMode".into()),
                    priority: 96,
                },
                StatusBarItem {
                    id: "ai-status".into(),
                    text: "AI Ready".into(),
                    icon: Some("robot"),
                    tooltip: Some("AI Agent Status".into()),
                    command: Some("foxkit.ai.toggle".into()),
                    priority: 50,
                },
                StatusBarItem {
                    id: "notifications".into(),
                    text: "".into(),
                    icon: Some("bell"),
                    tooltip: Some("Notifications".into()),
                    command: Some("notifications.showList".into()),
                    priority: 10,
                },
            ],
            remote: None,
            mode: StatusBarMode::Normal,
        }
    }

    /// Update cursor position display
    pub fn set_cursor_position(&mut self, line: usize, column: usize) {
        if let Some(item) = self.right_items.iter_mut().find(|i| i.id == "cursor-position") {
            item.text = format!("Ln {}, Col {}", line, column);
        }
    }

    /// Update language display
    pub fn set_language(&mut self, language: &str) {
        if let Some(item) = self.right_items.iter_mut().find(|i| i.id == "language") {
            item.text = language.to_string();
        }
    }

    /// Update git branch
    pub fn set_git_branch(&mut self, branch: &str) {
        if let Some(item) = self.left_items.iter_mut().find(|i| i.id == "git-branch") {
            item.text = branch.to_string();
        }
    }

    /// Update problem counts
    pub fn set_problems(&mut self, errors: usize, warnings: usize) {
        if let Some(item) = self.left_items.iter_mut().find(|i| i.id == "problems") {
            item.text = errors.to_string();
            item.tooltip = Some(format!("{} Errors", errors));
        }
        if let Some(item) = self.left_items.iter_mut().find(|i| i.id == "warnings") {
            item.text = warnings.to_string();
            item.tooltip = Some(format!("{} Warnings", warnings));
        }
    }

    /// Set status bar mode
    pub fn set_mode(&mut self, mode: StatusBarMode) {
        self.mode = mode;
    }

    /// Set remote connection
    pub fn set_remote(&mut self, remote: Option<RemoteStatus>) {
        self.remote = remote;
    }

    /// Render the status bar
    pub fn render(&self, theme: &Theme) -> StatusBarView {
        let bg_color = match self.mode {
            StatusBarMode::Normal => theme.colors.status_bar_bg,
            StatusBarMode::Debugging => theme.colors.status_bar_debugging_bg,
            StatusBarMode::NoFolder => theme.colors.status_bar_no_folder_bg,
            StatusBarMode::Remote => theme.colors.status_bar_remote_bg,
        };

        StatusBarView {
            left_items: self.left_items.iter().map(|i| StatusBarItemView {
                text: i.text.clone(),
                icon: i.icon,
                tooltip: i.tooltip.clone(),
                has_command: i.command.is_some(),
            }).collect(),
            right_items: self.right_items.iter().map(|i| StatusBarItemView {
                text: i.text.clone(),
                icon: i.icon,
                tooltip: i.tooltip.clone(),
                has_command: i.command.is_some(),
            }).collect(),
            remote: self.remote.clone().map(|r| RemoteStatusView {
                name: r.name,
                icon: r.icon,
            }),
            colors: StatusBarColors {
                background: bg_color,
                foreground: theme.colors.status_bar_fg,
                item_hover_bg: theme.colors.status_bar_item_hover_bg,
                remote_bg: theme.colors.status_bar_remote_bg,
            },
        }
    }
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct StatusBarItem {
    pub id: String,
    pub text: String,
    pub icon: Option<&'static str>,
    pub tooltip: Option<String>,
    pub command: Option<String>,
    pub priority: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusBarMode {
    Normal,
    Debugging,
    NoFolder,
    Remote,
}

#[derive(Debug, Clone)]
pub struct RemoteStatus {
    pub name: String,
    pub icon: &'static str,
}

/// Rendered status bar view
#[derive(Debug, Clone)]
pub struct StatusBarView {
    pub left_items: Vec<StatusBarItemView>,
    pub right_items: Vec<StatusBarItemView>,
    pub remote: Option<RemoteStatusView>,
    pub colors: StatusBarColors,
}

#[derive(Debug, Clone)]
pub struct StatusBarItemView {
    pub text: String,
    pub icon: Option<&'static str>,
    pub tooltip: Option<String>,
    pub has_command: bool,
}

#[derive(Debug, Clone)]
pub struct RemoteStatusView {
    pub name: String,
    pub icon: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct StatusBarColors {
    pub background: Color,
    pub foreground: Color,
    pub item_hover_bg: Color,
    pub remote_bg: Color,
}
