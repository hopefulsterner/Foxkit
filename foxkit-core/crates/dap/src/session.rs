//! Debug session management

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::mpsc;
use parking_lot::RwLock;
use anyhow::Result;

use crate::{AdapterConfig, DapClient, DebugEvent, DebugState, LaunchConfig, Breakpoint, Source};
use crate::client::SourceBreakpoint;

/// Debug session
pub struct DebugSession {
    /// Session ID
    pub id: u64,
    /// DAP client
    client: Arc<DapClient>,
    /// Breakpoints by file
    breakpoints: RwLock<HashMap<String, Vec<Breakpoint>>>,
    /// Event receiver
    event_rx: mpsc::UnboundedReceiver<DebugEvent>,
}

impl DebugSession {
    /// Create new session
    pub fn new(id: u64, config: AdapterConfig) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let client = Arc::new(DapClient::new(config, event_tx));

        Self {
            id,
            client,
            breakpoints: RwLock::new(HashMap::new()),
            event_rx,
        }
    }

    /// Start the session
    pub async fn start(&self, launch_config: LaunchConfig) -> Result<()> {
        self.client.initialize().await?;
        self.client.launch(&launch_config).await?;
        Ok(())
    }

    /// Stop the session
    pub async fn stop(&self) -> Result<()> {
        self.client.disconnect(true).await
    }

    /// Add breakpoint
    pub async fn add_breakpoint(&self, file: &str, line: i64, condition: Option<String>) -> Result<Breakpoint> {
        let source = Source {
            name: Some(file.to_string()),
            path: Some(file.to_string()),
            source_reference: None,
        };

        let bp = SourceBreakpoint {
            line,
            column: None,
            condition,
            hit_condition: None,
            log_message: None,
        };

        let mut file_bps = self.breakpoints.write()
            .entry(file.to_string())
            .or_default()
            .clone();

        // Get all breakpoints for this file and add new one
        let source_bps: Vec<_> = file_bps.iter()
            .filter_map(|b| b.line.map(|l| SourceBreakpoint {
                line: l,
                column: b.column,
                condition: b.condition.clone(),
                hit_condition: b.hit_condition.clone(),
                log_message: b.log_message.clone(),
            }))
            .chain(std::iter::once(bp))
            .collect();

        let result = self.client.set_breakpoints(source, source_bps).await?;

        // Update stored breakpoints
        self.breakpoints.write().insert(file.to_string(), result.clone());

        result.last().cloned()
            .ok_or_else(|| anyhow::anyhow!("No breakpoint returned"))
    }

    /// Remove breakpoint
    pub async fn remove_breakpoint(&self, file: &str, line: i64) -> Result<()> {
        let source = Source {
            name: Some(file.to_string()),
            path: Some(file.to_string()),
            source_reference: None,
        };

        let file_bps = self.breakpoints.read()
            .get(file)
            .cloned()
            .unwrap_or_default();

        let source_bps: Vec<_> = file_bps.iter()
            .filter(|b| b.line != Some(line))
            .filter_map(|b| b.line.map(|l| SourceBreakpoint {
                line: l,
                column: b.column,
                condition: b.condition.clone(),
                hit_condition: b.hit_condition.clone(),
                log_message: b.log_message.clone(),
            }))
            .collect();

        let result = self.client.set_breakpoints(source, source_bps).await?;
        self.breakpoints.write().insert(file.to_string(), result);

        Ok(())
    }

    /// Continue execution
    pub async fn continue_execution(&self) -> Result<()> {
        let threads = self.client.threads().await?;
        if let Some(thread) = threads.first() {
            self.client.continue_execution(thread.id).await?;
        }
        Ok(())
    }

    /// Step over
    pub async fn step_over(&self) -> Result<()> {
        let threads = self.client.threads().await?;
        if let Some(thread) = threads.first() {
            self.client.next(thread.id).await?;
        }
        Ok(())
    }

    /// Step into
    pub async fn step_into(&self) -> Result<()> {
        let threads = self.client.threads().await?;
        if let Some(thread) = threads.first() {
            self.client.step_in(thread.id).await?;
        }
        Ok(())
    }

    /// Step out
    pub async fn step_out(&self) -> Result<()> {
        let threads = self.client.threads().await?;
        if let Some(thread) = threads.first() {
            self.client.step_out(thread.id).await?;
        }
        Ok(())
    }

    /// Pause
    pub async fn pause(&self) -> Result<()> {
        let threads = self.client.threads().await?;
        if let Some(thread) = threads.first() {
            self.client.pause(thread.id).await?;
        }
        Ok(())
    }

    /// Evaluate expression
    pub async fn evaluate(&self, expression: &str) -> Result<String> {
        self.client.evaluate(expression, None, Some("repl")).await
    }

    /// Get state
    pub fn state(&self) -> DebugState {
        self.client.state()
    }

    /// Poll for events
    pub async fn next_event(&mut self) -> Option<DebugEvent> {
        self.event_rx.recv().await
    }
}

/// Debug session manager
pub struct DebugManager {
    sessions: RwLock<HashMap<u64, Arc<DebugSession>>>,
    next_id: std::sync::atomic::AtomicU64,
    adapter_configs: RwLock<HashMap<String, AdapterConfig>>,
}

impl DebugManager {
    pub fn new() -> Self {
        let manager = Self {
            sessions: RwLock::new(HashMap::new()),
            next_id: std::sync::atomic::AtomicU64::new(1),
            adapter_configs: RwLock::new(HashMap::new()),
        };

        // Register built-in adapters
        manager.register_adapter(crate::adapters::codelldb());
        manager.register_adapter(crate::adapters::node());
        manager.register_adapter(crate::adapters::python());

        manager
    }

    /// Register an adapter
    pub fn register_adapter(&self, config: AdapterConfig) {
        self.adapter_configs.write().insert(config.adapter_type.clone(), config);
    }

    /// Start a debug session
    pub async fn start_session(&self, launch_config: LaunchConfig) -> Result<Arc<DebugSession>> {
        let adapter_config = self.adapter_configs.read()
            .get(&launch_config.adapter_type)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Unknown adapter: {}", launch_config.adapter_type))?;

        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let session = Arc::new(DebugSession::new(id, adapter_config));
        
        session.start(launch_config).await?;
        
        self.sessions.write().insert(id, Arc::clone(&session));

        Ok(session)
    }

    /// Stop a session
    pub async fn stop_session(&self, id: u64) -> Result<()> {
        if let Some(session) = self.sessions.write().remove(&id) {
            session.stop().await?;
        }
        Ok(())
    }

    /// Get a session
    pub fn session(&self, id: u64) -> Option<Arc<DebugSession>> {
        self.sessions.read().get(&id).cloned()
    }

    /// Get all active sessions
    pub fn sessions(&self) -> Vec<Arc<DebugSession>> {
        self.sessions.read().values().cloned().collect()
    }
}

impl Default for DebugManager {
    fn default() -> Self {
        Self::new()
    }
}
