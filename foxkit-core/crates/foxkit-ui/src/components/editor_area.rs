//! Editor Area Component
//!
//! Main editor area with tabs, split views, and the editor content.

use crate::theme::Theme;
use crate::components::tabs::{TabBar, Tab, TabBarView, TabBarColors, TabView};
use crate::components::activity_bar::Color;

/// Editor area state
#[derive(Debug, Clone)]
pub struct EditorAreaState {
    /// Editor groups (for split view)
    pub groups: Vec<EditorGroup>,
    /// Active group index
    pub active_group: usize,
    /// Split layout
    pub layout: EditorLayout,
}

impl EditorAreaState {
    pub fn new() -> Self {
        Self {
            groups: vec![EditorGroup::new()],
            active_group: 0,
            layout: EditorLayout::Single,
        }
    }

    /// Get active group
    pub fn active_group(&self) -> Option<&EditorGroup> {
        self.groups.get(self.active_group)
    }

    /// Get active group mutably
    pub fn active_group_mut(&mut self) -> Option<&mut EditorGroup> {
        self.groups.get_mut(self.active_group)
    }

    /// Open a file in the active group
    pub fn open_file(&mut self, file_path: String, file_name: String) {
        if let Some(group) = self.active_group_mut() {
            // Check if already open
            if let Some(idx) = group.tabs.find_by_id(&file_path) {
                group.tabs.set_active(idx);
                return;
            }

            // Add new tab
            let icon = Self::icon_for_file(&file_name);
            let tab = Tab::new(&file_path, &file_name)
                .with_icon(icon)
                .with_tooltip(file_path.clone());
            
            let idx = group.tabs.add_tab(tab);
            group.tabs.set_active(idx);
            
            // Create editor state for the file
            group.editors.push(EditorState {
                file_path,
                content: String::new(),
                cursor: CursorPosition::default(),
                scroll: ScrollPosition::default(),
                selections: vec![],
            });
        }
    }

    /// Close active tab in active group
    pub fn close_active_tab(&mut self) {
        if let Some(group) = self.active_group_mut() {
            if let Some(idx) = group.tabs.active_index {
                group.tabs.remove_tab(idx);
                if idx < group.editors.len() {
                    group.editors.remove(idx);
                }
            }
        }
    }

    /// Split editor horizontally
    pub fn split_horizontal(&mut self) {
        let new_group = EditorGroup::new();
        self.groups.push(new_group);
        self.active_group = self.groups.len() - 1;
        self.layout = EditorLayout::Horizontal(vec![0.5, 0.5]);
    }

    /// Split editor vertically
    pub fn split_vertical(&mut self) {
        let new_group = EditorGroup::new();
        self.groups.push(new_group);
        self.active_group = self.groups.len() - 1;
        self.layout = EditorLayout::Vertical(vec![0.5, 0.5]);
    }

    /// Get icon for file based on extension
    fn icon_for_file(name: &str) -> &'static str {
        let ext = name.rsplit('.').next().unwrap_or("");
        match ext.to_lowercase().as_str() {
            "rs" => "rust",
            "js" => "javascript",
            "ts" => "typescript",
            "tsx" => "react",
            "jsx" => "react",
            "py" => "python",
            "go" => "go",
            "java" => "java",
            "c" | "h" => "c",
            "cpp" | "cc" | "hpp" => "cpp",
            "json" => "json",
            "toml" => "toml",
            "yaml" | "yml" => "yaml",
            "md" => "markdown",
            "html" => "html",
            "css" => "css",
            "scss" | "sass" => "sass",
            "svg" => "svg",
            "png" | "jpg" | "jpeg" | "gif" => "image",
            _ => "file",
        }
    }

    /// Render the editor area
    pub fn render(&self, theme: &Theme) -> EditorAreaView {
        EditorAreaView {
            groups: self.groups.iter().enumerate().map(|(i, group)| {
                group.render(theme, i == self.active_group)
            }).collect(),
            layout: self.layout.clone(),
            active_group: self.active_group,
            colors: EditorAreaColors {
                background: theme.colors.editor_bg,
                gutter_bg: theme.colors.editor_gutter_bg,
                line_number_fg: theme.colors.editor_line_number_fg,
                current_line_bg: theme.colors.editor_current_line_bg,
                selection_bg: theme.colors.editor_selection_bg,
                cursor_color: theme.colors.editor_cursor,
            },
        }
    }
}

