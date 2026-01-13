//! Plugin manifest

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin ID
    pub id: String,
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    #[serde(default)]
    pub description: String,
    /// Author
    #[serde(default)]
    pub author: Option<String>,
    /// License
    #[serde(default)]
    pub license: Option<String>,
    /// Repository URL
    #[serde(default)]
    pub repository: Option<String>,
    /// WASM file path (relative to manifest)
    pub wasm: String,
    /// Required permissions
    #[serde(default)]
    pub permissions: Vec<PluginPermission>,
    /// Activation events
    #[serde(default)]
    pub activation_events: Vec<ActivationEvent>,
    /// Contributed features
    #[serde(default)]
    pub contributes: PluginContributes,
    /// Configuration schema
    #[serde(default)]
    pub configuration: Option<ConfigurationSchema>,
    /// Minimum engine version
    #[serde(default)]
    pub engine: Option<String>,
    /// Dependencies on other plugins
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

impl PluginManifest {
    /// Check if plugin has permission
    pub fn has_permission(&self, permission: &PluginPermission) -> bool {
        self.permissions.contains(permission)
    }
}

/// Plugin permission
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    /// Read files in workspace
    FileRead,
    /// Write files in workspace
    FileWrite,
    /// Execute terminal commands
    Terminal,
    /// Network access
    Network,
    /// Access clipboard
    Clipboard,
    /// Access environment variables
    Environment,
    /// Access secrets/credentials
    Secrets,
    /// Git operations
    Git,
    /// Debug session access
    Debug,
    /// Custom permission
    Custom(String),
}

/// Activation event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
#[serde(rename_all = "camelCase")]
pub enum ActivationEvent {
    /// Activate on startup
    OnStartup,
    /// Activate for language
    OnLanguage(String),
    /// Activate for file pattern
    OnFilePattern(String),
    /// Activate on command
    OnCommand(String),
    /// Activate on view
    OnView(String),
    /// Activate when file exists in workspace
    WorkspaceContains(String),
}

/// Plugin contributions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginContributes {
    /// Commands
    pub commands: Vec<CommandContribution>,
    /// Keybindings
    pub keybindings: Vec<KeybindingContribution>,
    /// Menu items
    pub menus: HashMap<String, Vec<MenuContribution>>,
    /// Views
    pub views: HashMap<String, Vec<ViewContribution>>,
    /// Languages
    pub languages: Vec<LanguageContribution>,
    /// Themes
    pub themes: Vec<ThemeContribution>,
    /// Snippets
    pub snippets: Vec<SnippetContribution>,
}

/// Command contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContribution {
    /// Command ID
    pub id: String,
    /// Display title
    pub title: String,
    /// Category
    #[serde(default)]
    pub category: Option<String>,
    /// Icon
    #[serde(default)]
    pub icon: Option<String>,
}

/// Keybinding contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingContribution {
    /// Command to execute
    pub command: String,
    /// Key combination
    pub key: String,
    /// When clause
    #[serde(default)]
    pub when: Option<String>,
    /// Platform-specific keys
    #[serde(default)]
    pub mac: Option<String>,
    #[serde(default)]
    pub linux: Option<String>,
    #[serde(default)]
    pub windows: Option<String>,
}

/// Menu contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuContribution {
    /// Command to execute
    pub command: String,
    /// When clause
    #[serde(default)]
    pub when: Option<String>,
    /// Group
    #[serde(default)]
    pub group: Option<String>,
}

/// View contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewContribution {
    /// View ID
    pub id: String,
    /// View name
    pub name: String,
    /// When clause
    #[serde(default)]
    pub when: Option<String>,
}

/// Language contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageContribution {
    /// Language ID
    pub id: String,
    /// Display name
    pub name: String,
    /// File extensions
    #[serde(default)]
    pub extensions: Vec<String>,
    /// File names
    #[serde(default)]
    pub filenames: Vec<String>,
    /// First line pattern
    #[serde(default)]
    pub first_line: Option<String>,
    /// Configuration file
    #[serde(default)]
    pub configuration: Option<String>,
}

/// Theme contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeContribution {
    /// Theme ID
    pub id: String,
    /// Display label
    pub label: String,
    /// Theme file path
    pub path: String,
    /// UI theme (dark/light)
    #[serde(default = "default_theme_ui")]
    pub ui_theme: String,
}

fn default_theme_ui() -> String {
    "dark".to_string()
}

/// Snippet contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetContribution {
    /// Language ID
    pub language: String,
    /// Snippets file path
    pub path: String,
}

/// Configuration schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationSchema {
    /// Schema title
    pub title: String,
    /// Properties
    pub properties: HashMap<String, ConfigProperty>,
}

/// Configuration property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigProperty {
    /// Property type
    #[serde(rename = "type")]
    pub prop_type: String,
    /// Default value
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
    /// Enum values
    #[serde(default)]
    pub enum_values: Option<Vec<serde_json::Value>>,
}
