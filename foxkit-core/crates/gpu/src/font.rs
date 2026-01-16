//! Font management and rasterization using fontdue
//!
//! This module provides:
//! - Font loading from files and embedded fonts
//! - Glyph rasterization with subpixel rendering
//! - Font metrics calculation
//! - Font family and style management

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use fontdue::{Font, FontSettings, Metrics};
use parking_lot::RwLock;
use anyhow::{Result, anyhow};

/// Font weight
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontWeight {
    Thin,       // 100
    ExtraLight, // 200
    Light,      // 300
    Regular,    // 400
    Medium,     // 500
    SemiBold,   // 600
    Bold,       // 700
    ExtraBold,  // 800
    Black,      // 900
}

impl FontWeight {
    pub fn to_numeric(&self) -> u16 {
        match self {
            FontWeight::Thin => 100,
            FontWeight::ExtraLight => 200,
            FontWeight::Light => 300,
            FontWeight::Regular => 400,
            FontWeight::Medium => 500,
            FontWeight::SemiBold => 600,
            FontWeight::Bold => 700,
            FontWeight::ExtraBold => 800,
            FontWeight::Black => 900,
        }
    }

    pub fn from_numeric(weight: u16) -> Self {
        match weight {
            0..=150 => FontWeight::Thin,
            151..=250 => FontWeight::ExtraLight,
            251..=350 => FontWeight::Light,
            351..=450 => FontWeight::Regular,
            451..=550 => FontWeight::Medium,
            551..=650 => FontWeight::SemiBold,
            651..=750 => FontWeight::Bold,
            751..=850 => FontWeight::ExtraBold,
            _ => FontWeight::Black,
        }
    }
}

/// Font style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

/// Font key for looking up fonts
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FontKey {
    pub family: String,
    pub weight: FontWeight,
    pub style: FontStyle,
}

impl FontKey {
    pub fn new(family: impl Into<String>, weight: FontWeight, style: FontStyle) -> Self {
        Self {
            family: family.into(),
            weight,
            style,
        }
    }

    pub fn regular(family: impl Into<String>) -> Self {
        Self::new(family, FontWeight::Regular, FontStyle::Normal)
    }

    pub fn bold(family: impl Into<String>) -> Self {
        Self::new(family, FontWeight::Bold, FontStyle::Normal)
    }

    pub fn italic(family: impl Into<String>) -> Self {
        Self::new(family, FontWeight::Regular, FontStyle::Italic)
    }
}

/// Rasterized glyph data
#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    /// Glyph bitmap (grayscale, 1 byte per pixel)
    pub bitmap: Vec<u8>,
    /// Bitmap width
    pub width: u32,
    /// Bitmap height
    pub height: u32,
    /// Metrics
    pub metrics: GlyphMetrics,
}

/// Glyph metrics
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    /// Horizontal advance
    pub advance_width: f32,
    /// Horizontal bearing (offset from origin to left edge)
    pub bearing_x: f32,
    /// Vertical bearing (offset from baseline to top edge)
    pub bearing_y: f32,
    /// Glyph width
    pub width: f32,
    /// Glyph height
    pub height: f32,
}

impl From<Metrics> for GlyphMetrics {
    fn from(m: Metrics) -> Self {
        Self {
            advance_width: m.advance_width,
            bearing_x: m.xmin as f32,
            bearing_y: m.ymin as f32 + m.height as f32,
            width: m.width as f32,
            height: m.height as f32,
        }
    }
}

/// Font entry in the font system
struct FontEntry {
    font: Font,
    // Cache of rasterized glyphs: (char, size_px * 10) -> glyph
    glyph_cache: HashMap<(char, u32), RasterizedGlyph>,
}

/// Font system - manages fonts and rasterization
pub struct FontSystem {
    /// Loaded fonts
    fonts: HashMap<FontKey, FontEntry>,
    /// Default font key
    default_font: Option<FontKey>,
    /// Fallback fonts for missing glyphs
    fallback_fonts: Vec<FontKey>,
}

