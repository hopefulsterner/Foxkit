//! Query utilities

use tree_sitter::{Query, QueryCursor, QueryMatch};
use crate::SyntaxTree;

/// Query result
#[derive(Debug)]
pub struct QueryResult {
    pub pattern_index: usize,
    pub captures: Vec<QueryCapture>,
}

/// Query capture
#[derive(Debug)]
pub struct QueryCapture {
    pub name: String,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Run a query on a syntax tree
pub fn run_query(
    query: &Query,
    tree: &SyntaxTree,
    source: &str,
    range: Option<std::ops::Range<usize>>,
) -> Vec<QueryResult> {
    let mut cursor = QueryCursor::new();
    
    if let Some(r) = range {
        cursor.set_byte_range(r);
    }

    let source_bytes = source.as_bytes();
    let mut results = Vec::new();

    for match_ in cursor.matches(query, tree.inner().root_node(), source_bytes) {
        let captures: Vec<_> = match_
            .captures
            .iter()
            .map(|c| {
                let name = query.capture_names()[c.index as usize].to_string();
                QueryCapture {
                    name,
                    start: c.node.start_byte(),
                    end: c.node.end_byte(),
                    text: source[c.node.byte_range()].to_string(),
                }
            })
            .collect();

        results.push(QueryResult {
            pattern_index: match_.pattern_index,
            captures,
        });
    }

    results
}

/// Find all definitions in source
pub fn find_definitions(
    query: &Query,
    tree: &SyntaxTree,
    source: &str,
) -> Vec<Definition> {
    let results = run_query(query, tree, source, None);
    
    results
        .into_iter()
        .filter_map(|r| {
            let name_capture = r.captures.iter().find(|c| c.name == "name")?;
            let kind = r.captures.iter()
                .find(|c| c.name.starts_with("definition."))
                .map(|c| c.name.strip_prefix("definition.").unwrap_or(&c.name).to_string())
                .unwrap_or_else(|| "unknown".to_string());
            
            Some(Definition {
                name: name_capture.text.clone(),
                kind,
                start: name_capture.start,
                end: name_capture.end,
            })
        })
        .collect()
}

/// A definition (function, class, etc.)
#[derive(Debug, Clone)]
pub struct Definition {
    pub name: String,
    pub kind: String,
    pub start: usize,
    pub end: usize,
}

/// Find all references to a symbol
pub fn find_references(
    query: &Query,
    tree: &SyntaxTree,
    source: &str,
    symbol: &str,
) -> Vec<Reference> {
    let results = run_query(query, tree, source, None);
    
    results
        .into_iter()
        .filter_map(|r| {
            let ref_capture = r.captures.iter()
                .find(|c| c.name == "reference" && c.text == symbol)?;
            
            Some(Reference {
                start: ref_capture.start,
                end: ref_capture.end,
            })
        })
        .collect()
}

/// A reference to a symbol
#[derive(Debug, Clone)]
pub struct Reference {
    pub start: usize,
    pub end: usize,
}
