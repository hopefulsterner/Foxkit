//! Window management and event loop
//!
//! This module provides the main application window using winit and wgpu,
//! with real text rendering using the GPU font system.

use std::sync::Arc;
use anyhow::Result;
use parking_lot::RwLock;
use winit::{
    application::ApplicationHandler,
    event::{WindowEvent, ElementState, KeyEvent},
    event_loop::{ActiveEventLoop, EventLoop, ControlFlow},
    window::{Window, WindowId, WindowAttributes},
    keyboard::{Key, NamedKey},
    dpi::PhysicalSize,
};
use wgpu::*;
use gpu::{
    Scene, Primitive, Rect, Point, Size, Color, Renderer,
    FontSystem, FontKey, FontWeight, FontStyle,
    IntegratedTextRenderer, build_text_vertices,
};

/// Application state
pub struct App {
    state: Option<AppState>,
}

/// Initialized app state (after window created)
struct AppState {
    window: Arc<Window>,
    surface: Surface<'static>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    config: SurfaceConfiguration,
    renderer: Renderer,
    
    // Text rendering
    text_renderer: IntegratedTextRenderer,
    font_key: FontKey,
    
    // Editor state
    content: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
    scroll_offset: f32,
    line_height: f32,
    font_size: f32,
    char_width: f32,
    
    // UI state
    show_file_explorer: bool,
    selected_file_index: usize,
    files: Vec<String>,
    
    // Text vertex buffer for rendering
    text_vertex_buffer: Buffer,
    text_index_buffer: Buffer,
}

impl App {
    pub fn new() -> Self {
        Self { state: None }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        // Create window
        let window_attrs = WindowAttributes::default()
            .with_title("🦊 Foxkit IDE")
            .with_inner_size(PhysicalSize::new(1280u32, 720u32))
            .with_min_inner_size(PhysicalSize::new(400u32, 300u32));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create window")
        );

