//! Component system

use std::sync::Arc;
use crate::{Element, ElementId};
use theme::Theme;

/// Component context
pub struct Context {
    /// Current theme
    pub theme: Arc<Theme>,
    /// Focused element
    pub focus: Option<ElementId>,
    /// Hovered element
    pub hover: Option<ElementId>,
}

impl Context {
    /// Is element focused?
    pub fn is_focused(&self, id: ElementId) -> bool {
        self.focus == Some(id)
    }

    /// Is element hovered?
    pub fn is_hovered(&self, id: ElementId) -> bool {
        self.hover == Some(id)
    }
}

/// Render result
pub type RenderResult = Element;

/// Component trait
pub trait Component: Send + Sync {
    /// Component state type
    type State: Default + Send + Sync;

    /// Create initial state
    fn init(&self) -> Self::State {
        Self::State::default()
    }

    /// Render the component
    fn render(&self, state: &Self::State, ctx: &mut Context) -> RenderResult;

    /// Handle update (returns true if should re-render)
    fn update(&self, _state: &mut Self::State, _message: Message) -> bool {
        false
    }
}

/// Component message
#[derive(Debug, Clone)]
pub enum Message {
    Click,
    DoubleClick,
    Hover,
    Focus,
    Blur,
    KeyDown(String),
    KeyUp(String),
    TextInput(String),
    Custom(String),
}

/// Stateless component helper
pub struct Stateless<F>
where
    F: Fn(&mut Context) -> Element + Send + Sync,
{
    render_fn: F,
}

impl<F> Stateless<F>
where
    F: Fn(&mut Context) -> Element + Send + Sync,
{
    pub fn new(f: F) -> Self {
        Self { render_fn: f }
    }
}

impl<F> Component for Stateless<F>
where
    F: Fn(&mut Context) -> Element + Send + Sync,
{
    type State = ();

    fn render(&self, _state: &(), ctx: &mut Context) -> RenderResult {
        (self.render_fn)(ctx)
    }
}

/// Create a stateless component from a function
#[macro_export]
macro_rules! component {
    ($f:expr) => {
        $crate::component::Stateless::new($f)
    };
}
