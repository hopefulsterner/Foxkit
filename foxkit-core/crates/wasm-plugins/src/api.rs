//! Plugin API for extending Foxkit

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Plugin API version
pub const API_VERSION: &str = "0.1.0";

/// Plugin API trait
pub trait PluginApi {
    /// Get API version
    fn api_version(&self) -> &str {
        API_VERSION
    }

    /// Register a command
    fn register_command(&self, id: &str, handler: CommandHandler);

    /// Unregister a command
    fn unregister_command(&self, id: &str);

    /// Show information message
    fn show_info(&self, message: &str);

    /// Show warning message
    fn show_warning(&self, message: &str);

    /// Show error message
    fn show_error(&self, message: &str);

    /// Get workspace folders
    fn get_workspace_folders(&self) -> Vec<String>;

    /// Get active text editor
    fn get_active_editor(&self) -> Option<TextEditor>;

    /// Open a file
    fn open_file(&self, path: &str) -> Result<TextEditor, ApiError>;

    /// Create output channel
    fn create_output_channel(&self, name: &str) -> OutputChannel;

    /// Register tree data provider
    fn register_tree_provider(&self, view_id: &str, provider: Box<dyn TreeDataProvider>);

    /// Get configuration
    fn get_configuration(&self, section: &str) -> Configuration;

    /// Register completion provider
    fn register_completion_provider(&self, selector: DocumentSelector, provider: Box<dyn CompletionProvider>);

    /// Register hover provider
    fn register_hover_provider(&self, selector: DocumentSelector, provider: Box<dyn HoverProvider>);
}

/// Command handler function type
pub type CommandHandler = Box<dyn Fn(&[serde_json::Value]) -> Result<serde_json::Value, ApiError> + Send + Sync>;

/// API error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiError {
    NotFound(String),
    PermissionDenied(String),
    InvalidArgument(String),
    Internal(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            Self::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Text editor representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEditor {
    /// Document URI
    pub uri: String,
    /// Language ID
    pub language_id: String,
    /// Current selection
    pub selection: Selection,
    /// Visible ranges
    pub visible_ranges: Vec<Range>,
}

impl TextEditor {
    /// Get document text
    pub fn get_text(&self) -> String {
        // Would retrieve from editor service
        String::new()
    }

    /// Get text in range
    pub fn get_text_in_range(&self, range: &Range) -> String {
        String::new()
    }

    /// Insert text at position
    pub fn insert(&self, _position: Position, _text: &str) -> Result<(), ApiError> {
        Ok(())
    }

    /// Replace text in range
    pub fn replace(&self, _range: &Range, _text: &str) -> Result<(), ApiError> {
        Ok(())
    }

    /// Delete text in range
    pub fn delete(&self, _range: &Range) -> Result<(), ApiError> {
        Ok(())
    }
}

/// Selection in editor
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Selection {
    pub anchor: Position,
    pub active: Position,
}

impl Selection {
    pub fn is_empty(&self) -> bool {
        self.anchor.line == self.active.line && self.anchor.character == self.active.character
    }
}

/// Position in document
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Range in document
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Output channel for plugin output
#[derive(Debug)]
pub struct OutputChannel {
    name: String,
}

impl OutputChannel {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }

    pub fn append(&self, text: &str) {
        tracing::debug!(channel = %self.name, "{}", text);
    }

    pub fn append_line(&self, text: &str) {
        tracing::debug!(channel = %self.name, "{}", text);
    }

    pub fn clear(&self) {}

    pub fn show(&self) {}

    pub fn hide(&self) {}
}

/// Tree data provider for views
pub trait TreeDataProvider: Send + Sync {
    /// Get tree item for element
    fn get_tree_item(&self, element: &str) -> TreeItem;

    /// Get children of element (None for root)
    fn get_children(&self, element: Option<&str>) -> Vec<String>;

    /// Get parent of element
    fn get_parent(&self, element: &str) -> Option<String>;
}

/// Tree item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeItem {
    pub label: String,
    pub description: Option<String>,
    pub tooltip: Option<String>,
    pub icon: Option<String>,
    pub collapsible_state: CollapsibleState,
    pub command: Option<String>,
}

/// Collapsible state
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CollapsibleState {
    None,
    Collapsed,
    Expanded,
}

/// Document selector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSelector {
    pub language: Option<String>,
    pub scheme: Option<String>,
    pub pattern: Option<String>,
}

/// Configuration access
#[derive(Debug)]
pub struct Configuration {
    section: String,
}

impl Configuration {
    pub fn new(section: &str) -> Self {
        Self { section: section.to_string() }
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        None
    }

    pub fn has(&self, key: &str) -> bool {
        false
    }

    pub fn update(&self, key: &str, value: serde_json::Value) -> Result<(), ApiError> {
        Ok(())
    }
}

/// Completion provider trait
pub trait CompletionProvider: Send + Sync {
    fn provide_completions(&self, document: &str, position: Position) -> Vec<CompletionItem>;
}

/// Completion item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
    pub filter_text: Option<String>,
}

/// Completion item kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompletionItemKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

/// Hover provider trait
pub trait HoverProvider: Send + Sync {
    fn provide_hover(&self, document: &str, position: Position) -> Option<Hover>;
}

/// Hover content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hover {
    pub contents: Vec<MarkedString>,
    pub range: Option<Range>,
}

/// Marked string content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MarkedString {
    Plain(String),
    Code { language: String, value: String },
}
