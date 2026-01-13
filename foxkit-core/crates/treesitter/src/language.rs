//! Language definitions

use tree_sitter::Language as TSLanguage;

/// Supported language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    JavaScript,
    TypeScript,
    Tsx,
    Python,
    Go,
    C,
    Cpp,
    Java,
    Json,
    Html,
    Css,
    Markdown,
    Yaml,
    Toml,
    Bash,
}

impl Language {
    /// Get tree-sitter language
    pub fn ts_language(&self) -> TSLanguage {
        match self {
            #[cfg(feature = "rust")]
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            
            #[cfg(feature = "javascript")]
            Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            
            #[cfg(feature = "typescript")]
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            
            #[cfg(feature = "typescript")]
            Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            
            #[cfg(feature = "python")]
            Language::Python => tree_sitter_python::LANGUAGE.into(),
            
            #[cfg(feature = "json")]
            Language::Json => tree_sitter_json::LANGUAGE.into(),
            
            // Fallback for disabled features or unsupported languages
            _ => panic!("Language not available: {:?}", self),
        }
    }

    /// Get language from string ID
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "rust" | "rs" => Some(Language::Rust),
            "javascript" | "js" => Some(Language::JavaScript),
            "typescript" | "ts" => Some(Language::TypeScript),
            "typescriptreact" | "tsx" => Some(Language::Tsx),
            "python" | "py" => Some(Language::Python),
            "go" => Some(Language::Go),
            "c" => Some(Language::C),
            "cpp" | "c++" | "cxx" => Some(Language::Cpp),
            "java" => Some(Language::Java),
            "json" => Some(Language::Json),
            "html" | "htm" => Some(Language::Html),
            "css" => Some(Language::Css),
            "markdown" | "md" => Some(Language::Markdown),
            "yaml" | "yml" => Some(Language::Yaml),
            "toml" => Some(Language::Toml),
            "bash" | "sh" | "shellscript" => Some(Language::Bash),
            _ => None,
        }
    }

    /// Get language ID
    pub fn id(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
            Language::Tsx => "typescriptreact",
            Language::Python => "python",
            Language::Go => "go",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::Java => "java",
            Language::Json => "json",
            Language::Html => "html",
            Language::Css => "css",
            Language::Markdown => "markdown",
            Language::Yaml => "yaml",
            Language::Toml => "toml",
            Language::Bash => "bash",
        }
    }

    /// Get file extensions
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::Rust => &["rs"],
            Language::JavaScript => &["js", "mjs", "cjs"],
            Language::TypeScript => &["ts", "mts", "cts"],
            Language::Tsx => &["tsx"],
            Language::Python => &["py", "pyw", "pyi"],
            Language::Go => &["go"],
            Language::C => &["c", "h"],
            Language::Cpp => &["cpp", "cc", "cxx", "hpp", "hh", "hxx"],
            Language::Java => &["java"],
            Language::Json => &["json", "jsonc"],
            Language::Html => &["html", "htm"],
            Language::Css => &["css"],
            Language::Markdown => &["md", "markdown"],
            Language::Yaml => &["yaml", "yml"],
            Language::Toml => &["toml"],
            Language::Bash => &["sh", "bash", "zsh"],
        }
    }

    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Language::Rust),
            "js" | "mjs" | "cjs" => Some(Language::JavaScript),
            "ts" | "mts" | "cts" => Some(Language::TypeScript),
            "tsx" => Some(Language::Tsx),
            "jsx" => Some(Language::JavaScript), // JSX uses JS parser
            "py" | "pyw" | "pyi" => Some(Language::Python),
            "go" => Some(Language::Go),
            "c" | "h" => Some(Language::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Some(Language::Cpp),
            "java" => Some(Language::Java),
            "json" | "jsonc" => Some(Language::Json),
            "html" | "htm" => Some(Language::Html),
            "css" => Some(Language::Css),
            "md" | "markdown" => Some(Language::Markdown),
            "yaml" | "yml" => Some(Language::Yaml),
            "toml" => Some(Language::Toml),
            "sh" | "bash" | "zsh" => Some(Language::Bash),
            _ => None,
        }
    }
}
