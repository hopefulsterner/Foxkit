//! Copilot-style Inline AI Completions for Foxkit
//!
//! Ghost text suggestions, multi-line completions, and intelligent
//! code generation inline with the editor.

pub mod providers;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use uuid::Uuid;

pub use providers::{
    OpenAICompletionProvider,
    AnthropicCompletionProvider,
    OllamaCompletionProvider,
    MultiProviderCompletion,
    FimFormat,
};

/// Unique completion request ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompletionRequestId(pub Uuid);

impl CompletionRequestId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CompletionRequestId {
    fn default() -> Self {
        Self::new()
    }
}

/// Cursor position in the document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Text range in the document
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Context for inline completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineCompletionContext {
    pub file_path: String,
    pub language_id: String,
    pub prefix: String,
    pub suffix: String,
    pub cursor_position: Position,
    pub trigger_kind: TriggerKind,
    pub related_files: Vec<RelatedFile>,
}

/// Related file for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedFile {
    pub path: String,
    pub content: String,
    pub relevance_score: f32,
}

/// How the completion was triggered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerKind {
    Automatic,
    Manual,
    AfterNewline,
    AfterBracket,
}

/// An inline completion suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineCompletion {
    pub id: String,
    pub insert_text: String,
    pub range: Option<Range>,
    pub display_text: Option<String>,
    pub confidence: f32,
    pub kind: CompletionKind,
}

/// Kind of completion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionKind {
    SingleLine,
    MultiLine,
    Block,
    Snippet,
}

/// Inline completion request
#[derive(Debug, Clone)]
pub struct InlineCompletionRequest {
    pub id: CompletionRequestId,
    pub context: InlineCompletionContext,
    pub options: CompletionOptions,
}

/// Options for inline completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOptions {
    pub max_suggestions: usize,
    pub max_tokens: u32,
    pub temperature: f32,
    pub stop_sequences: Vec<String>,
    pub include_related_files: bool,
    pub debounce_ms: u64,
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            max_suggestions: 3,
            max_tokens: 256,
            temperature: 0.2,
            stop_sequences: vec!["\n\n".to_string(), "```".to_string()],
            include_related_files: true,
            debounce_ms: 75,
        }
    }
}

/// Result of inline completion
#[derive(Debug, Clone)]
pub struct InlineCompletionResult {
    pub request_id: CompletionRequestId,
    pub completions: Vec<InlineCompletion>,
    pub is_cached: bool,
    pub latency_ms: u64,
}

/// Inline completion provider trait
#[async_trait]
pub trait InlineCompletionProvider: Send + Sync {
    async fn provide_completions(
        &self,
        request: InlineCompletionRequest,
    ) -> Result<InlineCompletionResult, CompletionError>;

    fn cancel(&self, request_id: CompletionRequestId);
}

/// Completion error
#[derive(Debug, Clone)]
pub enum CompletionError {
    Cancelled,
    Timeout,
    NetworkError(String),
    RateLimited,
    ContextTooLong,
    InternalError(String),
}

impl std::fmt::Display for CompletionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "Completion cancelled"),
            Self::Timeout => write!(f, "Completion timeout"),
            Self::NetworkError(e) => write!(f, "Network error: {}", e),
            Self::RateLimited => write!(f, "Rate limited"),
            Self::ContextTooLong => write!(f, "Context too long"),
            Self::InternalError(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl std::error::Error for CompletionError {}

/// Ghost text display state
#[derive(Debug, Clone)]
pub struct GhostText {
    pub completion: InlineCompletion,
    pub position: Position,
    pub visible: bool,
    pub accepted: bool,
}

/// Telemetry for completions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompletionTelemetry {
    pub shown_count: u64,
    pub accepted_count: u64,
    pub rejected_count: u64,
    pub partial_accepted_count: u64,
    pub avg_latency_ms: f64,
    pub avg_confidence: f64,
}

impl CompletionTelemetry {
    pub fn acceptance_rate(&self) -> f64 {
        if self.shown_count == 0 {
            0.0
        } else {
            self.accepted_count as f64 / self.shown_count as f64
        }
    }
}

/// Cache entry for completions
#[derive(Debug, Clone)]
struct CacheEntry {
    prefix_hash: u64,
    completions: Vec<InlineCompletion>,
    timestamp: std::time::Instant,
}

