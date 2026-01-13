//! Inlay hint types

use serde::{Deserialize, Serialize};

use crate::InlayHintTooltip;

/// Inlay hint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlayHint {
    /// Position of the hint
    pub position: Position,
    /// Label to display
    pub label: InlayHintLabel,
    /// Kind of hint
    pub kind: InlayHintKind,
    /// Tooltip (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<InlayHintTooltip>,
    /// Padding before hint
    pub padding_left: bool,
    /// Padding after hint
    pub padding_right: bool,
    /// Data for resolution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl InlayHint {
    /// Create a type hint
    pub fn type_hint(position: Position, type_name: impl Into<String>) -> Self {
        Self {
            position,
            label: InlayHintLabel::String(format!(": {}", type_name.into())),
            kind: InlayHintKind::Type,
            tooltip: None,
            padding_left: false,
            padding_right: false,
            data: None,
        }
    }

    /// Create a parameter hint
    pub fn parameter_hint(position: Position, param_name: impl Into<String>) -> Self {
        Self {
            position,
            label: InlayHintLabel::String(format!("{}:", param_name.into())),
            kind: InlayHintKind::Parameter,
            tooltip: None,
            padding_left: false,
            padding_right: true,
            data: None,
        }
    }

    /// Create a chaining hint
    pub fn chaining_hint(position: Position, type_name: impl Into<String>) -> Self {
        Self {
            position,
            label: InlayHintLabel::String(type_name.into()),
            kind: InlayHintKind::Chaining,
            tooltip: None,
            padding_left: true,
            padding_right: false,
            data: None,
        }
    }

    /// Add tooltip
    pub fn with_tooltip(mut self, tooltip: InlayHintTooltip) -> Self {
        self.tooltip = Some(tooltip);
        self
    }
}

/// Inlay hint kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InlayHintKind {
    /// Type annotation
    Type,
    /// Parameter name
    Parameter,
    /// Method chain result type
    Chaining,
    /// Closure return type
    ClosureReturn,
    /// Generic parameter
    GenericParameter,
    /// Lifetime
    Lifetime,
    /// Binding mode (Rust ref/mut)
    BindingMode,
    /// Discriminant value (enum)
    Discriminant,
}

/// Inlay hint label
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InlayHintLabel {
    /// Simple string label
    String(String),
    /// Label with parts (for clickable sections)
    Parts(Vec<InlayHintLabelPart>),
}

impl InlayHintLabel {
    /// Get label text
    pub fn text(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Parts(parts) => parts.iter().map(|p| p.value.as_str()).collect(),
        }
    }
}

/// Label part (for multi-part labels)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlayHintLabelPart {
    /// Text value
    pub value: String,
    /// Tooltip for this part
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<InlayHintTooltip>,
    /// Location to navigate to on click
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    /// Command to execute on click
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Command>,
}

/// Location for navigation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// Command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub title: String,
    pub command: String,
    #[serde(default)]
    pub arguments: Vec<serde_json::Value>,
}

/// Position in document
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// Range in document
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn line(line: u32) -> Self {
        Self {
            start: Position::new(line, 0),
            end: Position::new(line, u32::MAX),
        }
    }
}
