//! # Foxkit Folding Range
//!
//! Code folding regions for collapsible sections.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Folding range service
pub struct FoldingRangeService {
    /// Folding state per file
    state: RwLock<HashMap<PathBuf, FoldingState>>,
    /// Events
    events: broadcast::Sender<FoldingEvent>,
    /// Configuration
    config: RwLock<FoldingConfig>,
}

impl FoldingRangeService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            state: RwLock::new(HashMap::new()),
            events,
            config: RwLock::new(FoldingConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<FoldingEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: FoldingConfig) {
        *self.config.write() = config;
    }

    /// Get folding ranges for file (would call LSP)
    pub async fn get_folding_ranges(&self, file: &PathBuf) -> Vec<FoldingRange> {
        // Would call LSP textDocument/foldingRange
        Vec::new()
    }

    /// Set folding ranges for file
    pub fn set_ranges(&self, file: PathBuf, ranges: Vec<FoldingRange>) {
        let state = FoldingState {
            ranges: ranges.clone(),
            folded: std::collections::HashSet::new(),
        };
        
        self.state.write().insert(file.clone(), state);
        
        let _ = self.events.send(FoldingEvent::RangesUpdated {
            file,
            count: ranges.len(),
        });
    }

    /// Fold a range
    pub fn fold(&self, file: &PathBuf, line: u32) {
        let mut state = self.state.write();
        
        if let Some(file_state) = state.get_mut(file) {
            // Find range starting at line
            for range in &file_state.ranges {
                if range.start_line == line {
                    file_state.folded.insert(line);
                    
                    let _ = self.events.send(FoldingEvent::Folded {
                        file: file.clone(),
                        range: range.clone(),
                    });
                    break;
                }
            }
        }
    }

    /// Unfold a range
    pub fn unfold(&self, file: &PathBuf, line: u32) {
        let mut state = self.state.write();
        
        if let Some(file_state) = state.get_mut(file) {
            if file_state.folded.remove(&line) {
                let range = file_state.ranges.iter()
                    .find(|r| r.start_line == line)
                    .cloned();
                    
                if let Some(range) = range {
                    let _ = self.events.send(FoldingEvent::Unfolded {
                        file: file.clone(),
                        range,
                    });
                }
            }
        }
    }

    /// Toggle fold at line
    pub fn toggle(&self, file: &PathBuf, line: u32) {
        let is_folded = {
            let state = self.state.read();
            state.get(file)
                .map(|s| s.folded.contains(&line))
                .unwrap_or(false)
        };

        if is_folded {
            self.unfold(file, line);
        } else {
            self.fold(file, line);
        }
    }

    /// Fold all regions
    pub fn fold_all(&self, file: &PathBuf) {
        let mut state = self.state.write();
        
        if let Some(file_state) = state.get_mut(file) {
            for range in &file_state.ranges {
                file_state.folded.insert(range.start_line);
            }
            
            let _ = self.events.send(FoldingEvent::FoldedAll {
                file: file.clone(),
            });
        }
    }

    /// Unfold all regions
    pub fn unfold_all(&self, file: &PathBuf) {
        let mut state = self.state.write();
        
        if let Some(file_state) = state.get_mut(file) {
            file_state.folded.clear();
            
            let _ = self.events.send(FoldingEvent::UnfoldedAll {
                file: file.clone(),
            });
        }
    }

    /// Fold to level
    pub fn fold_to_level(&self, file: &PathBuf, level: u32) {
        let mut state = self.state.write();
        
        if let Some(file_state) = state.get_mut(file) {
            file_state.folded.clear();
            
            // Build level map
            let levels = calculate_fold_levels(&file_state.ranges);
            
            for (range_idx, range_level) in levels {
                if range_level <= level {
                    file_state.folded.insert(file_state.ranges[range_idx].start_line);
                }
            }
            
            let _ = self.events.send(FoldingEvent::FoldedToLevel {
                file: file.clone(),
                level,
            });
        }
    }

    /// Check if line is folded
    pub fn is_folded(&self, file: &PathBuf, line: u32) -> bool {
        self.state.read()
            .get(file)
            .map(|s| s.folded.contains(&line))
            .unwrap_or(false)
    }

    /// Get all folded ranges
    pub fn get_folded(&self, file: &PathBuf) -> Vec<FoldingRange> {
        let state = self.state.read();
        
        state.get(file)
            .map(|s| {
                s.ranges.iter()
                    .filter(|r| s.folded.contains(&r.start_line))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get visible lines (accounting for folds)
    pub fn visible_lines(&self, file: &PathBuf, total_lines: u32) -> Vec<u32> {
        let state = self.state.read();
        
        let mut visible = Vec::new();
        let mut skip_until: Option<u32> = None;
        
        if let Some(file_state) = state.get(file) {
            for line in 0..total_lines {
                if let Some(end) = skip_until {
                    if line <= end {
                        continue;
                    }
                    skip_until = None;
                }
                
                visible.push(line);
                
                // Check if this line starts a folded range
                if file_state.folded.contains(&line) {
                    if let Some(range) = file_state.ranges.iter().find(|r| r.start_line == line) {
                        skip_until = Some(range.end_line);
                    }
                }
            }
        } else {
            visible = (0..total_lines).collect();
        }
        
        visible
    }
}

impl Default for FoldingRangeService {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate fold levels
fn calculate_fold_levels(ranges: &[FoldingRange]) -> Vec<(usize, u32)> {
    let mut levels = Vec::new();
    
    for (idx, range) in ranges.iter().enumerate() {
        let mut level = 0;
        
        for other in ranges {
            if other.start_line < range.start_line && other.end_line > range.end_line {
                level += 1;
            }
        }
        
        levels.push((idx, level));
    }
    
    levels
}

/// Folding state for a file
struct FoldingState {
    /// Available ranges
    ranges: Vec<FoldingRange>,
    /// Folded lines (start line of folded range)
    folded: std::collections::HashSet<u32>,
}

/// Folding range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldingRange {
    /// Start line (0-indexed)
    pub start_line: u32,
    /// Start character (optional)
    pub start_col: Option<u32>,
    /// End line (0-indexed)
    pub end_line: u32,
    /// End character (optional)
    pub end_col: Option<u32>,
    /// Folding kind
    pub kind: FoldingRangeKind,
    /// Collapsed text (what to show when folded)
    pub collapsed_text: Option<String>,
}

impl FoldingRange {
    pub fn new(start_line: u32, end_line: u32, kind: FoldingRangeKind) -> Self {
        Self {
            start_line,
            start_col: None,
            end_line,
            end_col: None,
            kind,
            collapsed_text: None,
        }
    }

    pub fn comment(start_line: u32, end_line: u32) -> Self {
        Self::new(start_line, end_line, FoldingRangeKind::Comment)
    }

    pub fn imports(start_line: u32, end_line: u32) -> Self {
        Self::new(start_line, end_line, FoldingRangeKind::Imports)
    }

    pub fn region(start_line: u32, end_line: u32) -> Self {
        Self::new(start_line, end_line, FoldingRangeKind::Region)
    }

    pub fn with_collapsed_text(mut self, text: impl Into<String>) -> Self {
        self.collapsed_text = Some(text.into());
        self
    }

    /// Line count when folded
    pub fn hidden_lines(&self) -> u32 {
        self.end_line.saturating_sub(self.start_line)
    }
}

/// Folding range kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FoldingRangeKind {
    /// Comment block
    Comment,
    /// Import statements
    Imports,
    /// Region markers (#region)
    Region,
}

impl FoldingRangeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Comment => "comment",
            Self::Imports => "imports",
            Self::Region => "region",
        }
    }

    pub fn default_collapsed_text(&self) -> &'static str {
        match self {
            Self::Comment => "/* ... */",
            Self::Imports => "...",
            Self::Region => "...",
        }
    }
}

