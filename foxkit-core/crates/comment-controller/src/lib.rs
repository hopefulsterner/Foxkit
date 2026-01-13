//! # Foxkit Comment Controller
//!
//! Code commenting features with language-aware comment styles.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Comment controller service
pub struct CommentControllerService {
    /// Comment configurations per language
    configs: RwLock<HashMap<String, CommentConfig>>,
    /// Events
    events: broadcast::Sender<CommentEvent>,
}

impl CommentControllerService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);
        let mut configs = HashMap::new();

        // Register default configurations
        for (lang, config) in default_comment_configs() {
            configs.insert(lang, config);
        }

        Self {
            configs: RwLock::new(configs),
            events,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<CommentEvent> {
        self.events.subscribe()
    }

    /// Register comment configuration for a language
    pub fn register_config(&self, language: impl Into<String>, config: CommentConfig) {
        self.configs.write().insert(language.into(), config);
    }

    /// Get configuration for language
    pub fn get_config(&self, language: &str) -> Option<CommentConfig> {
        self.configs.read().get(language).cloned()
    }

    /// Toggle line comment
    pub fn toggle_line_comment(
        &self,
        language: &str,
        lines: &[String],
        selection: &CommentSelection,
    ) -> CommentResult {
        let config = match self.get_config(language) {
            Some(c) => c,
            None => return CommentResult::no_change(),
        };

        let line_comment = match &config.line_comment {
            Some(c) => c,
            None => return CommentResult::no_change(),
        };

        // Check if all lines are already commented
        let all_commented = selection.lines().all(|line_idx| {
            if let Some(line) = lines.get(line_idx as usize) {
                line.trim_start().starts_with(line_comment)
            } else {
                false
            }
        });

        let edits: Vec<CommentEdit> = if all_commented {
            // Uncomment
            selection.lines()
                .filter_map(|line_idx| {
                    let line = lines.get(line_idx as usize)?;
                    let trimmed = line.trim_start();
                    
                    if trimmed.starts_with(line_comment) {
                        let indent = line.len() - trimmed.len();
                        let after_comment = trimmed.strip_prefix(line_comment)?;
                        let after_space = after_comment.strip_prefix(' ').unwrap_or(after_comment);
                        
                        Some(CommentEdit::replace(
                            line_idx,
                            0,
                            line.len() as u32,
                            format!("{}{}", " ".repeat(indent), after_space),
                        ))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            // Comment
            // Find minimum indentation
            let min_indent = selection.lines()
                .filter_map(|line_idx| lines.get(line_idx as usize))
                .filter(|line| !line.trim().is_empty())
                .map(|line| line.len() - line.trim_start().len())
                .min()
                .unwrap_or(0);

            selection.lines()
                .filter_map(|line_idx| {
                    let line = lines.get(line_idx as usize)?;
                    
                    if line.trim().is_empty() {
                        None // Don't comment empty lines
                    } else {
                        Some(CommentEdit::insert(
                            line_idx,
                            min_indent as u32,
                            format!("{} ", line_comment),
                        ))
                    }
                })
                .collect()
        };

        let _ = self.events.send(CommentEvent::LineCommentToggled {
            commented: !all_commented,
            line_count: selection.line_count(),
        });

        CommentResult { edits }
    }

    /// Toggle block comment
    pub fn toggle_block_comment(
        &self,
        language: &str,
        content: &str,
        selection: &CommentSelection,
    ) -> CommentResult {
        let config = match self.get_config(language) {
            Some(c) => c,
            None => return CommentResult::no_change(),
        };

        let block_comment = match &config.block_comment {
            Some(c) => c,
            None => {
                // Fall back to line comment
                return CommentResult::no_change();
            }
        };

        // Get selected text
        let lines: Vec<&str> = content.lines().collect();
        let selected_text = selection.extract_text(&lines);
        let trimmed = selected_text.trim();

        // Check if already has block comment
        let has_block = trimmed.starts_with(&block_comment.start) &&
            trimmed.ends_with(&block_comment.end);

        let edits = if has_block {
            // Remove block comment
            vec![
                CommentEdit::delete_at_selection_start(
                    selection,
                    block_comment.start.len() as u32,
                ),
                CommentEdit::delete_at_selection_end(
                    selection,
                    block_comment.end.len() as u32,
                ),
            ]
        } else {
            // Add block comment
            vec![
                CommentEdit::insert_at_selection_start(
                    selection,
                    block_comment.start.clone(),
                ),
                CommentEdit::insert_at_selection_end(
                    selection,
                    block_comment.end.clone(),
                ),
            ]
        };

        let _ = self.events.send(CommentEvent::BlockCommentToggled {
            commented: !has_block,
        });

        CommentResult { edits }
    }

    /// Add documentation comment
    pub fn insert_doc_comment(
        &self,
        language: &str,
        line: u32,
        indent: u32,
    ) -> Option<CommentResult> {
        let config = self.get_config(language)?;
        let doc = config.doc_comment.as_ref()?;

        let indent_str = " ".repeat(indent as usize);
        
        let text = match doc {
            DocComment::Line(prefix) => {
                format!("{}{} ", indent_str, prefix)
            }
            DocComment::Block { start, end, line_prefix } => {
                let prefix = line_prefix.as_deref().unwrap_or(" * ");
                format!(
                    "{}{}\n{}{}\n{}{}",
                    indent_str, start,
                    indent_str, prefix,
                    indent_str, end
                )
            }
        };

        Some(CommentResult {
            edits: vec![CommentEdit::insert(line, 0, text)],
        })
    }
}

impl Default for CommentControllerService {
    fn default() -> Self {
        Self::new()
    }
}

/// Default comment configurations
fn default_comment_configs() -> Vec<(String, CommentConfig)> {
    vec![
        ("rust".to_string(), CommentConfig {
            line_comment: Some("//".to_string()),
            block_comment: Some(BlockComment::new("/*", "*/")),
            doc_comment: Some(DocComment::Line("///".to_string())),
        }),
        ("javascript".to_string(), CommentConfig {
            line_comment: Some("//".to_string()),
            block_comment: Some(BlockComment::new("/*", "*/")),
            doc_comment: Some(DocComment::Block {
                start: "/**".to_string(),
                end: " */".to_string(),
                line_prefix: Some(" * ".to_string()),
            }),
        }),
        ("typescript".to_string(), CommentConfig {
            line_comment: Some("//".to_string()),
            block_comment: Some(BlockComment::new("/*", "*/")),
            doc_comment: Some(DocComment::Block {
                start: "/**".to_string(),
                end: " */".to_string(),
                line_prefix: Some(" * ".to_string()),
            }),
        }),
        ("python".to_string(), CommentConfig {
            line_comment: Some("#".to_string()),
            block_comment: None,
            doc_comment: Some(DocComment::Block {
                start: "\"\"\"".to_string(),
                end: "\"\"\"".to_string(),
                line_prefix: None,
            }),
        }),
        ("html".to_string(), CommentConfig {
            line_comment: None,
            block_comment: Some(BlockComment::new("<!--", "-->")),
            doc_comment: None,
        }),
        ("css".to_string(), CommentConfig {
            line_comment: None,
            block_comment: Some(BlockComment::new("/*", "*/")),
            doc_comment: None,
        }),
        ("c".to_string(), CommentConfig {
            line_comment: Some("//".to_string()),
            block_comment: Some(BlockComment::new("/*", "*/")),
            doc_comment: Some(DocComment::Block {
                start: "/**".to_string(),
                end: " */".to_string(),
                line_prefix: Some(" * ".to_string()),
            }),
        }),
        ("cpp".to_string(), CommentConfig {
            line_comment: Some("//".to_string()),
            block_comment: Some(BlockComment::new("/*", "*/")),
            doc_comment: Some(DocComment::Line("///".to_string())),
        }),
        ("go".to_string(), CommentConfig {
            line_comment: Some("//".to_string()),
            block_comment: Some(BlockComment::new("/*", "*/")),
            doc_comment: Some(DocComment::Line("//".to_string())),
        }),
        ("java".to_string(), CommentConfig {
            line_comment: Some("//".to_string()),
            block_comment: Some(BlockComment::new("/*", "*/")),
            doc_comment: Some(DocComment::Block {
                start: "/**".to_string(),
                end: " */".to_string(),
                line_prefix: Some(" * ".to_string()),
            }),
        }),
        ("ruby".to_string(), CommentConfig {
            line_comment: Some("#".to_string()),
            block_comment: Some(BlockComment::new("=begin", "=end")),
            doc_comment: None,
        }),
        ("shell".to_string(), CommentConfig {
            line_comment: Some("#".to_string()),
            block_comment: None,
            doc_comment: None,
        }),
        ("sql".to_string(), CommentConfig {
            line_comment: Some("--".to_string()),
            block_comment: Some(BlockComment::new("/*", "*/")),
            doc_comment: None,
        }),
        ("lua".to_string(), CommentConfig {
            line_comment: Some("--".to_string()),
            block_comment: Some(BlockComment::new("--[[", "]]")),
            doc_comment: None,
        }),
    ]
}

/// Comment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentConfig {
    /// Line comment prefix
    pub line_comment: Option<String>,
    /// Block comment
    pub block_comment: Option<BlockComment>,
    /// Documentation comment
    pub doc_comment: Option<DocComment>,
}

/// Block comment markers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockComment {
    pub start: String,
    pub end: String,
}

impl BlockComment {
    pub fn new(start: impl Into<String>, end: impl Into<String>) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
        }
    }
}

