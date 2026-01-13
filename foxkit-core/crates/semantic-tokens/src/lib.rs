//! # Foxkit Semantic Tokens
//!
//! Semantic syntax highlighting from LSP.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Semantic tokens service
pub struct SemanticTokensService {
    /// Cached tokens by file
    cache: RwLock<HashMap<PathBuf, SemanticTokensData>>,
    /// Token legend
    legend: RwLock<SemanticTokensLegend>,
    /// Events
    events: broadcast::Sender<SemanticTokensEvent>,
    /// Configuration
    config: RwLock<SemanticTokensConfig>,
}

impl SemanticTokensService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            cache: RwLock::new(HashMap::new()),
            legend: RwLock::new(SemanticTokensLegend::default()),
            events,
            config: RwLock::new(SemanticTokensConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<SemanticTokensEvent> {
        self.events.subscribe()
    }

    /// Configure semantic tokens
    pub fn configure(&self, config: SemanticTokensConfig) {
        *self.config.write() = config;
    }

    /// Set legend (from LSP capability)
    pub fn set_legend(&self, legend: SemanticTokensLegend) {
        *self.legend.write() = legend;
    }

    /// Get legend
    pub fn legend(&self) -> SemanticTokensLegend {
        self.legend.read().clone()
    }

    /// Set tokens for file
    pub fn set_tokens(&self, file: PathBuf, data: SemanticTokensData) {
        self.cache.write().insert(file.clone(), data);
        let _ = self.events.send(SemanticTokensEvent::Updated { file });
    }

    /// Get tokens for file
    pub fn get_tokens(&self, file: &PathBuf) -> Option<SemanticTokensData> {
        self.cache.read().get(file).cloned()
    }

    /// Get decoded tokens
    pub fn get_decoded_tokens(&self, file: &PathBuf) -> Vec<DecodedToken> {
        let Some(data) = self.get_tokens(file) else {
            return Vec::new();
        };

        let legend = self.legend.read();
        decode_tokens(&data.data, &legend)
    }

    /// Apply delta update
    pub fn apply_delta(&self, file: &PathBuf, delta: SemanticTokensDelta) {
        if let Some(mut data) = self.cache.write().get_mut(file) {
            // Apply edits
            for edit in delta.edits {
                let start = edit.start as usize;
                let delete_count = edit.delete_count as usize;
                
                // Remove old tokens
                if delete_count > 0 && start < data.data.len() {
                    let end = (start + delete_count).min(data.data.len());
                    data.data.drain(start..end);
                }

                // Insert new tokens
                if !edit.data.is_empty() {
                    let insert_pos = start.min(data.data.len());
                    for (i, token) in edit.data.into_iter().enumerate() {
                        data.data.insert(insert_pos + i, token);
                    }
                }
            }

            data.result_id = delta.result_id;
        }

        let _ = self.events.send(SemanticTokensEvent::Updated { file: file.clone() });
    }

    /// Invalidate tokens
    pub fn invalidate(&self, file: &PathBuf) {
        self.cache.write().remove(file);
        let _ = self.events.send(SemanticTokensEvent::Invalidated { file: file.clone() });
    }

    /// Clear cache
    pub fn clear(&self) {
        self.cache.write().clear();
    }
}

impl Default for SemanticTokensService {
    fn default() -> Self {
        Self::new()
    }
}

/// Semantic tokens data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTokensData {
    /// Result ID for delta updates
    pub result_id: Option<String>,
    /// Raw token data (line delta, char delta, length, type, modifiers)
    pub data: Vec<u32>,
}

impl SemanticTokensData {
    pub fn new(data: Vec<u32>) -> Self {
        Self { result_id: None, data }
    }

    pub fn with_result_id(mut self, id: impl Into<String>) -> Self {
        self.result_id = Some(id.into());
        self
    }
}

/// Semantic tokens delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTokensDelta {
    /// New result ID
    pub result_id: Option<String>,
    /// Edits to apply
    pub edits: Vec<SemanticTokensEdit>,
}

/// Semantic tokens edit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTokensEdit {
    /// Start index
    pub start: u32,
    /// Number of elements to delete
    pub delete_count: u32,
    /// Elements to insert
    pub data: Vec<u32>,
}

/// Semantic tokens legend
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SemanticTokensLegend {
    /// Token types
    pub token_types: Vec<String>,
    /// Token modifiers
    pub token_modifiers: Vec<String>,
}

impl SemanticTokensLegend {
    pub fn new(token_types: Vec<String>, token_modifiers: Vec<String>) -> Self {
        Self { token_types, token_modifiers }
    }

    pub fn standard() -> Self {
        Self {
            token_types: vec![
                "namespace".to_string(),
                "type".to_string(),
                "class".to_string(),
                "enum".to_string(),
                "interface".to_string(),
                "struct".to_string(),
                "typeParameter".to_string(),
                "parameter".to_string(),
                "variable".to_string(),
                "property".to_string(),
                "enumMember".to_string(),
                "event".to_string(),
                "function".to_string(),
                "method".to_string(),
                "macro".to_string(),
                "keyword".to_string(),
                "modifier".to_string(),
                "comment".to_string(),
                "string".to_string(),
                "number".to_string(),
                "regexp".to_string(),
                "operator".to_string(),
                "decorator".to_string(),
            ],
            token_modifiers: vec![
                "declaration".to_string(),
                "definition".to_string(),
                "readonly".to_string(),
                "static".to_string(),
                "deprecated".to_string(),
                "abstract".to_string(),
                "async".to_string(),
                "modification".to_string(),
                "documentation".to_string(),
                "defaultLibrary".to_string(),
            ],
        }
    }

