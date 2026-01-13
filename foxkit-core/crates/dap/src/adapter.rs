//! Debug adapter management

use std::process::Stdio;
use tokio::process::{Command, Child};
use anyhow::Result;

use crate::AdapterConfig;

/// Debug adapter process
pub struct DebugAdapter {
    config: AdapterConfig,
    process: Option<Child>,
}

impl DebugAdapter {
    pub fn new(config: AdapterConfig) -> Self {
        Self {
            config,
            process: None,
        }
    }

    /// Start the adapter
    pub async fn start(&mut self) -> Result<()> {
        let child = Command::new(&self.config.command)
            .args(&self.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        tracing::info!("Started debug adapter: {}", self.config.name);
        self.process = Some(child);

        Ok(())
    }

    /// Stop the adapter
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            child.kill().await?;
        }
        Ok(())
    }

    /// Is adapter running?
    pub fn is_running(&self) -> bool {
        self.process.is_some()
    }
}
