//! Panel container

use crate::ViewId;

/// Panel position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelPosition {
    Left,
    Right,
    Bottom,
}

impl PanelPosition {
    pub fn is_horizontal(&self) -> bool {
        matches!(self, PanelPosition::Bottom)
    }

    pub fn is_vertical(&self) -> bool {
        matches!(self, PanelPosition::Left | PanelPosition::Right)
    }
}

/// A container for panel views
#[derive(Debug)]
pub struct PanelContainer {
    position: PanelPosition,
    views: Vec<ViewId>,
}

impl PanelContainer {
    pub fn new(position: PanelPosition) -> Self {
        Self {
            position,
            views: Vec::new(),
        }
    }

    pub fn position(&self) -> PanelPosition {
        self.position
    }

    pub fn add_view(&mut self, id: ViewId) {
        if !self.views.contains(&id) {
            self.views.push(id);
        }
    }

    pub fn remove_view(&mut self, id: &ViewId) {
        self.views.retain(|v| v != id);
    }

    pub fn views(&self) -> Vec<&ViewId> {
        self.views.iter().collect()
    }

    pub fn reorder(&mut self, from: usize, to: usize) {
        if from < self.views.len() && to < self.views.len() {
            let view = self.views.remove(from);
            self.views.insert(to, view);
        }
    }

    pub fn contains(&self, id: &ViewId) -> bool {
        self.views.contains(id)
    }

    pub fn count(&self) -> usize {
        self.views.len()
    }
}

/// Panel layout
#[derive(Debug, Clone)]
pub struct PanelLayout {
    /// Left panel width
    pub left_width: f32,
    /// Right panel width  
    pub right_width: f32,
    /// Bottom panel height
    pub bottom_height: f32,
    /// Left panel visible
    pub left_visible: bool,
    /// Right panel visible
    pub right_visible: bool,
    /// Bottom panel visible
    pub bottom_visible: bool,
}

impl Default for PanelLayout {
    fn default() -> Self {
        Self {
            left_width: 250.0,
            right_width: 300.0,
            bottom_height: 200.0,
            left_visible: true,
            right_visible: false,
            bottom_visible: true,
        }
    }
}

impl PanelLayout {
    /// Calculate editor area bounds
    pub fn editor_bounds(&self, total_width: f32, total_height: f32) -> EditorBounds {
        let left = if self.left_visible { self.left_width } else { 0.0 };
        let right = if self.right_visible { self.right_width } else { 0.0 };
        let bottom = if self.bottom_visible { self.bottom_height } else { 0.0 };

        EditorBounds {
            x: left,
            y: 0.0,
            width: (total_width - left - right).max(0.0),
            height: (total_height - bottom).max(0.0),
        }
    }
}

/// Editor area bounds
#[derive(Debug, Clone)]
pub struct EditorBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
