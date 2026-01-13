//! Application lifecycle management

use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;

/// Application lifecycle trait
#[async_trait]
pub trait App: Send + Sync {
    /// Called when the application starts
    async fn on_start(&self) -> Result<()>;
    
    /// Called when the application is shutting down
    async fn on_shutdown(&self) -> Result<()>;
    
    /// Called when the application is activated (focused)
    fn on_activate(&self) {}
    
    /// Called when the application is deactivated (unfocused)  
    fn on_deactivate(&self) {}
}

/// Application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Starting,
    Running,
    ShuttingDown,
    Stopped,
}

/// Platform-agnostic application runner
pub struct AppRunner {
    state: AppState,
}

impl AppRunner {
    pub fn new() -> Self {
        Self {
            state: AppState::Starting,
        }
    }

    pub fn state(&self) -> AppState {
        self.state
    }
}
