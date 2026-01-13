//! Language configuration

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tree_sitter::{Parser, Language as TsLanguage, Query};

use crate::{SyntaxTree, HighlightEvent};

/// A programming language
pub struct Language {
    pub config: LanguageConfig,
    ts_language: TsLanguage,
    highlight_query: Option<Query>,
    injection_query: Option<Query>,
    locals_query: Option<Query>,
}

impl Language {
    /// Create a new language
    pub fn new(config: LanguageConfig, ts_language: TsLanguage) -> Self {
        Self {
            config,
            ts_language,
            highlight_query: None,
            injection_query: None,
            locals_query: None,
        }
    }

    /// Set highlight query
    pub fn with_highlights(mut self, query: &str) -> anyhow::Result<Self> {
        self.highlight_query = Some(Query::new(&self.ts_language, query)?);
        Ok(self)
    }

    /// Set injection query
    pub fn with_injections(mut self, query: &str) -> anyhow::Result<Self> {
        self.injection_query = Some(Query::new(&self.ts_language, query)?);
        Ok(self)
    }

    /// Set locals query
    pub fn with_locals(mut self, query: &str) -> anyhow::Result<Self> {
        self.locals_query = Some(Query::new(&self.ts_language, query)?);
        Ok(self)
    }

    /// Get tree-sitter language
    pub fn ts_language(&self) -> &TsLanguage {
        &self.ts_language
    }

    /// Parse text
    pub fn parse(&self, text: &str, old_tree: Option<&SyntaxTree>) -> Option<SyntaxTree> {
        let mut parser = Parser::new();
        parser.set_language(&self.ts_language).ok()?;
        
        let tree = parser.parse(text, old_tree.map(|t| t.inner()))?;
        Some(SyntaxTree::new(tree))
    }

    /// Get highlights
    pub fn highlight(
        &self,
        text: &str,
        tree: &SyntaxTree,
        range: std::ops::Range<usize>,
    ) -> Vec<HighlightEvent> {
        let query = match &self.highlight_query {
            Some(q) => q,
            None => return vec![],
        };

        let mut cursor = tree_sitter::QueryCursor::new();
        cursor.set_byte_range(range.clone());

        let mut events = Vec::new();
        let text_bytes = text.as_bytes();

        for match_ in cursor.matches(query, tree.inner().root_node(), text_bytes) {
            for capture in match_.captures {
                let node = capture.node;
                let capture_name = &query.capture_names()[capture.index as usize];
                
                events.push(HighlightEvent {
                    start: node.start_byte(),
                    end: node.end_byte(),
                    scope: capture_name.to_string(),
                });
            }
        }

        // Sort by start position
        events.sort_by_key(|e| e.start);
        events
    }
}

/// Language configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    /// Language name
    pub name: String,
    /// File extensions
    pub extensions: Vec<String>,
    /// Mime types
    #[serde(default)]
    pub mime_types: Vec<String>,
    /// Line comment prefix
    pub line_comment: Option<String>,
    /// Block comment markers
    pub block_comment: Option<(String, String)>,
    /// Auto-pairs
    #[serde(default)]
    pub auto_pairs: Vec<(char, char)>,
    /// Indent settings
    #[serde(default)]
    pub indent: IndentConfig,
    /// Scope for injection
    pub scope: Option<String>,
}

impl Default for LanguageConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            extensions: Vec::new(),
            mime_types: Vec::new(),
            line_comment: Some("//".into()),
            block_comment: Some(("/*".into(), "*/".into())),
            auto_pairs: vec![('(', ')'), ('[', ']'), ('{', '}'), ('"', '"'), ('\'', '\'')],
            indent: IndentConfig::default(),
            scope: None,
        }
    }
}

/// Indent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndentConfig {
    pub tab_size: u32,
    pub use_tabs: bool,
}

impl Default for IndentConfig {
    fn default() -> Self {
        Self {
            tab_size: 4,
            use_tabs: false,
        }
    }
}

/// Built-in language configs
pub mod configs {
    use super::*;

    pub fn rust() -> LanguageConfig {
        LanguageConfig {
            name: "rust".into(),
            extensions: vec!["rs".into()],
            line_comment: Some("//".into()),
            block_comment: Some(("/*".into(), "*/".into())),
            scope: Some("source.rust".into()),
            ..Default::default()
        }
    }

    pub fn typescript() -> LanguageConfig {
        LanguageConfig {
            name: "typescript".into(),
            extensions: vec!["ts".into(), "tsx".into()],
            line_comment: Some("//".into()),
            block_comment: Some(("/*".into(), "*/".into())),
            scope: Some("source.ts".into()),
            ..Default::default()
        }
    }

    pub fn javascript() -> LanguageConfig {
        LanguageConfig {
            name: "javascript".into(),
            extensions: vec!["js".into(), "jsx".into(), "mjs".into()],
            line_comment: Some("//".into()),
            block_comment: Some(("/*".into(), "*/".into())),
            scope: Some("source.js".into()),
            ..Default::default()
        }
    }

    pub fn python() -> LanguageConfig {
        LanguageConfig {
            name: "python".into(),
            extensions: vec!["py".into(), "pyi".into()],
            line_comment: Some("#".into()),
            block_comment: Some(("\"\"\"".into(), "\"\"\"".into())),
            scope: Some("source.python".into()),
            auto_pairs: vec![('(', ')'), ('[', ']'), ('{', '}'), ('"', '"'), ('\'', '\'')],
            ..Default::default()
        }
    }

    pub fn go() -> LanguageConfig {
        LanguageConfig {
            name: "go".into(),
            extensions: vec!["go".into()],
            line_comment: Some("//".into()),
            block_comment: Some(("/*".into(), "*/".into())),
            scope: Some("source.go".into()),
            indent: IndentConfig { tab_size: 4, use_tabs: true },
            ..Default::default()
        }
    }
}
