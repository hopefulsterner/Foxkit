//! AI Inline Completion Providers
//!
//! Implementations for different AI providers for ghost text completions.

use async_trait::async_trait;
use std::sync::Arc;
use parking_lot::Mutex;
use reqwest::Client;

use crate::{
    InlineCompletionProvider, InlineCompletionRequest, InlineCompletionResult,
    InlineCompletion, CompletionError, CompletionKind, CompletionRequestId,
};

/// FIM (Fill in the Middle) prompt format for code completion
#[derive(Debug, Clone, Copy)]
pub enum FimFormat {
    /// OpenAI style: prefix + suffix approach
    OpenAI,
    /// Anthropic style: conversation based
    Anthropic,
    /// CodeLlama/DeepSeek style: <PRE>, <SUF>, <MID> tokens
    CodeLlama,
    /// StarCoder style: <fim_prefix>, <fim_suffix>, <fim_middle>
    StarCoder,
    /// Ollama/generic style
    Generic,
}

/// Provider using OpenAI API (or compatible)
pub struct OpenAICompletionProvider {
    client: Client,
    api_key: String,
    model: String,
    endpoint: String,
    pending: Mutex<Option<CompletionRequestId>>,
}

impl OpenAICompletionProvider {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model: model.into(),
            endpoint: "https://api.openai.com/v1".into(),
            pending: Mutex::new(None),
        }
    }

    pub fn from_env() -> Result<Self, CompletionError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| CompletionError::InternalError("OPENAI_API_KEY not set".into()))?;
        Ok(Self::new(api_key, "gpt-4o"))
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    fn build_prompt(&self, context: &crate::InlineCompletionContext) -> String {
        format!(
            "Complete the following {} code. Only output the completion, nothing else.\n\n{}",
            context.language_id,
            context.prefix
        )
    }
}

#[async_trait]
impl InlineCompletionProvider for OpenAICompletionProvider {
    async fn provide_completions(
        &self,
        request: InlineCompletionRequest,
    ) -> Result<InlineCompletionResult, CompletionError> {
        *self.pending.lock() = Some(request.id);
        
        let start = std::time::Instant::now();
        let prompt = self.build_prompt(&request.context);

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a code completion assistant. Complete the code naturally without explanations. Only output the code that should come next."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": request.options.max_tokens,
            "temperature": request.options.temperature,
            "stop": request.options.stop_sequences,
            "n": request.options.max_suggestions.min(3),
        });

        let response = self.client
            .post(format!("{}/chat/completions", self.endpoint))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| CompletionError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            if response.status().as_u16() == 429 {
                return Err(CompletionError::RateLimited);
            }
            return Err(CompletionError::NetworkError(
                format!("HTTP {}", response.status())
            ));
        }

        let json: serde_json::Value = response.json()
            .await
            .map_err(|e| CompletionError::NetworkError(e.to_string()))?;

        let mut completions = Vec::new();
        if let Some(choices) = json["choices"].as_array() {
            for (i, choice) in choices.iter().enumerate() {
                if let Some(content) = choice["message"]["content"].as_str() {
                    let content = content.trim();
                    if !content.is_empty() {
                        completions.push(InlineCompletion {
                            id: format!("openai-{}-{}", request.id.0, i),
                            insert_text: content.to_string(),
                            range: None,
                            display_text: None,
                            confidence: 0.8 - (i as f32 * 0.1), // Decreasing confidence
                            kind: if content.contains('\n') {
                                CompletionKind::MultiLine
                            } else {
                                CompletionKind::SingleLine
                            },
                        });
                    }
                }
            }
        }

        Ok(InlineCompletionResult {
            request_id: request.id,
            completions,
            is_cached: false,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn cancel(&self, request_id: CompletionRequestId) {
        let mut pending = self.pending.lock();
        if *pending == Some(request_id) {
            *pending = None;
        }
    }
}

/// Provider using Anthropic API
pub struct AnthropicCompletionProvider {
    client: Client,
    api_key: String,
    model: String,
    pending: Mutex<Option<CompletionRequestId>>,
}

impl AnthropicCompletionProvider {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model: model.into(),
            pending: Mutex::new(None),
        }
    }

    pub fn from_env() -> Result<Self, CompletionError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| CompletionError::InternalError("ANTHROPIC_API_KEY not set".into()))?;
        Ok(Self::new(api_key, "claude-sonnet-4-20250514"))
    }

    fn build_prompt(&self, context: &crate::InlineCompletionContext) -> String {
        format!(
            "Complete the following {} code. Output ONLY the code that should come immediately after the cursor. No explanations.\n\nCode before cursor:\n```{}\n{}\n```\n\nCode after cursor:\n```{}\n{}\n```\n\nYour completion (code only):",
            context.language_id,
            context.language_id,
            context.prefix,
            context.language_id,
            context.suffix
        )
    }
}

