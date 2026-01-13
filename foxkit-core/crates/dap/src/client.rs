//! DAP client implementation

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot, Mutex};
use parking_lot::RwLock;
use anyhow::Result;

use crate::protocol::*;
use crate::{AdapterConfig, DebugEvent, DebugState, LaunchConfig, Breakpoint, StackFrame, Thread, Variable, Scope, Source};

/// DAP client
pub struct DapClient {
    /// Adapter configuration
    config: AdapterConfig,
    /// Current state
    state: RwLock<DebugState>,
    /// Request ID counter
    next_seq: AtomicI64,
    /// Pending requests
    pending: Mutex<HashMap<i64, oneshot::Sender<Response>>>,
    /// Event sender
    event_tx: mpsc::UnboundedSender<DebugEvent>,
    /// Server capabilities
    capabilities: RwLock<Option<Capabilities>>,
}

impl DapClient {
    pub fn new(config: AdapterConfig, event_tx: mpsc::UnboundedSender<DebugEvent>) -> Self {
        Self {
            config,
            state: RwLock::new(DebugState::Inactive),
            next_seq: AtomicI64::new(1),
            pending: Mutex::new(HashMap::new()),
            event_tx,
            capabilities: RwLock::new(None),
        }
    }

    /// Initialize the adapter
    pub async fn initialize(&self) -> Result<Capabilities> {
        let args = InitializeArguments {
            client_id: Some("foxkit".to_string()),
            client_name: Some("Foxkit".to_string()),
            adapter_id: self.config.adapter_type.clone(),
            locale: Some("en-US".to_string()),
            lines_start_at1: true,
            columns_start_at1: true,
            path_format: Some("path".to_string()),
            supports_variable_type: true,
            supports_variable_paging: false,
            supports_run_in_terminal_request: true,
            supports_memory_references: false,
            supports_progress_reporting: true,
            supports_invalidated_event: true,
        };

        // TODO: Send actual request over transport
        let capabilities = Capabilities::default();
        *self.capabilities.write() = Some(capabilities.clone());

        *self.state.write() = DebugState::Initializing;
        self.event_tx.send(DebugEvent::Initialized).ok();

        Ok(capabilities)
    }

    /// Launch a program
    pub async fn launch(&self, config: &LaunchConfig) -> Result<()> {
        *self.state.write() = DebugState::Running;

        // TODO: Send launch request
        self.event_tx.send(DebugEvent::ProcessStarted {
            name: config.program.clone().unwrap_or_default(),
            pid: None,
        }).ok();

        Ok(())
    }

    /// Attach to a running process
    pub async fn attach(&self, pid: i64) -> Result<()> {
        *self.state.write() = DebugState::Running;
        Ok(())
    }

    /// Disconnect
    pub async fn disconnect(&self, terminate: bool) -> Result<()> {
        *self.state.write() = DebugState::Terminated;
        self.event_tx.send(DebugEvent::Terminated { restart: false }).ok();
        Ok(())
    }

    /// Continue execution
    pub async fn continue_execution(&self, thread_id: i64) -> Result<()> {
        *self.state.write() = DebugState::Running;
        self.event_tx.send(DebugEvent::Continued { thread_id }).ok();
        Ok(())
    }

    /// Pause execution
    pub async fn pause(&self, thread_id: i64) -> Result<()> {
        *self.state.write() = DebugState::Stopped;
        Ok(())
    }

    /// Step over (next)
    pub async fn next(&self, thread_id: i64) -> Result<()> {
        *self.state.write() = DebugState::Running;
        Ok(())
    }

    /// Step into
    pub async fn step_in(&self, thread_id: i64) -> Result<()> {
        *self.state.write() = DebugState::Running;
        Ok(())
    }

    /// Step out
    pub async fn step_out(&self, thread_id: i64) -> Result<()> {
        *self.state.write() = DebugState::Running;
        Ok(())
    }

    /// Set breakpoints
    pub async fn set_breakpoints(&self, source: Source, breakpoints: Vec<SourceBreakpoint>) -> Result<Vec<Breakpoint>> {
        // TODO: Send actual request
        let result = breakpoints.into_iter().map(|bp| {
            Breakpoint {
                id: Some(1),
                verified: true,
                source: Some(source.clone()),
                line: Some(bp.line),
                column: bp.column,
                condition: bp.condition,
                hit_condition: bp.hit_condition,
                log_message: bp.log_message,
            }
        }).collect();

        Ok(result)
    }

    /// Get threads
    pub async fn threads(&self) -> Result<Vec<Thread>> {
        // TODO: Send actual request
        Ok(vec![Thread { id: 1, name: "main".to_string() }])
    }

    /// Get stack trace
    pub async fn stack_trace(&self, thread_id: i64, start_frame: Option<i64>, levels: Option<i64>) -> Result<Vec<StackFrame>> {
        // TODO: Send actual request
        Ok(vec![])
    }

    /// Get scopes for a frame
    pub async fn scopes(&self, frame_id: i64) -> Result<Vec<Scope>> {
        // TODO: Send actual request
        Ok(vec![])
    }

    /// Get variables
    pub async fn variables(&self, reference: i64, start: Option<i64>, count: Option<i64>) -> Result<Vec<Variable>> {
        // TODO: Send actual request
        Ok(vec![])
    }

    /// Evaluate expression
    pub async fn evaluate(&self, expression: &str, frame_id: Option<i64>, context: Option<&str>) -> Result<String> {
        // TODO: Send actual request
        Ok(String::new())
    }

    /// Get state
    pub fn state(&self) -> DebugState {
        *self.state.read()
    }

    /// Get capabilities
    pub fn capabilities(&self) -> Option<Capabilities> {
        self.capabilities.read().clone()
    }

    fn next_sequence(&self) -> i64 {
        self.next_seq.fetch_add(1, Ordering::SeqCst)
    }
}

/// Source breakpoint (request)
#[derive(Debug, Clone)]
pub struct SourceBreakpoint {
    pub line: i64,
    pub column: Option<i64>,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub log_message: Option<String>,
}
