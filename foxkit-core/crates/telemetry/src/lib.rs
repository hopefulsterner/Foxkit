//! # Foxkit Telemetry
//!
//! Metrics, logging, tracing, and crash reporting.

pub mod metrics;
pub mod logging;
pub mod spans;
pub mod reporter;
pub mod crash;

use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub use metrics::{Metrics, Counter, Gauge, Histogram};
pub use logging::{Logger, LogLevel, LogEntry};
pub use spans::{Span, SpanContext, TraceId};
pub use reporter::TelemetryReporter;
pub use crash::CrashReporter;

/// Telemetry service
pub struct TelemetryService {
    /// Configuration
    config: RwLock<TelemetryConfig>,
    /// Metrics collector
    metrics: Metrics,
    /// Logger
    logger: Logger,
    /// Reporter
    reporter: Option<Arc<dyn TelemetryReporter>>,
    /// Crash reporter
    crash_reporter: Option<CrashReporter>,
}

impl TelemetryService {
    pub fn new(config: TelemetryConfig) -> Self {
        Self {
            config: RwLock::new(config.clone()),
            metrics: Metrics::new(),
            logger: Logger::new(config.log_level),
            reporter: None,
            crash_reporter: None,
        }
    }

    /// Initialize telemetry
    pub fn init(&self) -> anyhow::Result<()> {
        let config = self.config.read();

        // Initialize logging
        self.logger.init(&config)?;

        // Initialize metrics
        if config.metrics_enabled {
            self.metrics.init(&config)?;
        }

        tracing::info!("Telemetry initialized");
        Ok(())
    }

    /// Get metrics
    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// Get logger
    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    /// Set reporter
    pub fn set_reporter<R: TelemetryReporter + 'static>(&mut self, reporter: R) {
        self.reporter = Some(Arc::new(reporter));
    }

    /// Enable crash reporting
    pub fn enable_crash_reporting(&mut self, endpoint: Option<String>) {
        self.crash_reporter = Some(CrashReporter::new(endpoint));
    }

    /// Track an event
    pub fn track(&self, event: Event) {
        if let Some(ref reporter) = self.reporter {
            reporter.report(event);
        }
    }

    /// Start a span
    pub fn span(&self, name: &str) -> Span {
        Span::new(name)
    }

    /// Flush all pending telemetry
    pub async fn flush(&self) {
        if let Some(ref reporter) = self.reporter {
            reporter.flush().await;
        }
    }

    /// Shutdown telemetry
    pub async fn shutdown(&self) {
        self.flush().await;
        tracing::info!("Telemetry shutdown");
    }
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TelemetryConfig {
    /// Enable telemetry
    pub enabled: bool,
    /// Log level
    pub log_level: LogLevel,
    /// Enable metrics collection
    pub metrics_enabled: bool,
    /// Metrics endpoint
    pub metrics_endpoint: Option<String>,
    /// Enable tracing
    pub tracing_enabled: bool,
    /// OTLP endpoint
    pub otlp_endpoint: Option<String>,
    /// Enable crash reporting
    pub crash_reporting: bool,
    /// Crash report endpoint
    pub crash_endpoint: Option<String>,
    /// Sample rate (0.0-1.0)
    pub sample_rate: f64,
    /// Machine ID for anonymized tracking
    pub machine_id: Option<String>,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: LogLevel::Info,
            metrics_enabled: true,
            metrics_endpoint: None,
            tracing_enabled: false,
            otlp_endpoint: None,
            crash_reporting: true,
            crash_endpoint: None,
            sample_rate: 1.0,
            machine_id: None,
        }
    }
}

/// Telemetry event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event name
    pub name: String,
    /// Event category
    pub category: EventCategory,
    /// Properties
    pub properties: std::collections::HashMap<String, serde_json::Value>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Event {
    pub fn new(name: impl Into<String>, category: EventCategory) -> Self {
        Self {
            name: name.into(),
            category,
            properties: std::collections::HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_property(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.properties.insert(key.into(), v);
        }
        self
    }
}

/// Event category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    Action,
    Performance,
    Error,
    Feature,
    Session,
}
