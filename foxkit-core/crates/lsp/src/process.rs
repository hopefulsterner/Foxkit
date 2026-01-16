//! LSP server process management

use std::process::Stdio;
use tokio::process::{Command, Child as TokioChild, ChildStdin, ChildStdout};
use anyhow::Result;

use crate::ServerConfig;

/// Manages a language server process
pub struct ServerProcess {
    child: TokioChild,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
}

impl ServerProcess {
    /// Spawn a new server process
    pub fn spawn(config: &ServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&config.command);
        
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn()?;
        
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();

        tracing::info!("Started language server: {} ({})", config.name, config.command);

        Ok(Self {
            child,
            stdin,
            stdout,
        })
    }

    /// Get stdin for writing
    pub fn stdin(&mut self) -> Option<ChildStdin> {
        self.stdin.take()
    }

    /// Get stdout for reading
    pub fn stdout(&mut self) -> Option<ChildStdout> {
        self.stdout.take()
    }

    /// Kill the process
    pub async fn kill(&mut self) -> Result<()> {
        self.child.kill().await?;
        Ok(())
    }

    /// Wait for process to exit
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        let status = self.child.wait().await?;
        Ok(status)
    }

    /// Check if process is still running
    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>> {
        Ok(self.child.try_wait()?)
    }
}

/// Find a language server binary
pub fn find_server(name: &str) -> Option<String> {
    // Check common paths
    let paths = [
        format!("/usr/bin/{}", name),
        format!("/usr/local/bin/{}", name),
        format!("{}/.local/bin/{}", std::env::var("HOME").unwrap_or_default(), name),
        format!("{}/.cargo/bin/{}", std::env::var("HOME").unwrap_or_default(), name),
    ];

    for path in &paths {
        if std::path::Path::new(path).exists() {
            return Some(path.clone());
        }
    }

    // Check PATH
    if let Ok(output) = std::process::Command::new("which").arg(name).output() {
        if output.status.success() {
            if let Ok(path) = String::from_utf8(output.stdout) {
                return Some(path.trim().to_string());
            }
        }
    }

    None
}

/// Check if a server is available
pub fn is_server_available(name: &str) -> bool {
    find_server(name).is_some()
}
