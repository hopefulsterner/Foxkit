//! Cursor types and management

use serde::{Deserialize, Serialize};
use crate::Position;

/// Cursor shape
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorShape {
    /// Vertical line (|)
    Line,
    /// Block cursor (█)
    Block,
    /// Underline (_)
    Underline,
}

impl Default for CursorShape {
    fn default() -> Self {
        CursorShape::Line
    }
}

/// Cursor blink mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorBlink {
    /// Cursor blinks
    Blink,
    /// Cursor is solid (no blink)
    Solid,
    /// Smooth blink (fade in/out)
    Smooth,
    /// Phase blink (expand/contract)
    Phase,
}

impl Default for CursorBlink {
    fn default() -> Self {
        CursorBlink::Blink
    }
}

/// Cursor style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorStyle {
    /// Shape
    pub shape: CursorShape,
    /// Blink mode
    pub blink: CursorBlink,
    /// Blink rate in ms
    pub blink_rate: u32,
    /// Width in pixels (for Line cursor)
    pub width: u32,
    /// Color (CSS color string)
    pub color: Option<String>,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self {
            shape: CursorShape::Line,
            blink: CursorBlink::Blink,
            blink_rate: 530,
            width: 2,
            color: None,
        }
    }
}

/// Cursor state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    /// Position
    pub position: Position,
    /// Style
    pub style: CursorStyle,
    /// Is visible (for blink)
    pub visible: bool,
    /// Cursor ID (for multi-cursor)
    pub id: u32,
}

impl Cursor {
    pub fn new(position: Position) -> Self {
        Self {
            position,
            style: CursorStyle::default(),
            visible: true,
            id: 0,
        }
    }

    pub fn with_id(mut self, id: u32) -> Self {
        self.id = id;
        self
    }

    pub fn with_style(mut self, style: CursorStyle) -> Self {
        self.style = style;
        self
    }

    /// Get line number
    pub fn line(&self) -> u32 {
        self.position.line
    }

    /// Get column
    pub fn column(&self) -> u32 {
        self.position.column
    }

    /// Move to position
    pub fn move_to(&mut self, position: Position) {
        self.position = position;
    }

    /// Move by delta
    pub fn move_by(&mut self, line_delta: i32, column_delta: i32) {
        self.position = self.position.offset(line_delta, column_delta);
    }

    /// Toggle visibility (for blink)
    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new(Position::zero())
    }
}

/// Cursor controller for managing blink state
pub struct CursorController {
    /// Current style
    style: CursorStyle,
    /// Is blinking enabled
    blink_enabled: bool,
    /// Last blink time
    last_blink: std::time::Instant,
    /// Current visibility
    visible: bool,
}

impl CursorController {
    pub fn new() -> Self {
        Self {
            style: CursorStyle::default(),
            blink_enabled: true,
            last_blink: std::time::Instant::now(),
            visible: true,
        }
    }

    pub fn with_style(mut self, style: CursorStyle) -> Self {
        self.style = style;
        self
    }

    /// Update blink state, return true if changed
    pub fn update(&mut self) -> bool {
        if !self.blink_enabled || self.style.blink == CursorBlink::Solid {
            if !self.visible {
                self.visible = true;
                return true;
            }
            return false;
        }

        let elapsed = self.last_blink.elapsed().as_millis() as u32;
        if elapsed >= self.style.blink_rate {
            self.visible = !self.visible;
            self.last_blink = std::time::Instant::now();
            return true;
        }

        false
    }

    /// Reset blink (show cursor immediately)
    pub fn reset_blink(&mut self) {
        self.visible = true;
        self.last_blink = std::time::Instant::now();
    }

    /// Is cursor visible?
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Enable/disable blink
    pub fn set_blink_enabled(&mut self, enabled: bool) {
        self.blink_enabled = enabled;
        if !enabled {
            self.visible = true;
        }
    }

    /// Get style
    pub fn style(&self) -> &CursorStyle {
        &self.style
    }

    /// Set style
    pub fn set_style(&mut self, style: CursorStyle) {
        self.style = style;
    }
}

impl Default for CursorController {
    fn default() -> Self {
        Self::new()
    }
}
