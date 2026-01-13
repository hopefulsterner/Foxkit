//! # Foxkit Comment Actions
//!
//! Toggle and manipulate comments in code.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Comment actions service
pub struct CommentActionsService {
    /// Language comment configurations
    configs: RwLock<HashMap<String, CommentConfig>>,
}

impl CommentActionsService {
    pub fn new() -> Self {
        let mut configs = HashMap::new();
        
        // Register built-in configs
        configs.insert("rust".to_string(), CommentConfig::rust());
        configs.insert("javascript".to_string(), CommentConfig::c_style());
        configs.insert("typescript".to_string(), CommentConfig::c_style());
        configs.insert("python".to_string(), CommentConfig::python());
        configs.insert("html".to_string(), CommentConfig::html());
        configs.insert("css".to_string(), CommentConfig::css());
        configs.insert("go".to_string(), CommentConfig::c_style());
        configs.insert("java".to_string(), CommentConfig::c_style());
        configs.insert("c".to_string(), CommentConfig::c_style());
        configs.insert("cpp".to_string(), CommentConfig::c_style());
        configs.insert("csharp".to_string(), CommentConfig::c_style());
        configs.insert("ruby".to_string(), CommentConfig::ruby());
        configs.insert("shell".to_string(), CommentConfig::hash());
        configs.insert("yaml".to_string(), CommentConfig::hash());
        configs.insert("toml".to_string(), CommentConfig::hash());
        configs.insert("sql".to_string(), CommentConfig::sql());
        configs.insert("lua".to_string(), CommentConfig::lua());

        Self {
            configs: RwLock::new(configs),
        }
    }

    /// Register language config
    pub fn register_config(&self, language: impl Into<String>, config: CommentConfig) {
        self.configs.write().insert(language.into(), config);
    }

    /// Get config for language
    pub fn get_config(&self, language: &str) -> Option<CommentConfig> {
        self.configs.read().get(language).cloned()
    }

    /// Toggle line comment
    pub fn toggle_line_comment(
        &self,
        content: &str,
        language: &str,
        selection: Selection,
    ) -> Option<CommentEdit> {
        let config = self.get_config(language)?;
        let line_comment = config.line_comment.as_ref()?;

        let lines: Vec<&str> = content.lines().collect();
        let start_line = selection.start_line as usize;
        let end_line = selection.end_line as usize;

        if end_line >= lines.len() {
            return None;
        }

        // Check if all lines are commented
        let all_commented = (start_line..=end_line).all(|i| {
            lines[i].trim_start().starts_with(line_comment)
        });

        let mut edits = Vec::new();

        if all_commented {
            // Uncomment
            for i in start_line..=end_line {
                let line = lines[i];
                let trimmed_start = line.len() - line.trim_start().len();
                
                if line.trim_start().starts_with(line_comment) {
                    let comment_end = trimmed_start + line_comment.len();
                    // Remove space after comment if present
                    let remove_end = if line.len() > comment_end && line.chars().nth(comment_end) == Some(' ') {
                        comment_end + 1
                    } else {
                        comment_end
                    };

                    edits.push(TextEdit {
                        start_line: i as u32,
                        start_col: trimmed_start as u32,
                        end_line: i as u32,
                        end_col: remove_end as u32,
                        new_text: String::new(),
                    });
                }
            }
        } else {
            // Comment
            // Find minimum indent
            let min_indent = (start_line..=end_line)
                .filter_map(|i| {
                    let line = lines[i];
                    if line.trim().is_empty() {
                        None
                    } else {
                        Some(line.len() - line.trim_start().len())
                    }
                })
                .min()
                .unwrap_or(0);

            for i in start_line..=end_line {
                let line = lines[i];
                if !line.trim().is_empty() {
                    edits.push(TextEdit {
                        start_line: i as u32,
                        start_col: min_indent as u32,
                        end_line: i as u32,
                        end_col: min_indent as u32,
                        new_text: format!("{} ", line_comment),
                    });
                }
            }
        }

        Some(CommentEdit {
            action: if all_commented {
                CommentAction::Uncomment
            } else {
                CommentAction::Comment
            },
            edits,
        })
    }

    /// Toggle block comment
    pub fn toggle_block_comment(
        &self,
        content: &str,
        language: &str,
        selection: Selection,
    ) -> Option<CommentEdit> {
        let config = self.get_config(language)?;
        let block = config.block_comment.as_ref()?;

        // Check if selection is already block commented
        let selected_text = extract_selection(content, &selection);
        let trimmed = selected_text.trim();

        let is_commented = trimmed.starts_with(&block.start) && trimmed.ends_with(&block.end);

        let mut edits = Vec::new();

        if is_commented {
            // Uncomment - remove start and end markers
            let text_start = selected_text.find(&block.start).unwrap_or(0);
            let text_end = selected_text.rfind(&block.end).unwrap_or(selected_text.len());

            // This is simplified - would need proper position calculation
            edits.push(TextEdit {
                start_line: selection.start_line,
                start_col: selection.start_col,
                end_line: selection.end_line,
                end_col: selection.end_col,
                new_text: selected_text[text_start + block.start.len()..text_end].to_string(),
            });
        } else {
            // Comment - wrap with block comment
            edits.push(TextEdit {
                start_line: selection.start_line,
                start_col: selection.start_col,
                end_line: selection.end_line,
                end_col: selection.end_col,
                new_text: format!("{}{}{}", block.start, selected_text, block.end),
            });
        }

        Some(CommentEdit {
            action: if is_commented {
                CommentAction::Uncomment
            } else {
                CommentAction::Comment
            },
            edits,
        })
    }

