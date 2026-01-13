//! Highlighting

use theme::{Color, TokenStyle};

/// A highlight event
#[derive(Debug, Clone)]
pub struct HighlightEvent {
    pub start: usize,
    pub end: usize,
    pub scope: String,
}

impl HighlightEvent {
    pub fn range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

/// Highlight configuration
#[derive(Debug, Clone)]
pub struct Highlight {
    pub scope: String,
    pub style: TokenStyle,
}

impl Highlight {
    pub fn new(scope: impl Into<String>, style: TokenStyle) -> Self {
        Self {
            scope: scope.into(),
            style,
        }
    }
}

/// Highlighted text span
#[derive(Debug, Clone)]
pub struct HighlightedSpan {
    pub text: String,
    pub style: Option<TokenStyle>,
}

/// Render highlighted text
pub fn render_highlights(
    text: &str,
    events: &[HighlightEvent],
    theme: &theme::SyntaxTheme,
) -> Vec<HighlightedSpan> {
    let mut spans = Vec::new();
    let mut pos = 0;

    for event in events {
        // Unhighlighted text before this event
        if event.start > pos {
            spans.push(HighlightedSpan {
                text: text[pos..event.start].to_string(),
                style: None,
            });
        }

        // Highlighted text
        let style = theme.style_for_scope(&event.scope);
        spans.push(HighlightedSpan {
            text: text[event.start..event.end].to_string(),
            style: Some(style),
        });

        pos = event.end;
    }

    // Remaining text
    if pos < text.len() {
        spans.push(HighlightedSpan {
            text: text[pos..].to_string(),
            style: None,
        });
    }

    spans
}
