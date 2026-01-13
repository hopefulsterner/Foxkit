//! # Foxkit Editor Scroll
//!
//! Smooth scrolling and scroll decorations.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Editor scroll service
pub struct EditorScrollService {
    /// Scroll state
    state: RwLock<ScrollState>,
    /// Configuration
    config: RwLock<ScrollConfig>,
}

impl EditorScrollService {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ScrollState::default()),
            config: RwLock::new(ScrollConfig::default()),
        }
    }

    /// Configure
    pub fn configure(&self, config: ScrollConfig) {
        *self.config.write() = config;
    }

    /// Get config
    pub fn config(&self) -> ScrollConfig {
        self.config.read().clone()
    }

    /// Get scroll state
    pub fn state(&self) -> ScrollState {
        self.state.read().clone()
    }

    /// Set scroll position
    pub fn set_scroll_position(&self, top: f64, left: f64) {
        let mut state = self.state.write();
        state.scroll_top = top;
        state.scroll_left = left;
    }

    /// Scroll to line
    pub fn scroll_to_line(&self, line: u32, position: ScrollPosition) -> ScrollAnimation {
        let config = self.config.read();
        let target_top = self.calculate_scroll_top(line, position);

        if config.smooth_scrolling {
            ScrollAnimation {
                target_top,
                duration_ms: config.scroll_duration_ms,
                easing: config.easing,
            }
        } else {
            ScrollAnimation {
                target_top,
                duration_ms: 0,
                easing: Easing::Linear,
            }
        }
    }

    fn calculate_scroll_top(&self, line: u32, position: ScrollPosition) -> f64 {
        let config = self.config.read();
        let state = self.state.read();
        let line_height = config.line_height;
        let viewport_height = state.viewport_height;

        let line_top = line as f64 * line_height;

        match position {
            ScrollPosition::Top => line_top - config.scroll_margin as f64 * line_height,
            ScrollPosition::Center => line_top - (viewport_height / 2.0) + (line_height / 2.0),
            ScrollPosition::Bottom => {
                line_top - viewport_height + config.scroll_margin as f64 * line_height
            }
            ScrollPosition::Reveal => {
                // Only scroll if line is outside viewport
                let current_top = state.scroll_top;
                let current_bottom = current_top + viewport_height;

                if line_top < current_top {
                    line_top - config.scroll_margin as f64 * line_height
                } else if line_top + line_height > current_bottom {
                    line_top + line_height - viewport_height
                        + config.scroll_margin as f64 * line_height
                } else {
                    current_top // No scroll needed
                }
            }
        }
    }

    /// Scroll by delta
    pub fn scroll_by(&self, delta_x: f64, delta_y: f64) -> ScrollAnimation {
        let config = self.config.read();
        let state = self.state.read();

        let target_top = (state.scroll_top + delta_y).max(0.0);
        let target_left = (state.scroll_left + delta_x).max(0.0);

        drop(state);
        self.set_scroll_position(target_top, target_left);

        if config.smooth_scrolling {
            ScrollAnimation {
                target_top,
                duration_ms: config.scroll_duration_ms / 2, // Faster for incremental
                easing: config.easing,
            }
        } else {
            ScrollAnimation {
                target_top,
                duration_ms: 0,
                easing: Easing::Linear,
            }
        }
    }

    /// Scroll page up
    pub fn page_up(&self) -> ScrollAnimation {
        let state = self.state.read();
        let page_size = state.viewport_height * 0.9;
        drop(state);
        self.scroll_by(0.0, -page_size)
    }

    /// Scroll page down
    pub fn page_down(&self) -> ScrollAnimation {
        let state = self.state.read();
        let page_size = state.viewport_height * 0.9;
        drop(state);
        self.scroll_by(0.0, page_size)
    }

    /// Scroll half page up
    pub fn half_page_up(&self) -> ScrollAnimation {
        let state = self.state.read();
        let half_page = state.viewport_height / 2.0;
        drop(state);
        self.scroll_by(0.0, -half_page)
    }

    /// Scroll half page down
    pub fn half_page_down(&self) -> ScrollAnimation {
        let state = self.state.read();
        let half_page = state.viewport_height / 2.0;
        drop(state);
        self.scroll_by(0.0, half_page)
    }

    /// Update viewport size
    pub fn set_viewport(&self, width: f64, height: f64) {
        let mut state = self.state.write();
        state.viewport_width = width;
        state.viewport_height = height;
    }

    /// Update content size
    pub fn set_content_size(&self, width: f64, height: f64) {
        let mut state = self.state.write();
        state.content_width = width;
        state.content_height = height;
    }

    /// Get visible line range
    pub fn visible_lines(&self) -> (u32, u32) {
        let config = self.config.read();
        let state = self.state.read();

        let first_line = (state.scroll_top / config.line_height).floor() as u32;
        let visible_lines = (state.viewport_height / config.line_height).ceil() as u32;
        let last_line = first_line + visible_lines;

        (first_line, last_line)
    }

    /// Check if line is visible
    pub fn is_line_visible(&self, line: u32) -> bool {
        let (first, last) = self.visible_lines();
        line >= first && line <= last
    }

    /// Calculate scrollbar
    pub fn scrollbar(&self) -> Scrollbar {
        let state = self.state.read();

        // Vertical
        let v_ratio = state.viewport_height / state.content_height.max(1.0);
        let v_thumb_height = (v_ratio * state.viewport_height).max(20.0);
        let v_track_space = state.viewport_height - v_thumb_height;
        let v_scroll_ratio = state.scroll_top / (state.content_height - state.viewport_height).max(1.0);
        let v_thumb_top = v_scroll_ratio * v_track_space;

        // Horizontal
        let h_ratio = state.viewport_width / state.content_width.max(1.0);
        let h_thumb_width = (h_ratio * state.viewport_width).max(20.0);
        let h_track_space = state.viewport_width - h_thumb_width;
        let h_scroll_ratio = state.scroll_left / (state.content_width - state.viewport_width).max(1.0);
        let h_thumb_left = h_scroll_ratio * h_track_space;

        Scrollbar {
            vertical: ScrollbarDimension {
                thumb_size: v_thumb_height,
                thumb_position: v_thumb_top,
                visible: v_ratio < 1.0,
            },
            horizontal: ScrollbarDimension {
                thumb_size: h_thumb_width,
                thumb_position: h_thumb_left,
                visible: h_ratio < 1.0,
            },
        }
    }
}

