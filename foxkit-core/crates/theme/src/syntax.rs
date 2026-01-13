//! Syntax highlighting theme

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::ThemeKind;

/// Syntax highlighting theme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxTheme {
    /// Default foreground
    pub foreground: Color,
    /// Default background
    pub background: Color,
    /// Selection background
    pub selection: Color,
    /// Cursor color
    pub cursor: Color,
    /// Line highlight
    pub line_highlight: Color,
    /// Token rules (scope -> style)
    #[serde(default)]
    pub rules: HashMap<String, TokenStyle>,
}

impl SyntaxTheme {
    /// Get style for a scope
    pub fn style_for_scope(&self, scope: &str) -> TokenStyle {
        // Try exact match first
        if let Some(style) = self.rules.get(scope) {
            return style.clone();
        }

        // Try parent scopes
        let parts: Vec<&str> = scope.split('.').collect();
        for i in (1..parts.len()).rev() {
            let parent = parts[..i].join(".");
            if let Some(style) = self.rules.get(&parent) {
                return style.clone();
            }
        }

        // Default style
        TokenStyle {
            foreground: Some(self.foreground),
            background: None,
            font_style: None,
        }
    }

    /// Default dark syntax theme
    pub fn dark() -> Self {
        let mut rules = HashMap::new();
        
        // Comments
        rules.insert("comment".into(), TokenStyle::color(0x6A9955));
        
        // Strings
        rules.insert("string".into(), TokenStyle::color(0xCE9178));
        
        // Keywords
        rules.insert("keyword".into(), TokenStyle::color(0x569CD6));
        rules.insert("keyword.control".into(), TokenStyle::color(0xC586C0));
        
        // Types
        rules.insert("entity.name.type".into(), TokenStyle::color(0x4EC9B0));
        rules.insert("entity.name.class".into(), TokenStyle::color(0x4EC9B0));
        rules.insert("support.type".into(), TokenStyle::color(0x4EC9B0));
        
        // Functions
        rules.insert("entity.name.function".into(), TokenStyle::color(0xDCDCAA));
        rules.insert("support.function".into(), TokenStyle::color(0xDCDCAA));
        
        // Variables
        rules.insert("variable".into(), TokenStyle::color(0x9CDCFE));
        rules.insert("variable.parameter".into(), TokenStyle::color(0x9CDCFE));
        
        // Constants
        rules.insert("constant".into(), TokenStyle::color(0xB5CEA8));
        rules.insert("constant.numeric".into(), TokenStyle::color(0xB5CEA8));
        
        // Operators
        rules.insert("keyword.operator".into(), TokenStyle::color(0xD4D4D4));
        
        // Punctuation
        rules.insert("punctuation".into(), TokenStyle::color(0xD4D4D4));

        Self {
            foreground: Color::hex(0xD4D4D4),
            background: Color::hex(0x1E1E1E),
            selection: Color::rgba(38, 79, 120, 180),
            cursor: Color::hex(0xAEAFAD),
            line_highlight: Color::rgba(255, 255, 255, 10),
            rules,
        }
    }

    /// Default light syntax theme
    pub fn light() -> Self {
        let mut rules = HashMap::new();
        
        // Comments
        rules.insert("comment".into(), TokenStyle::color(0x008000));
        
        // Strings
        rules.insert("string".into(), TokenStyle::color(0xA31515));
        
        // Keywords
        rules.insert("keyword".into(), TokenStyle::color(0x0000FF));
        rules.insert("keyword.control".into(), TokenStyle::color(0xAF00DB));
        
        // Types
        rules.insert("entity.name.type".into(), TokenStyle::color(0x267F99));
        rules.insert("entity.name.class".into(), TokenStyle::color(0x267F99));
        
        // Functions
        rules.insert("entity.name.function".into(), TokenStyle::color(0x795E26));
        
        // Variables
        rules.insert("variable".into(), TokenStyle::color(0x001080));
        
        // Constants
        rules.insert("constant".into(), TokenStyle::color(0x098658));
        rules.insert("constant.numeric".into(), TokenStyle::color(0x098658));

        Self {
            foreground: Color::hex(0x000000),
            background: Color::hex(0xFFFFFF),
            selection: Color::rgba(173, 214, 255, 180),
            cursor: Color::hex(0x000000),
            line_highlight: Color::rgba(0, 0, 0, 10),
            rules,
        }
    }

    /// Create default theme for kind
    pub fn default_for(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::Dark | ThemeKind::HighContrastDark => Self::dark(),
            ThemeKind::Light | ThemeKind::HighContrastLight => Self::light(),
        }
    }
}

impl Default for SyntaxTheme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Token style
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenStyle {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub font_style: Option<FontStyle>,
}

impl TokenStyle {
    pub fn color(hex: u32) -> Self {
        Self {
            foreground: Some(Color::hex(hex)),
            background: None,
            font_style: None,
        }
    }

    pub fn with_style(mut self, style: FontStyle) -> Self {
        self.font_style = Some(style);
        self
    }
}

/// Font style
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct FontStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
}

impl FontStyle {
    pub fn bold() -> Self {
        Self { bold: true, ..Default::default() }
    }

    pub fn italic() -> Self {
        Self { italic: true, ..Default::default() }
    }

    pub fn underline() -> Self {
        Self { underline: true, ..Default::default() }
    }
}
