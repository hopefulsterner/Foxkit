//! Sidebar Component
//!
//! The sidebar panel that shows Explorer, Search, Git, etc.

use crate::theme::Theme;
use crate::components::activity_bar::{ActivityId, Color};
use crate::components::tree_view::{TreeView, TreeNode};

/// Sidebar state
#[derive(Debug, Clone)]
pub struct SidebarState {
    /// Which view is currently shown
    pub active_view: ActivityId,
    /// Explorer state
    pub explorer: ExplorerState,
    /// Search state
    pub search: SearchState,
    /// Source control state
    pub source_control: SourceControlState,
    /// Debug state
    pub debug: DebugState,
    /// Extensions state
    pub extensions: ExtensionsState,
}

impl SidebarState {
    pub fn new() -> Self {
        Self {
            active_view: ActivityId::Explorer,
            explorer: ExplorerState::new(),
            search: SearchState::new(),
            source_control: SourceControlState::new(),
            debug: DebugState::new(),
            extensions: ExtensionsState::new(),
        }
    }

    /// Set active view
    pub fn set_active_view(&mut self, view: ActivityId) {
        self.active_view = view;
    }

    /// Render the sidebar
    pub fn render(&self, theme: &Theme) -> SidebarView {
        let content = match self.active_view {
            ActivityId::Explorer => SidebarContent::Explorer(self.explorer.render(theme)),
            ActivityId::Search => SidebarContent::Search(self.search.render(theme)),
            ActivityId::SourceControl => SidebarContent::SourceControl(self.source_control.render(theme)),
            ActivityId::Debug => SidebarContent::Debug(self.debug.render(theme)),
            ActivityId::Extensions => SidebarContent::Extensions(self.extensions.render(theme)),
            ActivityId::AiAgent => SidebarContent::AiAgent(AiAgentSidebarView::default()),
            ActivityId::Custom(_) => SidebarContent::Custom,
        };

        SidebarView {
            title: self.get_title(),
            content,
            background: theme.colors.sidebar_bg,
            border_color: theme.colors.sidebar_border,
            header_bg: theme.colors.sidebar_header_bg,
            header_fg: theme.colors.sidebar_header_fg,
        }
    }

    fn get_title(&self) -> &'static str {
        match self.active_view {
            ActivityId::Explorer => "EXPLORER",
            ActivityId::Search => "SEARCH",
            ActivityId::SourceControl => "SOURCE CONTROL",
            ActivityId::Debug => "RUN AND DEBUG",
            ActivityId::Extensions => "EXTENSIONS",
            ActivityId::AiAgent => "AI AGENT",
            ActivityId::Custom(_) => "CUSTOM",
        }
    }
}

impl Default for SidebarState {
    fn default() -> Self {
        Self::new()
    }
}

/// Rendered sidebar view
#[derive(Debug, Clone)]
pub struct SidebarView {
    pub title: &'static str,
    pub content: SidebarContent,
    pub background: Color,
    pub border_color: Color,
    pub header_bg: Color,
    pub header_fg: Color,
}

/// Sidebar content variants
#[derive(Debug, Clone)]
pub enum SidebarContent {
    Explorer(ExplorerView),
    Search(SearchView),
    SourceControl(SourceControlView),
    Debug(DebugView),
    Extensions(ExtensionsView),
    AiAgent(AiAgentSidebarView),
    Custom,
}

// ============ Explorer ============

#[derive(Debug, Clone)]
pub struct ExplorerState {
    /// Open folders section
    pub folders: Vec<FolderEntry>,
    /// Outline section collapsed
    pub outline_collapsed: bool,
    /// Timeline section collapsed
    pub timeline_collapsed: bool,
}

impl ExplorerState {
    pub fn new() -> Self {
        Self {
            folders: vec![],
            outline_collapsed: true,
            timeline_collapsed: true,
        }
    }

    pub fn add_folder(&mut self, name: String, path: String) {
        self.folders.push(FolderEntry {
            name,
            path,
            expanded: true,
            children: vec![],
        });
    }

    pub fn render(&self, theme: &Theme) -> ExplorerView {
        ExplorerView {
            sections: vec![
                ExplorerSection {
                    title: "OPEN EDITORS",
                    collapsed: false,
                    items: vec![],
                },
                ExplorerSection {
                    title: if self.folders.is_empty() { "NO FOLDER OPENED" } else { "FOLDERS" },
                    collapsed: false,
                    items: self.folders.iter().map(|f| {
                        TreeNode {
                            label: f.name.clone(),
                            icon: Some("folder"),
                            expanded: f.expanded,
                            children: f.children.clone(),
                            data: Some(f.path.clone()),
                        }
                    }).collect(),
                },
                ExplorerSection {
                    title: "OUTLINE",
                    collapsed: self.outline_collapsed,
                    items: vec![],
                },
                ExplorerSection {
                    title: "TIMELINE",
                    collapsed: self.timeline_collapsed,
                    items: vec![],
                },
            ],
            colors: ExplorerColors {
                section_header_fg: theme.colors.sidebar_section_header_fg,
                item_fg: theme.colors.sidebar_fg,
                item_hover_bg: theme.colors.list_hover_bg,
                item_selected_bg: theme.colors.list_active_bg,
                item_selected_fg: theme.colors.list_active_fg,
            },
        }
    }
}

