//! # Foxkit Quick Open
//!
//! Command palette and fuzzy finder.

pub mod picker;
pub mod providers;
pub mod fuzzy;
pub mod palette;
pub mod symbols;

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

pub use picker::{QuickPick, QuickPickItem, QuickPickOptions};
pub use providers::{Provider, ProviderContext};
pub use fuzzy::FuzzyMatcher;
pub use palette::CommandPalette;

/// Quick open service
pub struct QuickOpenService {
    /// Active picker
    active_picker: RwLock<Option<Arc<QuickPick>>>,
    /// Registered providers
    providers: RwLock<HashMap<String, Arc<dyn Provider>>>,
    /// Event channel
    events: broadcast::Sender<QuickOpenEvent>,
    /// Configuration
    config: RwLock<QuickOpenConfig>,
    /// Recent items
    recent: RwLock<Vec<RecentItem>>,
}

impl QuickOpenService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);
        
        Self {
            active_picker: RwLock::new(None),
            providers: RwLock::new(HashMap::new()),
            events,
            config: RwLock::new(QuickOpenConfig::default()),
            recent: RwLock::new(Vec::new()),
        }
    }

    /// Configure quick open
    pub fn configure(&self, config: QuickOpenConfig) {
        *self.config.write() = config;
    }

    /// Register a provider
    pub fn register<P: Provider + 'static>(&self, id: &str, provider: P) {
        self.providers.write().insert(id.to_string(), Arc::new(provider));
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<QuickOpenEvent> {
        self.events.subscribe()
    }

    /// Show quick pick
    pub async fn show(&self, options: QuickPickOptions) -> Option<QuickPickItem> {
        let picker = Arc::new(QuickPick::new(options));
        *self.active_picker.write() = Some(picker.clone());
        
        let _ = self.events.send(QuickOpenEvent::Opened);
        
        // Would await user selection
        // For now, return None
        None
    }

    /// Show command palette
    pub async fn show_commands(&self) {
        let _ = self.events.send(QuickOpenEvent::Opened);
        // Would show command palette
    }

    /// Show file picker
    pub async fn show_files(&self) {
        let _ = self.events.send(QuickOpenEvent::Opened);
        // Would show file picker
    }

    /// Show symbol picker
    pub async fn show_symbols(&self, workspace_symbols: bool) {
        let _ = self.events.send(QuickOpenEvent::Opened);
        // Would show symbol picker
    }

    /// Update input
    pub fn update_input(&self, input: &str) {
        if let Some(picker) = self.active_picker.read().as_ref() {
            picker.filter(input);
        }
        
        let _ = self.events.send(QuickOpenEvent::InputChanged {
            input: input.to_string(),
        });
    }

    /// Select item
    pub fn select(&self, index: usize) {
        if let Some(picker) = self.active_picker.read().as_ref() {
            picker.select(index);
        }
    }

    /// Accept selection
    pub fn accept(&self) {
        if let Some(picker) = self.active_picker.read().as_ref() {
            if let Some(item) = picker.selected_item() {
                // Add to recent
                self.add_recent(RecentItem {
                    label: item.label.clone(),
                    description: item.description.clone(),
                    kind: RecentItemKind::File,
                });
                
                let _ = self.events.send(QuickOpenEvent::Accepted {
                    item: item.clone(),
                });
            }
        }
        
        self.hide();
    }

    /// Hide picker
    pub fn hide(&self) {
        *self.active_picker.write() = None;
        let _ = self.events.send(QuickOpenEvent::Closed);
    }

    /// Add recent item
    fn add_recent(&self, item: RecentItem) {
        let mut recent = self.recent.write();
        
        // Remove if exists
        recent.retain(|r| r.label != item.label);
        
        // Add to front
        recent.insert(0, item);
        
        // Limit size
        let max = self.config.read().max_recent;
        recent.truncate(max);
    }

    /// Get recent items
    pub fn recent_items(&self) -> Vec<RecentItem> {
        self.recent.read().clone()
    }

    /// Is active?
    pub fn is_active(&self) -> bool {
        self.active_picker.read().is_some()
    }
}

impl Default for QuickOpenService {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick open configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickOpenConfig {
    /// Max items to display
    pub max_items: usize,
    /// Max recent items
    pub max_recent: usize,
    /// Preserve input between opens
    pub preserve_input: bool,
    /// Show icons
    pub show_icons: bool,
    /// Match on description
    pub match_on_description: bool,
    /// Sort by score
    pub sort_by_score: bool,
}

impl Default for QuickOpenConfig {
    fn default() -> Self {
        Self {
            max_items: 100,
            max_recent: 10,
            preserve_input: false,
            show_icons: true,
            match_on_description: true,
            sort_by_score: true,
        }
    }
}

/// Quick open event
#[derive(Debug, Clone)]
pub enum QuickOpenEvent {
    Opened,
    Closed,
    InputChanged { input: String },
    SelectionChanged { index: usize },
    Accepted { item: QuickPickItem },
}

/// Recent item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentItem {
    pub label: String,
    pub description: Option<String>,
    pub kind: RecentItemKind,
}

/// Recent item kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RecentItemKind {
    File,
    Command,
    Symbol,
    Workspace,
}
