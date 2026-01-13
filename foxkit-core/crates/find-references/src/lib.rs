//! # Foxkit Find References
//!
//! Find all references to a symbol across the workspace.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Find references service
pub struct FindReferencesService {
    /// Current results
    results: RwLock<Option<ReferencesResult>>,
    /// Events
    events: broadcast::Sender<ReferencesEvent>,
    /// Configuration
    config: RwLock<ReferencesConfig>,
}

impl FindReferencesService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            results: RwLock::new(None),
            events,
            config: RwLock::new(ReferencesConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<ReferencesEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: ReferencesConfig) {
        *self.config.write() = config;
    }

    /// Find all references
    pub async fn find_references(
        &self,
        file: &PathBuf,
        position: ReferencePosition,
        context: ReferenceContext,
    ) -> anyhow::Result<ReferencesResult> {
        let _ = self.events.send(ReferencesEvent::Searching {
            file: file.clone(),
        });

        // Would call LSP textDocument/references
        // For now, return empty result
        let result = ReferencesResult {
            symbol_name: String::new(),
            references: Vec::new(),
            include_declaration: context.include_declaration,
        };

        *self.results.write() = Some(result.clone());

        let _ = self.events.send(ReferencesEvent::Found {
            count: result.references.len(),
        });

        Ok(result)
    }

    /// Get current results
    pub fn current(&self) -> Option<ReferencesResult> {
        self.results.read().clone()
    }

    /// Clear results
    pub fn clear(&self) {
        *self.results.write() = None;
        let _ = self.events.send(ReferencesEvent::Cleared);
    }

    /// Get references grouped by file
    pub fn references_by_file(&self) -> HashMap<PathBuf, Vec<Reference>> {
        let mut by_file: HashMap<PathBuf, Vec<Reference>> = HashMap::new();

        if let Some(ref results) = *self.results.read() {
            for reference in &results.references {
                by_file
                    .entry(reference.location.file.clone())
                    .or_default()
                    .push(reference.clone());
            }
        }

        by_file
    }

    /// Get reference count
    pub fn count(&self) -> usize {
        self.results.read()
            .as_ref()
            .map(|r| r.references.len())
            .unwrap_or(0)
    }

    /// Get file count
    pub fn file_count(&self) -> usize {
        self.references_by_file().len()
    }

    /// Navigate to next reference
    pub fn next_reference(&self, current: &ReferenceLocation) -> Option<Reference> {
        let results = self.results.read();
        let results = results.as_ref()?;

        let current_idx = results.references.iter()
            .position(|r| r.location == *current)?;

        let next_idx = (current_idx + 1) % results.references.len();
        results.references.get(next_idx).cloned()
    }

    /// Navigate to previous reference
    pub fn previous_reference(&self, current: &ReferenceLocation) -> Option<Reference> {
        let results = self.results.read();
        let results = results.as_ref()?;

        let current_idx = results.references.iter()
            .position(|r| r.location == *current)?;

        let prev_idx = if current_idx == 0 {
            results.references.len() - 1
        } else {
            current_idx - 1
        };

        results.references.get(prev_idx).cloned()
    }
}

impl Default for FindReferencesService {
    fn default() -> Self {
        Self::new()
    }
}

/// References result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencesResult {
    /// Symbol being referenced
    pub symbol_name: String,
    /// All references
    pub references: Vec<Reference>,
    /// Whether declaration is included
    pub include_declaration: bool,
}

impl ReferencesResult {
    pub fn declaration(&self) -> Option<&Reference> {
        self.references.iter().find(|r| r.is_declaration)
    }

    pub fn non_declarations(&self) -> Vec<&Reference> {
        self.references.iter().filter(|r| !r.is_declaration).collect()
    }
}

/// A single reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    /// Location
    pub location: ReferenceLocation,
    /// Is this the declaration
    pub is_declaration: bool,
    /// Is this a write access
    pub is_write: bool,
    /// Preview line
    pub preview: Option<String>,
}

impl Reference {
    pub fn new(location: ReferenceLocation) -> Self {
        Self {
            location,
            is_declaration: false,
            is_write: false,
            preview: None,
        }
    }

    pub fn declaration(location: ReferenceLocation) -> Self {
        Self {
            location,
            is_declaration: true,
            is_write: false,
            preview: None,
        }
    }

    pub fn write_access(location: ReferenceLocation) -> Self {
        Self {
            location,
            is_declaration: false,
            is_write: true,
            preview: None,
        }
    }

