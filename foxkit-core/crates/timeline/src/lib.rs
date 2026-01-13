//! # Foxkit Timeline
//!
//! File history timeline view.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Timeline service
pub struct TimelineService {
    /// Registered providers
    providers: RwLock<Vec<Arc<dyn TimelineProvider>>>,
    /// Cached timeline items
    cache: RwLock<HashMap<PathBuf, Vec<TimelineItem>>>,
    /// Events
    events: broadcast::Sender<TimelineEvent>,
    /// Configuration
    config: RwLock<TimelineConfig>,
}

impl TimelineService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            providers: RwLock::new(Vec::new()),
            cache: RwLock::new(HashMap::new()),
            events,
            config: RwLock::new(TimelineConfig::default()),
        }
    }

    /// Register a timeline provider
    pub fn register_provider(&self, provider: Arc<dyn TimelineProvider>) {
        self.providers.write().push(provider);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<TimelineEvent> {
        self.events.subscribe()
    }

    /// Get timeline for a file
    pub async fn get_timeline(&self, file: &PathBuf) -> anyhow::Result<Vec<TimelineItem>> {
        let providers = self.providers.read().clone();
        let config = self.config.read().clone();
        let mut items = Vec::new();

        for provider in providers {
            if !config.enabled_sources.is_empty() 
                && !config.enabled_sources.contains(&provider.id().to_string()) {
                continue;
            }

            match provider.get_items(file).await {
                Ok(provider_items) => items.extend(provider_items),
                Err(e) => {
                    tracing::warn!("Timeline provider {} failed: {}", provider.id(), e);
                }
            }
        }

        // Sort by timestamp (newest first by default)
        if config.sort_order == TimelineSortOrder::NewestFirst {
            items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        } else {
            items.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        }

        // Cache and return
        self.cache.write().insert(file.clone(), items.clone());

        Ok(items)
    }

    /// Invalidate cache
    pub fn invalidate(&self, file: &PathBuf) {
        self.cache.write().remove(file);
        let _ = self.events.send(TimelineEvent::Invalidated { file: file.clone() });
    }

    /// Configure timeline
    pub fn configure(&self, config: TimelineConfig) {
        *self.config.write() = config;
    }
}

impl Default for TimelineService {
    fn default() -> Self {
        Self::new()
    }
}

/// Timeline provider trait
#[async_trait::async_trait]
pub trait TimelineProvider: Send + Sync {
    /// Provider ID
    fn id(&self) -> &str;

    /// Display label
    fn label(&self) -> &str;

    /// Get timeline items for a file
    async fn get_items(&self, file: &PathBuf) -> anyhow::Result<Vec<TimelineItem>>;
}

/// Timeline item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineItem {
    /// Unique ID
    pub id: String,
    /// Source (provider ID)
    pub source: String,
    /// Label
    pub label: String,
    /// Description
    pub description: Option<String>,
    /// Detail text
    pub detail: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Icon
    pub icon: TimelineIcon,
    /// Command to execute on selection
    pub command: Option<TimelineCommand>,
    /// Context value (for menus)
    pub context_value: Option<String>,
}

impl TimelineItem {
    pub fn new(
        id: impl Into<String>,
        source: impl Into<String>,
        label: impl Into<String>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            label: label.into(),
            description: None,
            detail: None,
            timestamp,
            icon: TimelineIcon::default(),
            command: None,
            context_value: None,
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

    pub fn with_icon(mut self, icon: TimelineIcon) -> Self {
        self.icon = icon;
        self
    }

    pub fn with_command(mut self, command: TimelineCommand) -> Self {
        self.command = Some(command);
        self
    }

    /// Format relative time
    pub fn relative_time(&self) -> String {
        let now = Utc::now();
        let diff = now.signed_duration_since(self.timestamp);

        if diff.num_seconds() < 60 {
            "just now".to_string()
        } else if diff.num_minutes() < 60 {
            let mins = diff.num_minutes();
            format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" })
        } else if diff.num_hours() < 24 {
            let hours = diff.num_hours();
            format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
        } else if diff.num_days() < 7 {
            let days = diff.num_days();
            format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
        } else if diff.num_weeks() < 4 {
            let weeks = diff.num_weeks();
            format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" })
        } else {
            self.timestamp.format("%b %d, %Y").to_string()
        }
    }
}

/// Timeline icon
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineIcon {
    /// Icon ID or path
    pub id: String,
    /// Theme color
    pub color: Option<String>,
}

impl TimelineIcon {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            color: None,
        }
    }

    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn git_commit() -> Self {
        Self::new("git-commit")
    }

    pub fn save() -> Self {
        Self::new("save")
    }

    pub fn edit() -> Self {
        Self::new("edit")
    }
}

