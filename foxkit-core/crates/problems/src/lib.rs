//! # Foxkit Problems Panel
//!
//! Problems/diagnostics panel view.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Problems panel service
pub struct ProblemsService {
    /// Diagnostics grouped by file
    diagnostics: RwLock<HashMap<PathBuf, Vec<ProblemItem>>>,
    /// Current filter
    filter: RwLock<ProblemsFilter>,
    /// Events
    events: broadcast::Sender<ProblemsEvent>,
    /// Statistics
    stats: RwLock<ProblemsStats>,
}

impl ProblemsService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            diagnostics: RwLock::new(HashMap::new()),
            filter: RwLock::new(ProblemsFilter::default()),
            events,
            stats: RwLock::new(ProblemsStats::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<ProblemsEvent> {
        self.events.subscribe()
    }

    /// Set diagnostics for a file
    pub fn set_diagnostics(&self, file: PathBuf, items: Vec<ProblemItem>) {
        self.diagnostics.write().insert(file.clone(), items);
        self.update_stats();

        let _ = self.events.send(ProblemsEvent::DiagnosticsChanged { file });
    }

    /// Clear diagnostics for a file
    pub fn clear_diagnostics(&self, file: &PathBuf) {
        self.diagnostics.write().remove(file);
        self.update_stats();

        let _ = self.events.send(ProblemsEvent::DiagnosticsChanged { file: file.clone() });
    }

    /// Clear all diagnostics
    pub fn clear_all(&self) {
        self.diagnostics.write().clear();
        self.update_stats();

        let _ = self.events.send(ProblemsEvent::AllCleared);
    }

    /// Set filter
    pub fn set_filter(&self, filter: ProblemsFilter) {
        *self.filter.write() = filter;
        let _ = self.events.send(ProblemsEvent::FilterChanged);
    }

    /// Get current filter
    pub fn filter(&self) -> ProblemsFilter {
        self.filter.read().clone()
    }

    /// Get statistics
    pub fn stats(&self) -> ProblemsStats {
        self.stats.read().clone()
    }

    /// Get all problems (filtered)
    pub fn get_problems(&self) -> Vec<ProblemGroup> {
        let filter = self.filter.read().clone();
        let diagnostics = self.diagnostics.read();

        let mut groups: Vec<ProblemGroup> = diagnostics
            .iter()
            .filter_map(|(file, items)| {
                let filtered: Vec<_> = items
                    .iter()
                    .filter(|item| filter.matches(item))
                    .cloned()
                    .collect();

                if filtered.is_empty() {
                    None
                } else {
                    Some(ProblemGroup {
                        file: file.clone(),
                        items: filtered,
                    })
                }
            })
            .collect();

        // Sort groups by file path
        groups.sort_by(|a, b| a.file.cmp(&b.file));

        groups
    }

    /// Get problems for a specific file
    pub fn get_file_problems(&self, file: &PathBuf) -> Vec<ProblemItem> {
        let filter = self.filter.read().clone();
        
        self.diagnostics.read()
            .get(file)
            .map(|items| {
                items.iter()
                    .filter(|item| filter.matches(item))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Update statistics
    fn update_stats(&self) {
        let diagnostics = self.diagnostics.read();
        let mut stats = ProblemsStats::default();

        for items in diagnostics.values() {
            stats.files += 1;
            for item in items {
                match item.severity {
                    ProblemSeverity::Error => stats.errors += 1,
                    ProblemSeverity::Warning => stats.warnings += 1,
                    ProblemSeverity::Information => stats.info += 1,
                    ProblemSeverity::Hint => stats.hints += 1,
                }
            }
        }

        *self.stats.write() = stats;
        let _ = self.events.send(ProblemsEvent::StatsChanged);
    }
}

impl Default for ProblemsService {
    fn default() -> Self {
        Self::new()
    }
}

/// Problem item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemItem {
    /// Severity
    pub severity: ProblemSeverity,
    /// Message
    pub message: String,
    /// Source (e.g., "rustc", "eslint")
    pub source: Option<String>,
    /// Code (e.g., "E0001")
    pub code: Option<ProblemCode>,
    /// File location
    pub location: ProblemLocation,
    /// Related information
    pub related: Vec<RelatedInformation>,
    /// Tags
    pub tags: Vec<ProblemTag>,
}

impl ProblemItem {
    pub fn error(message: impl Into<String>, location: ProblemLocation) -> Self {
        Self {
            severity: ProblemSeverity::Error,
            message: message.into(),
            source: None,
            code: None,
            location,
            related: Vec::new(),
            tags: Vec::new(),
        }
    }

    pub fn warning(message: impl Into<String>, location: ProblemLocation) -> Self {
        Self {
            severity: ProblemSeverity::Warning,
            message: message.into(),
            source: None,
            code: None,
            location,
            related: Vec::new(),
            tags: Vec::new(),
        }
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(ProblemCode::String(code.into()));
        self
    }
}

/// Problem severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProblemSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

impl ProblemSeverity {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Error => "$(error)",
            Self::Warning => "$(warning)",
            Self::Information => "$(info)",
            Self::Hint => "$(light-bulb)",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Information => "info",
            Self::Hint => "hint",
        }
    }
}

