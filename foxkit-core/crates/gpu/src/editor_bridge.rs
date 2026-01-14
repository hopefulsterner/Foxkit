//! Editor rendering bridge
//!
//! Connects the editor's RenderCommand system to the GPU Scene primitive system.
//! This module translates high-level editor rendering commands into GPU-renderable primitives.

use crate::{Color, Point, Rect, Scene, Primitive};

/// Line style for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStyle {
    Solid,
    Dotted,
    Dashed,
    Wavy,
}

/// Editor render command (mirrors editor::render::RenderCommand)
#[derive(Debug, Clone)]
pub enum EditorRenderCommand {
    /// Draw a filled rectangle
    FillRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
        corner_radius: f32,
    },
    /// Draw text
    DrawText {
        x: f32,
        y: f32,
        text: String,
        font_size: f32,
        color: [f32; 4],
        bold: bool,
        italic: bool,
    },
    /// Draw a line
    DrawLine {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        width: f32,
        color: [f32; 4],
        style: LineStyle,
    },
    /// Set clipping rectangle
    SetClip {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// Clear clipping
    ClearClip,
}

/// Editor scene builder - converts editor commands to GPU scene
pub struct EditorSceneBuilder {
    scene: Scene,
    clip_stack: Vec<Rect>,
    current_clip: Option<Rect>,
}

impl EditorSceneBuilder {
    /// Create a new editor scene builder
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            clip_stack: Vec::new(),
            current_clip: None,
        }
    }

    /// Process a batch of editor render commands
    pub fn process_commands(&mut self, commands: &[EditorRenderCommand]) {
        for cmd in commands {
            self.process_command(cmd);
        }
    }

    /// Process a single editor render command
    pub fn process_command(&mut self, command: &EditorRenderCommand) {
        match command {
            EditorRenderCommand::FillRect { x, y, width, height, color, corner_radius } => {
                // Check clipping
                if let Some(clip) = &self.current_clip {
                    let rect = Rect::new(*x, *y, *width, *height);
                    if !self.rects_intersect(&rect, clip) {
                        return; // Completely clipped
                    }
                }

                let color = Color::rgba(color[0], color[1], color[2], color[3]);
                let rect = Rect::new(*x, *y, *width, *height);
                
                if *corner_radius > 0.0 {
                    self.scene.fill_rounded_rect(rect, color, *corner_radius);
                } else {
                    self.scene.fill_rect(rect, color);
                }
            }

            EditorRenderCommand::DrawText { x, y, text, font_size, color, bold: _, italic: _ } => {
                // Check clipping
                if let Some(clip) = &self.current_clip {
                    if *x < clip.origin.x || *y < clip.origin.y ||
                       *x > clip.origin.x + clip.size.width ||
                       *y > clip.origin.y + clip.size.height {
                        return; // Text origin outside clip
                    }
                }

                let color = Color::rgba(color[0], color[1], color[2], color[3]);
                let position = Point::new(*x, *y);
                
                // TODO: Handle bold/italic with font variants
                self.scene.draw_text(text.clone(), position, color, *font_size);
            }

            EditorRenderCommand::DrawLine { x1, y1, x2, y2, width, color, style } => {
                let color = Color::rgba(color[0], color[1], color[2], color[3]);
                let start = Point::new(*x1, *y1);
                let end = Point::new(*x2, *y2);

                match style {
                    LineStyle::Solid => {
                        self.scene.draw_line(start, end, color, *width);
                    }
                    LineStyle::Dotted => {
                        self.draw_dotted_line(start, end, color, *width);
                    }
                    LineStyle::Dashed => {
                        self.draw_dashed_line(start, end, color, *width);
                    }
                    LineStyle::Wavy => {
                        self.draw_wavy_line(start, end, color, *width);
                    }
                }
            }

            EditorRenderCommand::SetClip { x, y, width, height } => {
                let clip = Rect::new(*x, *y, *width, *height);
                if let Some(current) = &self.current_clip {
                    self.clip_stack.push(*current);
                }
                self.current_clip = Some(clip);
            }

            EditorRenderCommand::ClearClip => {
                self.current_clip = self.clip_stack.pop();
            }
        }
    }

    /// Draw a dotted line
    fn draw_dotted_line(&mut self, start: Point, end: Point, color: Color, width: f32) {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let length = (dx * dx + dy * dy).sqrt();
        
        if length == 0.0 {
            return;
        }

        let dot_spacing = width * 3.0;
        let num_dots = (length / dot_spacing) as usize;
        
        let nx = dx / length;
        let ny = dy / length;

        for i in 0..=num_dots {
            let t = i as f32 * dot_spacing;
            let x = start.x + nx * t;
            let y = start.y + ny * t;
            
            // Draw dot as small circle (quad with corner_radius)
            let rect = Rect::new(x - width / 2.0, y - width / 2.0, width, width);
            self.scene.fill_rounded_rect(rect, color, width / 2.0);
        }
    }

    /// Draw a dashed line
    fn draw_dashed_line(&mut self, start: Point, end: Point, color: Color, width: f32) {
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let length = (dx * dx + dy * dy).sqrt();
        
        if length == 0.0 {
            return;
        }

        let dash_length = width * 4.0;
        let gap_length = width * 2.0;
        let segment_length = dash_length + gap_length;
        
        let nx = dx / length;
        let ny = dy / length;

        let mut t = 0.0;
        while t < length {
            let dash_end = (t + dash_length).min(length);
            
            let p1 = Point::new(start.x + nx * t, start.y + ny * t);
            let p2 = Point::new(start.x + nx * dash_end, start.y + ny * dash_end);
            
            self.scene.draw_line(p1, p2, color, width);
            
            t += segment_length;
        }
    }

    /// Draw a wavy line (for error underlines)
    fn draw_wavy_line(&mut self, start: Point, end: Point, color: Color, width: f32) {
        let dx = end.x - start.x;
        let length = dx.abs();
        
        if length == 0.0 {
            return;
        }

        let wave_period = width * 4.0;
        let wave_amplitude = width * 1.5;
        let num_segments = (length / (wave_period / 4.0)) as usize;
        
        let dir = if dx > 0.0 { 1.0 } else { -1.0 };
        
        let mut prev_point = start;
        
        for i in 1..=num_segments {
            let t = i as f32 * (wave_period / 4.0) * dir;
            let x = start.x + t;
            
            // Alternate up and down
            let phase = (i % 4) as f32;
            let y_offset = match i % 4 {
                1 => -wave_amplitude,
                3 => wave_amplitude,
                _ => 0.0,
            };
            
            let current_point = Point::new(x, start.y + y_offset);
            
            self.scene.draw_line(prev_point, current_point, color, width);
            prev_point = current_point;
        }
        
        // Connect to end
        if (prev_point.x - end.x).abs() > 0.1 {
            self.scene.draw_line(prev_point, end, color, width);
        }
    }

    /// Check if two rectangles intersect
    fn rects_intersect(&self, a: &Rect, b: &Rect) -> bool {
        a.origin.x < b.origin.x + b.size.width &&
        a.origin.x + a.size.width > b.origin.x &&
        a.origin.y < b.origin.y + b.size.height &&
        a.origin.y + a.size.height > b.origin.y
    }

    /// Finish building and return the scene
    pub fn finish(self) -> Scene {
        self.scene
    }

    /// Clear and return a mutable reference for reuse
    pub fn clear(&mut self) {
        self.scene.clear();
        self.clip_stack.clear();
        self.current_clip = None;
    }

    /// Get the scene for rendering
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
}

