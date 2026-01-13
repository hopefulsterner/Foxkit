//! Snippet types and expansion

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// A parsed snippet
#[derive(Debug, Clone)]
pub struct Snippet {
    /// Snippet name
    pub name: String,
    /// Trigger prefix
    pub prefix: String,
    /// Description
    pub description: String,
    /// Parsed body
    pub body: SnippetBody,
    /// Language scope
    pub scope: Option<String>,
}

impl Snippet {
    pub fn new(name: &str, prefix: &str, body: SnippetBody) -> Self {
        Self {
            name: name.to_string(),
            prefix: prefix.to_string(),
            description: String::new(),
            body,
            scope: None,
        }
    }

    /// Expand snippet with tab stop values
    pub fn expand(&self, values: &HashMap<usize, String>) -> String {
        self.body.expand(values)
    }

    /// Get all tab stops
    pub fn tab_stops(&self) -> Vec<&TabStop> {
        self.body.tab_stops()
    }

    /// Has placeholders?
    pub fn has_placeholders(&self) -> bool {
        !self.body.tab_stops().is_empty()
    }
}

/// Parsed snippet body
#[derive(Debug, Clone)]
pub struct SnippetBody {
    pub parts: Vec<SnippetPart>,
}

impl SnippetBody {
    pub fn new(parts: Vec<SnippetPart>) -> Self {
        Self { parts }
    }

    pub fn from_text(text: &str) -> Self {
        Self {
            parts: vec![SnippetPart::Text(text.to_string())],
        }
    }

    /// Expand with values
    pub fn expand(&self, values: &HashMap<usize, String>) -> String {
        let mut result = String::new();
        
        for part in &self.parts {
            match part {
                SnippetPart::Text(text) => result.push_str(text),
                SnippetPart::TabStop(ts) => {
                    if let Some(value) = values.get(&ts.index) {
                        result.push_str(value);
                    } else if let Some(ref default) = ts.placeholder {
                        result.push_str(default);
                    }
                }
                SnippetPart::Variable(var) => {
                    if let Some(ref value) = var.value {
                        result.push_str(value);
                    } else if let Some(ref default) = var.default {
                        result.push_str(default);
                    }
                }
                SnippetPart::Choice(ts, choices) => {
                    if let Some(value) = values.get(&ts.index) {
                        result.push_str(value);
                    } else if let Some(first) = choices.first() {
                        result.push_str(first);
                    }
                }
            }
        }
        
        result
    }

    /// Get tab stops
    pub fn tab_stops(&self) -> Vec<&TabStop> {
        self.parts
            .iter()
            .filter_map(|p| match p {
                SnippetPart::TabStop(ts) => Some(ts),
                SnippetPart::Choice(ts, _) => Some(ts),
                _ => None,
            })
            .collect()
    }

    /// Get final cursor position ($0)
    pub fn final_position(&self) -> Option<&TabStop> {
        self.tab_stops().into_iter().find(|ts| ts.index == 0)
    }
}

/// Part of a snippet body
#[derive(Debug, Clone)]
pub enum SnippetPart {
    /// Plain text
    Text(String),
    /// Tab stop ($1, ${1:placeholder})
    TabStop(TabStop),
    /// Variable ($TM_FILENAME, ${TM_FILENAME:default})
    Variable(Variable),
    /// Choice (${1|one,two,three|})
    Choice(TabStop, Vec<String>),
}

/// A tab stop
#[derive(Debug, Clone)]
pub struct TabStop {
    /// Tab stop index (0 = final position)
    pub index: usize,
    /// Default placeholder text
    pub placeholder: Option<String>,
    /// Transform (regex)
    pub transform: Option<Transform>,
}

impl TabStop {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            placeholder: None,
            transform: None,
        }
    }

    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = Some(placeholder.to_string());
        self
    }
}

/// A variable
#[derive(Debug, Clone)]
pub struct Variable {
    /// Variable name
    pub name: String,
    /// Resolved value
    pub value: Option<String>,
    /// Default if not resolved
    pub default: Option<String>,
    /// Transform
    pub transform: Option<Transform>,
}

impl Variable {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: None,
            default: None,
            transform: None,
        }
    }

    /// Resolve variable value
    pub fn resolve(&mut self, context: &SnippetContext) {
        self.value = context.get(&self.name);
    }
}

/// Variable transform
#[derive(Debug, Clone)]
pub struct Transform {
    pub regex: String,
    pub replacement: String,
    pub flags: String,
}

/// Context for variable resolution
#[derive(Debug, Clone, Default)]
pub struct SnippetContext {
    values: HashMap<String, String>,
}

impl SnippetContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set from editor context
    pub fn from_editor(
        filename: &str,
        filepath: &str,
        line: usize,
        selection: Option<&str>,
    ) -> Self {
        let mut ctx = Self::new();
        
        // File variables
        ctx.set("TM_FILENAME", filename);
        ctx.set("TM_FILEPATH", filepath);
        ctx.set("TM_DIRECTORY", std::path::Path::new(filepath)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or(""));
        ctx.set("TM_FILENAME_BASE", std::path::Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(""));
        
        // Line variables
        ctx.set("TM_LINE_INDEX", &(line.saturating_sub(1)).to_string());
        ctx.set("TM_LINE_NUMBER", &line.to_string());
        
        // Selection
        if let Some(sel) = selection {
            ctx.set("TM_SELECTED_TEXT", sel);
            ctx.set("SELECTION", sel);
        }
        
        // Current date/time
        let now = chrono::Local::now();
        ctx.set("CURRENT_YEAR", &now.format("%Y").to_string());
        ctx.set("CURRENT_MONTH", &now.format("%m").to_string());
        ctx.set("CURRENT_DATE", &now.format("%d").to_string());
        ctx.set("CURRENT_HOUR", &now.format("%H").to_string());
        ctx.set("CURRENT_MINUTE", &now.format("%M").to_string());
        ctx.set("CURRENT_SECOND", &now.format("%S").to_string());
        
        ctx
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.values.insert(name.to_string(), value.to_string());
    }

    pub fn get(&self, name: &str) -> Option<String> {
        self.values.get(name).cloned()
    }
}

// Note: chrono is not in deps, this is just for illustration
mod chrono {
    pub struct Local;
    impl Local {
        pub fn now() -> DateTime { DateTime }
    }
    pub struct DateTime;
    impl DateTime {
        pub fn format(&self, _fmt: &str) -> FormattedDate { FormattedDate }
    }
    pub struct FormattedDate;
    impl FormattedDate {
        pub fn to_string(&self) -> String { "2024".to_string() }
    }
}
