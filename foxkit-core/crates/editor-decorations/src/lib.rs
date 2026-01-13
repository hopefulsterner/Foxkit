//! # Foxkit Editor Decorations
//!
//! Text decorations and annotations in the editor.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

static DECORATION_ID: AtomicU64 = AtomicU64::new(1);

/// Editor decorations service
pub struct EditorDecorationsService {
    /// Decoration types
    types: RwLock<HashMap<String, DecorationRenderOptions>>,
    /// Decorations by file
    decorations: RwLock<HashMap<PathBuf, Vec<Decoration>>>,
}

impl EditorDecorationsService {
    pub fn new() -> Self {
        Self {
            types: RwLock::new(HashMap::new()),
            decorations: RwLock::new(HashMap::new()),
        }
    }

    /// Register decoration type
    pub fn register_type(&self, id: impl Into<String>, options: DecorationRenderOptions) {
        self.types.write().insert(id.into(), options);
    }

    /// Get decoration type
    pub fn get_type(&self, id: &str) -> Option<DecorationRenderOptions> {
        self.types.read().get(id).cloned()
    }

    /// Set decorations for file
    pub fn set_decorations(
        &self,
        file: PathBuf,
        type_id: &str,
        ranges: Vec<DecorationRange>,
    ) -> Vec<DecorationId> {
        let mut ids = Vec::new();
        let mut decorations = Vec::new();

        for range in ranges {
            let id = DecorationId(DECORATION_ID.fetch_add(1, Ordering::Relaxed));
            ids.push(id.clone());
            
            decorations.push(Decoration {
                id,
                type_id: type_id.to_string(),
                range,
            });
        }

        // Remove existing decorations of this type
        let file_decorations = self.decorations
            .write()
            .entry(file)
            .or_default();
        
        file_decorations.retain(|d| d.type_id != type_id);
        file_decorations.extend(decorations);

        ids
    }

    /// Add single decoration
    pub fn add_decoration(
        &self,
        file: PathBuf,
        type_id: &str,
        range: DecorationRange,
    ) -> DecorationId {
        let id = DecorationId(DECORATION_ID.fetch_add(1, Ordering::Relaxed));
        
        let decoration = Decoration {
            id: id.clone(),
            type_id: type_id.to_string(),
            range,
        };

        self.decorations
            .write()
            .entry(file)
            .or_default()
            .push(decoration);

        id
    }

    /// Remove decoration by ID
    pub fn remove_decoration(&self, file: &PathBuf, id: &DecorationId) {
        if let Some(decorations) = self.decorations.write().get_mut(file) {
            decorations.retain(|d| &d.id != id);
        }
    }

    /// Remove decorations by type
    pub fn remove_by_type(&self, file: &PathBuf, type_id: &str) {
        if let Some(decorations) = self.decorations.write().get_mut(file) {
            decorations.retain(|d| d.type_id != type_id);
        }
    }

    /// Get decorations for file
    pub fn get_decorations(&self, file: &PathBuf) -> Vec<Decoration> {
        self.decorations
            .read()
            .get(file)
            .cloned()
            .unwrap_or_default()
    }

    /// Get decorations for file and type
    pub fn get_decorations_by_type(&self, file: &PathBuf, type_id: &str) -> Vec<Decoration> {
        self.decorations
            .read()
            .get(file)
            .map(|decs| decs.iter().filter(|d| d.type_id == type_id).cloned().collect())
            .unwrap_or_default()
    }

    /// Get decorations at line
    pub fn get_decorations_at_line(&self, file: &PathBuf, line: u32) -> Vec<Decoration> {
        self.decorations
            .read()
            .get(file)
            .map(|decs| {
                decs.iter()
                    .filter(|d| d.range.contains_line(line))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Clear decorations for file
    pub fn clear_file(&self, file: &PathBuf) {
        self.decorations.write().remove(file);
    }

    /// Clear all decorations
    pub fn clear_all(&self) {
        self.decorations.write().clear();
    }
}

impl Default for EditorDecorationsService {
    fn default() -> Self {
        Self::new()
    }
}

/// Decoration ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DecorationId(u64);

/// Decoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decoration {
    /// Unique ID
    pub id: DecorationId,
    /// Decoration type
    pub type_id: String,
    /// Range
    pub range: DecorationRange,
}

/// Decoration range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecorationRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    /// Hover message
    pub hover_message: Option<String>,
}

impl DecorationRange {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
            hover_message: None,
        }
    }

    pub fn line(line: u32) -> Self {
        Self {
            start_line: line,
            start_col: 0,
            end_line: line,
            end_col: u32::MAX,
            hover_message: None,
        }
    }

    pub fn with_hover(mut self, message: impl Into<String>) -> Self {
        self.hover_message = Some(message.into());
        self
    }

    pub fn contains_line(&self, line: u32) -> bool {
        line >= self.start_line && line <= self.end_line
    }

    pub fn is_whole_line(&self) -> bool {
        self.start_col == 0 && self.end_col == u32::MAX
    }
}

