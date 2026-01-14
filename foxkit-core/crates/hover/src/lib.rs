//! # Foxkit Hover
//!
//! Hover information and quick info (LSP-compatible).

pub mod content;
pub mod provider;

pub use content::{HoverBuilder, SymbolHover, SymbolKind as HoverSymbolKind, ParameterInfo, DiagnosticHover};

use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub use provider::HoverProvider;

/// Hover information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hover {
    /// Hover content
    pub contents: HoverContents,
    /// Range to highlight
    pub range: Option<Range>,
}

impl Hover {
    pub fn new(contents: HoverContents) -> Self {
        Self {
            contents,
            range: None,
        }
    }

    pub fn text(text: &str) -> Self {
        Self::new(HoverContents::String(text.to_string()))
    }

    pub fn markdown(markdown: &str) -> Self {
        Self::new(HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown.to_string(),
        }))
    }

    pub fn code(code: &str, language: &str) -> Self {
        Self::markdown(&format!("```{}\n{}\n```", language, code))
    }

    pub fn with_range(mut self, range: Range) -> Self {
        self.range = Some(range);
        self
    }
}

/// Hover contents
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HoverContents {
    String(String),
    Markup(MarkupContent),
    Multiple(Vec<MarkedString>),
}

/// Marked string (for multiple contents)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MarkedString {
    String(String),
    Code { language: String, value: String },
}

impl MarkedString {
    pub fn code(language: &str, value: &str) -> Self {
        Self::Code {
            language: language.to_string(),
            value: value.to_string(),
        }
    }
}

/// Markup content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkupContent {
    pub kind: MarkupKind,
    pub value: String,
}

/// Markup kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkupKind {
    #[serde(rename = "plaintext")]
    PlainText,
    #[serde(rename = "markdown")]
    Markdown,
}

/// Position
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// Range
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn point(line: u32, character: u32) -> Self {
        let pos = Position::new(line, character);
        Self { start: pos, end: pos }
    }
}

/// Hover request parameters
#[derive(Debug, Clone)]
pub struct HoverParams {
    /// File path
    pub file_path: String,
    /// Position
    pub position: Position,
    /// Language ID
    pub language_id: String,
    /// Word at position
    pub word: Option<String>,
}

impl HoverParams {
    pub fn new(file_path: &str, position: Position) -> Self {
        Self {
            file_path: file_path.to_string(),
            position,
            language_id: String::new(),
            word: None,
        }
    }

    pub fn with_language(mut self, language_id: &str) -> Self {
        self.language_id = language_id.to_string();
        self
    }

    pub fn with_word(mut self, word: &str) -> Self {
        self.word = Some(word.to_string());
        self
    }
}

/// Hover service
pub struct HoverService {
    providers: RwLock<Vec<Arc<dyn HoverProvider>>>,
}

impl HoverService {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(Vec::new()),
        }
    }

    /// Register a provider
    pub fn register(&self, provider: Arc<dyn HoverProvider>) {
        self.providers.write().push(provider);
    }

    /// Get hover information
    pub fn hover(&self, params: &HoverParams) -> Option<Hover> {
        let providers = self.providers.read();
        
        for provider in providers.iter() {
            if provider.should_provide(params) {
                if let Some(hover) = provider.provide(params) {
                    return Some(hover);
                }
            }
        }
        
        None
    }
}

impl Default for HoverService {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for complex hover content
pub struct HoverBuilder {
    sections: Vec<String>,
}

impl HoverBuilder {
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
        }
    }

    /// Add a heading
    pub fn heading(mut self, text: &str) -> Self {
        self.sections.push(format!("### {}", text));
        self
    }

    /// Add a code block
    pub fn code(mut self, code: &str, language: &str) -> Self {
        self.sections.push(format!("```{}\n{}\n```", language, code));
        self
    }

    /// Add inline code
    pub fn inline_code(mut self, code: &str) -> Self {
        self.sections.push(format!("`{}`", code));
        self
    }

    /// Add plain text
    pub fn text(mut self, text: &str) -> Self {
        self.sections.push(text.to_string());
        self
    }

    /// Add a separator
    pub fn separator(mut self) -> Self {
        self.sections.push("---".to_string());
        self
    }

    /// Add a list
    pub fn list(mut self, items: &[&str]) -> Self {
        let list = items.iter()
            .map(|item| format!("- {}", item))
            .collect::<Vec<_>>()
            .join("\n");
        self.sections.push(list);
        self
    }

    /// Build the hover
    pub fn build(self) -> Hover {
        Hover::markdown(&self.sections.join("\n\n"))
    }
}

impl Default for HoverBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hover_builder() {
        let hover = HoverBuilder::new()
            .heading("Function")
            .code("fn example() {}", "rust")
            .text("This is an example function.")
            .build();

        match hover.contents {
            HoverContents::Markup(content) => {
                assert!(content.value.contains("### Function"));
                assert!(content.value.contains("```rust"));
            }
            _ => panic!("Expected markup content"),
        }
    }
}
