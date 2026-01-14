//! Key chord types

use std::fmt;
use serde::{Deserialize, Serialize};

/// A key chord (modifier + key)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    pub modifiers: Modifiers,
    pub key: Key,
}

impl Chord {
    pub fn new(modifiers: Modifiers, key: Key) -> Self {
        Self { modifiers, key }
    }

    /// Parse from string like "ctrl+shift+p"
    pub fn parse(s: &str) -> Option<Self> {
        let lowercased = s.to_lowercase();
        let parts: Vec<&str> = lowercased.split('+').collect();
        if parts.is_empty() {
            return None;
        }

        let mut modifiers = Modifiers::default();
        let mut key = None;

        for part in parts {
            match part.trim() {
                "ctrl" | "control" => modifiers.ctrl = true,
                "shift" => modifiers.shift = true,
                "alt" | "option" => modifiers.alt = true,
                "meta" | "cmd" | "command" | "win" | "super" => modifiers.meta = true,
                k => {
                    key = Key::from_str(k);
                }
            }
        }

        key.map(|k| Chord { modifiers, key: k })
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        
        if self.modifiers.ctrl {
            parts.push("ctrl");
        }
        if self.modifiers.shift {
            parts.push("shift");
        }
        if self.modifiers.alt {
            parts.push("alt");
        }
        if self.modifiers.meta {
            parts.push("meta");
        }
        
        parts.push(self.key.as_str());
        
        write!(f, "{}", parts.join("+"))
    }
}

/// Modifier keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Modifiers {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn ctrl() -> Self {
        Self { ctrl: true, ..Default::default() }
    }

    pub fn shift() -> Self {
        Self { shift: true, ..Default::default() }
    }

    pub fn alt() -> Self {
        Self { alt: true, ..Default::default() }
    }

    pub fn meta() -> Self {
        Self { meta: true, ..Default::default() }
    }

    /// Ctrl on Linux/Windows, Cmd on Mac
    pub fn cmd_or_ctrl() -> Self {
        #[cfg(target_os = "macos")]
        return Self::meta();
        #[cfg(not(target_os = "macos"))]
        return Self::ctrl();
    }

    pub fn any(&self) -> bool {
        self.ctrl || self.shift || self.alt || self.meta
    }
}

/// A key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    
    // Numbers
    Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,
    
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    
    // Special keys
    Escape, Tab, CapsLock, Space, Enter, Backspace, Delete,
    Insert, Home, End, PageUp, PageDown,
    Left, Right, Up, Down,
    
    // Punctuation
    Minus, Equal, BracketLeft, BracketRight, Backslash,
    Semicolon, Quote, Comma, Period, Slash, Backquote,
    
    // Numpad
    Numpad0, Numpad1, Numpad2, Numpad3, Numpad4,
    Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    NumpadAdd, NumpadSubtract, NumpadMultiply, NumpadDivide,
    NumpadDecimal, NumpadEnter,
}

