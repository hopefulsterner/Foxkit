//! # Foxkit Document Formatting
//!
//! Code formatting service with range and on-type support.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Document formatting service
pub struct FormattingService {
    /// Registered formatters
    formatters: RwLock<HashMap<String, Arc<dyn Formatter>>>,
    /// Events
    events: broadcast::Sender<FormattingEvent>,
    /// Configuration
    config: RwLock<FormattingConfig>,
}

impl FormattingService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            formatters: RwLock::new(HashMap::new()),
            events,
            config: RwLock::new(FormattingConfig::default()),
        }
    }

    /// Register a formatter
    pub fn register_formatter(&self, language: impl Into<String>, formatter: Arc<dyn Formatter>) {
        self.formatters.write().insert(language.into(), formatter);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<FormattingEvent> {
        self.events.subscribe()
    }

    /// Configure formatting
    pub fn configure(&self, config: FormattingConfig) {
        *self.config.write() = config;
    }

    /// Format entire document
    pub async fn format_document(
        &self,
        file: &PathBuf,
        language: &str,
        content: &str,
        options: FormattingOptions,
    ) -> anyhow::Result<Vec<TextEdit>> {
        let formatters = self.formatters.read();
        
        let formatter = formatters.get(language)
            .or_else(|| formatters.get("*"))
            .ok_or_else(|| anyhow::anyhow!("No formatter for {}", language))?;

        let _ = self.events.send(FormattingEvent::Started { 
            file: file.clone(),
            kind: FormattingKind::Document,
        });

        let edits = formatter.format(content, &options).await?;

        let _ = self.events.send(FormattingEvent::Completed {
            file: file.clone(),
            edit_count: edits.len(),
        });

        Ok(edits)
    }

    /// Format range
    pub async fn format_range(
        &self,
        file: &PathBuf,
        language: &str,
        content: &str,
        range: FormatRange,
        options: FormattingOptions,
    ) -> anyhow::Result<Vec<TextEdit>> {
        let formatters = self.formatters.read();
        
        let formatter = formatters.get(language)
            .or_else(|| formatters.get("*"))
            .ok_or_else(|| anyhow::anyhow!("No formatter for {}", language))?;

        let _ = self.events.send(FormattingEvent::Started {
            file: file.clone(),
            kind: FormattingKind::Range,
        });

        let edits = formatter.format_range(content, &range, &options).await?;

        let _ = self.events.send(FormattingEvent::Completed {
            file: file.clone(),
            edit_count: edits.len(),
        });

        Ok(edits)
    }

    /// Format on type
    pub async fn format_on_type(
        &self,
        file: &PathBuf,
        language: &str,
        content: &str,
        position: FormatPosition,
        char_typed: char,
        options: FormattingOptions,
    ) -> anyhow::Result<Vec<TextEdit>> {
        let config = self.config.read();
        
        if !config.format_on_type {
            return Ok(Vec::new());
        }

        // Check if char is a trigger
        if !config.format_on_type_triggers.contains(&char_typed) {
            return Ok(Vec::new());
        }

        let formatters = self.formatters.read();
        
        let formatter = formatters.get(language)
            .or_else(|| formatters.get("*"))
            .ok_or_else(|| anyhow::anyhow!("No formatter for {}", language))?;

        formatter.format_on_type(content, &position, char_typed, &options).await
    }

    /// Get default options
    pub fn default_options(&self) -> FormattingOptions {
        let config = self.config.read();
        
        FormattingOptions {
            tab_size: config.tab_size,
            insert_spaces: config.insert_spaces,
            trim_trailing_whitespace: config.trim_trailing_whitespace,
            insert_final_newline: config.insert_final_newline,
            trim_final_newlines: config.trim_final_newlines,
        }
    }
}

impl Default for FormattingService {
    fn default() -> Self {
        Self::new()
    }
}

/// Formatter trait
#[async_trait::async_trait]
pub trait Formatter: Send + Sync {
    /// Formatter ID
    fn id(&self) -> &str;

    /// Format entire content
    async fn format(&self, content: &str, options: &FormattingOptions) -> anyhow::Result<Vec<TextEdit>>;

    /// Format range
    async fn format_range(
        &self,
        content: &str,
        range: &FormatRange,
        options: &FormattingOptions,
    ) -> anyhow::Result<Vec<TextEdit>> {
        // Default: format entire document
        self.format(content, options).await
    }

