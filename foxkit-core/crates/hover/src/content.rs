//! Hover Content Building
//!
//! Rich hover content generation.

use std::fmt::Write;

/// Hover content builder
pub struct HoverBuilder {
    sections: Vec<HoverSection>,
}

/// Hover section
#[derive(Debug, Clone)]
pub struct HoverSection {
    pub kind: HoverSectionKind,
    pub content: String,
}

/// Kind of hover section
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverSectionKind {
    /// Type signature / definition
    Signature,
    /// Documentation
    Documentation,
    /// Code example
    CodeExample,
    /// Source location
    SourceLocation,
    /// Type information
    TypeInfo,
    /// Parameter info
    Parameters,
    /// Return type info
    Returns,
    /// Generic separator
    Separator,
}

impl HoverBuilder {
    pub fn new() -> Self {
        Self { sections: Vec::new() }
    }

    /// Add a signature (code block)
    pub fn signature(mut self, language: &str, code: &str) -> Self {
        self.sections.push(HoverSection {
            kind: HoverSectionKind::Signature,
            content: format!("```{}\n{}\n```", language, code),
        });
        self
    }

    /// Add documentation text
    pub fn documentation(mut self, doc: &str) -> Self {
        if !doc.is_empty() {
            self.sections.push(HoverSection {
                kind: HoverSectionKind::Documentation,
                content: doc.to_string(),
            });
        }
        self
    }

    /// Add a code example
    pub fn example(mut self, language: &str, code: &str) -> Self {
        self.sections.push(HoverSection {
            kind: HoverSectionKind::CodeExample,
            content: format!("**Example:**\n```{}\n{}\n```", language, code),
        });
        self
    }

    /// Add source location
    pub fn source_location(mut self, file: &str, line: u32) -> Self {
        self.sections.push(HoverSection {
            kind: HoverSectionKind::SourceLocation,
            content: format!("*Defined in {}:{}*", file, line),
        });
        self
    }

    /// Add type information
    pub fn type_info(mut self, type_str: &str) -> Self {
        self.sections.push(HoverSection {
            kind: HoverSectionKind::TypeInfo,
            content: format!("**Type:** `{}`", type_str),
        });
        self
    }

    /// Add parameter information
    pub fn parameters(mut self, params: &[(&str, &str, Option<&str>)]) -> Self {
        if params.is_empty() {
            return self;
        }

        let mut content = String::from("**Parameters:**\n");
        for (name, type_str, doc) in params {
            write!(content, "- `{}`: `{}`", name, type_str).unwrap();
            if let Some(d) = doc {
                write!(content, " - {}", d).unwrap();
            }
            content.push('\n');
        }

        self.sections.push(HoverSection {
            kind: HoverSectionKind::Parameters,
            content,
        });
        self
    }

    /// Add return type information
    pub fn returns(mut self, type_str: &str, doc: Option<&str>) -> Self {
        let mut content = format!("**Returns:** `{}`", type_str);
        if let Some(d) = doc {
            write!(content, " - {}", d).unwrap();
        }
        
        self.sections.push(HoverSection {
            kind: HoverSectionKind::Returns,
            content,
        });
        self
    }

    /// Add a horizontal separator
    pub fn separator(mut self) -> Self {
        self.sections.push(HoverSection {
            kind: HoverSectionKind::Separator,
            content: "---".to_string(),
        });
        self
    }

