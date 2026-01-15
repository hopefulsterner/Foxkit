//! GPU renderer - draws primitives

use std::sync::Arc;
use anyhow::Result;
use wgpu::*;
use bytemuck::{Pod, Zeroable};

use crate::{Color, Point, Rect, Scene, Primitive};

/// Vertex for 2D rendering
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x4,
        2 => Float32x2,
    ];

    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// GPU renderer
pub struct Renderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    uniform_buffer: Buffer,
    uniform_bind_group: BindGroup,
    viewport_size: [f32; 2],
    max_vertices: usize,
    max_indices: usize,
}

/// Uniforms
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Uniforms {
    viewport: [f32; 2],
    _padding: [f32; 2],
}

impl Renderer {
    /// Create new renderer
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, format: TextureFormat) -> Result<Self> {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Foxkit Shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/quad.wgsl").into()),
        });

        // Uniform buffer
        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
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
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        let max_vertices = 65536;
        let max_indices = 65536 * 6;

        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (max_vertices * std::mem::size_of::<Vertex>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Index Buffer"),
            size: (max_indices * std::mem::size_of::<u32>()) as u64,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            device,
            queue,
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_bind_group,
            viewport_size: [800.0, 600.0],
            max_vertices,
            max_indices,
        })
    }

    /// Resize viewport
    pub fn resize(&mut self, width: u32, height: u32) {
        self.viewport_size = [width as f32, height as f32];
    }

    /// Render a scene
    pub fn render<'a>(&'a self, pass: &mut RenderPass<'a>, scene: &Scene) {
        // Update uniforms
        let uniforms = Uniforms {
            viewport: self.viewport_size,
            _padding: [0.0; 2],
        };
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Build geometry
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for primitive in scene.primitives() {
            self.tesselate_primitive(primitive, &mut vertices, &mut indices);
        }

        if vertices.is_empty() {
            return;
        }

        // Upload geometry
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        self.queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));

        // Draw
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint32);
        pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
    }

    fn tesselate_primitive(&self, primitive: &Primitive, vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>) {
        match primitive {
            Primitive::Quad { rect, color, corner_radius } => {
                self.tesselate_quad(rect, *color, *corner_radius, vertices, indices);
            }
            Primitive::Line { start, end, color, width } => {
                self.tesselate_line(*start, *end, *color, *width, vertices, indices);
            }
            Primitive::Text { .. } => {
                // Text handled by TextRenderer
            }
        }
    }

    fn tesselate_quad(&self, rect: &Rect, color: Color, _corner_radius: f32, vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>) {
        let base = vertices.len() as u32;
        let x = rect.origin.x;
        let y = rect.origin.y;
        let w = rect.size.width;
        let h = rect.size.height;
        let c = [color.r, color.g, color.b, color.a];

        vertices.push(Vertex { position: [x, y], color: c, uv: [0.0, 0.0] });
        vertices.push(Vertex { position: [x + w, y], color: c, uv: [1.0, 0.0] });
        vertices.push(Vertex { position: [x + w, y + h], color: c, uv: [1.0, 1.0] });
        vertices.push(Vertex { position: [x, y + h], color: c, uv: [0.0, 1.0] });

        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn tesselate_line(&self, start: Point, end: Point, color: Color, width: f32, vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>) {
        let base = vertices.len() as u32;
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            return;
        }

        let nx = -dy / len * width * 0.5;
        let ny = dx / len * width * 0.5;
        let c = [color.r, color.g, color.b, color.a];

        vertices.push(Vertex { position: [start.x + nx, start.y + ny], color: c, uv: [0.0, 0.0] });
        vertices.push(Vertex { position: [start.x - nx, start.y - ny], color: c, uv: [0.0, 1.0] });
        vertices.push(Vertex { position: [end.x - nx, end.y - ny], color: c, uv: [1.0, 1.0] });
        vertices.push(Vertex { position: [end.x + nx, end.y + ny], color: c, uv: [1.0, 0.0] });

        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}
