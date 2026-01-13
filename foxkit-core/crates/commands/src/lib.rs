//! # Foxkit Commands
//!
//! Command palette and command registry.

pub mod palette;
pub mod handler;

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub use palette::{CommandPalette, PaletteItem};
pub use handler::CommandHandler;

/// Global command registry
pub static COMMANDS: Lazy<CommandRegistry> = Lazy::new(CommandRegistry::new);

/// Command ID type
pub type CommandId = &'static str;

/// Command execution result
pub type CommandResult = Result<Option<Box<dyn Any + Send>>, CommandError>;

/// Command error
#[derive(Debug, Clone, thiserror::Error)]
pub enum CommandError {
    #[error("Command not found: {0}")]
    NotFound(String),
    #[error("Command disabled: {0}")]
    Disabled(String),
    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

/// Command definition
#[derive(Debug, Clone)]
pub struct Command {
    /// Unique command ID
    pub id: String,
    /// Display title
    pub title: String,
    /// Category
    pub category: Option<String>,
    /// Icon
    pub icon: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Is command enabled
    pub enabled: bool,
    /// Is command visible in palette
    pub visible: bool,
}

impl Command {
    pub fn new(id: &str, title: &str) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            category: None,
            icon: None,
            description: None,
            enabled: true,
            visible: true,
        }
    }

    pub fn with_category(mut self, category: &str) -> Self {
        self.category = Some(category.to_string());
        self
    }

    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.to_string());
        self
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn hidden(mut self) -> Self {
        self.visible = false;
        self
    }

    /// Get full display title (category: title)
    pub fn full_title(&self) -> String {
        match &self.category {
            Some(cat) => format!("{}: {}", cat, self.title),
            None => self.title.clone(),
        }
    }
}

/// Command arguments
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandArgs {
    values: HashMap<String, serde_json::Value>,
}

impl CommandArgs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with<T: Serialize>(mut self, key: &str, value: T) -> Self {
        self.values.insert(key.to_string(), serde_json::to_value(value).unwrap());
        self
    }

    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.values.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get(key)
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key)
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key)
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Command handler function type
type HandlerFn = Arc<dyn Fn(CommandArgs) -> CommandResult + Send + Sync>;

