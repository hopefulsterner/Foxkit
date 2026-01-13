//! Cursor representation and manipulation

/// Cursor position in buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    /// Byte offset in buffer
    pub offset: usize,
    /// Preferred column (for vertical movement)
    pub preferred_column: Option<usize>,
}

/// Cursor visual shape
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorShape {
    /// Vertical bar (insert mode)
    #[default]
    Bar,
    /// Full block (normal mode)
    Block,
    /// Underline (replace mode)
    Underline,
    /// Hidden cursor
    Hidden,
}

impl Cursor {
    /// Create a cursor at offset
    pub fn new(offset: usize) -> Self {
        Self {
            offset,
            preferred_column: None,
        }
    }

    /// Move cursor left by n characters
    pub fn move_left(&mut self, n: usize) {
        self.offset = self.offset.saturating_sub(n);
        self.preferred_column = None;
    }

    /// Move cursor right by n characters
    pub fn move_right(&mut self, n: usize, max: usize) {
        self.offset = (self.offset + n).min(max);
        self.preferred_column = None;
    }
}
