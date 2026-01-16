//! # Foxkit Treesitter
//!
//! Syntax tree parsing using tree-sitter.
//!
//! ## Features
//!
//! - Tree-sitter parsing with incremental updates
//! - Query support with file-based loading
//! - Syntax highlighting
//! - Multiple language support
//!
//! ## Query Loading
//!
//! The query loader supports loading `.scm` files from disk:
//!
//! ```no_run
//! use treesitter::{QueryLoader, QueryType};
//!
//! let loader = QueryLoader::with_default_paths();
//! if let Some(query) = loader.load("rust", QueryType::Highlights) {
//!     println!("Loaded: {}", query.source);
//! }
//! ```

pub mod language;
pub mod parser;
pub mod query;
pub mod query_loader;
pub mod highlight;

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

pub use language::Language;
pub use parser::{Parser, Tree, Node};
pub use query::{Query, QueryCapture, QueryMatch};
pub use query_loader::{QueryLoader, QueryType, LoadedQuery};
pub use highlight::{Highlighter, HighlightEvent};

/// Tree-sitter service
pub struct TreeSitterService {
    /// Cached parsers per language
    parsers: RwLock<HashMap<String, Arc<Parser>>>,
    /// Cached trees per file
    trees: RwLock<HashMap<String, Arc<Tree>>>,
    /// Query loader
    query_loader: QueryLoader,
}

impl TreeSitterService {
    pub fn new() -> Self {
        Self {
            parsers: RwLock::new(HashMap::new()),
            trees: RwLock::new(HashMap::new()),
            query_loader: QueryLoader::with_default_paths(),
        }
    }

    /// Create with custom query loader
    pub fn with_query_loader(query_loader: QueryLoader) -> Self {
        Self {
            parsers: RwLock::new(HashMap::new()),
            trees: RwLock::new(HashMap::new()),
            query_loader,
        }
    }

    /// Get the query loader
    pub fn query_loader(&self) -> &QueryLoader {
        &self.query_loader
    }

    /// Get or create parser for language
    pub fn parser(&self, language_id: &str) -> Option<Arc<Parser>> {
        // Check cache
        if let Some(parser) = self.parsers.read().get(language_id) {
            return Some(Arc::clone(parser));
        }

        // Create new parser
        let language = Language::from_id(language_id)?;
        let parser = Arc::new(Parser::new(language));
        
        self.parsers.write().insert(language_id.to_string(), Arc::clone(&parser));
        Some(parser)
    }

    /// Parse source code
    pub fn parse(&self, language_id: &str, source: &str, file_id: &str) -> Option<Arc<Tree>> {
        let parser = self.parser(language_id)?;
        
        // Get old tree for incremental parsing
        let old_tree = self.trees.read().get(file_id).cloned();
        
        let tree = parser.parse(source, old_tree.as_deref())?;
        let tree = Arc::new(tree);
        
        self.trees.write().insert(file_id.to_string(), Arc::clone(&tree));
        Some(tree)
    }

    /// Get cached tree for file
    pub fn get_tree(&self, file_id: &str) -> Option<Arc<Tree>> {
        self.trees.read().get(file_id).cloned()
    }

    /// Invalidate cached tree
    pub fn invalidate(&self, file_id: &str) {
        self.trees.write().remove(file_id);
    }

    /// Query the syntax tree (simplified - returns basic match info)
    pub fn query(
        &self,
        language_id: &str,
        query_source: &str,
        tree: &Tree,
        source: &str,
    ) -> anyhow::Result<usize> {
        let language = Language::from_id(language_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown language: {}", language_id))?;
        
        let query = Query::new(language, query_source)?;
        Ok(query.match_count(tree.root_node(), source))
    }

    /// Get syntax highlighter
    pub fn highlighter(&self, language_id: &str) -> Option<Highlighter> {
        let language = Language::from_id(language_id)?;
        Some(Highlighter::with_loader(language, &self.query_loader))
    }

    /// Get highlighter with fallback to embedded queries only
    pub fn highlighter_embedded(&self, language_id: &str) -> Option<Highlighter> {
        let language = Language::from_id(language_id)?;
        Some(Highlighter::new(language))
    }

    /// Load a query for a language and query type
    pub fn load_query(&self, language_id: &str, query_type: QueryType) -> Option<Arc<LoadedQuery>> {
        self.query_loader.load(language_id, query_type)
    }
}

impl Default for TreeSitterService {
    fn default() -> Self {
        Self::new()
    }
}

/// Syntax node kind
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeKind {
    /// Kind name
    pub name: String,
    /// Is named node?
    pub is_named: bool,
}

/// Syntax error
#[derive(Debug, Clone)]
pub struct SyntaxError {
    /// Error message
    pub message: String,
    /// Start byte offset
    pub start: usize,
    /// End byte offset
    pub end: usize,
    /// Line number
    pub line: usize,
    /// Column number
    pub column: usize,
}

/// Find all syntax errors in tree
pub fn find_errors(tree: &Tree, _source: &str) -> Vec<SyntaxError> {
    let mut errors = Vec::new();
    let mut cursor = tree.walk();
    
    loop {
        let node = cursor.node();
        
        if node.is_error() || node.is_missing() {
            let start = node.start_byte();
            let end = node.end_byte();
            let start_pos = node.start_position();
            
            let message = if node.is_missing() {
                format!("Missing {}", node.kind())
            } else {
                "Syntax error".to_string()
            };

            errors.push(SyntaxError {
                message,
                start,
                end,
                line: start_pos.row,
                column: start_pos.column,
            });
        }

        // Traverse tree
        if cursor.goto_first_child() {
            continue;
        }
        
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return errors;
            }
        }
    }
}

/// Get node text
pub fn node_text<'a>(node: Node<'a>, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}
