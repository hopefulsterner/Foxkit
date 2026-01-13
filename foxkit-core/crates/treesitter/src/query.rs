//! Query support

use crate::{Language, Node};

/// Tree-sitter query
pub struct Query {
    inner: tree_sitter::Query,
}

impl Query {
    /// Create a new query
    pub fn new(language: Language, source: &str) -> anyhow::Result<Self> {
        let inner = tree_sitter::Query::new(&language.ts_language(), source)
            .map_err(|e| anyhow::anyhow!("Query error: {:?}", e))?;
        Ok(Self { inner })
    }

    /// Get capture names
    pub fn capture_names(&self) -> &[&str] {
        self.inner.capture_names()
    }

    /// Execute query and get matches
    pub fn matches<'a>(&'a self, node: Node<'a>, source: &'a str) -> Vec<QueryMatch<'a>> {
        let mut cursor = tree_sitter::QueryCursor::new();
        let matches = cursor.matches(&self.inner, node.inner, source.as_bytes());
        
        matches.map(|m| QueryMatch {
            pattern_index: m.pattern_index,
            captures: m.captures.iter().map(|c| QueryCapture {
                node: Node { inner: c.node },
                index: c.index as usize,
                name: self.inner.capture_names()[c.index as usize],
            }).collect(),
        }).collect()
    }

    /// Execute query and get captures
    pub fn captures<'a>(&'a self, node: Node<'a>, source: &'a str) -> Vec<QueryCapture<'a>> {
        let mut cursor = tree_sitter::QueryCursor::new();
        let captures = cursor.captures(&self.inner, node.inner, source.as_bytes());
        
        captures.flat_map(|(m, _)| {
            m.captures.iter().map(|c| QueryCapture {
                node: Node { inner: c.node },
                index: c.index as usize,
                name: self.inner.capture_names()[c.index as usize],
            })
        }).collect()
    }
}

/// Query match
#[derive(Debug)]
pub struct QueryMatch<'a> {
    /// Pattern index
    pub pattern_index: usize,
    /// Captures
    pub captures: Vec<QueryCapture<'a>>,
}

/// Query capture
#[derive(Debug)]
pub struct QueryCapture<'a> {
    /// Captured node
    pub node: Node<'a>,
    /// Capture index
    pub index: usize,
    /// Capture name
    pub name: &'a str,
}

/// Common queries
pub mod queries {
    /// Get function definitions query for language
    pub fn functions(language: &str) -> Option<&'static str> {
        match language {
            "rust" => Some(r#"
                (function_item
                    name: (identifier) @name) @function
            "#),
            "javascript" | "typescript" => Some(r#"
                (function_declaration
                    name: (identifier) @name) @function
                (arrow_function) @function
            "#),
            "python" => Some(r#"
                (function_definition
                    name: (identifier) @name) @function
            "#),
            _ => None,
        }
    }

    /// Get class definitions query for language
    pub fn classes(language: &str) -> Option<&'static str> {
        match language {
            "rust" => Some(r#"
                (struct_item
                    name: (type_identifier) @name) @class
                (impl_item
                    type: (type_identifier) @name) @class
            "#),
            "javascript" | "typescript" => Some(r#"
                (class_declaration
                    name: (identifier) @name) @class
            "#),
            "python" => Some(r#"
                (class_definition
                    name: (identifier) @name) @class
            "#),
            _ => None,
        }
    }

    /// Get imports query for language
    pub fn imports(language: &str) -> Option<&'static str> {
        match language {
            "rust" => Some(r#"
                (use_declaration) @import
            "#),
            "javascript" | "typescript" => Some(r#"
                (import_statement) @import
            "#),
            "python" => Some(r#"
                (import_statement) @import
                (import_from_statement) @import
            "#),
            _ => None,
        }
    }

    /// Get comments query for language
    pub fn comments(language: &str) -> Option<&'static str> {
        match language {
            "rust" => Some(r#"
                (line_comment) @comment
                (block_comment) @comment
            "#),
            "javascript" | "typescript" => Some(r#"
                (comment) @comment
            "#),
            "python" => Some(r#"
                (comment) @comment
            "#),
            _ => None,
        }
    }
}
