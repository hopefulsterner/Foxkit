//! # Foxkit Sticky Scroll
//!
//! Sticky context headers that show containing scopes.

use std::path::PathBuf;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Sticky scroll service
pub struct StickyScrollService {
    /// Configuration
    config: RwLock<StickyScrollConfig>,
    /// Current sticky lines
    current: RwLock<Option<StickyContext>>,
}

impl StickyScrollService {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(StickyScrollConfig::default()),
            current: RwLock::new(None),
        }
    }

    /// Configure service
    pub fn configure(&self, config: StickyScrollConfig) {
        *self.config.write() = config;
    }

    /// Get configuration
    pub fn config(&self) -> StickyScrollConfig {
        self.config.read().clone()
    }

    /// Compute sticky lines for viewport
    pub fn compute_sticky_lines(
        &self,
        scopes: &[Scope],
        viewport_start: u32,
        viewport_end: u32,
    ) -> Vec<StickyLine> {
        let config = self.config.read();
        
        if !config.enabled {
            return Vec::new();
        }

        let mut sticky_lines = Vec::new();

        // Find scopes that contain the viewport start but start before it
        for scope in scopes {
            if scope.start_line < viewport_start && scope.end_line > viewport_start {
                sticky_lines.push(StickyLine {
                    line_number: scope.start_line,
                    text: scope.header_text.clone(),
                    scope_kind: scope.kind.clone(),
                    depth: scope.depth,
                });
            }
        }

        // Sort by depth (outermost first)
        sticky_lines.sort_by_key(|l| l.depth);

        // Limit to max lines
        sticky_lines.truncate(config.max_lines as usize);

        sticky_lines
    }

    /// Update sticky context for scroll position
    pub fn update_for_scroll(&self, scopes: &[Scope], viewport_start: u32, viewport_end: u32) {
        let lines = self.compute_sticky_lines(scopes, viewport_start, viewport_end);
        
        *self.current.write() = Some(StickyContext {
            lines,
            viewport_start,
        });
    }

    /// Get current sticky context
    pub fn current(&self) -> Option<StickyContext> {
        self.current.read().clone()
    }

    /// Clear sticky context
    pub fn clear(&self) {
        *self.current.write() = None;
    }

    /// Handle click on sticky line
    pub fn click_sticky_line(&self, line_index: usize) -> Option<u32> {
        self.current
            .read()
            .as_ref()
            .and_then(|ctx| ctx.lines.get(line_index))
            .map(|line| line.line_number)
    }
}

impl Default for StickyScrollService {
    fn default() -> Self {
        Self::new()
    }
}

/// Scope information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    /// Start line of scope
    pub start_line: u32,
    /// End line of scope
    pub end_line: u32,
    /// Header text to display
    pub header_text: String,
    /// Scope kind
    pub kind: ScopeKind,
    /// Nesting depth
    pub depth: u32,
}

impl Scope {
    pub fn new(start_line: u32, end_line: u32, header_text: impl Into<String>) -> Self {
        Self {
            start_line,
            end_line,
            header_text: header_text.into(),
            kind: ScopeKind::Block,
            depth: 0,
        }
    }

    pub fn with_kind(mut self, kind: ScopeKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    pub fn contains_line(&self, line: u32) -> bool {
        line >= self.start_line && line <= self.end_line
    }
}

/// Scope kind
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScopeKind {
    /// Function/method
    Function,
    /// Class/struct/enum
    Class,
    /// Interface/trait
    Interface,
    /// Namespace/module
    Namespace,
    /// If/else/switch
    Conditional,
    /// For/while/loop
    Loop,
    /// Try/catch/finally
    TryCatch,
    /// Generic block
    Block,
    /// Other
    Other(String),
}

impl ScopeKind {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Function => "$(symbol-function)",
            Self::Class => "$(symbol-class)",
            Self::Interface => "$(symbol-interface)",
            Self::Namespace => "$(symbol-namespace)",
            Self::Conditional => "$(symbol-key)",
            Self::Loop => "$(sync)",
            Self::TryCatch => "$(shield)",
            Self::Block => "$(bracket)",
            Self::Other(_) => "$(code)",
        }
    }
}

