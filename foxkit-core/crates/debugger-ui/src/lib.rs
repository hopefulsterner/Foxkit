//! # Foxkit Debugger UI
//!
//! Debug session UI components and views.

pub mod views;
pub mod breakpoints;
pub mod variables;
pub mod callstack;
pub mod watch;
pub mod console;
pub mod toolbar;

use std::sync::Arc;
use parking_lot::RwLock;

pub use views::{DebugView, DebugViewId};
pub use breakpoints::BreakpointsView;
pub use variables::VariablesView;
pub use callstack::CallStackView;
pub use watch::WatchView;
pub use console::DebugConsole;
pub use toolbar::DebugToolbar;

/// Debug UI service
pub struct DebugUiService {
    /// Active debug session
    session: RwLock<Option<DebugSession>>,
    /// Breakpoints view
    breakpoints: BreakpointsView,
    /// Variables view
    variables: VariablesView,
    /// Call stack view
    callstack: CallStackView,
    /// Watch view
    watch: WatchView,
    /// Debug console
    console: DebugConsole,
    /// Toolbar state
    toolbar: DebugToolbar,
    /// Event listeners
    listeners: RwLock<Vec<Box<dyn Fn(&DebugUiEvent) + Send + Sync>>>,
}

impl DebugUiService {
    pub fn new() -> Self {
        Self {
            session: RwLock::new(None),
            breakpoints: BreakpointsView::new(),
            variables: VariablesView::new(),
            callstack: CallStackView::new(),
            watch: WatchView::new(),
            console: DebugConsole::new(),
            toolbar: DebugToolbar::new(),
            listeners: RwLock::new(Vec::new()),
        }
    }

    /// Start a debug session
    pub fn start_session(&self, config: DebugConfiguration) -> anyhow::Result<()> {
        let session = DebugSession::new(config);
        *self.session.write() = Some(session);
        
        self.toolbar.set_state(ToolbarState::Running);
        self.emit(DebugUiEvent::SessionStarted);
        
        Ok(())
    }

    /// Stop the debug session
    pub fn stop_session(&self) {
        if self.session.write().take().is_some() {
            self.toolbar.set_state(ToolbarState::Stopped);
            self.variables.clear();
            self.callstack.clear();
            self.emit(DebugUiEvent::SessionStopped);
        }
    }

    /// Handle debugger stopped event
    pub fn on_stopped(&self, reason: StopReason, location: Option<SourceLocation>) {
        self.toolbar.set_state(ToolbarState::Paused);
        
        if let Some(loc) = &location {
            self.emit(DebugUiEvent::Navigate(loc.clone()));
        }
        
        self.emit(DebugUiEvent::Stopped(reason));
    }

    /// Handle debugger continued event
    pub fn on_continued(&self) {
        self.toolbar.set_state(ToolbarState::Running);
        self.emit(DebugUiEvent::Continued);
    }

    /// Get breakpoints view
    pub fn breakpoints(&self) -> &BreakpointsView {
        &self.breakpoints
    }

    /// Get variables view
    pub fn variables(&self) -> &VariablesView {
        &self.variables
    }

    /// Get call stack view
    pub fn callstack(&self) -> &CallStackView {
        &self.callstack
    }

    /// Get watch view
    pub fn watch(&self) -> &WatchView {
        &self.watch
    }

    /// Get console
    pub fn console(&self) -> &DebugConsole {
        &self.console
    }

    /// Get toolbar
    pub fn toolbar(&self) -> &DebugToolbar {
        &self.toolbar
    }

    /// Subscribe to events
    pub fn subscribe<F>(&self, callback: F)
    where
        F: Fn(&DebugUiEvent) + Send + Sync + 'static,
    {
        self.listeners.write().push(Box::new(callback));
    }

    fn emit(&self, event: DebugUiEvent) {
        for listener in self.listeners.read().iter() {
            listener(&event);
        }
    }
}

impl Default for DebugUiService {
    fn default() -> Self {
        Self::new()
    }
}

/// Debug session
pub struct DebugSession {
    /// Configuration
    pub config: DebugConfiguration,
    /// Current thread ID
    pub thread_id: Option<i64>,
    /// Current frame ID
    pub frame_id: Option<i64>,
    /// Session state
    pub state: SessionState,
}

impl DebugSession {
    pub fn new(config: DebugConfiguration) -> Self {
        Self {
            config,
            thread_id: None,
            frame_id: None,
            state: SessionState::Initializing,
        }
    }
}

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Initializing,
    Running,
    Paused,
    Stopped,
}

/// Debug configuration
#[derive(Debug, Clone)]
pub struct DebugConfiguration {
    /// Configuration name
    pub name: String,
    /// Debug type
    pub debug_type: String,
    /// Request type (launch/attach)
    pub request: RequestType,
    /// Program to debug
    pub program: Option<String>,
    /// Arguments
    pub args: Vec<String>,
    /// Environment
    pub env: std::collections::HashMap<String, String>,
    /// Working directory
    pub cwd: Option<String>,
    /// Additional properties
    pub extra: serde_json::Value,
}

/// Request type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestType {
    Launch,
    Attach,
}

/// Debug UI event
#[derive(Debug, Clone)]
pub enum DebugUiEvent {
    SessionStarted,
    SessionStopped,
    Stopped(StopReason),
    Continued,
    BreakpointHit(i64),
    Navigate(SourceLocation),
    VariablesUpdated,
    CallStackUpdated,
    OutputReceived(String),
}

/// Stop reason
#[derive(Debug, Clone)]
pub enum StopReason {
    Step,
    Breakpoint,
    Exception,
    Pause,
    Entry,
    Goto,
    DataBreakpoint,
    InstructionBreakpoint,
}

/// Source location
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub path: String,
    pub line: u32,
    pub column: Option<u32>,
}

/// Toolbar state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarState {
    Stopped,
    Running,
    Paused,
}
