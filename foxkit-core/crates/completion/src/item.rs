//! Completion item

use serde::{Deserialize, Serialize};

/// Completion item kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum CompletionKind {
    Text = 1,
    Method = 2,
    Function = 3,
    Constructor = 4,
    Field = 5,
    Variable = 6,
    Class = 7,
    Interface = 8,
    Module = 9,
    Property = 10,
    Unit = 11,
    Value = 12,
    Enum = 13,
    Keyword = 14,
    Snippet = 15,
    Color = 16,
    File = 17,
    Reference = 18,
    Folder = 19,
    EnumMember = 20,
    Constant = 21,
    Struct = 22,
    Event = 23,
    Operator = 24,
    TypeParameter = 25,
}

impl CompletionKind {
    pub fn icon(&self) -> &'static str {
        match self {
            CompletionKind::Text => "symbol-text",
            CompletionKind::Method => "symbol-method",
            CompletionKind::Function => "symbol-function",
            CompletionKind::Constructor => "symbol-constructor",
            CompletionKind::Field => "symbol-field",
            CompletionKind::Variable => "symbol-variable",
            CompletionKind::Class => "symbol-class",
            CompletionKind::Interface => "symbol-interface",
            CompletionKind::Module => "symbol-module",
            CompletionKind::Property => "symbol-property",
            CompletionKind::Unit => "symbol-unit",
            CompletionKind::Value => "symbol-value",
            CompletionKind::Enum => "symbol-enum",
            CompletionKind::Keyword => "symbol-keyword",
            CompletionKind::Snippet => "symbol-snippet",
            CompletionKind::Color => "symbol-color",
            CompletionKind::File => "symbol-file",
            CompletionKind::Reference => "symbol-reference",
            CompletionKind::Folder => "folder",
            CompletionKind::EnumMember => "symbol-enum-member",
            CompletionKind::Constant => "symbol-constant",
            CompletionKind::Struct => "symbol-struct",
            CompletionKind::Event => "symbol-event",
            CompletionKind::Operator => "symbol-operator",
            CompletionKind::TypeParameter => "symbol-type-parameter",
        }
    }
}

/// Completion item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    /// Display label
    pub label: String,
    /// Kind
    pub kind: CompletionKind,
    /// Detail (type info, etc.)
    pub detail: Option<String>,
    /// Documentation
    pub documentation: Option<Documentation>,
    /// Deprecated
    pub deprecated: bool,
    /// Preselect
    pub preselect: bool,
    /// Sort text (for ordering)
    pub sort_text: Option<String>,
    /// Filter text (for matching)
    pub filter_text: Option<String>,
    /// Insert text (what gets inserted)
    pub insert_text: Option<String>,
    /// Insert text format
    pub insert_text_format: InsertTextFormat,
    /// Text edit (alternative to insert_text)
    pub text_edit: Option<TextEdit>,
    /// Additional text edits
    #[serde(default)]
    pub additional_edits: Vec<TextEdit>,
    /// Commit characters
    #[serde(default)]
    pub commit_characters: Vec<char>,
    /// Command to execute after completion
    pub command: Option<String>,
    /// Custom data (for resolve)
    pub data: Option<serde_json::Value>,
}

impl CompletionItem {
    pub fn simple(label: &str) -> Self {
        Self {
            label: label.to_string(),
            kind: CompletionKind::Text,
            detail: None,
            documentation: None,
            deprecated: false,
            preselect: false,
            sort_text: None,
            filter_text: None,
            insert_text: None,
            insert_text_format: InsertTextFormat::PlainText,
            text_edit: None,
            additional_edits: Vec::new(),
            commit_characters: Vec::new(),
            command: None,
            data: None,
        }
    }

    pub fn new(label: &str, kind: CompletionKind) -> Self {
        Self {
            label: label.to_string(),
            kind,
            ..Self::simple(label)
        }
    }

    pub fn keyword(label: &str) -> Self {
        Self::new(label, CompletionKind::Keyword)
    }

    pub fn function(label: &str) -> Self {
        Self::new(label, CompletionKind::Function)
    }

    pub fn method(label: &str) -> Self {
        Self::new(label, CompletionKind::Method)
    }

    pub fn variable(label: &str) -> Self {
        Self::new(label, CompletionKind::Variable)
    }

    pub fn snippet(label: &str, body: &str) -> Self {
        Self {
            label: label.to_string(),
            kind: CompletionKind::Snippet,
            insert_text: Some(body.to_string()),
            insert_text_format: InsertTextFormat::Snippet,
            ..Self::simple(label)
        }
    }

    pub fn with_detail(mut self, detail: &str) -> Self {
        self.detail = Some(detail.to_string());
        self
    }

    pub fn with_documentation(mut self, doc: &str) -> Self {
        self.documentation = Some(Documentation::String(doc.to_string()));
        self
    }

    pub fn with_markdown(mut self, markdown: &str) -> Self {
        self.documentation = Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown.to_string(),
        }));
        self
    }

    pub fn with_insert_text(mut self, text: &str) -> Self {
        self.insert_text = Some(text.to_string());
        self
    }

    pub fn with_sort_text(mut self, text: &str) -> Self {
        self.sort_text = Some(text.to_string());
        self
    }

    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    pub fn preselect(mut self) -> Self {
        self.preselect = true;
        self
    }

    /// Get the text to insert
    pub fn insert(&self) -> &str {
        self.insert_text.as_deref().unwrap_or(&self.label)
    }
}

/// Insert text format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum InsertTextFormat {
    #[default]
    PlainText = 1,
    Snippet = 2,
}

/// Documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Documentation {
    String(String),
    MarkupContent(MarkupContent),
}

/// Markup content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkupContent {
    pub kind: MarkupKind,
    pub value: String,
}

/// Markup kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkupKind {
    #[serde(rename = "plaintext")]
    PlainText,
    #[serde(rename = "markdown")]
    Markdown,
}

/// Text edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

/// Range
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Position
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Completion detail (extended info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionDetail {
    /// Type signature
    pub type_signature: Option<String>,
    /// Parameter info
    pub parameters: Vec<ParameterInfo>,
    /// Return type
    pub return_type: Option<String>,
    /// Source module/file
    pub source: Option<String>,
}

/// Parameter info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub type_name: Option<String>,
    pub documentation: Option<String>,
}
