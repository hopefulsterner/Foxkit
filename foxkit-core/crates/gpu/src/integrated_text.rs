//! Integrated text renderer with FontSystem
//!
//! Connects the FontSystem (fontdue-based glyph rasterization) with the GPU text rendering pipeline.

use std::collections::HashMap;
use std::sync::Arc;
use wgpu::*;
use parking_lot::RwLock;
use bytemuck;

use crate::{Color, Point, FontSystem, FontKey, FontMetrics};

/// Glyph identifier for caching
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlyphCacheKey {
    pub codepoint: char,
    pub font_key: FontKey,
    pub size_px: u32, // Size * 10 for sub-pixel precision
}

/// Cached glyph in the atlas
#[derive(Debug, Clone)]
pub struct AtlasGlyph {
    /// Position in atlas texture
    pub atlas_x: u32,
    pub atlas_y: u32,
    /// Size in atlas
    pub width: u32,
    pub height: u32,
    /// Glyph metrics for positioning
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance_width: f32,
}

/// Atlas packing state
struct AtlasPacker {
    /// Current row Y position
    row_y: u32,
    /// Current X position in row
    row_x: u32,
    /// Current row height
    row_height: u32,
    /// Atlas size
    atlas_size: u32,
}

impl AtlasPacker {
    fn new(atlas_size: u32) -> Self {
        Self {
            row_y: 0,
            row_x: 0,
            row_height: 0,
            atlas_size,
        }
    }

    /// Allocate space for a glyph, returns (x, y) or None if full
    fn allocate(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        // Add padding
        let padded_width = width + 1;
        let padded_height = height + 1;

        // Check if we need to move to next row
        if self.row_x + padded_width > self.atlas_size {
            self.row_y += self.row_height;
            self.row_x = 0;
            self.row_height = 0;
        }

        // Check if atlas is full
        if self.row_y + padded_height > self.atlas_size {
            return None;
        }

        let result = (self.row_x, self.row_y);
        self.row_x += padded_width;
        self.row_height = self.row_height.max(padded_height);

        Some(result)
    }

    fn reset(&mut self) {
        self.row_y = 0;
        self.row_x = 0;
        self.row_height = 0;
    }
}

/// Integrated text renderer using FontSystem for rasterization
pub struct IntegratedTextRenderer {
    /// Font system for rasterization
    font_system: Arc<RwLock<FontSystem>>,
    /// Glyph cache mapping
    glyph_cache: HashMap<GlyphCacheKey, AtlasGlyph>,
    /// Atlas texture
    atlas_texture: Texture,
    /// Atlas texture view
    atlas_view: TextureView,
    /// Atlas sampler
    sampler: Sampler,
    /// Atlas bind group layout
    bind_group_layout: BindGroupLayout,
    /// Atlas bind group
    bind_group: BindGroup,
    /// Uniform buffer for viewport
    uniform_buffer: Buffer,
    /// Atlas packer
    packer: AtlasPacker,
    /// Atlas size
    atlas_size: u32,
    /// Atlas CPU-side data for updates
    atlas_data: Vec<u8>,
    /// Queue for GPU uploads
    queue: Arc<Queue>,
    /// Device reference
    device: Arc<Device>,
    /// Render pipeline
    pipeline: RenderPipeline,
    /// Default font key
    default_font: FontKey,
    /// Current viewport size
    viewport_size: [f32; 2],
}

