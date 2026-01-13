//! Breakpoints view

use std::path::PathBuf;
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{DebugView, DebugViewId};

/// Breakpoints view
pub struct BreakpointsView {
    /// All breakpoints by file
    breakpoints: RwLock<HashMap<PathBuf, Vec<Breakpoint>>>,
    /// Visibility
    visible: bool,
    /// Selected breakpoint
    selected: RwLock<Option<BreakpointId>>,
}

impl BreakpointsView {
    pub fn new() -> Self {
        Self {
            breakpoints: RwLock::new(HashMap::new()),
            visible: true,
            selected: RwLock::new(None),
        }
    }

    /// Add breakpoint
    pub fn add(&self, bp: Breakpoint) {
        let mut bps = self.breakpoints.write();
        bps.entry(bp.location.path.clone())
            .or_default()
            .push(bp);
    }

    /// Remove breakpoint
    pub fn remove(&self, id: BreakpointId) {
        let mut bps = self.breakpoints.write();
        for list in bps.values_mut() {
            list.retain(|bp| bp.id != id);
        }
    }

    /// Toggle breakpoint enabled state
    pub fn toggle(&self, id: BreakpointId) {
        let mut bps = self.breakpoints.write();
        for list in bps.values_mut() {
            for bp in list.iter_mut() {
                if bp.id == id {
                    bp.enabled = !bp.enabled;
                    return;
                }
            }
        }
    }

    /// Get all breakpoints
    pub fn all(&self) -> Vec<Breakpoint> {
        self.breakpoints.read()
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect()
    }

    /// Get breakpoints for file
    pub fn for_file(&self, path: &PathBuf) -> Vec<Breakpoint> {
        self.breakpoints.read()
            .get(path)
            .cloned()
            .unwrap_or_default()
    }

    /// Select breakpoint
    pub fn select(&self, id: BreakpointId) {
        *self.selected.write() = Some(id);
    }

    /// Get breakpoint by ID
    pub fn get(&self, id: BreakpointId) -> Option<Breakpoint> {
        self.breakpoints.read()
            .values()
            .flat_map(|v| v.iter())
            .find(|bp| bp.id == id)
            .cloned()
    }

    /// Clear all breakpoints
    pub fn clear_all(&self) {
        self.breakpoints.write().clear();
    }
}

impl Default for BreakpointsView {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugView for BreakpointsView {
    fn id(&self) -> DebugViewId {
        DebugViewId::Breakpoints
    }

    fn title(&self) -> &str {
        "Breakpoints"
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
        self.clear_all();
    }
}

/// Breakpoint ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BreakpointId(pub u64);

/// Breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    /// Unique ID
    pub id: BreakpointId,
    /// Location
    pub location: BreakpointLocation,
    /// Is enabled
    pub enabled: bool,
    /// Condition expression
    pub condition: Option<String>,
    /// Hit count condition
    pub hit_condition: Option<String>,
    /// Log message (logpoint)
    pub log_message: Option<String>,
    /// Is verified by debugger
    pub verified: bool,
}

impl Breakpoint {
    pub fn new(path: PathBuf, line: u32) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        
        Self {
            id: BreakpointId(COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)),
            location: BreakpointLocation { path, line, column: None },
            enabled: true,
            condition: None,
            hit_condition: None,
            log_message: None,
            verified: false,
        }
    }

    /// Make it a conditional breakpoint
    pub fn with_condition(mut self, condition: String) -> Self {
        self.condition = Some(condition);
        self
    }

    /// Make it a logpoint
    pub fn as_logpoint(mut self, message: String) -> Self {
        self.log_message = Some(message);
        self
    }
}

/// Breakpoint location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakpointLocation {
    pub path: PathBuf,
    pub line: u32,
    pub column: Option<u32>,
}

/// Function breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionBreakpoint {
    pub id: BreakpointId,
    pub name: String,
    pub enabled: bool,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
}

/// Exception breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionBreakpoint {
    pub filter: String,
    pub label: String,
    pub enabled: bool,
    pub condition: Option<String>,
}

/// Data breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataBreakpoint {
    pub id: BreakpointId,
    pub data_id: String,
    pub access_type: DataAccessType,
    pub enabled: bool,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
}

/// Data access type for data breakpoints
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DataAccessType {
    Read,
    Write,
    ReadWrite,
}
