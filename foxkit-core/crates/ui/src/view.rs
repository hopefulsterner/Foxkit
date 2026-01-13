//! View trait

use crate::{Element, Context};

/// View trait - represents a renderable view
pub trait View: Send + Sync {
    /// Render the view
    fn render(&self, ctx: &mut Context) -> Element;

    /// View name (for debugging)
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// Implement View for closures
impl<F> View for F
where
    F: Fn(&mut Context) -> Element + Send + Sync,
{
    fn render(&self, ctx: &mut Context) -> Element {
        self(ctx)
    }
}
