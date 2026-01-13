//! Code lens commands

use serde::{Deserialize, Serialize};

/// Code lens command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLensCommand {
    /// Display title
    pub title: String,
    /// Command ID to execute
    pub command: String,
    /// Command arguments
    #[serde(default)]
    pub arguments: Vec<serde_json::Value>,
}

impl CodeLensCommand {
    /// Create a new command
    pub fn new(title: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            command: command.into(),
            arguments: Vec::new(),
        }
    }

    /// Add argument
    pub fn with_arg(mut self, arg: serde_json::Value) -> Self {
        self.arguments.push(arg);
        self
    }

    /// Execute the command
    pub async fn execute(&self) -> anyhow::Result<()> {
        tracing::debug!("Executing code lens command: {}", self.command);
        
        match self.command.as_str() {
            "foxkit.showReferences" => {
                // Would show references panel
                tracing::info!("Show references: {:?}", self.arguments);
            }
            "foxkit.showImplementations" => {
                // Would show implementations panel
                tracing::info!("Show implementations: {:?}", self.arguments);
            }
            "foxkit.runTest" => {
                // Would run test
                tracing::info!("Run test: {:?}", self.arguments);
            }
            "foxkit.debugTest" => {
                // Would debug test
                tracing::info!("Debug test: {:?}", self.arguments);
            }
            "foxkit.run" => {
                // Would run main
                tracing::info!("Run: {:?}", self.arguments);
            }
            "foxkit.debug" => {
                // Would start debugging
                tracing::info!("Debug: {:?}", self.arguments);
            }
            _ => {
                tracing::warn!("Unknown code lens command: {}", self.command);
            }
        }

        Ok(())
    }
}

/// Standard code lens commands
pub mod standard {
    use super::*;

    /// Show references command
    pub fn show_references(file: &str, line: u32, column: u32) -> CodeLensCommand {
        CodeLensCommand::new("references", "foxkit.showReferences")
            .with_arg(serde_json::json!({
                "file": file,
                "line": line,
                "column": column
            }))
    }

    /// Show implementations command
    pub fn show_implementations(file: &str, line: u32) -> CodeLensCommand {
        CodeLensCommand::new("implementations", "foxkit.showImplementations")
            .with_arg(serde_json::json!({
                "file": file,
                "line": line
            }))
    }

    /// Run test command
    pub fn run_test(test_name: &str, file: &str) -> CodeLensCommand {
        CodeLensCommand::new("▶ Run Test", "foxkit.runTest")
            .with_arg(serde_json::json!({
                "name": test_name,
                "file": file
            }))
    }

    /// Debug test command
    pub fn debug_test(test_name: &str, file: &str) -> CodeLensCommand {
        CodeLensCommand::new("🐛 Debug Test", "foxkit.debugTest")
            .with_arg(serde_json::json!({
                "name": test_name,
                "file": file
            }))
    }

    /// Run command
    pub fn run(file: &str) -> CodeLensCommand {
        CodeLensCommand::new("▶ Run", "foxkit.run")
            .with_arg(serde_json::json!({ "file": file }))
    }

    /// Debug command
    pub fn debug(file: &str) -> CodeLensCommand {
        CodeLensCommand::new("🐛 Debug", "foxkit.debug")
            .with_arg(serde_json::json!({ "file": file }))
    }
}