#[async_trait]
impl InlineCompletionProvider for AnthropicCompletionProvider {
    async fn provide_completions(
        &self,
        request: InlineCompletionRequest,
    ) -> Result<InlineCompletionResult, CompletionError> {
        *self.pending.lock() = Some(request.id);
        
        let start = std::time::Instant::now();
        let prompt = self.build_prompt(&request.context);

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.options.max_tokens,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": request.options.temperature,
        });

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| CompletionError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            if response.status().as_u16() == 429 {
                return Err(CompletionError::RateLimited);
            }
            return Err(CompletionError::NetworkError(
                format!("HTTP {}", response.status())
            ));
        }

        let json: serde_json::Value = response.json()
            .await
            .map_err(|e| CompletionError::NetworkError(e.to_string()))?;

        let mut completions = Vec::new();
        if let Some(content) = json["content"][0]["text"].as_str() {
            let content = content.trim();
            // Remove code fence if AI included it
            let content = content
                .strip_prefix("```")
                .and_then(|s| s.split_once('\n'))
                .map(|(_, s)| s.strip_suffix("```").unwrap_or(s))
                .unwrap_or(content);
            
            if !content.is_empty() {
                completions.push(InlineCompletion {
                    id: format!("anthropic-{}", request.id.0),
                    insert_text: content.to_string(),
                    range: None,
                    display_text: None,
                    confidence: 0.9,
                    kind: if content.contains('\n') {
                        CompletionKind::MultiLine
                    } else {
                        CompletionKind::SingleLine
                    },
                });
            }
        }

        Ok(InlineCompletionResult {
            request_id: request.id,
            completions,
            is_cached: false,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn cancel(&self, request_id: CompletionRequestId) {
        let mut pending = self.pending.lock();
        if *pending == Some(request_id) {
            *pending = None;
        }
    }
}

/// Provider using local Ollama models
pub struct OllamaCompletionProvider {
    client: Client,
    endpoint: String,
    model: String,
    fim_format: FimFormat,
    pending: Mutex<Option<CompletionRequestId>>,
}

impl OllamaCompletionProvider {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            endpoint: "http://localhost:11434".into(),
            model: model.into(),
            fim_format: FimFormat::Generic,
            pending: Mutex::new(None),
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    pub fn with_fim_format(mut self, format: FimFormat) -> Self {
        self.fim_format = format;
        self
    }

    fn build_fim_prompt(&self, context: &crate::InlineCompletionContext) -> String {
        match self.fim_format {
            FimFormat::CodeLlama => {
                format!(
                    "<PRE> {} <SUF>{} <MID>",
                    context.prefix,
                    context.suffix
                )
            }
            FimFormat::StarCoder => {
                format!(
                    "<fim_prefix>{}<fim_suffix>{}<fim_middle>",
                    context.prefix,
                    context.suffix
                )
            }
            FimFormat::Generic | FimFormat::OpenAI | FimFormat::Anthropic => {
                format!(
                    "Complete the following {} code. Output only the completion.\n\n{}",
                    context.language_id,
                    context.prefix
                )
            }
        }
    }
}

#[async_trait]
impl InlineCompletionProvider for OllamaCompletionProvider {
    async fn provide_completions(
        &self,
        request: InlineCompletionRequest,
    ) -> Result<InlineCompletionResult, CompletionError> {
        *self.pending.lock() = Some(request.id);
        
        let start = std::time::Instant::now();
        let prompt = self.build_fim_prompt(&request.context);

        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": request.options.temperature,
                "num_predict": request.options.max_tokens,
                "stop": request.options.stop_sequences,
            }
        });

        let response = self.client
            .post(format!("{}/api/generate", self.endpoint))
            .json(&body)
            .send()
            .await
            .map_err(|e| CompletionError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CompletionError::NetworkError(
                format!("HTTP {}", response.status())
            ));
        }

        let json: serde_json::Value = response.json()
            .await
            .map_err(|e| CompletionError::NetworkError(e.to_string()))?;

        let mut completions = Vec::new();
        if let Some(content) = json["response"].as_str() {
            let content = content.trim();
            if !content.is_empty() {
                completions.push(InlineCompletion {
                    id: format!("ollama-{}", request.id.0),
                    insert_text: content.to_string(),
                    range: None,
                    display_text: None,
                    confidence: 0.7,
                    kind: if content.contains('\n') {
                        CompletionKind::MultiLine
                    } else {
                        CompletionKind::SingleLine
                    },
                });
            }
        }

        Ok(InlineCompletionResult {
            request_id: request.id,
            completions,
            is_cached: false,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn cancel(&self, request_id: CompletionRequestId) {
        let mut pending = self.pending.lock();
        if *pending == Some(request_id) {
            *pending = None;
        }
    }
}

/// Multi-provider completion that tries providers in order
pub struct MultiProviderCompletion {
    providers: Vec<Arc<dyn InlineCompletionProvider>>,
}

impl MultiProviderCompletion {
    pub fn new(providers: Vec<Arc<dyn InlineCompletionProvider>>) -> Self {
        Self { providers }
    }

    /// Create from environment, trying available providers
    pub fn from_env() -> Self {
        let mut providers: Vec<Arc<dyn InlineCompletionProvider>> = Vec::new();
        
        // Try each provider in preference order
        if let Ok(provider) = AnthropicCompletionProvider::from_env() {
            providers.push(Arc::new(provider));
        }
        
        if let Ok(provider) = OpenAICompletionProvider::from_env() {
            providers.push(Arc::new(provider));
        }
        
        // Always try local Ollama
        providers.push(Arc::new(OllamaCompletionProvider::new("codellama")));
        
        Self { providers }
    }
}

#[async_trait]
impl InlineCompletionProvider for MultiProviderCompletion {
    async fn provide_completions(
        &self,
        request: InlineCompletionRequest,
    ) -> Result<InlineCompletionResult, CompletionError> {
        let mut last_error = CompletionError::InternalError("No providers available".into());
        
        for provider in &self.providers {
            match provider.provide_completions(request.clone()).await {
                Ok(result) if !result.completions.is_empty() => {
                    return Ok(result);
                }
                Ok(_) => continue, // Empty result, try next
                Err(e) => {
                    last_error = e;
                    continue;
                }
            }
        }
        
        Err(last_error)
    }

    fn cancel(&self, request_id: CompletionRequestId) {
        for provider in &self.providers {
            provider.cancel(request_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fim_prompt_codellama() {
        let provider = OllamaCompletionProvider::new("codellama")
            .with_fim_format(FimFormat::CodeLlama);
        
        let context = crate::InlineCompletionContext {
            file_path: "test.rs".into(),
            language_id: "rust".into(),
            prefix: "fn main() {\n    ".into(),
            suffix: "\n}".into(),
            cursor_position: crate::Position { line: 1, character: 4 },
            trigger_kind: crate::TriggerKind::Automatic,
            related_files: vec![],
        };
        
        let prompt = provider.build_fim_prompt(&context);
        assert!(prompt.contains("<PRE>"));
        assert!(prompt.contains("<SUF>"));
        assert!(prompt.contains("<MID>"));
    }

    #[test]
    fn test_multi_provider_from_env() {
        let multi = MultiProviderCompletion::from_env();
        assert!(!multi.providers.is_empty()); // At least Ollama
    }
}
