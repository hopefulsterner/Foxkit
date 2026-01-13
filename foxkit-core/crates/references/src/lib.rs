//! # Foxkit References
//!
//! Find all references to symbols throughout the workspace.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

pub use goto_definition::{Location, Position, Range};

/// References service
pub struct ReferencesService {
    /// Registered providers
    providers: RwLock<Vec<Arc<dyn ReferencesProvider>>>,
    /// Events
    events: broadcast::Sender<ReferencesEvent>,
    /// Configuration
    config: RwLock<ReferencesConfig>,
}

impl ReferencesService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            providers: RwLock::new(Vec::new()),
            events,
            config: RwLock::new(ReferencesConfig::default()),
        }
    }

    /// Register provider
    pub fn register_provider(&self, provider: Arc<dyn ReferencesProvider>) {
        self.providers.write().push(provider);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<ReferencesEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: ReferencesConfig) {
        *self.config.write() = config;
    }

    /// Find references
    pub async fn find_references(
        &self,
        file: &PathBuf,
        position: Position,
        context: ReferenceContext,
    ) -> anyhow::Result<Vec<Reference>> {
        let providers = self.providers.read().clone();
        let mut references = Vec::new();

        let _ = self.events.send(ReferencesEvent::Searching {
            file: file.clone(),
        });

        for provider in providers {
            match provider.find_references(file, &position, &context).await {
                Ok(refs) => references.extend(refs),
                Err(e) => {
                    tracing::warn!("Reference provider failed: {}", e);
                }
            }
        }

        // Group by file
        let grouped = group_by_file(&references);

        let _ = self.events.send(ReferencesEvent::Found {
            count: references.len(),
            file_count: grouped.len(),
        });

        Ok(references)
    }

    /// Find references with preview
    pub async fn find_references_with_preview(
        &self,
        file: &PathBuf,
        position: Position,
        context: ReferenceContext,
    ) -> anyhow::Result<ReferencesResult> {
        let references = self.find_references(file, position, context).await?;
        let grouped = group_by_file(&references);

        Ok(ReferencesResult {
            references: references.clone(),
            by_file: grouped,
            total_count: references.len(),
        })
    }
}

impl Default for ReferencesService {
    fn default() -> Self {
        Self::new()
    }
}

/// Group references by file
fn group_by_file(references: &[Reference]) -> HashMap<PathBuf, Vec<Reference>> {
    let mut grouped: HashMap<PathBuf, Vec<Reference>> = HashMap::new();
    
    for reference in references {
        grouped
            .entry(reference.location.file.clone())
            .or_default()
            .push(reference.clone());
    }

    // Sort references within each file by line
    for refs in grouped.values_mut() {
        refs.sort_by_key(|r| (r.location.range.start.line, r.location.range.start.col));
    }

    grouped
}

/// References provider trait
#[async_trait::async_trait]
pub trait ReferencesProvider: Send + Sync {
    /// Provider ID
    fn id(&self) -> &str;

    /// Find references
    async fn find_references(
        &self,
        file: &PathBuf,
        position: &Position,
        context: &ReferenceContext,
    ) -> anyhow::Result<Vec<Reference>>;
}

/// Reference context
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReferenceContext {
    /// Include the declaration
    pub include_declaration: bool,
}

impl ReferenceContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn include_declaration(mut self) -> Self {
        self.include_declaration = true;
        self
    }
}

/// Reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    /// Location
    pub location: Location,
    /// Reference kind
    pub kind: ReferenceKind,
    /// Is definition/declaration
    pub is_definition: bool,
    /// Preview text
    pub preview: Option<String>,
}

impl Reference {
    pub fn new(location: Location) -> Self {
        Self {
            location,
            kind: ReferenceKind::Read,
            is_definition: false,
            preview: None,
        }
    }

    pub fn read(location: Location) -> Self {
        Self {
            location,
            kind: ReferenceKind::Read,
            is_definition: false,
            preview: None,
        }
    }

    pub fn write(location: Location) -> Self {
        Self {
            location,
            kind: ReferenceKind::Write,
            is_definition: false,
            preview: None,
        }
    }

    pub fn definition(location: Location) -> Self {
        Self {
            location,
            kind: ReferenceKind::Read,
            is_definition: true,
            preview: None,
        }
    }

    pub fn with_preview(mut self, preview: impl Into<String>) -> Self {
        self.preview = Some(preview.into());
        self
    }

    pub fn with_kind(mut self, kind: ReferenceKind) -> Self {
        self.kind = kind;
        self
    }
}

