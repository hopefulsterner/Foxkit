//! Language definitions and grammar support for Foxkit.
//!
//! This crate provides language configuration, grammar loading, and
//! language-specific features for the editor.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Unique identifier for a language.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LanguageId(pub String);

impl LanguageId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Language scope for TextMate grammars.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguageScope(pub String);

/// Configuration for a language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    /// Unique identifier.
    pub id: LanguageId,
    /// Display name.
    pub name: String,
    /// File extensions (e.g., ["rs", "rust"]).
    pub extensions: Vec<String>,
    /// File name patterns.
    pub filenames: Vec<String>,
    /// First line patterns for detection.
    pub first_line_patterns: Vec<String>,
    /// TextMate scope.
    pub scope: Option<String>,
    /// Comment configuration.
    pub comments: Option<CommentConfig>,
    /// Bracket pairs.
    pub brackets: Vec<BracketPair>,
    /// Auto-closing pairs.
    pub auto_closing_pairs: Vec<AutoClosingPair>,
    /// Indentation rules.
    pub indentation: Option<IndentationRules>,
}

/// Comment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentConfig {
    /// Line comment token (e.g., "//").
    pub line: Option<String>,
    /// Block comment tokens (e.g., ["/*", "*/"]).
    pub block: Option<(String, String)>,
}

/// Bracket pair definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BracketPair {
    pub open: String,
    pub close: String,
}

/// Auto-closing pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoClosingPair {
    pub open: String,
    pub close: String,
    /// Contexts where this pair should not auto-close.
    pub not_in: Vec<String>,
}

/// Indentation rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndentationRules {
    /// Pattern that increases indentation.
    pub increase_indent_pattern: Option<String>,
    /// Pattern that decreases indentation.
    pub decrease_indent_pattern: Option<String>,
    /// Pattern for lines that should not be indented.
    pub unindented_line_pattern: Option<String>,
}

/// A loaded language with its grammar.
pub struct Language {
    pub config: LanguageConfig,
    pub grammar: Option<Grammar>,
}

/// Grammar for syntax highlighting.
pub struct Grammar {
    /// Tree-sitter language pointer.
    pub ts_language: tree_sitter::Language,
    /// Highlight queries.
    pub highlights_query: Option<String>,
    /// Injection queries.
    pub injections_query: Option<String>,
    /// Locals queries.
    pub locals_query: Option<String>,
}

/// Language detection result.
#[derive(Debug, Clone)]
pub struct LanguageMatch {
    pub language_id: LanguageId,
    pub confidence: MatchConfidence,
}

/// Confidence level for language detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchConfidence {
    Low,
    Medium,
    High,
    Exact,
}

/// Language registry for managing all languages.
pub struct LanguageRegistry {
    languages: RwLock<HashMap<LanguageId, Arc<Language>>>,
    extension_map: RwLock<HashMap<String, LanguageId>>,
    filename_map: RwLock<HashMap<String, LanguageId>>,
}

impl LanguageRegistry {
    /// Create a new language registry.
    pub fn new() -> Self {
        Self {
            languages: RwLock::new(HashMap::new()),
            extension_map: RwLock::new(HashMap::new()),
            filename_map: RwLock::new(HashMap::new()),
        }
    }

    /// Register a language.
    pub fn register(&self, language: Arc<Language>) {
        let id = language.config.id.clone();
        
        // Register extensions.
        {
            let mut ext_map = self.extension_map.write();
            for ext in &language.config.extensions {
                ext_map.insert(ext.to_lowercase(), id.clone());
            }
        }
        
        // Register filenames.
        {
            let mut name_map = self.filename_map.write();
            for name in &language.config.filenames {
                name_map.insert(name.to_lowercase(), id.clone());
            }
        }
        
        self.languages.write().insert(id, language);
    }

    /// Get a language by ID.
    pub fn get(&self, id: &LanguageId) -> Option<Arc<Language>> {
        self.languages.read().get(id).cloned()
    }

    /// Detect language from file path.
    pub fn detect_from_path(&self, path: &PathBuf) -> Option<LanguageMatch> {
        // Check filename first.
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if let Some(id) = self.filename_map.read().get(&name.to_lowercase()) {
                return Some(LanguageMatch {
                    language_id: id.clone(),
                    confidence: MatchConfidence::Exact,
                });
            }
        }
        
        // Check extension.
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(id) = self.extension_map.read().get(&ext.to_lowercase()) {
                return Some(LanguageMatch {
                    language_id: id.clone(),
                    confidence: MatchConfidence::High,
                });
            }
        }
        
        None
    }

    /// List all registered languages.
    pub fn list(&self) -> Vec<Arc<Language>> {
        self.languages.read().values().cloned().collect()
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}
