//! # Foxkit Minimap Decorations
//!
//! Highlights, markers, and annotations on the minimap.

use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Minimap decorations service
pub struct MinimapDecorationsService {
    /// Decorations by file
    decorations: RwLock<HashMap<PathBuf, Vec<MinimapDecoration>>>,
    /// Configuration
    config: RwLock<MinimapDecorationsConfig>,
}

impl MinimapDecorationsService {
    pub fn new() -> Self {
        Self {
            decorations: RwLock::new(HashMap::new()),
            config: RwLock::new(MinimapDecorationsConfig::default()),
        }
    }

    /// Configure service
    pub fn configure(&self, config: MinimapDecorationsConfig) {
        *self.config.write() = config;
    }

    /// Get configuration
    pub fn config(&self) -> MinimapDecorationsConfig {
        self.config.read().clone()
    }

    /// Set decorations for file
    pub fn set_decorations(&self, file: PathBuf, decorations: Vec<MinimapDecoration>) {
        self.decorations.write().insert(file, decorations);
    }

    /// Add decoration
    pub fn add_decoration(&self, file: &PathBuf, decoration: MinimapDecoration) {
        self.decorations
            .write()
            .entry(file.clone())
            .or_default()
            .push(decoration);
    }

    /// Remove decorations by source
    pub fn remove_by_source(&self, file: &PathBuf, source: &str) {
        if let Some(decorations) = self.decorations.write().get_mut(file) {
            decorations.retain(|d| d.source != source);
        }
    }

    /// Get decorations for file
    pub fn get_decorations(&self, file: &PathBuf) -> Vec<MinimapDecoration> {
        self.decorations
            .read()
            .get(file)
            .cloned()
            .unwrap_or_default()
    }

    /// Get decorations grouped by lane
    pub fn get_by_lane(&self, file: &PathBuf) -> HashMap<DecorationLane, Vec<MinimapDecoration>> {
        let decorations = self.get_decorations(file);
        let mut by_lane: HashMap<DecorationLane, Vec<MinimapDecoration>> = HashMap::new();

        for dec in decorations {
            by_lane.entry(dec.lane).or_default().push(dec);
        }

        by_lane
    }

    /// Clear decorations for file
    pub fn clear(&self, file: &PathBuf) {
        self.decorations.write().remove(file);
    }

    /// Clear all decorations
    pub fn clear_all(&self) {
        self.decorations.write().clear();
    }

    /// Add error markers
    pub fn add_error_markers(&self, file: &PathBuf, lines: Vec<u32>) {
        let config = self.config.read();
        
        if !config.show_errors {
            return;
        }

        for line in lines {
            self.add_decoration(file, MinimapDecoration {
                line,
                end_line: None,
                color: config.error_color.clone(),
                lane: DecorationLane::Right,
                kind: DecorationKind::Line,
                source: "diagnostics".to_string(),
                tooltip: Some("Error".to_string()),
            });
        }
    }

    /// Add warning markers
    pub fn add_warning_markers(&self, file: &PathBuf, lines: Vec<u32>) {
        let config = self.config.read();
        
        if !config.show_warnings {
            return;
        }

        for line in lines {
            self.add_decoration(file, MinimapDecoration {
                line,
                end_line: None,
                color: config.warning_color.clone(),
                lane: DecorationLane::Right,
                kind: DecorationKind::Line,
                source: "diagnostics".to_string(),
                tooltip: Some("Warning".to_string()),
            });
        }
    }

    /// Add search highlights
    pub fn add_search_highlights(&self, file: &PathBuf, lines: Vec<u32>) {
        let config = self.config.read();
        
        if !config.show_search {
            return;
        }

        for line in lines {
            self.add_decoration(file, MinimapDecoration {
                line,
                end_line: None,
                color: config.search_color.clone(),
                lane: DecorationLane::Center,
                kind: DecorationKind::Line,
                source: "search".to_string(),
                tooltip: None,
            });
        }
    }