impl Default for ExplorerState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct FolderEntry {
    pub name: String,
    pub path: String,
    pub expanded: bool,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone)]
pub struct ExplorerView {
    pub sections: Vec<ExplorerSection>,
    pub colors: ExplorerColors,
}

#[derive(Debug, Clone)]
pub struct ExplorerSection {
    pub title: &'static str,
    pub collapsed: bool,
    pub items: Vec<TreeNode>,
}

#[derive(Debug, Clone, Copy)]
pub struct ExplorerColors {
    pub section_header_fg: Color,
    pub item_fg: Color,
    pub item_hover_bg: Color,
    pub item_selected_bg: Color,
    pub item_selected_fg: Color,
}

// ============ Search ============

#[derive(Debug, Clone)]
pub struct SearchState {
    pub query: String,
    pub replace: String,
    pub show_replace: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub results: Vec<SearchResult>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            replace: String::new(),
            show_replace: false,
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
            results: vec![],
        }
    }

    pub fn render(&self, _theme: &Theme) -> SearchView {
        SearchView {
            query: self.query.clone(),
            replace: self.replace.clone(),
            show_replace: self.show_replace,
            case_sensitive: self.case_sensitive,
            whole_word: self.whole_word,
            use_regex: self.use_regex,
            results: self.results.clone(),
            result_count: self.results.iter().map(|r| r.matches.len()).sum(),
            file_count: self.results.len(),
        }
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SearchView {
    pub query: String,
    pub replace: String,
    pub show_replace: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub results: Vec<SearchResult>,
    pub result_count: usize,
    pub file_count: usize,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file_path: String,
    pub file_name: String,
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub line: u32,
    pub column: u32,
    pub preview: String,
    pub match_start: usize,
    pub match_end: usize,
}

// ============ Source Control ============

#[derive(Debug, Clone)]
pub struct SourceControlState {
    pub branch: String,
    pub staged_changes: Vec<GitChange>,
    pub changes: Vec<GitChange>,
    pub commit_message: String,
}

impl SourceControlState {
    pub fn new() -> Self {
        Self {
            branch: String::from("main"),
            staged_changes: vec![],
            changes: vec![],
            commit_message: String::new(),
        }
    }

    pub fn render(&self, _theme: &Theme) -> SourceControlView {
        SourceControlView {
            branch: self.branch.clone(),
            staged_changes: self.staged_changes.clone(),
            changes: self.changes.clone(),
            commit_message: self.commit_message.clone(),
        }
    }
}

impl Default for SourceControlState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SourceControlView {
    pub branch: String,
    pub staged_changes: Vec<GitChange>,
    pub changes: Vec<GitChange>,
    pub commit_message: String,
}

#[derive(Debug, Clone)]
pub struct GitChange {
    pub path: String,
    pub status: GitStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
}

// ============ Debug ============

#[derive(Debug, Clone)]
pub struct DebugState {
    pub configurations: Vec<DebugConfiguration>,
    pub selected_config: Option<usize>,
    pub is_running: bool,
}

impl DebugState {
    pub fn new() -> Self {
        Self {
            configurations: vec![],
            selected_config: None,
            is_running: false,
        }
    }

    pub fn render(&self, _theme: &Theme) -> DebugView {
        DebugView {
            configurations: self.configurations.clone(),
            selected_config: self.selected_config,
            is_running: self.is_running,
            variables: vec![],
            call_stack: vec![],
            breakpoints: vec![],
        }
    }
}

impl Default for DebugState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DebugConfiguration {
    pub name: String,
    pub config_type: String,
}

#[derive(Debug, Clone)]
pub struct DebugView {
    pub configurations: Vec<DebugConfiguration>,
    pub selected_config: Option<usize>,
    pub is_running: bool,
    pub variables: Vec<DebugVariable>,
    pub call_stack: Vec<StackFrame>,
    pub breakpoints: Vec<Breakpoint>,
}

#[derive(Debug, Clone)]
pub struct DebugVariable {
    pub name: String,
    pub value: String,
    pub var_type: String,
}

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub name: String,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub file: String,
    pub line: u32,
    pub enabled: bool,
}

// ============ Extensions ============

#[derive(Debug, Clone)]
pub struct ExtensionsState {
    pub search_query: String,
    pub installed: Vec<Extension>,
    pub recommended: Vec<Extension>,
}

impl ExtensionsState {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            installed: vec![],
            recommended: vec![],
        }
    }

    pub fn render(&self, _theme: &Theme) -> ExtensionsView {
        ExtensionsView {
            search_query: self.search_query.clone(),
            installed: self.installed.clone(),
            recommended: self.recommended.clone(),
        }
    }
}

impl Default for ExtensionsState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ExtensionsView {
    pub search_query: String,
    pub installed: Vec<Extension>,
    pub recommended: Vec<Extension>,
}

#[derive(Debug, Clone)]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub description: String,
    pub version: String,
    pub installed: bool,
    pub enabled: bool,
}

// ============ AI Agent Sidebar ============

#[derive(Debug, Clone, Default)]
pub struct AiAgentSidebarView {
    pub conversations: Vec<ConversationPreview>,
    pub active_conversation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConversationPreview {
    pub id: String,
    pub title: String,
    pub last_message: String,
    pub timestamp: String,
}
