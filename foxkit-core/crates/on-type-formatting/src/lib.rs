//! # Foxkit On-Type Formatting
//!
//! Automatic formatting triggered while typing.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// On-type formatting service
pub struct OnTypeFormattingService {
    /// Trigger characters per language
    triggers: RwLock<HashMap<String, Vec<char>>>,
    /// Events
    events: broadcast::Sender<OnTypeFormattingEvent>,
    /// Configuration
    config: RwLock<OnTypeFormattingConfig>,
}

impl OnTypeFormattingService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);
        
        let mut triggers = HashMap::new();
        
        // Default triggers
        triggers.insert("typescript".to_string(), vec!['\n', ';', '}']);
        triggers.insert("javascript".to_string(), vec!['\n', ';', '}']);
        triggers.insert("rust".to_string(), vec!['\n', ';', '}']);
        triggers.insert("go".to_string(), vec!['\n', '}']);
        triggers.insert("python".to_string(), vec!['\n', ':']);
        triggers.insert("c".to_string(), vec!['\n', ';', '}']);
        triggers.insert("cpp".to_string(), vec!['\n', ';', '}']);
        triggers.insert("csharp".to_string(), vec!['\n', ';', '}']);
        triggers.insert("java".to_string(), vec!['\n', ';', '}']);

        Self {
            triggers: RwLock::new(triggers),
            events,
            config: RwLock::new(OnTypeFormattingConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<OnTypeFormattingEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: OnTypeFormattingConfig) {
        *self.config.write() = config;
    }

    /// Set trigger characters for language
    pub fn set_triggers(&self, language: impl Into<String>, triggers: Vec<char>) {
        self.triggers.write().insert(language.into(), triggers);
    }

    /// Get trigger characters for language
    pub fn get_triggers(&self, language: &str) -> Vec<char> {
        self.triggers.read()
            .get(language)
            .cloned()
            .unwrap_or_default()
    }

    /// Check if character is a trigger
    pub fn is_trigger(&self, language: &str, ch: char) -> bool {
        self.triggers.read()
            .get(language)
            .map(|t| t.contains(&ch))
            .unwrap_or(false)
    }

    /// Format on type (would call LSP)
    pub async fn format_on_type(
        &self,
        file: &PathBuf,
        language: &str,
        position: FormatPosition,
        ch: char,
        options: FormatOptions,
    ) -> anyhow::Result<Vec<TextEdit>> {
        // Check if enabled
        if !self.config.read().enabled {
            return Ok(Vec::new());
        }

        // Check if this is a trigger character
        if !self.is_trigger(language, ch) {
            return Ok(Vec::new());
        }

        let _ = self.events.send(OnTypeFormattingEvent::Formatting {
            file: file.clone(),
            character: ch,
        });

        // Would call LSP textDocument/onTypeFormatting
        let edits = Vec::new();

        let _ = self.events.send(OnTypeFormattingEvent::Formatted {
            file: file.clone(),
            edit_count: edits.len(),
        });

        Ok(edits)
    }

    /// Get first trigger character for language
    pub fn first_trigger(&self, language: &str) -> Option<char> {
        self.triggers.read()
            .get(language)
            .and_then(|t| t.first().copied())
    }

    /// Get more trigger characters (all except first)
    pub fn more_triggers(&self, language: &str) -> Vec<char> {
        self.triggers.read()
            .get(language)
            .map(|t| t.iter().skip(1).copied().collect())
            .unwrap_or_default()
    }
}

impl Default for OnTypeFormattingService {
    fn default() -> Self {
        Self::new()
    }
}

/// Text edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: TextRange,
    pub new_text: String,
}

impl TextEdit {
    pub fn new(range: TextRange, new_text: impl Into<String>) -> Self {
        Self { range, new_text: new_text.into() }
    }

    pub fn insert(position: FormatPosition, text: impl Into<String>) -> Self {
        Self {
            range: TextRange::point(position.line, position.col),
            new_text: text.into(),
        }
    }
}

/// Text range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl TextRange {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
    }

    pub fn point(line: u32, col: u32) -> Self {
        Self { start_line: line, start_col: col, end_line: line, end_col: col }
    }
}

/// Format position
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FormatPosition {
    pub line: u32,
    pub col: u32,
}

impl FormatPosition {
    pub fn new(line: u32, col: u32) -> Self {
        Self { line, col }
    }
}

/// Format options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatOptions {
    pub tab_size: u32,
    pub insert_spaces: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
        }
    }
}

/// Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnTypeFormattingConfig {
    /// Enable on-type formatting
    pub enabled: bool,
    /// Per-language enablement
    pub languages: HashMap<String, bool>,
}

impl Default for OnTypeFormattingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            languages: HashMap::new(),
        }
    }
}

