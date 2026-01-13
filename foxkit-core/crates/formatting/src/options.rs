//! Formatting options

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Formatting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingOptions {
    /// Tab size in spaces
    pub tab_size: u32,
    /// Use spaces instead of tabs
    pub insert_spaces: bool,
    /// Trim trailing whitespace
    pub trim_trailing_whitespace: bool,
    /// Insert final newline
    pub insert_final_newline: bool,
    /// Trim final newlines
    pub trim_final_newlines: bool,
    /// Max line length (0 = no limit)
    pub max_line_length: u32,
    /// Language-specific options
    #[serde(default)]
    pub language_options: HashMap<String, serde_json::Value>,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: true,
            insert_final_newline: true,
            trim_final_newlines: true,
            max_line_length: 0,
            language_options: HashMap::new(),
        }
    }
}

impl FormattingOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tab_size(mut self, size: u32) -> Self {
        self.tab_size = size;
        self
    }

    pub fn with_tabs(mut self) -> Self {
        self.insert_spaces = false;
        self
    }

    pub fn with_spaces(mut self) -> Self {
        self.insert_spaces = true;
        self
    }

    pub fn with_max_line_length(mut self, length: u32) -> Self {
        self.max_line_length = length;
        self
    }

    /// Get indent string
    pub fn indent(&self) -> String {
        if self.insert_spaces {
            " ".repeat(self.tab_size as usize)
        } else {
            "\t".to_string()
        }
    }

    /// Get indent string for level
    pub fn indent_level(&self, level: u32) -> String {
        self.indent().repeat(level as usize)
    }
}

/// Language-specific formatting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageFormattingConfig {
    /// Formatter to use
    pub formatter: Option<String>,
    /// Format on save
    pub format_on_save: bool,
    /// Format on paste
    pub format_on_paste: bool,
    /// Format on type
    pub format_on_type: bool,
    /// Default formatting options
    pub options: FormattingOptions,
}

impl Default for LanguageFormattingConfig {
    fn default() -> Self {
        Self {
            formatter: None,
            format_on_save: false,
            format_on_paste: false,
            format_on_type: false,
            options: FormattingOptions::default(),
        }
    }
}

/// Indentation detection
pub fn detect_indentation(content: &str) -> FormattingOptions {
    let mut space_lines = 0;
    let mut tab_lines = 0;
    let mut space_counts: HashMap<usize, usize> = HashMap::new();

    for line in content.lines() {
        if line.is_empty() {
            continue;
        }

        let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
        if indent.is_empty() {
            continue;
        }

        if indent.contains('\t') {
            tab_lines += 1;
        } else {
            space_lines += 1;
            let count = indent.len();
            *space_counts.entry(count).or_default() += 1;
        }
    }

    let mut options = FormattingOptions::default();

    if tab_lines > space_lines {
        options.insert_spaces = false;
    } else {
        options.insert_spaces = true;
        
        // Detect tab size from most common indent
        if !space_counts.is_empty() {
            // Find GCD of indent sizes
            let sizes: Vec<usize> = space_counts.keys().copied().collect();
            if let Some(&min_size) = sizes.iter().filter(|&&s| s > 0).min() {
                let tab_size = if min_size <= 8 { min_size } else { 4 };
                options.tab_size = tab_size as u32;
            }
        }
    }

    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_spaces() {
        let content = "fn main() {\n    let x = 1;\n    let y = 2;\n}";
        let options = detect_indentation(content);
        assert!(options.insert_spaces);
        assert_eq!(options.tab_size, 4);
    }

    #[test]
    fn test_detect_tabs() {
        let content = "fn main() {\n\tlet x = 1;\n\tlet y = 2;\n}";
        let options = detect_indentation(content);
        assert!(!options.insert_spaces);
    }
}
