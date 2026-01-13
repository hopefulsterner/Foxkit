//! Call stack view

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{DebugView, DebugViewId, SourceLocation};

/// Call stack view
pub struct CallStackView {
    /// Threads
    threads: RwLock<Vec<Thread>>,
    /// Selected thread
    selected_thread: RwLock<Option<i64>>,
    /// Selected frame
    selected_frame: RwLock<Option<i64>>,
    /// Visibility
    visible: bool,
}

impl CallStackView {
    pub fn new() -> Self {
        Self {
            threads: RwLock::new(Vec::new()),
            selected_thread: RwLock::new(None),
            selected_frame: RwLock::new(None),
            visible: true,
        }
    }

    /// Set threads
    pub fn set_threads(&self, threads: Vec<Thread>) {
        *self.threads.write() = threads;
    }

    /// Get threads
    pub fn threads(&self) -> Vec<Thread> {
        self.threads.read().clone()
    }

    /// Select thread
    pub fn select_thread(&self, thread_id: i64) {
        *self.selected_thread.write() = Some(thread_id);
    }

    /// Select frame
    pub fn select_frame(&self, frame_id: i64) {
        *self.selected_frame.write() = Some(frame_id);
    }

    /// Get selected thread ID
    pub fn selected_thread(&self) -> Option<i64> {
        *self.selected_thread.read()
    }

    /// Get selected frame ID
    pub fn selected_frame(&self) -> Option<i64> {
        *self.selected_frame.read()
    }

    /// Get stack frames for thread
    pub fn frames_for_thread(&self, thread_id: i64) -> Vec<StackFrame> {
        self.threads.read()
            .iter()
            .find(|t| t.id == thread_id)
            .map(|t| t.stack_frames.clone())
            .unwrap_or_default()
    }

    /// Clear call stack
    pub fn clear(&self) {
        self.threads.write().clear();
        *self.selected_thread.write() = None;
        *self.selected_frame.write() = None;
    }
}

impl Default for CallStackView {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugView for CallStackView {
    fn id(&self) -> DebugViewId {
        DebugViewId::CallStack
    }

    fn title(&self) -> &str {
        "Call Stack"
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn refresh(&mut self) {
        // Refresh from DAP
    }

    fn clear(&mut self) {
        CallStackView::clear(self);
    }
}

/// Debug thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    /// Thread ID
    pub id: i64,
    /// Thread name
    pub name: String,
    /// Stack frames
    pub stack_frames: Vec<StackFrame>,
}

/// Stack frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    /// Frame ID
    pub id: i64,
    /// Frame name (function name)
    pub name: String,
    /// Source location
    pub source: Option<Source>,
    /// Line number
    pub line: u32,
    /// Column
    pub column: u32,
    /// End line
    pub end_line: Option<u32>,
    /// End column  
    pub end_column: Option<u32>,
    /// Can restart frame?
    pub can_restart: Option<bool>,
    /// Instruction pointer reference
    pub instruction_pointer_reference: Option<String>,
    /// Module ID
    pub module_id: Option<String>,
    /// Presentation hint
    pub presentation_hint: Option<FramePresentationHint>,
}

impl StackFrame {
    /// Get display name
    pub fn display_name(&self) -> &str {
        &self.name
    }

    /// Get location
    pub fn location(&self) -> Option<SourceLocation> {
        self.source.as_ref().map(|s| SourceLocation {
            path: s.path.clone().unwrap_or_default(),
            line: self.line,
            column: Some(self.column),
        })
    }
}

/// Source info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// Source name
    pub name: Option<String>,
    /// Source path
    pub path: Option<String>,
    /// Source reference
    pub source_reference: Option<i64>,
    /// Presentation hint
    pub presentation_hint: Option<SourcePresentationHint>,
    /// Origin
    pub origin: Option<String>,
}

/// Source presentation hint
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourcePresentationHint {
    Normal,
    Emphasize,
    Deemphasize,
}

/// Frame presentation hint
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FramePresentationHint {
    Normal,
    Label,
    Subtle,
}