impl OnTypeFormattingConfig {
    pub fn is_enabled_for(&self, language: &str) -> bool {
        if !self.enabled {
            return false;
        }
        
        self.languages
            .get(language)
            .copied()
            .unwrap_or(true)
    }
}

/// Event
#[derive(Debug, Clone)]
pub enum OnTypeFormattingEvent {
    Formatting { file: PathBuf, character: char },
    Formatted { file: PathBuf, edit_count: usize },
    Skipped { file: PathBuf, reason: String },
}

/// Auto-indent provider
pub struct AutoIndentProvider {
    /// Indent rules per language
    rules: HashMap<String, IndentRules>,
}

impl AutoIndentProvider {
    pub fn new() -> Self {
        let mut rules = HashMap::new();

        // C-style languages
        let c_style = IndentRules {
            increase_indent_pattern: r#"\{[^}"']*$"#.to_string(),
            decrease_indent_pattern: r#"^\s*\}"#.to_string(),
            indent_next_line_pattern: None,
            unindented_line_pattern: None,
        };

        for lang in &["javascript", "typescript", "c", "cpp", "csharp", "java", "rust", "go"] {
            rules.insert(lang.to_string(), c_style.clone());
        }

        // Python
        rules.insert("python".to_string(), IndentRules {
            increase_indent_pattern: r#":\s*(#.*)?$"#.to_string(),
            decrease_indent_pattern: r#"^\s*(return|break|continue|raise|pass)\b"#.to_string(),
            indent_next_line_pattern: Some(r#"\\\s*$"#.to_string()),
            unindented_line_pattern: None,
        });

        Self { rules }
    }

    pub fn get_rules(&self, language: &str) -> Option<&IndentRules> {
        self.rules.get(language)
    }

    pub fn calculate_indent(
        &self,
        language: &str,
        previous_line: &str,
        current_line: &str,
        tab_size: u32,
    ) -> u32 {
        let rules = match self.get_rules(language) {
            Some(r) => r,
            None => return get_leading_whitespace(previous_line, tab_size),
        };

        let base_indent = get_leading_whitespace(previous_line, tab_size);

        // Check for indent increase
        let increase = regex::Regex::new(&rules.increase_indent_pattern)
            .ok()
            .map(|r| r.is_match(previous_line))
            .unwrap_or(false);

        // Check for indent decrease
        let decrease = regex::Regex::new(&rules.decrease_indent_pattern)
            .ok()
            .map(|r| r.is_match(current_line))
            .unwrap_or(false);

        if increase && !decrease {
            base_indent + tab_size
        } else if decrease && !increase {
            base_indent.saturating_sub(tab_size)
        } else {
            base_indent
        }
    }
}

impl Default for AutoIndentProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn get_leading_whitespace(line: &str, tab_size: u32) -> u32 {
    let mut count = 0;
    
    for ch in line.chars() {
        match ch {
            ' ' => count += 1,
            '\t' => count += tab_size,
            _ => break,
        }
    }
    
    count
}

/// Indent rules
#[derive(Debug, Clone)]
pub struct IndentRules {
    /// Pattern that increases indent
    pub increase_indent_pattern: String,
    /// Pattern that decreases indent
    pub decrease_indent_pattern: String,
    /// Pattern that indents only next line
    pub indent_next_line_pattern: Option<String>,
    /// Pattern for lines that shouldn't be indented
    pub unindented_line_pattern: Option<String>,
}

/// Bracket auto-close
pub struct BracketAutoClose {
    /// Pairs
    pairs: Vec<BracketPair>,
}

impl BracketAutoClose {
    pub fn new() -> Self {
        Self {
            pairs: vec![
                BracketPair::new('(', ')'),
                BracketPair::new('[', ']'),
                BracketPair::new('{', '}'),
                BracketPair::new('"', '"'),
                BracketPair::new('\'', '\''),
                BracketPair::new('`', '`'),
            ],
        }
    }

    pub fn should_auto_close(&self, ch: char) -> Option<char> {
        self.pairs.iter()
            .find(|p| p.open == ch)
            .map(|p| p.close)
    }

    pub fn should_skip_close(&self, ch: char, next_char: Option<char>) -> bool {
        if let Some(next) = next_char {
            self.pairs.iter().any(|p| p.close == ch && p.close == next)
        } else {
            false
        }
    }
}

impl Default for BracketAutoClose {
    fn default() -> Self {
        Self::new()
    }
}

/// Bracket pair
#[derive(Debug, Clone)]
pub struct BracketPair {
    pub open: char,
    pub close: char,
}

impl BracketPair {
    pub fn new(open: char, close: char) -> Self {
        Self { open, close }
    }
}
