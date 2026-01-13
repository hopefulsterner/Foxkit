//! Completion request/response types

use serde::{Deserialize, Serialize};
use crate::{Message, ToolCall, ToolSchema};

/// Completion request
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Temperature (0.0 - 2.0)
    pub temperature: Option<f32>,
    /// Available tools
    pub tools: Vec<ToolSchema>,
    /// Stop sequences
    pub stop: Vec<String>,
    /// System prompt (will be prepended)
    pub system: Option<String>,
}

/// Completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Generated content
    pub content: String,
    /// Tool calls (if any)
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Finish reason
    pub finish_reason: Option<String>,
    /// Token usage
    pub usage: Option<Usage>,
}

/// Token usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Stream chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Content delta
    pub content: String,
    /// Whether this is the final chunk
    pub done: bool,
}