impl Default for EditorAreaState {
    fn default() -> Self {
        Self::new()
    }
}

/// A group of editors (one pane in split view)
#[derive(Debug, Clone)]
pub struct EditorGroup {
    pub tabs: TabBar,
    pub editors: Vec<EditorState>,
}

impl EditorGroup {
    pub fn new() -> Self {
        Self {
            tabs: TabBar::new(),
            editors: vec![],
        }
    }

    pub fn render(&self, theme: &Theme, is_active: bool) -> EditorGroupView {
        EditorGroupView {
            tabs: TabBarView {
                tabs: self.tabs.tabs.iter().enumerate().map(|(i, tab)| {
                    TabView {
                        title: tab.title.clone(),
                        icon: tab.icon,
                        modified: tab.modified,
                        pinned: tab.pinned,
                        preview: tab.preview,
                        is_active: self.tabs.active_index == Some(i),
                        tooltip: tab.tooltip.clone(),
                    }
                }).collect(),
                active_index: self.tabs.active_index,
                colors: TabBarColors {
                    background: theme.colors.tab_bar_bg,
                    active_bg: theme.colors.tab_active_bg,
                    active_fg: theme.colors.tab_active_fg,
                    inactive_bg: theme.colors.tab_inactive_bg,
                    inactive_fg: theme.colors.tab_inactive_fg,
                    border: theme.colors.tab_border,
                    modified_dot: theme.colors.tab_modified_dot,
                },
            },
            active_editor: self.tabs.active_index.and_then(|i| {
                self.editors.get(i).map(|e| e.render())
            }),
            is_active,
            is_empty: self.tabs.tabs.is_empty(),
        }
    }
}

impl Default for EditorGroup {
    fn default() -> Self {
        Self::new()
    }
}

/// Editor split layout
#[derive(Debug, Clone)]
pub enum EditorLayout {
    Single,
    Horizontal(Vec<f32>), // Split ratios
    Vertical(Vec<f32>),
    Grid { rows: usize, cols: usize },
}

/// Single editor state
#[derive(Debug, Clone)]
pub struct EditorState {
    pub file_path: String,
    pub content: String,
    pub cursor: CursorPosition,
    pub scroll: ScrollPosition,
    pub selections: Vec<Selection>,
}

impl EditorState {
    pub fn render(&self) -> EditorContentView {
        let lines: Vec<_> = self.content.lines().map(String::from).collect();
        let line_count = lines.len().max(1);
        
        EditorContentView {
            lines,
            line_count,
            cursor: self.cursor,
            scroll: self.scroll,
            selections: self.selections.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CursorPosition {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ScrollPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub start: CursorPosition,
    pub end: CursorPosition,
}

/// Rendered editor area view
#[derive(Debug, Clone)]
pub struct EditorAreaView {
    pub groups: Vec<EditorGroupView>,
    pub layout: EditorLayout,
    pub active_group: usize,
    pub colors: EditorAreaColors,
}

/// Rendered editor group view
#[derive(Debug, Clone)]
pub struct EditorGroupView {
    pub tabs: TabBarView,
    pub active_editor: Option<EditorContentView>,
    pub is_active: bool,
    pub is_empty: bool,
}

/// Rendered editor content view
#[derive(Debug, Clone)]
pub struct EditorContentView {
    pub lines: Vec<String>,
    pub line_count: usize,
    pub cursor: CursorPosition,
    pub scroll: ScrollPosition,
    pub selections: Vec<Selection>,
}

#[derive(Debug, Clone, Copy)]
pub struct EditorAreaColors {
    pub background: Color,
    pub gutter_bg: Color,
    pub line_number_fg: Color,
    pub current_line_bg: Color,
    pub selection_bg: Color,
    pub cursor_color: Color,
}
