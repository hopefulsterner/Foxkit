//! Window management and event loop
//!
//! This module provides the main application window using winit and wgpu,
//! with REAL interactive text editing like VS Code.
use std::sync::Arc;
use std::path::PathBuf;
use std::fs;
use anyhow::Result;
use parking_lot::RwLock;
use winit::{
    application::ApplicationHandler,
    event::{WindowEvent, ElementState, KeyEvent, MouseButton, MouseScrollDelta},
    event_loop::{ActiveEventLoop, EventLoop, ControlFlow},
    window::{Window, WindowId, WindowAttributes},
    keyboard::{Key, NamedKey, ModifiersState},
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
/// Editor mode (like Vim but simpler)
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EditorMode {
    Normal,
    Insert,
}
/// Selection range
#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}
impl Selection {
    pub fn is_empty(&self) -> bool {
        self.start_line == self.end_line && self.start_col == self.end_col
    }
    
    /// Get normalized selection (start before end)
    pub fn normalized(&self) -> (usize, usize, usize, usize) {
        if self.start_line < self.end_line || 
           (self.start_line == self.end_line && self.start_col <= self.end_col) {
            (self.start_line, self.start_col, self.end_line, self.end_col)
        } else {
            (self.end_line, self.end_col, self.start_line, self.start_col)
        }
    }
}
/// Undo/Redo action
#[derive(Clone, Debug)]
pub enum EditAction {
    Insert { line: usize, col: usize, text: String },
    Delete { line: usize, col: usize, text: String },
    InsertLine { line: usize },
    DeleteLine { line: usize, content: String },
}
/// Panel types for the IDE
#[derive(Clone, Debug, PartialEq)]
pub enum PanelType {
    Editor,
    FileExplorer,
    Terminal,
    Output,
    AIChat,
}
/// A panel in the IDE layout
#[derive(Clone, Debug)]
pub struct Panel {
    pub panel_type: PanelType,
    pub rect: Rect,
    pub is_visible: bool,
    pub title: String,
}
/// Panel layout manager
#[derive(Clone, Debug)]
pub struct PanelLayout {
    pub panels: Vec<Panel>,
    pub splitters: Vec<Splitter>,
}
/// A resizable splitter between panels
#[derive(Clone, Debug)]
pub struct Splitter {
    pub rect: Rect,
    pub orientation: SplitterOrientation,
    pub is_dragging: bool,
    pub drag_start: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub enum SplitterOrientation {
    Horizontal,
    Vertical,
}
pub struct AppState {
    window: Arc<Window>,
    surface: Surface<'static>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    config: SurfaceConfiguration,
    renderer: Renderer,
    
    // Text rendering
    text_renderer: IntegratedTextRenderer,
    font_key: FontKey,
    
    // Editor state - REAL EDITING
    content: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
    scroll_offset: f32,
    line_height: f32,
    font_size: f32,
    char_width: f32,
    
    // Editor mode
    mode: EditorMode,
    
    // Selection
    selection: Option<Selection>,
    is_selecting: bool,
    
    // Clipboard
    clipboard: String,
    
    // Undo/Redo
    undo_stack: Vec<Vec<EditAction>>,
    redo_stack: Vec<Vec<EditAction>>,
    current_action_group: Vec<EditAction>,
    
    // File management
    current_file: Option<PathBuf>,
    is_modified: bool,
    
    // Keyboard modifiers
    modifiers: ModifiersState,
    
    // UI state
    show_file_explorer: bool,
    selected_file_index: usize,
    files: Vec<(String, PathBuf, bool)>, // (display_name, path, is_dir)
    current_dir: PathBuf,
    
    // Text vertex buffer for rendering
    text_vertex_buffer: Buffer,
    text_index_buffer: Buffer,
    
    // Cursor blink timer
    cursor_visible: bool,
    last_cursor_toggle: std::time::Instant,
    
    // Mouse state
    mouse_x: f32,
    mouse_y: f32,
    
    // Panel system
    panel_layout: PanelLayout,
    active_panel: Option<PanelType>,
    dragging_splitter: Option<usize>,
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
            .with_title("Foxkit IDE")
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
        // Try to load a real file, fall back to welcome content
        let (content, current_file) = load_initial_file();
        
        // Get current directory for file explorer
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let files = scan_directory(&current_dir);
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
            mode: EditorMode::Insert, // Start in insert mode for easy editing
            selection: None,
            is_selecting: false,
            clipboard: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_action_group: Vec::new(),
            current_file,
            is_modified: false,
            modifiers: ModifiersState::empty(),
            show_file_explorer: true,
            selected_file_index: 0,
            files,
            current_dir,
            text_vertex_buffer,
            text_index_buffer,
            cursor_visible: true,
            last_cursor_toggle: std::time::Instant::now(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            panel_layout: AppState::create_default_panel_layout(size.width as f32, size.height as f32),
            active_panel: Some(PanelType::Editor),
            dragging_splitter: None,
        });
        tracing::info!("🦊 Foxkit - Real Interactive Editor Ready!");
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
            WindowEvent::ModifiersChanged(mods) => {
                state.modifiers = mods.state();
            }
            WindowEvent::MouseInput { state: button_state, button, .. } => {
                if button == MouseButton::Left {
                    if button_state == ElementState::Pressed {
                        state.is_selecting = true;
                        // Position cursor at click position
                        state.position_cursor_at_mouse();
                        state.selection = Some(Selection {
                            start_line: state.cursor_line,
                            start_col: state.cursor_col,
                            end_line: state.cursor_line,
                            end_col: state.cursor_col,
                        });
                    } else {
                        state.is_selecting = false;
                        // Clear selection if it's empty
                        if let Some(sel) = &state.selection {
                            if sel.is_empty() {
                                state.selection = None;
                            }
                        }
                    }
                    state.window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                state.mouse_x = position.x as f32;
                state.mouse_y = position.y as f32;
                
                if state.is_selecting {
                    state.position_cursor_at_mouse();
                    if let Some(ref mut sel) = state.selection {
                        sel.end_line = state.cursor_line;
                        sel.end_col = state.cursor_col;
                    }
                    state.window.request_redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * state.line_height * 3.0,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };
                
                let max_scroll = (state.content.len() as f32 * state.line_height - state.config.height as f32 + 100.0).max(0.0);
                state.scroll_offset = (state.scroll_offset - scroll_amount).clamp(0.0, max_scroll);
                state.window.request_redraw();
            }
            WindowEvent::KeyboardInput { event: KeyEvent { logical_key, state: key_state, .. }, .. } => {
                if key_state == ElementState::Pressed {
                    state.handle_key_press(&logical_key, event_loop);
                }
            }
            WindowEvent::RedrawRequested => {
                // Blink cursor
                let now = std::time::Instant::now();
                if now.duration_since(state.last_cursor_toggle).as_millis() > 500 {
                    state.cursor_visible = !state.cursor_visible;
                    state.last_cursor_toggle = now;
                }
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
/// Load initial file - try Cargo.toml, README.md, or create welcome content
fn load_initial_file() -> (Vec<String>, Option<PathBuf>) {
    let candidates = [
        "Cargo.toml".to_string(),
        "README.md".to_string(),
        "src/main.rs".to_string(),
        "src/lib.rs".to_string(),
    ];
    
    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                let lines: Vec<String> = content.lines().map(String::from).collect();
                tracing::info!("📄 Loaded file: {}", candidate);
                return (if lines.is_empty() { vec![String::new()] } else { lines }, Some(path));
            }
        }
    }
    
    // Default welcome content
    let content = vec![
        "// 🦊 Welcome to Foxkit IDE!".to_string(),
        "//".to_string(),
        "// This is a REAL interactive editor - start typing!".to_string(),
        "//".to_string(),
        "// Keyboard shortcuts:".to_string(),
        "//   Ctrl+S  - Save file".to_string(),
        "//   Ctrl+O  - Open file".to_string(),
        "//   Ctrl+N  - New file".to_string(),
        "//   Ctrl+Z  - Undo".to_string(),
        "//   Ctrl+Y  - Redo".to_string(),
        "//   Ctrl+C  - Copy".to_string(),
        "//   Ctrl+V  - Paste".to_string(),
        "//   Ctrl+X  - Cut".to_string(),
        "//   Ctrl+A  - Select all".to_string(),
        "//   Tab     - Toggle file explorer".to_string(),
        "//   Escape  - Exit (or switch to Normal mode)".to_string(),
        "".to_string(),
        "fn main() {".to_string(),
        "    // Start typing here...".to_string(),
        "    println!(\"Hello from Foxkit!\");".to_string(),
        "}".to_string(),
    ];
    (content, None)
}
/// Scan directory for file explorer
fn scan_directory(dir: &PathBuf) -> Vec<(String, PathBuf, bool)> {
    let mut entries = Vec::new();
    
    if let Ok(read_dir) = fs::read_dir(dir) {
        let mut items: Vec<_> = read_dir
            .filter_map(|e| e.ok())
            .map(|e| {
                let path = e.path();
                let is_dir = path.is_dir();
                let name = e.file_name().to_string_lossy().to_string();
                (name, path, is_dir)
            })
            .filter(|(name, _, _)| !name.starts_with('.') && name != "target" && name != "node_modules")
            .collect();
        
        // Sort: directories first, then alphabetically
        items.sort_by(|a, b| {
            match (a.2, b.2) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.0.to_lowercase().cmp(&b.0.to_lowercase()),
            }
        });
        
        for (name, path, is_dir) in items {
            let display = if is_dir {
                format!("📁 {}", name)
            } else {
                let icon = match path.extension().and_then(|e| e.to_str()) {
                    Some("rs") => "[R]",
                    Some("toml") => "[T]",
                    Some("md") => "[M]",
                    Some("json") => "[J]",
                    Some("ts" | "tsx") => "[TS]",
                    Some("js" | "jsx") => "[JS]",
                    Some("py") => "[PY]",
                    Some("html") => "[H]",
                    Some("css") => "[C]",
                    _ => "[F]",
                };
                format!("{} {}", icon, name)
            };
            entries.push((display, path, is_dir));
        }
    }
    
    entries
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
    
    /// Position cursor based on mouse click
    fn position_cursor_at_mouse(&mut self) {
        let explorer_width = if self.show_file_explorer { 200.0 } else { 0.0 };
        let gutter_width = 50.0;
        let title_height = 35.0;
        let tab_height = 30.0;
        
        let editor_x = explorer_width + gutter_width + 10.0;
        let editor_y = title_height + tab_height;
        
        // Check if click is in editor area
        if self.mouse_x >= editor_x && self.mouse_y >= editor_y {
            let rel_y = self.mouse_y - editor_y + self.scroll_offset;
            let rel_x = self.mouse_x - editor_x;
            
            let line = (rel_y / self.line_height) as usize;
            let col = (rel_x / self.char_width) as usize;
            
            self.cursor_line = line.min(self.content.len().saturating_sub(1));
            let line_len = self.content.get(self.cursor_line).map(|l| l.chars().count()).unwrap_or(0);
            self.cursor_col = col.min(line_len);
            
            // Reset cursor blink on click
            self.cursor_visible = true;
            self.last_cursor_toggle = std::time::Instant::now();
        }
    }
    
    /// Handle keyboard input - REAL TEXT EDITING
    fn handle_key_press(&mut self, key: &Key, event_loop: &ActiveEventLoop) {
        let ctrl = self.modifiers.control_key();
        let shift = self.modifiers.shift_key();
        
        // Reset cursor blink on any key
        self.cursor_visible = true;
        self.last_cursor_toggle = std::time::Instant::now();
        
        match key {
            // === CONTROL KEY SHORTCUTS ===
            _ if ctrl => {
                match key {
                    Key::Character(c) if c.as_str() == "s" => {
                        self.save_file();
                    }
                    Key::Character(c) if c.as_str() == "o" => {
                        // Open file (simplified - open first file in explorer)
                        if !self.files.is_empty() {
                            let (_, path, is_dir) = &self.files[self.selected_file_index];
                            if !is_dir {
                                self.open_file(path.clone());
                            }
                        }
                    }
                    Key::Character(c) if c.as_str() == "n" => {
                        self.new_file();
                    }
                    Key::Character(c) if c.as_str() == "z" => {
                        self.undo();
                    }
                    Key::Character(c) if c.as_str() == "y" => {
                        self.redo();
                    }
                    Key::Character(c) if c.as_str() == "c" => {
                        self.copy();
                    }
                    Key::Character(c) if c.as_str() == "v" => {
                        self.paste();
                    }
                    Key::Character(c) if c.as_str() == "x" => {
                        self.cut();
                    }
                    Key::Character(c) if c.as_str() == "a" => {
                        self.select_all();
                    }
                    Key::Named(NamedKey::Home) => {
                        // Go to start of document
                        self.cursor_line = 0;
                        self.cursor_col = 0;
                        self.scroll_offset = 0.0;
                    }
                    Key::Named(NamedKey::End) => {
                        // Go to end of document
                        self.cursor_line = self.content.len().saturating_sub(1);
                        self.cursor_col = self.content.get(self.cursor_line).map(|l| l.chars().count()).unwrap_or(0);
                        self.ensure_cursor_visible();
                    }
                    _ => {}
                }
            }
            
            // === NAMED KEYS ===
            Key::Named(NamedKey::Escape) => {
                if self.mode == EditorMode::Insert {
                    self.mode = EditorMode::Normal;
                } else {
                    event_loop.exit();
                }
            }
            
            Key::Named(NamedKey::Tab) if !ctrl => {
                if shift {
                    // Toggle file explorer
                    self.show_file_explorer = !self.show_file_explorer;
                } else {
                    // Insert tab (4 spaces)
                    self.insert_text("    ");
                }
            }
            
            Key::Named(NamedKey::Enter) => {
                self.insert_newline();
            }
            
            Key::Named(NamedKey::Backspace) => {
                self.backspace();
            }
            
            Key::Named(NamedKey::Delete) => {
                self.delete();
            }
            
            Key::Named(NamedKey::ArrowUp) => {
                self.clear_selection_if_not_shift(shift);
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.clamp_cursor_col();
                }
                if shift { self.extend_selection(); }
                self.ensure_cursor_visible();
            }
            
            Key::Named(NamedKey::ArrowDown) => {
                self.clear_selection_if_not_shift(shift);
                if self.cursor_line < self.content.len().saturating_sub(1) {
                    self.cursor_line += 1;
                    self.clamp_cursor_col();
                }
                if shift { self.extend_selection(); }
                self.ensure_cursor_visible();
            }
            
            Key::Named(NamedKey::ArrowLeft) => {
                self.clear_selection_if_not_shift(shift);
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.cursor_col = self.content.get(self.cursor_line).map(|l| l.chars().count()).unwrap_or(0);
                }
                if shift { self.extend_selection(); }
            }
            
            Key::Named(NamedKey::ArrowRight) => {
                self.clear_selection_if_not_shift(shift);
                let line_len = self.content.get(self.cursor_line).map(|l| l.chars().count()).unwrap_or(0);
                if self.cursor_col < line_len {
                    self.cursor_col += 1;
                } else if self.cursor_line < self.content.len().saturating_sub(1) {
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                }
                if shift { self.extend_selection(); }
            }
            
            Key::Named(NamedKey::Home) => {
                self.clear_selection_if_not_shift(shift);
                self.cursor_col = 0;
                if shift { self.extend_selection(); }
            }
            
            Key::Named(NamedKey::End) => {
                self.clear_selection_if_not_shift(shift);
                self.cursor_col = self.content.get(self.cursor_line).map(|l| l.chars().count()).unwrap_or(0);
                if shift { self.extend_selection(); }
            }
            
            Key::Named(NamedKey::PageUp) => {
                let lines_per_page = (self.config.height as f32 / self.line_height) as usize;
                self.cursor_line = self.cursor_line.saturating_sub(lines_per_page);
                self.scroll_offset = (self.scroll_offset - lines_per_page as f32 * self.line_height).max(0.0);
                self.clamp_cursor_col();
            }
            
            Key::Named(NamedKey::PageDown) => {
                let lines_per_page = (self.config.height as f32 / self.line_height) as usize;
                self.cursor_line = (self.cursor_line + lines_per_page).min(self.content.len().saturating_sub(1));
                let max_scroll = (self.content.len() as f32 * self.line_height - self.config.height as f32).max(0.0);
                self.scroll_offset = (self.scroll_offset + lines_per_page as f32 * self.line_height).min(max_scroll);
                self.clamp_cursor_col();
            }
            
            // === CHARACTER INPUT ===
            Key::Character(c) => {
                // Insert the typed character
                self.insert_text(c.as_str());
            }
            
            Key::Named(NamedKey::Space) => {
                self.insert_text(" ");
            }
            
            _ => {}
        }
        
        self.window.request_redraw();
    }
    
    /// Clamp cursor column to line length
    fn clamp_cursor_col(&mut self) {
        let line_len = self.content.get(self.cursor_line).map(|l| l.chars().count()).unwrap_or(0);
        self.cursor_col = self.cursor_col.min(line_len);
    }
    
    /// Insert text at cursor position - REAL EDITING
    fn insert_text(&mut self, text: &str) {
        // Delete selection first if any
        self.delete_selection();
        
        if self.content.is_empty() {
            self.content.push(String::new());
        }
        
        let line = &mut self.content[self.cursor_line];
        
        // Convert cursor_col to byte index
        let byte_idx: usize = line.chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();
        
        // Insert text
        line.insert_str(byte_idx, text);
        self.cursor_col += text.chars().count();
        
        self.is_modified = true;
        
        // Record for undo
        self.current_action_group.push(EditAction::Insert {
            line: self.cursor_line,
            col: self.cursor_col - text.chars().count(),
            text: text.to_string(),
        });
    }
    
    /// Insert a new line at cursor position
    fn insert_newline(&mut self) {
        self.delete_selection();
        
        if self.content.is_empty() {
            self.content.push(String::new());
        }
        
        let line = &self.content[self.cursor_line];
        let byte_idx: usize = line.chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();
        
        // Get indentation from current line
        let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
        
        // Split line
        let rest = line[byte_idx..].to_string();
        self.content[self.cursor_line].truncate(byte_idx);
        
        // Insert new line with indentation
        self.cursor_line += 1;
        self.content.insert(self.cursor_line, format!(" indent, rest));
        self.cursor_col = indent.chars().count();
        
        self.is_modified = true;
        self.ensure_cursor_visible();
        
        self.current_action_group.push(EditAction::InsertLine { line: self.cursor_line });
    }
    
    /// Delete character before cursor (backspace)
    fn backspace(&mut self) {
        // If there's a selection, delete it
        if self.selection.is_some() && !self.selection.as_ref().unwrap().is_empty() {
            self.delete_selection();
            return;
        }
        
        if self.content.is_empty() {
            return;
        }
        
        if self.cursor_col > 0 {
            // Delete character in current line
            let line = &mut self.content[self.cursor_line];
            let byte_idx: usize = line.chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();
            let prev_char_len = line[..byte_idx].chars().last().map(|c| c.len_utf8()).unwrap_or(0);
            let deleted = line[byte_idx - prev_char_len..byte_idx].to_string();
            line.replace_range(byte_idx - prev_char_len..byte_idx, "");
            self.cursor_col -= 1;
            
            self.current_action_group.push(EditAction::Delete {
                line: self.cursor_line,
                col: self.cursor_col,
                text: deleted,
            });
        } else if self.cursor_line > 0 {
            // Join with previous line
            let current_line = self.content.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.content[self.cursor_line].chars().count();
            self.content[self.cursor_line].push_str(&current_line);
            
            self.current_action_group.push(EditAction::DeleteLine {
                line: self.cursor_line + 1,
                content: current_line,
            });
        }
        
        self.is_modified = true;
        self.ensure_cursor_visible();
    }
    
    /// Delete character at cursor (delete key)
    fn delete(&mut self) {
        // If there's a selection, delete it
        if self.selection.is_some() && !self.selection.as_ref().unwrap().is_empty() {
            self.delete_selection();
            return;
        }
        
        if self.content.is_empty() {
            return;
        }
        
        let line_len = self.content[self.cursor_line].chars().count();
        
        if self.cursor_col < line_len {
            // Delete character in current line
            let line = &mut self.content[self.cursor_line];
            let byte_idx: usize = line.chars().take(self.cursor_col).map(|c| c.len_utf8()).sum();
            let next_char_len = line[byte_idx..].chars().next().map(|c| c.len_utf8()).unwrap_or(0);
            let deleted = line[byte_idx..byte_idx + next_char_len].to_string();
            line.replace_range(byte_idx..byte_idx + next_char_len, "");
            
            self.current_action_group.push(EditAction::Delete {
                line: self.cursor_line,
                col: self.cursor_col,
                text: deleted,
            });
        } else if self.cursor_line < self.content.len() - 1 {
            // Join with next line
            let next_line = self.content.remove(self.cursor_line + 1);
            self.content[self.cursor_line].push_str(&next_line);
            
            self.current_action_group.push(EditAction::DeleteLine {
                line: self.cursor_line + 1,
                content: next_line,
            });
        }
        
        self.is_modified = true;
    }
    
    /// Delete selected text
    fn delete_selection(&mut self) {
        let Some(sel) = self.selection.take() else { return };
        if sel.is_empty() { return; }
        
        let (start_line, start_col, end_line, end_col) = sel.normalized();
        
        if start_line == end_line {
            // Single line selection
            let line = &mut self.content[start_line];
            let start_byte: usize = line.chars().take(start_col).map(|c| c.len_utf8()).sum();
            let end_byte: usize = line.chars().take(end_col).map(|c| c.len_utf8()).sum();
            line.replace_range(start_byte..end_byte, "");
        } else {
            // Multi-line selection
            let first_line_start: usize = self.content[start_line].chars().take(start_col).map(|c| c.len_utf8()).sum();
            let last_line_end: usize = self.content[end_line].chars().take(end_col).map(|c| c.len_utf8()).sum();
            
            let remaining = self.content[end_line][last_line_end..].to_string();
            self.content[start_line].truncate(first_line_start);
            self.content[start_line].push_str(&remaining);
            
            // Remove middle lines
            for _ in start_line + 1..=end_line {
                self.content.remove(start_line + 1);
            }
        }
        
        self.cursor_line = start_line;
        self.cursor_col = start_col;
        self.is_modified = true;
    }
    
    fn clear_selection_if_not_shift(&mut self, shift: bool) {
        if !shift {
            self.selection = None;
        } else if self.selection.is_none() {
            self.selection = Some(Selection {
                start_line: self.cursor_line,
                start_col: self.cursor_col,
                end_line: self.cursor_line,
                end_col: self.cursor_col,
            });
        }
    }
    
    fn extend_selection(&mut self) {
        if let Some(ref mut sel) = self.selection {
            sel.end_line = self.cursor_line;
            sel.end_col = self.cursor_col;
        }
    }
    
    fn select_all(&mut self) {
        self.selection = Some(Selection {
            start_line: 0,
            start_col: 0,
            end_line: self.content.len().saturating_sub(1),
            end_col: self.content.last().map(|l| l.chars().count()).unwrap_or(0),
        });
        self.cursor_line = self.content.len().saturating_sub(1);
        self.cursor_col = self.content.last().map(|l| l.chars().count()).unwrap_or(0);
    }
    
    fn copy(&mut self) {
        let Some(sel) = &self.selection else { return };
        if sel.is_empty() { return; }
        
        let (start_line, start_col, end_line, end_col) = sel.normalized();
        
        if start_line == end_line {
            let line = &self.content[start_line];
            let start_byte: usize = line.chars().take(start_col).map(|c| c.len_utf8()).sum();
            let end_byte: usize = line.chars().take(end_col).map(|c| c.len_utf8()).sum();
            self.clipboard = line[start_byte..end_byte].to_string();
        } else {
            let mut result = String::new();
            for line_idx in start_line..=end_line {
                let line = &self.content[line_idx];
                if line_idx == start_line {
                    let start_byte: usize = line.chars().take(start_col).map(|c| c.len_utf8()).sum();
                    result.push_str(&line[start_byte..]);
                } else if line_idx == end_line {
                    let end_byte: usize = line.chars().take(end_col).map(|c| c.len_utf8()).sum();
                    result.push('\n');
                    result.push_str(&line[..end_byte]);
                } else {
                    result.push('\n');
                    result.push_str(line);
                }
            }
            self.clipboard = result;
        }
    }
    
    fn paste(&mut self) {
        if self.clipboard.is_empty() { return; }
        
        self.delete_selection();
        
        // Clone clipboard to avoid borrow issues
        let clipboard_content = self.clipboard.clone();
        let lines: Vec<&str> = clipboard_content.split('\n').collect();
        
        if lines.len() == 1 {
            self.insert_text(&clipboard_content);
        } else {
            for (i, line_text) in lines.iter().enumerate() {
                if i > 0 {
                    self.insert_newline();
                }
                if !line_text.is_empty() {
                    self.insert_text(line_text);
                }
            }
        }
    }
    
    fn cut(&mut self) {
        self.copy();
        self.delete_selection();
    }
    
    fn undo(&mut self) {
        // Commit current action group
        if !self.current_action_group.is_empty() {
            self.undo_stack.push(std::mem::take(&mut self.current_action_group));
        }
        
        let Some(actions) = self.undo_stack.pop() else { return };
        
        for action in actions.iter().rev() {
            match action {
                EditAction::Insert { line, col, text } => {
                    let l = &mut self.content[*line];
                    let start_byte: usize = l.chars().take(*col).map(|c| c.len_utf8()).sum();
                    let end_byte: usize = l.chars().take(*col + text.chars().count()).map(|c| c.len_utf8()).sum();
                    l.replace_range(start_byte..end_byte, "");
                    self.cursor_line = *line;
                    self.cursor_col = *col;
                }
                EditAction::Delete { line, col, text } => {
                    let l = &mut self.content[*line];
                    let byte_idx: usize = l.chars().take(*col).map(|c| c.len_utf8()).sum();
                    l.insert_str(byte_idx, text);
                    self.cursor_line = *line;
                    self.cursor_col = *col + text.chars().count();
                }
                EditAction::InsertLine { line } => {
                    if *line > 0 {
                        let removed = self.content.remove(*line);
                        self.content[*line - 1].push_str(&removed);
                    }
                    self.cursor_line = line.saturating_sub(1);
                }
                EditAction::DeleteLine { line, content } => {
                    self.content.insert(*line, content.clone());
                }
            }
        }
        
        self.redo_stack.push(actions);
        self.ensure_cursor_visible();
    }
    
    fn redo(&mut self) {
        let Some(actions) = self.redo_stack.pop() else { return };
        
        for action in &actions {
            match action {
                EditAction::Insert { line, col, text } => {
                    let l = &mut self.content[*line];
                    let byte_idx: usize = l.chars().take(*col).map(|c| c.len_utf8()).sum();
                    l.insert_str(byte_idx, text);
                    self.cursor_line = *line;
                    self.cursor_col = *col + text.chars().count();
                }
                EditAction::Delete { line, col, text } => {
                    let l = &mut self.content[*line];
                    let start_byte: usize = l.chars().take(*col).map(|c| c.len_utf8()).sum();
                    let end_byte: usize = l.chars().take(*col + text.chars().count()).map(|c| c.len_utf8()).sum();
                    l.replace_range(start_byte..end_byte, "");
                    self.cursor_line = *line;
                    self.cursor_col = *col;
                }
                EditAction::InsertLine { line } => {
                    self.content.insert(*line, String::new());
                }
                EditAction::DeleteLine { line, .. } => {
                    self.content.remove(*line);
                }
            }
        }
        
        self.undo_stack.push(actions);
        self.ensure_cursor_visible();
    }
    
    /// Save file to disk
    fn save_file(&mut self) {
        // Commit current actions for undo
        if !self.current_action_group.is_empty() {
            self.undo_stack.push(std::mem::take(&mut self.current_action_group));
        }
        
        let path = self.current_file.clone().unwrap_or_else(|| {
            // Default save path
            PathBuf::from(r#"untitled.txt"#)
        });
        
        let content = self.content.join(r#"
"#);
        match fs::write(&path, &content) {
            Ok(_) => {
                self.current_file = Some(path.clone());
                self.is_modified = false;
                tracing::info!("Saved: {}", path.display());
            }
            Err(e) => {
                tracing::error!("Failed to save: {}", e);
            }
        }
    }
    
    /// Open a file
    fn open_file(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
            Ok(content) => {
                self.content = content.lines().map(String::from).collect();
                if self.content.is_empty() {
                    self.content.push(String::new());
                }
                self.current_file = Some(path.clone());
                self.cursor_line = 0;
                self.cursor_col = 0;
                self.scroll_offset = 0.0;
                self.is_modified = false;
                self.undo_stack.clear();
                self.redo_stack.clear();
                self.selection = None;
                tracing::info!("Opened: {}", path.display());
            }
            Err(e) => {
                tracing::error!("Failed to open: {}", e);
            }
        }
    }
    
    /// Create new file
    fn new_file(&mut self) {
        self.content = vec![String::new()];
        self.current_file = None;
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0.0;
        self.is_modified = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.selection = None;
        tracing::info!(r#"New file created"#);
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
            "Foxkit IDE".to_string(),
            Point { x: 10.0, y: 22.0 },
            &self.font_key,
            self.font_size,
            Color { r: 0.9, g: 0.9, b: 0.95, a: 1.0 },
        );
        all_glyphs.extend(title_glyphs);
        // File path in title
        let file_display = self.current_file.as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "untitled".to_string());
        let modified_mark = if self.is_modified { " *" } else { "" };
        let path_glyphs = self.text_renderer.layout_text(
            format!(" file_display, modified_mark),
            Point { x: 120.0, y: 22.0 },
            &self.font_key,
            self.font_size - 2.0,
            Color { r: 0.6, g: 0.6, b: 0.65, a: 1.0 },
        );
        all_glyphs.extend(path_glyphs);
        // Tab text - show actual filename
        let tab_name = self.current_file.as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("untitled");
        let tab_text = format!("{},{}", tab_name, if self.is_modified { " *" } else { "" });
        let tab_glyphs = self.text_renderer.layout_text(
            &tab_text,
            Point { x: explorer_width + 15.0, y: title_height + 20.0 },
            &self.font_key,
            self.font_size - 1.0,
            Color { r: 0.85, g: 0.85, b: 0.9, a: 1.0 },
        );
        all_glyphs.extend(tab_glyphs);
        // File explorer
        if self.show_file_explorer {
            let explorer_header = self.text_renderer.layout_text(
                "EXPLORER".to_string(),
                Point { x: 10.0, y: title_height + 18.0 },
                &self.font_key,
                self.font_size - 3.0,
                Color { r: 0.5, g: 0.5, b: 0.55, a: 1.0 },
            );
            all_glyphs.extend(explorer_header);
            for (i, (display_name, _path, _is_dir)) in self.files.iter().enumerate() {
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
                    display_name,
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
            let line_num = format!("{:4}", line_idx + 1);
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
        let modified_indicator = if self.is_modified { " [Modified]" } else { "" };
        let _file_name = self.current_file.as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("untitled");
        let status_text = format!(
            "Ln {}, Col {}  |  UTF-8  |  Rust  |  {} lines{}".to_string(),
            self.cursor_line + 1,
            self.cursor_col + 1,
            self.content.len(),
            modified_indicator
        );
        let status_glyphs = self.text_renderer.layout_text(
            &status_text,
            Point { x: width - 350.0, y: height - 8.0 },
            &self.font_key,
            self.font_size - 2.0,
            Color { r: 0.6, g: 0.6, b: 0.65, a: 1.0 },
        );
        all_glyphs.extend(status_glyphs);
        // Mode indicator with color
        let (mode_text, mode_color) = match self.mode {
            EditorMode::Normal => ("NORMAL", Color { r: 0.4, g: 0.6, b: 0.9, a: 1.0 }),
            EditorMode::Insert => ("INSERT", Color { r: 0.4, g: 0.8, b: 0.4, a: 1.0 }),
        };
        let mode_glyphs = self.text_renderer.layout_text(
            mode_text,
            Point { x: 10.0, y: height - 8.0 },
            &self.font_key,
            self.font_size - 2.0,
            mode_color,
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
            tokens.push((trimmed, comment_color));
        } else {
            // Very simple tokenization
            let mut remaining = trimmed;
            while !remaining.is_empty() {
                // Check for string
                if remaining.starts_with('"') {
                    if let Some(end) = remaining[1..].find('"') {
                        let s = &remaining[..end + 2];
                        tokens.push((s, string_color));
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
                            tokens.push((kw, keyword_color));
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
                    tokens.push((remaining[..end], type_color));
                    remaining = &remaining[end..];
                    continue;
                }
                // Check for function calls (word followed by `(`)
                if remaining.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) {
                    let end = remaining.find(|c: char| !c.is_alphanumeric() && c != '_' && c != '!').unwrap_or(remaining.len());
                    let word = &remaining[..end];
                    let is_fn = remaining[end..].starts_with('(') || word.ends_with('!');
                    if is_fn {
                        tokens.push((word, function_color));
                    } else {
                        tokens.push((word, default_color));
                    }
                    remaining = &remaining[end..];
                    continue;
                }
                // Default: take one character
                let ch = remaining.chars().next().unwrap();
                tokens.push((ch, default_color));
                remaining = &remaining[ch.len_utf8()..];
            }
        }
        tokens
    }
    fn render_file_explorer(&self, scene: &mut Scene, rect: Rect) {
        // File explorer content
        let mut y_offset = rect.origin.y + 5.0;
        
        // Directory title
        scene.draw_text(
            "EXPLORER".to_string(),
            Point { x: rect.origin.x + 8.0, y: y_offset },
            Color { r: 0.6, g: 0.6, b: 0.7, a: 1.0 },
            10.0,
        );
        y_offset += 20.0;
        // Files list
        for (i, file) in self.files.iter().enumerate() {
            if y_offset + 20.0 > rect.origin.y + rect.size.height {
                break;
            }
            if i == self.selected_file_index {
                // Selected file background
                scene.add(Primitive::Quad {
                    rect: Rect {
                        origin: Point { x: rect.origin.x, y: y_offset - 2.0 },
                        size: Size { width: rect.size.width, height: 18.0 },
                    },
                    color: Color { r: 0.25, g: 0.35, b: 0.55, a: 1.0 },
                    corner_radius: 0.0,
                });
            }
            // File icon (simple colored square)
            let icon_color = if file.2 { // is_dir
                Color { r: 0.7, g: 0.5, b: 0.3, a: 1.0 }
            } else {
                Color { r: 0.5, g: 0.6, b: 0.8, a: 1.0 }
            };
            scene.add(Primitive::Quad {
                rect: Rect {
                    origin: Point { x: rect.origin.x + 8.0, y: y_offset },
                    size: Size { width: 12.0, height: 12.0 },
                },
                color: icon_color,
                corner_radius: 2.0,
            });
            // File name
            scene.draw_text(
                file.0.clone(), // name
                Point { x: rect.origin.x + 25.0, y: y_offset + 1.0 },
                Color { r: 0.9, g: 0.9, b: 0.95, a: 1.0 },
                11.0,
            );
            y_offset += 18.0;
        }
    }
    fn render_editor(&self, scene: &mut Scene, rect: Rect) {
        let gutter_width = 50.0;
        let editor_y = rect.origin.y;
        let first_line = (self.scroll_offset / self.line_height) as usize;
        // Gutter background
        scene.add(Primitive::Quad {
            rect: Rect {
                origin: Point { x: rect.origin.x, y: editor_y },
                size: Size { width: gutter_width, height: rect.size.height },
            },
            color: Color { r: 0.13, g: 0.13, b: 0.15, a: 1.0 },
            corner_radius: 0.0,
        });
        // Current line highlight
        if self.cursor_line >= first_line {
            let relative_line = self.cursor_line - first_line;
            let cursor_y = editor_y + relative_line as f32 * self.line_height - (self.scroll_offset % self.line_height);
            
            if cursor_y >= editor_y && cursor_y < rect.origin.y + rect.size.height {
                // Current line highlight
                scene.add(Primitive::Quad {
                    rect: Rect {
                        origin: Point { x: rect.origin.x + gutter_width, y: cursor_y },
                        size: Size { width: rect.size.width - gutter_width - 12.0, height: self.line_height },
                    },
                    color: Color { r: 0.16, g: 0.16, b: 0.2, a: 1.0 },
                    corner_radius: 0.0,
                });
                // Blinking cursor
                if self.cursor_visible {
                    let cursor_x = rect.origin.x + gutter_width + 10.0 + self.cursor_col as f32 * self.char_width;
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
        }
        // Selection highlight
        if let Some(sel) = &self.selection {
            if !sel.is_empty() {
                let (start_line, start_col, end_line, end_col) = sel.normalized();
                
                for line_idx in start_line..=end_line {
                    if line_idx < first_line { continue; }
                    let relative_line = line_idx - first_line;
                    let sel_y = editor_y + relative_line as f32 * self.line_height - (self.scroll_offset % self.line_height);
                    
                    if sel_y >= editor_y && sel_y < rect.origin.y + rect.size.height {
                        let line_len = self.content.get(line_idx).map(|l| l.chars().count()).unwrap_or(0);
                        let sel_start_col = if line_idx == start_line { start_col } else { 0 };
                        let sel_end_col = if line_idx == end_line { end_col } else { line_len };
                        
                        let sel_x = rect.origin.x + gutter_width + 10.0 + sel_start_col as f32 * self.char_width;
                        let sel_width = (sel_end_col - sel_start_col) as f32 * self.char_width;
                        
                        if sel_width > 0.0 {
                            scene.add(Primitive::Quad {
                                rect: Rect {
                                    origin: Point { x: sel_x, y: sel_y },
                                    size: Size { width: sel_width, height: self.line_height },
                                },
                                color: Color { r: 0.25, g: 0.35, b: 0.55, a: 0.5 },
                                corner_radius: 0.0,
                            });
                        }
                    }
                }
            }
        }
        // Render text content
        for (line_idx, line) in self.content.iter().enumerate() {
            if line_idx < first_line { continue; }
            let relative_line = line_idx - first_line;
            let y = editor_y + relative_line as f32 * self.line_height - (self.scroll_offset % self.line_height);
            
            if y >= rect.origin.y + rect.size.height { break; }
            if y + self.line_height < editor_y { continue; }
            // Line number
            scene.draw_text(
                scene,
                format!("{:4}", line_idx + 1),
                Point { x: rect.origin.x + 8.0, y: y + 2.0 },
                Color { r: 0.5, g: 0.5, b: 0.6, a: 1.0 },
                11.0,
            );
            // Line content
            scene.draw_text(
                scene,
                line,
                Point { x: rect.origin.x + gutter_width + 10.0, y: y + 2.0 },
                Color { r: 0.9, g: 0.9, b: 0.95, a: 1.0 },
                self.font_size,
            );
        }
        // Scrollbar
        let scrollbar_x = rect.origin.x + rect.size.width - 12.0;
        scene.add(Primitive::Quad {
            rect: Rect {
                origin: Point { x: scrollbar_x, y: editor_y },
                size: Size { width: 10.0, height: rect.size.height },
            },
            color: Color { r: 0.13, g: 0.13, b: 0.15, a: 1.0 },
            corner_radius: 5.0,
        });
        let total_content_height = self.content.len() as f32 * self.line_height;
        if total_content_height > rect.size.height {
            let thumb_ratio = rect.size.height / total_content_height;
            let thumb_height = (rect.size.height * thumb_ratio).max(30.0);
            let scroll_ratio = self.scroll_offset / (total_content_height - rect.size.height);
            let thumb_y = editor_y + scroll_ratio * (rect.size.height - thumb_height);
            scene.add(Primitive::Quad {
                rect: Rect {
                    origin: Point { x: scrollbar_x + 2.0, y: thumb_y },
                    size: Size { width: 6.0, height: thumb_height },
                },
                color: Color { r: 0.35, g: 0.35, b: 0.4, a: 1.0 },
                corner_radius: 3.0,
            });
        }
    }
    fn render_terminal(&self, scene: &mut Scene, rect: Rect) {
        // Terminal placeholder
        scene.draw_text(
            scene,
            "Terminal - Coming Soon...".to_string(),
            Point { x: rect.origin.x + 10.0, y: rect.origin.y + 10.0 },
            Color { r: 0.6, g: 0.6, b: 0.7, a: 1.0 },
            12.0,
        );
    }
    fn render_ai_chat(&self, scene: &mut Scene, rect: Rect) {
        // AI Chat placeholder
        scene.draw_text(
            scene,
            "AI Chat - Coming Soon...".to_string(),
            Point { x: rect.origin.x + 10.0, y: rect.origin.y + 10.0 },
            Color { r: 0.6, g: 0.6, b: 0.7, a: 1.0 },
            12.0,
        );
    }
    fn render_output(&self, scene: &mut Scene, rect: Rect) {
        // Output panel placeholder
        scene.draw_text(
            scene,
            "Output - Coming Soon...".to_string(),
            Point { x: rect.origin.x + 10.0, y: rect.origin.y + 10.0 },
            Color { r: 0.6, g: 0.6, b: 0.7, a: 1.0 },
            12.0,
        );
    }
    fn build_scene(&self) -> Scene {
        let mut scene = Scene::new();
        let width = self.config.width as f32;
        let height = self.config.height as f32;
        // Title bar background
        scene.add(Primitive::Quad {
            rect: Rect {
                origin: Point { x: 0.0, y: 0.0 },
                size: Size { width, height: 35.0 },
            },
            color: Color { r: 0.15, g: 0.15, b: 0.18, a: 1.0 },
            corner_radius: 0.0,
        });
        // Title bar text
        scene.draw_text(
            
            "Foxkit IDE".to_string(),
            Point { x: 10.0, y: 8.0 },
            Color { r: 0.9, g: 0.9, b: 0.95, a: 1.0 },
            16.0,
        );
        // Render panels
        for panel in &self.panel_layout.panels {
            if !panel.is_visible {
                continue;
            }
            // Panel background
            scene.add(Primitive::Quad {
                rect: panel.rect,
                color: match panel.panel_type {
                    PanelType::FileExplorer => Color { r: 0.12, g: 0.12, b: 0.15, a: 1.0 },
                    PanelType::Editor => Color { r: 0.08, g: 0.08, b: 0.1, a: 1.0 },
                    PanelType::Terminal => Color { r: 0.05, g: 0.05, b: 0.08, a: 1.0 },
                    PanelType::AIChat => Color { r: 0.1, g: 0.1, b: 0.12, a: 1.0 },
                    PanelType::Output => Color { r: 0.08, g: 0.08, b: 0.1, a: 1.0 },
                },
                corner_radius: 0.0,
            });
            // Panel title bar
            let title_height = 25.0;
            scene.add(Primitive::Quad {
                rect: Rect {
                    origin: Point { x: panel.rect.origin.x, y: panel.rect.origin.y },
                    size: Size { width: panel.rect.size.width, height: title_height },
                },
                color: Color { r: 0.2, g: 0.2, b: 0.25, a: 1.0 },
                corner_radius: 0.0,
            });
            // Panel title text
            scene.draw_text(
                
                panel.title.clone(),
                Point { x: panel.rect.origin.x + 8.0, y: panel.rect.origin.y + 6.0 },
                Color { r: 0.8, g: 0.8, b: 0.85, a: 1.0 },
                12.0,
            );
            // Panel content based on type
            let content_rect = Rect {
                origin: Point { x: panel.rect.origin.x, y: panel.rect.origin.y + title_height },
                size: Size { width: panel.rect.size.width, height: panel.rect.size.height - title_height },
            };
            match panel.panel_type {
                PanelType::FileExplorer => self.render_file_explorer(&mut scene, content_rect),
                PanelType::Editor => self.render_editor(&mut scene, content_rect),
                PanelType::Terminal => self.render_terminal(&mut scene, content_rect),
                PanelType::AIChat => self.render_ai_chat(&mut scene, content_rect),
                PanelType::Output => self.render_output(&mut scene, content_rect),
            }
        }
        // Render splitters
        for splitter in &self.panel_layout.splitters {
            scene.add(Primitive::Quad {
                rect: splitter.rect,
                color: if splitter.is_dragging {
                    Color { r: 0.6, g: 0.6, b: 0.7, a: 1.0 }
                } else {
                    Color { r: 0.3, g: 0.3, b: 0.35, a: 1.0 }
                },
                corner_radius: 0.0,
            });
        }
        // Status bar
        let status_height = 24.0;
        scene.add(Primitive::Quad {
            rect: Rect {
                origin: Point { x: 0.0, y: height - status_height },
                size: Size { width, height: status_height },
            },
            color: Color { r: 0.15, g: 0.15, b: 0.18, a: 1.0 },
            corner_radius: 0.0,
        });
        // Status bar text
        let status_text = format!("Ln {}, Col {} | {}", self.cursor_line + 1, self.cursor_col + 1,
            if self.is_modified { "Modified" } else { "Saved" });
        scene.draw_text(
            
            &status_text,
            Point { x: 10.0, y: height - status_height + 6.0 },
            Color { r: 0.7, g: 0.7, b: 0.75, a: 1.0 },
            11.0,
        );
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
