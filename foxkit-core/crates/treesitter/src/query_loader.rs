//! Query file loader for Tree-sitter queries
//!
//! Loads `.scm` query files from disk with support for:
//! - Multiple query types (highlights, brackets, indents, etc.)
//! - Inheritance and composition of queries
//! - Caching for performance
//! - Fallback to embedded queries

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
// anyhow is used for error handling in loading

/// Query types supported by the loader
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryType {
    /// Syntax highlighting
    Highlights,
    /// Bracket matching
    Brackets,
    /// Code folding / indentation
    Indents,
    /// Language injections (e.g., regex in strings)
    Injections,
    /// Document outline / symbols
    Outline,
    /// Text objects (word, function, class, etc.)
    TextObjects,
    /// Runnable detection (tests, main, etc.)
    Runnables,
    /// Debugger breakpoints
    Debugger,
    /// Import statements
    Imports,
    /// Override rules
    Overrides,
}

impl QueryType {
    /// Get filename for this query type
    pub fn filename(&self) -> &'static str {
        match self {
            QueryType::Highlights => "highlights.scm",
            QueryType::Brackets => "brackets.scm",
            QueryType::Indents => "indents.scm",
            QueryType::Injections => "injections.scm",
            QueryType::Outline => "outline.scm",
            QueryType::TextObjects => "textobjects.scm",
            QueryType::Runnables => "runnables.scm",
            QueryType::Debugger => "debugger.scm",
            QueryType::Imports => "imports.scm",
            QueryType::Overrides => "overrides.scm",
        }
    }

    /// All query types
    pub fn all() -> &'static [QueryType] {
        &[
            QueryType::Highlights,
            QueryType::Brackets,
            QueryType::Indents,
            QueryType::Injections,
            QueryType::Outline,
            QueryType::TextObjects,
            QueryType::Runnables,
            QueryType::Debugger,
            QueryType::Imports,
            QueryType::Overrides,
        ]
    }
}

/// Loaded query content
#[derive(Debug, Clone)]
pub struct LoadedQuery {
    /// Query source code
    pub source: String,
    /// Where the query was loaded from
    pub path: Option<PathBuf>,
    /// Whether this is a fallback/embedded query
    pub is_fallback: bool,
}

impl LoadedQuery {
    /// Create a new loaded query
    pub fn new(source: String, path: Option<PathBuf>) -> Self {
        Self {
            source,
            path,
            is_fallback: false,
        }
    }

    /// Create a fallback query from embedded source
    pub fn fallback(source: &'static str) -> Self {
        Self {
            source: source.to_string(),
            path: None,
            is_fallback: true,
        }
    }
}

/// Query loader with caching
pub struct QueryLoader {
    /// Search paths for query files
    search_paths: Vec<PathBuf>,
    /// Cache of loaded queries: (language, query_type) -> content
    cache: RwLock<HashMap<(String, QueryType), Arc<LoadedQuery>>>,
}

impl QueryLoader {
    /// Create a new query loader
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Create a loader with default search paths
    pub fn with_default_paths() -> Self {
        let mut loader = Self::new();
        
        // Add common search paths
        if let Some(home) = dirs::home_dir() {
            // User-level query overrides
            loader.add_search_path(home.join(".config/foxkit/queries"));
        }
        
        // Workspace-local queries
        loader.add_search_path(PathBuf::from("queries"));
        
        // Bundled queries (from zed-base)
        loader.add_search_path(PathBuf::from("zed-base/crates/languages/src"));
        
        loader
    }

