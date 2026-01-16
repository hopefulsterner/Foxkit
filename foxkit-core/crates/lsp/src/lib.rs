//! # Foxkit LSP
//!
//! Language Server Protocol client implementation.
//! Manages language servers for code intelligence.

pub mod capabilities;
pub mod client;
pub mod manager;
pub mod process;
pub mod requests;
pub mod transport;
pub mod workspace;

pub use capabilities::{build_client_capabilities, ServerCapabilityAnalyzer};
pub use requests::{LspRequestBuilder, LspNotificationBuilder, file_uri, pos, range};
pub use workspace::{WorkspaceManager, TextDocument, WorkspaceConfiguration, LanguageConfiguration};

use std::collections::HashMap;

pub use lsp_types::*;
pub use client::LspClient;
pub use manager::LspManager;

/// Language server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Language ID (e.g., "rust", "typescript")
    pub language_id: String,
    /// Server name
    pub name: String,
    /// Command to start server
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// File patterns this server handles
    pub file_patterns: Vec<String>,
    /// Initialization options
    pub initialization_options: Option<serde_json::Value>,
    /// Root patterns (for finding project root)
    pub root_patterns: Vec<String>,
}

impl ServerConfig {
    pub fn new(language_id: &str, name: &str, command: &str) -> Self {
        Self {
            language_id: language_id.to_string(),
            name: name.to_string(),
            command: command.to_string(),
            args: Vec::new(),
            env: HashMap::new(),
            file_patterns: Vec::new(),
            initialization_options: None,
            root_patterns: vec![".git".to_string()],
        }
    }

    pub fn with_args(mut self, args: Vec<&str>) -> Self {
        self.args = args.into_iter().map(String::from).collect();
        self
    }

    pub fn with_patterns(mut self, patterns: Vec<&str>) -> Self {
        self.file_patterns = patterns.into_iter().map(String::from).collect();
        self
    }

    pub fn with_root_patterns(mut self, patterns: Vec<&str>) -> Self {
        self.root_patterns = patterns.into_iter().map(String::from).collect();
        self
    }
}

/// Built-in server configurations
pub mod servers {
    use super::*;

    pub fn rust_analyzer() -> ServerConfig {
        ServerConfig::new("rust", "rust-analyzer", "rust-analyzer")
            .with_patterns(vec!["*.rs"])
            .with_root_patterns(vec!["Cargo.toml", "Cargo.lock"])
    }

    pub fn typescript_language_server() -> ServerConfig {
        ServerConfig::new("typescript", "typescript-language-server", "typescript-language-server")
            .with_args(vec!["--stdio"])
            .with_patterns(vec!["*.ts", "*.tsx", "*.js", "*.jsx"])
            .with_root_patterns(vec!["package.json", "tsconfig.json"])
    }

    pub fn pylsp() -> ServerConfig {
        ServerConfig::new("python", "pylsp", "pylsp")
            .with_patterns(vec!["*.py"])
            .with_root_patterns(vec!["pyproject.toml", "setup.py", "requirements.txt"])
    }

    pub fn gopls() -> ServerConfig {
        ServerConfig::new("go", "gopls", "gopls")
            .with_patterns(vec!["*.go"])
            .with_root_patterns(vec!["go.mod", "go.sum"])
    }

    pub fn clangd() -> ServerConfig {
        ServerConfig::new("c", "clangd", "clangd")
            .with_patterns(vec!["*.c", "*.cpp", "*.h", "*.hpp", "*.cc", "*.cxx"])
            .with_root_patterns(vec!["compile_commands.json", "CMakeLists.txt"])
    }

    pub fn all() -> Vec<ServerConfig> {
        vec![
            rust_analyzer(),
            typescript_language_server(),
            pylsp(),
            gopls(),
            clangd(),
        ]
    }
}

/// LSP request/response types
#[derive(Debug)]
pub struct PendingRequest {
    pub id: i32,
    pub method: String,
    pub sent_at: std::time::Instant,
}

/// Server state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerState {
    /// Not started
    Stopped,
    /// Starting up
    Starting,
    /// Initialized and ready
    Running,
    /// Shutting down
    ShuttingDown,
    /// Crashed or failed
    Failed,
}

/// Diagnostic with source info
#[derive(Debug, Clone)]
pub struct FileDiagnostic {
    pub uri: Url,
    pub diagnostics: Vec<Diagnostic>,
    pub version: Option<i32>,
}

/// Code action with command
#[derive(Debug, Clone)]
pub enum CodeActionOrCommand {
    Action(CodeAction),
    Command(Command),
}

/// Completion with documentation
#[derive(Debug, Clone)]
pub struct ResolvedCompletion {
    pub item: CompletionItem,
    pub documentation: Option<Documentation>,
}

/// LSP event
#[derive(Debug, Clone)]
pub enum LspEvent {
    /// Server started
    ServerStarted { language_id: String },
    /// Server stopped
    ServerStopped { language_id: String },
    /// Server crashed
    ServerCrashed { language_id: String, error: String },
    /// Diagnostics published
    DiagnosticsPublished { uri: Url, diagnostics: Vec<Diagnostic> },
    /// Progress update
    Progress { token: ProgressToken, value: ProgressParams },
    /// Log message
    LogMessage { level: MessageType, message: String },
    /// Show message
    ShowMessage { level: MessageType, message: String },
}
