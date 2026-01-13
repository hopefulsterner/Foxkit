//! Quick open providers

use std::path::PathBuf;
use async_trait::async_trait;

use crate::QuickPickItem;

/// Provider context
pub struct ProviderContext {
    /// Current workspace
    pub workspace: PathBuf,
    /// Current file
    pub current_file: Option<PathBuf>,
    /// Query string
    pub query: String,
}

/// Provider trait
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider ID
    fn id(&self) -> &str;

    /// Prefix (e.g., "@" for symbols, "#" for tags)
    fn prefix(&self) -> Option<&str> {
        None
    }

    /// Provide items
    async fn provide(&self, ctx: &ProviderContext) -> anyhow::Result<Vec<QuickPickItem>>;
}

/// File provider
pub struct FileProvider {
    workspace: PathBuf,
}

impl FileProvider {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Provider for FileProvider {
    fn id(&self) -> &str {
        "files"
    }

    async fn provide(&self, ctx: &ProviderContext) -> anyhow::Result<Vec<QuickPickItem>> {
        let mut items = Vec::new();

        fn walk(dir: &PathBuf, items: &mut Vec<QuickPickItem>, workspace: &PathBuf) -> anyhow::Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                // Skip hidden and common excluded directories
                if name.starts_with('.') || name == "node_modules" || name == "target" {
                    continue;
                }

                if path.is_dir() {
                    walk(&path, items, workspace)?;
                } else if path.is_file() {
                    let relative = path.strip_prefix(workspace)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();

                    let icon = file_icon(&path);

                    items.push(QuickPickItem::new(&relative)
                        .with_description(path.parent()
                            .and_then(|p| p.strip_prefix(workspace).ok())
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default())
                        .with_icon(icon));
                }
            }
            Ok(())
        }

        walk(&self.workspace, &mut items, &self.workspace)?;

        Ok(items)
    }
}

fn file_icon(path: &PathBuf) -> String {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "md" => "markdown",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "html" => "html",
        "css" | "scss" | "sass" => "css",
        _ => "file",
    }.to_string()
}

/// Recent files provider
pub struct RecentFilesProvider {
    recent: Vec<PathBuf>,
}

impl RecentFilesProvider {
    pub fn new(recent: Vec<PathBuf>) -> Self {
        Self { recent }
    }
}

#[async_trait]
impl Provider for RecentFilesProvider {
    fn id(&self) -> &str {
        "recent"
    }

    async fn provide(&self, ctx: &ProviderContext) -> anyhow::Result<Vec<QuickPickItem>> {
        Ok(self.recent.iter()
            .map(|path| {
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                
                QuickPickItem::new(name)
                    .with_description(path.to_string_lossy())
                    .with_icon(file_icon(path))
            })
            .collect())
    }
}

/// Command provider
pub struct CommandProvider {
    commands: Vec<CommandInfo>,
}

#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub id: String,
    pub title: String,
    pub category: Option<String>,
    pub keybinding: Option<String>,
}

impl CommandProvider {
    pub fn new(commands: Vec<CommandInfo>) -> Self {
        Self { commands }
    }
}

#[async_trait]
impl Provider for CommandProvider {
    fn id(&self) -> &str {
        "commands"
    }

    fn prefix(&self) -> Option<&str> {
        Some(">")
    }

    async fn provide(&self, ctx: &ProviderContext) -> anyhow::Result<Vec<QuickPickItem>> {
        Ok(self.commands.iter()
            .map(|cmd| {
                let label = if let Some(cat) = &cmd.category {
                    format!("{}: {}", cat, cmd.title)
                } else {
                    cmd.title.clone()
                };

                QuickPickItem::new(label)
                    .with_description(cmd.keybinding.clone().unwrap_or_default())
            })
            .collect())
    }
}

/// Symbol provider
pub struct SymbolProvider;

#[async_trait]
impl Provider for SymbolProvider {
    fn id(&self) -> &str {
        "symbols"
    }

    fn prefix(&self) -> Option<&str> {
        Some("@")
    }

    async fn provide(&self, ctx: &ProviderContext) -> anyhow::Result<Vec<QuickPickItem>> {
        // Would query LSP for symbols
        Ok(Vec::new())
    }
}

/// Line provider (go to line)
pub struct LineProvider;

#[async_trait]
impl Provider for LineProvider {
    fn id(&self) -> &str {
        "lines"
    }

    fn prefix(&self) -> Option<&str> {
        Some(":")
    }

    async fn provide(&self, ctx: &ProviderContext) -> anyhow::Result<Vec<QuickPickItem>> {
        // Would provide line navigation
        Ok(Vec::new())
    }
}

/// Workspace symbol provider
pub struct WorkspaceSymbolProvider;

#[async_trait]
impl Provider for WorkspaceSymbolProvider {
    fn id(&self) -> &str {
        "workspace-symbols"
    }

    fn prefix(&self) -> Option<&str> {
        Some("#")
    }

    async fn provide(&self, ctx: &ProviderContext) -> anyhow::Result<Vec<QuickPickItem>> {
        // Would query LSP for workspace symbols
        Ok(Vec::new())
    }
}
