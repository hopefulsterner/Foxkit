//! Styling system

use crate::layout::{Edge, Direction, Align, Justify};

/// Style properties
#[derive(Debug, Clone, Default)]
pub struct Style {
    // Size
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub min_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,

    // Spacing
    pub margin: Edge,
    pub padding: Edge,

    // Layout
    pub direction: Direction,
    pub align: Align,
    pub justify: Justify,
    pub gap: f32,
    pub flex: f32,

    // Visual
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub border_radius: f32,
    pub opacity: f32,

    // Text
    pub font_size: Option<f32>,
    pub font_weight: FontWeight,
    pub text_align: TextAlign,
    pub line_height: Option<f32>,

    // Cursor
    pub cursor: Cursor,
}

impl Style {
    pub fn new() -> Self {
        Self {
            opacity: 1.0,
            ..Default::default()
        }
    }
}

/// Style builder
#[derive(Default)]
pub struct StyleBuilder {
    style: Style,
}

impl StyleBuilder {
    pub fn new() -> Self {
        Self { style: Style::new() }
    }

    // Size
    pub fn width(mut self, w: f32) -> Self { self.style.width = Some(w); self }
    pub fn height(mut self, h: f32) -> Self { self.style.height = Some(h); self }
    pub fn size(self, w: f32, h: f32) -> Self { self.width(w).height(h) }
    pub fn min_width(mut self, w: f32) -> Self { self.style.min_width = Some(w); self }
    pub fn min_height(mut self, h: f32) -> Self { self.style.min_height = Some(h); self }
    pub fn max_width(mut self, w: f32) -> Self { self.style.max_width = Some(w); self }
    pub fn max_height(mut self, h: f32) -> Self { self.style.max_height = Some(h); self }

    // Spacing
    pub fn margin(mut self, m: Edge) -> Self { self.style.margin = m; self }
    pub fn margin_all(self, m: f32) -> Self { self.margin(Edge::all(m)) }
    pub fn padding(mut self, p: Edge) -> Self { self.style.padding = p; self }
    pub fn padding_all(self, p: f32) -> Self { self.padding(Edge::all(p)) }
    pub fn gap(mut self, g: f32) -> Self { self.style.gap = g; self }

    // Layout
    pub fn direction(mut self, d: Direction) -> Self { self.style.direction = d; self }
    pub fn row(self) -> Self { self.direction(Direction::Horizontal) }
    pub fn column(self) -> Self { self.direction(Direction::Vertical) }
    pub fn align(mut self, a: Align) -> Self { self.style.align = a; self }
    pub fn justify(mut self, j: Justify) -> Self { self.style.justify = j; self }
    pub fn flex(mut self, f: f32) -> Self { self.style.flex = f; self }

    // Visual
    pub fn background(mut self, c: Color) -> Self { self.style.background = Some(c); self }
    pub fn foreground(mut self, c: Color) -> Self { self.style.foreground = Some(c); self }
    pub fn border(mut self, color: Color, width: f32) -> Self {
        self.style.border_color = Some(color);
        self.style.border_width = width;
        self
    }
    pub fn border_radius(mut self, r: f32) -> Self { self.style.border_radius = r; self }
    pub fn opacity(mut self, o: f32) -> Self { self.style.opacity = o; self }

    // Text
    pub fn font_size(mut self, s: f32) -> Self { self.style.font_size = Some(s); self }
    pub fn font_weight(mut self, w: FontWeight) -> Self { self.style.font_weight = w; self }
    pub fn bold(self) -> Self { self.font_weight(FontWeight::Bold) }
    pub fn text_align(mut self, a: TextAlign) -> Self { self.style.text_align = a; self }
    pub fn line_height(mut self, h: f32) -> Self { self.style.line_height = Some(h); self }

    // Cursor
    pub fn cursor(mut self, c: Cursor) -> Self { self.style.cursor = c; self }

    pub fn build(self) -> Style {
        self.style
    }
}

/// Color
#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self { r: 0, g: 0, b: 0, a: 0 };
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
            a: 255,
        }
    }
}

/// Font weight
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FontWeight {
    Thin,
    Light,
    #[default]
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

/// Text alignment
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

/// Cursor style
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Cursor {
    #[default]
    Default,
    Pointer,
    Text,
    Move,
    NotAllowed,
    Wait,
    Crosshair,
    ResizeNS,
    ResizeEW,
    ResizeNESW,
    ResizeNWSE,
}