impl IntegratedTextRenderer {
    /// Create a new integrated text renderer
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        font_system: Arc<RwLock<FontSystem>>,
        format: TextureFormat,
    ) -> Self {
        let atlas_size = 2048u32;

        // Create atlas texture
        let atlas_texture = device.create_texture(&TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let atlas_view = atlas_texture.create_view(&TextureViewDescriptor::default());

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Glyph Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        // Create uniform buffer for viewport
        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Text Uniform Buffer"),
            size: 16, // vec2<f32> viewport + vec2<f32> padding
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Text Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Text Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&atlas_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Create shader and pipeline
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Integrated Text Shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/text.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Text Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[TextVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        Self {
            font_system,
            glyph_cache: HashMap::new(),
            atlas_texture,
            atlas_view,
            sampler,
            bind_group_layout,
            bind_group,
            uniform_buffer,
            packer: AtlasPacker::new(atlas_size),
            atlas_size,
            atlas_data: vec![0u8; (atlas_size * atlas_size) as usize],
            queue,
            device,
            pipeline,
            default_font: FontKey::regular("monospace"),
            viewport_size: [1280.0, 720.0],
        }
    }

    /// Set the default font
    pub fn set_default_font(&mut self, font_key: FontKey) {
        self.default_font = font_key;
    }

    /// Get or cache a glyph
    pub fn get_glyph(&mut self, ch: char, font_key: &FontKey, size_px: f32) -> Option<AtlasGlyph> {
        let cache_key = GlyphCacheKey {
            codepoint: ch,
            font_key: font_key.clone(),
            size_px: (size_px * 10.0) as u32,
        };

        // Check cache
        if let Some(cached) = self.glyph_cache.get(&cache_key) {
            return Some(cached.clone());
        }

        // Rasterize using FontSystem
        let rasterized = {
            let mut font_system = self.font_system.write();
            font_system.rasterize_glyph(font_key, ch, size_px)
        };

        let rasterized = rasterized?;

        // Allocate space in atlas
        let (atlas_x, atlas_y) = self.packer.allocate(rasterized.width, rasterized.height)?;

        // Copy bitmap to atlas data
        for y in 0..rasterized.height {
            for x in 0..rasterized.width {
                let src_idx = (y * rasterized.width + x) as usize;
                let dst_idx = ((atlas_y + y) * self.atlas_size + atlas_x + x) as usize;
                if src_idx < rasterized.bitmap.len() && dst_idx < self.atlas_data.len() {
                    self.atlas_data[dst_idx] = rasterized.bitmap[src_idx];
                }
            }
        }

        // Upload to GPU
        self.queue.write_texture(
            ImageCopyTexture {
                texture: &self.atlas_texture,
                mip_level: 0,
                origin: Origin3d {
                    x: atlas_x,
                    y: atlas_y,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            &rasterized.bitmap,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(rasterized.width),
                rows_per_image: Some(rasterized.height),
            },
            Extent3d {
                width: rasterized.width,
                height: rasterized.height,
                depth_or_array_layers: 1,
            },
        );

        let atlas_glyph = AtlasGlyph {
            atlas_x,
            atlas_y,
            width: rasterized.width,
            height: rasterized.height,
            bearing_x: rasterized.metrics.bearing_x,
            bearing_y: rasterized.metrics.bearing_y,
            advance_width: rasterized.metrics.advance_width,
        };

        self.glyph_cache.insert(cache_key, atlas_glyph.clone());
        Some(atlas_glyph)
    }

    /// Layout and render text, returning positioned glyphs
    pub fn layout_text(
        &mut self,
        text: &str,
        position: Point,
        font_key: &FontKey,
        size_px: f32,
        color: Color,
    ) -> Vec<PositionedGlyph> {
        let mut glyphs = Vec::new();
        let mut x = position.x;
        let y = position.y;

        for ch in text.chars() {
            if let Some(glyph) = self.get_glyph(ch, font_key, size_px) {
                // Position glyph: x + bearing, y adjusted for baseline
                // bearing_y is the offset from baseline to top of glyph
                let glyph_y = y - glyph.bearing_y + size_px;
                glyphs.push(PositionedGlyph {
                    x: x + glyph.bearing_x,
                    y: glyph_y,
                    width: glyph.width as f32,
                    height: glyph.height as f32,
                    atlas_x: glyph.atlas_x,
                    atlas_y: glyph.atlas_y,
                    atlas_width: glyph.width,
                    atlas_height: glyph.height,
                    color,
                });
                x += glyph.advance_width;
            } else if ch == ' ' {
                // Space - advance by estimated width
                x += size_px * 0.5;
            } else if ch == '\t' {
                // Tab - advance by 4 spaces
                x += size_px * 2.0;
            }
        }

        glyphs
    }

    /// Measure text width
    pub fn measure_text(&mut self, text: &str, font_key: &FontKey, size_px: f32) -> f32 {
        let mut width = 0.0;

        for ch in text.chars() {
            if let Some(glyph) = self.get_glyph(ch, font_key, size_px) {
                width += glyph.advance_width;
            } else if ch == ' ' {
                width += size_px * 0.5;
            } else if ch == '\t' {
                width += size_px * 2.0;
            }
        }

        width
    }

    /// Get font metrics for a font at a specific size
    pub fn font_metrics(&self, font_key: &FontKey, size_px: f32) -> Option<FontMetrics> {
        let font_system = self.font_system.read();
        font_system.font_metrics(font_key, size_px)
    }

    /// Clear the glyph cache (e.g., on font change)
    pub fn clear_cache(&mut self) {
        self.glyph_cache.clear();
        self.packer.reset();
        self.atlas_data.fill(0);
    }

    /// Get the bind group for rendering
    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    /// Get the render pipeline
    pub fn pipeline(&self) -> &RenderPipeline {
        &self.pipeline
    }

    /// Get atlas size for UV calculations
    pub fn atlas_size(&self) -> f32 {
        self.atlas_size as f32
    }

    /// Set viewport size and update uniform buffer
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport_size = [width, height];
        let uniforms: [f32; 4] = [width, height, 0.0, 0.0];
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));
    }

    /// Prepare for rendering (call before draw)
    pub fn prepare(&mut self, width: u32, height: u32) {
        self.set_viewport(width as f32, height as f32);
    }
}

