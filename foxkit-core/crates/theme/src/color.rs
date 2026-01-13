//! Color types

use serde::{Deserialize, Serialize};

/// RGBA Color (0-255)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const TRANSPARENT: Self = Self { r: 0, g: 0, b: 0, a: 0 };
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };

    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    pub fn to_f32(&self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
        }
    }
}

/// HSLA Color
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Hsla {
    pub h: f32, // 0-360
    pub s: f32, // 0-1
    pub l: f32, // 0-1
    pub a: f32, // 0-1
}

impl Hsla {
    pub fn new(h: f32, s: f32, l: f32, a: f32) -> Self {
        Self { h, s, l, a }
    }

    pub fn hsl(h: f32, s: f32, l: f32) -> Self {
        Self::new(h, s, l, 1.0)
    }

    pub fn to_rgba(&self) -> Rgba {
        let c = (1.0 - (2.0 * self.l - 1.0).abs()) * self.s;
        let x = c * (1.0 - ((self.h / 60.0) % 2.0 - 1.0).abs());
        let m = self.l - c / 2.0;

        let (r, g, b) = match (self.h / 60.0) as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Rgba::new(
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }
}

/// Color (wrapper for serialization flexibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct Color(pub Rgba);

impl Color {
    pub const TRANSPARENT: Self = Self(Rgba::TRANSPARENT);
    pub const BLACK: Self = Self(Rgba::BLACK);
    pub const WHITE: Self = Self(Rgba::WHITE);

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(Rgba::rgb(r, g, b))
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(Rgba::new(r, g, b, a))
    }

    pub fn hex(hex: u32) -> Self {
        Self(Rgba::rgb(
            ((hex >> 16) & 0xFF) as u8,
            ((hex >> 8) & 0xFF) as u8,
            (hex & 0xFF) as u8,
        ))
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.trim_start_matches('#');
        
        match s.len() {
            3 => {
                // #RGB
                let r = u8::from_str_radix(&s[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&s[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&s[2..3], 16).ok()? * 17;
                Some(Self::rgb(r, g, b))
            }
            4 => {
                // #RGBA
                let r = u8::from_str_radix(&s[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&s[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&s[2..3], 16).ok()? * 17;
                let a = u8::from_str_radix(&s[3..4], 16).ok()? * 17;
                Some(Self::rgba(r, g, b, a))
            }
            6 => {
                // #RRGGBB
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            8 => {
                // #RRGGBBAA
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                let a = u8::from_str_radix(&s[6..8], 16).ok()?;
                Some(Self::rgba(r, g, b, a))
            }
            _ => None,
        }
    }

    pub fn with_alpha(&self, a: u8) -> Self {
        let mut color = self.0;
        color.a = a;
        Self(color)
    }

    pub fn lighten(&self, amount: f32) -> Self {
        let [r, g, b, a] = self.0.to_f32();
        Self(Rgba::new(
            ((r + amount).clamp(0.0, 1.0) * 255.0) as u8,
            ((g + amount).clamp(0.0, 1.0) * 255.0) as u8,
            ((b + amount).clamp(0.0, 1.0) * 255.0) as u8,
            (a * 255.0) as u8,
        ))
    }

    pub fn darken(&self, amount: f32) -> Self {
        self.lighten(-amount)
    }

    pub fn to_f32(&self) -> [f32; 4] {
        self.0.to_f32()
    }
}

impl From<Rgba> for Color {
    fn from(rgba: Rgba) -> Self {
        Self(rgba)
    }
}

impl From<Color> for Rgba {
    fn from(color: Color) -> Self {
        color.0
    }
}

impl From<String> for Color {
    fn from(s: String) -> Self {
        Self::from_hex(&s).unwrap_or(Self::TRANSPARENT)
    }
}

impl From<Color> for String {
    fn from(color: Color) -> Self {
        color.0.to_hex()
    }
}
