//! Quick pick widget

use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::FuzzyMatcher;

/// Quick pick widget
pub struct QuickPick {
    /// Options
    options: QuickPickOptions,
    /// All items
    items: RwLock<Vec<QuickPickItem>>,
    /// Filtered items
    filtered: RwLock<Vec<FilteredItem>>,
    /// Current input
    input: RwLock<String>,
    /// Selected index
    selected_index: RwLock<usize>,
    /// Fuzzy matcher
    matcher: FuzzyMatcher,
}

impl QuickPick {
    pub fn new(options: QuickPickOptions) -> Self {
        let items = options.items.clone();
        
        Self {
            options,
            items: RwLock::new(items.clone()),
            filtered: RwLock::new(items.into_iter().enumerate()
                .map(|(i, item)| FilteredItem { item, score: 0, original_index: i })
                .collect()),
            input: RwLock::new(String::new()),
            selected_index: RwLock::new(0),
            matcher: FuzzyMatcher::new(),
        }
    }

    /// Set items
    pub fn set_items(&self, items: Vec<QuickPickItem>) {
        *self.items.write() = items.clone();
        self.filter(&self.input.read());
    }

    /// Filter items
    pub fn filter(&self, query: &str) {
        *self.input.write() = query.to_string();
        
        let items = self.items.read();
        let mut filtered: Vec<FilteredItem> = if query.is_empty() {
            items.iter().enumerate()
                .map(|(i, item)| FilteredItem {
                    item: item.clone(),
                    score: 0,
                    original_index: i,
                })
                .collect()
        } else {
            items.iter().enumerate()
                .filter_map(|(i, item)| {
                    let score = self.matcher.score(&item.label, query)
                        .or_else(|| {
                            if self.options.match_on_description {
                                item.description.as_ref()
                                    .and_then(|d| self.matcher.score(d, query))
                            } else {
                                None
                            }
                        });
                    
                    score.map(|s| FilteredItem {
                        item: item.clone(),
                        score: s,
                        original_index: i,
                    })
                })
                .collect()
        };

        // Sort by score
        if self.options.sort_by_score && !query.is_empty() {
            filtered.sort_by(|a, b| b.score.cmp(&a.score));
        }

        *self.filtered.write() = filtered;
        *self.selected_index.write() = 0;
    }

    /// Get filtered items
    pub fn filtered_items(&self) -> Vec<QuickPickItem> {
        self.filtered.read().iter()
            .take(self.options.max_items.unwrap_or(usize::MAX))
            .map(|f| f.item.clone())
            .collect()
    }

    /// Get current input
    pub fn input(&self) -> String {
        self.input.read().clone()
    }

    /// Select by index
    pub fn select(&self, index: usize) {
        let len = self.filtered.read().len();
        if index < len {
            *self.selected_index.write() = index;
        }
    }

    /// Move selection up
    pub fn select_previous(&self) {
        let mut index = self.selected_index.write();
        if *index > 0 {
            *index -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&self) {
        let len = self.filtered.read().len();
        let mut index = self.selected_index.write();
        if *index + 1 < len {
            *index += 1;
        }
    }

    /// Get selected index
    pub fn selected_index(&self) -> usize {
        *self.selected_index.read()
    }

    /// Get selected item
    pub fn selected_item(&self) -> Option<QuickPickItem> {
        let index = *self.selected_index.read();
        self.filtered.read().get(index)
            .map(|f| f.item.clone())
    }

    /// Total count
    pub fn total_count(&self) -> usize {
        self.items.read().len()
    }

    /// Filtered count
    pub fn filtered_count(&self) -> usize {
        self.filtered.read().len()
    }
}

/// Quick pick item
#[derive(Debug, Serialize, Deserialize)]
pub struct QuickPickItem {
    /// Label
    pub label: String,
    /// Description (shown after label)
    pub description: Option<String>,
    /// Detail (shown below label)
    pub detail: Option<String>,
    /// Icon ID
    pub icon: Option<String>,
    /// Is picked
    pub picked: bool,
    /// Always show (don't filter out)
    pub always_show: bool,
    /// Custom data
    #[serde(skip)]
    pub data: Option<Box<dyn std::any::Any + Send + Sync>>,
}

impl Clone for QuickPickItem {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            description: self.description.clone(),
            detail: self.detail.clone(),
            icon: self.icon.clone(),
            picked: self.picked,
            always_show: self.always_show,
            data: None, // Cannot clone dyn Any
        }
    }
}

impl QuickPickItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            detail: None,
            icon: None,
            picked: false,
            always_show: false,
            data: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// Filtered item with score
#[derive(Debug, Clone)]
struct FilteredItem {
    item: QuickPickItem,
    score: i64,
    original_index: usize,
}

/// Quick pick options
#[derive(Debug, Clone, Default)]
pub struct QuickPickOptions {
    /// Title
    pub title: Option<String>,
    /// Placeholder text
    pub placeholder: Option<String>,
    /// Items
    pub items: Vec<QuickPickItem>,
    /// Can pick many
    pub can_pick_many: bool,
    /// Match on description
    pub match_on_description: bool,
    /// Match on detail
    pub match_on_detail: bool,
    /// Sort by score
    pub sort_by_score: bool,
    /// Busy indicator
    pub busy: bool,
    /// Max items to show
    pub max_items: Option<usize>,
    /// Ignore focus out
    pub ignore_focus_out: bool,
}

impl QuickPickOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn with_items(mut self, items: Vec<QuickPickItem>) -> Self {
        self.items = items;
        self
    }
}

/// Quick input box
pub struct InputBox {
    /// Title
    pub title: Option<String>,
    /// Placeholder
    pub placeholder: Option<String>,
    /// Value
    pub value: String,
    /// Validation message
    pub validation_message: Option<String>,
    /// Is password
    pub password: bool,
    /// Prompt
    pub prompt: Option<String>,
}

impl InputBox {
    pub fn new() -> Self {
        Self {
            title: None,
            placeholder: None,
            value: String::new(),
            validation_message: None,
            password: false,
            prompt: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn as_password(mut self) -> Self {
        self.password = true;
        self
    }

    /// Validate input
    pub fn validate<F>(&mut self, f: F)
    where
        F: Fn(&str) -> Option<String>,
    {
        self.validation_message = f(&self.value);
    }
}

impl Default for InputBox {
    fn default() -> Self {
        Self::new()
    }
}