    /// Format on type
    async fn format_on_type(
        &self,
        content: &str,
        position: &FormatPosition,
        char_typed: char,
        options: &FormattingOptions,
    ) -> anyhow::Result<Vec<TextEdit>> {
        Ok(Vec::new())
    }
}

/// Text edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    /// Range to replace
    pub range: FormatRange,
    /// New text
    pub new_text: String,
}

impl TextEdit {
    pub fn new(range: FormatRange, new_text: impl Into<String>) -> Self {
        Self { range, new_text: new_text.into() }
    }

    pub fn replace(
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
        new_text: impl Into<String>,
    ) -> Self {
        Self {
            range: FormatRange::new(start_line, start_col, end_line, end_col),
            new_text: new_text.into(),
        }
    }

    pub fn insert(line: u32, col: u32, text: impl Into<String>) -> Self {
        Self {
            range: FormatRange::new(line, col, line, col),
            new_text: text.into(),
        }
    }

    pub fn delete(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            range: FormatRange::new(start_line, start_col, end_line, end_col),
            new_text: String::new(),
        }
    }
}

/// Format range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl FormatRange {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
    }

    pub fn full_document(line_count: u32) -> Self {
        Self {
            start_line: 0,
            start_col: 0,
            end_line: line_count,
            end_col: 0,
        }
    }
}

/// Format position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatPosition {
    pub line: u32,
    pub col: u32,
}

impl FormatPosition {
    pub fn new(line: u32, col: u32) -> Self {
        Self { line, col }
    }
}

/// Formatting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingOptions {
    /// Tab size
    pub tab_size: u32,
    /// Insert spaces instead of tabs
    pub insert_spaces: bool,
    /// Trim trailing whitespace
    pub trim_trailing_whitespace: bool,
    /// Insert final newline
    pub insert_final_newline: bool,
    /// Trim final newlines
    pub trim_final_newlines: bool,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: true,
            insert_final_newline: true,
            trim_final_newlines: true,
        }
    }
}

/// Formatting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingConfig {
    /// Default tab size
    pub tab_size: u32,
    /// Insert spaces
    pub insert_spaces: bool,
    /// Trim trailing whitespace
    pub trim_trailing_whitespace: bool,
    /// Insert final newline
    pub insert_final_newline: bool,
    /// Trim final newlines
    pub trim_final_newlines: bool,
    /// Format on save
    pub format_on_save: bool,
    /// Format on paste
    pub format_on_paste: bool,
    /// Format on type
    pub format_on_type: bool,
    /// Format on type trigger characters
    pub format_on_type_triggers: Vec<char>,
    /// Default formatter by language
    pub default_formatter: HashMap<String, String>,
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: true,
            insert_final_newline: true,
            trim_final_newlines: true,
            format_on_save: false,
            format_on_paste: false,
            format_on_type: false,
            format_on_type_triggers: vec!['\n', ';', '}'],
            default_formatter: HashMap::new(),
        }
    }
}

/// Formatting event
#[derive(Debug, Clone)]
pub enum FormattingEvent {
    Started { file: PathBuf, kind: FormattingKind },
    Completed { file: PathBuf, edit_count: usize },
    Failed { file: PathBuf, error: String },
}

/// Formatting kind
#[derive(Debug, Clone, Copy)]
pub enum FormattingKind {
    Document,
    Range,
    OnType,
    OnSave,
    OnPaste,
}

/// Prettier formatter (example)
pub struct PrettierFormatter {
    config_path: Option<PathBuf>,
}

impl PrettierFormatter {
    pub fn new() -> Self {
        Self { config_path: None }
    }

    pub fn with_config(mut self, path: PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }
}

impl Default for PrettierFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Formatter for PrettierFormatter {
    fn id(&self) -> &str {
        "prettier"
    }

    async fn format(&self, content: &str, options: &FormattingOptions) -> anyhow::Result<Vec<TextEdit>> {
        // Would call prettier CLI or API
        Ok(Vec::new())
    }
}

/// Rustfmt formatter
pub struct RustfmtFormatter {
    edition: String,
}

impl RustfmtFormatter {
    pub fn new() -> Self {
        Self { edition: "2021".to_string() }
    }

    pub fn with_edition(mut self, edition: impl Into<String>) -> Self {
        self.edition = edition.into();
        self
    }
}

impl Default for RustfmtFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Formatter for RustfmtFormatter {
    fn id(&self) -> &str {
        "rustfmt"
    }

    async fn format(&self, content: &str, options: &FormattingOptions) -> anyhow::Result<Vec<TextEdit>> {
        // Would call rustfmt CLI or API
        Ok(Vec::new())
    }
}