/// Sticky line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickyLine {
    /// Original line number
    pub line_number: u32,
    /// Text to display
    pub text: String,
    /// Scope kind
    pub scope_kind: ScopeKind,
    /// Nesting depth
    pub depth: u32,
}

impl StickyLine {
    /// Get truncated text
    pub fn truncated_text(&self, max_len: usize) -> String {
        if self.text.len() <= max_len {
            self.text.clone()
        } else {
            format!("{}...", &self.text[..max_len - 3])
        }
    }
}

/// Current sticky context
#[derive(Debug, Clone)]
pub struct StickyContext {
    /// Sticky lines
    pub lines: Vec<StickyLine>,
    /// Viewport start line
    pub viewport_start: u32,
}

impl StickyContext {
    pub fn height(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

/// Sticky scroll configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickyScrollConfig {
    /// Enable sticky scroll
    pub enabled: bool,
    /// Maximum sticky lines
    pub max_lines: u32,
    /// Default scope kind to show
    pub default_model: StickyScrollModel,
    /// Scroll with editor
    pub scroll_with_editor: bool,
}

impl Default for StickyScrollConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_lines: 5,
            default_model: StickyScrollModel::OutlineModel,
            scroll_with_editor: true,
        }
    }
}

/// Sticky scroll model
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StickyScrollModel {
    /// Use outline model
    OutlineModel,
    /// Use folding provider model
    FoldingProviderModel,
    /// Use indentation model
    IndentationModel,
}

/// Extract scopes from tree-sitter
pub fn extract_scopes_from_tree(
    content: &str,
    _language: &str,
) -> Vec<Scope> {
    // Would use tree-sitter to parse and extract scopes
    // For now, use simple heuristics
    
    let mut scopes = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut depth = 0;
    let mut scope_stack: Vec<(u32, String, u32)> = Vec::new(); // (start_line, header, depth)

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        
        // Detect scope starts (simplified)
        if trimmed.ends_with('{') {
            let header = trimmed.trim_end_matches('{').trim().to_string();
            if !header.is_empty() {
                scope_stack.push((i as u32, header, depth));
            }
            depth += 1;
        }

        // Detect scope ends
        if trimmed.starts_with('}') || trimmed == "}" {
            depth = depth.saturating_sub(1);
            
            if let Some((start_line, header, scope_depth)) = scope_stack.pop() {
                scopes.push(Scope {
                    start_line,
                    end_line: i as u32,
                    header_text: header,
                    kind: infer_scope_kind(&lines[start_line as usize]),
                    depth: scope_depth,
                });
            }
        }
    }

    scopes
}

fn infer_scope_kind(line: &str) -> ScopeKind {
    let trimmed = line.trim().to_lowercase();
    
    if trimmed.starts_with("fn ") || trimmed.starts_with("func ")
        || trimmed.starts_with("function ") || trimmed.starts_with("def ")
        || trimmed.contains("fn ") || trimmed.contains("func(")
    {
        ScopeKind::Function
    } else if trimmed.starts_with("class ") || trimmed.starts_with("struct ")
        || trimmed.starts_with("enum ")
    {
        ScopeKind::Class
    } else if trimmed.starts_with("interface ") || trimmed.starts_with("trait ") {
        ScopeKind::Interface
    } else if trimmed.starts_with("mod ") || trimmed.starts_with("namespace ")
        || trimmed.starts_with("module ")
    {
        ScopeKind::Namespace
    } else if trimmed.starts_with("if ") || trimmed.starts_with("else ")
        || trimmed.starts_with("match ") || trimmed.starts_with("switch ")
    {
        ScopeKind::Conditional
    } else if trimmed.starts_with("for ") || trimmed.starts_with("while ")
        || trimmed.starts_with("loop ")
    {
        ScopeKind::Loop
    } else if trimmed.starts_with("try ") || trimmed.starts_with("catch ")
        || trimmed.starts_with("finally ")
    {
        ScopeKind::TryCatch
    } else {
        ScopeKind::Block
    }
}
