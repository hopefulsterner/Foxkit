//! Snippet Support
//!
//! LSP snippet parsing and expansion.

use std::collections::HashMap;

/// Parsed snippet with placeholders
#[derive(Debug, Clone)]
pub struct Snippet {
    /// Original snippet text
    pub source: String,
    /// Parsed elements
    pub elements: Vec<SnippetElement>,
    /// Placeholder definitions
    pub placeholders: HashMap<u32, PlaceholderDef>,
}

/// Snippet element
#[derive(Debug, Clone)]
pub enum SnippetElement {
    /// Plain text
    Text(String),
    /// Tabstop: $0, $1, etc.
    Tabstop(u32),
    /// Placeholder: ${1:default}
    Placeholder { index: u32, default: String },
    /// Choice: ${1|one,two,three|}
    Choice { index: u32, choices: Vec<String> },
    /// Variable: $name or ${name:default}
    Variable { name: String, default: Option<String> },
    /// Nested placeholder
    Nested { index: u32, elements: Vec<SnippetElement> },
}

/// Placeholder definition
#[derive(Debug, Clone)]
pub struct PlaceholderDef {
    pub index: u32,
    pub default: String,
    pub choices: Option<Vec<String>>,
}

impl Snippet {
    /// Parse a snippet string
    pub fn parse(source: &str) -> Result<Self, SnippetError> {
        let mut parser = SnippetParser::new(source);
        parser.parse()
    }

    /// Expand snippet with variable values
    pub fn expand(&self, variables: &HashMap<String, String>) -> String {
        let mut result = String::new();
        for element in &self.elements {
            result.push_str(&self.expand_element(element, variables));
        }
        result
    }

    fn expand_element(&self, element: &SnippetElement, variables: &HashMap<String, String>) -> String {
        match element {
            SnippetElement::Text(text) => text.clone(),
            SnippetElement::Tabstop(_) => String::new(),
            SnippetElement::Placeholder { default, .. } => default.clone(),
            SnippetElement::Choice { choices, .. } => {
                choices.first().cloned().unwrap_or_default()
            }
            SnippetElement::Variable { name, default } => {
                variables.get(name).cloned()
                    .or_else(|| default.clone())
                    .or_else(|| self.builtin_variable(name))
                    .unwrap_or_default()
            }
            SnippetElement::Nested { elements, .. } => {
                elements.iter()
                    .map(|e| self.expand_element(e, variables))
                    .collect()
            }
        }
    }

    fn builtin_variable(&self, name: &str) -> Option<String> {
        match name {
            "TM_CURRENT_LINE" => Some(String::new()),
            "TM_CURRENT_WORD" => Some(String::new()),
            "TM_LINE_INDEX" => Some("0".to_string()),
            "TM_LINE_NUMBER" => Some("1".to_string()),
            "TM_FILENAME" => Some("untitled".to_string()),
            "TM_FILENAME_BASE" => Some("untitled".to_string()),
            "TM_DIRECTORY" => Some(".".to_string()),
            "TM_FILEPATH" => Some("./untitled".to_string()),
            "CLIPBOARD" => Some(String::new()),
            "CURRENT_YEAR" => Some(chrono::Utc::now().format("%Y").to_string()),
            "CURRENT_YEAR_SHORT" => Some(chrono::Utc::now().format("%y").to_string()),
            "CURRENT_MONTH" => Some(chrono::Utc::now().format("%m").to_string()),
            "CURRENT_MONTH_NAME" => Some(chrono::Utc::now().format("%B").to_string()),
            "CURRENT_MONTH_NAME_SHORT" => Some(chrono::Utc::now().format("%b").to_string()),
            "CURRENT_DATE" => Some(chrono::Utc::now().format("%d").to_string()),
            "CURRENT_DAY_NAME" => Some(chrono::Utc::now().format("%A").to_string()),
            "CURRENT_DAY_NAME_SHORT" => Some(chrono::Utc::now().format("%a").to_string()),
            "CURRENT_HOUR" => Some(chrono::Utc::now().format("%H").to_string()),
            "CURRENT_MINUTE" => Some(chrono::Utc::now().format("%M").to_string()),
            "CURRENT_SECOND" => Some(chrono::Utc::now().format("%S").to_string()),
            "UUID" => Some(uuid::Uuid::new_v4().to_string()),
            "RANDOM" => Some(format!("{:06}", rand::random::<u32>() % 1000000)),
            "RANDOM_HEX" => Some(format!("{:06x}", rand::random::<u32>() % 0xFFFFFF)),
            _ => None,
        }
    }

