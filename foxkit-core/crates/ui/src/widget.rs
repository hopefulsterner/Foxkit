//! Built-in widgets

use crate::element::ElementKind;
use crate::{Element, Style, StyleBuilder, Context};
use crate::style::Color;
use crate::layout::{Direction, Edge};

/// Create a flex row
pub fn row() -> Element {
    Element::div().styled(StyleBuilder::new().row().build())
}

/// Create a flex column
pub fn column() -> Element {
    Element::div().styled(StyleBuilder::new().column().build())
}

/// Create a text label
pub fn label(text: impl Into<String>) -> Element {
    Element::text(text)
}

/// Create a button
pub fn button(label: impl Into<String>) -> Element {
    Element::button(label)
        .styled(StyleBuilder::new()
            .padding_all(8.0)
            .border_radius(4.0)
            .cursor(crate::style::Cursor::Pointer)
            .build())
}

/// Create a text input
pub fn text_input(placeholder: impl Into<String>) -> Element {
    let mut el = Element::input();
    if let ElementKind::Input { placeholder: ref mut p, .. } = el.kind {
        *p = Some(placeholder.into());
    }
    el.styled(StyleBuilder::new()
        .padding_all(8.0)
        .border(Color::hex(0xCCCCCC), 1.0)
        .border_radius(4.0)
        .build())
}

/// Create a spacer
pub fn spacer() -> Element {
    Element::div().styled(StyleBuilder::new().flex(1.0).build())
}

/// Create a divider
pub fn divider() -> Element {
    Element::div().styled(StyleBuilder::new()
        .height(1.0)
        .background(Color::hex(0xE0E0E0))
        .margin(Edge::xy(0.0, 8.0))
        .build())
}

/// Create a scrollable container
pub fn scroll_view() -> Element {
    Element::new(ElementKind::Custom("scroll".to_string()))
}

/// Create a list item
pub fn list_item() -> Element {
    Element::div().styled(StyleBuilder::new()
        .padding(Edge::xy(16.0, 8.0))
        .row()
        .build())
}

/// Icon widget
pub fn icon(name: impl Into<String>) -> Element {
    Element::new(ElementKind::Custom(format!("icon:{}", name.into())))
        .styled(StyleBuilder::new().size(16.0, 16.0).build())
}

/// Badge widget
pub fn badge(text: impl Into<String>) -> Element {
    Element::text(text)
        .styled(StyleBuilder::new()
            .padding(Edge::xy(6.0, 2.0))
            .background(Color::hex(0x007ACC))
            .foreground(Color::WHITE)
            .border_radius(10.0)
            .font_size(12.0)
            .build())
}

/// Tooltip wrapper
pub fn tooltip(content: Element, tip: impl Into<String>) -> Element {
    Element::new(ElementKind::Custom(format!("tooltip:{}", tip.into())))
        .child(content)
}

/// Progress bar
pub fn progress_bar(value: f32) -> Element {
    let width = (value.clamp(0.0, 1.0) * 100.0).round();
    
    Element::div()
        .styled(StyleBuilder::new()
            .height(4.0)
            .background(Color::hex(0xE0E0E0))
            .border_radius(2.0)
            .build())
        .child(
            Element::div()
                .styled(StyleBuilder::new()
                    .width(width)
                    .height(4.0)
                    .background(Color::hex(0x007ACC))
                    .border_radius(2.0)
                    .build())
        )
}

/// Checkbox
pub fn checkbox(checked: bool) -> Element {
    Element::new(ElementKind::Custom(format!("checkbox:{}", checked)))
        .styled(StyleBuilder::new()
            .size(16.0, 16.0)
            .border(Color::hex(0xCCCCCC), 1.0)
            .border_radius(2.0)
            .cursor(crate::style::Cursor::Pointer)
            .build())
}

/// Toggle switch
pub fn toggle(on: bool) -> Element {
    Element::new(ElementKind::Custom(format!("toggle:{}", on)))
        .styled(StyleBuilder::new()
            .size(40.0, 20.0)
            .border_radius(10.0)
            .cursor(crate::style::Cursor::Pointer)
            .build())
}