    /// Add a search path for query files
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }

    /// Set search paths
    pub fn set_search_paths(&mut self, paths: Vec<PathBuf>) {
        self.search_paths = paths;
    }

    /// Get search paths
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }

    /// Load a query for a language
    pub fn load(&self, language: &str, query_type: QueryType) -> Option<Arc<LoadedQuery>> {
        let cache_key = (language.to_string(), query_type);
        
        // Check cache first
        if let Some(cached) = self.cache.read().get(&cache_key) {
            return Some(Arc::clone(cached));
        }

        // Try to load from disk
        let query = self.load_from_disk(language, query_type)
            .or_else(|| self.get_fallback(language, query_type));

        // Cache the result
        if let Some(ref q) = query {
            self.cache.write().insert(cache_key, Arc::clone(q));
        }

        query
    }

    /// Load query from disk
    fn load_from_disk(&self, language: &str, query_type: QueryType) -> Option<Arc<LoadedQuery>> {
        for base_path in &self.search_paths {
            // Try language-specific directory
            let path = base_path.join(language).join(query_type.filename());
            
            if let Ok(content) = std::fs::read_to_string(&path) {
                tracing::debug!("Loaded query from {:?}", path);
                return Some(Arc::new(LoadedQuery::new(content, Some(path))));
            }

            // Try alternate naming (e.g., tree-sitter-rust)
            let alt_path = base_path
                .join(format!("tree-sitter-{}", language))
                .join("queries")
                .join(query_type.filename());
            
            if let Ok(content) = std::fs::read_to_string(&alt_path) {
                tracing::debug!("Loaded query from {:?}", alt_path);
                return Some(Arc::new(LoadedQuery::new(content, Some(alt_path))));
            }
        }

        None
    }

    /// Get fallback/embedded query
    fn get_fallback(&self, language: &str, query_type: QueryType) -> Option<Arc<LoadedQuery>> {
        // Only provide fallbacks for highlights (the most common need)
        if query_type != QueryType::Highlights {
            return None;
        }

        let source = match language {
            "rust" => Some(RUST_HIGHLIGHTS_FALLBACK),
            "javascript" | "typescript" | "tsx" | "jsx" => Some(JS_HIGHLIGHTS_FALLBACK),
            "python" => Some(PYTHON_HIGHLIGHTS_FALLBACK),
            "json" => Some(JSON_HIGHLIGHTS_FALLBACK),
            "toml" => Some(TOML_HIGHLIGHTS_FALLBACK),
            "markdown" | "md" => Some(MARKDOWN_HIGHLIGHTS_FALLBACK),
            _ => None,
        }?;

        tracing::debug!("Using fallback query for {} {}", language, query_type.filename());
        Some(Arc::new(LoadedQuery::fallback(source)))
    }

    /// Load all queries for a language
    pub fn load_all(&self, language: &str) -> HashMap<QueryType, Arc<LoadedQuery>> {
        let mut queries = HashMap::new();
        
        for query_type in QueryType::all() {
            if let Some(query) = self.load(language, *query_type) {
                queries.insert(*query_type, query);
            }
        }
        
        queries
    }

    /// Reload a specific query (bypasses cache)
    pub fn reload(&self, language: &str, query_type: QueryType) -> Option<Arc<LoadedQuery>> {
        let cache_key = (language.to_string(), query_type);
        
        // Remove from cache
        self.cache.write().remove(&cache_key);
        
        // Load fresh
        self.load(language, query_type)
    }

    /// Get list of available languages
    pub fn available_languages(&self) -> Vec<String> {
        let mut languages = std::collections::HashSet::new();
        
        for base_path in &self.search_paths {
            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let name = entry.file_name().to_string_lossy().to_string();
                        // Skip non-language directories
                        if !name.starts_with('.') && !name.contains("test") {
                            languages.insert(name);
                        }
                    }
                }
            }
        }
        
        let mut result: Vec<_> = languages.into_iter().collect();
        result.sort();
        result
    }
}

impl Default for QueryLoader {
    fn default() -> Self {
        Self::with_default_paths()
    }
}

// Fallback queries for common languages

const RUST_HIGHLIGHTS_FALLBACK: &str = r#"
(line_comment) @comment
(block_comment) @comment

; Keywords
["as" "async" "await" "break" "const" "continue" "else" "enum" "extern"
 "fn" "for" "if" "impl" "in" "let" "loop" "match" "mod" "move" "mut"
 "pub" "ref" "return" "self" "static" "struct" "trait" "type" "unsafe"
 "use" "where" "while"] @keyword

