//! GPU context for a window/surface

use std::sync::Arc;
use anyhow::Result;
use wgpu::*;

use crate::{Renderer, Size};

/// GPU context for a specific window
pub struct GpuContext {
    /// wgpu device
    device: Arc<Device>,
    /// Command queue
    queue: Arc<Queue>,
    /// Surface
    surface: Surface<'static>,
    /// Surface configuration
    config: SurfaceConfiguration,
    /// Renderer
    renderer: Renderer,
    /// Current size
    size: Size,
}

impl GpuContext {
    /// Create new GPU context
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        surface: Surface<'static>,
        format: TextureFormat,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let renderer = Renderer::new(Arc::clone(&device), Arc::clone(&queue), format)?;

        Ok(Self {
            device,
            queue,
            surface,
            config,
            renderer,
            size: Size::new(width as f32, height as f32),
        })
    }

    /// Resize the surface
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.size = Size::new(width as f32, height as f32);
            self.renderer.resize(width, height);
        }
    }

    /// Get current size
    pub fn size(&self) -> Size {
        self.size
    }

    /// Begin a frame
    pub fn begin_frame(&mut self) -> Result<Frame> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&TextureViewDescriptor::default());

        Ok(Frame {
            output,
            view,
            encoder: self.device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Frame Encoder"),
            }),
        })
    }

    /// End frame and present
    pub fn end_frame(&self, frame: Frame) {
        self.queue.submit(std::iter::once(frame.encoder.finish()));
        frame.output.present();
    }

    /// Get renderer
    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    /// Get mutable renderer
    pub fn renderer_mut(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    /// Get device
    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }

    /// Get queue
    pub fn queue(&self) -> &Arc<Queue> {
        &self.queue
    }
}

/// Active frame
pub struct Frame {
    output: SurfaceTexture,
    pub view: TextureView,
    pub encoder: CommandEncoder,
}

impl Frame {
    /// Create a render pass
    pub fn begin_render_pass<'a>(&'a mut self, clear_color: crate::Color) -> RenderPass<'a> {
        self.encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &self.view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color {
                        r: clear_color.r as f64,
                        g: clear_color.g as f64,
                        b: clear_color.b as f64,
                        a: clear_color.a as f64,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
}