    /// Build as markdown
    pub fn build_markdown(&self) -> String {
        self.sections
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Build as plain text
    pub fn build_plaintext(&self) -> String {
        self.sections
            .iter()
            .map(|s| Self::strip_markdown(&s.content))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn strip_markdown(text: &str) -> String {
        let mut result = text.to_string();
        // Remove code blocks
        while let Some(start) = result.find("```") {
            if let Some(end) = result[start + 3..].find("```") {
                let code_start = result[start + 3..].find('\n').unwrap_or(0);
                let code = &result[start + 3 + code_start + 1..start + 3 + end];
                result = format!("{}{}{}", &result[..start], code.trim(), &result[start + 6 + end..]);
            } else {
                break;
            }
        }
        // Remove inline code
        result = result.replace('`', "");
        // Remove bold/italic
        result = result.replace("**", "");
        result = result.replace("*", "");
        result
    }
}

impl Default for HoverBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Symbol hover information
#[derive(Debug, Clone)]
pub struct SymbolHover {
    /// Symbol name
    pub name: String,
    /// Symbol kind
    pub kind: SymbolKind,
    /// Container (module, class, etc.)
    pub container: Option<String>,
    /// Type signature
    pub signature: Option<String>,
    /// Documentation
    pub documentation: Option<String>,
    /// Parameters (for functions)
    pub parameters: Vec<ParameterInfo>,
    /// Return type (for functions)
    pub return_type: Option<String>,
    /// Generic type parameters
    pub type_parameters: Vec<String>,
    /// Source file
    pub source_file: Option<String>,
    /// Source line
    pub source_line: Option<u32>,
    /// Deprecation message
    pub deprecated: Option<String>,
    /// Since version
    pub since: Option<String>,
}

/// Symbol kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
    Constant,
    Function,
    Method,
    Class,
    Struct,
    Interface,
    Trait,
    Enum,
    EnumMember,
    Module,
    Namespace,
    Type,
    TypeParameter,
    Property,
    Field,
    Parameter,
    Macro,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Variable => "variable",
            Self::Constant => "constant",
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Enum => "enum",
            Self::EnumMember => "enum member",
            Self::Module => "module",
            Self::Namespace => "namespace",
            Self::Type => "type",
            Self::TypeParameter => "type parameter",
            Self::Property => "property",
            Self::Field => "field",
            Self::Parameter => "parameter",
            Self::Macro => "macro",
        }
    }
}

/// Parameter information
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub name: String,
    pub type_str: Option<String>,
    pub documentation: Option<String>,
    pub default_value: Option<String>,
    pub is_optional: bool,
    pub is_rest: bool,
}

impl SymbolHover {
    /// Build hover content from symbol info
    pub fn to_hover(&self, language: &str) -> HoverBuilder {
        let mut builder = HoverBuilder::new();

        // Build signature
        let sig = self.build_signature(language);
        builder = builder.signature(language, &sig);

        // Add deprecation warning
        if let Some(msg) = &self.deprecated {
            builder = builder.documentation(&format!("⚠️ **Deprecated:** {}", msg));
        }

        // Add documentation
        if let Some(doc) = &self.documentation {
            builder = builder.documentation(doc);
        }

        // Add parameters for functions
        if !self.parameters.is_empty() && matches!(self.kind, SymbolKind::Function | SymbolKind::Method) {
            let params: Vec<_> = self.parameters.iter()
                .map(|p| {
                    (
                        p.name.as_str(),
                        p.type_str.as_deref().unwrap_or("unknown"),
                        p.documentation.as_deref(),
                    )
                })
                .collect();
            builder = builder.parameters(&params);
        }

        // Add return type
        if let Some(ret) = &self.return_type {
            builder = builder.returns(ret, None);
        }

        // Add source location
        if let (Some(file), Some(line)) = (&self.source_file, self.source_line) {
            builder = builder.source_location(file, line);
        }

        builder
    }

    fn build_signature(&self, language: &str) -> String {
        match self.kind {
            SymbolKind::Function | SymbolKind::Method => {
                self.build_function_signature(language)
            }
            SymbolKind::Variable | SymbolKind::Constant => {
                self.build_variable_signature(language)
            }
            SymbolKind::Class | SymbolKind::Struct | SymbolKind::Interface | SymbolKind::Trait => {
                self.build_type_signature(language)
            }
            _ => {
                self.signature.clone().unwrap_or_else(|| self.name.clone())
            }
        }
    }