/// Documentation comment style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocComment {
    /// Single-line doc comment prefix (e.g., "///")
    Line(String),
    /// Block-style doc comment
    Block {
        start: String,
        end: String,
        line_prefix: Option<String>,
    },
}

/// Comment selection
#[derive(Debug, Clone)]
pub struct CommentSelection {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl CommentSelection {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
    }

    pub fn line_range(start: u32, end: u32) -> Self {
        Self {
            start_line: start,
            start_col: 0,
            end_line: end,
            end_col: u32::MAX,
        }
    }

    pub fn lines(&self) -> impl Iterator<Item = u32> {
        self.start_line..=self.end_line
    }

    pub fn line_count(&self) -> u32 {
        self.end_line - self.start_line + 1
    }

    pub fn extract_text(&self, lines: &[&str]) -> String {
        lines[self.start_line as usize..=self.end_line as usize]
            .join("\n")
    }
}

/// Comment edit
#[derive(Debug, Clone)]
pub struct CommentEdit {
    pub line: u32,
    pub start_col: u32,
    pub end_col: Option<u32>,
    pub text: String,
    pub kind: CommentEditKind,
}

#[derive(Debug, Clone)]
pub enum CommentEditKind {
    Insert,
    Delete,
    Replace,
}

impl CommentEdit {
    pub fn insert(line: u32, col: u32, text: impl Into<String>) -> Self {
        Self {
            line,
            start_col: col,
            end_col: None,
            text: text.into(),
            kind: CommentEditKind::Insert,
        }
    }