impl Default for EditorScrollService {
    fn default() -> Self {
        Self::new()
    }
}

/// Scroll state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScrollState {
    /// Current scroll top
    pub scroll_top: f64,
    /// Current scroll left
    pub scroll_left: f64,
    /// Viewport width
    pub viewport_width: f64,
    /// Viewport height
    pub viewport_height: f64,
    /// Content width
    pub content_width: f64,
    /// Content height
    pub content_height: f64,
}

/// Scroll configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollConfig {
    /// Enable smooth scrolling
    pub smooth_scrolling: bool,
    /// Scroll duration in ms
    pub scroll_duration_ms: u32,
    /// Easing function
    pub easing: Easing,
    /// Line height
    pub line_height: f64,
    /// Scroll margin (lines above/below cursor)
    pub scroll_margin: u32,
    /// Fast scroll sensitivity
    pub fast_scroll_sensitivity: f64,
    /// Mouse wheel scroll sensitivity
    pub wheel_sensitivity: f64,
    /// Scrollbar visibility
    pub scrollbar_visibility: ScrollbarVisibility,
    /// Scrollbar size
    pub scrollbar_size: u32,
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            smooth_scrolling: true,
            scroll_duration_ms: 100,
            easing: Easing::EaseOutCubic,
            line_height: 20.0,
            scroll_margin: 5,
            fast_scroll_sensitivity: 5.0,
            wheel_sensitivity: 1.0,
            scrollbar_visibility: ScrollbarVisibility::Auto,
            scrollbar_size: 10,
        }
    }
}

/// Scroll position target
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ScrollPosition {
    /// Line at top of viewport
    Top,
    /// Line at center of viewport
    Center,
    /// Line at bottom of viewport
    Bottom,
    /// Minimal scroll to reveal line
    Reveal,
}

/// Scroll animation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollAnimation {
    pub target_top: f64,
    pub duration_ms: u32,
    pub easing: Easing,
}

impl ScrollAnimation {
    pub fn interpolate(&self, progress: f64) -> f64 {
        self.easing.apply(progress) * self.target_top
    }
}

/// Easing function
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseOutCubic,
    EaseOutQuart,
}

impl Easing {
    pub fn apply(&self, t: f64) -> f64 {
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => t * (2.0 - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Self::EaseOutCubic => {
                let t = t - 1.0;
                t * t * t + 1.0
            }
            Self::EaseOutQuart => {
                let t = t - 1.0;
                1.0 - t * t * t * t
            }
        }
    }
}

/// Scrollbar visibility
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ScrollbarVisibility {
    /// Always visible
    Visible,
    /// Hide when not scrolling
    Auto,
    /// Always hidden
    Hidden,
}

/// Scrollbar dimensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scrollbar {
    pub vertical: ScrollbarDimension,
    pub horizontal: ScrollbarDimension,
}

/// Single scrollbar dimension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollbarDimension {
    pub thumb_size: f64,
    pub thumb_position: f64,
    pub visible: bool,
}

/// Scroll decoration (for minimap/overview ruler)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollDecoration {
    /// Line number
    pub line: u32,
    /// Color
    pub color: String,
    /// Lane
    pub lane: DecorationLane,
}

/// Decoration lane
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DecorationLane {
    Left,
    Center,
    Right,
    Full,
}