/// A glyph positioned for rendering
#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub atlas_x: u32,
    pub atlas_y: u32,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub color: Color,
}

/// Vertex for text rendering
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct TextVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl TextVertex {
    const ATTRIBS: [VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
    ];

    pub fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<TextVertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Build vertex data for positioned glyphs
pub fn build_text_vertices(
    glyphs: &[PositionedGlyph],
    atlas_size: f32,
) -> (Vec<TextVertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(glyphs.len() * 4);
    let mut indices = Vec::with_capacity(glyphs.len() * 6);

    for glyph in glyphs {
        let base = vertices.len() as u32;

        let x0 = glyph.x;
        let y0 = glyph.y;
        let x1 = glyph.x + glyph.width;
        let y1 = glyph.y + glyph.height;

        let u0 = glyph.atlas_x as f32 / atlas_size;
        let v0 = glyph.atlas_y as f32 / atlas_size;
        let u1 = (glyph.atlas_x + glyph.atlas_width) as f32 / atlas_size;
        let v1 = (glyph.atlas_y + glyph.atlas_height) as f32 / atlas_size;

        let color = [glyph.color.r, glyph.color.g, glyph.color.b, glyph.color.a];

        vertices.push(TextVertex { position: [x0, y0], uv: [u0, v0], color });
        vertices.push(TextVertex { position: [x1, y0], uv: [u1, v0], color });
        vertices.push(TextVertex { position: [x1, y1], uv: [u1, v1], color });
        vertices.push(TextVertex { position: [x0, y1], uv: [u0, v1], color });

        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    (vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlas_packer() {
        let mut packer = AtlasPacker::new(256);
        
        // First allocation
        let pos1 = packer.allocate(32, 32);
        assert_eq!(pos1, Some((0, 0)));

        // Second allocation in same row
        let pos2 = packer.allocate(32, 32);
        assert_eq!(pos2, Some((33, 0)));

        // Third allocation should still fit in first row
        let pos3 = packer.allocate(100, 50);
        assert_eq!(pos3, Some((66, 0)));
    }

    #[test]
    fn test_atlas_packer_new_row() {
        let mut packer = AtlasPacker::new(100);
        
        // Fill first row
        packer.allocate(50, 30);
        packer.allocate(40, 30);

        // This should go to next row
        let pos = packer.allocate(60, 20);
        assert_eq!(pos, Some((0, 31)));
    }

    #[test]
    fn test_glyph_cache_key() {
        let key1 = GlyphCacheKey {
            codepoint: 'A',
            font_key: FontKey::regular("mono"),
            size_px: 140,
        };
        let key2 = GlyphCacheKey {
            codepoint: 'A',
            font_key: FontKey::regular("mono"),
            size_px: 140,
        };
        assert_eq!(key1, key2);
    }
}
