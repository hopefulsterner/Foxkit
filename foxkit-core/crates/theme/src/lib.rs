//! # Foxkit Theme
//!
//! Theming system for UI and syntax highlighting.
//! Compatible with VS Code themes and TextMate grammars.

pub mod color;
pub mod syntax;
pub mod ui;

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

pub use color::{Color, Hsla, Rgba};
pub use syntax::{SyntaxTheme, TokenStyle, FontStyle};
pub use ui::UiTheme;

/// Complete theme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Theme name
    pub name: String,
    /// Theme type
    pub kind: ThemeKind,
    /// UI colors
    pub ui: UiTheme,
    /// Syntax highlighting
    pub syntax: SyntaxTheme,
    /// Custom colors
    #[serde(default)]
    pub colors: HashMap<String, Color>,
}

impl Theme {
    /// Create a new theme
    pub fn new(name: impl Into<String>, kind: ThemeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            ui: UiTheme::default_for(kind),
            syntax: SyntaxTheme::default_for(kind),
            colors: HashMap::new(),
        }
    }

    /// Dark theme
    pub fn dark() -> Self {
        Self::new("Foxkit Dark", ThemeKind::Dark)
    }

    /// Light theme
    pub fn light() -> Self {
        Self::new("Foxkit Light", ThemeKind::Light)
    }

    /// Get a custom color
    pub fn color(&self, key: &str) -> Option<Color> {
        self.colors.get(key).copied()
    }

    /// Load from JSON
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Convert to JSON
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Load VS Code theme
    pub fn from_vscode(json: &str) -> anyhow::Result<Self> {
        vscode::parse_vscode_theme(json)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Theme kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeKind {
    Dark,
    Light,
    HighContrastDark,
    HighContrastLight,
}

impl ThemeKind {
    pub fn is_dark(&self) -> bool {
        matches!(self, ThemeKind::Dark | ThemeKind::HighContrastDark)
    }

    pub fn is_light(&self) -> bool {
        !self.is_dark()
    }

    pub fn is_high_contrast(&self) -> bool {
        matches!(self, ThemeKind::HighContrastDark | ThemeKind::HighContrastLight)
    }
}

/// Theme registry
pub struct ThemeRegistry {
    themes: HashMap<String, Arc<Theme>>,
    active: String,
}

impl ThemeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            themes: HashMap::new(),
            active: "Foxkit Dark".to_string(),
        };
        
        // Register built-in themes
        registry.register(Theme::dark());
        registry.register(Theme::light());
        
        registry
    }

    /// Register a theme
    pub fn register(&mut self, theme: Theme) {
        self.themes.insert(theme.name.clone(), Arc::new(theme));
    }

    /// Get a theme by name
    pub fn get(&self, name: &str) -> Option<Arc<Theme>> {
        self.themes.get(name).cloned()
    }

    /// Get active theme
    pub fn active(&self) -> Arc<Theme> {
        self.themes.get(&self.active).cloned()
            .unwrap_or_else(|| Arc::new(Theme::dark()))
    }

    /// Set active theme
    pub fn set_active(&mut self, name: &str) -> bool {
        if self.themes.contains_key(name) {
            self.active = name.to_string();
            true
        } else {
            false
        }
    }

    /// List all theme names
    pub fn list(&self) -> Vec<&str> {
        self.themes.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// VS Code theme parsing
mod vscode {
    use super::*;

    #[derive(Deserialize)]
    struct VscodeTheme {
        name: Option<String>,
        #[serde(rename = "type")]
        kind: Option<String>,
        colors: Option<HashMap<String, String>>,
        #[serde(rename = "tokenColors")]
        token_colors: Option<Vec<TokenColor>>,
    }

    #[derive(Deserialize)]
    struct TokenColor {
        scope: Option<ScopeValue>,
        settings: TokenSettings,
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ScopeValue {
        Single(String),
        Multiple(Vec<String>),
    }

    #[derive(Deserialize)]
    struct TokenSettings {
        foreground: Option<String>,
        background: Option<String>,
        #[serde(rename = "fontStyle")]
        font_style: Option<String>,
    }

    pub fn parse_vscode_theme(json: &str) -> anyhow::Result<Theme> {
        let vscode: VscodeTheme = serde_json::from_str(json)?;

        let kind = match vscode.kind.as_deref() {
            Some("light") => ThemeKind::Light,
            Some("hc") | Some("hcDark") => ThemeKind::HighContrastDark,
            Some("hcLight") => ThemeKind::HighContrastLight,
            _ => ThemeKind::Dark,
        };

        let mut theme = Theme::new(
            vscode.name.unwrap_or_else(|| "Imported Theme".to_string()),
            kind,
        );

        // Parse colors
        if let Some(colors) = vscode.colors {
            for (key, value) in colors {
                if let Some(color) = Color::from_hex(&value) {
                    theme.colors.insert(key, color);
                }
            }
        }

        // Parse token colors
        if let Some(token_colors) = vscode.token_colors {
            for tc in token_colors {
                let scopes = match tc.scope {
                    Some(ScopeValue::Single(s)) => vec![s],
                    Some(ScopeValue::Multiple(v)) => v,
                    None => continue,
                };

                let style = TokenStyle {
                    foreground: tc.settings.foreground.as_ref().and_then(|s| Color::from_hex(s)),
                    background: tc.settings.background.as_ref().and_then(|s| Color::from_hex(s)),
                    font_style: tc.settings.font_style.as_ref().map(|s| parse_font_style(s)),
                };

                for scope in scopes {
                    theme.syntax.rules.insert(scope, style.clone());
                }
            }
        }

        Ok(theme)
    }

    fn parse_font_style(s: &str) -> FontStyle {
        let mut style = FontStyle::default();
        for part in s.split_whitespace() {
            match part {
                "bold" => style.bold = true,
                "italic" => style.italic = true,
                "underline" => style.underline = true,
                "strikethrough" => style.strikethrough = true,
                _ => {}
            }
        }
        style
    }
}
