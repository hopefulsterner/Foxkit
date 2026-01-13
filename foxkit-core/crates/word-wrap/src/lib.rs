//! # Foxkit Word Wrap
//!
//! Text wrapping configuration and rendering.

use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Word wrap service
pub struct WordWrapService {
    /// Configuration
    config: RwLock<WordWrapConfig>,
    /// Language-specific overrides
    overrides: RwLock<HashMap<String, WordWrapConfig>>,
}

impl WordWrapService {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(WordWrapConfig::default()),
            overrides: RwLock::new(HashMap::new()),
        }
    }

    /// Configure default word wrap
    pub fn configure(&self, config: WordWrapConfig) {
        *self.config.write() = config;
    }

    /// Set language override
    pub fn set_override(&self, language: impl Into<String>, config: WordWrapConfig) {
        self.overrides.write().insert(language.into(), config);
    }

    /// Get config for language
    pub fn get_config(&self, language: Option<&str>) -> WordWrapConfig {
        if let Some(lang) = language {
            if let Some(config) = self.overrides.read().get(lang) {
                return config.clone();
            }
        }
        self.config.read().clone()
    }

    /// Compute wrapped lines
    pub fn compute_wrapped_lines(
        &self,
        content: &str,
        viewport_width: u32,
        language: Option<&str>,
    ) -> Vec<WrappedLine> {
        let config = self.get_config(language);

        if !config.enabled {
            return content
                .lines()
                .enumerate()
                .map(|(i, line)| WrappedLine {
                    original_line: i as u32,
                    content: line.to_string(),
                    is_wrapped: false,
                    indent: 0,
                })
                .collect();
        }

        let wrap_width = match config.wrap_column {
            WrapColumn::Viewport => viewport_width,
            WrapColumn::Fixed(col) => col,
            WrapColumn::Bounded { min, max } => viewport_width.clamp(min, max),
        };

        let mut wrapped = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let line_wrapped = self.wrap_line(line, wrap_width, &config);
            
            for (i, segment) in line_wrapped.into_iter().enumerate() {
                wrapped.push(WrappedLine {
                    original_line: line_num as u32,
                    content: segment,
                    is_wrapped: i > 0,
                    indent: if i > 0 { config.wrapped_indent } else { 0 },
                });
            }
        }

        wrapped
    }

    /// Wrap a single line
    fn wrap_line(&self, line: &str, width: u32, config: &WordWrapConfig) -> Vec<String> {
        if line.len() as u32 <= width {
            return vec![line.to_string()];
        }

        let mut segments = Vec::new();
        let mut remaining = line;

        while !remaining.is_empty() {
            let max_len = (width as usize).min(remaining.len());
            
            // Find wrap point
            let wrap_point = match config.wrap_style {
                WrapStyle::Word => {
                    // Find last space before max_len
                    remaining[..max_len]
                        .rfind(|c: char| c.is_whitespace())
                        .unwrap_or(max_len)
                }
                WrapStyle::Character => max_len,
                WrapStyle::WordBounded => {
                    // Word wrap with minimum segment length
                    if let Some(pos) = remaining[..max_len].rfind(|c: char| c.is_whitespace()) {
                        if pos > 10 {
                            pos
                        } else {
                            max_len
                        }
                    } else {
                        max_len
                    }
                }
            };

            let (segment, rest) = remaining.split_at(wrap_point);
            segments.push(segment.trim_end().to_string());
            remaining = rest.trim_start();
        }

        if segments.is_empty() {
            segments.push(String::new());
        }

        segments
    }

    /// Toggle word wrap
    pub fn toggle(&self) {
        let mut config = self.config.write();
        config.enabled = !config.enabled;
    }

    /// Is word wrap enabled
    pub fn is_enabled(&self) -> bool {
        self.config.read().enabled
    }

    /// Get wrapped line count
    pub fn wrapped_line_count(&self, content: &str, viewport_width: u32, language: Option<&str>) -> usize {
        self.compute_wrapped_lines(content, viewport_width, language).len()
    }

    /// Convert wrapped line number to original
    pub fn wrapped_to_original(&self, wrapped_lines: &[WrappedLine], wrapped_line: u32) -> u32 {
        wrapped_lines
            .get(wrapped_line as usize)
            .map(|l| l.original_line)
            .unwrap_or(0)
    }

    /// Convert original line to first wrapped line
    pub fn original_to_wrapped(&self, wrapped_lines: &[WrappedLine], original_line: u32) -> u32 {
        wrapped_lines
            .iter()
            .position(|l| l.original_line == original_line)
            .map(|p| p as u32)
            .unwrap_or(0)
    }
}

impl Default for WordWrapService {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapped line
#[derive(Debug, Clone)]
pub struct WrappedLine {
    /// Original line number
    pub original_line: u32,
    /// Content
    pub content: String,
    /// Is this a continuation
    pub is_wrapped: bool,
    /// Additional indent
    pub indent: u32,
}

impl WrappedLine {
    pub fn display_indent(&self) -> String {
        " ".repeat(self.indent as usize)
    }
}

/// Word wrap configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordWrapConfig {
    /// Enable word wrap
    pub enabled: bool,
    /// Wrap column
    pub wrap_column: WrapColumn,
    /// Wrap style
    pub wrap_style: WrapStyle,
    /// Additional indent for wrapped lines
    pub wrapped_indent: u32,
    /// Wrap on whitespace only
    pub whitespace_only: bool,
}

impl Default for WordWrapConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            wrap_column: WrapColumn::Viewport,
            wrap_style: WrapStyle::Word,
            wrapped_indent: 0,
            whitespace_only: true,
        }
    }
}

/// Wrap column setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WrapColumn {
    /// Wrap at viewport width
    Viewport,
    /// Fixed column
    Fixed(u32),
    /// Bounded (min/max)
    Bounded { min: u32, max: u32 },
}

/// Wrap style
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WrapStyle {
    /// Break at word boundaries
    Word,
    /// Break at any character
    Character,
    /// Word with fallback to character
    WordBounded,
}
