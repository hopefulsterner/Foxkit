//! Task configuration

use serde::{Deserialize, Serialize};
use crate::Task;

/// Task configuration file (tasks.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    /// Config version
    #[serde(default = "default_version")]
    pub version: String,
    /// Tasks
    #[serde(default)]
    pub tasks: Vec<Task>,
    /// OS-specific configuration
    pub windows: Option<OsConfig>,
    pub linux: Option<OsConfig>,
    pub osx: Option<OsConfig>,
}

fn default_version() -> String {
    "2.0.0".to_string()
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            tasks: Vec::new(),
            windows: None,
            linux: None,
            osx: None,
        }
    }
}

/// OS-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsConfig {
    pub shell: Option<String>,
    pub args: Option<Vec<String>>,
}

impl TaskConfig {
    /// Load from JSON string
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Create a sample configuration
    pub fn sample() -> Self {
        Self {
            version: default_version(),
            tasks: vec![
                Task::shell("build", "npm run build"),
                Task::shell("test", "npm test"),
                Task::shell("dev", "npm run dev").in_background(),
            ],
            windows: None,
            linux: None,
            osx: None,
        }
    }
}
