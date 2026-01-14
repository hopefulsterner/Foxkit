//! AI Tools / Function Calling
//! 
//! Tools that the AI can use to interact with the IDE and codebase

use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use globset::{Glob, GlobSetBuilder};
use rayon::prelude::*;

/// Tool trait - implement this to create AI-callable tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (used for function calling)
    fn name(&self) -> &str;
    
    /// Tool description (shown to AI)
    fn description(&self) -> &str;
    
    /// JSON schema for tool parameters
    fn schema(&self) -> ToolSchema;
    
    /// Execute the tool with given arguments
    async fn execute(&self, arguments: Value) -> Result<ToolResult>;
}

/// Tool schema for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Tool call from AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Result of tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn success(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            content: content.into(),
            is_error: false,
        }
    }

    pub fn error(tool_call_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            content: error.into(),
            is_error: true,
        }
    }
}

// ============================================================================
// Built-in Tools
// ============================================================================

/// Read file contents
pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file in the workspace"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().into(),
            description: self.description().into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "Optional start line (1-indexed)"
                    },
                    "end_line": {
                        "type": "integer", 
                        "description": "Optional end line (1-indexed)"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(&self, arguments: Value) -> Result<ToolResult> {
        let path = arguments["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("path is required"))?;
        
        let content = tokio::fs::read_to_string(path).await?;
        
        // Handle line range if specified
        let start = arguments["start_line"].as_u64().map(|n| n as usize);
        let end = arguments["end_line"].as_u64().map(|n| n as usize);
        
        let result = if start.is_some() || end.is_some() {
            let lines: Vec<&str> = content.lines().collect();
            let start = start.unwrap_or(1).saturating_sub(1);
            let end = end.unwrap_or(lines.len()).min(lines.len());
            lines[start..end].join("\n")
        } else {
            content
        };

        Ok(ToolResult::success("", result))
    }
}

/// Write/create file
pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file (creates if doesn't exist)"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().into(),
            description: self.description().into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn execute(&self, arguments: Value) -> Result<ToolResult> {
        let path = arguments["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("path is required"))?;
        let content = arguments["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("content is required"))?;
        
        // Create parent directories if needed
        if let Some(parent) = std::path::Path::new(path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        tokio::fs::write(path, content).await?;
        
        Ok(ToolResult::success("", format!("Successfully wrote to {}", path)))
    }
}

/// Search codebase
pub struct SearchTool;

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search_code"
    }

    fn description(&self) -> &str {
        "Search for code patterns in the workspace"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().into(),
            description: self.description().into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query (supports regex)"
                    },
                    "path_pattern": {
                        "type": "string",
                        "description": "Optional glob pattern to filter files"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results (default 20)"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, arguments: Value) -> Result<ToolResult> {
        let query = arguments["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("query is required"))?;
        
        let path_pattern = arguments["path_pattern"].as_str();
        let max_results = arguments["max_results"].as_u64().unwrap_or(20) as usize;
        
        // Prepare glob filter
        let glob_set = if let Some(pattern) = path_pattern {
            let mut builder = GlobSetBuilder::new();
            builder.add(Glob::new(pattern)?);
            Some(builder.build()?)
        } else {
            None
        };

        // TODO: Get workspace root from context/settings
        let root = Path::new(".");
        
        // Collect all candidate files
        let files: Vec<PathBuf> = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != "target" && name != "node_modules" && !name.starts_with('.')
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .filter(|p| {
                if let Some(ref gs) = glob_set {
                    gs.is_match(p)
                } else {
                    true
                }
            })
            .collect();

        // Search files in parallel using rayon
        let mut results: Vec<String> = files.par_iter()
            .filter_map(|path| {
                let content = std::fs::read_to_string(path).ok()?;
                let mut matches = Vec::new();
                
                for (i, line) in content.lines().enumerate() {
                    if line.contains(query) {
                        matches.push(format!("{}:{}: {}", path.display(), i + 1, line.trim()));
                        if matches.len() >= max_results { break; }
                    }
                }
                
                if matches.is_empty() {
                    None
                } else {
                    Some(matches.join("\n"))
                }
            })
            .collect();
        
        results.truncate(max_results);

        if results.is_empty() {
            Ok(ToolResult::success("", "No matches found"))
        } else {
            Ok(ToolResult::success("", results.join("\n")))
        }
    }
}

/// Run terminal command
pub struct RunCommandTool;

#[async_trait]
impl Tool for RunCommandTool {
    fn name(&self) -> &str {
        "run_command"
    }

    fn description(&self) -> &str {
        "Run a shell command in the workspace"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().into(),
            description: self.description().into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command to run"
                    },
                    "working_dir": {
                        "type": "string",
                        "description": "Optional working directory"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, arguments: Value) -> Result<ToolResult> {
        let command = arguments["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("command is required"))?;
        
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        let result = if output.status.success() {
            stdout.to_string()
        } else {
            format!("Error (exit code {:?}):\n{}\n{}", output.status.code(), stdout, stderr)
        };

        Ok(ToolResult::success("", result))
    }
}

/// Get package dependencies
pub struct GetDependenciesTool;

#[async_trait]
impl Tool for GetDependenciesTool {
    fn name(&self) -> &str {
        "get_dependencies"
    }

    fn description(&self) -> &str {
        "Get the dependencies of a package in the monorepo"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().into(),
            description: self.description().into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "package": {
                        "type": "string",
                        "description": "Package name"
                    }
                },
                "required": ["package"]
            }),
        }
    }

    async fn execute(&self, arguments: Value) -> Result<ToolResult> {
        let _package = arguments["package"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("package is required"))?;
        
        // TODO: Use MonorepoIntel to get dependencies
        Ok(ToolResult::success("", "Dependencies lookup not yet implemented"))
    }
}
