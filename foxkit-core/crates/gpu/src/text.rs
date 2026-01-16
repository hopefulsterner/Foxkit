//! GPU-accelerated text rendering

use std::collections::HashMap;
use std::sync::Arc;
use wgpu::*;


/// Glyph identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphId {
    pub codepoint: char,
    pub font_size: u16,
    pub subpixel_bin: u8, // For subpixel positioning
}

/// Cached glyph data
#[derive(Debug, Clone)]
pub struct CachedGlyph {
    /// Position in atlas
    pub atlas_x: u32,
    pub atlas_y: u32,
    /// Size in atlas
    pub width: u32,
    pub height: u32,
    /// Glyph metrics
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance: f32,
}

/// Glyph cache (texture atlas)
pub struct GlyphCache {
    /// Cached glyphs
    glyphs: HashMap<GlyphId, CachedGlyph>,
    /// Atlas texture
    atlas: Option<Texture>,
    /// Atlas texture view
    atlas_view: Option<TextureView>,
    /// Atlas size
    atlas_size: u32,
    /// Current row in atlas
    current_row: u32,
    /// Current x position in row
    current_x: u32,
    /// Current row height
    row_height: u32,
    /// Device reference
    device: Arc<Device>,
    /// Queue reference
    queue: Arc<Queue>,
}

impl GlyphCache {
    /// Create new glyph cache
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        let atlas_size = 2048;

        let atlas = device.create_texture(&TextureDescriptor {
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

        let atlas_view = atlas.create_view(&TextureViewDescriptor::default());

        Self {
            glyphs: HashMap::new(),
            atlas: Some(atlas),
            atlas_view: Some(atlas_view),
            atlas_size,
            current_row: 0,
            current_x: 0,
            row_height: 0,
            device,
            queue,
        }
    }

    /// Get or rasterize a glyph
    pub fn get_or_insert(&mut self, id: GlyphId) -> Option<&CachedGlyph> {
        if !self.glyphs.contains_key(&id) {
            self.rasterize_glyph(id);
        }
        self.glyphs.get(&id)
    }

    /// Rasterize a glyph and add to atlas
    fn rasterize_glyph(&mut self, id: GlyphId) {
        // TODO: Use actual font rasterization (fontdue, ab_glyph, etc.)
        // For now, create a placeholder glyph
        let width = (id.font_size as u32 / 2).max(1);
        let height = id.font_size as u32;

        // Check if we need a new row
        if self.current_x + width > self.atlas_size {
            self.current_row += self.row_height;
            self.current_x = 0;
            self.row_height = 0;
        }

        // Check if atlas is full
        if self.current_row + height > self.atlas_size {
            // Atlas full - would need to resize or evict
            return;
        }

        let glyph = CachedGlyph {
            atlas_x: self.current_x,
            atlas_y: self.current_row,
            width,
            height,
            bearing_x: 0.0,
            bearing_y: height as f32,
            advance: width as f32 + 1.0,
        };

        // Update atlas position
        self.current_x += width + 1; // 1px padding
        self.row_height = self.row_height.max(height + 1);

        self.glyphs.insert(id, glyph);
    }

    /// Get atlas texture view
    pub fn atlas_view(&self) -> Option<&TextureView> {
        self.atlas_view.as_ref()
    }

    /// Clear cache
    pub fn clear(&mut self) {
        self.glyphs.clear();
        self.current_row = 0;
        self.current_x = 0;
        self.row_height = 0;
    }
}

/// Text renderer
pub struct TextRenderer {
    glyph_cache: GlyphCache,
    pipeline: RenderPipeline,
    sampler: Sampler,
}

impl TextRenderer {
    /// Create new text renderer
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, format: TextureFormat) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/text.wgsl").into()),
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Glyph Sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Text Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
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
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        Self {
            glyph_cache: GlyphCache::new(device, queue),
            pipeline,
            sampler,
        }
    }

    /// Measure text dimensions
    pub fn measure(&mut self, text: &str, font_size: f32) -> (f32, f32) {
        let mut width = 0.0f32;
        let height = font_size;

        for c in text.chars() {
            let id = GlyphId {
                codepoint: c,
                font_size: font_size as u16,
                subpixel_bin: 0,
            };

            if let Some(glyph) = self.glyph_cache.get_or_insert(id) {
                width += glyph.advance;
            }
        }

        (width, height)
    }

    /// Get glyph cache (for direct access)
    pub fn glyph_cache(&self) -> &GlyphCache {
        &self.glyph_cache
    }

    /// Get mutable glyph cache
    pub fn glyph_cache_mut(&mut self) -> &mut GlyphCache {
        &mut self.glyph_cache
    }
}

/// Shaped text run
pub struct ShapedRun {
    pub glyphs: Vec<ShapedGlyph>,
    pub width: f32,
    pub height: f32,
}

/// Shaped glyph
pub struct ShapedGlyph {
    pub id: GlyphId,
    pub x: f32,
    pub y: f32,
}
