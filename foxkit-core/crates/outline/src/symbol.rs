//! Document symbols

use crate::Range;
use serde::{Deserialize, Serialize};

/// Symbol kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum SymbolKind {
    File = 1,
    Module = 2,
    Namespace = 3,
    Package = 4,
    Class = 5,
    Method = 6,
    Property = 7,
    Field = 8,
    Constructor = 9,
    Enum = 10,
    Interface = 11,
    Function = 12,
    Variable = 13,
    Constant = 14,
    String = 15,
    Number = 16,
    Boolean = 17,
    Array = 18,
    Object = 19,
    Key = 20,
    Null = 21,
    EnumMember = 22,
    Struct = 23,
    Event = 24,
    Operator = 25,
    TypeParameter = 26,
}

impl SymbolKind {
    pub fn icon(&self) -> &'static str {
        match self {
            SymbolKind::File => "symbol-file",
            SymbolKind::Module => "symbol-module",
            SymbolKind::Namespace => "symbol-namespace",
            SymbolKind::Package => "package",
            SymbolKind::Class => "symbol-class",
            SymbolKind::Method => "symbol-method",
            SymbolKind::Property => "symbol-property",
            SymbolKind::Field => "symbol-field",
            SymbolKind::Constructor => "symbol-constructor",
            SymbolKind::Enum => "symbol-enum",
            SymbolKind::Interface => "symbol-interface",
            SymbolKind::Function => "symbol-function",
            SymbolKind::Variable => "symbol-variable",
            SymbolKind::Constant => "symbol-constant",
            SymbolKind::String => "symbol-string",
            SymbolKind::Number => "symbol-number",
            SymbolKind::Boolean => "symbol-boolean",
            SymbolKind::Array => "symbol-array",
            SymbolKind::Object => "symbol-object",
            SymbolKind::Key => "symbol-key",
            SymbolKind::Null => "symbol-null",
            SymbolKind::EnumMember => "symbol-enum-member",
            SymbolKind::Struct => "symbol-struct",
            SymbolKind::Event => "symbol-event",
            SymbolKind::Operator => "symbol-operator",
            SymbolKind::TypeParameter => "symbol-type-parameter",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SymbolKind::File => "file",
            SymbolKind::Module => "module",
            SymbolKind::Namespace => "namespace",
            SymbolKind::Package => "package",
            SymbolKind::Class => "class",
            SymbolKind::Method => "method",
            SymbolKind::Property => "property",
            SymbolKind::Field => "field",
            SymbolKind::Constructor => "constructor",
            SymbolKind::Enum => "enum",
            SymbolKind::Interface => "interface",
            SymbolKind::Function => "function",
            SymbolKind::Variable => "variable",
            SymbolKind::Constant => "constant",
            SymbolKind::String => "string",
            SymbolKind::Number => "number",
            SymbolKind::Boolean => "boolean",
            SymbolKind::Array => "array",
            SymbolKind::Object => "object",
            SymbolKind::Key => "key",
            SymbolKind::Null => "null",
            SymbolKind::EnumMember => "enum member",
            SymbolKind::Struct => "struct",
            SymbolKind::Event => "event",
            SymbolKind::Operator => "operator",
            SymbolKind::TypeParameter => "type parameter",
        }
    }
}

/// Symbol tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolTag {
    Deprecated = 1,
}

/// Document symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSymbol {
    /// Symbol name
    pub name: String,
    /// Detail (type info, signature, etc.)
    pub detail: Option<String>,
    /// Symbol kind
    pub kind: SymbolKind,
    /// Tags
    #[serde(default)]
    pub tags: Vec<SymbolTag>,
    /// Full range (including body)
    pub range: Range,
    /// Selection range (name only)
    pub selection_range: Range,
    /// Children
    #[serde(default)]
    pub children: Vec<DocumentSymbol>,
}

impl DocumentSymbol {
    pub fn new(name: &str, kind: SymbolKind, range: Range) -> Self {
        Self {
            name: name.to_string(),
            detail: None,
            kind,
            tags: Vec::new(),
            range,
            selection_range: range,
            children: Vec::new(),
        }
    }

    pub fn with_detail(mut self, detail: &str) -> Self {
        self.detail = Some(detail.to_string());
        self
    }

    pub fn with_selection_range(mut self, range: Range) -> Self {
        self.selection_range = range;
        self
    }

    pub fn with_children(mut self, children: Vec<DocumentSymbol>) -> Self {
        self.children = children;
        self
    }

    pub fn deprecated(mut self) -> Self {
        self.tags.push(SymbolTag::Deprecated);
        self
    }

    /// Is this symbol deprecated?
    pub fn is_deprecated(&self) -> bool {
        self.tags.contains(&SymbolTag::Deprecated)
    }

    /// Get start line
    pub fn start_line(&self) -> u32 {
        self.range.start.line
    }

    /// Get end line
    pub fn end_line(&self) -> u32 {
        self.range.end.line
    }
}

/// Symbol builder
pub struct SymbolBuilder {
    name: String,
    kind: SymbolKind,
    detail: Option<String>,
    range: Option<Range>,
    selection_range: Option<Range>,
    tags: Vec<SymbolTag>,
    children: Vec<DocumentSymbol>,
}

impl SymbolBuilder {
    pub fn new(name: &str, kind: SymbolKind) -> Self {
        Self {
            name: name.to_string(),
            kind,
            detail: None,
            range: None,
            selection_range: None,
            tags: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn function(name: &str) -> Self {
        Self::new(name, SymbolKind::Function)
    }

    pub fn method(name: &str) -> Self {
        Self::new(name, SymbolKind::Method)
    }

    pub fn class(name: &str) -> Self {
        Self::new(name, SymbolKind::Class)
    }

    pub fn struct_(name: &str) -> Self {
        Self::new(name, SymbolKind::Struct)
    }

    pub fn enum_(name: &str) -> Self {
        Self::new(name, SymbolKind::Enum)
    }

    pub fn interface(name: &str) -> Self {
        Self::new(name, SymbolKind::Interface)
    }

    pub fn variable(name: &str) -> Self {
        Self::new(name, SymbolKind::Variable)
    }

    pub fn constant(name: &str) -> Self {
        Self::new(name, SymbolKind::Constant)
    }

    pub fn detail(mut self, detail: &str) -> Self {
        self.detail = Some(detail.to_string());
        self
    }

    pub fn range(mut self, start_line: u32, end_line: u32) -> Self {
        self.range = Some(Range::new(
            crate::Position::new(start_line, 0),
            crate::Position::new(end_line, u32::MAX),
        ));
        self
    }

    pub fn full_range(mut self, range: Range) -> Self {
        self.range = Some(range);
        self
    }

    pub fn selection(mut self, range: Range) -> Self {
        self.selection_range = Some(range);
        self
    }

    pub fn deprecated(mut self) -> Self {
        self.tags.push(SymbolTag::Deprecated);
        self
    }

    pub fn child(mut self, child: DocumentSymbol) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: Vec<DocumentSymbol>) -> Self {
        self.children = children;
        self
    }

    pub fn build(self) -> DocumentSymbol {
        let range = self.range.unwrap_or(Range::at_line(0));
        DocumentSymbol {
            name: self.name,
            detail: self.detail,
            kind: self.kind,
            tags: self.tags,
            range,
            selection_range: self.selection_range.unwrap_or(range),
            children: self.children,
        }
    }
}
