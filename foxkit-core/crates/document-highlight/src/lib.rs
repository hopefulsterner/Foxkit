//! # Foxkit Document Highlight
//!
//! Highlight all occurrences of a symbol in a document.

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Document highlight service
pub struct DocumentHighlightService {
    /// Current highlights
    current: RwLock<Option<HighlightState>>,
    /// Events
    events: broadcast::Sender<DocumentHighlightEvent>,
    /// Configuration
    config: RwLock<DocumentHighlightConfig>,
}

impl DocumentHighlightService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            current: RwLock::new(None),
            events,
            config: RwLock::new(DocumentHighlightConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<DocumentHighlightEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: DocumentHighlightConfig) {
        *self.config.write() = config;
    }

    /// Get highlights at position (would call LSP)
    pub async fn get_highlights(
        &self,
        file: &PathBuf,
        position: HighlightPosition,
    ) -> Vec<DocumentHighlight> {
        // Would call LSP textDocument/documentHighlight
        // For now, return empty (to be implemented)
        Vec::new()
    }

    /// Set current highlights
    pub fn set_highlights(&self, file: PathBuf, highlights: Vec<DocumentHighlight>) {
        let state = HighlightState {
            file: file.clone(),
            highlights: highlights.clone(),
        };
        *self.current.write() = Some(state);

        let _ = self.events.send(DocumentHighlightEvent::HighlightsChanged {
            file,
            count: highlights.len(),
        });
    }

    /// Clear highlights
    pub fn clear(&self) {
        if self.current.write().take().is_some() {
            let _ = self.events.send(DocumentHighlightEvent::Cleared);
        }
    }

    /// Get current highlights
    pub fn current(&self) -> Option<HighlightState> {
        self.current.read().clone()
    }

    /// Navigate to next highlight
    pub fn next_highlight(&self, current_pos: HighlightPosition) -> Option<DocumentHighlight> {
        let state = self.current.read();
        let state = state.as_ref()?;

        // Find next highlight after current position
        state.highlights.iter()
            .find(|h| {
                h.range.start_line > current_pos.line ||
                    (h.range.start_line == current_pos.line && h.range.start_col > current_pos.col)
            })
            .or_else(|| state.highlights.first())
            .cloned()
    }

    /// Navigate to previous highlight
    pub fn previous_highlight(&self, current_pos: HighlightPosition) -> Option<DocumentHighlight> {
        let state = self.current.read();
        let state = state.as_ref()?;

        // Find previous highlight before current position
        state.highlights.iter()
            .rev()
            .find(|h| {
                h.range.start_line < current_pos.line ||
                    (h.range.start_line == current_pos.line && h.range.start_col < current_pos.col)
            })
            .or_else(|| state.highlights.last())
            .cloned()
    }

    /// Group highlights by kind
    pub fn highlights_by_kind(&self) -> HighlightsByKind {
        let state = self.current.read();
        
        let mut result = HighlightsByKind::default();
        
        if let Some(ref state) = *state {
            for h in &state.highlights {
                match h.kind {
                    HighlightKind::Text => result.text.push(h.clone()),
                    HighlightKind::Read => result.read.push(h.clone()),
                    HighlightKind::Write => result.write.push(h.clone()),
                }
            }
        }

        result
    }
}

impl Default for DocumentHighlightService {
    fn default() -> Self {
        Self::new()
    }
}

/// Current highlight state
#[derive(Debug, Clone)]
pub struct HighlightState {
    /// File being highlighted
    pub file: PathBuf,
    /// All highlights
    pub highlights: Vec<DocumentHighlight>,
}

impl HighlightState {
    pub fn count(&self) -> usize {
        self.highlights.len()
    }

    pub fn read_count(&self) -> usize {
        self.highlights.iter().filter(|h| h.kind == HighlightKind::Read).count()
    }

    pub fn write_count(&self) -> usize {
        self.highlights.iter().filter(|h| h.kind == HighlightKind::Write).count()
    }
}