    pub fn get_type(&self, index: u32) -> Option<&str> {
        self.token_types.get(index as usize).map(|s| s.as_str())
    }

    pub fn get_modifiers(&self, bits: u32) -> Vec<&str> {
        let mut modifiers = Vec::new();
        for (i, modifier) in self.token_modifiers.iter().enumerate() {
            if bits & (1 << i) != 0 {
                modifiers.push(modifier.as_str());
            }
        }
        modifiers
    }
}

/// Decoded semantic token
#[derive(Debug, Clone)]
pub struct DecodedToken {
    /// Line number
    pub line: u32,
    /// Start character
    pub start: u32,
    /// Token length
    pub length: u32,
    /// Token type
    pub token_type: String,
    /// Token modifiers
    pub modifiers: Vec<String>,
}

impl DecodedToken {
    pub fn end(&self) -> u32 {
        self.start + self.length
    }
}

/// Decode raw tokens
pub fn decode_tokens(data: &[u32], legend: &SemanticTokensLegend) -> Vec<DecodedToken> {
    let mut tokens = Vec::new();
    let mut line: u32 = 0;
    let mut start: u32 = 0;

    // Tokens are encoded as 5-tuples
    for chunk in data.chunks(5) {
        if chunk.len() != 5 {
            break;
        }

        let delta_line = chunk[0];
        let delta_start = chunk[1];
        let length = chunk[2];
        let token_type = chunk[3];
        let modifiers = chunk[4];

        // Update position
        if delta_line > 0 {
            line += delta_line;
            start = delta_start;
        } else {
            start += delta_start;
        }

        let type_name = legend.get_type(token_type)
            .unwrap_or("unknown")
            .to_string();

        let modifier_names: Vec<String> = legend.get_modifiers(modifiers)
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        tokens.push(DecodedToken {
            line,
            start,
            length,
            token_type: type_name,
            modifiers: modifier_names,
        });
    }

    tokens
}

/// Encode tokens
pub fn encode_tokens(tokens: &[DecodedToken], legend: &SemanticTokensLegend) -> Vec<u32> {
    let mut data = Vec::with_capacity(tokens.len() * 5);
    let mut prev_line: u32 = 0;
    let mut prev_start: u32 = 0;

    for token in tokens {
        let delta_line = token.line - prev_line;
        let delta_start = if delta_line > 0 {
            token.start
        } else {
            token.start - prev_start
        };

        // Find type index
        let type_index = legend.token_types.iter()
            .position(|t| t == &token.token_type)
            .unwrap_or(0) as u32;

        // Build modifiers bitmap
        let mut modifiers: u32 = 0;
        for modifier in &token.modifiers {
            if let Some(i) = legend.token_modifiers.iter().position(|m| m == modifier) {
                modifiers |= 1 << i;
            }
        }

        data.push(delta_line);
        data.push(delta_start);
        data.push(token.length);
        data.push(type_index);
        data.push(modifiers);

        prev_line = token.line;
        prev_start = token.start;
    }

    data
}

/// Semantic tokens configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTokensConfig {
    /// Enable semantic highlighting
    pub enabled: bool,
    /// Token type styling
    pub styles: HashMap<String, TokenStyle>,
}

impl Default for SemanticTokensConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            styles: HashMap::new(),
        }
    }
}

/// Token style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStyle {
    /// Foreground color
    pub foreground: Option<String>,
    /// Font style (bold, italic, underline)
    pub font_style: Option<String>,
}

/// Semantic tokens event
#[derive(Debug, Clone)]
pub enum SemanticTokensEvent {
    Updated { file: PathBuf },
    Invalidated { file: PathBuf },
}

/// Semantic token scope mapping
pub fn scope_for_token(token_type: &str, modifiers: &[String]) -> String {
    let mut scope = match token_type {
        "namespace" => "entity.name.namespace",
        "type" => "entity.name.type",
        "class" => "entity.name.class",
        "enum" => "entity.name.enum",
        "interface" => "entity.name.interface",
        "struct" => "entity.name.struct",
        "typeParameter" => "entity.name.type.parameter",
        "parameter" => "variable.parameter",
        "variable" => "variable",
        "property" => "variable.other.property",
        "enumMember" => "variable.other.enummember",
        "event" => "variable.other.event",
        "function" => "entity.name.function",
        "method" => "entity.name.method",
        "macro" => "entity.name.macro",
        "keyword" => "keyword",
        "modifier" => "keyword.modifier",
        "comment" => "comment",
        "string" => "string",
        "number" => "constant.numeric",
        "regexp" => "string.regexp",
        "operator" => "keyword.operator",
        "decorator" => "entity.name.decorator",
        _ => "source",
    }.to_string();

    // Append modifiers
    for modifier in modifiers {
        match modifier.as_str() {
            "declaration" => scope.push_str(".declaration"),
            "definition" => scope.push_str(".definition"),
            "readonly" => scope.push_str(".readonly"),
            "static" => scope.push_str(".static"),
            "deprecated" => scope.push_str(".deprecated"),
            "async" => scope.push_str(".async"),
            _ => {}
        }
    }

    scope
}
