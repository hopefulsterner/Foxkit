//! Layout system

use crate::Style;

/// Size
#[derive(Debug, Clone, Copy, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }
}

/// Bounds (position + size)
#[derive(Debug, Clone, Copy, Default)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Bounds {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.width &&
        py >= self.y && py <= self.y + self.height
    }

    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    pub fn inset(&self, edge: Edge) -> Self {
        Self {
            x: self.x + edge.left,
            y: self.y + edge.top,
            width: self.width - edge.left - edge.right,
            height: self.height - edge.top - edge.bottom,
        }
    }
}

/// Edge insets (margin, padding, border)
#[derive(Debug, Clone, Copy, Default)]
pub struct Edge {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edge {
    pub fn all(value: f32) -> Self {
        Self { top: value, right: value, bottom: value, left: value }
    }

    pub fn xy(x: f32, y: f32) -> Self {
        Self { top: y, right: x, bottom: y, left: x }
    }

    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self { top, right, bottom, left }
    }

    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

/// Layout trait
pub trait Layout {
    fn layout(&mut self, ctx: &mut LayoutContext);
}

/// Layout context
pub struct LayoutContext {
    /// Available bounds
    pub available: Bounds,
    /// Current position
    pub cursor_x: f32,
    pub cursor_y: f32,
    /// Layout direction
    pub direction: Direction,
    /// Row height (for wrapping)
    pub row_height: f32,
}

impl LayoutContext {
    pub fn new(available: Bounds) -> Self {
        Self {
            available,
            cursor_x: available.x,
            cursor_y: available.y,
            direction: Direction::Vertical,
            row_height: 0.0,
        }
    }

    /// Create a child context
    pub fn child_context(&self, bounds: &Bounds, style: &Style) -> Self {
        let inner = bounds.inset(style.padding);
        let mut ctx = Self::new(inner);
        ctx.direction = style.direction;
        ctx
    }

    /// Allocate space for an element
    pub fn allocate(&mut self, style: &Style) -> Bounds {
        let width = style.width.unwrap_or(self.available.width);
        let height = style.height.unwrap_or(0.0);

        let bounds = match self.direction {
            Direction::Vertical => {
                let b = Bounds::new(self.cursor_x, self.cursor_y, width, height);
                self.cursor_y += height + style.margin.bottom;
                b
            }
            Direction::Horizontal => {
                let b = Bounds::new(self.cursor_x, self.cursor_y, width, height);
                self.cursor_x += width + style.margin.right;
                self.row_height = self.row_height.max(height);
                b
            }
        };

        bounds
    }

    /// Remaining width
    pub fn remaining_width(&self) -> f32 {
        self.available.width - (self.cursor_x - self.available.x)
    }

    /// Remaining height
    pub fn remaining_height(&self) -> f32 {
        self.available.height - (self.cursor_y - self.available.y)
    }
}

/// Layout direction
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    Vertical,
    Horizontal,
}

/// Alignment
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

/// Justify content
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}
