//! Application settings management
//! 
//! Hierarchical settings system with:
//! - Default settings
//! - User settings
//! - Workspace settings
//! - Extension settings

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use anyhow::Result;

/// Main settings structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Editor settings
    pub editor: EditorSettings,
    /// Terminal settings
    pub terminal: TerminalSettings,
    /// AI assistant settings
    pub ai: AiSettings,
    /// Theme and appearance
    pub appearance: AppearanceSettings,
    /// Monorepo intelligence settings
    pub monorepo: MonorepoSettings,
    /// Collaboration settings
    pub collaboration: CollaborationSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    pub font_family: String,
    pub font_size: f32,
    pub tab_size: u32,
    pub insert_spaces: bool,
    pub line_numbers: LineNumbersMode,
    pub minimap_enabled: bool,
    pub word_wrap: WordWrap,
    pub auto_save: AutoSave,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LineNumbersMode {
    Off,
    On,
    Relative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WordWrap {
    Off,
    On,
    Bounded { column: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutoSave {
    Off,
    AfterDelay { ms: u64 },
    OnFocusChange,
    OnWindowChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSettings {
    pub font_family: String,
    pub font_size: f32,
    pub shell: Option<String>,
    pub env: std::collections::HashMap<String, String>,
    pub scrollback: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettings {
    pub enabled: bool,
    pub provider: AiProvider,
    pub model: String,
    pub api_key_env: Option<String>,
    pub context_window: u32,
    pub inline_suggestions: bool,
    pub chat_enabled: bool,
    pub autonomous_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiProvider {
    OpenAI,
    Anthropic,
    Azure,
    Ollama,
    Custom { endpoint: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    pub theme: String,
    pub icon_theme: String,
    pub ui_scale: f32,
    pub sidebar_position: SidebarPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SidebarPosition {
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonorepoSettings {
    /// Enable monorepo intelligence features
    pub enabled: bool,
    /// Package manager detection
    pub package_manager: Option<PackageManager>,
    /// Build system detection  
    pub build_system: Option<BuildSystem>,
    /// Paths to ignore when scanning
    pub ignore_paths: Vec<String>,
    /// Enable cross-package navigation
    pub cross_package_navigation: bool,
    /// Enable dependency graph visualization
    pub dependency_graph: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
    Cargo,
    Go,
    Pip,
    Poetry,
    Maven,
    Gradle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildSystem {
    Nx,
    Turborepo,
    Lerna,
    Bazel,
    Buck,
    Pants,
    Cargo,
    Make,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSettings {
    pub enabled: bool,
    pub show_cursors: bool,
    pub show_selections: bool,
    pub share_terminal: bool,
    pub share_debugger: bool,
}

impl Settings {
    /// Load settings from disk or return defaults
    pub fn load_or_default() -> Result<Self> {
        // Try to load from config file
        let config_path = Self::config_path();
        
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let settings: Settings = toml::from_str(&content)?;
            Ok(settings)
        } else {
            Ok(Self::default())
        }
    }

    /// Save settings to disk
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        
        Ok(())
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("foxkit")
            .join("settings.toml")
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            editor: EditorSettings {
                font_family: "JetBrains Mono".into(),
                font_size: 14.0,
                tab_size: 4,
                insert_spaces: true,
                line_numbers: LineNumbersMode::On,
                minimap_enabled: true,
                word_wrap: WordWrap::Off,
                auto_save: AutoSave::AfterDelay { ms: 1000 },
            },
            terminal: TerminalSettings {
                font_family: "JetBrains Mono".into(),
                font_size: 13.0,
                shell: None,
                env: std::collections::HashMap::new(),
                scrollback: 10000,
            },
            ai: AiSettings {
                enabled: true,
                provider: AiProvider::Anthropic,
                model: "claude-sonnet-4-20250514".into(),
                api_key_env: Some("ANTHROPIC_API_KEY".into()),
                context_window: 128000,
                inline_suggestions: true,
                chat_enabled: true,
                autonomous_mode: false,
            },
            appearance: AppearanceSettings {
                theme: "Foxkit Dark".into(),
                icon_theme: "Foxkit Icons".into(),
                ui_scale: 1.0,
                sidebar_position: SidebarPosition::Left,
            },
            monorepo: MonorepoSettings {
                enabled: true,
                package_manager: None,
                build_system: None,
                ignore_paths: vec![
                    "node_modules".into(),
                    "target".into(),
                    "dist".into(),
                    ".git".into(),
                ],
                cross_package_navigation: true,
                dependency_graph: true,
            },
            collaboration: CollaborationSettings {
                enabled: true,
                show_cursors: true,
                show_selections: true,
                share_terminal: false,
                share_debugger: false,
            },
        }
    }
}