/// Reference kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceKind {
    /// Read access
    Read,
    /// Write access
    Write,
    /// Text (in comments/strings)
    Text,
}

impl ReferenceKind {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Read => "$(symbol-variable)",
            Self::Write => "$(edit)",
            Self::Text => "$(symbol-string)",
        }
    }
}

/// References result
#[derive(Debug, Clone)]
pub struct ReferencesResult {
    /// All references
    pub references: Vec<Reference>,
    /// Grouped by file
    pub by_file: HashMap<PathBuf, Vec<Reference>>,
    /// Total count
    pub total_count: usize,
}

impl ReferencesResult {
    pub fn file_count(&self) -> usize {
        self.by_file.len()
    }

    pub fn files(&self) -> impl Iterator<Item = &PathBuf> {
        self.by_file.keys()
    }

    pub fn references_in_file(&self, file: &PathBuf) -> Option<&Vec<Reference>> {
        self.by_file.get(file)
    }
}

/// References configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencesConfig {
    /// Include declaration by default
    pub include_declaration: bool,
    /// Max results
    pub max_results: usize,
    /// Show in sidebar
    pub show_in_sidebar: bool,
}

impl Default for ReferencesConfig {
    fn default() -> Self {
        Self {
            include_declaration: true,
            max_results: 1000,
            show_in_sidebar: true,
        }
    }
}

/// References event
#[derive(Debug, Clone)]
pub enum ReferencesEvent {
    Searching { file: PathBuf },
    Found { count: usize, file_count: usize },
    Error { error: String },
}

/// References panel view model
pub struct ReferencesPanelViewModel {
    service: Arc<ReferencesService>,
    /// Current result
    result: RwLock<Option<ReferencesResult>>,
    /// Selected reference
    selected: RwLock<Option<usize>>,
    /// Expanded files
    expanded: RwLock<Vec<PathBuf>>,
    /// Is loading
    loading: RwLock<bool>,
}

impl ReferencesPanelViewModel {
    pub fn new(service: Arc<ReferencesService>) -> Self {
        Self {
            service,
            result: RwLock::new(None),
            selected: RwLock::new(None),
            expanded: RwLock::new(Vec::new()),
            loading: RwLock::new(false),
        }
    }

    pub async fn search(&self, file: &PathBuf, position: Position, include_declaration: bool) {
        *self.loading.write() = true;

        let context = if include_declaration {
            ReferenceContext::new().include_declaration()
        } else {
            ReferenceContext::new()
        };

        match self.service.find_references_with_preview(file, position, context).await {
            Ok(result) => {
                // Auto-expand all files
                *self.expanded.write() = result.files().cloned().collect();
                *self.result.write() = Some(result);
                *self.selected.write() = Some(0);
            }
            Err(e) => {
                tracing::error!("Failed to find references: {}", e);
            }
        }

        *self.loading.write() = false;
    }

    pub fn result(&self) -> Option<ReferencesResult> {
        self.result.read().clone()
    }

    pub fn selected(&self) -> Option<Reference> {
        let selected = *self.selected.read();
        let result = self.result.read();
        
        selected.and_then(|idx| {
            result.as_ref().and_then(|r| r.references.get(idx).cloned())
        })
    }

    pub fn select(&self, index: usize) {
        if let Some(ref result) = *self.result.read() {
            if index < result.total_count {
                *self.selected.write() = Some(index);
            }
        }
    }

    pub fn select_next(&self) {
        let mut selected = self.selected.write();
        if let Some(ref result) = *self.result.read() {
            if let Some(idx) = *selected {
                if idx + 1 < result.total_count {
                    *selected = Some(idx + 1);
                }
            }
        }
    }

    pub fn select_previous(&self) {
        let mut selected = self.selected.write();
        if let Some(idx) = *selected {
            if idx > 0 {
                *selected = Some(idx - 1);
            }
        }
    }

    pub fn toggle_file(&self, file: &PathBuf) {
        let mut expanded = self.expanded.write();
        if let Some(pos) = expanded.iter().position(|f| f == file) {
            expanded.remove(pos);
        } else {
            expanded.push(file.clone());
        }
    }

    pub fn is_file_expanded(&self, file: &PathBuf) -> bool {
        self.expanded.read().contains(file)
    }

    pub fn is_loading(&self) -> bool {
        *self.loading.read()
    }

    pub fn clear(&self) {
        *self.result.write() = None;
        *self.selected.write() = None;
        self.expanded.write().clear();
    }
}
