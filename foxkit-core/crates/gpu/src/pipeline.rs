//! End-to-end editor rendering pipeline
//!
//! Connects all the pieces together:
//! - EditorController (scroll + soft wrap)
//! - EditorRenderer (layout + render commands)  
//! - EditorSceneBuilder (render commands -> GPU scene)
//! - IntegratedTextRenderer (font rasterization -> GPU)
//! - Renderer (GPU scene -> frames)

use std::sync::Arc;
use parking_lot::RwLock;
use wgpu::*;

use crate::{
    Color, Point, Rect, Scene, Primitive, Renderer,
    FontSystem, FontKey, FontWeight, FontStyle, FontMetrics,
    EditorSceneBuilder, EditorRenderCommand, EditorLineStyle,
};
use crate::integrated_text::{IntegratedTextRenderer, PositionedGlyph, build_text_vertices, TextVertex};

/// Prepared frame data for rendering
#[derive(Debug, Clone, Default)]
pub struct PreparedFrame {
    /// Number of text indices to draw
    pub text_index_count: u32,
}

/// Editor render pipeline - complete rendering solution
pub struct EditorRenderPipeline {
    /// GPU renderer for primitives
    renderer: Renderer,
    /// Integrated text renderer
    text_renderer: IntegratedTextRenderer,
    /// Font system
    font_system: Arc<RwLock<FontSystem>>,
    /// Scene builder
    scene_builder: EditorSceneBuilder,
    /// Vertex buffer for text
    text_vertex_buffer: Buffer,
    /// Index buffer for text
    text_index_buffer: Buffer,
    /// Max text vertices
    max_text_vertices: usize,
    /// Queue reference
    queue: Arc<Queue>,
    /// Device reference  
    device: Arc<Device>,
    /// Viewport size
    viewport_size: (u32, u32),
    /// Default font
    default_font: FontKey,
    /// Default font size
    default_font_size: f32,
}

