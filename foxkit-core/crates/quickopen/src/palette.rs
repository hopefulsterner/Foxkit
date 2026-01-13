//! Command palette

use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{QuickPick, QuickPickItem, QuickPickOptions, FuzzyMatcher};

/// Command palette
pub struct CommandPalette {
    /// All commands
    commands: RwLock<Vec<PaletteCommand>>,
    /// Categories
    categories: RwLock<Vec<String>>,
    /// Recent commands
    recent: RwLock<Vec<String>>,
    /// Fuzzy matcher
    matcher: FuzzyMatcher,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            commands: RwLock::new(Vec::new()),
            categories: RwLock::new(Vec::new()),
            recent: RwLock::new(Vec::new()),
            matcher: FuzzyMatcher::new(),
        }
    }

    /// Register a command
    pub fn register(&self, command: PaletteCommand) {
        let category = command.category.clone();
        self.commands.write().push(command);
        
        if let Some(cat) = category {
            let mut categories = self.categories.write();
            if !categories.contains(&cat) {
                categories.push(cat);
            }
        }
    }

    /// Unregister a command
    pub fn unregister(&self, id: &str) {
        self.commands.write().retain(|c| c.id != id);
    }

    /// Get filtered commands
    pub fn filter(&self, query: &str) -> Vec<&PaletteCommand> {
        let commands = self.commands.read();
        let query = query.strip_prefix('>').unwrap_or(query).trim();
        
        if query.is_empty() {
            // Show all, with recent first
            let recent = self.recent.read();
            let mut result: Vec<_> = commands.iter().collect();
            
            result.sort_by(|a, b| {
                let a_recent = recent.iter().position(|id| id == &a.id);
                let b_recent = recent.iter().position(|id| id == &b.id);
                
                match (a_recent, b_recent) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.label().cmp(&b.label()),
                }
            });
            
            return result;
        }

        // Filter and score
        let mut scored: Vec<_> = commands.iter()
            .filter_map(|cmd| {
                let score = self.matcher.score(&cmd.label(), query)
                    .or_else(|| self.matcher.score(&cmd.id, query));
                score.map(|s| (cmd, s))
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.cmp(&a.1));

        scored.into_iter().map(|(cmd, _)| cmd).collect()
    }

    /// Execute command
    pub async fn execute(&self, id: &str) -> anyhow::Result<()> {
        // Add to recent
        {
            let mut recent = self.recent.write();
            recent.retain(|i| i != id);
            recent.insert(0, id.to_string());
            recent.truncate(10);
        }

        // Find and execute
        let commands = self.commands.read();
        let command = commands.iter()
            .find(|c| c.id == id)
            .ok_or_else(|| anyhow::anyhow!("Command not found: {}", id))?;

        tracing::info!("Executing command: {}", id);
        
        // Would execute the command callback
        Ok(())
    }

    /// Get all commands as quick pick items
    pub fn as_quick_pick_items(&self) -> Vec<QuickPickItem> {
        self.commands.read().iter()
            .map(|cmd| {
                let label = cmd.label();
                let mut item = QuickPickItem::new(label);
                
                if let Some(ref kb) = cmd.keybinding {
                    item = item.with_description(kb.clone());
                }
                
                item
            })
            .collect()
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

/// Palette command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaletteCommand {
    /// Command ID
    pub id: String,
    /// Display title
    pub title: String,
    /// Category
    pub category: Option<String>,
    /// Keybinding
    pub keybinding: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Is enabled
    pub enabled: bool,
}