impl Default for EditorSceneBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert color array to GPU Color
pub fn color_from_array(c: [f32; 4]) -> Color {
    Color::rgba(c[0], c[1], c[2], c[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fill_rect() {
        let mut builder = EditorSceneBuilder::new();
        builder.process_command(&EditorRenderCommand::FillRect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
            color: [1.0, 0.0, 0.0, 1.0],
            corner_radius: 0.0,
        });
        
        let scene = builder.finish();
        assert_eq!(scene.len(), 1);
    }

    #[test]
    fn test_draw_text() {
        let mut builder = EditorSceneBuilder::new();
        builder.process_command(&EditorRenderCommand::DrawText {
            x: 10.0,
            y: 20.0,
            text: "Hello".to_string(),
            font_size: 14.0,
            color: [1.0, 1.0, 1.0, 1.0],
            bold: false,
            italic: false,
        });
        
        let scene = builder.finish();
        assert_eq!(scene.len(), 1);
    }

    #[test]
    fn test_clipping() {
        let mut builder = EditorSceneBuilder::new();
        
        // Set clip
        builder.process_command(&EditorRenderCommand::SetClip {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        });
        
        // This rect is inside clip
        builder.process_command(&EditorRenderCommand::FillRect {
            x: 10.0,
            y: 10.0,
            width: 50.0,
            height: 50.0,
            color: [1.0, 0.0, 0.0, 1.0],
            corner_radius: 0.0,
        });
        
        // This rect is outside clip
        builder.process_command(&EditorRenderCommand::FillRect {
            x: 200.0,
            y: 200.0,
            width: 50.0,
            height: 50.0,
            color: [0.0, 1.0, 0.0, 1.0],
            corner_radius: 0.0,
        });
        
        let scene = builder.finish();
        assert_eq!(scene.len(), 1); // Only the first rect should be added
    }

    #[test]
    fn test_line_styles() {
        let mut builder = EditorSceneBuilder::new();
        
        // Solid line
        builder.process_command(&EditorRenderCommand::DrawLine {
            x1: 0.0, y1: 0.0,
            x2: 100.0, y2: 0.0,
            width: 2.0,
            color: [1.0, 1.0, 1.0, 1.0],
            style: LineStyle::Solid,
        });
        
        // Dotted line
        builder.process_command(&EditorRenderCommand::DrawLine {
            x1: 0.0, y1: 10.0,
            x2: 100.0, y2: 10.0,
            width: 2.0,
            color: [1.0, 0.0, 0.0, 1.0],
            style: LineStyle::Dotted,
        });
        
        let scene = builder.finish();
        assert!(scene.len() >= 2); // At least solid + dots
    }
}
