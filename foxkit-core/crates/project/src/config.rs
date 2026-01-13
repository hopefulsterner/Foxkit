//! Project configuration

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Project-specific configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Build configuration
    #[serde(default)]
    pub build: BuildConfig,
    /// Run configuration
    #[serde(default)]
    pub run: RunConfig,
    /// Test configuration
    #[serde(default)]
    pub test: TestConfig,
    /// Debug configuration
    #[serde(default)]
    pub debug: DebugConfig,
    /// Custom settings
    #[serde(flatten)]
    pub custom: HashMap<String, Value>,
}

impl ProjectConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load from file
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        
        if path.extension().map(|e| e == "toml").unwrap_or(false) {
            Ok(toml::from_str(&content)?)
        } else {
            Ok(serde_json::from_str(&content)?)
        }
    }

    /// Save to file
    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = if path.extension().map(|e| e == "toml").unwrap_or(false) {
            toml::to_string_pretty(self)?
        } else {
            serde_json::to_string_pretty(self)?
        };
        
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Build configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Build command
    pub command: Option<String>,
    /// Working directory
    pub cwd: Option<PathBuf>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Build arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Pre-build commands
    #[serde(default)]
    pub pre_build: Vec<String>,
    /// Post-build commands
    #[serde(default)]
    pub post_build: Vec<String>,
}

/// Run configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunConfig {
    /// Run command
    pub command: Option<String>,
    /// Program arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory
    pub cwd: Option<PathBuf>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Port to use (for servers)
    pub port: Option<u16>,
}

/// Test configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestConfig {
    /// Test command
    pub command: Option<String>,
    /// Test framework
    pub framework: Option<String>,
    /// Test patterns
    #[serde(default)]
    pub patterns: Vec<String>,
    /// Coverage enabled
    pub coverage: bool,
    /// Watch mode
    pub watch: bool,
}

/// Debug configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugConfig {
    /// Debug type
    #[serde(rename = "type")]
    pub debug_type: Option<String>,
    /// Request type (launch/attach)
    pub request: Option<String>,
    /// Program to debug
    pub program: Option<PathBuf>,
    /// Program arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory
    pub cwd: Option<PathBuf>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Port for attach
    pub port: Option<u16>,
    /// Stop on entry
    pub stop_on_entry: bool,
    /// Source maps enabled
    pub source_maps: bool,
}
