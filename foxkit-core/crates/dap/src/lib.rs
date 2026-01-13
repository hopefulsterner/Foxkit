//! # Foxkit DAP
//!
//! Debug Adapter Protocol client for debugging support.

pub mod adapter;
pub mod client;
pub mod protocol;
pub mod session;

use std::sync::Arc;
use std::collections::HashMap;
use std::path::PathBuf;
use parking_lot::RwLock;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub use adapter::DebugAdapter;
pub use client::DapClient;
pub use session::DebugSession;

/// Debug adapter configuration
#[derive(Debug, Clone)]
pub struct AdapterConfig {
    /// Adapter type (e.g., "lldb", "node", "python")
    pub adapter_type: String,
    /// Display name
    pub name: String,
    /// Command to start adapter
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Languages this adapter supports
    pub languages: Vec<String>,
}

impl AdapterConfig {
    pub fn new(adapter_type: &str, name: &str, command: &str) -> Self {
        Self {
            adapter_type: adapter_type.to_string(),
            name: name.to_string(),
            command: command.to_string(),
            args: Vec::new(),
            languages: Vec::new(),
        }
    }
}

/// Built-in adapter configurations
pub mod adapters {
    use super::*;

    pub fn codelldb() -> AdapterConfig {
        AdapterConfig {
            adapter_type: "lldb".to_string(),
            name: "CodeLLDB".to_string(),
            command: "codelldb".to_string(),
            args: vec![],
            languages: vec!["rust".to_string(), "c".to_string(), "cpp".to_string()],
        }
    }

    pub fn node() -> AdapterConfig {
        AdapterConfig {
            adapter_type: "node".to_string(),
            name: "Node.js Debug".to_string(),
            command: "node".to_string(),
            args: vec!["--inspect".to_string()],
            languages: vec!["javascript".to_string(), "typescript".to_string()],
        }
    }

    pub fn python() -> AdapterConfig {
        AdapterConfig {
            adapter_type: "python".to_string(),
            name: "Python Debugger".to_string(),
            command: "python".to_string(),
            args: vec!["-m".to_string(), "debugpy.adapter".to_string()],
            languages: vec!["python".to_string()],
        }
    }
}

/// Launch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchConfig {
    /// Configuration name
    pub name: String,
    /// Request type (launch or attach)
    pub request: RequestType,
    /// Adapter type
    #[serde(rename = "type")]
    pub adapter_type: String,
    /// Program to debug
    #[serde(default)]
    pub program: Option<String>,
    /// Command line arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Current working directory
    #[serde(default)]
    pub cwd: Option<String>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Stop on entry
    #[serde(default)]
    pub stop_on_entry: bool,
    /// Additional adapter-specific options
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Request type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestType {
    Launch,
    Attach,
}

/// Breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    /// Breakpoint ID (assigned by adapter)
    pub id: Option<i64>,
    /// Is breakpoint verified?
    pub verified: bool,
    /// Source file
    pub source: Option<Source>,
    /// Line number
    pub line: Option<i64>,
    /// Column number
    pub column: Option<i64>,
    /// Condition expression
    pub condition: Option<String>,
    /// Hit count condition
    pub hit_condition: Option<String>,
    /// Log message (logpoint)
    pub log_message: Option<String>,
}

/// Source file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// Display name
    pub name: Option<String>,
    /// File path
    pub path: Option<String>,
    /// Source reference
    pub source_reference: Option<i64>,
}

/// Stack frame
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
    /// Frame ID
    pub id: i64,
    /// Frame name
    pub name: String,
    /// Source location
    pub source: Option<Source>,
    /// Line number
    pub line: i64,
    /// Column number
    pub column: i64,
    /// Module ID
    pub module_id: Option<serde_json::Value>,
}

/// Thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    /// Thread ID
    pub id: i64,
    /// Thread name
    pub name: String,
}

/// Variable
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    /// Variable name
    pub name: String,
    /// Variable value
    pub value: String,
    /// Type
    #[serde(rename = "type")]
    pub var_type: Option<String>,
    /// Variables reference (for nested variables)
    pub variables_reference: i64,
    /// Named variables count
    pub named_variables: Option<i64>,
    /// Indexed variables count
    pub indexed_variables: Option<i64>,
}

/// Scope
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    /// Scope name
    pub name: String,
    /// Variables reference
    pub variables_reference: i64,
    /// Is expensive to fetch?
    pub expensive: bool,
    /// Source location
    pub source: Option<Source>,
    /// Line number
    pub line: Option<i64>,
}

/// Debug event
#[derive(Debug, Clone)]
pub enum DebugEvent {
    /// Debugger initialized
    Initialized,
    /// Stopped (breakpoint, step, etc.)
    Stopped {
        reason: StopReason,
        thread_id: i64,
        all_threads_stopped: bool,
    },
    /// Continued
    Continued { thread_id: i64 },
    /// Thread started
    ThreadStarted { thread_id: i64 },
    /// Thread exited
    ThreadExited { thread_id: i64 },
    /// Output (console, stdout, stderr)
    Output {
        category: OutputCategory,
        output: String,
    },
    /// Breakpoint changed
    BreakpointChanged { breakpoint: Breakpoint },
    /// Module loaded
    ModuleLoaded { name: String, path: Option<String> },
    /// Process started
    ProcessStarted { name: String, pid: Option<i64> },
    /// Terminated
    Terminated { restart: bool },
    /// Exited
    Exited { exit_code: i64 },
}

/// Stop reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    Breakpoint,
    Step,
    Pause,
    Exception,
    Entry,
    Goto,
    DataBreakpoint,
    InstructionBreakpoint,
}

/// Output category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputCategory {
    Console,
    Stdout,
    Stderr,
    Telemetry,
}

/// Debug state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugState {
    /// Not debugging
    Inactive,
    /// Initializing
    Initializing,
    /// Running
    Running,
    /// Stopped at breakpoint/step
    Stopped,
    /// Terminated
    Terminated,
}