    pub fn with_preview(mut self, preview: impl Into<String>) -> Self {
        self.preview = Some(preview.into());
        self
    }
}

/// Reference location
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReferenceLocation {
    /// File path
    pub file: PathBuf,
    /// Range
    pub range: ReferenceRange,
}

impl ReferenceLocation {
    pub fn new(file: PathBuf, range: ReferenceRange) -> Self {
        Self { file, range }
    }
}

/// Reference range
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReferenceRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl ReferenceRange {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
    }

    pub fn single_line(line: u32, start_col: u32, end_col: u32) -> Self {
        Self { start_line: line, start_col, end_line: line, end_col }
    }
}

/// Reference position
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReferencePosition {
    pub line: u32,
    pub col: u32,
}

impl ReferencePosition {
    pub fn new(line: u32, col: u32) -> Self {
        Self { line, col }
    }
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

    pub fn with_declaration(mut self) -> Self {
        self.include_declaration = true;
        self
    }
}

/// References configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencesConfig {
    /// Include declaration by default
    pub include_declaration: bool,
    /// Show in peek view
    pub show_in_peek: bool,
    /// Maximum results
    pub max_results: usize,
}

impl Default for ReferencesConfig {
    fn default() -> Self {
        Self {
            include_declaration: true,
            show_in_peek: true,
            max_results: 1000,
        }
    }
}

/// References event
#[derive(Debug, Clone)]
pub enum ReferencesEvent {
    Searching { file: PathBuf },
    Found { count: usize },
    Cleared,
}

/// References view model for UI
pub struct ReferencesViewModel {
    service: Arc<FindReferencesService>,
    /// Expanded files
    expanded: RwLock<std::collections::HashSet<PathBuf>>,
    /// Selected reference index
    selected: RwLock<usize>,
}

impl ReferencesViewModel {
    pub fn new(service: Arc<FindReferencesService>) -> Self {
        Self {
            service,
            expanded: RwLock::new(std::collections::HashSet::new()),
            selected: RwLock::new(0),
        }
    }

    pub fn toggle_file(&self, file: &PathBuf) {
        let mut expanded = self.expanded.write();
        if expanded.contains(file) {
            expanded.remove(file);
        } else {
            expanded.insert(file.clone());
        }
    }

    pub fn is_expanded(&self, file: &PathBuf) -> bool {
        self.expanded.read().contains(file)
    }

    pub fn expand_all(&self) {
        let by_file = self.service.references_by_file();
        let mut expanded = self.expanded.write();
        for file in by_file.keys() {
            expanded.insert(file.clone());
        }
    }

    pub fn collapse_all(&self) {
        self.expanded.write().clear();
    }

    pub fn select(&self, index: usize) {
        *self.selected.write() = index;
    }

    pub fn selected(&self) -> Option<Reference> {
        let index = *self.selected.read();
        self.service.current()
            .and_then(|r| r.references.get(index).cloned())
    }

    pub fn select_next(&self) {
        let mut selected = self.selected.write();
        let count = self.service.count();
        if count > 0 {
            *selected = (*selected + 1) % count;
        }
    }

    pub fn select_previous(&self) {
        let mut selected = self.selected.write();
        let count = self.service.count();
        if count > 0 {
            *selected = if *selected == 0 { count - 1 } else { *selected - 1 };
        }
    }

    pub fn status_text(&self) -> String {
        let count = self.service.count();
        let files = self.service.file_count();
        format!("{} references in {} files", count, files)
    }
}

/// References peek widget
pub struct ReferencesPeekWidget {
    /// References
    references: Vec<Reference>,
    /// Selected index
    selected: usize,
    /// Preview content cache
    previews: HashMap<ReferenceLocation, Vec<String>>,
}

impl ReferencesPeekWidget {
    pub fn new(references: Vec<Reference>) -> Self {
        Self {
            references,
            selected: 0,
            previews: HashMap::new(),
        }
    }

    pub fn selected(&self) -> Option<&Reference> {
        self.references.get(self.selected)
    }

    pub fn select(&mut self, index: usize) {
        if index < self.references.len() {
            self.selected = index;
        }
    }

    pub fn next(&mut self) {
        if !self.references.is_empty() {
            self.selected = (self.selected + 1) % self.references.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.references.is_empty() {
            self.selected = if self.selected == 0 {
                self.references.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn set_preview(&mut self, location: ReferenceLocation, lines: Vec<String>) {
        self.previews.insert(location, lines);
    }

    pub fn get_preview(&self, location: &ReferenceLocation) -> Option<&Vec<String>> {
        self.previews.get(location)
    }
}
