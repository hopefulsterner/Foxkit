//! # Foxkit UI
//!
//! Component-based UI framework.
//! Inspired by GPUI (Zed's framework) + React patterns.

pub mod component;
pub mod element;
pub mod event;
pub mod layout;
pub mod style;
pub mod view;
pub mod widget;

use std::sync::Arc;
use parking_lot::RwLock;

pub use component::{Component, Context, RenderResult};
pub use element::{Element, ElementId};
pub use event::{Event, EventPhase, MouseEvent, KeyEvent};
pub use layout::{Layout, LayoutContext, Bounds, Edge, Size};
pub use style::{Style, StyleBuilder};
pub use view::View;
pub use widget::*;

/// UI Application
pub struct App {
    /// Root view
    root: Option<Arc<RwLock<dyn View>>>,
    /// Theme
    theme: Arc<theme::Theme>,
    /// Window size
    size: Size,
    /// Focus chain
    focus: Option<ElementId>,
    /// Hover element
    hover: Option<ElementId>,
}

impl App {
    pub fn new(theme: Arc<theme::Theme>) -> Self {
        Self {
            root: None,
            theme,
            size: Size::new(800.0, 600.0),
            focus: None,
            hover: None,
        }
    }

    /// Set root view
    pub fn set_root(&mut self, view: impl View + 'static) {
        self.root = Some(Arc::new(RwLock::new(view)));
    }

    /// Resize
    pub fn resize(&mut self, width: f32, height: f32) {
        self.size = Size::new(width, height);
    }

    /// Render the UI
    pub fn render(&self) -> Option<Element> {
        let root = self.root.as_ref()?;
        let view = root.read();
        
        let mut ctx = Context {
            theme: Arc::clone(&self.theme),
            focus: self.focus,
            hover: self.hover,
        };

        Some(view.render(&mut ctx))
    }

    /// Layout the UI
    pub fn layout(&self, element: &mut Element) {
        let bounds = Bounds {
            x: 0.0,
            y: 0.0,
            width: self.size.width,
            height: self.size.height,
        };

        let mut ctx = LayoutContext::new(bounds);
        element.layout(&mut ctx);
    }

    /// Handle event
    pub fn handle_event(&mut self, event: Event) -> bool {
        // TODO: Event dispatch
        false
    }

    /// Get theme
    pub fn theme(&self) -> &Arc<theme::Theme> {
        &self.theme
    }

    /// Set focus
    pub fn set_focus(&mut self, id: Option<ElementId>) {
        self.focus = id;
    }
}

/// Reexport common types
pub mod prelude {
    pub use super::{
        Component, Context, RenderResult,
        Element, ElementId,
        Event, MouseEvent, KeyEvent,
        Layout, Bounds, Size,
        Style, StyleBuilder,
        View,
        App,
    };
    pub use super::widget::*;
}
