//! Event handling

use crate::ElementId;

/// UI Event
#[derive(Debug, Clone)]
pub struct Event {
    /// Event type
    pub kind: EventKind,
    /// Target element
    pub target: Option<ElementId>,
    /// Event phase
    pub phase: EventPhase,
    /// Is propagation stopped?
    pub stopped: bool,
    /// Is default prevented?
    pub default_prevented: bool,
}

impl Event {
    pub fn new(kind: EventKind) -> Self {
        Self {
            kind,
            target: None,
            phase: EventPhase::Bubble,
            stopped: false,
            default_prevented: false,
        }
    }

    pub fn stop_propagation(&mut self) {
        self.stopped = true;
    }

    pub fn prevent_default(&mut self) {
        self.default_prevented = true;
    }
}

/// Event kind
#[derive(Debug, Clone)]
pub enum EventKind {
    Mouse(MouseEvent),
    Key(KeyEvent),
    Focus(FocusEvent),
    Scroll(ScrollEvent),
    Custom(String, serde_json::Value),
}

/// Event phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

/// Mouse event
#[derive(Debug, Clone)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub x: f32,
    pub y: f32,
    pub button: MouseButton,
    pub modifiers: Modifiers,
}

/// Mouse event kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    Down,
    Up,
    Move,
    Enter,
    Leave,
    Click,
    DoubleClick,
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

/// Keyboard event
#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub kind: KeyEventKind,
    pub key: Key,
    pub modifiers: Modifiers,
    pub text: Option<String>,
}

/// Key event kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventKind {
    Down,
    Up,
    Press,
}

/// Key
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    // Numbers
    Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // Special
    Escape, Tab, CapsLock, Shift, Control, Alt, Meta,
    Space, Enter, Backspace, Delete,
    Insert, Home, End, PageUp, PageDown,
    Left, Right, Up, Down,
    // Other
    Named(String),
}

/// Modifier keys
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Modifiers {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn shift() -> Self {
        Self { shift: true, ..Default::default() }
    }

    pub fn ctrl() -> Self {
        Self { ctrl: true, ..Default::default() }
    }

    pub fn alt() -> Self {
        Self { alt: true, ..Default::default() }
    }

    pub fn meta() -> Self {
        Self { meta: true, ..Default::default() }
    }

    pub fn cmd_or_ctrl() -> Self {
        #[cfg(target_os = "macos")]
        return Self::meta();
        #[cfg(not(target_os = "macos"))]
        return Self::ctrl();
    }

    pub fn any(&self) -> bool {
        self.shift || self.ctrl || self.alt || self.meta
    }
}

/// Focus event
#[derive(Debug, Clone)]
pub struct FocusEvent {
    pub kind: FocusEventKind,
    pub related: Option<ElementId>,
}

/// Focus event kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusEventKind {
    Focus,
    Blur,
}

/// Scroll event
#[derive(Debug, Clone)]
pub struct ScrollEvent {
    pub delta_x: f32,
    pub delta_y: f32,
    pub modifiers: Modifiers,
}