impl Key {
    pub fn from_str(s: &str) -> Option<Self> {
        let key = match s.to_lowercase().as_str() {
            // Letters
            "a" => Key::A, "b" => Key::B, "c" => Key::C, "d" => Key::D,
            "e" => Key::E, "f" => Key::F, "g" => Key::G, "h" => Key::H,
            "i" => Key::I, "j" => Key::J, "k" => Key::K, "l" => Key::L,
            "m" => Key::M, "n" => Key::N, "o" => Key::O, "p" => Key::P,
            "q" => Key::Q, "r" => Key::R, "s" => Key::S, "t" => Key::T,
            "u" => Key::U, "v" => Key::V, "w" => Key::W, "x" => Key::X,
            "y" => Key::Y, "z" => Key::Z,
            
            // Numbers
            "0" => Key::Key0, "1" => Key::Key1, "2" => Key::Key2,
            "3" => Key::Key3, "4" => Key::Key4, "5" => Key::Key5,
            "6" => Key::Key6, "7" => Key::Key7, "8" => Key::Key8,
            "9" => Key::Key9,
            
            // Function keys
            "f1" => Key::F1, "f2" => Key::F2, "f3" => Key::F3,
            "f4" => Key::F4, "f5" => Key::F5, "f6" => Key::F6,
            "f7" => Key::F7, "f8" => Key::F8, "f9" => Key::F9,
            "f10" => Key::F10, "f11" => Key::F11, "f12" => Key::F12,
            
            // Special
            "escape" | "esc" => Key::Escape,
            "tab" => Key::Tab,
            "capslock" => Key::CapsLock,
            "space" => Key::Space,
            "enter" | "return" => Key::Enter,
            "backspace" => Key::Backspace,
            "delete" | "del" => Key::Delete,
            "insert" => Key::Insert,
            "home" => Key::Home,
            "end" => Key::End,
            "pageup" => Key::PageUp,
            "pagedown" => Key::PageDown,
            "left" | "arrowleft" => Key::Left,
            "right" | "arrowright" => Key::Right,
            "up" | "arrowup" => Key::Up,
            "down" | "arrowdown" => Key::Down,
            
            // Punctuation
            "-" | "minus" => Key::Minus,
            "=" | "equal" => Key::Equal,
            "[" | "bracketleft" => Key::BracketLeft,
            "]" | "bracketright" => Key::BracketRight,
            "\\" | "backslash" => Key::Backslash,
            ";" | "semicolon" => Key::Semicolon,
            "'" | "quote" => Key::Quote,
            "," | "comma" => Key::Comma,
            "." | "period" => Key::Period,
            "/" | "slash" => Key::Slash,
            "`" | "backquote" => Key::Backquote,
            
            _ => return None,
        };
        Some(key)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Key::A => "a", Key::B => "b", Key::C => "c", Key::D => "d",
            Key::E => "e", Key::F => "f", Key::G => "g", Key::H => "h",
            Key::I => "i", Key::J => "j", Key::K => "k", Key::L => "l",
            Key::M => "m", Key::N => "n", Key::O => "o", Key::P => "p",
            Key::Q => "q", Key::R => "r", Key::S => "s", Key::T => "t",
            Key::U => "u", Key::V => "v", Key::W => "w", Key::X => "x",
            Key::Y => "y", Key::Z => "z",
            
            Key::Key0 => "0", Key::Key1 => "1", Key::Key2 => "2",
            Key::Key3 => "3", Key::Key4 => "4", Key::Key5 => "5",
            Key::Key6 => "6", Key::Key7 => "7", Key::Key8 => "8",
            Key::Key9 => "9",
            
            Key::F1 => "f1", Key::F2 => "f2", Key::F3 => "f3",
            Key::F4 => "f4", Key::F5 => "f5", Key::F6 => "f6",
            Key::F7 => "f7", Key::F8 => "f8", Key::F9 => "f9",
            Key::F10 => "f10", Key::F11 => "f11", Key::F12 => "f12",
            
            Key::Escape => "escape",
            Key::Tab => "tab",
            Key::CapsLock => "capslock",
            Key::Space => "space",
            Key::Enter => "enter",
            Key::Backspace => "backspace",
            Key::Delete => "delete",
            Key::Insert => "insert",
            Key::Home => "home",
            Key::End => "end",
            Key::PageUp => "pageup",
            Key::PageDown => "pagedown",
            Key::Left => "left",
            Key::Right => "right",
            Key::Up => "up",
            Key::Down => "down",
            
            Key::Minus => "-",
            Key::Equal => "=",
            Key::BracketLeft => "[",
            Key::BracketRight => "]",
            Key::Backslash => "\\",
            Key::Semicolon => ";",
            Key::Quote => "'",
            Key::Comma => ",",
            Key::Period => ".",
            Key::Slash => "/",
            Key::Backquote => "`",
            
            Key::Numpad0 => "numpad0",
            Key::Numpad1 => "numpad1",
            Key::Numpad2 => "numpad2",
            Key::Numpad3 => "numpad3",
            Key::Numpad4 => "numpad4",
            Key::Numpad5 => "numpad5",
            Key::Numpad6 => "numpad6",
            Key::Numpad7 => "numpad7",
            Key::Numpad8 => "numpad8",
            Key::Numpad9 => "numpad9",
            Key::NumpadAdd => "numpad_add",
            Key::NumpadSubtract => "numpad_subtract",
            Key::NumpadMultiply => "numpad_multiply",
            Key::NumpadDivide => "numpad_divide",
            Key::NumpadDecimal => "numpad_decimal",
            Key::NumpadEnter => "numpad_enter",
        }
    }
}
