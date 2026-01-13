//! # Foxkit Gutter
//!
//! Line number gutter and indicators.

use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Gutter service
pub struct GutterService {
    /// Gutter decorations by file
    decorations: RwLock<HashMap<PathBuf, Vec<GutterDecoration>>>,
    /// Configuration
    config: RwLock<GutterConfig>,
}

impl GutterService {
    pub fn new() -> Self {
        Self {
            decorations: RwLock::new(HashMap::new()),
            config: RwLock::new(GutterConfig::default()),
        }
    }

    /// Configure gutter
    pub fn configure(&self, config: GutterConfig) {
        *self.config.write() = config;
    }

    /// Get configuration
    pub fn config(&self) -> GutterConfig {
        self.config.read().clone()
    }

    /// Set decorations for file
    pub fn set_decorations(&self, file: PathBuf, decorations: Vec<GutterDecoration>) {
        self.decorations.write().insert(file, decorations);
    }

    /// Add decoration
    pub fn add_decoration(&self, file: &PathBuf, decoration: GutterDecoration) {
        self.decorations
            .write()
            .entry(file.clone())
            .or_default()
            .push(decoration);
    }

    /// Remove decorations by source
    pub fn remove_by_source(&self, file: &PathBuf, source: &str) {
        if let Some(decs) = self.decorations.write().get_mut(file) {
            decs.retain(|d| d.source != source);
        }
    }

    /// Get decorations for file
    pub fn get_decorations(&self, file: &PathBuf) -> Vec<GutterDecoration> {
        self.decorations
            .read()
            .get(file)
            .cloned()
            .unwrap_or_default()
    }

    /// Get decoration at line
    pub fn get_decoration_at_line(&self, file: &PathBuf, line: u32) -> Option<GutterDecoration> {
        self.decorations
            .read()
            .get(file)
            .and_then(|decs| decs.iter().find(|d| d.line == line).cloned())
    }

    /// Get all decorations at line (sorted by priority)
    pub fn get_decorations_at_line(&self, file: &PathBuf, line: u32) -> Vec<GutterDecoration> {
        let mut decs: Vec<_> = self.decorations
            .read()
            .get(file)
            .map(|all| all.iter().filter(|d| d.line == line).cloned().collect())
            .unwrap_or_default();
        
        decs.sort_by(|a, b| b.priority.cmp(&a.priority));
        decs
    }

    /// Clear decorations for file
    pub fn clear_file(&self, file: &PathBuf) {
        self.decorations.write().remove(file);
    }

    /// Clear all decorations
    pub fn clear_all(&self) {
        self.decorations.write().clear();
    }

    /// Format line number
    pub fn format_line_number(&self, line: u32, current_line: u32, total_lines: u32) -> String {
        let config = self.config.read();
        let display_line = line + 1; // Convert to 1-based

        match config.line_numbers {
            LineNumberMode::Off => String::new(),
            LineNumberMode::On => {
                let width = total_lines.to_string().len();
                format!("{:>width$}", display_line, width = width)
            }
            LineNumberMode::Relative => {
                if line == current_line {
                    let width = total_lines.to_string().len();
                    format!("{:>width$}", display_line, width = width)
                } else {
                    let relative = (line as i64 - current_line as i64).unsigned_abs();
                    let width = total_lines.to_string().len();
                    format!("{:>width$}", relative, width = width)
                }
            }
            LineNumberMode::Interval(n) => {
                if display_line % n == 0 || line == current_line {
                    let width = total_lines.to_string().len();
                    format!("{:>width$}", display_line, width = width)
                } else {
                    String::new()
                }
            }
        }
    }

    /// Calculate gutter width
    pub fn calculate_width(&self, total_lines: u32) -> u32 {
        let config = self.config.read();
        let mut width = 0;

        // Line numbers width
        if config.line_numbers != LineNumberMode::Off {
            width += total_lines.to_string().len() as u32 + 2; // +2 for padding
        }

        // Folding width
        if config.folding {
            width += 2;
        }

        // Glyph margin
        if config.glyph_margin {
            width += 2;
        }

        width.max(4) // Minimum width
    }

