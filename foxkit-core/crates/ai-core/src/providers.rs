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
            request: CompletionRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
            use async_stream::stream;
            
            let body = serde_json::json!({
                "model": self.model,
                "messages": request.messages,
                "max_tokens": request.max_tokens.unwrap_or(4096),
                "temperature": request.temperature.unwrap_or(0.7),
                "stream": true,
            });

            let response = self.client
                .post(format!("{}/chat/completions", self.endpoint))
                .header("Authorization", format!("Bearer {}", self.api_key))
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
                                            if let Some(delta) = json["choices"][0]["delta"]["content"].as_str() {
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
            request: CompletionRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
            use async_stream::stream;
            
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
                "stream": true,
            });

            let response = self.client
                .post(format!("{}/api/chat", self.endpoint))
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
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                                    if let Some(content) = json["message"]["content"].as_str() {
                                        if !content.is_empty() {
                                            yield Ok(StreamChunk {
                                                content: content.to_string(),
                                                done: json["done"].as_bool().unwrap_or(false),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => yield Err(anyhow::anyhow!("Stream error: {}", e)),
                    }
                }
            };

            Ok(Box::pin(stream))
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

// ============================================================================
// Google Gemini Provider
// ============================================================================

pub mod google {
    use super::*;
    use reqwest::Client;

    pub struct GeminiProvider {
        client: Client,
        api_key: String,
        model: String,
    }

    impl GeminiProvider {
        pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
            Self {
                client: Client::new(),
                api_key: api_key.into(),
                model: model.into(),
            }
        }

        pub fn from_env() -> Result<Self> {
            let api_key = std::env::var("GOOGLE_API_KEY")
                .or_else(|_| std::env::var("GEMINI_API_KEY"))
                .map_err(|_| anyhow::anyhow!("GOOGLE_API_KEY or GEMINI_API_KEY not set"))?;
            Ok(Self::new(api_key, "gemini-2.0-flash-exp"))
        }
    }

    #[async_trait]
    impl Provider for GeminiProvider {
        fn name(&self) -> &str {
            "Google Gemini"
        }

        async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
            // Convert messages to Gemini format
            let contents: Vec<serde_json::Value> = request.messages
                .iter()
                .filter(|m| m.role != crate::Role::System) // Gemini handles system differently
                .map(|m| {
                    let role = match m.role {
                        crate::Role::User => "user",
                        crate::Role::Assistant => "model",
                        _ => "user",
                    };
                    serde_json::json!({
                        "role": role,
                        "parts": [{"text": m.content}]
                    })
                })
                .collect();

            let system_instruction = request.messages
                .iter()
                .find(|m| m.role == crate::Role::System)
                .map(|m| serde_json::json!({"parts": [{"text": m.content}]}));

            let mut body = serde_json::json!({
                "contents": contents,
                "generationConfig": {
                    "maxOutputTokens": request.max_tokens.unwrap_or(4096),
                    "temperature": request.temperature.unwrap_or(0.7),
                }
            });

            if let Some(sys) = system_instruction {
                body["systemInstruction"] = sys;
            }

            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                self.model, self.api_key
            );

            let response = self.client
                .post(&url)
                .json(&body)
                .send()
                .await?;

            let json: serde_json::Value = response.json().await?;
            
            let content = json["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string();

            let finish_reason = json["candidates"][0]["finishReason"]
                .as_str()
                .map(String::from);

            Ok(CompletionResponse {
                content,
                tool_calls: None,
                finish_reason,
                usage: None,
            })
        }

        async fn complete_stream(
            &self,
            request: CompletionRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
            use async_stream::stream;
            
            let contents: Vec<serde_json::Value> = request.messages
                .iter()
                .filter(|m| m.role != crate::Role::System)
                .map(|m| {
                    let role = match m.role {
                        crate::Role::User => "user",
                        crate::Role::Assistant => "model",
                        _ => "user",
                    };
                    serde_json::json!({
                        "role": role,
                        "parts": [{"text": m.content}]
                    })
                })
                .collect();

            let body = serde_json::json!({
                "contents": contents,
                "generationConfig": {
                    "maxOutputTokens": request.max_tokens.unwrap_or(4096),
                    "temperature": request.temperature.unwrap_or(0.7),
                }
            });

            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
                self.model, self.api_key
            );

            let response = self.client
                .post(&url)
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
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                        if let Some(text) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                                            yield Ok(StreamChunk {
                                                content: text.to_string(),
                                                done: false,
                                            });
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
                "gemini-2.0-flash-exp".into(),
                "gemini-1.5-pro".into(),
                "gemini-1.5-flash".into(),
                "gemini-1.5-flash-8b".into(),
            ])
        }

        async fn health_check(&self) -> Result<bool> {
            Ok(!self.api_key.is_empty())
        }
    }
}

