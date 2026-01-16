//! # Foxkit Snippets
//!
//! Code snippet management with VS Code-compatible format.

pub mod snippet;
pub mod parser;
pub mod registry;

use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize, Serialize};

pub use snippet::{Snippet, SnippetBody, TabStop, Variable};
pub use parser::SnippetParser;
pub use registry::SnippetRegistry;

/// Snippet scope
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SnippetScope {
    /// All languages
    Global,
    /// Specific language
    Language(String),
    /// Multiple languages
    Languages(Vec<String>),
}

impl SnippetScope {
    pub fn matches(&self, language: &str) -> bool {
        match self {
            SnippetScope::Global => true,
            SnippetScope::Language(lang) => lang == language,
            SnippetScope::Languages(langs) => langs.iter().any(|l| l == language),
        }
    }
}

impl Default for SnippetScope {
    fn default() -> Self {
        SnippetScope::Global
    }
}

/// Snippet file (VS Code format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetFile {
    #[serde(flatten)]
    pub snippets: HashMap<String, SnippetDefinition>,
}

impl SnippetFile {
    pub fn new() -> Self {
        Self {
            snippets: HashMap::new(),
        }
    }

    /// Load from file
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let file: SnippetFile = serde_json::from_str(&content)?;
        Ok(file)
    }

    /// Save to file
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Add a snippet
    pub fn add(&mut self, name: &str, definition: SnippetDefinition) {
        self.snippets.insert(name.to_string(), definition);
    }
}

impl Default for SnippetFile {
    fn default() -> Self {
        Self::new()
    }
}

/// VS Code snippet definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetDefinition {
    /// Trigger prefix(es)
    pub prefix: PrefixOrPrefixes,
    /// Snippet body (lines)
    pub body: BodyLines,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Scope (languages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

impl SnippetDefinition {
    pub fn new(prefix: &str, body: Vec<String>) -> Self {
        Self {
            prefix: PrefixOrPrefixes::Single(prefix.to_string()),
            body: BodyLines::Multiple(body),
            description: String::new(),
            scope: None,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn with_scope(mut self, scope: &str) -> Self {
        self.scope = Some(scope.to_string());
        self
    }

    /// Get all prefixes
    pub fn prefixes(&self) -> Vec<&str> {
        match &self.prefix {
            PrefixOrPrefixes::Single(s) => vec![s.as_str()],
            PrefixOrPrefixes::Multiple(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }

    /// Get body as string
    pub fn body_text(&self) -> String {
        match &self.body {
            BodyLines::Single(s) => s.clone(),
            BodyLines::Multiple(lines) => lines.join("\n"),
        }
    }

    /// Get scopes
    pub fn scopes(&self) -> Vec<String> {
        self.scope
            .as_ref()
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default()
    }
}

/// Prefix can be single or multiple
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PrefixOrPrefixes {
    Single(String),
    Multiple(Vec<String>),
}

/// Body can be single line or multiple
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BodyLines {
    Single(String),
    Multiple(Vec<String>),
}

/// Built-in snippets for common languages
pub fn builtin_snippets() -> HashMap<String, SnippetFile> {
    let mut files = HashMap::new();

    // Rust snippets
    let mut rust = SnippetFile::new();
    rust.add("fn", SnippetDefinition::new("fn", vec![
        "fn ${1:name}(${2:args}) ${3:-> ${4:Type} }{".to_string(),
        "\t$0".to_string(),
        "}".to_string(),
    ]).with_description("Function definition"));
    
    rust.add("impl", SnippetDefinition::new("impl", vec![
        "impl ${1:Type} {".to_string(),
        "\t$0".to_string(),
        "}".to_string(),
    ]).with_description("Implementation block"));
    
    rust.add("test", SnippetDefinition::new("test", vec![
        "#[test]".to_string(),
        "fn ${1:test_name}() {".to_string(),
        "\t$0".to_string(),
        "}".to_string(),
    ]).with_description("Test function"));

    files.insert("rust".to_string(), rust);

    // TypeScript snippets
    let mut typescript = SnippetFile::new();
    typescript.add("fn", SnippetDefinition::new("fn", vec![
        "function ${1:name}(${2:params}): ${3:void} {".to_string(),
        "\t$0".to_string(),
        "}".to_string(),
    ]).with_description("Function"));
    
    typescript.add("af", SnippetDefinition::new("af", vec![
        "const ${1:name} = (${2:params}) => {".to_string(),
        "\t$0".to_string(),
        "};".to_string(),
    ]).with_description("Arrow function"));
    
    typescript.add("cl", SnippetDefinition::new("cl", vec![
        "console.log($0);".to_string(),
    ]).with_description("Console log"));

    files.insert("javascript".to_string(), typescript.clone());
    files.insert("typescript".to_string(), typescript);

    // Python snippets
    let mut python = SnippetFile::new();
    python.add("def", SnippetDefinition::new("def", vec![
        "def ${1:name}(${2:args}):".to_string(),
        "\t${3:pass}$0".to_string(),
    ]).with_description("Function definition"));
    
    python.add("class", SnippetDefinition::new("class", vec![
        "class ${1:Name}:".to_string(),
        "\tdef __init__(self${2:, args}):".to_string(),
        "\t\t${3:pass}$0".to_string(),
    ]).with_description("Class definition"));

    files.insert("python".to_string(), python);

    files
}
