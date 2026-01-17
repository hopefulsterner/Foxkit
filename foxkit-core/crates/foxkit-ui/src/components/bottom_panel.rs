//! Bottom Panel Component
//!
//! Bottom panel containing Terminal, Problems, Output, Debug Console, and AI Chat.

use crate::theme::Theme;
use crate::components::tabs::{TabBar, Tab, TabBarView, TabBarColors, TabView};
use crate::components::activity_bar::Color;

/// Bottom panel state
#[derive(Debug, Clone)]
pub struct BottomPanelState {
    /// Tab bar for panel views
    pub tabs: TabBar,
    /// Terminal instances
    pub terminals: Vec<TerminalInstance>,
    /// Active terminal index
    pub active_terminal: usize,
    /// Problems list
    pub problems: ProblemsState,
    /// Output channels
    pub output: OutputState,
    /// Debug console
    pub debug_console: DebugConsoleState,
    /// AI Chat
    pub ai_chat: AiChatState,
    /// Is panel maximized
    pub maximized: bool,
}

impl BottomPanelState {
    pub fn new() -> Self {
        let mut tabs = TabBar::new();
        
        // Add default panel tabs
        tabs.add_tab(Tab::new("problems", "Problems").with_icon("warning"));
        tabs.add_tab(Tab::new("output", "Output").with_icon("output"));
        tabs.add_tab(Tab::new("debug-console", "Debug Console").with_icon("debug-console"));
        tabs.add_tab(Tab::new("terminal", "Terminal").with_icon("terminal"));
        tabs.add_tab(Tab::new("ai-chat", "AI Chat").with_icon("robot"));
        
        // Terminal is default
        tabs.set_active(3);

        Self {
            tabs,
            terminals: vec![TerminalInstance::new(0)],
            active_terminal: 0,
            problems: ProblemsState::new(),
            output: OutputState::new(),
            debug_console: DebugConsoleState::new(),
            ai_chat: AiChatState::new(),
            maximized: false,
        }
    }

    /// Create new terminal
    pub fn new_terminal(&mut self) {
        let id = self.terminals.len();
        self.terminals.push(TerminalInstance::new(id));
        self.active_terminal = id;
    }

    /// Close terminal
    pub fn close_terminal(&mut self, index: usize) {
        if self.terminals.len() > 1 && index < self.terminals.len() {
            self.terminals.remove(index);
            if self.active_terminal >= self.terminals.len() {
                self.active_terminal = self.terminals.len() - 1;
            }
        }
    }

    /// Get active panel ID
    pub fn active_panel(&self) -> Option<&str> {
        self.tabs.active_tab().map(|t| t.id.as_str())
    }

    /// Toggle maximize
    pub fn toggle_maximize(&mut self) {
        self.maximized = !self.maximized;
    }

    /// Render the bottom panel
    pub fn render(&self, theme: &Theme) -> BottomPanelView {
        let content = match self.active_panel() {
            Some("problems") => PanelContent::Problems(self.problems.render()),
            Some("output") => PanelContent::Output(self.output.render()),
            Some("debug-console") => PanelContent::DebugConsole(self.debug_console.render()),
            Some("terminal") => PanelContent::Terminal(
                self.terminals.get(self.active_terminal)
                    .map(|t| t.render())
                    .unwrap_or_default()
            ),
            Some("ai-chat") => PanelContent::AiChat(self.ai_chat.render(theme)),
            _ => PanelContent::Empty,
        };

        BottomPanelView {
            tabs: TabBarView {
                tabs: self.tabs.tabs.iter().enumerate().map(|(i, tab)| {
                    let badge = match tab.id.as_str() {
                        "problems" => {
                            let count = self.problems.error_count + self.problems.warning_count;
                            if count > 0 { Some(count.to_string()) } else { None }
                        }
                        _ => None,
                    };
                    
                    TabView {
                        title: if let Some(b) = badge {
                            format!("{} ({})", tab.title, b)
                        } else {
                            tab.title.clone()
                        },
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
                    background: theme.colors.panel_tab_bar_bg,
                    active_bg: theme.colors.panel_tab_active_bg,
                    active_fg: theme.colors.panel_tab_active_fg,
                    inactive_bg: theme.colors.panel_tab_inactive_bg,
                    inactive_fg: theme.colors.panel_tab_inactive_fg,
                    border: theme.colors.panel_border,
                    modified_dot: theme.colors.tab_modified_dot,
                },
            },
            content,
            terminal_tabs: self.terminals.iter().enumerate().map(|(i, t)| {
                TerminalTabView {
                    index: i,
                    name: t.name.clone(),
                    is_active: i == self.active_terminal,
                }
            }).collect(),
            maximized: self.maximized,
            colors: BottomPanelColors {
                background: theme.colors.panel_bg,
                border: theme.colors.panel_border,
                header_bg: theme.colors.panel_header_bg,
            },
        }
    }
}

impl Default for BottomPanelState {
    fn default() -> Self {
        Self::new()
    }
}

// ============ Terminal ============

#[derive(Debug, Clone)]
pub struct TerminalInstance {
    pub id: usize,
    pub name: String,
    pub cwd: String,
    pub lines: Vec<TerminalLine>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_offset: usize,
}

impl TerminalInstance {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            name: format!("bash #{}", id + 1),
            cwd: String::from("~"),
            lines: vec![
                TerminalLine {
                    text: format!("Welcome to Foxkit Terminal"),
                    is_input: false,
                },
                TerminalLine {
                    text: format!("$ "),
                    is_input: true,
                },
            ],
            cursor_x: 2,
            cursor_y: 1,
            scroll_offset: 0,
        }
    }