/// Folding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldingConfig {
    /// Enable folding
    pub enabled: bool,
    /// Show fold controls
    pub show_fold_controls: FoldControlVisibility,
    /// Maximum fold regions
    pub max_regions: usize,
    /// Fold comments by default
    pub fold_comments: bool,
    /// Fold imports by default
    pub fold_imports: bool,
    /// Default collapsed text
    pub default_collapsed_text: String,
}

impl Default for FoldingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_fold_controls: FoldControlVisibility::MouseOver,
            max_regions: 5000,
            fold_comments: false,
            fold_imports: false,
            default_collapsed_text: "...".to_string(),
        }
    }
}

/// Fold control visibility
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FoldControlVisibility {
    /// Always show
    Always,
    /// Only on hover
    MouseOver,
    /// Never show
    Never,
}

/// Folding event
#[derive(Debug, Clone)]
pub enum FoldingEvent {
    RangesUpdated { file: PathBuf, count: usize },
    Folded { file: PathBuf, range: FoldingRange },
    Unfolded { file: PathBuf, range: FoldingRange },
    FoldedAll { file: PathBuf },
    UnfoldedAll { file: PathBuf },
    FoldedToLevel { file: PathBuf, level: u32 },
}

/// Fold indicator for gutter
pub struct FoldIndicator {
    pub line: u32,
    pub kind: FoldIndicatorKind,
}

#[derive(Debug, Clone, Copy)]
pub enum FoldIndicatorKind {
    /// Collapsible (not folded)
    Collapsible,
    /// Collapsed (folded)
    Collapsed,
    /// End of fold range
    End,
}

impl FoldIndicator {
    pub fn icon(&self) -> &'static str {
        match self.kind {
            FoldIndicatorKind::Collapsible => "▼",
            FoldIndicatorKind::Collapsed => "▶",
            FoldIndicatorKind::End => "",
        }
    }
}

/// Build fold indicators for gutter
pub fn build_fold_indicators(
    ranges: &[FoldingRange],
    folded: &std::collections::HashSet<u32>,
) -> Vec<FoldIndicator> {
    let mut indicators = Vec::new();

    for range in ranges {
        let kind = if folded.contains(&range.start_line) {
            FoldIndicatorKind::Collapsed
        } else {
            FoldIndicatorKind::Collapsible
        };

        indicators.push(FoldIndicator {
            line: range.start_line,
            kind,
        });
    }

    indicators
}
