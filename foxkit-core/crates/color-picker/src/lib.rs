//! # Foxkit Color Picker
//!
//! Inline color detection and editing.

use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Color picker service
pub struct ColorPickerService {
    /// Cached colors
    cache: RwLock<HashMap<PathBuf, Vec<ColorInfo>>>,
    /// Events
    events: broadcast::Sender<ColorPickerEvent>,
    /// Configuration
    config: RwLock<ColorPickerConfig>,
    /// Color patterns
    patterns: ColorPatterns,
}

impl ColorPickerService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            cache: RwLock::new(HashMap::new()),
            events,
            config: RwLock::new(ColorPickerConfig::default()),
            patterns: ColorPatterns::new(),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<ColorPickerEvent> {
        self.events.subscribe()
    }

    /// Configure color picker
    pub fn configure(&self, config: ColorPickerConfig) {
        *self.config.write() = config;
    }

    /// Find colors in document
    pub fn find_colors(&self, file: &PathBuf, content: &str) -> Vec<ColorInfo> {
        let config = self.config.read();
        
        if !config.enabled {
            return Vec::new();
        }

        let mut colors = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            // Find hex colors
            if config.detect_hex {
                colors.extend(self.find_hex_colors(line, line_num as u32));
            }

            // Find rgb/rgba colors
            if config.detect_rgb {
                colors.extend(self.find_rgb_colors(line, line_num as u32));
            }

            // Find hsl/hsla colors
            if config.detect_hsl {
                colors.extend(self.find_hsl_colors(line, line_num as u32));
            }

            // Find named colors
            if config.detect_named {
                colors.extend(self.find_named_colors(line, line_num as u32));
            }
        }

        // Cache
        self.cache.write().insert(file.clone(), colors.clone());

        colors
    }

    fn find_hex_colors(&self, line: &str, line_num: u32) -> Vec<ColorInfo> {
        let mut colors = Vec::new();

        for cap in self.patterns.hex.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                if let Some(color) = Color::from_hex(m.as_str()) {
                    colors.push(ColorInfo {
                        color,
                        range: ColorRange::single_line(
                            line_num,
                            m.start() as u32,
                            m.end() as u32,
                        ),
                        format: ColorFormat::Hex,
                        original: m.as_str().to_string(),
                    });
                }
            }
        }

        colors
    }

    fn find_rgb_colors(&self, line: &str, line_num: u32) -> Vec<ColorInfo> {
        let mut colors = Vec::new();

        for cap in self.patterns.rgb.captures_iter(line) {
            if let (Some(m), Some(r), Some(g), Some(b)) = 
                (cap.get(0), cap.get(1), cap.get(2), cap.get(3)) 
            {
                let r: u8 = r.as_str().parse().unwrap_or(0);
                let g: u8 = g.as_str().parse().unwrap_or(0);
                let b: u8 = b.as_str().parse().unwrap_or(0);
                let a: f32 = cap.get(4)
                    .and_then(|a| a.as_str().parse().ok())
                    .unwrap_or(1.0);

                colors.push(ColorInfo {
                    color: Color::rgba(r, g, b, a),
                    range: ColorRange::single_line(
                        line_num,
                        m.start() as u32,
                        m.end() as u32,
                    ),
                    format: if cap.get(4).is_some() { 
                        ColorFormat::Rgba 
                    } else { 
                        ColorFormat::Rgb 
                    },
                    original: m.as_str().to_string(),
                });
            }
        }

        colors
    }

    fn find_hsl_colors(&self, line: &str, line_num: u32) -> Vec<ColorInfo> {
        let mut colors = Vec::new();

        for cap in self.patterns.hsl.captures_iter(line) {
            if let (Some(m), Some(h), Some(s), Some(l)) = 
                (cap.get(0), cap.get(1), cap.get(2), cap.get(3)) 
            {
                let h: f32 = h.as_str().parse().unwrap_or(0.0);
                let s: f32 = s.as_str().trim_end_matches('%').parse().unwrap_or(0.0) / 100.0;
                let l: f32 = l.as_str().trim_end_matches('%').parse().unwrap_or(0.0) / 100.0;
                let a: f32 = cap.get(4)
                    .and_then(|a| a.as_str().parse().ok())
                    .unwrap_or(1.0);

                if let Some(color) = Color::from_hsl(h, s, l, a) {
                    colors.push(ColorInfo {
                        color,
                        range: ColorRange::single_line(
                            line_num,
                            m.start() as u32,
                            m.end() as u32,
                        ),
                        format: if cap.get(4).is_some() { 
                            ColorFormat::Hsla 
                        } else { 
                            ColorFormat::Hsl 
                        },
                        original: m.as_str().to_string(),
                    });
                }
            }
        }

        colors
    }

    fn find_named_colors(&self, line: &str, line_num: u32) -> Vec<ColorInfo> {
        let mut colors = Vec::new();

        for cap in self.patterns.named.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                let name = m.as_str().to_lowercase();
                if let Some(color) = named_color(&name) {
                    colors.push(ColorInfo {
                        color,
                        range: ColorRange::single_line(
                            line_num,
                            m.start() as u32,
                            m.end() as u32,
                        ),
                        format: ColorFormat::Named,
                        original: m.as_str().to_string(),
                    });
                }
            }
        }

        colors
    }

    /// Invalidate cache
    pub fn invalidate(&self, file: &PathBuf) {
        self.cache.write().remove(file);
    }
}

impl Default for ColorPickerService {
    fn default() -> Self {
        Self::new()
    }
}

