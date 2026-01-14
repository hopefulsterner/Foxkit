//! Model Context Protocol (MCP) Client
//!
//! MCP enables AI to use external tools through a standardized protocol.
//! This module implements the MCP client for connecting to MCP servers.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;
use parking_lot::RwLock as SyncRwLock;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::{Tool, ToolCall, ToolResult, ToolSchema};

/// MCP Server connection types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpServerConfig {
    /// Connect via stdio (spawns process)
    #[serde(rename = "stdio")]
    Stdio {
        command: String,
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    },
    /// Connect via Server-Sent Events (SSE)
    #[serde(rename = "sse")]
    Sse {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

/// MCP Protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "jsonrpc")]
pub struct JsonRpcMessage {
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcMessage {
    pub fn request(id: u64, method: &str, params: Value) -> Self {
        Self {
            id: Some(id),
            method: Some(method.to_string()),
            params: Some(params),
            result: None,
            error: None,
        }
    }

    pub fn notification(method: &str, params: Value) -> Self {
        Self {
            id: None,
            method: Some(method.to_string()),
            params: Some(params),
            result: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// MCP Server capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(default)]
    pub tools: Option<ToolsCapability>,
    #[serde(default)]
    pub resources: Option<ResourcesCapability>,
    #[serde(default)]
    pub prompts: Option<PromptsCapability>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(default)]
    pub list_changed: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourcesCapability {
    #[serde(default)]
    pub subscribe: bool,
    #[serde(default)]
    pub list_changed: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptsCapability {
    #[serde(default)]
    pub list_changed: bool,
}

/// MCP Tool definition from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub input_schema: Value,
}

/// MCP Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// MCP Prompt definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub arguments: Vec<McpPromptArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

/// MCP Client for communicating with an MCP server
pub struct McpClient {
    name: String,
    config: McpServerConfig,
    capabilities: SyncRwLock<ServerCapabilities>,
    tools: SyncRwLock<Vec<McpToolDef>>,
    resources: SyncRwLock<Vec<McpResource>>,
    prompts: SyncRwLock<Vec<McpPrompt>>,
    request_id: SyncRwLock<u64>,
    connected: SyncRwLock<bool>,
    // For stdio transport
    child: RwLock<Option<Child>>,
}

impl McpClient {
    /// Create a new MCP client
    pub fn new(name: impl Into<String>, config: McpServerConfig) -> Self {
        Self {
            name: name.into(),
            config,
            capabilities: SyncRwLock::new(ServerCapabilities::default()),
            tools: SyncRwLock::new(Vec::new()),
            resources: SyncRwLock::new(Vec::new()),
            prompts: SyncRwLock::new(Vec::new()),
            request_id: SyncRwLock::new(0),
            connected: SyncRwLock::new(false),
            child: RwLock::new(None),
        }
    }

    /// Get server name
    pub fn name(&self) -> &str {
        &self.name
    }

    fn next_id(&self) -> u64 {
        let mut id = self.request_id.write();
        *id += 1;
        *id
    }

    /// Initialize connection to the MCP server
    pub async fn connect(&self) -> Result<()> {
        match &self.config {
            McpServerConfig::Stdio { command, args, env } => {
                self.connect_stdio(command, args, env).await?;
            }
            McpServerConfig::Sse { url, headers } => {
                self.connect_sse(url, headers).await?;
            }
        }

        // Initialize protocol
        self.initialize().await?;
        
        // List available tools
        self.refresh_tools().await?;
        
        *self.connected.write() = true;
        Ok(())
    }

    async fn connect_stdio(
        &self,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<()> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.envs(env);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let child = cmd.spawn()?;
        *self.child.write().await = Some(child);
        
        Ok(())
    }

    async fn connect_sse(&self, _url: &str, _headers: &HashMap<String, String>) -> Result<()> {
        // SSE connection - would use reqwest with SSE support
        // For now, we focus on stdio which is more common
        Ok(())
    }

    async fn initialize(&self) -> Result<ServerCapabilities> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "roots": {
                    "listChanged": true
                }
            },
            "clientInfo": {
                "name": "foxkit",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        let result = self.send_request("initialize", params).await?;
        
        let capabilities: ServerCapabilities = serde_json::from_value(
            result.get("capabilities").cloned().unwrap_or_default()
        )?;
        
        *self.capabilities.write() = capabilities.clone();
        
        // Send initialized notification
        self.send_notification("notifications/initialized", serde_json::json!({})).await?;
        
        Ok(capabilities)
    }

    /// Refresh the list of available tools
    pub async fn refresh_tools(&self) -> Result<Vec<McpToolDef>> {
        let result = self.send_request("tools/list", serde_json::json!({})).await?;
        
        let tools: Vec<McpToolDef> = serde_json::from_value(
            result.get("tools").cloned().unwrap_or(Value::Array(vec![]))
        )?;
        
        *self.tools.write() = tools.clone();
        Ok(tools)
    }

    /// List available tools
    pub fn list_tools(&self) -> Vec<McpToolDef> {
        self.tools.read().clone()
    }

    /// Call a tool on the MCP server
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        let result = self.send_request("tools/call", params).await?;
        
        // Handle the content array response
        if let Some(content) = result.get("content") {
            if let Some(arr) = content.as_array() {
                if let Some(first) = arr.first() {
                    if let Some(text) = first.get("text") {
                        return Ok(text.clone());
                    }
                }
            }
        }
        
        Ok(result)
    }

    /// List available resources
    pub async fn list_resources(&self) -> Result<Vec<McpResource>> {
        let result = self.send_request("resources/list", serde_json::json!({})).await?;
        
        let resources: Vec<McpResource> = serde_json::from_value(
            result.get("resources").cloned().unwrap_or(Value::Array(vec![]))
        )?;
        
        *self.resources.write() = resources.clone();
        Ok(resources)
    }

    /// Read a resource
    pub async fn read_resource(&self, uri: &str) -> Result<Value> {
        let params = serde_json::json!({
            "uri": uri
        });

        self.send_request("resources/read", params).await
    }

    /// List available prompts
    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>> {
        let result = self.send_request("prompts/list", serde_json::json!({})).await?;
        
        let prompts: Vec<McpPrompt> = serde_json::from_value(
            result.get("prompts").cloned().unwrap_or(Value::Array(vec![]))
        )?;
        
        *self.prompts.write() = prompts.clone();
        Ok(prompts)
    }

    /// Get a prompt
    pub async fn get_prompt(&self, name: &str, arguments: HashMap<String, String>) -> Result<Value> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        self.send_request("prompts/get", params).await
    }

    async fn send_request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id();
        let message = JsonRpcMessage::request(id, method, params);
        
        self.send_message(&message).await?;
        self.receive_response(id).await
    }

    async fn send_notification(&self, method: &str, params: Value) -> Result<()> {
        let message = JsonRpcMessage::notification(method, params);
        self.send_message(&message).await
    }

    async fn send_message(&self, message: &JsonRpcMessage) -> Result<()> {
        let json = serde_json::to_string(message)?;
        
        match &self.config {
            McpServerConfig::Stdio { .. } => {
                let mut child_guard = self.child.write().await;
                if let Some(child) = child_guard.as_mut() {
                    if let Some(stdin) = child.stdin.as_mut() {
                        stdin.write_all(json.as_bytes()).await?;
                        stdin.write_all(b"\n").await?;
                        stdin.flush().await?;
                    }
                }
            }
            McpServerConfig::Sse { .. } => {
                // HTTP POST for SSE transport
            }
        }
        
        Ok(())
    }

    async fn receive_response(&self, expected_id: u64) -> Result<Value> {
        match &self.config {
            McpServerConfig::Stdio { .. } => {
                let mut child_guard = self.child.write().await;
                if let Some(child) = child_guard.as_mut() {
                    if let Some(stdout) = child.stdout.as_mut() {
                        let mut reader = BufReader::new(stdout);
                        let mut line = String::new();
                        
                        loop {
                            line.clear();
                            let n = reader.read_line(&mut line).await?;
                            if n == 0 {
                                anyhow::bail!("MCP server disconnected");
                            }
                            
                            if let Ok(msg) = serde_json::from_str::<JsonRpcMessage>(&line) {
                                if msg.id == Some(expected_id) {
                                    if let Some(error) = msg.error {
                                        anyhow::bail!("MCP error: {} - {}", error.code, error.message);
                                    }
                                    return Ok(msg.result.unwrap_or(Value::Null));
                                }
                            }
                        }
                    }
                }
                anyhow::bail!("No stdout available")
            }
            McpServerConfig::Sse { .. } => {
                anyhow::bail!("SSE response handling not implemented")
            }
        }
    }

    /// Disconnect from the MCP server
    pub async fn disconnect(&self) -> Result<()> {
        *self.connected.write() = false;
        
        let mut child_guard = self.child.write().await;
        if let Some(mut child) = child_guard.take() {
            child.kill().await?;
        }
        
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.read()
    }
}