/// Problem code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProblemCode {
    String(String),
    Number(i64),
    WithLink { value: String, target: String },
}

/// Problem location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemLocation {
    pub line: u32,
    pub column: u32,
    pub end_line: Option<u32>,
    pub end_column: Option<u32>,
}

impl ProblemLocation {
    pub fn new(line: u32, column: u32) -> Self {
        Self {
            line,
            column,
            end_line: None,
            end_column: None,
        }
    }

    pub fn with_end(mut self, end_line: u32, end_column: u32) -> Self {
        self.end_line = Some(end_line);
        self.end_column = Some(end_column);
        self
    }
}

/// Related information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedInformation {
    pub file: PathBuf,
    pub location: ProblemLocation,
    pub message: String,
}

/// Problem tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProblemTag {
    Unnecessary,
    Deprecated,
}

/// Problem group (by file)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemGroup {
    pub file: PathBuf,
    pub items: Vec<ProblemItem>,
}

impl ProblemGroup {
    pub fn error_count(&self) -> usize {
        self.items.iter().filter(|i| i.severity == ProblemSeverity::Error).count()
    }

    pub fn warning_count(&self) -> usize {
        self.items.iter().filter(|i| i.severity == ProblemSeverity::Warning).count()
    }
}

/// Problems filter
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProblemsFilter {
    /// Show errors
    pub show_errors: bool,
    /// Show warnings
    pub show_warnings: bool,
    /// Show info
    pub show_info: bool,
    /// Show hints
    pub show_hints: bool,
    /// Text filter
    pub text: Option<String>,
    /// Source filter
    pub source: Option<String>,
    /// Exclude patterns
    pub exclude_patterns: Vec<String>,
}

impl ProblemsFilter {
    pub fn all() -> Self {
        Self {
            show_errors: true,
            show_warnings: true,
            show_info: true,
            show_hints: true,
            text: None,
            source: None,
            exclude_patterns: Vec::new(),
        }
    }

    pub fn errors_only() -> Self {
        Self {
            show_errors: true,
            show_warnings: false,
            show_info: false,
            show_hints: false,
            text: None,
            source: None,
            exclude_patterns: Vec::new(),
        }
    }

    pub fn matches(&self, item: &ProblemItem) -> bool {
        // Check severity
        match item.severity {
            ProblemSeverity::Error if !self.show_errors => return false,
            ProblemSeverity::Warning if !self.show_warnings => return false,
            ProblemSeverity::Information if !self.show_info => return false,
            ProblemSeverity::Hint if !self.show_hints => return false,
            _ => {}
        }

        // Check text filter
        if let Some(ref text) = self.text {
            let text_lower = text.to_lowercase();
            if !item.message.to_lowercase().contains(&text_lower) {
                return false;
            }
        }

        // Check source filter
        if let Some(ref source) = self.source {
            if item.source.as_ref() != Some(source) {
                return false;
            }
        }

        true
    }
}

/// Problems statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProblemsStats {
    pub files: usize,
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    pub hints: usize,
}

impl ProblemsStats {
    pub fn total(&self) -> usize {
        self.errors + self.warnings + self.info + self.hints
    }

    pub fn summary(&self) -> String {
        format!("{} errors, {} warnings", self.errors, self.warnings)
    }
}

/// Problems event
#[derive(Debug, Clone)]
pub enum ProblemsEvent {
    DiagnosticsChanged { file: PathBuf },
    AllCleared,
    FilterChanged,
    StatsChanged,
}

/// Problems view model
pub struct ProblemsViewModel {
    service: Arc<ProblemsService>,
    /// Expanded groups
    expanded: RwLock<HashMap<PathBuf, bool>>,
    /// Selected item
    selected: RwLock<Option<(PathBuf, usize)>>,
}

impl ProblemsViewModel {
    pub fn new(service: Arc<ProblemsService>) -> Self {
        Self {
            service,
            expanded: RwLock::new(HashMap::new()),
            selected: RwLock::new(None),
        }
    }

    pub fn groups(&self) -> Vec<ProblemGroup> {
        self.service.get_problems()
    }

    pub fn stats(&self) -> ProblemsStats {
        self.service.stats()
    }

    pub fn is_expanded(&self, file: &PathBuf) -> bool {
        *self.expanded.read().get(file).unwrap_or(&true)
    }

    pub fn toggle_expanded(&self, file: &PathBuf) {
        let mut expanded = self.expanded.write();
        let current = *expanded.get(file).unwrap_or(&true);
        expanded.insert(file.clone(), !current);
    }

    pub fn select(&self, file: PathBuf, index: usize) {
        *self.selected.write() = Some((file, index));
    }

    pub fn selected(&self) -> Option<(PathBuf, usize)> {
        self.selected.read().clone()
    }

    pub fn selected_item(&self) -> Option<ProblemItem> {
        let selected = self.selected.read().clone()?;
        let problems = self.service.get_file_problems(&selected.0);
        problems.get(selected.1).cloned()
    }

    pub fn select_next(&self) {
        // Navigate to next problem
    }

    pub fn select_previous(&self) {
        // Navigate to previous problem
    }
}