    /// Add git change markers
    pub fn add_git_changes(&self, file: &PathBuf, changes: Vec<GitChange>) {
        let config = self.config.read();
        
        if !config.show_git_changes {
            return;
        }

        for change in changes {
            let color = match change.kind {
                GitChangeKind::Added => config.git_added_color.clone(),
                GitChangeKind::Modified => config.git_modified_color.clone(),
                GitChangeKind::Deleted => config.git_deleted_color.clone(),
            };

            self.add_decoration(file, MinimapDecoration {
                line: change.start_line,
                end_line: Some(change.end_line),
                color,
                lane: DecorationLane::Left,
                kind: DecorationKind::Range,
                source: "git".to_string(),
                tooltip: Some(format!("{:?}", change.kind)),
            });
        }
    }

    /// Add selection highlight
    pub fn add_selection(&self, file: &PathBuf, start_line: u32, end_line: u32) {
        let config = self.config.read();
        
        self.remove_by_source(file, "selection");

        self.add_decoration(file, MinimapDecoration {
            line: start_line,
            end_line: Some(end_line),
            color: config.selection_color.clone(),
            lane: DecorationLane::Full,
            kind: DecorationKind::Range,
            source: "selection".to_string(),
            tooltip: None,
        });
    }
}

impl Default for MinimapDecorationsService {
    fn default() -> Self {
        Self::new()
    }
}

/// Minimap decoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimapDecoration {
    /// Start line
    pub line: u32,
    /// End line (for ranges)
    pub end_line: Option<u32>,
    /// Color
    pub color: String,
    /// Lane
    pub lane: DecorationLane,
    /// Kind
    pub kind: DecorationKind,
    /// Source identifier
    pub source: String,
    /// Tooltip
    pub tooltip: Option<String>,
}

impl MinimapDecoration {
    pub fn line(line: u32, color: impl Into<String>) -> Self {
        Self {
            line,
            end_line: None,
            color: color.into(),
            lane: DecorationLane::Right,
            kind: DecorationKind::Line,
            source: String::new(),
            tooltip: None,
        }
    }

    pub fn range(start: u32, end: u32, color: impl Into<String>) -> Self {
        Self {
            line: start,
            end_line: Some(end),
            color: color.into(),
            lane: DecorationLane::Right,
            kind: DecorationKind::Range,
            source: String::new(),
            tooltip: None,
        }
    }

    pub fn with_lane(mut self, lane: DecorationLane) -> Self {
        self.lane = lane;
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Get line range
    pub fn line_range(&self) -> (u32, u32) {
        (self.line, self.end_line.unwrap_or(self.line))
    }
}

/// Decoration lane
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DecorationLane {
    /// Left lane (e.g., git changes)
    Left,
    /// Center lane
    Center,
    /// Right lane (e.g., errors)
    Right,
    /// Full width
    Full,
}

/// Decoration kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DecorationKind {
    /// Single line marker
    Line,
    /// Range highlight
    Range,
    /// Gutter marker
    Gutter,
}

/// Git change
#[derive(Debug, Clone)]
pub struct GitChange {
    pub start_line: u32,
    pub end_line: u32,
    pub kind: GitChangeKind,
}

/// Git change kind
#[derive(Debug, Clone, Copy)]
pub enum GitChangeKind {
    Added,
    Modified,
    Deleted,
}

/// Minimap decorations configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimapDecorationsConfig {
    /// Show error markers
    pub show_errors: bool,
    /// Show warning markers
    pub show_warnings: bool,
    /// Show search highlights
    pub show_search: bool,
    /// Show git changes
    pub show_git_changes: bool,
    /// Error color
    pub error_color: String,
    /// Warning color
    pub warning_color: String,
    /// Search highlight color
    pub search_color: String,
    /// Selection color
    pub selection_color: String,
    /// Git added color
    pub git_added_color: String,
    /// Git modified color
    pub git_modified_color: String,
    /// Git deleted color
    pub git_deleted_color: String,
}

impl Default for MinimapDecorationsConfig {
    fn default() -> Self {
        Self {
            show_errors: true,
            show_warnings: true,
            show_search: true,
            show_git_changes: true,
            error_color: "#ff0000".to_string(),
            warning_color: "#ffcc00".to_string(),
            search_color: "#515c6a".to_string(),
            selection_color: "#264f78".to_string(),
            git_added_color: "#587c0c".to_string(),
            git_modified_color: "#0c7d9d".to_string(),
            git_deleted_color: "#94151b".to_string(),
        }
    }
}