impl EditorRenderPipeline {
    /// Create a new editor render pipeline
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        format: TextureFormat,
    ) -> anyhow::Result<Self> {
        let font_system = Arc::new(RwLock::new(FontSystem::new()));
        
        let renderer = Renderer::new(device.clone(), queue.clone(), format)?;
        let text_renderer = IntegratedTextRenderer::new(
            device.clone(),
            queue.clone(),
            font_system.clone(),
            format,
        );

        let max_text_vertices = 65536;
        
        let text_vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Text Vertex Buffer"),
            size: (max_text_vertices * std::mem::size_of::<TextVertex>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let text_index_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Text Index Buffer"),
            size: (max_text_vertices * 6 * std::mem::size_of::<u32>()) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            renderer,
            text_renderer,
            font_system,
            scene_builder: EditorSceneBuilder::new(),
            text_vertex_buffer,
            text_index_buffer,
            max_text_vertices,
            queue,
            device,
            viewport_size: (800, 600),
            default_font: FontKey::regular("monospace"),
            default_font_size: 14.0,
        })
    }

    /// Load a font from bytes
    pub fn load_font(&mut self, key: FontKey, data: &[u8]) -> anyhow::Result<()> {
        self.font_system.write().load_font(key, data)
    }

    /// Load a font from file
    pub fn load_font_file(&mut self, key: FontKey, path: &str) -> anyhow::Result<()> {
        self.font_system.write().load_font_file(key, path)
    }

    /// Set default font
    pub fn set_default_font(&mut self, key: FontKey) {
        self.default_font = key.clone();
        self.font_system.write().set_default_font(key.clone());
        self.text_renderer.set_default_font(key);
    }

    /// Set default font size
    pub fn set_default_font_size(&mut self, size: f32) {
        self.default_font_size = size;
    }

    /// Resize viewport
    pub fn resize(&mut self, width: u32, height: u32) {
        self.viewport_size = (width, height);
        self.renderer.resize(width, height);
    }

    /// Get font metrics
    pub fn font_metrics(&self) -> Option<FontMetrics> {
        self.font_system.read().font_metrics(&self.default_font, self.default_font_size)
    }

    /// Process editor render commands and build scene
    pub fn process_commands(&mut self, commands: &[EditorRenderCommand]) {
        self.scene_builder = EditorSceneBuilder::new();
        self.scene_builder.process_commands(commands);
    }

    /// Prepare rendering data from commands (call before render_prepared)
    pub fn prepare(&mut self, commands: &[EditorRenderCommand]) -> PreparedFrame {
        // Separate text commands from other commands
        let (text_commands, other_commands): (Vec<_>, Vec<_>) = commands.iter().cloned().partition(|cmd| {
            matches!(cmd, EditorRenderCommand::DrawText { .. })
        });

        // Build scene from non-text commands
        self.scene_builder.clear();
        self.scene_builder.process_commands(&other_commands);

        // Build text vertices
        let mut all_glyphs = Vec::new();
        for cmd in &text_commands {
            if let EditorRenderCommand::DrawText { x, y, text, font_size, color, bold, italic } = cmd {
                let font_key = self.get_font_key(*bold, *italic);
                let glyph_color = Color::rgba(color[0], color[1], color[2], color[3]);
                
                let glyphs = self.text_renderer.layout_text(
                    text,
                    Point::new(*x, *y),
                    &font_key,
                    *font_size,
                    glyph_color,
                );
                all_glyphs.extend(glyphs);
            }
        }

        let (text_vertices, text_indices) = if !all_glyphs.is_empty() {
            let atlas_size = self.text_renderer.atlas_size();
            build_text_vertices(&all_glyphs, atlas_size)
        } else {
            (Vec::new(), Vec::new())
        };

        // Upload to GPU
        if !text_vertices.is_empty() {
            self.queue.write_buffer(&self.text_vertex_buffer, 0, bytemuck::cast_slice(&text_vertices));
            self.queue.write_buffer(&self.text_index_buffer, 0, bytemuck::cast_slice(&text_indices));
        }

        PreparedFrame {
            text_index_count: text_indices.len() as u32,
        }
    }

    /// Render a prepared frame
    pub fn render_prepared<'a>(&'a self, pass: &mut RenderPass<'a>, prepared: &PreparedFrame) {
        // Render primitives (rectangles, lines)
        self.renderer.render(pass, self.scene_builder.scene());

        // Draw text
        if prepared.text_index_count > 0 {
            pass.set_pipeline(self.text_renderer.pipeline());
            pass.set_bind_group(0, self.text_renderer.bind_group(), &[]);
            pass.set_vertex_buffer(0, self.text_vertex_buffer.slice(..));
            pass.set_index_buffer(self.text_index_buffer.slice(..), IndexFormat::Uint32);
            pass.draw_indexed(0..prepared.text_index_count, 0, 0..1);
        }
    }

    /// Convenience method: prepare and render in one call
    /// Note: This creates a temporary scene, which has lifetime limitations
    pub fn render_commands(&mut self, commands: &[EditorRenderCommand]) -> PreparedFrame {
        self.prepare(commands)
    }

    /// Get font key based on style
    fn get_font_key(&self, bold: bool, italic: bool) -> FontKey {
        let weight = if bold { FontWeight::Bold } else { FontWeight::Regular };
        let style = if italic { FontStyle::Italic } else { FontStyle::Normal };
        FontKey::new(self.default_font.family.clone(), weight, style)
    }

    /// Clear all caches
    pub fn clear_caches(&mut self) {
        self.text_renderer.clear_cache();
        self.font_system.write().clear_caches();
    }

    /// Get the font system for external use
    pub fn font_system(&self) -> Arc<RwLock<FontSystem>> {
        self.font_system.clone()
    }

    /// Get text renderer for external text measurement
    pub fn text_renderer_mut(&mut self) -> &mut IntegratedTextRenderer {
        &mut self.text_renderer
    }

    /// Measure text width
    pub fn measure_text(&mut self, text: &str, bold: bool, italic: bool) -> f32 {
        let font_key = self.get_font_key(bold, italic);
        self.text_renderer.measure_text(text, &font_key, self.default_font_size)
    }
}

/// Frame builder - helper for building a complete frame
pub struct FrameBuilder {
    commands: Vec<EditorRenderCommand>,
    background_color: [f32; 4],
}

