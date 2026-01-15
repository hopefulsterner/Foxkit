//! Crash reporting

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Crash reporter
pub struct CrashReporter {
    /// Crash report endpoint
    endpoint: Option<String>,
    /// Pending reports
    pending: Mutex<Vec<CrashReport>>,
    /// Crash dump directory
    dump_dir: PathBuf,
}

impl CrashReporter {
    pub fn new(endpoint: Option<String>) -> Self {
        let dump_dir = std::env::temp_dir().join("foxkit-crashes");
        std::fs::create_dir_all(&dump_dir).ok();

        Self {
            endpoint,
            pending: Mutex::new(Vec::new()),
            dump_dir,
        }
    }

    /// Install panic hook
    pub fn install_panic_hook(&self) {
        let dump_dir = self.dump_dir.clone();
        
        std::panic::set_hook(Box::new(move |info| {
            // Collect crash info
            let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            let location = info.location().map(|l| {
                format!("{}:{}:{}", l.file(), l.line(), l.column())
            });

            let report = CrashReport {
                id: uuid_v4(),
                timestamp: chrono::Utc::now(),
                message,
                location,
                backtrace: capture_backtrace(),
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                context: std::collections::HashMap::new(),
            };

            // Write to disk
            let path = dump_dir.join(format!("crash-{}.json", report.id));
            if let Ok(json) = serde_json::to_string_pretty(&report) {
                std::fs::write(&path, json).ok();
            }

            // Print to stderr
            eprintln!("PANIC: {}", report.message);
            if let Some(ref loc) = report.location {
                eprintln!("  at {}", loc);
            }
        }));
    }

    /// Record a crash report
    pub fn record(&self, report: CrashReport) {
        self.pending.lock().push(report);
    }

    /// Send pending reports
    pub async fn send_pending(&self) -> anyhow::Result<usize> {
        let endpoint = match &self.endpoint {
            Some(e) => e,
            None => return Ok(0),
        };

        let reports: Vec<CrashReport> = self.pending.lock().drain(..).collect();
        if reports.is_empty() {
            return Ok(0);
        }

        let client = reqwest::Client::new();
        
        for report in &reports {
            client
                .post(endpoint)
                .json(report)
                .send()
                .await?;
        }

        Ok(reports.len())
    }

    /// Load crash reports from disk
    pub fn load_pending(&self) -> Vec<CrashReport> {
        let mut reports = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.dump_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(report) = serde_json::from_str(&content) {
                            reports.push(report);
                        }
                    }
                }
            }
        }

        reports
    }

    /// Clean up sent reports
    pub fn cleanup(&self) {
        if let Ok(entries) = std::fs::read_dir(&self.dump_dir) {
            for entry in entries.flatten() {
                std::fs::remove_file(entry.path()).ok();
            }
        }
    }
}

/// Crash report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    /// Report ID
    pub id: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Crash message
    pub message: String,
    /// Source location
    pub location: Option<String>,
    /// Backtrace
    pub backtrace: Option<String>,
    /// OS
    pub os: String,
    /// Architecture
    pub arch: String,
    /// App version
    pub version: String,
    /// Additional context
    pub context: std::collections::HashMap<String, String>,
}

impl CrashReport {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            id: uuid_v4(),
            timestamp: chrono::Utc::now(),
            message: message.into(),
            location: None,
            backtrace: capture_backtrace(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            context: std::collections::HashMap::new(),
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

/// Generate a UUID v4
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id();
    format!("{:016x}-{:08x}", nanos, pid)
}

/// Capture backtrace
fn capture_backtrace() -> Option<String> {
    // SAFETY: This is called from a panic handler, which is single-threaded context
    unsafe { std::env::set_var("RUST_BACKTRACE", "1"); }
    let bt = std::backtrace::Backtrace::capture();
    match bt.status() {
        std::backtrace::BacktraceStatus::Captured => Some(bt.to_string()),
        _ => None,
    }
}

/// Error reporter helper
pub struct ErrorReporter {
    crash_reporter: Arc<CrashReporter>,
}

impl ErrorReporter {
    pub fn new(crash_reporter: Arc<CrashReporter>) -> Self {
        Self { crash_reporter }
    }

    /// Report an error
    pub fn report_error(&self, error: &anyhow::Error) {
        let report = CrashReport::new(error.to_string())
            .with_context("type", "error");
        
        self.crash_reporter.record(report);
    }
}
