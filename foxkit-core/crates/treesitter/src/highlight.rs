//! Syntax highlighting

use crate::{Language, Node, Tree};

/// Syntax highlighter
pub struct Highlighter {
    language: Language,
    query: Option<tree_sitter::Query>,
}

impl Highlighter {
    /// Create a new highlighter
    pub fn new(language: Language) -> Self {
        let query_source = Self::highlight_query(&language);
        let query = query_source.and_then(|q| {
            tree_sitter::Query::new(&language.ts_language(), q).ok()
        });

        Self { language, query }
    }

    /// Highlight source code
    pub fn highlight<'a>(&'a self, tree: &'a Tree, source: &'a str) -> Vec<HighlightEvent> {
        let Some(ref query) = self.query else {
            return vec![];
        };

        let mut cursor = tree_sitter::QueryCursor::new();
        let captures = cursor.captures(query, tree.root_node().inner, source.as_bytes());
        
        let capture_names = query.capture_names();
        
        captures.flat_map(|(m, _)| {
            m.captures.iter().map(|c| {
                let scope = capture_names[c.index as usize];
                HighlightEvent {
                    start: c.node.start_byte(),
                    end: c.node.end_byte(),
                    scope: scope.to_string(),
                }
            })
        }).collect()
    }

    /// Get highlight query for language
    fn highlight_query(language: &Language) -> Option<&'static str> {
        match language {
            Language::Rust => Some(RUST_HIGHLIGHTS),
            Language::JavaScript | Language::TypeScript | Language::Tsx => Some(JS_HIGHLIGHTS),
            Language::Python => Some(PYTHON_HIGHLIGHTS),
            Language::Json => Some(JSON_HIGHLIGHTS),
            _ => None,
        }
    }
}

/// Highlight event
#[derive(Debug, Clone)]
pub struct HighlightEvent {
    /// Start byte
    pub start: usize,
    /// End byte
    pub end: usize,
    /// Scope name
    pub scope: String,
}

impl HighlightEvent {
    /// Convert scope to theme key
    pub fn theme_key(&self) -> &str {
        // Map tree-sitter scope to TextMate-like scope
        match self.scope.as_str() {
            "keyword" | "keyword.control" | "keyword.function" => "keyword",
            "string" | "string.special" => "string",
            "comment" => "comment",
            "function" | "function.method" => "entity.name.function",
            "type" | "type.builtin" => "entity.name.type",
            "variable" | "variable.parameter" => "variable",
            "constant" | "constant.builtin" => "constant",
            "number" => "constant.numeric",
            "operator" => "keyword.operator",
            "punctuation" => "punctuation",
            "property" => "variable.other.property",
            "attribute" => "entity.other.attribute-name",
            other => other,
        }
    }
}

// Highlight queries for supported languages

const RUST_HIGHLIGHTS: &str = r#"
(line_comment) @comment
(block_comment) @comment

"as" @keyword
"async" @keyword
"await" @keyword
"break" @keyword
"const" @keyword
"continue" @keyword
"else" @keyword
"enum" @keyword
"extern" @keyword
"fn" @keyword.function
"for" @keyword
"if" @keyword
"impl" @keyword
"in" @keyword
"let" @keyword
"loop" @keyword
"match" @keyword
"mod" @keyword
"move" @keyword
"mut" @keyword
"pub" @keyword
"ref" @keyword
"return" @keyword
"self" @keyword
"static" @keyword
"struct" @keyword
"trait" @keyword
"type" @keyword
"unsafe" @keyword
"use" @keyword
"where" @keyword
"while" @keyword

(string_literal) @string
(raw_string_literal) @string
(char_literal) @string

(integer_literal) @number
(float_literal) @number

(boolean_literal) @constant.builtin

(type_identifier) @type
(primitive_type) @type.builtin

(identifier) @variable
(field_identifier) @property

(function_item name: (identifier) @function)
(call_expression function: (identifier) @function)
"#;

const JS_HIGHLIGHTS: &str = r#"
(comment) @comment

"as" @keyword
"async" @keyword
"await" @keyword
"break" @keyword
"case" @keyword
"catch" @keyword
"class" @keyword
"const" @keyword
"continue" @keyword
"default" @keyword
"delete" @keyword
"do" @keyword
"else" @keyword
"export" @keyword
"extends" @keyword
"finally" @keyword
"for" @keyword
"from" @keyword
"function" @keyword.function
"if" @keyword
"import" @keyword
"in" @keyword
"instanceof" @keyword
"let" @keyword
"new" @keyword
"of" @keyword
"return" @keyword
"static" @keyword
"switch" @keyword
"throw" @keyword
"try" @keyword
"typeof" @keyword
"var" @keyword
"while" @keyword
"with" @keyword
"yield" @keyword

(string) @string
(template_string) @string
(regex) @string.special

(number) @number

(true) @constant.builtin
(false) @constant.builtin
(null) @constant.builtin
(undefined) @constant.builtin

(identifier) @variable
(property_identifier) @property

(function_declaration name: (identifier) @function)
(method_definition name: (property_identifier) @function.method)
(call_expression function: (identifier) @function)
"#;

const PYTHON_HIGHLIGHTS: &str = r#"
(comment) @comment

"and" @keyword
"as" @keyword
"assert" @keyword
"async" @keyword
"await" @keyword
"break" @keyword
"class" @keyword
"continue" @keyword
"def" @keyword.function
"del" @keyword
"elif" @keyword
"else" @keyword
"except" @keyword
"finally" @keyword
"for" @keyword
"from" @keyword
"global" @keyword
"if" @keyword
"import" @keyword
"in" @keyword
"is" @keyword
"lambda" @keyword
"not" @keyword
"or" @keyword
"pass" @keyword
"raise" @keyword
"return" @keyword
"try" @keyword
"while" @keyword
"with" @keyword
"yield" @keyword

(string) @string
(interpolation) @string.special

(integer) @number
(float) @number

(true) @constant.builtin
(false) @constant.builtin
(none) @constant.builtin

(identifier) @variable
(attribute) @property

(function_definition name: (identifier) @function)
(call function: (identifier) @function)
"#;

const JSON_HIGHLIGHTS: &str = r#"
(string) @string
(number) @number
(true) @constant.builtin
(false) @constant.builtin
(null) @constant.builtin
(pair key: (string) @property)
"#;