impl FrameBuilder {
    /// Create a new frame builder
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            background_color: [0.1, 0.1, 0.1, 1.0],
        }
    }

    /// Set background color
    pub fn background(&mut self, color: [f32; 4]) -> &mut Self {
        self.background_color = color;
        self
    }

    /// Add a filled rectangle
    pub fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> &mut Self {
        self.commands.push(EditorRenderCommand::FillRect {
            x, y, width, height, color, corner_radius: 0.0,
        });
        self
    }

    /// Add a rounded rectangle
    pub fn fill_rounded_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4], radius: f32) -> &mut Self {
        self.commands.push(EditorRenderCommand::FillRect {
            x, y, width, height, color, corner_radius: radius,
        });
        self
    }

    /// Add text
    pub fn text(&mut self, x: f32, y: f32, text: impl Into<String>, font_size: f32, color: [f32; 4]) -> &mut Self {
        self.commands.push(EditorRenderCommand::DrawText {
            x, y, text: text.into(), font_size, color, bold: false, italic: false,
        });
        self
    }

    /// Add bold text
    pub fn bold_text(&mut self, x: f32, y: f32, text: impl Into<String>, font_size: f32, color: [f32; 4]) -> &mut Self {
        self.commands.push(EditorRenderCommand::DrawText {
            x, y, text: text.into(), font_size, color, bold: true, italic: false,
        });
        self
    }

    /// Add italic text
    pub fn italic_text(&mut self, x: f32, y: f32, text: impl Into<String>, font_size: f32, color: [f32; 4]) -> &mut Self {
        self.commands.push(EditorRenderCommand::DrawText {
            x, y, text: text.into(), font_size, color, bold: false, italic: true,
        });
        self
    }

    /// Add a line
    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: [f32; 4]) -> &mut Self {
        self.commands.push(EditorRenderCommand::DrawLine {
            x1, y1, x2, y2, width, color, style: EditorLineStyle::Solid,
        });
        self
    }

    /// Add a dotted line
    pub fn dotted_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: [f32; 4]) -> &mut Self {
        self.commands.push(EditorRenderCommand::DrawLine {
            x1, y1, x2, y2, width, color, style: EditorLineStyle::Dotted,
        });
        self
    }

    /// Add a wavy underline (for errors)
    pub fn wavy_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: [f32; 4]) -> &mut Self {
        self.commands.push(EditorRenderCommand::DrawLine {
            x1, y1, x2, y2, width, color, style: EditorLineStyle::Wavy,
        });
        self
    }

    /// Set clipping rectangle
    pub fn push_clip(&mut self, x: f32, y: f32, width: f32, height: f32) -> &mut Self {
        self.commands.push(EditorRenderCommand::SetClip { x, y, width, height });
        self
    }

    /// Clear clipping
    pub fn pop_clip(&mut self) -> &mut Self {
        self.commands.push(EditorRenderCommand::ClearClip);
        self
    }

    /// Get the commands
    pub fn commands(&self) -> &[EditorRenderCommand] {
        &self.commands
    }

    /// Take the commands (consuming self)
    pub fn build(self) -> Vec<EditorRenderCommand> {
        self.commands
    }

    /// Get background color
    pub fn background_color(&self) -> [f32; 4] {
        self.background_color
    }
}

impl Default for FrameBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Render configuration
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Background color
    pub background_color: [f32; 4],
    /// Foreground (text) color
    pub foreground_color: [f32; 4],
    /// Selection color
    pub selection_color: [f32; 4],
    /// Cursor color
    pub cursor_color: [f32; 4],
    /// Line number color
    pub line_number_color: [f32; 4],
    /// Current line highlight color
    pub current_line_color: [f32; 4],
    /// Error underline color
    pub error_color: [f32; 4],
    /// Warning underline color
    pub warning_color: [f32; 4],
    /// Font size
    pub font_size: f32,
    /// Line height multiplier
    pub line_height_multiplier: f32,
    /// Gutter width in characters
    pub gutter_width: usize,
    /// Show line numbers
    pub show_line_numbers: bool,
    /// Cursor blink rate (seconds)
    pub cursor_blink_rate: f32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            background_color: [0.12, 0.12, 0.12, 1.0],
            foreground_color: [0.9, 0.9, 0.9, 1.0],
            selection_color: [0.25, 0.45, 0.7, 0.5],
            cursor_color: [0.9, 0.9, 0.9, 1.0],
            line_number_color: [0.5, 0.5, 0.5, 1.0],
            current_line_color: [0.15, 0.15, 0.15, 1.0],
            error_color: [0.9, 0.3, 0.3, 1.0],
            warning_color: [0.9, 0.7, 0.3, 1.0],
            font_size: 14.0,
            line_height_multiplier: 1.4,
            gutter_width: 5,
            show_line_numbers: true,
            cursor_blink_rate: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_builder() {
        let mut builder = FrameBuilder::new();
        builder
            .background([0.1, 0.1, 0.1, 1.0])
            .fill_rect(0.0, 0.0, 100.0, 100.0, [1.0, 0.0, 0.0, 1.0])
            .text(10.0, 20.0, "Hello", 14.0, [1.0, 1.0, 1.0, 1.0]);

        assert_eq!(builder.commands().len(), 2);
    }

    #[test]
    fn test_render_config_default() {
        let config = RenderConfig::default();
        assert_eq!(config.font_size, 14.0);
        assert!(config.show_line_numbers);
    }

    #[test]
    fn test_frame_builder_clips() {
        let mut builder = FrameBuilder::new();
        builder
            .push_clip(10.0, 10.0, 100.0, 100.0)
            .fill_rect(20.0, 20.0, 50.0, 50.0, [1.0, 1.0, 1.0, 1.0])
            .pop_clip();

        assert_eq!(builder.commands().len(), 3);
    }
}