/// Document highlight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentHighlight {
    /// Range
    pub range: HighlightRange,
    /// Kind
    pub kind: HighlightKind,
}

impl DocumentHighlight {
    pub fn new(range: HighlightRange, kind: HighlightKind) -> Self {
        Self { range, kind }
    }

    pub fn text(range: HighlightRange) -> Self {
        Self { range, kind: HighlightKind::Text }
    }

    pub fn read(range: HighlightRange) -> Self {
        Self { range, kind: HighlightKind::Read }
    }

    pub fn write(range: HighlightRange) -> Self {
        Self { range, kind: HighlightKind::Write }
    }

    /// Check if position is within this highlight
    pub fn contains(&self, pos: &HighlightPosition) -> bool {
        if pos.line < self.range.start_line || pos.line > self.range.end_line {
            return false;
        }
        if pos.line == self.range.start_line && pos.col < self.range.start_col {
            return false;
        }
        if pos.line == self.range.end_line && pos.col > self.range.end_col {
            return false;
        }
        true
    }
}

/// Highlight kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HighlightKind {
    /// Textual occurrence
    Text,
    /// Read access
    Read,
    /// Write access
    Write,
}

impl HighlightKind {
    pub fn decoration_style(&self) -> &'static str {
        match self {
            Self::Text => "editor.wordHighlightBackground",
            Self::Read => "editor.wordHighlightStrongBackground",
            Self::Write => "editor.wordHighlightTextBackground",
        }
    }

    pub fn border_style(&self) -> Option<&'static str> {
        match self {
            Self::Write => Some("editor.wordHighlightTextBorder"),
            _ => None,
        }
    }
}

/// Highlight range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl HighlightRange {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
    }

    pub fn single_line(line: u32, start_col: u32, end_col: u32) -> Self {
        Self { start_line: line, start_col, end_line: line, end_col }
    }
}

/// Highlight position
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HighlightPosition {
    pub line: u32,
    pub col: u32,
}

impl HighlightPosition {
    pub fn new(line: u32, col: u32) -> Self {
        Self { line, col }
    }
}

/// Highlights grouped by kind
#[derive(Debug, Default, Clone)]
pub struct HighlightsByKind {
    pub text: Vec<DocumentHighlight>,
    pub read: Vec<DocumentHighlight>,
    pub write: Vec<DocumentHighlight>,
}

impl HighlightsByKind {
    pub fn total(&self) -> usize {
        self.text.len() + self.read.len() + self.write.len()
    }
}

/// Document highlight configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentHighlightConfig {
    /// Enable document highlight
    pub enabled: bool,
    /// Delay before requesting highlights (ms)
    pub delay_ms: u64,
    /// Highlight same words
    pub highlight_same_words: bool,
    /// Show read/write distinction
    pub show_access_kind: bool,
}

impl Default for DocumentHighlightConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            delay_ms: 100,
            highlight_same_words: true,
            show_access_kind: true,
        }
    }
}

/// Document highlight event
#[derive(Debug, Clone)]
pub enum DocumentHighlightEvent {
    HighlightsChanged { file: PathBuf, count: usize },
    Cleared,
}

/// Highlight decoration builder
pub struct HighlightDecorationBuilder {
    highlights: Vec<DocumentHighlight>,
}

impl HighlightDecorationBuilder {
    pub fn new(highlights: Vec<DocumentHighlight>) -> Self {
        Self { highlights }
    }

    /// Build decorations for editor
    pub fn build(&self) -> Vec<HighlightDecoration> {
        self.highlights
            .iter()
            .map(|h| HighlightDecoration {
                range: h.range.clone(),
                style: h.kind.decoration_style().to_string(),
                border: h.kind.border_style().map(|s| s.to_string()),
            })
            .collect()
    }
}

/// Highlight decoration
#[derive(Debug, Clone)]
pub struct HighlightDecoration {
    pub range: HighlightRange,
    pub style: String,
    pub border: Option<String>,
}
