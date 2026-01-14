//! Completion Item Resolution
//!
//! Lazy loading of completion details.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use async_trait::async_trait;

/// Completion item with resolvable details
#[derive(Debug, Clone)]
pub struct ResolvableCompletion {
    /// Unique ID for resolution
    pub id: String,
    /// Label shown in completion list
    pub label: String,
    /// Kind of completion
    pub kind: CompletionKind,
    /// Short detail (type info)
    pub detail: Option<String>,
    /// Sort text for ordering
    pub sort_text: Option<String>,
    /// Filter text for matching
    pub filter_text: Option<String>,
    /// Insert text or snippet
    pub insert_text: Option<String>,
    /// Is this a snippet?
    pub is_snippet: bool,
    /// Preselect this item
    pub preselect: bool,
    /// Deprecated flag
    pub deprecated: bool,
    /// Documentation (lazy loaded)
    pub documentation: Option<Documentation>,
    /// Additional text edits (lazy loaded)
    pub additional_edits: Option<Vec<TextEdit>>,
    /// Data for resolution
    pub data: Option<serde_json::Value>,
}

/// Completion kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

/// Documentation content
#[derive(Debug, Clone)]
pub enum Documentation {
    String(String),
    Markdown(String),
}

/// Text edit for additional edits
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: TextRange,
    pub new_text: String,
}

/// Text range
#[derive(Debug, Clone, Copy)]
pub struct TextRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// Completion resolver trait
#[async_trait]
pub trait CompletionResolver: Send + Sync {
    /// Resolve additional details for a completion item
    async fn resolve(&self, item: &mut ResolvableCompletion) -> anyhow::Result<()>;
}

/// Caching completion resolver
pub struct CachingResolver {
    /// Inner resolver
    inner: Arc<dyn CompletionResolver>,
    /// Cache of resolved items
    cache: RwLock<HashMap<String, ResolvedDetails>>,
}

/// Cached resolved details
#[derive(Debug, Clone)]
struct ResolvedDetails {
    documentation: Option<Documentation>,
    additional_edits: Option<Vec<TextEdit>>,
}

impl CachingResolver {
    pub fn new(inner: Arc<dyn CompletionResolver>) -> Self {
        Self {
            inner,
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub async fn resolve(&self, item: &mut ResolvableCompletion) -> anyhow::Result<()> {
        // Check cache first
        if let Some(cached) = self.cache.read().get(&item.id) {
            item.documentation = cached.documentation.clone();
            item.additional_edits = cached.additional_edits.clone();
            return Ok(());
        }

        // Resolve from inner
        self.inner.resolve(item).await?;

        // Cache result
        self.cache.write().insert(item.id.clone(), ResolvedDetails {
            documentation: item.documentation.clone(),
            additional_edits: item.additional_edits.clone(),
        });

        Ok(())
    }

    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}

/// Completion ranking
pub struct CompletionRanker {
    /// Boost for exact prefix matches
    pub exact_prefix_boost: i32,
    /// Boost for recently used items
    pub recency_boost: i32,
    /// Boost for items from current file
    pub locality_boost: i32,
    /// Penalty for deprecated items
    pub deprecated_penalty: i32,
    /// Recently used items (label -> timestamp)
    recent_items: RwLock<HashMap<String, u64>>,
}

impl Default for CompletionRanker {
    fn default() -> Self {
        Self {
            exact_prefix_boost: 100,
            recency_boost: 50,
            locality_boost: 25,
            deprecated_penalty: -100,
            recent_items: RwLock::new(HashMap::new()),
        }
    }
}

impl CompletionRanker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate score for a completion item
    pub fn score(&self, item: &ResolvableCompletion, query: &str, is_local: bool) -> i32 {
        let mut score = 0;

        // Exact prefix match
        if item.label.starts_with(query) {
            score += self.exact_prefix_boost;
        }

        // Recent usage
        if let Some(&timestamp) = self.recent_items.read().get(&item.label) {
            let age = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                .saturating_sub(timestamp);
            
            // Boost decreases with age (max 1 hour)
            let recency_factor = (3600u64.saturating_sub(age.min(3600))) as i32 / 72;
            score += self.recency_boost * recency_factor / 50;
        }

        // Locality
        if is_local {
            score += self.locality_boost;
        }

        // Deprecated penalty
        if item.deprecated {
            score += self.deprecated_penalty;
        }

        // Kind-based boost
        score += Self::kind_score(item.kind);

        score
    }

    fn kind_score(kind: CompletionKind) -> i32 {
        match kind {
            CompletionKind::Variable => 10,
            CompletionKind::Function | CompletionKind::Method => 8,
            CompletionKind::Field | CompletionKind::Property => 7,
            CompletionKind::Class | CompletionKind::Struct => 6,
            CompletionKind::Interface => 5,
            CompletionKind::Keyword => 4,
            CompletionKind::Snippet => 3,
            _ => 0,
        }
    }

    /// Record item usage
    pub fn record_usage(&self, label: &str) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.recent_items.write().insert(label.to_string(), timestamp);
    }

    /// Sort completions by score
    pub fn rank(&self, items: &mut [ResolvableCompletion], query: &str, local_symbols: &[String]) {
        let local_set: std::collections::HashSet<_> = local_symbols.iter().collect();
        
        items.sort_by(|a, b| {
            let score_a = self.score(a, query, local_set.contains(&a.label));
            let score_b = self.score(b, query, local_set.contains(&b.label));
            score_b.cmp(&score_a) // Descending
        });
    }
}

/// Completion deduplication
pub struct CompletionDeduplicator;

impl CompletionDeduplicator {
    /// Remove duplicate completions, keeping the best one
    pub fn deduplicate(items: Vec<ResolvableCompletion>) -> Vec<ResolvableCompletion> {
        let mut seen: HashMap<String, ResolvableCompletion> = HashMap::new();
        
        for item in items {
            let key = format!("{}:{:?}", item.label, item.kind);
            
            if let Some(existing) = seen.get(&key) {
                // Keep the one with more details
                if item.documentation.is_some() && existing.documentation.is_none() {
                    seen.insert(key, item);
                }
            } else {
                seen.insert(key, item);
            }
        }
        
        seen.into_values().collect()
    }
}

/// Import completion helper
#[derive(Debug, Clone)]
pub struct ImportCompletion {
    pub label: String,
    pub module_path: String,
    pub import_kind: ImportKind,
}

#[derive(Debug, Clone, Copy)]
pub enum ImportKind {
    Default,
    Named,
    Namespace,
    Type,
}

impl ImportCompletion {
    /// Generate import statement
    pub fn to_import_statement(&self, language: &str) -> String {
        match language {
            "typescript" | "javascript" => match self.import_kind {
                ImportKind::Default => format!("import {} from '{}';", self.label, self.module_path),
                ImportKind::Named => format!("import {{ {} }} from '{}';", self.label, self.module_path),
                ImportKind::Namespace => format!("import * as {} from '{}';", self.label, self.module_path),
                ImportKind::Type => format!("import type {{ {} }} from '{}';", self.label, self.module_path),
            },
            "python" => format!("from {} import {}", self.module_path, self.label),
            "rust" => format!("use {}::{};", self.module_path, self.label),
            "go" => format!("import \"{}\"", self.module_path),
            _ => format!("import {}", self.label),
        }
    }
}
