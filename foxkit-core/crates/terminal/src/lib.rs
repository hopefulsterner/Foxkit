//! # Foxkit Terminal
//! 
//! Runtime-aware terminal emulator with:
//! - Per-package terminals
//! - Environment isolation
//! - Live process visualization
//! - Debug + terminal fusion
//! - Task orchestration integration

pub mod pty;
pub mod screen;
pub mod shell;
pub mod task;

use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use anyhow::Result;

pub use pty::Pty;
pub use screen::{Screen, Cell, CellStyle};
pub use shell::Shell;
pub use task::{Task, TaskStatus};

/// Terminal instance
pub struct Terminal {
    /// Unique terminal ID
    id: TerminalId,
    /// Terminal title
    title: String,
    /// Working directory
    cwd: PathBuf,
    /// Environment variables
    env: HashMap<String, String>,
    /// PTY handle
    pty: Option<Pty>,
    /// Screen buffer
    screen: Arc<RwLock<Screen>>,
    /// Input sender
    input_tx: Option<mpsc::UnboundedSender<Vec<u8>>>,
    /// Terminal dimensions
    size: TerminalSize,
    /// Is terminal active?
    active: bool,
    /// Associated package (for monorepo awareness)
    package: Option<String>,
}

/// Terminal identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerminalId(pub u64);

/// Terminal dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub rows: u16,
    pub cols: u16,
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self { rows: 24, cols: 80 }
    }
}

impl Terminal {
    /// Create a new terminal
    pub fn new(id: TerminalId) -> Self {
        Self {
            id,
            title: String::from("Terminal"),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            env: std::env::vars().collect(),
            pty: None,
            screen: Arc::new(RwLock::new(Screen::new(24, 80))),
            input_tx: None,
            size: TerminalSize::default(),
            active: false,
            package: None,
        }
    }

    /// Create a terminal for a specific package in the monorepo
    pub fn for_package(id: TerminalId, package_name: &str, package_path: PathBuf) -> Self {
        let mut term = Self::new(id);
        term.title = format!("Terminal: {}", package_name);
        term.cwd = package_path;
        term.package = Some(package_name.to_string());
        term
    }

    /// Set working directory
    pub fn with_cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = cwd;
        self
    }

    /// Set environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set terminal size
    pub fn with_size(mut self, rows: u16, cols: u16) -> Self {
        self.size = TerminalSize { rows, cols };
        self
    }

    /// Spawn shell process
    pub async fn spawn(&mut self, shell: Option<&str>) -> Result<()> {
        let shell = shell.unwrap_or_else(|| {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()).leak()
        });

        tracing::info!("Spawning terminal with shell: {}", shell);

        // Create PTY
        let (pty, input_tx) = Pty::spawn(
            shell,
            &self.cwd,
            &self.env,
            self.size,
            Arc::clone(&self.screen),
        ).await?;

        self.pty = Some(pty);
        self.input_tx = Some(input_tx);
        self.active = true;

        Ok(())
    }

    /// Write input to terminal
    pub fn write(&self, data: &[u8]) -> Result<()> {
        if let Some(tx) = &self.input_tx {
            tx.send(data.to_vec())
                .map_err(|e| anyhow::anyhow!("Failed to send input: {}", e))?;
        }
        Ok(())
    }

    /// Write a string to terminal
    pub fn write_str(&self, s: &str) -> Result<()> {
        self.write(s.as_bytes())
    }

    /// Resize terminal
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.size = TerminalSize { rows, cols };
        self.screen.write().resize(rows as usize, cols as usize);
        
        if let Some(pty) = &self.pty {
            pty.resize(rows, cols)?;
        }
        
        Ok(())
    }

    /// Get terminal ID
    pub fn id(&self) -> TerminalId {
        self.id
    }

    /// Get terminal title
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set terminal title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Get working directory
    pub fn cwd(&self) -> &PathBuf {
        &self.cwd
    }

    /// Get screen for rendering
    pub fn screen(&self) -> &Arc<RwLock<Screen>> {
        &self.screen
    }

    /// Get terminal size
    pub fn size(&self) -> TerminalSize {
        self.size
    }

    /// Is terminal active?
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get associated package
    pub fn package(&self) -> Option<&str> {
        self.package.as_deref()
    }

    /// Kill the terminal process
    pub fn kill(&mut self) -> Result<()> {
        if let Some(pty) = self.pty.take() {
            pty.kill()?;
        }
        self.active = false;
        self.input_tx = None;
        Ok(())
    }

    /// Clear the screen
    pub fn clear(&self) {
        self.screen.write().clear();
        // Send clear command to terminal
        let _ = self.write(b"\x1b[2J\x1b[H");
    }
}

/// Terminal manager - manages multiple terminals
pub struct TerminalManager {
    terminals: HashMap<TerminalId, Terminal>,
    next_id: u64,
    active_terminal: Option<TerminalId>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            terminals: HashMap::new(),
            next_id: 1,
            active_terminal: None,
        }
    }

    /// Create a new terminal
    pub fn create(&mut self) -> TerminalId {
        let id = TerminalId(self.next_id);
        self.next_id += 1;
        
        let terminal = Terminal::new(id);
        self.terminals.insert(id, terminal);
        
        if self.active_terminal.is_none() {
            self.active_terminal = Some(id);
        }
        
        id
    }

    /// Create a terminal for a package
    pub fn create_for_package(&mut self, name: &str, path: PathBuf) -> TerminalId {
        let id = TerminalId(self.next_id);
        self.next_id += 1;
        
        let terminal = Terminal::for_package(id, name, path);
        self.terminals.insert(id, terminal);
        
        id
    }

    /// Get a terminal by ID
    pub fn get(&self, id: TerminalId) -> Option<&Terminal> {
        self.terminals.get(&id)
    }

    /// Get a mutable terminal by ID
    pub fn get_mut(&mut self, id: TerminalId) -> Option<&mut Terminal> {
        self.terminals.get_mut(&id)
    }

    /// Get active terminal
    pub fn active(&self) -> Option<&Terminal> {
        self.active_terminal.and_then(|id| self.terminals.get(&id))
    }

    /// Get mutable active terminal
    pub fn active_mut(&mut self) -> Option<&mut Terminal> {
        self.active_terminal.and_then(|id| self.terminals.get_mut(&id))
    }

    /// Set active terminal
    pub fn set_active(&mut self, id: TerminalId) {
        if self.terminals.contains_key(&id) {
            self.active_terminal = Some(id);
        }
    }

    /// Close a terminal
    pub fn close(&mut self, id: TerminalId) -> Result<()> {
        if let Some(mut terminal) = self.terminals.remove(&id) {
            terminal.kill()?;
        }
        
        // Update active terminal if needed
        if self.active_terminal == Some(id) {
            self.active_terminal = self.terminals.keys().next().copied();
        }
        
        Ok(())
    }

    /// List all terminal IDs
    pub fn list(&self) -> Vec<TerminalId> {
        self.terminals.keys().copied().collect()
    }

    /// Number of terminals
    pub fn count(&self) -> usize {
        self.terminals.len()
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}