        // Initialize wgpu
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Failed to get adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: Some("Foxkit Device"),
                required_features: Features::empty(),
                required_limits: Limits::default(),
                ..Default::default()
            },
            None,
        ))
        .expect("Failed to create device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let renderer = Renderer::new(device.clone(), queue.clone(), format)
            .expect("Failed to create renderer");

        // Initialize font system and text renderer
        let font_system = Arc::new(RwLock::new(FontSystem::new()));
        let font_key = FontKey::new("DejaVu Sans Mono", FontWeight::Regular, FontStyle::Normal);
        
        // Load system font
        {
            let mut fs = font_system.write();
            match fs.load_font_file(
                font_key.clone(),
                "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
            ) {
                Ok(_) => tracing::info!("✅ Font loaded successfully: DejaVu Sans Mono"),
                Err(e) => tracing::error!("❌ Failed to load font: {}", e),
            }
            fs.set_default_font(font_key.clone());
        }

        let text_renderer = IntegratedTextRenderer::new(
            device.clone(),
            queue.clone(),
            font_system,
            format,
        );

        // Create text vertex/index buffers
        let text_vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Text Vertex Buffer"),
            size: 1024 * 1024, // 1MB
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let text_index_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Text Index Buffer"),
            size: 512 * 1024, // 512KB
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Sample content to display
        let content = vec![
            "// 🦊 Welcome to Foxkit IDE!".to_string(),
            "//".to_string(),
            "// A next-generation monorepo development platform".to_string(),
            "// DNA: Theia (Cloud/Extensions) × Zed (Performance/Collaboration)".to_string(),
            "".to_string(),
            "fn main() {".to_string(),
            "    println!(\"Hello, Foxkit!\");".to_string(),
            "".to_string(),
            "    // GPU-accelerated text rendering".to_string(),
            "    let renderer = TextRenderer::new();".to_string(),
            "".to_string(),
            "    // Real font rendering with fontdue".to_string(),
            "    let font = FontSystem::load(\"DejaVu Sans Mono\");".to_string(),
            "".to_string(),
            "    // File explorer sidebar".to_string(),
            "    let explorer = FileExplorer::new();".to_string(),
            "".to_string(),
            "    // Tab management".to_string(),
            "    let tabs = TabBar::new();".to_string(),
            "}".to_string(),
            "".to_string(),
            "struct Editor {".to_string(),
            "    content: Vec<String>,".to_string(),
            "    cursor: Position,".to_string(),
            "    scroll: ScrollState,".to_string(),
            "}".to_string(),
            "".to_string(),
            "impl Editor {".to_string(),
            "    fn render(&self, scene: &mut Scene) {".to_string(),
            "        // Render editor content".to_string(),
            "        for (i, line) in self.content.iter().enumerate() {".to_string(),
            "            scene.draw_text(line, Point::new(60.0, i as f32 * 20.0));".to_string(),
            "        }".to_string(),
            "    }".to_string(),
            "}".to_string(),
            "".to_string(),
            "// Press arrow keys to move cursor".to_string(),
            "// Press Page Up/Down to scroll".to_string(),
            "// Press Tab to toggle file explorer".to_string(),
            "// Press Escape to quit".to_string(),
        ];

        // Sample file list
        let files = vec![
            "📁 src".to_string(),
            "  📄 main.rs".to_string(),
            "  📄 lib.rs".to_string(),
            "  📁 editor".to_string(),
            "    📄 mod.rs".to_string(),
            "    📄 buffer.rs".to_string(),
            "    📄 cursor.rs".to_string(),
            "  📁 ui".to_string(),
            "    📄 mod.rs".to_string(),
            "    📄 window.rs".to_string(),
            "📄 Cargo.toml".to_string(),
            "📄 README.md".to_string(),
        ];

        let font_size = 14.0;
        let line_height = font_size * 1.5;
        let char_width = font_size * 0.6; // Approximate monospace width

        self.state = Some(AppState {
            window,
            surface,
            device,
            queue,
            config,
            renderer,
            text_renderer,
            font_key,
            content,
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0.0,
            line_height,
            font_size,
            char_width,
            show_file_explorer: true,
            selected_file_index: 0,
            files,
            text_vertex_buffer,
            text_index_buffer,
        });

        tracing::info!("🦊 Foxkit window created with text rendering!");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = &mut self.state else { return };

        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("Window close requested");
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    state.config.width = new_size.width;
                    state.config.height = new_size.height;
                    state.surface.configure(&state.device, &state.config);
                    state.renderer.resize(new_size.width, new_size.height);
                    state.window.request_redraw();
                }
            }

            WindowEvent::KeyboardInput { event: KeyEvent { logical_key, state: key_state, .. }, .. } => {
                if key_state == ElementState::Pressed {
                    match logical_key {
                        Key::Named(NamedKey::Escape) => {
                            event_loop.exit();
                        }
                        Key::Named(NamedKey::Tab) => {
                            state.show_file_explorer = !state.show_file_explorer;
                            state.window.request_redraw();
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            if state.cursor_line > 0 {
                                state.cursor_line -= 1;
                                state.ensure_cursor_visible();
                            }
                            state.window.request_redraw();
                        }
                        Key::Named(NamedKey::ArrowDown) => {
                            if state.cursor_line < state.content.len().saturating_sub(1) {
                                state.cursor_line += 1;
                                state.ensure_cursor_visible();
                            }
                            state.window.request_redraw();
                        }
                        Key::Named(NamedKey::ArrowLeft) => {
                            if state.cursor_col > 0 {
                                state.cursor_col -= 1;
                            }
                            state.window.request_redraw();
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            let line_len = state.content.get(state.cursor_line)
                                .map(|l| l.chars().count())
                                .unwrap_or(0);
                            if state.cursor_col < line_len {
                                state.cursor_col += 1;
                            }
                            state.window.request_redraw();
                        }
                        Key::Named(NamedKey::PageUp) => {
                            let lines_per_page = (state.config.height as f32 / state.line_height) as usize;
                            state.scroll_offset = (state.scroll_offset - lines_per_page as f32 * state.line_height).max(0.0);
                            state.window.request_redraw();
                        }
                        Key::Named(NamedKey::PageDown) => {
                            let max_scroll = (state.content.len() as f32 * state.line_height - state.config.height as f32).max(0.0);
                            let lines_per_page = (state.config.height as f32 / state.line_height) as usize;
                            state.scroll_offset = (state.scroll_offset + lines_per_page as f32 * state.line_height).min(max_scroll);
                            state.window.request_redraw();
                        }
                        Key::Named(NamedKey::Home) => {
                            state.cursor_col = 0;
                            state.window.request_redraw();
                        }
                        Key::Named(NamedKey::End) => {
                            state.cursor_col = state.content.get(state.cursor_line)
                                .map(|l| l.chars().count())
                                .unwrap_or(0);
                            state.window.request_redraw();
                        }
                        _ => {}
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                state.render();
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

impl AppState {
    fn ensure_cursor_visible(&mut self) {
        let title_height = 35.0;
        let cursor_y = self.cursor_line as f32 * self.line_height + title_height;
        let viewport_height = self.config.height as f32 - title_height - 24.0; // minus status bar
        
        if cursor_y < self.scroll_offset + title_height {
            self.scroll_offset = (cursor_y - title_height).max(0.0);
        }
        if cursor_y + self.line_height > self.scroll_offset + viewport_height + title_height {
            self.scroll_offset = cursor_y + self.line_height - viewport_height - title_height;
        }
    }

    fn render(&mut self) {
        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(SurfaceError::OutOfMemory) => {
                tracing::error!("Out of GPU memory!");
                return;
            }
            Err(e) => {
                tracing::warn!("Surface error: {:?}", e);
                return;
            }
        };

        let view = output.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // Build scene (rectangles/backgrounds)
        let scene = self.build_scene();

        // Prepare text renderer with viewport size
        self.text_renderer.prepare(self.config.width, self.config.height);

        // Build text glyphs
        let text_glyphs = self.build_text_glyphs();
        let (text_vertices, text_indices) = build_text_vertices(&text_glyphs, self.text_renderer.atlas_size());
        
        // Upload text geometry
        if !text_vertices.is_empty() {
            self.queue.write_buffer(&self.text_vertex_buffer, 0, bytemuck::cast_slice(&text_vertices));
            self.queue.write_buffer(&self.text_index_buffer, 0, bytemuck::cast_slice(&text_indices));
        }

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: 0.11,
                            g: 0.11,
                            b: 0.13,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Render background primitives
            self.renderer.render(&mut pass, &scene);

            // Render text
            if !text_indices.is_empty() {
                pass.set_pipeline(self.text_renderer.pipeline());
                pass.set_bind_group(0, self.text_renderer.bind_group(), &[]);
                pass.set_vertex_buffer(0, self.text_vertex_buffer.slice(..));
                pass.set_index_buffer(self.text_index_buffer.slice(..), IndexFormat::Uint32);
                pass.draw_indexed(0..text_indices.len() as u32, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn build_text_glyphs(&mut self) -> Vec<gpu::PositionedGlyph> {
        let mut all_glyphs = Vec::new();
        let width = self.config.width as f32;
        let height = self.config.height as f32;
        
        let explorer_width = if self.show_file_explorer { 200.0 } else { 0.0 };
        let gutter_width = 50.0;
        let title_height = 35.0;
        let status_height = 24.0;
        let tab_height = 30.0;
        
        let editor_x = explorer_width + gutter_width + 10.0;
        let editor_y = title_height + tab_height;

        // Title text
        let title_glyphs = self.text_renderer.layout_text(
            "Foxkit IDE",
            Point { x: 10.0, y: 22.0 },
            &self.font_key,
            self.font_size,
            Color { r: 0.9, g: 0.9, b: 0.95, a: 1.0 },
        );
        all_glyphs.extend(title_glyphs);

        // File path in title
        let path_glyphs = self.text_renderer.layout_text(
            "main.rs - foxkit/src",
            Point { x: 120.0, y: 22.0 },
            &self.font_key,
            self.font_size - 2.0,
            Color { r: 0.6, g: 0.6, b: 0.65, a: 1.0 },
        );
        all_glyphs.extend(path_glyphs);

        // Tab text
        let tab_glyphs = self.text_renderer.layout_text(
            "main.rs",
            Point { x: explorer_width + 15.0, y: title_height + 20.0 },
            &self.font_key,
            self.font_size - 1.0,
            Color { r: 0.85, g: 0.85, b: 0.9, a: 1.0 },
        );
        all_glyphs.extend(tab_glyphs);

        // File explorer
        if self.show_file_explorer {
            let explorer_header = self.text_renderer.layout_text(
                "EXPLORER",
                Point { x: 10.0, y: title_height + 18.0 },
                &self.font_key,
                self.font_size - 3.0,
                Color { r: 0.5, g: 0.5, b: 0.55, a: 1.0 },
            );
            all_glyphs.extend(explorer_header);

            for (i, file) in self.files.iter().enumerate() {
                let y = title_height + 40.0 + i as f32 * 22.0;
                if y > height - status_height {
                    break;
                }
                let color = if i == self.selected_file_index {
                    Color { r: 0.9, g: 0.9, b: 0.95, a: 1.0 }
                } else {
                    Color { r: 0.7, g: 0.7, b: 0.75, a: 1.0 }
                };
                let file_glyphs = self.text_renderer.layout_text(
                    file,
                    Point { x: 10.0, y },
                    &self.font_key,
                    self.font_size - 1.0,
                    color,
                );
                all_glyphs.extend(file_glyphs);
            }
        }

        // Line numbers and content
        let first_line = (self.scroll_offset / self.line_height) as usize;
        let visible_lines = ((height - editor_y - status_height) / self.line_height) as usize + 2;

        for i in 0..visible_lines {
            let line_idx = first_line + i;
            if line_idx >= self.content.len() {
                break;
            }

            let y = editor_y + i as f32 * self.line_height - (self.scroll_offset % self.line_height) + self.line_height * 0.75;

            // Line number
            let line_num = format!("{:>4}", line_idx + 1);
            let line_num_glyphs = self.text_renderer.layout_text(
                &line_num,
                Point { x: explorer_width + 5.0, y },
                &self.font_key,
                self.font_size - 1.0,
                Color { r: 0.45, g: 0.45, b: 0.5, a: 1.0 },
            );
            all_glyphs.extend(line_num_glyphs);

            // Line content with syntax highlighting
            let line = &self.content[line_idx];
            let tokens = self.tokenize_line(line);
            let mut x = editor_x;

            for (text, color) in tokens {
                let token_glyphs = self.text_renderer.layout_text(
                    &text,
                    Point { x, y },
                    &self.font_key,
                    self.font_size,
                    color,
                );
                // Advance x based on text width
                x += text.chars().count() as f32 * self.char_width;
                all_glyphs.extend(token_glyphs);
            }
        }

        // Status bar text
        let status_text = format!(
            "Ln {}, Col {}  |  UTF-8  |  Rust  |  {} lines",
            self.cursor_line + 1,
            self.cursor_col + 1,
            self.content.len()
        );
        let status_glyphs = self.text_renderer.layout_text(
            &status_text,
            Point { x: width - 300.0, y: height - 8.0 },
            &self.font_key,
            self.font_size - 2.0,
            Color { r: 0.6, g: 0.6, b: 0.65, a: 1.0 },
        );
        all_glyphs.extend(status_glyphs);

        // Mode indicator
        let mode_glyphs = self.text_renderer.layout_text(
            "NORMAL",
            Point { x: 10.0, y: height - 8.0 },
            &self.font_key,
            self.font_size - 2.0,
            Color { r: 0.4, g: 0.7, b: 0.4, a: 1.0 },
        );
        all_glyphs.extend(mode_glyphs);

        all_glyphs
    }

    fn tokenize_line(&self, line: &str) -> Vec<(String, Color)> {
        let mut tokens = Vec::new();
        let trimmed = line.trim_start();
        let leading_spaces = line.len() - trimmed.len();
        
        if leading_spaces > 0 {
            tokens.push((" ".repeat(leading_spaces), Color { r: 0.85, g: 0.85, b: 0.9, a: 1.0 }));
        }

        let comment_color = Color { r: 0.45, g: 0.6, b: 0.45, a: 1.0 };
        let keyword_color = Color { r: 0.7, g: 0.5, b: 0.85, a: 1.0 };
        let string_color = Color { r: 0.85, g: 0.65, b: 0.45, a: 1.0 };
        let function_color = Color { r: 0.55, g: 0.75, b: 0.95, a: 1.0 };
        let type_color = Color { r: 0.45, g: 0.8, b: 0.7, a: 1.0 };
        let default_color = Color { r: 0.85, g: 0.85, b: 0.9, a: 1.0 };

        if trimmed.starts_with("//") {
            tokens.push((trimmed.to_string(), comment_color));
        } else {
            // Very simple tokenization
            let mut remaining = trimmed;
            while !remaining.is_empty() {
                // Check for string
                if remaining.starts_with('"') {
                    if let Some(end) = remaining[1..].find('"') {
                        let s = &remaining[..end + 2];
                        tokens.push((s.to_string(), string_color));
                        remaining = &remaining[end + 2..];
                        continue;
                    }
                }

                // Check for keywords
                let keywords = ["fn", "let", "mut", "pub", "struct", "impl", "for", "in", "if", "else", "return", "use", "mod", "self", "Self"];
                let mut found_keyword = false;
                for kw in keywords {
                    if remaining.starts_with(kw) {
                        let next_char = remaining.chars().nth(kw.len());
                        if next_char.map(|c| !c.is_alphanumeric() && c != '_').unwrap_or(true) {
                            tokens.push((kw.to_string(), keyword_color));
                            remaining = &remaining[kw.len()..];
                            found_keyword = true;
                            break;
                        }
                    }
                }
                if found_keyword {
                    continue;
                }

                // Check for types (capitalized words)
                if remaining.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    let end = remaining.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(remaining.len());
                    tokens.push((remaining[..end].to_string(), type_color));
                    remaining = &remaining[end..];
                    continue;
                }

                // Check for function calls (word followed by `(`)
                if remaining.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) {
                    let end = remaining.find(|c: char| !c.is_alphanumeric() && c != '_' && c != '!').unwrap_or(remaining.len());
                    let word = &remaining[..end];
                    let is_fn = remaining[end..].starts_with('(') || word.ends_with('!');
                    if is_fn {
                        tokens.push((word.to_string(), function_color));
                    } else {
                        tokens.push((word.to_string(), default_color));
                    }
                    remaining = &remaining[end..];
                    continue;
                }

                // Default: take one character
                let ch = remaining.chars().next().unwrap();
                tokens.push((ch.to_string(), default_color));
                remaining = &remaining[ch.len_utf8()..];
            }
        }

        tokens
    }

    fn build_scene(&self) -> Scene {
        let mut scene = Scene::new();
        
        let width = self.config.width as f32;
        let height = self.config.height as f32;
        let explorer_width = if self.show_file_explorer { 200.0 } else { 0.0 };
        let gutter_width = 50.0;
        let title_height = 35.0;
        let status_height = 24.0;
        let tab_height = 30.0;

        // Title bar background
        scene.add(Primitive::Quad {
            rect: Rect {
                origin: Point { x: 0.0, y: 0.0 },
                size: Size { width, height: title_height },
            },
            color: Color { r: 0.15, g: 0.15, b: 0.18, a: 1.0 },
            corner_radius: 0.0,
        });

        // File explorer background
        if self.show_file_explorer {
            scene.add(Primitive::Quad {
                rect: Rect {
                    origin: Point { x: 0.0, y: title_height },
                    size: Size { width: explorer_width, height: height - title_height - status_height },
                },
                color: Color { r: 0.13, g: 0.13, b: 0.15, a: 1.0 },
                corner_radius: 0.0,
            });

            // Selected file highlight
            let selected_y = title_height + 30.0 + self.selected_file_index as f32 * 22.0;
            scene.add(Primitive::Quad {
                rect: Rect {
                    origin: Point { x: 0.0, y: selected_y },
                    size: Size { width: explorer_width, height: 22.0 },
                },
                color: Color { r: 0.2, g: 0.2, b: 0.25, a: 1.0 },
                corner_radius: 0.0,
            });

            // Explorer/editor separator
            scene.add(Primitive::Quad {
                rect: Rect {
                    origin: Point { x: explorer_width - 1.0, y: title_height },
                    size: Size { width: 1.0, height: height - title_height - status_height },
                },
                color: Color { r: 0.25, g: 0.25, b: 0.28, a: 1.0 },
                corner_radius: 0.0,
            });
        }

        // Tab bar background
        scene.add(Primitive::Quad {
            rect: Rect {
                origin: Point { x: explorer_width, y: title_height },
                size: Size { width: width - explorer_width, height: tab_height },
            },
            color: Color { r: 0.12, g: 0.12, b: 0.14, a: 1.0 },
            corner_radius: 0.0,
        });

        // Active tab
        scene.add(Primitive::Quad {
            rect: Rect {
                origin: Point { x: explorer_width, y: title_height },
                size: Size { width: 120.0, height: tab_height },
            },
            color: Color { r: 0.18, g: 0.18, b: 0.22, a: 1.0 },
            corner_radius: 0.0,
        });

        // Tab bottom border
        scene.add(Primitive::Quad {
            rect: Rect {
                origin: Point { x: explorer_width, y: title_height + tab_height - 2.0 },
                size: Size { width: 120.0, height: 2.0 },
            },
            color: Color { r: 0.4, g: 0.6, b: 0.9, a: 1.0 },
            corner_radius: 0.0,
        });

        // Gutter background
        scene.add(Primitive::Quad {
            rect: Rect {
                origin: Point { x: explorer_width, y: title_height + tab_height },
                size: Size { width: gutter_width, height: height - title_height - tab_height - status_height },
            },
            color: Color { r: 0.13, g: 0.13, b: 0.15, a: 1.0 },
            corner_radius: 0.0,
        });

        // Current line highlight
        let editor_y = title_height + tab_height;
        let first_line = (self.scroll_offset / self.line_height) as usize;
        
        if self.cursor_line >= first_line {
            let relative_line = self.cursor_line - first_line;
            let cursor_y = editor_y + relative_line as f32 * self.line_height - (self.scroll_offset % self.line_height);
            
            if cursor_y >= editor_y && cursor_y < height - status_height {
                scene.add(Primitive::Quad {
                    rect: Rect {
                        origin: Point { x: explorer_width + gutter_width, y: cursor_y },
                        size: Size { width: width - explorer_width - gutter_width - 12.0, height: self.line_height },
                    },
                    color: Color { r: 0.16, g: 0.16, b: 0.2, a: 1.0 },
                    corner_radius: 0.0,
                });

                // Cursor
                let cursor_x = explorer_width + gutter_width + 10.0 + self.cursor_col as f32 * self.char_width;
                scene.add(Primitive::Quad {
                    rect: Rect {
                        origin: Point { x: cursor_x, y: cursor_y + 2.0 },
                        size: Size { width: 2.0, height: self.line_height - 4.0 },
                    },
                    color: Color { r: 0.9, g: 0.9, b: 0.95, a: 1.0 },
                    corner_radius: 1.0,
                });
            }
        }

        // Scrollbar track
        let scrollbar_x = width - 12.0;
        scene.add(Primitive::Quad {
            rect: Rect {
                origin: Point { x: scrollbar_x, y: editor_y },
                size: Size { width: 10.0, height: height - editor_y - status_height },
            },
            color: Color { r: 0.13, g: 0.13, b: 0.15, a: 1.0 },
            corner_radius: 5.0,
        });

        // Scrollbar thumb
        let total_content_height = self.content.len() as f32 * self.line_height;
        let viewport_height = height - editor_y - status_height;
        if total_content_height > viewport_height {
            let thumb_ratio = viewport_height / total_content_height;
            let thumb_height = (viewport_height * thumb_ratio).max(30.0);
            let scroll_ratio = self.scroll_offset / (total_content_height - viewport_height);
            let thumb_y = editor_y + scroll_ratio * (viewport_height - thumb_height);

            scene.add(Primitive::Quad {
                rect: Rect {
                    origin: Point { x: scrollbar_x + 2.0, y: thumb_y },
                    size: Size { width: 6.0, height: thumb_height },
                },
                color: Color { r: 0.35, g: 0.35, b: 0.4, a: 1.0 },
                corner_radius: 3.0,
            });
        }

        // Status bar background
        scene.add(Primitive::Quad {
            rect: Rect {
                origin: Point { x: 0.0, y: height - status_height },
                size: Size { width, height: status_height },
            },
            color: Color { r: 0.15, g: 0.15, b: 0.18, a: 1.0 },
            corner_radius: 0.0,
        });

        // Status bar left accent
        scene.add(Primitive::Quad {
            rect: Rect {
                origin: Point { x: 0.0, y: height - status_height },
                size: Size { width: 80.0, height: status_height },
            },
            color: Color { r: 0.25, g: 0.5, b: 0.35, a: 1.0 },
            corner_radius: 0.0,
        });

        scene
    }
}

/// Run the Foxkit application
pub fn run() -> Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    
    Ok(())
}