/// Color patterns
struct ColorPatterns {
    hex: Regex,
    rgb: Regex,
    hsl: Regex,
    named: Regex,
}

impl ColorPatterns {
    fn new() -> Self {
        Self {
            hex: Regex::new(r"#(?:[0-9a-fA-F]{3,4}){1,2}\b").unwrap(),
            rgb: Regex::new(r"rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})(?:\s*,\s*([\d.]+))?\s*\)").unwrap(),
            hsl: Regex::new(r"hsla?\(\s*([\d.]+)\s*,\s*([\d.]+%?)\s*,\s*([\d.]+%?)(?:\s*,\s*([\d.]+))?\s*\)").unwrap(),
            named: Regex::new(r"\b(red|blue|green|yellow|orange|purple|pink|white|black|gray|grey|cyan|magenta)\b").unwrap(),
        }
    }
}

/// Color information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorInfo {
    /// Parsed color
    pub color: Color,
    /// Range in document
    pub range: ColorRange,
    /// Original format
    pub format: ColorFormat,
    /// Original text
    pub original: String,
}

/// Color range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl ColorRange {
    pub fn single_line(line: u32, start_col: u32, end_col: u32) -> Self {
        Self { start_line: line, start_col, end_line: line, end_col }
    }
}

/// Color format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorFormat {
    Hex,
    Rgb,
    Rgba,
    Hsl,
    Hsla,
    Named,
}

/// Color value
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

impl Color {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            4 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                let a = u8::from_str_radix(&hex[3..4].repeat(2), 16).ok()?;
                Some(Self::rgba(r, g, b, a as f32 / 255.0))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::rgba(r, g, b, a as f32 / 255.0))
            }
            _ => None,
        }
    }

    pub fn from_hsl(h: f32, s: f32, l: f32, a: f32) -> Option<Self> {
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;

        let (r, g, b) = match h as i32 {
            0..=59 => (c, x, 0.0),
            60..=119 => (x, c, 0.0),
            120..=179 => (0.0, c, x),
            180..=239 => (0.0, x, c),
            240..=299 => (x, 0.0, c),
            300..=359 => (c, 0.0, x),
            _ => (0.0, 0.0, 0.0),
        };

        Some(Self::rgba(
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
            a,
        ))
    }

    pub fn to_hex(&self) -> String {
        if (self.a - 1.0).abs() < f32::EPSILON {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, (self.a * 255.0) as u8)
        }
    }

    pub fn to_rgb(&self) -> String {
        if (self.a - 1.0).abs() < f32::EPSILON {
            format!("rgb({}, {}, {})", self.r, self.g, self.b)
        } else {
            format!("rgba({}, {}, {}, {})", self.r, self.g, self.b, self.a)
        }
    }

    pub fn to_hsl(&self) -> String {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;

        if (max - min).abs() < f32::EPSILON {
            if (self.a - 1.0).abs() < f32::EPSILON {
                format!("hsl(0, 0%, {:.0}%)", l * 100.0)
            } else {
                format!("hsla(0, 0%, {:.0}%, {})", l * 100.0, self.a)
            }
        } else {
            let d = max - min;
            let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };

            let h = if (max - r).abs() < f32::EPSILON {
                (g - b) / d + if g < b { 6.0 } else { 0.0 }
            } else if (max - g).abs() < f32::EPSILON {
                (b - r) / d + 2.0
            } else {
                (r - g) / d + 4.0
            };

            let h = h * 60.0;

            if (self.a - 1.0).abs() < f32::EPSILON {
                format!("hsl({:.0}, {:.0}%, {:.0}%)", h, s * 100.0, l * 100.0)
            } else {
                format!("hsla({:.0}, {:.0}%, {:.0}%, {})", h, s * 100.0, l * 100.0, self.a)
            }
        }
    }

    /// Luminance for contrast calculations
    pub fn luminance(&self) -> f32 {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
}

/// Color picker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPickerConfig {
    /// Enable color detection
    pub enabled: bool,
    /// Detect hex colors
    pub detect_hex: bool,
    /// Detect rgb/rgba colors
    pub detect_rgb: bool,
    /// Detect hsl/hsla colors
    pub detect_hsl: bool,
    /// Detect named colors
    pub detect_named: bool,
    /// Default format for new colors
    pub default_format: ColorFormat,
}

impl Default for ColorPickerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            detect_hex: true,
            detect_rgb: true,
            detect_hsl: true,
            detect_named: true,
            default_format: ColorFormat::Hex,
        }
    }
}

/// Color picker event
#[derive(Debug, Clone)]
pub enum ColorPickerEvent {
    ColorsUpdated { file: PathBuf },
    ColorChanged { file: PathBuf, color: Color },
}

/// Get named color
fn named_color(name: &str) -> Option<Color> {
    match name {
        "red" => Some(Color::rgb(255, 0, 0)),
        "green" => Some(Color::rgb(0, 128, 0)),
        "blue" => Some(Color::rgb(0, 0, 255)),
        "yellow" => Some(Color::rgb(255, 255, 0)),
        "orange" => Some(Color::rgb(255, 165, 0)),
        "purple" => Some(Color::rgb(128, 0, 128)),
        "pink" => Some(Color::rgb(255, 192, 203)),
        "white" => Some(Color::rgb(255, 255, 255)),
        "black" => Some(Color::rgb(0, 0, 0)),
        "gray" | "grey" => Some(Color::rgb(128, 128, 128)),
        "cyan" => Some(Color::rgb(0, 255, 255)),
        "magenta" => Some(Color::rgb(255, 0, 255)),
        _ => None,
    }
}
