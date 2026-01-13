//! Logging subsystem

use std::sync::Arc;
use std::path::PathBuf;
use std::collections::VecDeque;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing_subscriber::{
    EnvFilter,
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

use crate::TelemetryConfig;

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }

    pub fn to_tracing(&self) -> tracing::Level {
        match self {
            Self::Trace => tracing::Level::TRACE,
            Self::Debug => tracing::Level::DEBUG,
            Self::Info => tracing::Level::INFO,
            Self::Warn => tracing::Level::WARN,
            Self::Error => tracing::Level::ERROR,
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" | "warning" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => Err("Invalid log level"),
        }
    }
}

/// Logger
pub struct Logger {
    /// Current log level
    level: RwLock<LogLevel>,
    /// Log file path
    file: RwLock<Option<PathBuf>>,
    /// In-memory log buffer (for UI)
    buffer: RwLock<VecDeque<LogEntry>>,
    /// Buffer capacity
    buffer_capacity: usize,
    /// Subscribers
    subscribers: RwLock<Vec<Box<dyn Fn(&LogEntry) + Send + Sync>>>,
}

impl Logger {
    pub fn new(level: LogLevel) -> Self {
        Self {
            level: RwLock::new(level),
            file: RwLock::new(None),
            buffer: RwLock::new(VecDeque::new()),
            buffer_capacity: 10000,
            subscribers: RwLock::new(Vec::new()),
        }
    }

    /// Initialize logging
    pub fn init(&self, config: &TelemetryConfig) -> anyhow::Result<()> {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(config.log_level.as_str()));

        let subscriber = tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_target(true).with_thread_ids(true));

        subscriber.try_init().ok();

        Ok(())
    }

    /// Set log level
    pub fn set_level(&self, level: LogLevel) {
        *self.level.write() = level;
    }

    /// Get log level
    pub fn level(&self) -> LogLevel {
        *self.level.read()
    }

    /// Set log file
    pub fn set_file(&self, path: PathBuf) {
        *self.file.write() = Some(path);
    }

    /// Log an entry
    pub fn log(&self, entry: LogEntry) {
        // Add to buffer
        let mut buffer = self.buffer.write();
        if buffer.len() >= self.buffer_capacity {
            buffer.pop_front();
        }
        buffer.push_back(entry.clone());
        drop(buffer);

        // Notify subscribers
        for subscriber in self.subscribers.read().iter() {
            subscriber(&entry);
        }
    }

    /// Get recent log entries
    pub fn recent(&self, count: usize) -> Vec<LogEntry> {
        let buffer = self.buffer.read();
        buffer.iter().rev().take(count).cloned().collect()
    }

    /// Subscribe to log entries
    pub fn subscribe<F>(&self, callback: F)
    where
        F: Fn(&LogEntry) + Send + Sync + 'static,
    {
        self.subscribers.write().push(Box::new(callback));
    }

    /// Clear log buffer
    pub fn clear(&self) {
        self.buffer.write().clear();
    }

    /// Search logs
    pub fn search(&self, query: &str) -> Vec<LogEntry> {
        let query = query.to_lowercase();
        self.buffer
            .read()
            .iter()
            .filter(|e| e.message.to_lowercase().contains(&query))
            .cloned()
            .collect()
    }

    /// Filter by level
    pub fn filter_by_level(&self, level: LogLevel) -> Vec<LogEntry> {
        self.buffer
            .read()
            .iter()
            .filter(|e| e.level as u8 >= level as u8)
            .cloned()
            .collect()
    }
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Log level
    pub level: LogLevel,
    /// Message
    pub message: String,
    /// Target (module path)
    pub target: Option<String>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Thread ID
    pub thread_id: Option<u64>,
    /// Span context
    pub span: Option<String>,
    /// Additional fields
    pub fields: std::collections::HashMap<String, serde_json::Value>,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            target: None,
            timestamp: chrono::Utc::now(),
            thread_id: None,
            span: None,
            fields: std::collections::HashMap::new(),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Info, message)
    }

    pub fn warn(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Warn, message)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Error, message)
    }

    pub fn debug(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Debug, message)
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.fields.insert(key.into(), v);
        }
        self
    }
}

/// Output writer for log file rotation
pub struct RotatingFileWriter {
    path: PathBuf,
    max_size: u64,
    max_files: usize,
    current_size: u64,
}

impl RotatingFileWriter {
    pub fn new(path: PathBuf, max_size: u64, max_files: usize) -> Self {
        Self {
            path,
            max_size,
            max_files,
            current_size: 0,
        }
    }

    /// Rotate log files
    pub fn rotate(&mut self) -> anyhow::Result<()> {
        // Rename existing files
        for i in (1..self.max_files).rev() {
            let from = self.path.with_extension(format!("log.{}", i));
            let to = self.path.with_extension(format!("log.{}", i + 1));
            if from.exists() {
                std::fs::rename(&from, &to)?;
            }
        }

        // Rename current to .1
        let to = self.path.with_extension("log.1");
        if self.path.exists() {
            std::fs::rename(&self.path, &to)?;
        }

        self.current_size = 0;
        Ok(())
    }
}