impl FontSystem {
    /// Create a new font system
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            default_font: None,
            fallback_fonts: Vec::new(),
        }
    }

    /// Load a font from bytes
    pub fn load_font(&mut self, key: FontKey, data: &[u8]) -> Result<()> {
        let font = Font::from_bytes(data, FontSettings::default())
            .map_err(|e| anyhow!("Failed to load font: {}", e))?;

        self.fonts.insert(key, FontEntry {
            font,
            glyph_cache: HashMap::new(),
        });

        Ok(())
    }

    /// Load a font from a file
    pub fn load_font_file(&mut self, key: FontKey, path: impl AsRef<Path>) -> Result<()> {
        let data = std::fs::read(path.as_ref())?;
        self.load_font(key, &data)
    }

    /// Set the default font
    pub fn set_default_font(&mut self, key: FontKey) {
        self.default_font = Some(key);
    }

    /// Add a fallback font
    pub fn add_fallback_font(&mut self, key: FontKey) {
        self.fallback_fonts.push(key);
    }

    /// Load embedded fallback fonts
    pub fn load_embedded_fonts(&mut self) -> Result<()> {
        // We'll embed a basic monospace font for the editor
        // For now, just set up the structure - actual fonts would be embedded via include_bytes!
        
        // Placeholder: In production, embed actual fonts like:
        // let data = include_bytes!("../../fonts/JetBrainsMono-Regular.ttf");
        // self.load_font(FontKey::regular("JetBrains Mono"), data)?;
        
        Ok(())
    }

    /// Get a font by key
    pub fn get_font(&self, key: &FontKey) -> Option<&Font> {
        self.fonts.get(key).map(|e| &e.font)
    }

    /// Rasterize a glyph
    pub fn rasterize_glyph(&mut self, key: &FontKey, ch: char, size_px: f32) -> Option<RasterizedGlyph> {
        // Cache key uses size * 10 to handle fractional sizes
        let cache_key = (ch, (size_px * 10.0) as u32);

        // Try the requested font
        if let Some(entry) = self.fonts.get_mut(key) {
            if let Some(cached) = entry.glyph_cache.get(&cache_key) {
                return Some(cached.clone());
            }

            // Rasterize the glyph
            let (metrics, bitmap) = entry.font.rasterize(ch, size_px);
            
            let glyph = RasterizedGlyph {
                bitmap,
                width: metrics.width as u32,
                height: metrics.height as u32,
                metrics: metrics.into(),
            };

            entry.glyph_cache.insert(cache_key, glyph.clone());
            return Some(glyph);
        }

        // Try fallback fonts
        for fallback_key in &self.fallback_fonts.clone() {
            if let Some(entry) = self.fonts.get_mut(fallback_key) {
                if entry.font.lookup_glyph_index(ch) != 0 {
                    if let Some(cached) = entry.glyph_cache.get(&cache_key) {
                        return Some(cached.clone());
                    }

                    let (metrics, bitmap) = entry.font.rasterize(ch, size_px);
                    
                    let glyph = RasterizedGlyph {
                        bitmap,
                        width: metrics.width as u32,
                        height: metrics.height as u32,
                        metrics: metrics.into(),
                    };

                    entry.glyph_cache.insert(cache_key, glyph.clone());
                    return Some(glyph);
                }
            }
        }

        // Try default font
        if let Some(default_key) = &self.default_font.clone() {
            if default_key != key {
                if let Some(entry) = self.fonts.get_mut(default_key) {
                    if let Some(cached) = entry.glyph_cache.get(&cache_key) {
                        return Some(cached.clone());
                    }

                    let (metrics, bitmap) = entry.font.rasterize(ch, size_px);
                    
                    let glyph = RasterizedGlyph {
                        bitmap,
                        width: metrics.width as u32,
                        height: metrics.height as u32,
                        metrics: metrics.into(),
                    };

                    entry.glyph_cache.insert(cache_key, glyph.clone());
                    return Some(glyph);
                }
            }
        }

        None
    }

    /// Get font metrics (ascender, descender, line gap)
    pub fn font_metrics(&self, key: &FontKey, size_px: f32) -> Option<FontMetrics> {
        self.fonts.get(key).map(|entry| {
            let metrics = entry.font.horizontal_line_metrics(size_px);
            metrics.map(|m| FontMetrics {
                ascender: m.ascent,
                descender: m.descent,
                line_gap: m.line_gap,
                line_height: m.new_line_size,
            }).unwrap_or(FontMetrics {
                ascender: size_px * 0.8,
                descender: size_px * -0.2,
                line_gap: 0.0,
                line_height: size_px * 1.2,
            })
        })
    }

    /// Measure text width
    pub fn measure_text(&mut self, key: &FontKey, text: &str, size_px: f32) -> f32 {
        let mut width = 0.0;

        for ch in text.chars() {
            if let Some(glyph) = self.rasterize_glyph(key, ch, size_px) {
                width += glyph.metrics.advance_width;
            } else {
                // Fallback: estimate based on font size
                width += size_px * 0.6;
            }
        }

        width
    }

    /// Clear all glyph caches
    pub fn clear_caches(&mut self) {
        for entry in self.fonts.values_mut() {
            entry.glyph_cache.clear();
        }
    }

    /// Get number of cached glyphs
    pub fn cached_glyph_count(&self) -> usize {
        self.fonts.values().map(|e| e.glyph_cache.len()).sum()
    }
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Font metrics for a specific size
#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {
    /// Distance from baseline to top of tallest glyph
    pub ascender: f32,
    /// Distance from baseline to bottom of lowest glyph (negative)
    pub descender: f32,
    /// Extra space between lines
    pub line_gap: f32,
    /// Total line height
    pub line_height: f32,
}

impl FontMetrics {
    /// Calculate line height with a multiplier
    pub fn line_height_with_multiplier(&self, multiplier: f32) -> f32 {
        (self.ascender - self.descender + self.line_gap) * multiplier
    }
}

/// Thread-safe font system wrapper
pub struct SharedFontSystem {
    inner: Arc<RwLock<FontSystem>>,
}

impl SharedFontSystem {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(FontSystem::new())),
        }
    }

    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, FontSystem> {
        self.inner.read()
    }

    pub fn write(&self) -> parking_lot::RwLockWriteGuard<'_, FontSystem> {
        self.inner.write()
    }
}

impl Default for SharedFontSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SharedFontSystem {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_key() {
        let key = FontKey::regular("JetBrains Mono");
        assert_eq!(key.family, "JetBrains Mono");
        assert_eq!(key.weight, FontWeight::Regular);
        assert_eq!(key.style, FontStyle::Normal);

        let bold_key = FontKey::bold("JetBrains Mono");
        assert_eq!(bold_key.weight, FontWeight::Bold);
    }

    #[test]
    fn test_font_weight_conversion() {
        assert_eq!(FontWeight::from_numeric(400), FontWeight::Regular);
        assert_eq!(FontWeight::from_numeric(700), FontWeight::Bold);
        assert_eq!(FontWeight::Bold.to_numeric(), 700);
    }

    #[test]
    fn test_font_system_creation() {
        let font_system = FontSystem::new();
        assert!(font_system.default_font.is_none());
        assert!(font_system.fallback_fonts.is_empty());
    }
}
