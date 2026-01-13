//! Telemetry reporter

use std::collections::VecDeque;
use std::sync::Arc;
use parking_lot::Mutex;
use async_trait::async_trait;

use crate::Event;

/// Telemetry reporter trait
#[async_trait]
pub trait TelemetryReporter: Send + Sync {
    /// Report an event
    fn report(&self, event: Event);

    /// Flush pending events
    async fn flush(&self);
}

/// Batch reporter with buffering
pub struct BatchReporter {
    /// Buffer of pending events
    buffer: Mutex<VecDeque<Event>>,
    /// Maximum buffer size
    max_buffer: usize,
    /// Endpoint URL
    endpoint: String,
}

impl BatchReporter {
    pub fn new(endpoint: String, max_buffer: usize) -> Self {
        Self {
            buffer: Mutex::new(VecDeque::new()),
            max_buffer,
            endpoint,
        }
    }
}

#[async_trait]
impl TelemetryReporter for BatchReporter {
    fn report(&self, event: Event) {
        let mut buffer = self.buffer.lock();
        
        // If buffer is full, drop oldest
        if buffer.len() >= self.max_buffer {
            buffer.pop_front();
        }
        
        buffer.push_back(event);
    }

    async fn flush(&self) {
        let events: Vec<Event> = {
            let mut buffer = self.buffer.lock();
            buffer.drain(..).collect()
        };

        if events.is_empty() {
            return;
        }

        // Send to endpoint
        match send_events(&self.endpoint, &events).await {
            Ok(_) => {
                tracing::debug!("Flushed {} telemetry events", events.len());
            }
            Err(e) => {
                tracing::warn!("Failed to flush telemetry: {}", e);
                // Re-add events to buffer
                let mut buffer = self.buffer.lock();
                for event in events.into_iter().rev() {
                    buffer.push_front(event);
                }
            }
        }
    }
}

/// Console reporter (for debugging)
pub struct ConsoleReporter;

#[async_trait]
impl TelemetryReporter for ConsoleReporter {
    fn report(&self, event: Event) {
        tracing::info!(
            event = %event.name,
            category = ?event.category,
            "telemetry event"
        );
    }

    async fn flush(&self) {
        // No-op for console
    }
}

/// Null reporter (telemetry disabled)
pub struct NullReporter;

#[async_trait]
impl TelemetryReporter for NullReporter {
    fn report(&self, _event: Event) {
        // Discard
    }

    async fn flush(&self) {
        // No-op
    }
}

/// Multi-reporter (send to multiple destinations)
pub struct MultiReporter {
    reporters: Vec<Arc<dyn TelemetryReporter>>,
}

impl MultiReporter {
    pub fn new(reporters: Vec<Arc<dyn TelemetryReporter>>) -> Self {
        Self { reporters }
    }
}

#[async_trait]
impl TelemetryReporter for MultiReporter {
    fn report(&self, event: Event) {
        for reporter in &self.reporters {
            reporter.report(event.clone());
        }
    }

    async fn flush(&self) {
        for reporter in &self.reporters {
            reporter.flush().await;
        }
    }
}

/// Send events to endpoint
async fn send_events(endpoint: &str, events: &[Event]) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    
    client
        .post(endpoint)
        .json(events)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

/// Sampling filter
pub struct SamplingFilter {
    rate: f64,
}

impl SamplingFilter {
    pub fn new(rate: f64) -> Self {
        Self {
            rate: rate.clamp(0.0, 1.0),
        }
    }

    /// Should sample this event?
    pub fn should_sample(&self) -> bool {
        if self.rate >= 1.0 {
            return true;
        }
        if self.rate <= 0.0 {
            return false;
        }

        // Simple random sampling
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        
        (nanos as f64 / u32::MAX as f64) < self.rate
    }
}