/// Timeline command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineCommand {
    pub id: String,
    pub title: String,
    pub args: Vec<serde_json::Value>,
}

impl TimelineCommand {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            args: Vec::new(),
        }
    }

    pub fn with_args(mut self, args: Vec<serde_json::Value>) -> Self {
        self.args = args;
        self
    }
}

/// Timeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineConfig {
    /// Enabled sources
    pub enabled_sources: Vec<String>,
    /// Page size
    pub page_size: usize,
    /// Sort order
    pub sort_order: TimelineSortOrder,
    /// Show relative time
    pub show_relative_time: bool,
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            enabled_sources: Vec::new(), // All sources
            page_size: 100,
            sort_order: TimelineSortOrder::NewestFirst,
            show_relative_time: true,
        }
    }
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimelineSortOrder {
    NewestFirst,
    OldestFirst,
}

/// Timeline event
#[derive(Debug, Clone)]
pub enum TimelineEvent {
    Invalidated { file: PathBuf },
    ItemsLoaded { file: PathBuf, count: usize },
}

/// Git history provider
pub struct GitTimelineProvider {
    // Would have Git service reference
}

impl GitTimelineProvider {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for GitTimelineProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TimelineProvider for GitTimelineProvider {
    fn id(&self) -> &str {
        "git"
    }

    fn label(&self) -> &str {
        "Git History"
    }

    async fn get_items(&self, file: &PathBuf) -> anyhow::Result<Vec<TimelineItem>> {
        // Would query Git for file history
        Ok(Vec::new())
    }
}

/// Local history provider
pub struct LocalHistoryProvider {
    /// History directory
    history_dir: PathBuf,
}

impl LocalHistoryProvider {
    pub fn new(history_dir: PathBuf) -> Self {
        Self { history_dir }
    }
}

#[async_trait::async_trait]
impl TimelineProvider for LocalHistoryProvider {
    fn id(&self) -> &str {
        "local-history"
    }

    fn label(&self) -> &str {
        "Local History"
    }

    async fn get_items(&self, file: &PathBuf) -> anyhow::Result<Vec<TimelineItem>> {
        // Would scan local history directory for saves
        Ok(Vec::new())
    }
}

/// Timeline view model
pub struct TimelineViewModel {
    service: Arc<TimelineService>,
    /// Current file
    file: RwLock<Option<PathBuf>>,
    /// Loaded items
    items: RwLock<Vec<TimelineItem>>,
    /// Selected index
    selected: RwLock<Option<usize>>,
    /// Loading state
    loading: RwLock<bool>,
}

impl TimelineViewModel {
    pub fn new(service: Arc<TimelineService>) -> Self {
        Self {
            service,
            file: RwLock::new(None),
            items: RwLock::new(Vec::new()),
            selected: RwLock::new(None),
            loading: RwLock::new(false),
        }
    }

    pub async fn load(&self, file: PathBuf) -> anyhow::Result<()> {
        *self.loading.write() = true;
        *self.file.write() = Some(file.clone());

        let items = self.service.get_timeline(&file).await?;
        *self.items.write() = items;
        *self.loading.write() = false;

        Ok(())
    }

    pub fn items(&self) -> Vec<TimelineItem> {
        self.items.read().clone()
    }

    pub fn is_loading(&self) -> bool {
        *self.loading.read()
    }

    pub fn select(&self, index: usize) {
        *self.selected.write() = Some(index);
    }

    pub fn selected(&self) -> Option<TimelineItem> {
        let index = (*self.selected.read())?;
        self.items.read().get(index).cloned()
    }

    pub fn refresh(&self) {
        if let Some(file) = self.file.read().clone() {
            self.service.invalidate(&file);
        }
    }
}