// ============================================================================
// Azure OpenAI Provider
// ============================================================================

pub mod azure {
    use super::*;
    use reqwest::Client;

    pub struct AzureOpenAIProvider {
        client: Client,
        api_key: String,
        endpoint: String,
        deployment: String,
        api_version: String,
    }

    impl AzureOpenAIProvider {
        pub fn new(
            endpoint: impl Into<String>,
            api_key: impl Into<String>,
            deployment: impl Into<String>,
        ) -> Self {
            Self {
                client: Client::new(),
                api_key: api_key.into(),
                endpoint: endpoint.into(),
                deployment: deployment.into(),
                api_version: "2024-06-01".into(),
            }
        }

        pub fn from_env() -> Result<Self> {
            let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT")
                .map_err(|_| anyhow::anyhow!("AZURE_OPENAI_ENDPOINT not set"))?;
            let api_key = std::env::var("AZURE_OPENAI_API_KEY")
                .map_err(|_| anyhow::anyhow!("AZURE_OPENAI_API_KEY not set"))?;
            let deployment = std::env::var("AZURE_OPENAI_DEPLOYMENT")
                .unwrap_or_else(|_| "gpt-4".into());
            Ok(Self::new(endpoint, api_key, deployment))
        }

        pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
            self.api_version = version.into();
            self
        }
    }

    #[async_trait]
    impl Provider for AzureOpenAIProvider {
        fn name(&self) -> &str {
            "Azure OpenAI"
        }

        async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
            let body = serde_json::json!({
                "messages": request.messages,
                "max_tokens": request.max_tokens.unwrap_or(4096),
                "temperature": request.temperature.unwrap_or(0.7),
            });

            let url = format!(
                "{}/openai/deployments/{}/chat/completions?api-version={}",
                self.endpoint, self.deployment, self.api_version
            );

            let response = self.client
                .post(&url)
                .header("api-key", &self.api_key)
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
            request: CompletionRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
            use async_stream::stream;
            
            let body = serde_json::json!({
                "messages": request.messages,
                "max_tokens": request.max_tokens.unwrap_or(4096),
                "temperature": request.temperature.unwrap_or(0.7),
                "stream": true,
            });

            let url = format!(
                "{}/openai/deployments/{}/chat/completions?api-version={}",
                self.endpoint, self.deployment, self.api_version
            );

            let response = self.client
                .post(&url)
                .header("api-key", &self.api_key)
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
                                            if let Some(delta) = json["choices"][0]["delta"]["content"].as_str() {
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
            Ok(vec![self.deployment.clone()])
        }

        async fn health_check(&self) -> Result<bool> {
            Ok(!self.api_key.is_empty() && !self.endpoint.is_empty())
        }
    }
}

// ============================================================================
// Provider Registry
// ============================================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Registry of all available LLM providers
pub struct ProviderRegistry {
    providers: RwLock<HashMap<String, Arc<dyn Provider>>>,
    default_provider: RwLock<Option<String>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            default_provider: RwLock::new(None),
        }
    }

    /// Register a provider
    pub fn register(&self, name: impl Into<String>, provider: Arc<dyn Provider>) {
        let name = name.into();
        let is_first = self.providers.read().is_empty();
        self.providers.write().insert(name.clone(), provider);
        
        if is_first {
            *self.default_provider.write() = Some(name);
        }
    }

    /// Get a provider by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        self.providers.read().get(name).cloned()
    }

    /// Get the default provider
    pub fn default(&self) -> Option<Arc<dyn Provider>> {
        let default_name = self.default_provider.read().clone()?;
        self.get(&default_name)
    }

    /// Set default provider
    pub fn set_default(&self, name: impl Into<String>) {
        *self.default_provider.write() = Some(name.into());
    }

    /// List all registered providers
    pub fn list(&self) -> Vec<String> {
        self.providers.read().keys().cloned().collect()
    }

    /// Create registry with all providers from environment
    pub fn from_env() -> Self {
        let registry = Self::new();
        
        // Try to register each provider from environment
        if let Ok(provider) = anthropic::AnthropicProvider::from_env() {
            registry.register("anthropic", Arc::new(provider));
        }
        
        if let Ok(provider) = openai::OpenAIProvider::from_env() {
            registry.register("openai", Arc::new(provider));
        }
        
        if let Ok(provider) = google::GeminiProvider::from_env() {
            registry.register("google", Arc::new(provider));
        }
        
        if let Ok(provider) = azure::AzureOpenAIProvider::from_env() {
            registry.register("azure", Arc::new(provider));
        }
        
        // Always try Ollama (local)
        let ollama = ollama::OllamaProvider::new("llama3.2");
        registry.register("ollama", Arc::new(ollama));
        
        registry
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
