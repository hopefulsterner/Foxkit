//! # AI Core - Foxkit's Intelligent Brain 🧠
//! 
//! AI is not a plugin in Foxkit - it's a core system layer.
//! 
//! This module provides:
//! - Multi-provider LLM integration (OpenAI, Anthropic, Azure, Google, Ollama)
//! - Monorepo-aware context building
//! - Streaming completions
//! - Tool/function calling
//! - MCP (Model Context Protocol) support
//! - Autonomous agent capabilities

pub mod context;
pub mod providers;
pub mod tools;
pub mod agent;
pub mod completion;
pub mod mcp;

use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;
use futures::Stream;
use std::pin::Pin;

pub use context::AiContext;
pub use providers::{Provider, ProviderConfig, ProviderRegistry, ProviderType};
pub use tools::{Tool, ToolCall, ToolResult, ToolSchema};
pub use agent::{Agent, AgentMode};
pub use completion::{CompletionRequest, CompletionResponse, StreamChunk};
pub use mcp::{McpClient, McpRegistry, McpServerConfig, McpTool};

/// AI Service - the main interface for AI capabilities
pub struct AiService {
    /// Current provider
    provider: Arc<dyn Provider>,
    /// Context builder
    context: Arc<AiContext>,
    /// Available tools
    tools: Vec<Arc<dyn Tool>>,
}

impl AiService {
    /// Create a new AI service with a provider
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self {
            provider,
            context: Arc::new(AiContext::new()),
            tools: Vec::new(),
        }
    }

    /// Register a tool
    pub fn register_tool(&mut self, tool: Arc<dyn Tool>) {
        self.tools.push(tool);
    }

    /// Complete a prompt
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        self.provider.complete(request).await
    }

    /// Stream a completion
    pub async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        self.provider.complete_stream(request).await
    }

    /// Chat with context
    pub async fn chat(&self, messages: Vec<Message>) -> Result<CompletionResponse> {
        let request = CompletionRequest {
            messages,
            tools: self.tools.iter().map(|t| t.schema()).collect(),
            ..Default::default()
        };
        
        self.provider.complete(request).await
    }

    /// Execute tool calls from a response
    pub async fn execute_tools(&self, tool_calls: Vec<ToolCall>) -> Result<Vec<ToolResult>> {
        let mut results = Vec::new();
        
        for call in tool_calls {
            let tool = self.tools
                .iter()
                .find(|t| t.name() == call.name)
                .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", call.name))?;
            
            let result = tool.execute(call.arguments).await?;
            results.push(result);
        }
        
        Ok(results)
    }

    /// Create an autonomous agent
    pub fn create_agent(&self, mode: AgentMode) -> Agent {
        Agent::new(
            Arc::clone(&self.provider),
            self.tools.clone(),
            mode,
        )
    }
}

/// Chat message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    /// Role (system, user, assistant, tool)
    pub role: Role,
    /// Message content
    pub content: String,
    /// Optional name (for tool results)
    pub name: Option<String>,
    /// Tool calls (for assistant messages)
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool call ID (for tool results)
    pub tool_call_id: Option<String>,
}

/// Message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}
