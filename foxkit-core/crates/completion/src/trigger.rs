//! Completion triggers

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Trigger kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerKind {
    /// Manually invoked (Ctrl+Space)
    Invoked = 1,
    /// Triggered by character
    TriggerCharacter = 2,
    /// Re-triggered while typing
    TriggerForIncompleteCompletions = 3,
}

/// Trigger character configuration
#[derive(Debug, Clone)]
pub struct TriggerCharacter {
    /// Character that triggers completion
    pub character: char,
    /// Trigger in string literals
    pub trigger_in_string: bool,
    /// Trigger in comments
    pub trigger_in_comment: bool,
}

impl TriggerCharacter {
    pub fn new(character: char) -> Self {
        Self {
            character,
            trigger_in_string: false,
            trigger_in_comment: false,
        }
    }

    pub fn in_strings(mut self) -> Self {
        self.trigger_in_string = true;
        self
    }

    pub fn in_comments(mut self) -> Self {
        self.trigger_in_comment = true;
        self
    }
}

/// Language-specific trigger configuration
#[derive(Debug, Clone)]
pub struct TriggerConfig {
    /// Characters that trigger completion
    pub characters: Vec<TriggerCharacter>,
    /// Auto-trigger on word characters
    pub auto_trigger: bool,
    /// Minimum word length for auto-trigger
    pub min_word_length: usize,
}

impl TriggerConfig {
    pub fn new() -> Self {
        Self {
            characters: Vec::new(),
            auto_trigger: true,
            min_word_length: 1,
        }
    }

    pub fn with_character(mut self, c: char) -> Self {
        self.characters.push(TriggerCharacter::new(c));
        self
    }

    pub fn with_characters(mut self, chars: &[char]) -> Self {
        for &c in chars {
            self.characters.push(TriggerCharacter::new(c));
        }
        self
    }

    pub fn is_trigger(&self, c: char) -> bool {
        self.characters.iter().any(|tc| tc.character == c)
    }
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Default trigger configurations by language
pub fn default_triggers() -> HashMap<String, TriggerConfig> {
    let mut configs = HashMap::new();

    // Rust
    configs.insert(
        "rust".to_string(),
        TriggerConfig::new()
            .with_characters(&['.', ':', '<', '>', '"', '/']),
    );

    // JavaScript/TypeScript
    configs.insert(
        "javascript".to_string(),
        TriggerConfig::new()
            .with_characters(&['.', '/', '"', '\'', '`', '<']),
    );
    configs.insert("typescript".to_string(), configs["javascript"].clone());
    configs.insert("javascriptreact".to_string(), configs["javascript"].clone());
    configs.insert("typescriptreact".to_string(), configs["javascript"].clone());

    // Python
    configs.insert(
        "python".to_string(),
        TriggerConfig::new()
            .with_characters(&['.', '/', '"', '\'']),
    );

    // Go
    configs.insert(
        "go".to_string(),
        TriggerConfig::new()
            .with_characters(&['.', '/', '"']),
    );

    // C/C++
    configs.insert(
        "c".to_string(),
        TriggerConfig::new()
            .with_characters(&['.', '-', '>', '<', '"', '/']),
    );
    configs.insert("cpp".to_string(), configs["c"].clone());

    // HTML
    configs.insert(
        "html".to_string(),
        TriggerConfig::new()
            .with_characters(&['<', '/', '"', '=']),
    );

    // CSS
    configs.insert(
        "css".to_string(),
        TriggerConfig::new()
            .with_characters(&[':', '.', '#', '@']),
    );

    // JSON
    configs.insert(
        "json".to_string(),
        TriggerConfig::new()
            .with_characters(&['"']),
    );

    configs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triggers() {
        let triggers = default_triggers();
        let rust = &triggers["rust"];
        
        assert!(rust.is_trigger('.'));
        assert!(rust.is_trigger(':'));
        assert!(!rust.is_trigger('@'));
    }
}
