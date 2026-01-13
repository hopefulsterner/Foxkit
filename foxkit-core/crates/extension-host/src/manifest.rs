//! Extension manifest (package.json equivalent)

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use semver::Version;

use crate::{Permission, ContributionKind};

/// Extension manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// Extension name
    pub name: String,
    /// Display name
    #[serde(default)]
    pub display_name: Option<String>,
    /// Publisher
    pub publisher: String,
    /// Version
    pub version: String,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
    /// Extension kind
    #[serde(default)]
    pub kind: ExtensionKind,
    /// Main entry point (WASM module)
    #[serde(default)]
    pub main: Option<String>,
    /// Activation events
    #[serde(default)]
    pub activation_events: Vec<String>,
    /// Required permissions
    #[serde(default)]
    pub permissions: Vec<Permission>,
    /// Contributions
    #[serde(default)]
    pub contributes: Contributions,
    /// Dependencies
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    /// Engine compatibility
    #[serde(default)]
    pub engines: EngineRequirements,
    /// Categories
    #[serde(default)]
    pub categories: Vec<String>,
    /// Keywords
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Icon path
    #[serde(default)]
    pub icon: Option<String>,
    /// Repository URL
    #[serde(default)]
    pub repository: Option<String>,
    /// License
    #[serde(default)]
    pub license: Option<String>,
}

impl ExtensionManifest {
    /// Check if extension has a specific contribution type
    pub fn has_contribution(&self, kind: ContributionKind) -> bool {
        match kind {
            ContributionKind::Commands => !self.contributes.commands.is_empty(),
            ContributionKind::Languages => !self.contributes.languages.is_empty(),
            ContributionKind::Grammars => !self.contributes.grammars.is_empty(),
            ContributionKind::Themes => !self.contributes.themes.is_empty(),
            ContributionKind::Snippets => !self.contributes.snippets.is_empty(),
            ContributionKind::Keybindings => !self.contributes.keybindings.is_empty(),
            ContributionKind::Views => !self.contributes.views.is_empty(),
            ContributionKind::Debuggers => !self.contributes.debuggers.is_empty(),
            ContributionKind::TaskProviders => !self.contributes.task_definitions.is_empty(),
        }
    }

    /// Get display name or name
    pub fn display_name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.name)
    }

    /// Parse version
    pub fn parsed_version(&self) -> Option<Version> {
        Version::parse(&self.version).ok()
    }

    /// Is extension UI-only (no runtime)?
    pub fn is_ui_only(&self) -> bool {
        self.kind == ExtensionKind::UI || self.main.is_none()
    }
}

/// Extension kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionKind {
    /// Workspace extension (runs in extension host)
    #[default]
    Workspace,
    /// UI extension (runs in UI process)
    UI,
    /// Universal (can run in either)
    Universal,
}

/// Engine requirements
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngineRequirements {
    /// Required Foxkit version
    #[serde(default)]
    pub foxkit: Option<String>,
    /// Required VS Code version (for compat)
    #[serde(default)]
    pub vscode: Option<String>,
}

/// Extension contributions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Contributions {
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub languages: Vec<LanguageContribution>,
    #[serde(default)]
    pub grammars: Vec<GrammarContribution>,
    #[serde(default)]
    pub themes: Vec<ThemeContribution>,
    #[serde(default)]
    pub snippets: Vec<SnippetContribution>,
    #[serde(default)]
    pub keybindings: Vec<KeybindingContribution>,
    #[serde(default)]
    pub views: HashMap<String, Vec<ViewContribution>>,
    #[serde(default)]
    pub debuggers: Vec<DebuggerContribution>,
    #[serde(default)]
    pub task_definitions: Vec<TaskDefinition>,
    #[serde(default)]
    pub configuration: Option<ConfigurationContribution>,
    #[serde(default)]
    pub menus: HashMap<String, Vec<MenuContribution>>,
}

/// Command contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContribution {
    pub command: String,
    pub title: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub enabled_when: Option<String>,
}

/// Language contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageContribution {
    pub id: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub filenames: Vec<String>,
    #[serde(default)]
    pub first_line: Option<String>,
    #[serde(default)]
    pub configuration: Option<String>,
}

/// Grammar contribution (TextMate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarContribution {
    pub language: String,
    pub scope_name: String,
    pub path: String,
    #[serde(default)]
    pub embedded_languages: HashMap<String, String>,
}

/// Theme contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeContribution {
    pub id: String,
    pub label: String,
    pub path: String,
    #[serde(default)]
    pub ui_theme: String,
}

/// Snippet contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetContribution {
    pub language: String,
    pub path: String,
}

/// Keybinding contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingContribution {
    pub command: String,
    pub key: String,
    #[serde(default)]
    pub mac: Option<String>,
    #[serde(default)]
    pub when: Option<String>,
}

/// View contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewContribution {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

/// Debugger contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebuggerContribution {
    #[serde(rename = "type")]
    pub debug_type: String,
    pub label: String,
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub languages: Vec<String>,
}

/// Task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    #[serde(rename = "type")]
    pub task_type: String,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
}

/// Configuration contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationContribution {
    pub title: String,
    #[serde(default)]
    pub properties: HashMap<String, ConfigurationProperty>,
}

/// Configuration property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationProperty {
    #[serde(rename = "type")]
    pub property_type: String,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,
}

/// Menu contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuContribution {
    pub command: String,
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
}

/// Contribution - generic contribution type
#[derive(Debug, Clone)]
pub enum Contribution {
    Command(CommandContribution),
    Language(LanguageContribution),
    Grammar(GrammarContribution),
    Theme(ThemeContribution),
    Keybinding(KeybindingContribution),
    View(ViewContribution),
    Debugger(DebuggerContribution),
}
