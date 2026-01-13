//! LLM Provider implementations
//! 
//! Supports: OpenAI, Anthropic, Azure OpenAI, Ollama, Custom

use std::pin::Pin;
use async_trait::async_trait;
use anyhow::Result;
use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::{CompletionRequest, CompletionResponse, StreamChunk};

/// LLM Provider trait
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;
    
    /// Complete a request
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    
    /// Stream a completion
    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>>;
    
    /// List available models
    async fn list_models(&self) -> Result<Vec<String>>;
    
    /// Check if provider is available
    async fn health_check(&self) -> Result<bool>;
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    AzureOpenAI,
    Ollama,
    Custom,
}

// ============================================================================
// Anthropic Provider (Default for Foxkit!)
// ============================================================================

pub mod anthropic {
    use super::*;
    use reqwest::Client;
    
    pub struct AnthropicProvider {
        client: Client,
        api_key: String,
        model: String,
    }

    impl AnthropicProvider {
        pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
            Self {
                client: Client::new(),
                api_key: api_key.into(),
                model: model.into(),
            }
        }

        pub fn from_env() -> Result<Self> {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;
            Ok(Self::new(api_key, "claude-sonnet-4-20250514"))
        }
    }

    #[async_trait]
    impl Provider for AnthropicProvider {
        fn name(&self) -> &str {
            "Anthropic"
        }

        async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
            let body = serde_json::json!({
                "model": self.model,
                "max_tokens": request.max_tokens.unwrap_or(4096),
                "messages": request.messages,
                "temperature": request.temperature.unwrap_or(0.7),
            });

            let response = self.client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await?;

            let json: serde_json::Value = response.json().await?;
            
            let content = json["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string();

            Ok(CompletionResponse {
                content,
                tool_calls: None,
                finish_reason: json["stop_reason"].as_str().map(String::from),
                usage: None,
            })
        }

        async fn complete_stream(
            &self,
            request: CompletionRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
            use async_stream::stream;
            
            let body = serde_json::json!({
                "model": self.model,
                "max_tokens": request.max_tokens.unwrap_or(4096),
                "messages": request.messages,
                "temperature": request.temperature.unwrap_or(0.7),
                "stream": true,
            });

            let response = self.client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await?;

            let stream = stream! {
                let mut stream = response.bytes_stream();
                use futures::StreamExt;
                
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            for line in text.lines() {
                                if line.starts_with("data: ") {
                                    let data = &line[6..];
                                    if data != "[DONE]" {
                                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                            if let Some(delta) = json["delta"]["text"].as_str() {
                                                yield Ok(StreamChunk {
                                                    content: delta.to_string(),
                                                    done: false,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => yield Err(anyhow::anyhow!("Stream error: {}", e)),
                    }
                }
                
                yield Ok(StreamChunk {
                    content: String::new(),
                    done: true,
                });
            };

            Ok(Box::pin(stream))
        }

        async fn list_models(&self) -> Result<Vec<String>> {
            Ok(vec![
                "claude-sonnet-4-20250514".into(),
                "claude-3-5-sonnet-20241022".into(),
                "claude-3-opus-20240229".into(),
                "claude-3-haiku-20240307".into(),
            ])
        }

        async fn health_check(&self) -> Result<bool> {
            Ok(!self.api_key.is_empty())
        }
    }
}

// ============================================================================
// OpenAI Provider
// ============================================================================

pub mod openai {
    use super::*;
    use reqwest::Client;

    pub struct OpenAIProvider {
        client: Client,
        api_key: String,
        model: String,
        endpoint: String,
    }

    impl OpenAIProvider {
        pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
            Self {
                client: Client::new(),
                api_key: api_key.into(),
                model: model.into(),
                endpoint: "https://api.openai.com/v1".into(),
            }
        }

        pub fn from_env() -> Result<Self> {
            let api_key = std::env::var("OPENAI_API_KEY")
                .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
            Ok(Self::new(api_key, "gpt-4o"))
        }

        /// Use custom endpoint (for Azure OpenAI, etc.)
        pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
            self.endpoint = endpoint.into();
            self
        }
    }

    #[async_trait]
    impl Provider for OpenAIProvider {
        fn name(&self) -> &str {
            "OpenAI"
        }

        async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
            let body = serde_json::json!({
                "model": self.model,
                "messages": request.messages,
                "max_tokens": request.max_tokens.unwrap_or(4096),
                "temperature": request.temperature.unwrap_or(0.7),
            });

            let response = self.client
                .post(format!("{}/chat/completions", self.endpoint))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .json(&body)
                .send()
                .await?;

            let json: serde_json::Value = response.json().await?;
            
            let content = json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();

            Ok(CompletionResponse {
                content,
                tool_calls: None,
                finish_reason: json["choices"][0]["finish_reason"].as_str().map(String::from),
                usage: None,
            })
        }

        async fn complete_stream(
            &self,
            _request: CompletionRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
            // Similar streaming implementation
            todo!("OpenAI streaming")
        }

        async fn list_models(&self) -> Result<Vec<String>> {
            Ok(vec![
                "gpt-4o".into(),
                "gpt-4o-mini".into(),
                "gpt-4-turbo".into(),
                "gpt-4".into(),
                "gpt-3.5-turbo".into(),
            ])
        }

        async fn health_check(&self) -> Result<bool> {
            Ok(!self.api_key.is_empty())
        }
    }
}

// ============================================================================
// Ollama Provider (Local LLMs)
// ============================================================================

pub mod ollama {
    use super::*;
    use reqwest::Client;

    pub struct OllamaProvider {
        client: Client,
        endpoint: String,
        model: String,
    }

    impl OllamaProvider {
        pub fn new(model: impl Into<String>) -> Self {
            Self {
                client: Client::new(),
                endpoint: "http://localhost:11434".into(),
                model: model.into(),
            }
        }

        pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
            self.endpoint = endpoint.into();
            self
        }
    }

    #[async_trait]
    impl Provider for OllamaProvider {
        fn name(&self) -> &str {
            "Ollama"
        }

        async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
            let messages: Vec<serde_json::Value> = request.messages
                .iter()
                .map(|m| serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                }))
                .collect();

            let body = serde_json::json!({
                "model": self.model,
                "messages": messages,
                "stream": false,
            });

            let response = self.client
                .post(format!("{}/api/chat", self.endpoint))
                .json(&body)
                .send()
                .await?;

            let json: serde_json::Value = response.json().await?;
            
            let content = json["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();

            Ok(CompletionResponse {
                content,
                tool_calls: None,
                finish_reason: Some("stop".into()),
                usage: None,
            })
        }

        async fn complete_stream(
            &self,
            _request: CompletionRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
            todo!("Ollama streaming")
        }

        async fn list_models(&self) -> Result<Vec<String>> {
            let response = self.client
                .get(format!("{}/api/tags", self.endpoint))
                .send()
                .await?;

            let json: serde_json::Value = response.json().await?;
            
            let models = json["models"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m["name"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Ok(models)
        }

        async fn health_check(&self) -> Result<bool> {
            let response = self.client
                .get(format!("{}/api/tags", self.endpoint))
                .send()
                .await;
            
            Ok(response.is_ok())
        }
    }
}
