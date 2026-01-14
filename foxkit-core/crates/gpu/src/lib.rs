//! # Foxkit GPU
//!
//! Hardware-accelerated rendering using wgpu (like Zed).
//! Provides:
//! - GPU text rendering with subpixel antialiasing
//! - Efficient glyph caching
//! - UI primitive rendering
//! - Shader-based effects

pub mod context;
pub mod renderer;
pub mod scene;
pub mod shaders;
pub mod text;
pub mod font;
pub mod editor_bridge;

use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::Result;
use wgpu::*;

pub use context::GpuContext;
pub use renderer::Renderer;
pub use scene::{Scene, Primitive};
pub use text::{TextRenderer, GlyphCache};
pub use font::{FontSystem, FontKey, FontWeight, FontStyle, RasterizedGlyph, GlyphMetrics, FontMetrics, SharedFontSystem};
pub use editor_bridge::{EditorSceneBuilder, EditorRenderCommand, LineStyle as EditorLineStyle};

/// GPU subsystem
pub struct Gpu {
    /// wgpu instance
    instance: Instance,
    /// Adapter
    adapter: Option<Adapter>,
    /// Device
    device: Option<Arc<Device>>,
    /// Queue
    queue: Option<Arc<Queue>>,
    /// Surface format
    surface_format: Option<TextureFormat>,
}

impl Gpu {
    /// Create new GPU subsystem
    pub fn new() -> Self {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            dx12_shader_compiler: Dx12Compiler::Fxc,
            flags: InstanceFlags::default(),
            gles_minor_version: Gles3MinorVersion::Automatic,
        });

        Self {
            instance,
            adapter: None,
            device: None,
            queue: None,
            surface_format: None,
        }
    }

    /// Initialize GPU with a surface
    pub async fn initialize(&mut self, surface: &Surface<'_>) -> Result<()> {
        // Request adapter
        let adapter = self.instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find GPU adapter"))?;

        tracing::info!("GPU: {}", adapter.get_info().name);

        // Request device
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("Foxkit GPU"),
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                },
                None,
            )
            .await?;

        // Get surface format
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        self.adapter = Some(adapter);
        self.device = Some(Arc::new(device));
        self.queue = Some(Arc::new(queue));
        self.surface_format = Some(format);

        Ok(())
    }

    /// Get device
    pub fn device(&self) -> Option<Arc<Device>> {
        self.device.clone()
    }

    /// Get queue
    pub fn queue(&self) -> Option<Arc<Queue>> {
        self.queue.clone()
    }

    /// Get surface format
    pub fn surface_format(&self) -> Option<TextureFormat> {
        self.surface_format
    }

    /// Create a surface for a window
    pub fn create_surface<'a>(&self, window: impl Into<SurfaceTarget<'a>>) -> Result<Surface<'a>> {
        let surface = self.instance.create_surface(window)?;
        Ok(surface)
    }

    /// Configure surface
    pub fn configure_surface(&self, surface: &Surface, width: u32, height: u32) {
        if let (Some(device), Some(format)) = (&self.device, self.surface_format) {
            surface.configure(device, &SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format,
                width,
                height,
                present_mode: PresentMode::Fifo,
                alpha_mode: CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            });
        }
    }

    /// Create a renderer
    pub fn create_renderer(&self) -> Result<Renderer> {
        let device = self.device.clone()
            .ok_or_else(|| anyhow::anyhow!("GPU not initialized"))?;
        let queue = self.queue.clone()
            .ok_or_else(|| anyhow::anyhow!("GPU not initialized"))?;
        let format = self.surface_format
            .ok_or_else(|| anyhow::anyhow!("GPU not initialized"))?;

        Renderer::new(device, queue, format)
    }

    /// Get adapter info
    pub fn adapter_info(&self) -> Option<AdapterInfo> {
        self.adapter.as_ref().map(|a| a.get_info())
    }
}

impl Default for Gpu {
    fn default() -> Self {
        Self::new()
    }
}

/// Color (RGBA, 0.0-1.0)
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as f32 / 255.0,
            g: ((hex >> 8) & 0xFF) as f32 / 255.0,
            b: (hex & 0xFF) as f32 / 255.0,
            a: 1.0,
        }
    }
}

/// 2D Point
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// 2D Size
#[derive(Debug, Clone, Copy, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// Rectangle
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.origin.x
            && point.x <= self.origin.x + self.size.width
            && point.y >= self.origin.y
            && point.y <= self.origin.y + self.size.height
    }
}
