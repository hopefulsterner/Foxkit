//! # Foxkit Completion
//!
//! Code completion and IntelliSense (LSP-compatible).

pub mod item;
pub mod provider;
pub mod resolve;
pub mod snippet;
pub mod trigger;

pub use resolve::{ResolvableCompletion, CompletionRanker, CompletionDeduplicator, ImportCompletion};
pub use snippet::{Snippet, SnippetElement, SnippetTemplates};

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub use item::{CompletionItem, CompletionKind, CompletionDetail};
pub use provider::CompletionProvider;
pub use trigger::{TriggerKind, TriggerCharacter};

/// Completion list
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompletionList {
    /// Items
    pub items: Vec<CompletionItem>,
    /// Is incomplete (more items available)
    pub is_incomplete: bool,
}

impl CompletionList {
    pub fn new(items: Vec<CompletionItem>) -> Self {
        Self {
            items,
            is_incomplete: false,
        }
    }

    pub fn incomplete(items: Vec<CompletionItem>) -> Self {
        Self {
            items,
            is_incomplete: true,
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Filter and sort items
    pub fn filter(&mut self, query: &str) {
        if query.is_empty() {
            return;
        }

        let query_lower = query.to_lowercase();

        // Filter and score items
        let mut scored: Vec<_> = self.items
            .iter()
            .filter_map(|item| {
                let score = fuzzy_score(&item.label, &query_lower);
                if score > 0.0 {
                    Some((item.clone(), score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        self.items = scored.into_iter().map(|(item, _)| item).collect();
    }
}

/// Fuzzy scoring function
fn fuzzy_score(label: &str, query: &str) -> f64 {
    let label_lower = label.to_lowercase();
    
    // Exact prefix match
    if label_lower.starts_with(query) {
        return 100.0 + (1.0 / label.len() as f64);
    }

    // Contains match
    if label_lower.contains(query) {
        return 50.0 + (1.0 / label.len() as f64);
    }

    // Fuzzy character match
    let mut score = 0.0;
    let mut query_idx = 0;
    let query_chars: Vec<char> = query.chars().collect();
    let mut consecutive = 0;

    for (i, c) in label_lower.chars().enumerate() {
        if query_idx < query_chars.len() && c == query_chars[query_idx] {
            score += 1.0;
            if consecutive > 0 {
                score += consecutive as f64 * 0.5;
            }
            // Bonus for start of word
            if i == 0 || !label.chars().nth(i - 1).map(|c| c.is_alphanumeric()).unwrap_or(true) {
                score += 2.0;
            }
            consecutive += 1;
            query_idx += 1;
        } else {
            consecutive = 0;
        }
    }

    if query_idx == query_chars.len() {
        score / label.len() as f64 * 10.0
    } else {
        0.0
    }
}

/// Completion context
#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// Trigger kind
    pub trigger_kind: TriggerKind,
    /// Trigger character (if trigger_kind is TriggerCharacter)
    pub trigger_character: Option<char>,
    /// Word at cursor
    pub word: String,
    /// Full line
    pub line: String,
    /// Line number (0-indexed)
    pub line_number: u32,
    /// Column (0-indexed)
    pub column: u32,
    /// Language ID
    pub language_id: String,
    /// File path
    pub file_path: String,
}

impl CompletionContext {
    pub fn invoked(word: &str, line: &str, line_number: u32, column: u32) -> Self {
        Self {
            trigger_kind: TriggerKind::Invoked,
            trigger_character: None,
            word: word.to_string(),
            line: line.to_string(),
            line_number,
            column,
            language_id: String::new(),
            file_path: String::new(),
        }
    }

    pub fn triggered(char: char, word: &str, line: &str, line_number: u32, column: u32) -> Self {
        Self {
            trigger_kind: TriggerKind::TriggerCharacter,
            trigger_character: Some(char),
            word: word.to_string(),
            line: line.to_string(),
            line_number,
            column,
            language_id: String::new(),
            file_path: String::new(),
        }
    }

    pub fn with_language(mut self, language_id: &str) -> Self {
        self.language_id = language_id.to_string();
        self
    }

    pub fn with_file(mut self, file_path: &str) -> Self {
        self.file_path = file_path.to_string();
        self
    }
}

/// Completion service
pub struct CompletionService {
    providers: RwLock<Vec<Arc<dyn CompletionProvider>>>,
}

impl CompletionService {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(Vec::new()),
        }
    }

    /// Register a provider
    pub fn register(&self, provider: Arc<dyn CompletionProvider>) {
        self.providers.write().push(provider);
    }

    /// Get completions from all providers
    pub fn complete(&self, context: &CompletionContext) -> CompletionList {
        let providers = self.providers.read();
        let mut all_items = Vec::new();
        let mut is_incomplete = false;

        for provider in providers.iter() {
            if provider.should_provide(context) {
                let list = provider.provide(context);
                is_incomplete |= list.is_incomplete;
                all_items.extend(list.items);
            }
        }

        CompletionList {
            items: all_items,
            is_incomplete,
        }
    }

    /// Resolve additional details for completion item
    pub fn resolve(&self, item: &CompletionItem) -> CompletionItem {
        let providers = self.providers.read();
        
        for provider in providers.iter() {
            if let Some(resolved) = provider.resolve(item) {
                return resolved;
            }
        }

        item.clone()
    }
}

impl Default for CompletionService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_score() {
        assert!(fuzzy_score("println", "println") > fuzzy_score("print", "println"));
        assert!(fuzzy_score("println", "pln") > 0.0);
        assert!(fuzzy_score("print_line", "pline") > 0.0);
    }

    #[test]
    fn test_filter() {
        let mut list = CompletionList::new(vec![
            CompletionItem::simple("println"),
            CompletionItem::simple("print"),
            CompletionItem::simple("format"),
        ]);

        list.filter("pr");
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].label, "print");
    }
}
