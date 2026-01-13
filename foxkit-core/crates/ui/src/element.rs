//! UI elements

use std::sync::atomic::{AtomicU64, Ordering};
use smallvec::SmallVec;

use crate::{Style, Bounds, LayoutContext, Event};

/// Element ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementId(pub u64);

impl ElementId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for ElementId {
    fn default() -> Self {
        Self::new()
    }
}

/// UI Element
pub struct Element {
    /// Element ID
    pub id: ElementId,
    /// Element kind
    pub kind: ElementKind,
    /// Style
    pub style: Style,
    /// Children
    pub children: SmallVec<[Element; 4]>,
    /// Computed bounds (after layout)
    pub bounds: Bounds,
    /// Is interactive?
    pub interactive: bool,
    /// Event handlers
    handlers: Vec<Box<dyn Fn(&Event) -> bool + Send + Sync>>,
}

impl Element {
    /// Create a new element
    pub fn new(kind: ElementKind) -> Self {
        Self {
            id: ElementId::new(),
            kind,
            style: Style::default(),
            children: SmallVec::new(),
            bounds: Bounds::default(),
            interactive: false,
            handlers: Vec::new(),
        }
    }

    /// Create a div (container)
    pub fn div() -> Self {
        Self::new(ElementKind::Div)
    }

    /// Create a text element
    pub fn text(content: impl Into<String>) -> Self {
        Self::new(ElementKind::Text(content.into()))
    }

    /// Create an image element
    pub fn image(src: impl Into<String>) -> Self {
        Self::new(ElementKind::Image(src.into()))
    }

    /// Create an input element
    pub fn input() -> Self {
        let mut el = Self::new(ElementKind::Input { value: String::new(), placeholder: None });
        el.interactive = true;
        el
    }

    /// Create a button
    pub fn button(label: impl Into<String>) -> Self {
        let mut el = Self::new(ElementKind::Button(label.into()));
        el.interactive = true;
        el
    }

    /// Set style
    pub fn styled(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Add child
    pub fn child(mut self, child: Element) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children
    pub fn children(mut self, children: impl IntoIterator<Item = Element>) -> Self {
        self.children.extend(children);
        self
    }

    /// Add event handler
    pub fn on_event(mut self, handler: impl Fn(&Event) -> bool + Send + Sync + 'static) -> Self {
        self.handlers.push(Box::new(handler));
        self.interactive = true;
        self
    }

    /// Layout this element
    pub fn layout(&mut self, ctx: &mut LayoutContext) {
        // Apply layout based on style
        self.bounds = ctx.allocate(&self.style);

        // Layout children
        let mut child_ctx = ctx.child_context(&self.bounds, &self.style);
        for child in &mut self.children {
            child.layout(&mut child_ctx);
        }
    }

    /// Handle event
    pub fn handle_event(&self, event: &Event) -> bool {
        for handler in &self.handlers {
            if handler(event) {
                return true;
            }
        }
        false
    }

    /// Hit test
    pub fn hit_test(&self, x: f32, y: f32) -> Option<ElementId> {
        if !self.bounds.contains(x, y) {
            return None;
        }

        // Check children first (front to back)
        for child in self.children.iter().rev() {
            if let Some(id) = child.hit_test(x, y) {
                return Some(id);
            }
        }

        if self.interactive {
            Some(self.id)
        } else {
            None
        }
    }
}

/// Element kind
#[derive(Debug, Clone)]
pub enum ElementKind {
    /// Container div
    Div,
    /// Text content
    Text(String),
    /// Image
    Image(String),
    /// Button
    Button(String),
    /// Text input
    Input { value: String, placeholder: Option<String> },
    /// Custom element
    Custom(String),
}
