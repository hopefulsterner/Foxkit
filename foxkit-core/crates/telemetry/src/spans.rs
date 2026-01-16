//! Distributed tracing spans

use std::sync::Arc;
use std::time::Instant;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Trace ID (128-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(pub u128);

impl TraceId {
    pub fn new() -> Self {
        Self(rand_id())
    }

    pub fn to_hex(&self) -> String {
        format!("{:032x}", self.0)
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

/// Span ID (64-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(pub u64);

impl SpanId {
    pub fn new() -> Self {
        Self(rand_id() as u64)
    }

    pub fn to_hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}

impl Default for SpanId {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate random ID
fn rand_id() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    nanos ^ (std::process::id() as u128) << 64
}

/// Span context for distributed tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanContext {
    /// Trace ID
    pub trace_id: TraceId,
    /// Span ID
    pub span_id: SpanId,
    /// Parent span ID
    pub parent_span_id: Option<SpanId>,
    /// Trace flags
    pub trace_flags: TraceFlags,
    /// Trace state (vendor-specific)
    pub trace_state: Option<String>,
}

impl SpanContext {
    pub fn new() -> Self {
        Self {
            trace_id: TraceId::new(),
            span_id: SpanId::new(),
            parent_span_id: None,
            trace_flags: TraceFlags::SAMPLED,
            trace_state: None,
        }
    }

    /// Create child context
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id,
            span_id: SpanId::new(),
            parent_span_id: Some(self.span_id),
            trace_flags: self.trace_flags,
            trace_state: self.trace_state.clone(),
        }
    }

    /// Parse from W3C traceparent header
    pub fn from_traceparent(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 {
            return None;
        }

        let trace_id = u128::from_str_radix(parts[1], 16).ok()?;
        let span_id = u64::from_str_radix(parts[2], 16).ok()?;
        let flags = u8::from_str_radix(parts[3], 16).ok()?;

        Some(Self {
            trace_id: TraceId(trace_id),
            span_id: SpanId(span_id),
            parent_span_id: None,
            trace_flags: TraceFlags::from_bits_truncate(flags),
            trace_state: None,
        })
    }

    /// Convert to W3C traceparent header
    pub fn to_traceparent(&self) -> String {
        format!(
            "00-{}-{}-{:02x}",
            self.trace_id.to_hex(),
            self.span_id.to_hex(),
            self.trace_flags.bits()
        )
    }
}

impl Default for SpanContext {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Trace flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TraceFlags: u8 {
        const SAMPLED = 0x01;
    }
}

impl serde::Serialize for TraceFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(self.bits())
    }
}

impl<'de> serde::Deserialize<'de> for TraceFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bits = u8::deserialize(deserializer)?;
        Ok(TraceFlags::from_bits_truncate(bits))
    }
}

/// A tracing span
pub struct Span {
    /// Span name
    name: String,
    /// Context
    context: SpanContext,
    /// Start time
    start: Instant,
    /// Attributes
    attributes: RwLock<Vec<(String, SpanValue)>>,
    /// Events
    events: RwLock<Vec<SpanEvent>>,
    /// Status
    status: RwLock<SpanStatus>,
    /// Finished?
    finished: RwLock<bool>,
}

impl Span {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            context: SpanContext::new(),
            start: Instant::now(),
            attributes: RwLock::new(Vec::new()),
            events: RwLock::new(Vec::new()),
            status: RwLock::new(SpanStatus::Unset),
            finished: RwLock::new(false),
        }
    }

    /// Create child span
    pub fn child(&self, name: &str) -> Self {
        Self {
            name: name.to_string(),
            context: self.context.child(),
            start: Instant::now(),
            attributes: RwLock::new(Vec::new()),
            events: RwLock::new(Vec::new()),
            status: RwLock::new(SpanStatus::Unset),
            finished: RwLock::new(false),
        }
    }

    /// Get name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get context
    pub fn context(&self) -> &SpanContext {
        &self.context
    }

    /// Set attribute
    pub fn set_attribute(&self, key: impl Into<String>, value: impl Into<SpanValue>) {
        self.attributes.write().push((key.into(), value.into()));
    }

    /// Add event
    pub fn add_event(&self, name: impl Into<String>) {
        self.events.write().push(SpanEvent {
            name: name.into(),
            timestamp: Instant::now(),
            attributes: Vec::new(),
        });
    }

    /// Record exception
    pub fn record_exception(&self, error: &dyn std::error::Error) {
        self.set_attribute("exception.type", std::any::type_name_of_val(error));
        self.set_attribute("exception.message", error.to_string());
        *self.status.write() = SpanStatus::Error(error.to_string());
    }

    /// Set status
    pub fn set_status(&self, status: SpanStatus) {
        *self.status.write() = status;
    }

    /// Mark as OK
    pub fn ok(&self) {
        *self.status.write() = SpanStatus::Ok;
    }

    /// Get elapsed duration
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    /// End the span
    pub fn end(&self) {
        *self.finished.write() = true;
        let duration = self.elapsed();
        tracing::debug!(
            span = %self.name,
            trace_id = %self.context.trace_id.to_hex(),
            duration_ms = duration.as_millis(),
            "span ended"
        );
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if !*self.finished.read() {
            self.end();
        }
    }
}

/// Span value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SpanValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<SpanValue>),
}

impl From<&str> for SpanValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<String> for SpanValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<i64> for SpanValue {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<f64> for SpanValue {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}

impl From<bool> for SpanValue {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

/// Span event
#[derive(Debug, Clone)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: Instant,
    pub attributes: Vec<(String, SpanValue)>,
}

/// Span status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanStatus {
    Unset,
    Ok,
    Error(String),
}
