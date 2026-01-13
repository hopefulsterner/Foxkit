//! # Foxkit Inlay Hints
//!
//! Inline type annotations and parameter hints.

pub mod provider;
pub mod types;
pub mod rust;
pub mod typescript;

use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub use provider::InlayHintProvider;
pub use types::{InlayHint, InlayHintKind, InlayHintLabel, Position, Range};

/// Inlay hints service
pub struct InlayHintsService {
    /// Registered providers
    providers: RwLock<Vec<Arc<dyn InlayHintProvider>>>,
    /// Cached hints by file
    cache: RwLock<HashMap<PathBuf, Vec<InlayHint>>>,
    /// Configuration
    config: RwLock<InlayHintsConfig>,
}

impl InlayHintsService {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(Vec::new()),
            cache: RwLock::new(HashMap::new()),
            config: RwLock::new(InlayHintsConfig::default()),
        }
    }

    /// Configure inlay hints
    pub fn configure(&self, config: InlayHintsConfig) {
        *self.config.write() = config;
    }

    /// Register a provider
    pub fn register<P: InlayHintProvider + 'static>(&self, provider: P) {
        self.providers.write().push(Arc::new(provider));
    }

    /// Get inlay hints for a range
    pub async fn get_hints(
        &self,
        file: &PathBuf,
        content: &str,
        range: Option<Range>,
    ) -> Vec<InlayHint> {
        let config = self.config.read().clone();
        
        // Check cache first
        if let Some(cached) = self.cache.read().get(file) {
            if let Some(range) = range {
                return cached.iter()
                    .filter(|h| h.position.line >= range.start.line && h.position.line <= range.end.line)
                    .cloned()
                    .collect();
            }
            return cached.clone();
        }

        let mut hints = Vec::new();

        for provider in self.providers.read().iter() {
            if let Ok(mut provided) = provider.provide_hints(file, content, &config).await {
                hints.append(&mut provided);
            }
        }

        // Sort by position
        hints.sort_by(|a, b| {
            a.position.line.cmp(&b.position.line)
                .then_with(|| a.position.character.cmp(&b.position.character))
        });

        // Cache
        self.cache.write().insert(file.clone(), hints.clone());

        // Filter by range if specified
        if let Some(range) = range {
            hints.into_iter()
                .filter(|h| h.position.line >= range.start.line && h.position.line <= range.end.line)
                .collect()
        } else {
            hints
        }
    }

    /// Resolve an inlay hint (for lazy tooltip loading)
    pub async fn resolve(&self, hint: &InlayHint) -> anyhow::Result<InlayHint> {
        // Add tooltip if not present
        let mut resolved = hint.clone();
        
        if resolved.tooltip.is_none() {
            resolved.tooltip = Some(InlayHintTooltip::String(
                format!("{:?}: {}", hint.kind, hint.label.text())
            ));
        }
        
        Ok(resolved)
    }

    /// Invalidate cache for file
    pub fn invalidate(&self, file: &PathBuf) {
        self.cache.write().remove(file);
    }

    /// Clear all cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}

impl Default for InlayHintsService {
    fn default() -> Self {
        let service = Self::new();
        
        // Register built-in providers
        service.register(rust::RustInlayHintProvider::new());
        service.register(typescript::TypeScriptInlayHintProvider::new());
        
        service
    }
}

/// Inlay hints configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlayHintsConfig {
    /// Enable inlay hints
    pub enabled: bool,
    /// Show type hints
    pub show_type_hints: bool,
    /// Show parameter hints
    pub show_parameter_hints: bool,
    /// Show chaining hints
    pub show_chaining_hints: bool,
    /// Show closure return type hints
    pub show_closure_return_hints: bool,
    /// Maximum length before truncation
    pub max_length: usize,
    /// Hide for obvious types
    pub hide_obvious_types: bool,
    /// Parameter name hints mode
    pub parameter_hints_mode: ParameterHintsMode,
}

impl Default for InlayHintsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_type_hints: true,
            show_parameter_hints: true,
            show_chaining_hints: true,
            show_closure_return_hints: true,
            max_length: 25,
            hide_obvious_types: true,
            parameter_hints_mode: ParameterHintsMode::All,
        }
    }
}

/// Parameter hints mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ParameterHintsMode {
    /// Show for all parameters
    All,
    /// Show only for literals
    Literals,
    /// Never show
    None,
}

/// Inlay hint tooltip
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InlayHintTooltip {
    String(String),
    Markup(MarkupContent),
}

/// Markup content for tooltips
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkupContent {
    pub kind: MarkupKind,
    pub value: String,
}

/// Markup kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MarkupKind {
    PlainText,
    Markdown,
}