    /// Get all tabstop indices in order
    pub fn tabstops(&self) -> Vec<u32> {
        let mut stops: Vec<u32> = self.placeholders.keys().copied().collect();
        stops.sort();
        // $0 should be last
        if let Some(pos) = stops.iter().position(|&x| x == 0) {
            let zero = stops.remove(pos);
            stops.push(zero);
        }
        stops
    }

    /// Get placeholder default value
    pub fn placeholder_default(&self, index: u32) -> Option<&str> {
        self.placeholders.get(&index).map(|p| p.default.as_str())
    }
}

/// Snippet parser
struct SnippetParser<'a> {
    source: &'a str,
    pos: usize,
    elements: Vec<SnippetElement>,
    placeholders: HashMap<u32, PlaceholderDef>,
}

impl<'a> SnippetParser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            pos: 0,
            elements: Vec::new(),
            placeholders: HashMap::new(),
        }
    }

    fn parse(mut self) -> Result<Snippet, SnippetError> {
        while self.pos < self.source.len() {
            if self.peek() == Some('$') {
                self.parse_dollar()?;
            } else if self.peek() == Some('\\') {
                self.parse_escape()?;
            } else {
                self.parse_text();
            }
        }

        Ok(Snippet {
            source: self.source.to_string(),
            elements: self.elements,
            placeholders: self.placeholders,
        })
    }

    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn parse_text(&mut self) {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch == '$' || ch == '\\' {
                break;
            }
            self.advance();
        }
        if self.pos > start {
            let text = self.source[start..self.pos].to_string();
            self.elements.push(SnippetElement::Text(text));
        }
    }

    fn parse_escape(&mut self) -> Result<(), SnippetError> {
        self.advance(); // consume '\'
        if let Some(ch) = self.advance() {
            self.elements.push(SnippetElement::Text(ch.to_string()));
        }
        Ok(())
    }

    fn parse_dollar(&mut self) -> Result<(), SnippetError> {
        self.advance(); // consume '$'
        
        if self.peek() == Some('{') {
            self.parse_braced()?;
        } else if self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            self.parse_tabstop()?;
        } else if self.peek().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) {
            self.parse_variable()?;
        } else {
            self.elements.push(SnippetElement::Text("$".to_string()));
        }
        
        Ok(())
    }

    fn parse_tabstop(&mut self) -> Result<(), SnippetError> {
        let start = self.pos;
        while self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            self.advance();
        }
        let index: u32 = self.source[start..self.pos].parse()
            .map_err(|_| SnippetError::InvalidTabstop)?;
        
        self.placeholders.entry(index).or_insert(PlaceholderDef {
            index,
            default: String::new(),
            choices: None,
        });
        
        self.elements.push(SnippetElement::Tabstop(index));
        Ok(())
    }

    fn parse_variable(&mut self) -> Result<(), SnippetError> {
        let start = self.pos;
        while self.peek().map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false) {
            self.advance();
        }
        let name = self.source[start..self.pos].to_string();
        self.elements.push(SnippetElement::Variable { name, default: None });
        Ok(())
    }

    fn parse_braced(&mut self) -> Result<(), SnippetError> {
        self.advance(); // consume '{'
        
        // Check if it starts with a number (placeholder) or letter (variable)
        if self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            self.parse_placeholder()?;
        } else {
            self.parse_braced_variable()?;
        }
        
        Ok(())
    }

    fn parse_placeholder(&mut self) -> Result<(), SnippetError> {
        // Parse index
        let start = self.pos;
        while self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            self.advance();
        }
        let index: u32 = self.source[start..self.pos].parse()
            .map_err(|_| SnippetError::InvalidTabstop)?;

        if self.peek() == Some(':') {
            self.advance(); // consume ':'
            let default = self.parse_until_close()?;
            
            self.placeholders.insert(index, PlaceholderDef {
                index,
                default: default.clone(),
                choices: None,
            });
            
            self.elements.push(SnippetElement::Placeholder { index, default });
        } else if self.peek() == Some('|') {
            self.advance(); // consume '|'
            let choices = self.parse_choices()?;
            
            self.placeholders.insert(index, PlaceholderDef {
                index,
                default: choices.first().cloned().unwrap_or_default(),
                choices: Some(choices.clone()),
            });
            
            self.elements.push(SnippetElement::Choice { index, choices });
        } else {
            self.placeholders.entry(index).or_insert(PlaceholderDef {
                index,
                default: String::new(),
                choices: None,
            });
            self.elements.push(SnippetElement::Tabstop(index));
        }

        if self.peek() == Some('}') {
            self.advance();
        }
        
        Ok(())
    }

    fn parse_braced_variable(&mut self) -> Result<(), SnippetError> {
        let start = self.pos;
        while self.peek().map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false) {
            self.advance();
        }
        let name = self.source[start..self.pos].to_string();

        let default = if self.peek() == Some(':') {
            self.advance();
            Some(self.parse_until_close()?)
        } else {
            None
        };

        if self.peek() == Some('}') {
            self.advance();
        }

        self.elements.push(SnippetElement::Variable { name, default });
        Ok(())
    }

    fn parse_until_close(&mut self) -> Result<String, SnippetError> {
        let mut result = String::new();
        let mut depth = 1;
        
        while self.pos < self.source.len() && depth > 0 {
            match self.peek() {
                Some('{') => {
                    depth += 1;
                    result.push(self.advance().unwrap());
                }
                Some('}') => {
                    depth -= 1;
                    if depth > 0 {
                        result.push(self.advance().unwrap());
                    }
                }
                Some('\\') => {
                    self.advance();
                    if let Some(ch) = self.advance() {
                        result.push(ch);
                    }
                }
                Some(ch) => {
                    result.push(ch);
                    self.advance();
                }
                None => break,
            }
        }
        
        Ok(result)
    }

    fn parse_choices(&mut self) -> Result<Vec<String>, SnippetError> {
        let mut choices = Vec::new();
        let mut current = String::new();
        
        while self.pos < self.source.len() {
            match self.peek() {
                Some('|') => {
                    self.advance();
                    if !current.is_empty() {
                        choices.push(current);
                    }
                    break;
                }
                Some(',') => {
                    self.advance();
                    choices.push(current);
                    current = String::new();
                }
                Some('\\') => {
                    self.advance();
                    if let Some(ch) = self.advance() {
                        current.push(ch);
                    }
                }
                Some(ch) => {
                    current.push(ch);
                    self.advance();
                }
                None => break,
            }
        }
        
        Ok(choices)
    }
}

