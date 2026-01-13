//! # Foxkit Go to Definition
//!
//! Navigate to symbol definitions, declarations, and type definitions.

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Go to definition service
pub struct GotoDefinitionService {
    /// Navigation history
    history: RwLock<NavigationHistory>,
    /// Events
    events: broadcast::Sender<GotoEvent>,
    /// Configuration
    config: RwLock<GotoConfig>,
}

impl GotoDefinitionService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            history: RwLock::new(NavigationHistory::new()),
            events,
            config: RwLock::new(GotoConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<GotoEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: GotoConfig) {
        *self.config.write() = config;
    }

    /// Go to definition
    pub async fn goto_definition(
        &self,
        file: &PathBuf,
        position: GotoPosition,
    ) -> anyhow::Result<Vec<Location>> {
        // Would call LSP textDocument/definition
        let _ = self.events.send(GotoEvent::Navigating {
            kind: NavigationKind::Definition,
            from: Location::new(file.clone(), position),
        });

        // Placeholder - actual implementation would call LSP
        Ok(Vec::new())
    }

    /// Go to declaration
    pub async fn goto_declaration(
        &self,
        file: &PathBuf,
        position: GotoPosition,
    ) -> anyhow::Result<Vec<Location>> {
        // Would call LSP textDocument/declaration
        let _ = self.events.send(GotoEvent::Navigating {
            kind: NavigationKind::Declaration,
            from: Location::new(file.clone(), position),
        });

        Ok(Vec::new())
    }

    /// Go to type definition
    pub async fn goto_type_definition(
        &self,
        file: &PathBuf,
        position: GotoPosition,
    ) -> anyhow::Result<Vec<Location>> {
        // Would call LSP textDocument/typeDefinition
        let _ = self.events.send(GotoEvent::Navigating {
            kind: NavigationKind::TypeDefinition,
            from: Location::new(file.clone(), position),
        });

        Ok(Vec::new())
    }

    /// Go to implementation
    pub async fn goto_implementation(
        &self,
        file: &PathBuf,
        position: GotoPosition,
    ) -> anyhow::Result<Vec<Location>> {
        // Would call LSP textDocument/implementation
        let _ = self.events.send(GotoEvent::Navigating {
            kind: NavigationKind::Implementation,
            from: Location::new(file.clone(), position),
        });

        Ok(Vec::new())
    }

    /// Navigate to location and record in history
    pub fn navigate_to(&self, from: Location, to: Location) {
        self.history.write().push(from.clone());
        
        let _ = self.events.send(GotoEvent::Navigated {
            from,
            to: to.clone(),
        });
    }

    /// Go back in navigation history
    pub fn go_back(&self) -> Option<Location> {
        let location = self.history.write().go_back()?;
        let _ = self.events.send(GotoEvent::WentBack { location: location.clone() });
        Some(location)
    }

    /// Go forward in navigation history
    pub fn go_forward(&self) -> Option<Location> {
        let location = self.history.write().go_forward()?;
        let _ = self.events.send(GotoEvent::WentForward { location: location.clone() });
        Some(location)
    }

    /// Check if can go back
    pub fn can_go_back(&self) -> bool {
        self.history.read().can_go_back()
    }

    /// Check if can go forward
    pub fn can_go_forward(&self) -> bool {
        self.history.read().can_go_forward()
    }

    /// Peek definition (show inline without navigating)
    pub async fn peek_definition(
        &self,
        file: &PathBuf,
        position: GotoPosition,
    ) -> anyhow::Result<Vec<PeekResult>> {
        let locations = self.goto_definition(file, position).await?;

        let results: Vec<PeekResult> = locations
            .into_iter()
            .map(|loc| PeekResult {
                location: loc,
                preview: None, // Would load preview content
            })
            .collect();

        Ok(results)
    }
}

impl Default for GotoDefinitionService {
    fn default() -> Self {
        Self::new()
    }
}