; Literals
(string_literal) @string
(raw_string_literal) @string
(char_literal) @string
(integer_literal) @number
(float_literal) @number
(boolean_literal) @constant.builtin

; Types
(type_identifier) @type
(primitive_type) @type.builtin

; Identifiers
(identifier) @variable
(field_identifier) @property

; Functions
(function_item name: (identifier) @function.definition)
(call_expression function: (identifier) @function)
"#;

const JS_HIGHLIGHTS_FALLBACK: &str = r#"
(comment) @comment

; Keywords
["as" "async" "await" "break" "case" "catch" "class" "const" "continue"
 "default" "delete" "do" "else" "export" "extends" "finally" "for"
 "from" "function" "if" "import" "in" "instanceof" "let" "new" "of"
 "return" "static" "switch" "throw" "try" "typeof" "var" "while"
 "with" "yield"] @keyword

; Literals
(string) @string
(template_string) @string
(number) @number
(true) @constant.builtin
(false) @constant.builtin
(null) @constant.builtin
(undefined) @constant.builtin

; Identifiers
(identifier) @variable
(property_identifier) @property

; Functions
(function_declaration name: (identifier) @function.definition)
(call_expression function: (identifier) @function)
"#;

const PYTHON_HIGHLIGHTS_FALLBACK: &str = r#"
(comment) @comment

; Keywords
["and" "as" "assert" "async" "await" "break" "class" "continue" "def"
 "del" "elif" "else" "except" "finally" "for" "from" "global" "if"
 "import" "in" "is" "lambda" "not" "or" "pass" "raise" "return" "try"
 "while" "with" "yield"] @keyword

; Literals
(string) @string
(integer) @number
(float) @number
(true) @constant.builtin
(false) @constant.builtin
(none) @constant.builtin

; Identifiers
(identifier) @variable
(attribute) @property

; Functions
(function_definition name: (identifier) @function.definition)
(call function: (identifier) @function)
"#;

const JSON_HIGHLIGHTS_FALLBACK: &str = r#"
(string) @string
(number) @number
(true) @constant.builtin
(false) @constant.builtin
(null) @constant.builtin
(pair key: (string) @property)
"#;

const TOML_HIGHLIGHTS_FALLBACK: &str = r#"
(comment) @comment
(string) @string
(integer) @number
(float) @number
(boolean) @constant.builtin
(bare_key) @property
(quoted_key) @property
(table (bare_key) @type)
(table_array_element (bare_key) @type)
"#;

const MARKDOWN_HIGHLIGHTS_FALLBACK: &str = r#"
(atx_heading (atx_h1_marker)) @keyword
(atx_heading (atx_h2_marker)) @keyword
(atx_heading (atx_h3_marker)) @keyword
(atx_heading (heading_content) @title)

(emphasis) @emphasis
(strong_emphasis) @strong

(code_span) @string
(fenced_code_block) @string

(link (link_text) @string)
(link (link_destination) @link)

(list_marker_minus) @punctuation
(list_marker_plus) @punctuation
(list_marker_star) @punctuation
(list_marker_dot) @punctuation
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_type_filename() {
        assert_eq!(QueryType::Highlights.filename(), "highlights.scm");
        assert_eq!(QueryType::Brackets.filename(), "brackets.scm");
    }

    #[test]
    fn test_query_loader_fallback() {
        let loader = QueryLoader::new();
        
        // Should get fallback for rust highlights
        let query = loader.load("rust", QueryType::Highlights);
        assert!(query.is_some());
        assert!(query.unwrap().is_fallback);
    }

    #[test]
    fn test_cache() {
        let loader = QueryLoader::new();
        
        // First load
        let q1 = loader.load("rust", QueryType::Highlights);
        // Second load should hit cache
        let q2 = loader.load("rust", QueryType::Highlights);
        
        assert!(Arc::ptr_eq(&q1.unwrap(), &q2.unwrap()));
    }
}
