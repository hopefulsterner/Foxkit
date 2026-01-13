//! Foxkit GPUI - GPU-accelerated UI framework.
//!
//! Inspired by Zed's GPUI, this provides a high-performance,
//! GPU-rendered UI system for building the editor interface.

use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// A unique identifier for UI elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementId(pub u64);

/// 2D point.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// 2D size.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

/// Rectangle bounds.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Bounds {
    pub origin: Point,
    pub size: Size,
}

impl Bounds {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point { x, y },
            size: Size { width, height },
        }
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.origin.x
            && point.x < self.origin.x + self.size.width
            && point.y >= self.origin.y
            && point.y < self.origin.y + self.size.height
    }
}

/// RGBA color.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const WHITE: Color = Color::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);
    pub const TRANSPARENT: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);
}

/// Edge insets (padding/margin).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: vertical,
            bottom: vertical,
            left: horizontal,
            right: horizontal,
        }
    }
}

/// Corner radii for rounded rectangles.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Corners {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Corners {
    pub fn all(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }
}

/// Layout direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Axis {
    #[default]
    Horizontal,
    Vertical,
}

/// Main axis alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MainAxisAlignment {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Cross axis alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrossAxisAlignment {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

/// Style for an element.
#[derive(Debug, Clone, Default)]
pub struct Style {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub corner_radius: Corners,
    pub padding: Edges,
    pub margin: Edges,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,
    pub flex_grow: f32,
    pub flex_shrink: f32,
}

/// A render context for drawing.
pub struct RenderContext {
    bounds: Bounds,
    scale_factor: f32,
}

impl RenderContext {
    pub fn new(bounds: Bounds, scale_factor: f32) -> Self {
        Self { bounds, scale_factor }
    }

    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }
}

/// Window handle.
pub struct Window {
    id: u64,
    bounds: RwLock<Bounds>,
    scale_factor: RwLock<f32>,
}

impl Window {
    pub fn new(id: u64, bounds: Bounds, scale_factor: f32) -> Self {
        Self {
            id,
            bounds: RwLock::new(bounds),
            scale_factor: RwLock::new(scale_factor),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn bounds(&self) -> Bounds {
        *self.bounds.read()
    }

    pub fn scale_factor(&self) -> f32 {
        *self.scale_factor.read()
    }
}

/// Application context.
pub struct AppContext {
    windows: RwLock<Vec<Arc<Window>>>,
}

impl AppContext {
    pub fn new() -> Self {
        Self {
            windows: RwLock::new(Vec::new()),
        }
    }

    pub fn add_window(&self, window: Arc<Window>) {
        self.windows.write().push(window);
    }
}

impl Default for AppContext {
    fn default() -> Self {
        Self::new()
    }
}