/// Snippet parsing error
#[derive(Debug, Clone, thiserror::Error)]
pub enum SnippetError {
    #[error("Invalid tabstop")]
    InvalidTabstop,
    #[error("Unclosed brace")]
    UnclosedBrace,
    #[error("Invalid escape sequence")]
    InvalidEscape,
}

/// Common snippet templates
pub struct SnippetTemplates;

impl SnippetTemplates {
    /// Rust function snippet
    pub fn rust_fn() -> &'static str {
        "fn ${1:name}(${2:args}) ${3:-> ${4:ReturnType} }{\n\t$0\n}"
    }

    /// Rust struct snippet
    pub fn rust_struct() -> &'static str {
        "#[derive(${1:Debug, Clone})]\npub struct ${2:Name} {\n\t${3:field}: ${4:Type},\n}"
    }

    /// Rust impl snippet
    pub fn rust_impl() -> &'static str {
        "impl ${1:Type} {\n\t$0\n}"
    }

    /// Rust test snippet
    pub fn rust_test() -> &'static str {
        "#[test]\nfn ${1:test_name}() {\n\t$0\n}"
    }

    /// TypeScript function snippet
    pub fn ts_function() -> &'static str {
        "function ${1:name}(${2:params}): ${3:void} {\n\t$0\n}"
    }

    /// TypeScript arrow function snippet
    pub fn ts_arrow() -> &'static str {
        "const ${1:name} = (${2:params}) => {\n\t$0\n};"
    }

    /// TypeScript interface snippet
    pub fn ts_interface() -> &'static str {
        "interface ${1:Name} {\n\t${2:property}: ${3:type};\n}"
    }

    /// Python function snippet
    pub fn py_def() -> &'static str {
        "def ${1:name}(${2:args}):\n\t${3:pass}$0"
    }

    /// Python class snippet
    pub fn py_class() -> &'static str {
        "class ${1:Name}:\n\tdef __init__(self${2:, args}):\n\t\t${3:pass}$0"
    }

    /// Go function snippet
    pub fn go_func() -> &'static str {
        "func ${1:name}(${2:args}) ${3:error} {\n\t$0\n}"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tabstop() {
        let snippet = Snippet::parse("hello $1 world").unwrap();
        assert_eq!(snippet.elements.len(), 3);
    }

    #[test]
    fn test_placeholder() {
        let snippet = Snippet::parse("${1:default}").unwrap();
        assert_eq!(snippet.placeholder_default(1), Some("default"));
    }

    #[test]
    fn test_choice() {
        let snippet = Snippet::parse("${1|one,two,three|}").unwrap();
        if let SnippetElement::Choice { choices, .. } = &snippet.elements[0] {
            assert_eq!(choices, &["one", "two", "three"]);
        } else {
            panic!("Expected Choice element");
        }
    }
}
