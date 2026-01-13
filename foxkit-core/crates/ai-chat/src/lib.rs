//! AI Chat Panel Integration for Foxkit
//!
//! Provides conversational AI interface with context-aware assistance,
//! code generation, and intelligent suggestions.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Unique identifier for a chat session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a message
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

/// Role in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

/// Message content type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum MessageContent {
    Text(String),
    Code { language: String, code: String },
    Image { url: String, alt: Option<String> },
    File { path: String, name: String },
    Markdown(String),
    Mixed(Vec<MessageContent>),
}

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: MessageId,
    pub session_id: SessionId,
    pub role: Role,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
    pub metadata: MessageMetadata,
}

/// Metadata attached to a message
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub model: Option<String>,
    pub tokens_used: Option<u32>,
    pub latency_ms: Option<u64>,
    pub context_files: Vec<String>,
    pub references: Vec<CodeReference>,
}

/// Reference to code in the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeReference {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub symbol: Option<String>,
}

/// Context provided to the AI
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatContext {
    pub active_file: Option<String>,
    pub selection: Option<String>,
    pub visible_files: Vec<String>,
    pub workspace_info: Option<WorkspaceInfo>,
    pub recent_diagnostics: Vec<DiagnosticInfo>,
    pub custom: HashMap<String, serde_json::Value>,
}

/// Workspace information for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub name: String,
    pub root_path: String,
    pub language_stats: HashMap<String, u32>,
    pub framework_hints: Vec<String>,
}

/// Diagnostic information for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    pub file: String,
    pub line: u32,
    pub severity: String,
    pub message: String,
}

/// Chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub session_id: SessionId,
    pub messages: Vec<ChatMessage>,
    pub context: ChatContext,
    pub options: CompletionOptions,
}

/// Options for completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOptions {
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop_sequences: Vec<String>,
    pub stream: bool,
}

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            model: "gpt-4".to_string(),
            max_tokens: Some(4096),
            temperature: Some(0.7),
            top_p: Some(0.95),
            stop_sequences: Vec::new(),
            stream: true,
        }
    }
}

/// Streaming chunk from completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub delta: String,
    pub finish_reason: Option<FinishReason>,
}

/// Reason for completion finish
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
}

/// Chat session
#[derive(Debug)]
pub struct ChatSession {
    pub id: SessionId,
    pub title: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub context: ChatContext,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ChatSession {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            title: None,
            messages: Vec::new(),
            context: ChatContext::default(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_message(&mut self, role: Role, content: MessageContent) -> MessageId {
        let message = ChatMessage {
            id: MessageId::new(),
            session_id: self.id,
            role,
            content,
            timestamp: Utc::now(),
            metadata: MessageMetadata::default(),
        };
        let id = message.id;
        self.messages.push(message);
        self.updated_at = Utc::now();
        id
    }
}

impl Default for ChatSession {
    fn default() -> Self {
        Self::new()
    }
}

/// AI chat provider trait
#[async_trait]
pub trait AiChatProvider: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<ChatMessage, ChatError>;
    async fn stream_complete(
        &self,
        request: CompletionRequest,
    ) -> Result<Box<dyn StreamingResponse>, ChatError>;
    fn supported_models(&self) -> Vec<ModelInfo>;
}

/// Streaming response trait
#[async_trait]
pub trait StreamingResponse: Send {
    async fn next_chunk(&mut self) -> Option<Result<StreamChunk, ChatError>>;
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_window: u32,
    pub capabilities: Vec<String>,
}

/// Chat error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatError {
    NetworkError(String),
    AuthenticationError(String),
    RateLimited { retry_after: Option<u64> },
    ModelNotFound(String),
    ContextTooLong { max: u32, actual: u32 },
    ContentFiltered(String),
    InternalError(String),
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkError(e) => write!(f, "Network error: {}", e),
            Self::AuthenticationError(e) => write!(f, "Authentication error: {}", e),
            Self::RateLimited { retry_after } => {
                write!(f, "Rate limited")?;
                if let Some(secs) = retry_after {
                    write!(f, ", retry after {} seconds", secs)?;
                }
                Ok(())
            }
            Self::ModelNotFound(m) => write!(f, "Model not found: {}", m),
            Self::ContextTooLong { max, actual } => {
                write!(f, "Context too long: {} > {}", actual, max)
            }
            Self::ContentFiltered(r) => write!(f, "Content filtered: {}", r),
            Self::InternalError(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl std::error::Error for ChatError {}

/// Chat panel service
pub struct AiChatService {
    sessions: RwLock<HashMap<SessionId, ChatSession>>,
    active_session: RwLock<Option<SessionId>>,
    providers: RwLock<HashMap<String, Arc<dyn AiChatProvider>>>,
    config: RwLock<ChatConfig>,
}

/// Chat configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub default_model: String,
    pub system_prompt: Option<String>,
    pub include_workspace_context: bool,
    pub include_active_file: bool,
    pub max_context_files: usize,
    pub history_limit: usize,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            default_model: "gpt-4".to_string(),
            system_prompt: Some("You are a helpful coding assistant.".to_string()),
            include_workspace_context: true,
            include_active_file: true,
            max_context_files: 10,
            history_limit: 100,
        }
    }
}

impl AiChatService {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            active_session: RwLock::new(None),
            providers: RwLock::new(HashMap::new()),
            config: RwLock::new(ChatConfig::default()),
        }
    }

    pub fn create_session(&self) -> SessionId {
        let session = ChatSession::new();
        let id = session.id;
        self.sessions.write().insert(id, session);
        *self.active_session.write() = Some(id);
        id
    }

    pub fn get_session(&self, id: SessionId) -> Option<ChatSession> {
        self.sessions.read().get(&id).map(|s| ChatSession {
            id: s.id,
            title: s.title.clone(),
            messages: s.messages.clone(),
            context: s.context.clone(),
            created_at: s.created_at,
            updated_at: s.updated_at,
        })
    }

    pub fn list_sessions(&self) -> Vec<SessionId> {
        self.sessions.read().keys().copied().collect()
    }

    pub fn delete_session(&self, id: SessionId) {
        self.sessions.write().remove(&id);
        let mut active = self.active_session.write();
        if *active == Some(id) {
            *active = None;
        }
    }

    pub fn register_provider(&self, name: String, provider: Arc<dyn AiChatProvider>) {
        self.providers.write().insert(name, provider);
    }

    pub fn active_session_id(&self) -> Option<SessionId> {
        *self.active_session.read()
    }

    pub fn set_active_session(&self, id: SessionId) {
        if self.sessions.read().contains_key(&id) {
            *self.active_session.write() = Some(id);
        }
    }
}

impl Default for AiChatService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let service = AiChatService::new();
        let id = service.create_session();
        assert!(service.get_session(id).is_some());
    }

    #[test]
    fn test_message_content() {
        let content = MessageContent::Code {
            language: "rust".to_string(),
            code: "fn main() {}".to_string(),
        };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("rust"));
    }
}
