//! Smooth scrolling animation for the editor
//!
//! Provides smooth, animated scrolling with easing functions.

use std::time::{Duration, Instant};

/// Scroll animation state
#[derive(Debug, Clone)]
pub struct ScrollAnimation {
    /// Start scroll position
    start_offset: f32,
    /// Target scroll position
    target_offset: f32,
    /// Current scroll position
    current_offset: f32,
    /// Animation start time
    start_time: Option<Instant>,
    /// Animation duration
    duration: Duration,
    /// Easing function
    easing: EasingFunction,
    /// Is animation active?
    active: bool,
}

/// Easing functions for smooth scrolling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasingFunction {
    /// Linear interpolation
    Linear,
    /// Ease out (decelerating)
    EaseOut,
    /// Ease in-out (accelerating then decelerating)
    EaseInOut,
    /// Cubic ease out (smoother deceleration)
    CubicOut,
    /// Exponential decay (natural feeling)
    ExpoOut,
}

impl EasingFunction {
    /// Apply the easing function to a progress value (0.0 - 1.0)
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            EasingFunction::Linear => t,
            EasingFunction::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            EasingFunction::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            EasingFunction::CubicOut => 1.0 - (1.0 - t).powi(3),
            EasingFunction::ExpoOut => {
                if t >= 1.0 {
                    1.0
                } else {
                    1.0 - 2.0_f32.powf(-10.0 * t)
                }
            }
        }
    }
}

impl ScrollAnimation {
    /// Create a new scroll animation
    pub fn new() -> Self {
        Self {
            start_offset: 0.0,
            target_offset: 0.0,
            current_offset: 0.0,
            start_time: None,
            duration: Duration::from_millis(150),
            easing: EasingFunction::CubicOut,
            active: false,
        }
    }

    /// Create with custom duration and easing
    pub fn with_settings(duration: Duration, easing: EasingFunction) -> Self {
        Self {
            start_offset: 0.0,
            target_offset: 0.0,
            current_offset: 0.0,
            start_time: None,
            duration,
            easing,
            active: false,
        }
    }

    /// Start animating to a new target offset
    pub fn scroll_to(&mut self, target: f32) {
        if (self.target_offset - target).abs() < 0.1 {
            return; // Already at target
        }

        self.start_offset = self.current_offset;
        self.target_offset = target;
        self.start_time = Some(Instant::now());
        self.active = true;
    }

    /// Scroll by a delta amount
    pub fn scroll_by(&mut self, delta: f32) {
        self.scroll_to(self.target_offset + delta);
    }

    /// Update the animation and return the current offset
    pub fn update(&mut self) -> f32 {
        if !self.active {
            return self.current_offset;
        }

        let Some(start_time) = self.start_time else {
            self.active = false;
            return self.current_offset;
        };

        let elapsed = start_time.elapsed();
        
        if elapsed >= self.duration {
            // Animation complete
            self.current_offset = self.target_offset;
            self.active = false;
            self.start_time = None;
            return self.current_offset;
        }

        // Calculate progress
        let progress = elapsed.as_secs_f32() / self.duration.as_secs_f32();
        let eased_progress = self.easing.apply(progress);

        // Interpolate
        self.current_offset = self.start_offset + (self.target_offset - self.start_offset) * eased_progress;
        self.current_offset
    }

    /// Get current offset without updating
    pub fn current(&self) -> f32 {
        self.current_offset
    }

    /// Get target offset
    pub fn target(&self) -> f32 {
        self.target_offset
    }

    /// Check if animation is active
    pub fn is_animating(&self) -> bool {
        self.active
    }

    /// Immediately jump to an offset (no animation)
    pub fn jump_to(&mut self, offset: f32) {
        self.start_offset = offset;
        self.target_offset = offset;
        self.current_offset = offset;
        self.active = false;
        self.start_time = None;
    }

    /// Set animation duration
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }

    /// Set easing function
    pub fn set_easing(&mut self, easing: EasingFunction) {
        self.easing = easing;
    }

    /// Stop the animation at current position
    pub fn stop(&mut self) {
        self.target_offset = self.current_offset;
        self.active = false;
        self.start_time = None;
    }
}

impl Default for ScrollAnimation {
    fn default() -> Self {
        Self::new()
    }
}

/// Scroll state for both vertical and horizontal scrolling
#[derive(Debug, Clone)]
pub struct ScrollState {
    /// Vertical scroll animation
    pub vertical: ScrollAnimation,
    /// Horizontal scroll animation
    pub horizontal: ScrollAnimation,
    /// Content size (height, width)
    pub content_size: (f32, f32),
    /// Viewport size (height, width)
    pub viewport_size: (f32, f32),
    /// Line height for line-based scrolling
    pub line_height: f32,
}

impl ScrollState {
    /// Create new scroll state
    pub fn new(viewport_height: f32, viewport_width: f32, line_height: f32) -> Self {
        Self {
            vertical: ScrollAnimation::new(),
            horizontal: ScrollAnimation::new(),
            content_size: (0.0, 0.0),
            viewport_size: (viewport_height, viewport_width),
            line_height,
        }
    }

