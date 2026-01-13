//! # Foxkit Syntax
//!
//! Tree-sitter based syntax highlighting and code analysis.

pub mod highlight;
pub mod language;
pub mod query;
pub mod tree;

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

pub use highlight::{Highlight, HighlightEvent};
pub use language::{Language, LanguageConfig};
pub use tree::{SyntaxTree, SyntaxNode};

/// Language registry
pub struct LanguageRegistry {
    languages: HashMap<String, Arc<Language>>,
    by_extension: HashMap<String, String>,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        Self {
            languages: HashMap::new(),
            by_extension: HashMap::new(),
        }
    }

    /// Register a language
    pub fn register(&mut self, language: Language) {
        let name = language.config.name.clone();
        
        // Register extensions
        for ext in &language.config.extensions {
            self.by_extension.insert(ext.clone(), name.clone());
        }
        
        self.languages.insert(name, Arc::new(language));
    }

    /// Get language by name
    pub fn get(&self, name: &str) -> Option<Arc<Language>> {
        self.languages.get(name).cloned()
    }

    /// Get language by file extension
    pub fn for_extension(&self, ext: &str) -> Option<Arc<Language>> {
        let name = self.by_extension.get(ext)?;
        self.get(name)
    }

    /// Get language by file path
    pub fn for_path(&self, path: &std::path::Path) -> Option<Arc<Language>> {
        let ext = path.extension()?.to_str()?;
        self.for_extension(ext)
    }

    /// List all language names
    pub fn list(&self) -> Vec<&str> {
        self.languages.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Syntax highlighter
pub struct SyntaxHighlighter {
    language: Arc<Language>,
    tree: Option<SyntaxTree>,
}

impl SyntaxHighlighter {
    pub fn new(language: Arc<Language>) -> Self {
        Self {
            language,
            tree: None,
        }
    }

    /// Parse text and update syntax tree
    pub fn parse(&mut self, text: &str) {
        self.tree = self.language.parse(text, self.tree.as_ref());
    }

    /// Parse incrementally after edit
    pub fn parse_edit(&mut self, text: &str, edit: tree_sitter::InputEdit) {
        if let Some(tree) = &mut self.tree {
            tree.edit(&edit);
        }
        self.tree = self.language.parse(text, self.tree.as_ref());
    }

    /// Get highlights for a range
    pub fn highlights(&self, text: &str, range: std::ops::Range<usize>) -> Vec<HighlightEvent> {
        let tree = match &self.tree {
            Some(t) => t,
            None => return vec![],
        };

        self.language.highlight(text, tree, range)
    }

    /// Get syntax tree
    pub fn tree(&self) -> Option<&SyntaxTree> {
        self.tree.as_ref()
    }

    /// Get language
    pub fn language(&self) -> &Language {
        &self.language
    }
}

/// Global shared registry
pub static LANGUAGES: once_cell::sync::Lazy<RwLock<LanguageRegistry>> =
    once_cell::sync::Lazy::new(|| RwLock::new(LanguageRegistry::new()));
