//! LSP transport layer (JSON-RPC over stdio)

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::Mutex;
use anyhow::Result;

/// JSON-RPC transport
pub struct Transport {
    writer: Arc<Mutex<ChildStdin>>,
    reader: Arc<Mutex<BufReader<ChildStdout>>>,
}

impl Transport {
    /// Create new transport
    pub fn new(stdin: Option<ChildStdin>, stdout: Option<ChildStdout>) -> Self {
        Self {
            writer: Arc::new(Mutex::new(stdin.expect("stdin required"))),
            reader: Arc::new(Mutex::new(BufReader::new(stdout.expect("stdout required")))),
        }
    }

    /// Send a request
    pub async fn send_request(&self, id: i64, method: &str, params: serde_json::Value) -> Result<()> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        self.send_message(&request).await
    }

    /// Send a notification
    pub fn send_notification(&self, method: &str, params: serde_json::Value) -> Result<()> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let writer = self.writer.clone();
        let msg = format_message(&notification)?;

        tokio::spawn(async move {
            let mut writer = writer.lock().await;
            writer.write_all(msg.as_bytes()).await.ok();
            writer.flush().await.ok();
        });

        Ok(())
    }

    async fn send_message(&self, message: &serde_json::Value) -> Result<()> {
        let msg = format_message(message)?;

        let mut writer = self.writer.lock().await;
        writer.write_all(msg.as_bytes()).await?;
        writer.flush().await?;

        Ok(())
    }

    /// Read a message
    pub async fn read_message(&self) -> Result<serde_json::Value> {
        let mut reader = self.reader.lock().await;

        // Read headers
        let mut content_length: Option<usize> = None;
        let mut line = String::new();

        loop {
            line.clear();
            reader.read_line(&mut line).await?;

            let line = line.trim();
            if line.is_empty() {
                break;
            }

            if let Some(len) = line.strip_prefix("Content-Length: ") {
                content_length = Some(len.parse()?);
            }
        }

        let content_length = content_length
            .ok_or_else(|| anyhow::anyhow!("Missing Content-Length header"))?;

        // Read content
        let mut content = vec![0u8; content_length];
        reader.read_exact(&mut content).await?;

        let message: serde_json::Value = serde_json::from_slice(&content)?;

        Ok(message)
    }
}

fn format_message(message: &serde_json::Value) -> Result<String> {
    let content = serde_json::to_string(message)?;
    Ok(format!("Content-Length: {}\r\n\r\n{}", content.len(), content))
}

/// Message types
#[derive(Debug, Clone)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

#[derive(Debug, Clone)]
pub struct Request {
    pub id: i64,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct Response {
    pub id: i64,
    pub result: Option<serde_json::Value>,
    pub error: Option<ResponseError>,
}

#[derive(Debug, Clone)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub method: String,
    pub params: serde_json::Value,
}

impl Message {
    pub fn parse(value: serde_json::Value) -> Result<Self> {
        if value.get("id").is_some() && value.get("method").is_some() {
            // Request
            Ok(Message::Request(Request {
                id: value["id"].as_i64().unwrap_or(0),
                method: value["method"].as_str().unwrap_or("").to_string(),
                params: value.get("params").cloned().unwrap_or(serde_json::Value::Null),
            }))
        } else if value.get("id").is_some() {
            // Response
            Ok(Message::Response(Response {
                id: value["id"].as_i64().unwrap_or(0),
                result: value.get("result").cloned(),
                error: value.get("error").map(|e| ResponseError {
                    code: e["code"].as_i64().unwrap_or(0) as i32,
                    message: e["message"].as_str().unwrap_or("").to_string(),
                    data: e.get("data").cloned(),
                }),
            }))
        } else if value.get("method").is_some() {
            // Notification
            Ok(Message::Notification(Notification {
                method: value["method"].as_str().unwrap_or("").to_string(),
                params: value.get("params").cloned().unwrap_or(serde_json::Value::Null),
            }))
        } else {
            anyhow::bail!("Invalid message format")
        }
    }
}
