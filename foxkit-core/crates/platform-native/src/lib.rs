//! Native platform abstractions for Foxkit.
//!
//! This crate provides cross-platform abstractions for windows, input,
//! clipboard, file dialogs, and other OS-level features.

use std::path::PathBuf;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Platform-specific window handle.
#[derive(Debug, Clone, Copy)]
pub struct WindowHandle(pub u64);

/// Screen information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screen {
    pub id: u32,
    pub bounds: ScreenBounds,
    pub scale_factor: f64,
    pub is_primary: bool,
}

/// Screen bounds in physical pixels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScreenBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Window creation options.
#[derive(Debug, Clone)]
pub struct WindowOptions {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub resizable: bool,
    pub decorations: bool,
    pub transparent: bool,
    pub always_on_top: bool,
    pub fullscreen: bool,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: String::from("Foxkit"),
            width: 1280,
            height: 720,
            min_width: Some(400),
            min_height: Some(300),
            max_width: None,
            max_height: None,
            resizable: true,
            decorations: true,
            transparent: false,
            always_on_top: false,
            fullscreen: false,
        }
    }
}

/// Clipboard content types.
#[derive(Debug, Clone)]
pub enum ClipboardContent {
    Text(String),
    Html(String),
    Image(Vec<u8>),
    Files(Vec<PathBuf>),
}

/// Clipboard service.
pub struct Clipboard {
    content: RwLock<Option<ClipboardContent>>,
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            content: RwLock::new(None),
        }
    }

    pub fn get_text(&self) -> Option<String> {
        match &*self.content.read() {
            Some(ClipboardContent::Text(s)) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn set_text(&self, text: String) {
        *self.content.write() = Some(ClipboardContent::Text(text));
    }

    pub fn clear(&self) {
        *self.content.write() = None;
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

/// File dialog options.
#[derive(Debug, Clone, Default)]
pub struct FileDialogOptions {
    pub title: Option<String>,
    pub default_path: Option<PathBuf>,
    pub filters: Vec<FileFilter>,
    pub multiple: bool,
    pub directory: bool,
}

/// File filter for dialogs.
#[derive(Debug, Clone)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

/// File dialog result.
#[derive(Debug, Clone)]
pub enum FileDialogResult {
    Single(PathBuf),
    Multiple(Vec<PathBuf>),
    Cancelled,
}

/// Mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

/// Keyboard modifier keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool, // Cmd on macOS, Win on Windows
}

/// Cursor style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorStyle {
    #[default]
    Default,
    Pointer,
    Text,
    Crosshair,
    Move,
    NotAllowed,
    Wait,
    Progress,
    ResizeNS,
    ResizeEW,
    ResizeNESW,
    ResizeNWSE,
    Grab,
    Grabbing,
}

/// Platform capabilities.
#[derive(Debug, Clone)]
pub struct PlatformCapabilities {
    pub supports_transparency: bool,
    pub supports_blur: bool,
    pub supports_native_tabs: bool,
    pub supports_touch: bool,
    pub supports_stylus: bool,
}

/// Platform information.
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub os: OperatingSystem,
    pub version: String,
    pub arch: Architecture,
    pub capabilities: PlatformCapabilities,
}

/// Operating system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    Windows,
    MacOS,
    Linux,
    Unknown,
}

/// CPU architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    Aarch64,
    Unknown,
}

/// Get current platform info.
pub fn get_platform_info() -> PlatformInfo {
    PlatformInfo {
        os: if cfg!(target_os = "windows") {
            OperatingSystem::Windows
        } else if cfg!(target_os = "macos") {
            OperatingSystem::MacOS
        } else if cfg!(target_os = "linux") {
            OperatingSystem::Linux
        } else {
            OperatingSystem::Unknown
        },
        version: String::new(),
        arch: if cfg!(target_arch = "x86_64") {
            Architecture::X86_64
        } else if cfg!(target_arch = "aarch64") {
            Architecture::Aarch64
        } else {
            Architecture::Unknown
        },
        capabilities: PlatformCapabilities {
            supports_transparency: true,
            supports_blur: cfg!(target_os = "macos"),
            supports_native_tabs: cfg!(target_os = "macos"),
            supports_touch: true,
            supports_stylus: true,
        },
    }
}