/// Navigation history
struct NavigationHistory {
    /// Past locations
    back: Vec<Location>,
    /// Forward locations
    forward: Vec<Location>,
    /// Maximum history size
    max_size: usize,
}

impl NavigationHistory {
    fn new() -> Self {
        Self {
            back: Vec::new(),
            forward: Vec::new(),
            max_size: 100,
        }
    }

    fn push(&mut self, location: Location) {
        // Clear forward history when new navigation occurs
        self.forward.clear();
        
        self.back.push(location);
        
        // Trim if too large
        if self.back.len() > self.max_size {
            self.back.remove(0);
        }
    }

    fn go_back(&mut self) -> Option<Location> {
        let location = self.back.pop()?;
        self.forward.push(location.clone());
        self.back.last().cloned()
    }

    fn go_forward(&mut self) -> Option<Location> {
        let location = self.forward.pop()?;
        self.back.push(location.clone());
        Some(location)
    }

    fn can_go_back(&self) -> bool {
        self.back.len() > 1
    }

    fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }
}

/// Location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// File path
    pub file: PathBuf,
    /// Position
    pub position: GotoPosition,
    /// Optional range
    pub range: Option<LocationRange>,
}

impl Location {
    pub fn new(file: PathBuf, position: GotoPosition) -> Self {
        Self { file, position, range: None }
    }

    pub fn with_range(mut self, range: LocationRange) -> Self {
        self.range = Some(range);
        self
    }
}

/// Position
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GotoPosition {
    pub line: u32,
    pub col: u32,
}

impl GotoPosition {
    pub fn new(line: u32, col: u32) -> Self {
        Self { line, col }
    }
}

/// Location range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl LocationRange {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
    }
}

/// Navigation kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationKind {
    Definition,
    Declaration,
    TypeDefinition,
    Implementation,
}

impl NavigationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Definition => "definition",
            Self::Declaration => "declaration",
            Self::TypeDefinition => "type definition",
            Self::Implementation => "implementation",
        }
    }

    pub fn command(&self) -> &'static str {
        match self {
            Self::Definition => "editor.action.revealDefinition",
            Self::Declaration => "editor.action.revealDeclaration",
            Self::TypeDefinition => "editor.action.goToTypeDefinition",
            Self::Implementation => "editor.action.goToImplementation",
        }
    }
}

/// Peek result
#[derive(Debug, Clone)]
pub struct PeekResult {
    /// Location
    pub location: Location,
    /// Preview content
    pub preview: Option<String>,
}

/// Goto configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GotoConfig {
    /// Enable peek on hover
    pub peek_on_hover: bool,
    /// Ctrl+click behavior
    pub ctrl_click: CtrlClickBehavior,
    /// Open in new tab
    pub open_in_new_tab: bool,
    /// Show multiple results in picker
    pub show_picker_for_multiple: bool,
}

impl Default for GotoConfig {
    fn default() -> Self {
        Self {
            peek_on_hover: false,
            ctrl_click: CtrlClickBehavior::GoToDefinition,
            open_in_new_tab: false,
            show_picker_for_multiple: true,
        }
    }
}

/// Ctrl+click behavior
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CtrlClickBehavior {
    GoToDefinition,
    GoToDeclaration,
    GoToTypeDefinition,
    GoToImplementation,
}

/// Goto event
#[derive(Debug, Clone)]
pub enum GotoEvent {
    Navigating { kind: NavigationKind, from: Location },
    Navigated { from: Location, to: Location },
    WentBack { location: Location },
    WentForward { location: Location },
    PeekOpened { location: Location },
    PeekClosed,
}

/// Link definition (for document links)
#[derive(Debug, Clone)]
pub struct LinkDefinition {
    pub range: LocationRange,
    pub target: Location,
    pub tooltip: Option<String>,
}

impl LinkDefinition {
    pub fn new(range: LocationRange, target: Location) -> Self {
        Self { range, target, tooltip: None }
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}
