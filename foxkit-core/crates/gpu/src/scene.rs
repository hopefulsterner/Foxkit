//! Scene graph for GPU rendering

use crate::{Color, Point, Rect};

/// Scene - collection of primitives to render
pub struct Scene {
    primitives: Vec<Primitive>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
        }
    }

    /// Add a primitive
    pub fn add(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
    }

    /// Draw a filled rectangle
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.add(Primitive::Quad {
            rect,
            color,
            corner_radius: 0.0,
        });
    }

    /// Draw a rounded rectangle
    pub fn fill_rounded_rect(&mut self, rect: Rect, color: Color, radius: f32) {
        self.add(Primitive::Quad {
            rect,
            color,
            corner_radius: radius,
        });
    }

    /// Draw a line
    pub fn draw_line(&mut self, start: Point, end: Point, color: Color, width: f32) {
        self.add(Primitive::Line {
            start,
            end,
            color,
            width,
        });
    }

    /// Draw text
    pub fn draw_text(&mut self, text: String, position: Point, color: Color, font_size: f32) {
        self.add(Primitive::Text {
            text,
            position,
            color,
            font_size,
            font_family: None,
        });
    }

    /// Get all primitives
    pub fn primitives(&self) -> &[Primitive] {
        &self.primitives
    }

    /// Clear all primitives
    pub fn clear(&mut self) {
        self.primitives.clear();
    }

    /// Number of primitives
    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

/// Rendering primitive
#[derive(Debug, Clone)]
pub enum Primitive {
    /// Filled quad/rectangle
    Quad {
        rect: Rect,
        color: Color,
        corner_radius: f32,
    },
    /// Line
    Line {
        start: Point,
        end: Point,
        color: Color,
        width: f32,
    },
    /// Text
    Text {
        text: String,
        position: Point,
        color: Color,
        font_size: f32,
        font_family: Option<String>,
    },
}

/// Layer for z-ordering
pub struct Layer {
    pub primitives: Vec<Primitive>,
    pub z_index: i32,
    pub opacity: f32,
    pub clip: Option<Rect>,
}

impl Layer {
    pub fn new(z_index: i32) -> Self {
        Self {
            primitives: Vec::new(),
            z_index,
            opacity: 1.0,
            clip: None,
        }
    }

    pub fn add(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn with_clip(mut self, clip: Rect) -> Self {
        self.clip = Some(clip);
        self
    }
}

/// Scene builder for declarative scene construction
pub struct SceneBuilder {
    layers: Vec<Layer>,
    current_layer: usize,
}

impl SceneBuilder {
    pub fn new() -> Self {
        Self {
            layers: vec![Layer::new(0)],
            current_layer: 0,
        }
    }

    /// Add a layer
    pub fn push_layer(&mut self, z_index: i32) -> &mut Self {
        self.layers.push(Layer::new(z_index));
        self.current_layer = self.layers.len() - 1;
        self
    }

    /// Pop current layer
    pub fn pop_layer(&mut self) -> &mut Self {
        if self.current_layer > 0 {
            self.current_layer -= 1;
        }
        self
    }

    /// Add primitive to current layer
    pub fn add(&mut self, primitive: Primitive) -> &mut Self {
        self.layers[self.current_layer].add(primitive);
        self
    }

    /// Build final scene
    pub fn build(mut self) -> Scene {
        // Sort layers by z-index
        self.layers.sort_by_key(|l| l.z_index);

        let mut scene = Scene::new();
        for layer in self.layers {
            for primitive in layer.primitives {
                scene.add(primitive);
            }
        }
        scene
    }
}

impl Default for SceneBuilder {
    fn default() -> Self {
        Self::new()
    }
}