/// Inline completion service
pub struct InlineCompletionService {
    provider: RwLock<Option<Arc<dyn InlineCompletionProvider>>>,
    current_ghost: RwLock<Option<GhostText>>,
    pending_request: RwLock<Option<CompletionRequestId>>,
    cache: RwLock<VecDeque<CacheEntry>>,
    telemetry: RwLock<CompletionTelemetry>,
    config: RwLock<InlineCompletionConfig>,
}

/// Configuration for inline completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineCompletionConfig {
    pub enabled: bool,
    pub auto_trigger: bool,
    pub debounce_ms: u64,
    pub cache_size: usize,
    pub cache_ttl_ms: u64,
    pub show_confidence: bool,
    pub min_confidence: f32,
    pub excluded_languages: Vec<String>,
}

impl Default for InlineCompletionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_trigger: true,
            debounce_ms: 75,
            cache_size: 50,
            cache_ttl_ms: 30000,
            show_confidence: false,
            min_confidence: 0.3,
            excluded_languages: vec!["plaintext".to_string()],
        }
    }
}

impl InlineCompletionService {
    pub fn new() -> Self {
        Self {
            provider: RwLock::new(None),
            current_ghost: RwLock::new(None),
            pending_request: RwLock::new(None),
            cache: RwLock::new(VecDeque::new()),
            telemetry: RwLock::new(CompletionTelemetry::default()),
            config: RwLock::new(InlineCompletionConfig::default()),
        }
    }

    pub fn set_provider(&self, provider: Arc<dyn InlineCompletionProvider>) {
        *self.provider.write() = Some(provider);
    }

    pub fn show_ghost_text(&self, completion: InlineCompletion, position: Position) {
        *self.current_ghost.write() = Some(GhostText {
            completion,
            position,
            visible: true,
            accepted: false,
        });
        self.telemetry.write().shown_count += 1;
    }

    pub fn hide_ghost_text(&self) {
        if let Some(ghost) = self.current_ghost.write().take() {
            if !ghost.accepted {
                self.telemetry.write().rejected_count += 1;
            }
        }
    }

    pub fn accept_completion(&self) -> Option<String> {
        let mut ghost = self.current_ghost.write();
        if let Some(ref mut g) = *ghost {
            g.accepted = true;
            self.telemetry.write().accepted_count += 1;
            return Some(g.completion.insert_text.clone());
        }
        None
    }

    pub fn accept_word(&self) -> Option<String> {
        let ghost = self.current_ghost.read();
        if let Some(ref g) = *ghost {
            let text = &g.completion.insert_text;
            let word_end = text
                .find(|c: char| c.is_whitespace() || c == '.' || c == '(' || c == '{')
                .unwrap_or(text.len());
            self.telemetry.write().partial_accepted_count += 1;
            return Some(text[..word_end].to_string());
        }
        None
    }

    pub fn accept_line(&self) -> Option<String> {
        let ghost = self.current_ghost.read();
        if let Some(ref g) = *ghost {
            let text = &g.completion.insert_text;
            let line_end = text.find('\n').unwrap_or(text.len());
            self.telemetry.write().partial_accepted_count += 1;
            return Some(text[..line_end].to_string());
        }
        None
    }

    pub fn get_ghost_text(&self) -> Option<GhostText> {
        self.current_ghost.read().clone()
    }

    pub fn get_telemetry(&self) -> CompletionTelemetry {
        self.telemetry.read().clone()
    }

    pub fn cancel_pending(&self) {
        if let Some(id) = self.pending_request.write().take() {
            if let Some(ref provider) = *self.provider.read() {
                provider.cancel(id);
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.read().enabled
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.config.write().enabled = enabled;
    }
}

impl Default for InlineCompletionService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_acceptance_rate() {
        let mut telemetry = CompletionTelemetry::default();
        telemetry.shown_count = 100;
        telemetry.accepted_count = 25;
        assert!((telemetry.acceptance_rate() - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_ghost_text_lifecycle() {
        let service = InlineCompletionService::new();
        let completion = InlineCompletion {
            id: "test".to_string(),
            insert_text: "hello world".to_string(),
            range: None,
            display_text: None,
            confidence: 0.9,
            kind: CompletionKind::SingleLine,
        };
        service.show_ghost_text(completion, Position { line: 0, character: 0 });
        assert!(service.get_ghost_text().is_some());
        service.hide_ghost_text();
        assert!(service.get_ghost_text().is_none());
    }
}