/// Command registry
pub struct CommandRegistry {
    commands: RwLock<HashMap<String, Command>>,
    handlers: RwLock<HashMap<String, HandlerFn>>,
    history: RwLock<Vec<String>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let registry = Self {
            commands: RwLock::new(HashMap::new()),
            handlers: RwLock::new(HashMap::new()),
            history: RwLock::new(Vec::new()),
        };
        registry.register_builtin_commands();
        registry
    }

    /// Register a command
    pub fn register<F>(&self, command: Command, handler: F)
    where
        F: Fn(CommandArgs) -> CommandResult + Send + Sync + 'static,
    {
        let id = command.id.clone();
        self.commands.write().insert(id.clone(), command);
        self.handlers.write().insert(id, Arc::new(handler));
    }

    /// Execute command by ID
    pub fn execute(&self, id: &str, args: CommandArgs) -> CommandResult {
        let command = self.commands.read().get(id).cloned();
        
        match command {
            None => Err(CommandError::NotFound(id.to_string())),
            Some(cmd) if !cmd.enabled => Err(CommandError::Disabled(id.to_string())),
            Some(_) => {
                let handler = self.handlers.read().get(id).cloned();
                match handler {
                    Some(h) => {
                        self.history.write().push(id.to_string());
                        h(args)
                    }
                    None => Err(CommandError::NotFound(id.to_string())),
                }
            }
        }
    }

    /// Execute command (shorthand)
    pub fn run(&self, id: &str) -> CommandResult {
        self.execute(id, CommandArgs::new())
    }

    /// Get command by ID
    pub fn get(&self, id: &str) -> Option<Command> {
        self.commands.read().get(id).cloned()
    }

    /// Get all commands
    pub fn all(&self) -> Vec<Command> {
        self.commands.read().values().cloned().collect()
    }

    /// Get visible commands
    pub fn visible(&self) -> Vec<Command> {
        self.commands
            .read()
            .values()
            .filter(|c| c.visible)
            .cloned()
            .collect()
    }

    /// Search commands
    pub fn search(&self, query: &str) -> Vec<Command> {
        let query = query.to_lowercase();
        self.commands
            .read()
            .values()
            .filter(|c| {
                c.visible
                    && (c.title.to_lowercase().contains(&query)
                        || c.id.to_lowercase().contains(&query)
                        || c.category.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false))
            })
            .cloned()
            .collect()
    }

    /// Get command history
    pub fn history(&self) -> Vec<String> {
        self.history.read().clone()
    }

    /// Set command enabled state
    pub fn set_enabled(&self, id: &str, enabled: bool) {
        if let Some(cmd) = self.commands.write().get_mut(id) {
            cmd.enabled = enabled;
        }
    }

    fn register_builtin_commands(&self) {
        // File commands
        self.register(
            Command::new("workbench.action.files.newUntitledFile", "New File")
                .with_category("File"),
            |_| Ok(None),
        );
        self.register(
            Command::new("workbench.action.files.openFile", "Open File...")
                .with_category("File"),
            |_| Ok(None),
        );
        self.register(
            Command::new("workbench.action.files.save", "Save")
                .with_category("File"),
            |_| Ok(None),
        );
        self.register(
            Command::new("workbench.action.files.saveAll", "Save All")
                .with_category("File"),
            |_| Ok(None),
        );

        // Edit commands
        self.register(
            Command::new("editor.action.clipboardCutAction", "Cut")
                .with_category("Edit"),
            |_| Ok(None),
        );
        self.register(
            Command::new("editor.action.clipboardCopyAction", "Copy")
                .with_category("Edit"),
            |_| Ok(None),
        );
        self.register(
            Command::new("editor.action.clipboardPasteAction", "Paste")
                .with_category("Edit"),
            |_| Ok(None),
        );
        self.register(
            Command::new("editor.action.selectAll", "Select All")
                .with_category("Edit"),
            |_| Ok(None),
        );

        // View commands
        self.register(
            Command::new("workbench.action.showCommands", "Show All Commands")
                .with_category("View"),
            |_| Ok(None),
        );
        self.register(
            Command::new("workbench.action.quickOpen", "Go to File...")
                .with_category("View"),
            |_| Ok(None),
        );
        self.register(
            Command::new("workbench.view.explorer", "Explorer")
                .with_category("View"),
            |_| Ok(None),
        );
        self.register(
            Command::new("workbench.view.search", "Search")
                .with_category("View"),
            |_| Ok(None),
        );
        self.register(
            Command::new("workbench.view.scm", "Source Control")
                .with_category("View"),
            |_| Ok(None),
        );
        self.register(
            Command::new("workbench.view.debug", "Run and Debug")
                .with_category("View"),
            |_| Ok(None),
        );
        self.register(
            Command::new("workbench.view.extensions", "Extensions")
                .with_category("View"),
            |_| Ok(None),
        );

        // Terminal commands
        self.register(
            Command::new("workbench.action.terminal.new", "New Terminal")
                .with_category("Terminal"),
            |_| Ok(None),
        );
        self.register(
            Command::new("workbench.action.terminal.toggleTerminal", "Toggle Terminal")
                .with_category("View"),
            |_| Ok(None),
        );

        // Editor commands
        self.register(
            Command::new("editor.action.formatDocument", "Format Document")
                .with_category("Editor"),
            |_| Ok(None),
        );
        self.register(
            Command::new("editor.action.commentLine", "Toggle Line Comment")
                .with_category("Editor"),
            |_| Ok(None),
        );
        self.register(
            Command::new("editor.action.blockComment", "Toggle Block Comment")
                .with_category("Editor"),
            |_| Ok(None),
        );
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}