/// MCP Tool wrapper that implements our Tool trait
pub struct McpTool {
    client: Arc<McpClient>,
    definition: McpToolDef,
}

impl McpTool {
    pub fn new(client: Arc<McpClient>, definition: McpToolDef) -> Self {
        Self { client, definition }
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        &self.definition.name
    }

    fn description(&self) -> &str {
        self.definition.description.as_deref().unwrap_or("")
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.definition.name.clone(),
            description: self.definition.description.clone().unwrap_or_default(),
            parameters: self.definition.input_schema.clone(),
        }
    }

    async fn execute(&self, arguments: Value) -> Result<ToolResult> {
        let result = self.client.call_tool(&self.definition.name, arguments).await?;
        
        let content = match result {
            Value::String(s) => s,
            other => serde_json::to_string_pretty(&other)?,
        };
        
        Ok(ToolResult::success("", content))
    }
}

/// Registry of MCP servers
pub struct McpRegistry {
    clients: SyncRwLock<HashMap<String, Arc<McpClient>>>,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            clients: SyncRwLock::new(HashMap::new()),
        }
    }

    /// Register an MCP server configuration
    pub fn register(&self, name: impl Into<String>, config: McpServerConfig) {
        let name = name.into();
        let client = Arc::new(McpClient::new(&name, config));
        self.clients.write().insert(name, client);
    }

    /// Get an MCP client
    pub fn get(&self, name: &str) -> Option<Arc<McpClient>> {
        self.clients.read().get(name).cloned()
    }

    /// Connect all registered servers
    pub async fn connect_all(&self) -> Result<()> {
        let clients: Vec<Arc<McpClient>> = self.clients.read().values().cloned().collect();
        for client in clients {
            client.connect().await?;
        }
        Ok(())
    }

    /// Get all tools from all connected MCP servers
    pub fn all_tools(&self) -> Vec<Arc<dyn Tool>> {
        let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
        
        let clients: Vec<Arc<McpClient>> = self.clients.read().values().cloned().collect();
        for client in clients {
            if client.is_connected() {
                for tool_def in client.list_tools() {
                    tools.push(Arc::new(McpTool::new(Arc::clone(&client), tool_def)));
                }
            }
        }
        
        tools
    }

    /// List all registered servers
    pub fn list(&self) -> Vec<String> {
        self.clients.read().keys().cloned().collect()
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_message() {
        let msg = JsonRpcMessage::request(1, "tools/list", serde_json::json!({}));
        assert_eq!(msg.id, Some(1));
        assert_eq!(msg.method, Some("tools/list".to_string()));
    }

    #[test]
    fn test_mcp_server_config() {
        let config = McpServerConfig::Stdio {
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@anthropic/mcp-server-fs".to_string()],
            env: HashMap::new(),
        };
        
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("stdio"));
    }
}