impl PaletteCommand {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            category: None,
            keybinding: None,
            description: None,
            enabled: true,
        }
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    pub fn with_keybinding(mut self, keybinding: impl Into<String>) -> Self {
        self.keybinding = Some(keybinding.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Get full label with category
    pub fn label(&self) -> String {
        if let Some(ref cat) = self.category {
            format!("{}: {}", cat, self.title)
        } else {
            self.title.clone()
        }
    }
}

/// Built-in commands
pub fn builtin_commands() -> Vec<PaletteCommand> {
    vec![
        // File commands
        PaletteCommand::new("foxkit.newFile", "New File")
            .with_category("File")
            .with_keybinding("Ctrl+N"),
        PaletteCommand::new("foxkit.openFile", "Open File")
            .with_category("File")
            .with_keybinding("Ctrl+O"),
        PaletteCommand::new("foxkit.save", "Save")
            .with_category("File")
            .with_keybinding("Ctrl+S"),
        PaletteCommand::new("foxkit.saveAs", "Save As...")
            .with_category("File")
            .with_keybinding("Ctrl+Shift+S"),
        PaletteCommand::new("foxkit.closeFile", "Close File")
            .with_category("File")
            .with_keybinding("Ctrl+W"),

        // Edit commands
        PaletteCommand::new("foxkit.undo", "Undo")
            .with_category("Edit")
            .with_keybinding("Ctrl+Z"),
        PaletteCommand::new("foxkit.redo", "Redo")
            .with_category("Edit")
            .with_keybinding("Ctrl+Shift+Z"),
        PaletteCommand::new("foxkit.cut", "Cut")
            .with_category("Edit")
            .with_keybinding("Ctrl+X"),
        PaletteCommand::new("foxkit.copy", "Copy")
            .with_category("Edit")
            .with_keybinding("Ctrl+C"),
        PaletteCommand::new("foxkit.paste", "Paste")
            .with_category("Edit")
            .with_keybinding("Ctrl+V"),
        PaletteCommand::new("foxkit.find", "Find")
            .with_category("Edit")
            .with_keybinding("Ctrl+F"),
        PaletteCommand::new("foxkit.replace", "Find and Replace")
            .with_category("Edit")
            .with_keybinding("Ctrl+H"),

        // View commands
        PaletteCommand::new("foxkit.toggleSidebar", "Toggle Sidebar")
            .with_category("View")
            .with_keybinding("Ctrl+B"),
        PaletteCommand::new("foxkit.togglePanel", "Toggle Panel")
            .with_category("View")
            .with_keybinding("Ctrl+J"),
        PaletteCommand::new("foxkit.zoomIn", "Zoom In")
            .with_category("View")
            .with_keybinding("Ctrl++"),
        PaletteCommand::new("foxkit.zoomOut", "Zoom Out")
            .with_category("View")
            .with_keybinding("Ctrl+-"),

        // Go commands
        PaletteCommand::new("foxkit.goToFile", "Go to File")
            .with_category("Go")
            .with_keybinding("Ctrl+P"),
        PaletteCommand::new("foxkit.goToSymbol", "Go to Symbol")
            .with_category("Go")
            .with_keybinding("Ctrl+Shift+O"),
        PaletteCommand::new("foxkit.goToLine", "Go to Line")
            .with_category("Go")
            .with_keybinding("Ctrl+G"),
        PaletteCommand::new("foxkit.goToDefinition", "Go to Definition")
            .with_category("Go")
            .with_keybinding("F12"),

        // Run commands
        PaletteCommand::new("foxkit.startDebugging", "Start Debugging")
            .with_category("Run")
            .with_keybinding("F5"),
        PaletteCommand::new("foxkit.runWithoutDebugging", "Run Without Debugging")
            .with_category("Run")
            .with_keybinding("Ctrl+F5"),
        PaletteCommand::new("foxkit.toggleBreakpoint", "Toggle Breakpoint")
            .with_category("Run")
            .with_keybinding("F9"),

        // Terminal commands
        PaletteCommand::new("foxkit.newTerminal", "New Terminal")
            .with_category("Terminal")
            .with_keybinding("Ctrl+`"),

        // Git commands
        PaletteCommand::new("foxkit.gitCommit", "Git: Commit")
            .with_category("Git"),
        PaletteCommand::new("foxkit.gitPush", "Git: Push")
            .with_category("Git"),
        PaletteCommand::new("foxkit.gitPull", "Git: Pull")
            .with_category("Git"),
        PaletteCommand::new("foxkit.gitCheckout", "Git: Checkout to...")
            .with_category("Git"),

        // Settings
        PaletteCommand::new("foxkit.openSettings", "Preferences: Open Settings")
            .with_category("Preferences")
            .with_keybinding("Ctrl+,"),
        PaletteCommand::new("foxkit.openKeybindings", "Preferences: Open Keyboard Shortcuts")
            .with_category("Preferences")
            .with_keybinding("Ctrl+K Ctrl+S"),
    ]
}