    /// Build gutter content for line
    pub fn build_gutter_line(
        &self,
        file: &PathBuf,
        line: u32,
        current_line: u32,
        total_lines: u32,
        is_folded: bool,
        can_fold: bool,
    ) -> GutterLine {
        let config = self.config.read();
        let decorations = self.get_decorations_at_line(file, line);

        GutterLine {
            line_number: if config.line_numbers != LineNumberMode::Off {
                Some(self.format_line_number(line, current_line, total_lines))
            } else {
                None
            },
            fold_indicator: if config.folding && can_fold {
                Some(if is_folded {
                    FoldIndicator::Collapsed
                } else {
                    FoldIndicator::Expanded
                })
            } else {
                None
            },
            glyph: decorations.first().and_then(|d| d.glyph.clone()),
            background: decorations.first().and_then(|d| d.background.clone()),
            is_current: line == current_line,
        }
    }
}

impl Default for GutterService {
    fn default() -> Self {
        Self::new()
    }
}

/// Gutter decoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GutterDecoration {
    /// Line number
    pub line: u32,
    /// Glyph/icon
    pub glyph: Option<GutterGlyph>,
    /// Background color
    pub background: Option<String>,
    /// Priority (higher = more important)
    pub priority: u32,
    /// Source identifier
    pub source: String,
    /// Tooltip
    pub tooltip: Option<String>,
}

impl GutterDecoration {
    pub fn new(line: u32, source: impl Into<String>) -> Self {
        Self {
            line,
            glyph: None,
            background: None,
            priority: 0,
            source: source.into(),
            tooltip: None,
        }
    }

    pub fn with_glyph(mut self, glyph: GutterGlyph) -> Self {
        self.glyph = Some(glyph);
        self
    }

    pub fn with_background(mut self, color: impl Into<String>) -> Self {
        self.background = Some(color.into());
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// Gutter glyph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GutterGlyph {
    /// Icon by ID
    Icon(String),
    /// Text character
    Text(String),
    /// Color circle
    Circle(String),
    /// Custom SVG
    Svg(String),
}

impl GutterGlyph {
    pub fn breakpoint() -> Self {
        Self::Circle("#e51400".to_string())
    }

    pub fn breakpoint_disabled() -> Self {
        Self::Circle("#848484".to_string())
    }

    pub fn breakpoint_conditional() -> Self {
        Self::Icon("debug-breakpoint-conditional".to_string())
    }

    pub fn bookmark() -> Self {
        Self::Icon("bookmark".to_string())
    }

    pub fn error() -> Self {
        Self::Icon("error".to_string())
    }

    pub fn warning() -> Self {
        Self::Icon("warning".to_string())
    }

    pub fn info() -> Self {
        Self::Icon("info".to_string())
    }
}

/// Fold indicator
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FoldIndicator {
    Expanded,
    Collapsed,
}

impl FoldIndicator {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Expanded => "▼",
            Self::Collapsed => "▶",
        }
    }
}

/// Gutter line content
#[derive(Debug, Clone)]
pub struct GutterLine {
    /// Line number text
    pub line_number: Option<String>,
    /// Fold indicator
    pub fold_indicator: Option<FoldIndicator>,
    /// Glyph
    pub glyph: Option<GutterGlyph>,
    /// Background color
    pub background: Option<String>,
    /// Is current line
    pub is_current: bool,
}

/// Gutter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GutterConfig {
    /// Line numbers mode
    pub line_numbers: LineNumberMode,
    /// Show folding controls
    pub folding: bool,
    /// Show glyph margin
    pub glyph_margin: bool,
    /// Highlight current line number
    pub highlight_active: bool,
    /// Render final newline
    pub render_final_newline: bool,
}

impl Default for GutterConfig {
    fn default() -> Self {
        Self {
            line_numbers: LineNumberMode::On,
            folding: true,
            glyph_margin: true,
            highlight_active: true,
            render_final_newline: true,
        }
    }
}

/// Line number mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineNumberMode {
    /// No line numbers
    Off,
    /// Absolute line numbers
    On,
    /// Relative to cursor
    Relative,
    /// Show every N lines
    Interval(u32),
}