    /// Add block comment
    pub fn add_block_comment(&self, language: &str, selection: Selection) -> Option<CommentEdit> {
        let config = self.get_config(language)?;
        let block = config.block_comment.as_ref()?;

        let edits = vec![
            // Insert start
            TextEdit {
                start_line: selection.start_line,
                start_col: selection.start_col,
                end_line: selection.start_line,
                end_col: selection.start_col,
                new_text: block.start.clone(),
            },
            // Insert end
            TextEdit {
                start_line: selection.end_line,
                start_col: selection.end_col,
                end_line: selection.end_line,
                end_col: selection.end_col,
                new_text: block.end.clone(),
            },
        ];

        Some(CommentEdit {
            action: CommentAction::Comment,
            edits,
        })
    }
}

impl Default for CommentActionsService {
    fn default() -> Self {
        Self::new()
    }
}

fn extract_selection(content: &str, selection: &Selection) -> String {
    let lines: Vec<&str> = content.lines().collect();
    
    if selection.start_line == selection.end_line {
        let line = lines.get(selection.start_line as usize).unwrap_or(&"");
        line.chars()
            .skip(selection.start_col as usize)
            .take((selection.end_col - selection.start_col) as usize)
            .collect()
    } else {
        let mut result = String::new();
        
        for i in selection.start_line..=selection.end_line {
            let line = lines.get(i as usize).unwrap_or(&"");
            
            if i == selection.start_line {
                result.push_str(&line.chars().skip(selection.start_col as usize).collect::<String>());
                result.push('\n');
            } else if i == selection.end_line {
                result.push_str(&line.chars().take(selection.end_col as usize).collect::<String>());
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        result
    }
}

/// Comment configuration for a language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentConfig {
    /// Line comment string
    pub line_comment: Option<String>,
    /// Block comment
    pub block_comment: Option<BlockComment>,
}

impl CommentConfig {
    pub fn new() -> Self {
        Self {
            line_comment: None,
            block_comment: None,
        }
    }

    pub fn with_line(mut self, comment: impl Into<String>) -> Self {
        self.line_comment = Some(comment.into());
        self
    }

    pub fn with_block(mut self, start: impl Into<String>, end: impl Into<String>) -> Self {
        self.block_comment = Some(BlockComment {
            start: start.into(),
            end: end.into(),
        });
        self
    }

    /// C-style comments (// and /* */)
    pub fn c_style() -> Self {
        Self::new()
            .with_line("//")
            .with_block("/*", "*/")
    }

    /// Rust comments
    pub fn rust() -> Self {
        Self::c_style()
    }

    /// Python comments
    pub fn python() -> Self {
        Self::new()
            .with_line("#")
            .with_block("\"\"\"", "\"\"\"")
    }

    /// HTML comments
    pub fn html() -> Self {
        Self::new()
            .with_block("<!--", "-->")
    }

    /// CSS comments
    pub fn css() -> Self {
        Self::new()
            .with_block("/*", "*/")
    }

    /// Ruby comments
    pub fn ruby() -> Self {
        Self::new()
            .with_line("#")
            .with_block("=begin", "=end")
    }

    /// Hash comments
    pub fn hash() -> Self {
        Self::new()
            .with_line("#")
    }

    /// SQL comments
    pub fn sql() -> Self {
        Self::new()
            .with_line("--")
            .with_block("/*", "*/")
    }

    /// Lua comments
    pub fn lua() -> Self {
        Self::new()
            .with_line("--")
            .with_block("--[[", "]]")
    }
}

impl Default for CommentConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Block comment markers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockComment {
    pub start: String,
    pub end: String,
}

/// Selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl Selection {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    pub fn line(line: u32) -> Self {
        Self {
            start_line: line,
            start_col: 0,
            end_line: line,
            end_col: u32::MAX,
        }
    }

    pub fn lines(start: u32, end: u32) -> Self {
        Self {
            start_line: start,
            start_col: 0,
            end_line: end,
            end_col: u32::MAX,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start_line == self.end_line && self.start_col == self.end_col
    }

    pub fn is_single_line(&self) -> bool {
        self.start_line == self.end_line
    }
}

/// Text edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub new_text: String,
}

/// Comment edit result
#[derive(Debug, Clone)]
pub struct CommentEdit {
    pub action: CommentAction,
    pub edits: Vec<TextEdit>,
}

/// Comment action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentAction {
    Comment,
    Uncomment,
}
