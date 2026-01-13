//! # Foxkit Outline
//!
//! Document outline and symbols (LSP-compatible).

pub mod symbol;
pub mod provider;

use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub use symbol::{DocumentSymbol, SymbolKind, SymbolTag};
pub use provider::OutlineProvider;

/// Position in document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// Range in document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn at_line(line: u32) -> Self {
        Self {
            start: Position::new(line, 0),
            end: Position::new(line, u32::MAX),
        }
    }
}

/// Document outline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outline {
    /// File path
    pub file_path: String,
    /// Symbols
    pub symbols: Vec<DocumentSymbol>,
}

impl Outline {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            symbols: Vec::new(),
        }
    }

    pub fn with_symbols(mut self, symbols: Vec<DocumentSymbol>) -> Self {
        self.symbols = symbols;
        self
    }

    /// Flatten all symbols (including children)
    pub fn flatten(&self) -> Vec<&DocumentSymbol> {
        fn collect<'a>(symbols: &'a [DocumentSymbol], result: &mut Vec<&'a DocumentSymbol>) {
            for symbol in symbols {
                result.push(symbol);
                collect(&symbol.children, result);
            }
        }
        
        let mut result = Vec::new();
        collect(&self.symbols, &mut result);
        result
    }

    /// Find symbol at position
    pub fn symbol_at(&self, position: Position) -> Option<&DocumentSymbol> {
        fn find_at<'a>(symbols: &'a [DocumentSymbol], pos: Position) -> Option<&'a DocumentSymbol> {
            for symbol in symbols {
                if contains_position(&symbol.range, pos) {
                    // Check children first (more specific)
                    if let Some(child) = find_at(&symbol.children, pos) {
                        return Some(child);
                    }
                    return Some(symbol);
                }
            }
            None
        }
        
        find_at(&self.symbols, position)
    }

    /// Filter by kind
    pub fn filter_by_kind(&self, kind: SymbolKind) -> Vec<&DocumentSymbol> {
        self.flatten().into_iter().filter(|s| s.kind == kind).collect()
    }

    /// Get all functions
    pub fn functions(&self) -> Vec<&DocumentSymbol> {
        self.flatten()
            .into_iter()
            .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
            .collect()
    }

    /// Get all classes/structs
    pub fn classes(&self) -> Vec<&DocumentSymbol> {
        self.flatten()
            .into_iter()
            .filter(|s| matches!(s.kind, SymbolKind::Class | SymbolKind::Struct | SymbolKind::Interface))
            .collect()
    }
}

fn contains_position(range: &Range, pos: Position) -> bool {
    if pos.line < range.start.line || pos.line > range.end.line {
        return false;
    }
    if pos.line == range.start.line && pos.character < range.start.character {
        return false;
    }
    if pos.line == range.end.line && pos.character > range.end.character {
        return false;
    }
    true
}

/// Outline request
#[derive(Debug, Clone)]
pub struct OutlineRequest {
    pub file_path: String,
    pub content: String,
    pub language_id: String,
}

impl OutlineRequest {
    pub fn new(file_path: &str, content: &str, language_id: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            content: content.to_string(),
            language_id: language_id.to_string(),
        }
    }
}

/// Outline service
pub struct OutlineService {
    providers: RwLock<Vec<Arc<dyn OutlineProvider>>>,
}

impl OutlineService {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(Vec::new()),
        }
    }

    /// Register a provider
    pub fn register(&self, provider: Arc<dyn OutlineProvider>) {
        self.providers.write().push(provider);
    }

    /// Get outline for document
    pub fn outline(&self, request: &OutlineRequest) -> Option<Outline> {
        let providers = self.providers.read();
        
        for provider in providers.iter() {
            if provider.supports(&request.language_id) {
                if let Some(outline) = provider.provide(request) {
                    return Some(outline);
                }
            }
        }
        
        None
    }
}

impl Default for OutlineService {
    fn default() -> Self {
        Self::new()
    }
}

/// Breadcrumb item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreadcrumbItem {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
}

/// Get breadcrumbs for position
pub fn breadcrumbs(outline: &Outline, position: Position) -> Vec<BreadcrumbItem> {
    fn find_path(
        symbols: &[DocumentSymbol],
        pos: Position,
        path: &mut Vec<BreadcrumbItem>,
    ) -> bool {
        for symbol in symbols {
            if contains_position(&symbol.range, pos) {
                path.push(BreadcrumbItem {
                    name: symbol.name.clone(),
                    kind: symbol.kind,
                    range: symbol.range,
                });
                
                // Search children
                find_path(&symbol.children, pos, path);
                return true;
            }
        }
        false
    }
    
    let mut path = Vec::new();
    find_path(&outline.symbols, position, &mut path);
    path
}