    pub fn delete(line: u32, start_col: u32, end_col: u32) -> Self {
        Self {
            line,
            start_col,
            end_col: Some(end_col),
            text: String::new(),
            kind: CommentEditKind::Delete,
        }
    }

    pub fn replace(line: u32, start_col: u32, end_col: u32, text: impl Into<String>) -> Self {
        Self {
            line,
            start_col,
            end_col: Some(end_col),
            text: text.into(),
            kind: CommentEditKind::Replace,
        }
    }

    pub fn insert_at_selection_start(selection: &CommentSelection, text: String) -> Self {
        Self::insert(selection.start_line, selection.start_col, text)
    }

    pub fn insert_at_selection_end(selection: &CommentSelection, text: String) -> Self {
        Self::insert(selection.end_line, selection.end_col, text)
    }

    pub fn delete_at_selection_start(selection: &CommentSelection, len: u32) -> Self {
        Self::delete(
            selection.start_line,
            selection.start_col,
            selection.start_col + len,
        )
    }

    pub fn delete_at_selection_end(selection: &CommentSelection, len: u32) -> Self {
        Self::delete(
            selection.end_line,
            selection.end_col.saturating_sub(len),
            selection.end_col,
        )
    }
}

/// Comment result
#[derive(Debug, Clone)]
pub struct CommentResult {
    pub edits: Vec<CommentEdit>,
}

impl CommentResult {
    pub fn no_change() -> Self {
        Self { edits: Vec::new() }
    }

    pub fn has_changes(&self) -> bool {
        !self.edits.is_empty()
    }
}

/// Comment event
#[derive(Debug, Clone)]
pub enum CommentEvent {
    LineCommentToggled { commented: bool, line_count: u32 },
    BlockCommentToggled { commented: bool },
    DocCommentInserted { line: u32 },
}