    fn build_function_signature(&self, language: &str) -> String {
        if let Some(sig) = &self.signature {
            return sig.clone();
        }

        let params = self.parameters.iter()
            .map(|p| {
                if let Some(t) = &p.type_str {
                    match language {
                        "rust" => format!("{}: {}", p.name, t),
                        "typescript" | "javascript" => format!("{}: {}", p.name, t),
                        "python" => {
                            if let Some(d) = &p.default_value {
                                format!("{}: {} = {}", p.name, t, d)
                            } else {
                                format!("{}: {}", p.name, t)
                            }
                        }
                        "go" => format!("{} {}", p.name, t),
                        _ => format!("{}: {}", p.name, t),
                    }
                } else {
                    p.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let ret = self.return_type.as_deref().unwrap_or("");

        match language {
            "rust" => {
                let keyword = if self.kind == SymbolKind::Method { "" } else { "fn " };
                if ret.is_empty() || ret == "()" {
                    format!("{}{}({})", keyword, self.name, params)
                } else {
                    format!("{}{}({}) -> {}", keyword, self.name, params, ret)
                }
            }
            "typescript" | "javascript" => {
                format!("function {}({}): {}", self.name, params, if ret.is_empty() { "void" } else { ret })
            }
            "python" => {
                if ret.is_empty() {
                    format!("def {}({}):", self.name, params)
                } else {
                    format!("def {}({}) -> {}:", self.name, params, ret)
                }
            }
            "go" => {
                format!("func {}({}) {}", self.name, params, ret)
            }
            _ => {
                format!("{}({})", self.name, params)
            }
        }
    }

    fn build_variable_signature(&self, language: &str) -> String {
        if let Some(sig) = &self.signature {
            return sig.clone();
        }

        let type_str = self.return_type.as_deref().unwrap_or("unknown");
        
        match language {
            "rust" => {
                let keyword = if self.kind == SymbolKind::Constant { "const" } else { "let" };
                format!("{} {}: {}", keyword, self.name, type_str)
            }
            "typescript" => {
                let keyword = if self.kind == SymbolKind::Constant { "const" } else { "let" };
                format!("{} {}: {}", keyword, self.name, type_str)
            }
            "python" => {
                format!("{}: {}", self.name, type_str)
            }
            "go" => {
                format!("var {} {}", self.name, type_str)
            }
            _ => {
                format!("{}: {}", self.name, type_str)
            }
        }
    }

    fn build_type_signature(&self, language: &str) -> String {
        if let Some(sig) = &self.signature {
            return sig.clone();
        }

        let type_params = if self.type_parameters.is_empty() {
            String::new()
        } else {
            format!("<{}>", self.type_parameters.join(", "))
        };

        match language {
            "rust" => {
                let keyword = match self.kind {
                    SymbolKind::Struct => "struct",
                    SymbolKind::Trait => "trait",
                    SymbolKind::Enum => "enum",
                    _ => "type",
                };
                format!("{} {}{}", keyword, self.name, type_params)
            }
            "typescript" => {
                let keyword = match self.kind {
                    SymbolKind::Class => "class",
                    SymbolKind::Interface => "interface",
                    SymbolKind::Enum => "enum",
                    _ => "type",
                };
                format!("{} {}{}", keyword, self.name, type_params)
            }
            "python" => {
                format!("class {}", self.name)
            }
            "go" => {
                format!("type {} struct", self.name)
            }
            _ => {
                format!("{}{}", self.name, type_params)
            }
        }
    }
}

/// Diagnostic hover
#[derive(Debug, Clone)]
pub struct DiagnosticHover {
    pub severity: DiagnosticHoverSeverity,
    pub message: String,
    pub code: Option<String>,
    pub source: Option<String>,
    pub related: Vec<RelatedInfo>,
}

#[derive(Debug, Clone, Copy)]
pub enum DiagnosticHoverSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Debug, Clone)]
pub struct RelatedInfo {
    pub message: String,
    pub location: String,
}

impl DiagnosticHover {
    pub fn to_hover(&self) -> HoverBuilder {
        let mut builder = HoverBuilder::new();

        // Severity icon
        let icon = match self.severity {
            DiagnosticHoverSeverity::Error => "❌",
            DiagnosticHoverSeverity::Warning => "⚠️",
            DiagnosticHoverSeverity::Information => "ℹ️",
            DiagnosticHoverSeverity::Hint => "💡",
        };

        // Build header
        let mut header = format!("{} {}", icon, self.message);
        if let Some(code) = &self.code {
            write!(header, " `{}`", code).unwrap();
        }
        if let Some(source) = &self.source {
            write!(header, " ({})", source).unwrap();
        }

        builder = builder.documentation(&header);

        // Add related information
        if !self.related.is_empty() {
            let related_text = self.related.iter()
                .map(|r| format!("- {} ([link]({}))", r.message, r.location))
                .collect::<Vec<_>>()
                .join("\n");
            builder = builder.documentation(&format!("**Related:**\n{}", related_text));
        }

        builder
    }
}
