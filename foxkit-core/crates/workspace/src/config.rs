//! Workspace configuration

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Workspace configuration/settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Editor settings
    #[serde(default)]
    pub editor: EditorConfig,
    /// Files settings
    #[serde(default)]
    pub files: FilesConfig,
    /// Search settings
    #[serde(default)]
    pub search: SearchConfig,
    /// Raw settings (for extensions)
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

impl WorkspaceConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a setting by key path (e.g., "editor.fontSize")
    pub fn get(&self, key: &str) -> Option<&Value> {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.is_empty() {
            return None;
        }

        // Check in other for full path
        self.other.get(key)
    }

    /// Set a setting
    pub fn set(&mut self, key: &str, value: Value) {
        self.other.insert(key.to_string(), value);
    }
}

/// Editor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    #[serde(rename = "fontSize", default = "default_font_size")]
    pub font_size: u32,
    
    #[serde(rename = "fontFamily", default = "default_font_family")]
    pub font_family: String,
    
    #[serde(rename = "tabSize", default = "default_tab_size")]
    pub tab_size: u32,
    
    #[serde(rename = "insertSpaces", default = "default_insert_spaces")]
    pub insert_spaces: bool,
    
    #[serde(rename = "wordWrap", default)]
    pub word_wrap: WordWrap,
    
    #[serde(rename = "lineNumbers", default)]
    pub line_numbers: LineNumbers,
    
    #[serde(rename = "minimap.enabled", default = "default_true")]
    pub minimap_enabled: bool,
    
    #[serde(rename = "cursorStyle", default)]
    pub cursor_style: CursorStyle,
    
    #[serde(rename = "cursorBlinking", default)]
    pub cursor_blinking: CursorBlinking,
    
    #[serde(rename = "renderWhitespace", default)]
    pub render_whitespace: RenderWhitespace,
    
    #[serde(rename = "autoSave", default)]
    pub auto_save: AutoSave,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            font_size: 14,
            font_family: "Consolas, 'Courier New', monospace".to_string(),
            tab_size: 4,
            insert_spaces: true,
            word_wrap: WordWrap::Off,
            line_numbers: LineNumbers::On,
            minimap_enabled: true,
            cursor_style: CursorStyle::Line,
            cursor_blinking: CursorBlinking::Blink,
            render_whitespace: RenderWhitespace::Selection,
            auto_save: AutoSave::Off,
        }
    }
}

fn default_font_size() -> u32 { 14 }
fn default_font_family() -> String { "Consolas, 'Courier New', monospace".to_string() }
fn default_tab_size() -> u32 { 4 }
fn default_insert_spaces() -> bool { true }
fn default_true() -> bool { true }

/// Word wrap mode
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WordWrap {
    #[default]
    Off,
    On,
    WordWrapColumn,
    Bounded,
}

/// Line numbers mode
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineNumbers {
    #[default]
    On,
    Off,
    Relative,
    Interval,
}

/// Cursor style
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle {
    #[default]
    Line,
    Block,
    Underline,
    LineThin,
    BlockOutline,
    UnderlineThin,
}

/// Cursor blinking
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CursorBlinking {
    #[default]
    Blink,
    Smooth,
    Phase,
    Expand,
    Solid,
}

/// Render whitespace
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RenderWhitespace {
    None,
    Boundary,
    #[default]
    Selection,
    Trailing,
    All,
}

/// Auto save mode
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AutoSave {
    #[default]
    Off,
    AfterDelay,
    OnFocusChange,
    OnWindowChange,
}

/// Files configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesConfig {
    #[serde(default)]
    pub exclude: HashMap<String, bool>,
    
    #[serde(rename = "autoSaveDelay", default = "default_auto_save_delay")]
    pub auto_save_delay: u32,
    
    #[serde(default)]
    pub encoding: String,
    
    #[serde(rename = "eol", default = "default_eol")]
    pub eol: String,
    
    #[serde(rename = "trimTrailingWhitespace", default)]
    pub trim_trailing_whitespace: bool,
    
    #[serde(rename = "insertFinalNewline", default)]
    pub insert_final_newline: bool,
}

impl Default for FilesConfig {
    fn default() -> Self {
        let mut exclude = HashMap::new();
        exclude.insert("**/.git".to_string(), true);
        exclude.insert("**/.svn".to_string(), true);
        exclude.insert("**/.hg".to_string(), true);
        exclude.insert("**/CVS".to_string(), true);
        exclude.insert("**/.DS_Store".to_string(), true);
        exclude.insert("**/Thumbs.db".to_string(), true);

        Self {
            exclude,
            auto_save_delay: 1000,
            encoding: "utf8".to_string(),
            eol: "auto".to_string(),
            trim_trailing_whitespace: false,
            insert_final_newline: false,
        }
    }
}

fn default_auto_save_delay() -> u32 { 1000 }
fn default_eol() -> String { "auto".to_string() }

/// Search configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default)]
    pub exclude: HashMap<String, bool>,
    
    #[serde(rename = "useIgnoreFiles", default = "default_true")]
    pub use_ignore_files: bool,
    
    #[serde(rename = "useGlobalIgnoreFiles", default = "default_true")]
    pub use_global_ignore_files: bool,
    
    #[serde(rename = "followSymlinks", default = "default_true")]
    pub follow_symlinks: bool,
}