    /// Update content size
    pub fn set_content_size(&mut self, height: f32, width: f32) {
        self.content_size = (height, width);
    }

    /// Update viewport size
    pub fn set_viewport_size(&mut self, height: f32, width: f32) {
        self.viewport_size = (height, width);
    }

    /// Scroll down by lines
    pub fn scroll_down(&mut self, lines: usize) {
        let delta = lines as f32 * self.line_height;
        let max_scroll = (self.content_size.0 - self.viewport_size.0).max(0.0);
        let new_target = (self.vertical.target() + delta).min(max_scroll);
        self.vertical.scroll_to(new_target);
    }

    /// Scroll up by lines
    pub fn scroll_up(&mut self, lines: usize) {
        let delta = lines as f32 * self.line_height;
        let new_target = (self.vertical.target() - delta).max(0.0);
        self.vertical.scroll_to(new_target);
    }

    /// Scroll to a specific line
    pub fn scroll_to_line(&mut self, line: usize) {
        let target = line as f32 * self.line_height;
        let max_scroll = (self.content_size.0 - self.viewport_size.0).max(0.0);
        self.vertical.scroll_to(target.min(max_scroll));
    }

    /// Ensure a line is visible (scroll if needed)
    pub fn ensure_line_visible(&mut self, line: usize) {
        let line_top = line as f32 * self.line_height;
        let line_bottom = line_top + self.line_height;
        
        let current = self.vertical.target();
        let viewport_bottom = current + self.viewport_size.0;

        if line_top < current {
            // Line is above viewport, scroll up
            self.vertical.scroll_to(line_top);
        } else if line_bottom > viewport_bottom {
            // Line is below viewport, scroll down
            let target = line_bottom - self.viewport_size.0;
            self.vertical.scroll_to(target.max(0.0));
        }
    }

    /// Page down
    pub fn page_down(&mut self) {
        let page_size = self.viewport_size.0 - self.line_height; // Leave one line overlap
        let max_scroll = (self.content_size.0 - self.viewport_size.0).max(0.0);
        let new_target = (self.vertical.target() + page_size).min(max_scroll);
        self.vertical.scroll_to(new_target);
    }

    /// Page up
    pub fn page_up(&mut self) {
        let page_size = self.viewport_size.0 - self.line_height;
        let new_target = (self.vertical.target() - page_size).max(0.0);
        self.vertical.scroll_to(new_target);
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.vertical.scroll_to(0.0);
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        let max_scroll = (self.content_size.0 - self.viewport_size.0).max(0.0);
        self.vertical.scroll_to(max_scroll);
    }

    /// Scroll horizontally
    pub fn scroll_horizontal(&mut self, delta: f32) {
        let max_scroll = (self.content_size.1 - self.viewport_size.1).max(0.0);
        let new_target = (self.horizontal.target() + delta).clamp(0.0, max_scroll);
        self.horizontal.scroll_to(new_target);
    }

    /// Update both animations
    pub fn update(&mut self) -> (f32, f32) {
        (self.vertical.update(), self.horizontal.update())
    }

    /// Check if any animation is active
    pub fn is_animating(&self) -> bool {
        self.vertical.is_animating() || self.horizontal.is_animating()
    }

    /// Get first visible line
    pub fn first_visible_line(&self) -> usize {
        (self.vertical.current() / self.line_height).floor() as usize
    }

    /// Get number of visible lines
    pub fn visible_lines(&self) -> usize {
        (self.viewport_size.0 / self.line_height).ceil() as usize + 1
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new(600.0, 800.0, 20.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_functions() {
        // All easing functions should map 0 -> 0 and 1 -> 1
        for easing in [
            EasingFunction::Linear,
            EasingFunction::EaseOut,
            EasingFunction::EaseInOut,
            EasingFunction::CubicOut,
            EasingFunction::ExpoOut,
        ] {
            assert!((easing.apply(0.0) - 0.0).abs() < 0.001, "{:?} at 0", easing);
            assert!((easing.apply(1.0) - 1.0).abs() < 0.001, "{:?} at 1", easing);
        }
    }

    #[test]
    fn test_scroll_animation() {
        let mut anim = ScrollAnimation::new();
        
        assert!(!anim.is_animating());
        assert_eq!(anim.current(), 0.0);
        
        anim.scroll_to(100.0);
        assert!(anim.is_animating());
        assert_eq!(anim.target(), 100.0);
    }

    #[test]
    fn test_scroll_state() {
        let mut state = ScrollState::new(600.0, 800.0, 20.0);
        state.set_content_size(2000.0, 1000.0);
        
        assert_eq!(state.first_visible_line(), 0);
        
        state.scroll_down(5);
        assert_eq!(state.vertical.target(), 100.0);
    }

    #[test]
    fn test_ensure_line_visible() {
        let mut state = ScrollState::new(600.0, 800.0, 20.0);
        state.set_content_size(2000.0, 1000.0);
        
        // Line 50 is at y=1000, which is outside initial viewport
        state.ensure_line_visible(50);
        
        // Should scroll to show line 50
        let target = state.vertical.target();
        assert!(target > 0.0);
    }
}