    pub fn render(&self) -> TerminalView {
        TerminalView {
            lines: self.lines.clone(),
            cursor_x: self.cursor_x,
            cursor_y: self.cursor_y,
            scroll_offset: self.scroll_offset,
            cwd: self.cwd.clone(),
            colors: TerminalColors::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalLine {
    pub text: String,
    pub is_input: bool,
}

#[derive(Debug, Clone)]
pub struct TerminalView {
    pub lines: Vec<TerminalLine>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_offset: usize,
    pub cwd: String,
    pub colors: TerminalColors,
}

impl Default for TerminalView {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            cursor_x: 0,
            cursor_y: 0,
            scroll_offset: 0,
            cwd: String::new(),
            colors: TerminalColors::default(),
        }
    }
}

/// Colors for terminal rendering
#[derive(Debug, Clone, Copy)]
pub struct TerminalColors {
    pub background: Color,
    pub foreground: Color,
    pub cursor: Color,
}

impl Default for TerminalColors {
    fn default() -> Self {
        Self {
            background: Color::hex(0x1E1E1E),
            foreground: Color::hex(0xCCCCCC),
            cursor: Color::hex(0xFFFFFF),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalTabView {
    pub index: usize,
    pub name: String,
    pub is_active: bool,
}

// ============ Problems ============

#[derive(Debug, Clone)]
pub struct ProblemsState {
    pub items: Vec<ProblemItem>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub filter: ProblemsFilter,
}

impl ProblemsState {
    pub fn new() -> Self {
        Self {
            items: vec![],
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            filter: ProblemsFilter::All,
        }
    }

    pub fn render(&self) -> ProblemsView {
        ProblemsView {
            items: self.items.clone(),
            error_count: self.error_count,
            warning_count: self.warning_count,
            info_count: self.info_count,
            filter: self.filter,
        }
    }
}

impl Default for ProblemsState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ProblemItem {
    pub severity: ProblemSeverity,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub source: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemsFilter {
    All,
    Errors,
    Warnings,
}

#[derive(Debug, Clone)]
pub struct ProblemsView {
    pub items: Vec<ProblemItem>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub filter: ProblemsFilter,
}

// ============ Output ============

#[derive(Debug, Clone)]
pub struct OutputState {
    pub channels: Vec<OutputChannel>,
    pub active_channel: usize,
}

impl OutputState {
    pub fn new() -> Self {
        Self {
            channels: vec![
                OutputChannel {
                    name: String::from("Log"),
                    lines: vec![],
                },
            ],
            active_channel: 0,
        }
    }

    pub fn render(&self) -> OutputView {
        OutputView {
            channels: self.channels.iter().map(|c| c.name.clone()).collect(),
            active_channel: self.active_channel,
            lines: self.channels.get(self.active_channel)
                .map(|c| c.lines.clone())
                .unwrap_or_default(),
        }
    }
}

impl Default for OutputState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct OutputChannel {
    pub name: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OutputView {
    pub channels: Vec<String>,
    pub active_channel: usize,
    pub lines: Vec<String>,
}

// ============ Debug Console ============

#[derive(Debug, Clone)]
pub struct DebugConsoleState {
    pub entries: Vec<DebugConsoleEntry>,
    pub input: String,
}

impl DebugConsoleState {
    pub fn new() -> Self {
        Self {
            entries: vec![],
            input: String::new(),
        }
    }

    pub fn render(&self) -> DebugConsoleView {
        DebugConsoleView {
            entries: self.entries.clone(),
            input: self.input.clone(),
        }
    }
}

impl Default for DebugConsoleState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DebugConsoleEntry {
    pub kind: DebugConsoleEntryKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugConsoleEntryKind {
    Input,
    Output,
    Error,
    Info,
}

#[derive(Debug, Clone)]
pub struct DebugConsoleView {
    pub entries: Vec<DebugConsoleEntry>,
    pub input: String,
}

// ============ AI Chat ============

#[derive(Debug, Clone)]
pub struct AiChatState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub is_loading: bool,
    pub model: String,
    pub context_files: Vec<String>,
}

impl AiChatState {
    pub fn new() -> Self {
        Self {
            messages: vec![
                ChatMessage {
                    role: ChatRole::Assistant,
                    content: "Hello! I'm your AI coding assistant. How can I help you today?".into(),
                    timestamp: "Just now".into(),
                },
            ],
            input: String::new(),
            is_loading: false,
            model: String::from("gpt-4"),
            context_files: vec![],
        }
    }

    pub fn send_message(&mut self, content: String) {
        self.messages.push(ChatMessage {
            role: ChatRole::User,
            content,
            timestamp: "Just now".into(),
        });
        self.input.clear();
        self.is_loading = true;
    }

    pub fn receive_response(&mut self, content: String) {
        self.messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content,
            timestamp: "Just now".into(),
        });
        self.is_loading = false;
    }

    pub fn render(&self, theme: &Theme) -> AiChatView {
        AiChatView {
            messages: self.messages.iter().map(|m| ChatMessageView {
                role: m.role,
                content: m.content.clone(),
                timestamp: m.timestamp.clone(),
                colors: match m.role {
                    ChatRole::User => ChatMessageColors {
                        background: theme.colors.chat_user_bg,
                        foreground: theme.colors.chat_user_fg,
                    },
                    ChatRole::Assistant => ChatMessageColors {
                        background: theme.colors.chat_assistant_bg,
                        foreground: theme.colors.chat_assistant_fg,
                    },
                    ChatRole::System => ChatMessageColors {
                        background: theme.colors.chat_system_bg,
                        foreground: theme.colors.chat_system_fg,
                    },
                },
            }).collect(),
            input: self.input.clone(),
            is_loading: self.is_loading,
            model: self.model.clone(),
            context_files: self.context_files.clone(),
            colors: AiChatColors {
                background: theme.colors.panel_bg,
                foreground: theme.colors.panel_fg,
                input_bg: theme.colors.input_bg,
                input_border: theme.colors.input_border,
                input_foreground: theme.colors.input_fg,
                placeholder_foreground: theme.colors.input_placeholder_fg,
                user_bubble_bg: theme.colors.ai_bubble_user_bg,
                assistant_bubble_bg: theme.colors.ai_bubble_assistant_bg,
            },
        }
    }
}

impl Default for AiChatState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone)]
pub struct AiChatView {
    pub messages: Vec<ChatMessageView>,
    pub input: String,
    pub is_loading: bool,
    pub model: String,
    pub context_files: Vec<String>,
    pub colors: AiChatColors,
}

#[derive(Debug, Clone)]
pub struct ChatMessageView {
    pub role: ChatRole,
    pub content: String,
    pub timestamp: String,
    pub colors: ChatMessageColors,
}

#[derive(Debug, Clone, Copy)]
pub struct ChatMessageColors {
    pub background: Color,
    pub foreground: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct AiChatColors {
    pub background: Color,
    pub foreground: Color,
    pub input_bg: Color,
    pub input_border: Color,
    pub input_foreground: Color,
    pub placeholder_foreground: Color,
    pub user_bubble_bg: Color,
    pub assistant_bubble_bg: Color,
}

// ============ Panel View ============

#[derive(Debug, Clone)]
pub enum PanelContent {
    Problems(ProblemsView),
    Output(OutputView),
    DebugConsole(DebugConsoleView),
    Terminal(TerminalView),
    AiChat(AiChatView),
    Empty,
}

#[derive(Debug, Clone)]
pub struct BottomPanelView {
    pub tabs: TabBarView,
    pub content: PanelContent,
    pub terminal_tabs: Vec<TerminalTabView>,
    pub maximized: bool,
    pub colors: BottomPanelColors,
}

#[derive(Debug, Clone, Copy)]
pub struct BottomPanelColors {
    pub background: Color,
    pub border: Color,
    pub header_bg: Color,
}