/// Decoration render options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecorationRenderOptions {
    /// Background color
    pub background_color: Option<String>,
    /// Border color
    pub border_color: Option<String>,
    /// Border width
    pub border_width: Option<String>,
    /// Border style
    pub border_style: Option<BorderStyle>,
    /// Border radius
    pub border_radius: Option<String>,
    /// Foreground color (text)
    pub color: Option<String>,
    /// Font style
    pub font_style: Option<FontStyle>,
    /// Font weight
    pub font_weight: Option<String>,
    /// Text decoration
    pub text_decoration: Option<String>,
    /// Outline
    pub outline: Option<String>,
    /// Is whole line
    pub is_whole_line: bool,
    /// Gutter icon
    pub gutter_icon_path: Option<String>,
    /// Before content
    pub before: Option<AttachmentOptions>,
    /// After content
    pub after: Option<AttachmentOptions>,
    /// Overview ruler color
    pub overview_ruler_color: Option<String>,
    /// Overview ruler lane
    pub overview_ruler_lane: Option<OverviewRulerLane>,
    /// Minimap color
    pub minimap_color: Option<String>,
}

impl DecorationRenderOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn background(mut self, color: impl Into<String>) -> Self {
        self.background_color = Some(color.into());
        self
    }

    pub fn border(mut self, color: impl Into<String>) -> Self {
        self.border_color = Some(color.into());
        self
    }

    pub fn foreground(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn italic(mut self) -> Self {
        self.font_style = Some(FontStyle::Italic);
        self
    }

    pub fn bold(mut self) -> Self {
        self.font_weight = Some("bold".to_string());
        self
    }

    pub fn underline(mut self, style: impl Into<String>) -> Self {
        self.text_decoration = Some(style.into());
        self
    }

    pub fn whole_line(mut self) -> Self {
        self.is_whole_line = true;
        self
    }

    pub fn gutter_icon(mut self, path: impl Into<String>) -> Self {
        self.gutter_icon_path = Some(path.into());
        self
    }
}

/// Border style
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BorderStyle {
    Solid,
    Dashed,
    Dotted,
    Double,
    None,
}

/// Font style
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

/// Attachment options (before/after)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentOptions {
    /// Content text
    pub content_text: Option<String>,
    /// Content icon
    pub content_icon_path: Option<String>,
    /// Color
    pub color: Option<String>,
    /// Background color
    pub background_color: Option<String>,
    /// Border
    pub border: Option<String>,
    /// Margin
    pub margin: Option<String>,
    /// Width
    pub width: Option<String>,
    /// Height
    pub height: Option<String>,
}

/// Overview ruler lane
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OverviewRulerLane {
    Left,
    Center,
    Right,
    Full,
}

/// Built-in decoration types
pub mod builtin {
    use super::*;

    pub fn error_squiggly() -> DecorationRenderOptions {
        DecorationRenderOptions::new()
            .underline("wavy underline red")
    }

    pub fn warning_squiggly() -> DecorationRenderOptions {
        DecorationRenderOptions::new()
            .underline("wavy underline yellow")
    }

    pub fn info_squiggly() -> DecorationRenderOptions {
        DecorationRenderOptions::new()
            .underline("wavy underline blue")
    }

    pub fn search_highlight() -> DecorationRenderOptions {
        DecorationRenderOptions::new()
            .background("editor.findMatchBackground")
    }

    pub fn current_search() -> DecorationRenderOptions {
        DecorationRenderOptions::new()
            .background("editor.findMatchHighlightBackground")
            .border("editor.findMatchHighlightBorder")
    }

    pub fn selection() -> DecorationRenderOptions {
        DecorationRenderOptions::new()
            .background("editor.selectionBackground")
    }

    pub fn word_highlight() -> DecorationRenderOptions {
        DecorationRenderOptions::new()
            .background("editor.wordHighlightBackground")
            .border("editor.wordHighlightBorder")
    }

    pub fn line_highlight() -> DecorationRenderOptions {
        DecorationRenderOptions::new()
            .background("editor.lineHighlightBackground")
            .border("editor.lineHighlightBorder")
            .whole_line()
    }
}
