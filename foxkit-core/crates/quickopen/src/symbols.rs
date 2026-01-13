//! Symbol picker

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::{QuickPickItem, FuzzyMatcher};

/// Symbol kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

impl SymbolKind {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::File => "📄",
            Self::Module => "📦",
            Self::Namespace => "📁",
            Self::Package => "📦",
            Self::Class => "🔷",
            Self::Method => "🔹",
            Self::Property => "🔸",
            Self::Field => "🔸",
            Self::Constructor => "🔨",
            Self::Enum => "📊",
            Self::Interface => "🔷",
            Self::Function => "ƒ",
            Self::Variable => "𝑥",
            Self::Constant => "π",
            Self::String => "\"",
            Self::Number => "#",
            Self::Boolean => "◉",
            Self::Array => "[]",
            Self::Object => "{}",
            Self::Key => "🔑",
            Self::Null => "∅",
            Self::EnumMember => "•",
            Self::Struct => "🔶",
            Self::Event => "⚡",
            Self::Operator => "±",
            Self::TypeParameter => "T",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Module => "module",
            Self::Namespace => "namespace",
            Self::Package => "package",
            Self::Class => "class",
            Self::Method => "method",
            Self::Property => "property",
            Self::Field => "field",
            Self::Constructor => "constructor",
            Self::Enum => "enum",
            Self::Interface => "interface",
            Self::Function => "function",
            Self::Variable => "variable",
            Self::Constant => "constant",
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Array => "array",
            Self::Object => "object",
            Self::Key => "key",
            Self::Null => "null",
            Self::EnumMember => "enum member",
            Self::Struct => "struct",
            Self::Event => "event",
            Self::Operator => "operator",
            Self::TypeParameter => "type parameter",
        }
    }
}

/// Document symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSymbol {
    pub name: String,
    pub detail: Option<String>,
    pub kind: SymbolKind,
    pub range: SymbolRange,
    pub selection_range: SymbolRange,
    pub children: Vec<DocumentSymbol>,
}

impl DocumentSymbol {
    /// Flatten to list
    pub fn flatten(&self) -> Vec<FlatSymbol> {
        let mut result = vec![FlatSymbol {
            name: self.name.clone(),
            detail: self.detail.clone(),
            kind: self.kind,
            range: self.range.clone(),
            container: None,
        }];

        for child in &self.children {
            let mut child_symbols = child.flatten();
            for sym in &mut child_symbols {
                if sym.container.is_none() {
                    sym.container = Some(self.name.clone());
                }
            }
            result.extend(child_symbols);
        }

        result
    }
}

/// Flattened symbol
#[derive(Debug, Clone)]
pub struct FlatSymbol {
    pub name: String,
    pub detail: Option<String>,
    pub kind: SymbolKind,
    pub range: SymbolRange,
    pub container: Option<String>,
}

impl FlatSymbol {
    /// Convert to quick pick item
    pub fn to_quick_pick_item(&self) -> QuickPickItem {
        let label = format!("{} {}", self.kind.icon(), self.name);
        let mut item = QuickPickItem::new(label);
        
        if let Some(ref container) = self.container {
            item = item.with_description(container.clone());
        }
        
        if let Some(ref detail) = self.detail {
            item = item.with_detail(detail.clone());
        }
        
        item
    }
}

/// Symbol range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// Workspace symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: SymbolLocation,
    pub container_name: Option<String>,
}

impl WorkspaceSymbol {
    /// Convert to quick pick item
    pub fn to_quick_pick_item(&self) -> QuickPickItem {
        let label = format!("{} {}", self.kind.icon(), self.name);
        let mut item = QuickPickItem::new(label);
        
        let description = self.location.path.to_string_lossy();
        item = item.with_description(description);
        
        if let Some(ref container) = self.container_name {
            item = item.with_detail(format!("in {}", container));
        }
        
        item
    }
}

/// Symbol location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolLocation {
    pub path: PathBuf,
    pub range: SymbolRange,
}

/// Symbol picker
pub struct SymbolPicker {
    /// Document symbols
    symbols: Vec<FlatSymbol>,
    /// Fuzzy matcher
    matcher: FuzzyMatcher,
    /// Filter by kind
    kind_filter: Option<SymbolKind>,
}

impl SymbolPicker {
    pub fn new(symbols: Vec<DocumentSymbol>) -> Self {
        let flat: Vec<_> = symbols.iter()
            .flat_map(|s| s.flatten())
            .collect();
        
        Self {
            symbols: flat,
            matcher: FuzzyMatcher::new(),
            kind_filter: None,
        }
    }

    /// Set kind filter
    pub fn filter_kind(&mut self, kind: SymbolKind) {
        self.kind_filter = Some(kind);
    }

    /// Clear kind filter
    pub fn clear_kind_filter(&mut self) {
        self.kind_filter = None;
    }

    /// Filter symbols
    pub fn filter(&self, query: &str) -> Vec<(&FlatSymbol, i64)> {
        let query = query.strip_prefix('@').unwrap_or(query).trim();
        
        let mut results: Vec<_> = self.symbols.iter()
            .filter(|s| {
                if let Some(kind) = self.kind_filter {
                    s.kind == kind
                } else {
                    true
                }
            })
            .filter_map(|s| {
                if query.is_empty() {
                    Some((s, 0))
                } else {
                    self.matcher.score(&s.name, query).map(|score| (s, score))
                }
            })
            .collect();

        // Sort by score
        results.sort_by(|a, b| b.1.cmp(&a.1));

        results
    }

    /// Get as quick pick items
    pub fn as_quick_pick_items(&self, query: &str) -> Vec<QuickPickItem> {
        self.filter(query)
            .into_iter()
            .map(|(s, _)| s.to_quick_pick_item())
            .collect()
    }
}
