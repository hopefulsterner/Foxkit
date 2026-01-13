//! Metrics collection

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;

use crate::TelemetryConfig;

/// Metrics collector
pub struct Metrics {
    /// Registered counters
    counters: RwLock<HashMap<String, Arc<Counter>>>,
    /// Registered gauges
    gauges: RwLock<HashMap<String, Arc<Gauge>>>,
    /// Registered histograms
    histograms: RwLock<HashMap<String, Arc<Histogram>>>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
        }
    }

    /// Initialize metrics subsystem
    pub fn init(&self, config: &TelemetryConfig) -> anyhow::Result<()> {
        // Would initialize Prometheus or other metrics backend
        tracing::debug!("Metrics initialized");
        Ok(())
    }

    /// Get or create a counter
    pub fn counter(&self, name: &str) -> Arc<Counter> {
        let mut counters = self.counters.write();
        counters
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Counter::new(name)))
            .clone()
    }

    /// Get or create a gauge
    pub fn gauge(&self, name: &str) -> Arc<Gauge> {
        let mut gauges = self.gauges.write();
        gauges
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Gauge::new(name)))
            .clone()
    }

    /// Get or create a histogram
    pub fn histogram(&self, name: &str) -> Arc<Histogram> {
        let mut histograms = self.histograms.write();
        histograms
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Histogram::new(name)))
            .clone()
    }

    /// Get all metrics as a snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        let mut data = HashMap::new();

        for (name, counter) in self.counters.read().iter() {
            data.insert(name.clone(), MetricValue::Counter(counter.get()));
        }

        for (name, gauge) in self.gauges.read().iter() {
            data.insert(name.clone(), MetricValue::Gauge(gauge.get()));
        }

        for (name, histogram) in self.histograms.read().iter() {
            data.insert(name.clone(), MetricValue::Histogram(histogram.snapshot()));
        }

        MetricsSnapshot { data }
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Counter metric (monotonically increasing)
pub struct Counter {
    name: String,
    value: std::sync::atomic::AtomicU64,
    labels: RwLock<HashMap<String, String>>,
}

impl Counter {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: std::sync::atomic::AtomicU64::new(0),
            labels: RwLock::new(HashMap::new()),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Increment by 1
    pub fn inc(&self) {
        self.add(1);
    }

    /// Add value
    pub fn add(&self, n: u64) {
        self.value.fetch_add(n, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get current value
    pub fn get(&self) -> u64 {
        self.value.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Add label
    pub fn with_label(&self, key: &str, value: &str) -> &Self {
        self.labels.write().insert(key.to_string(), value.to_string());
        self
    }
}

/// Gauge metric (can go up or down)
pub struct Gauge {
    name: String,
    value: std::sync::atomic::AtomicI64,
    labels: RwLock<HashMap<String, String>>,
}

impl Gauge {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: std::sync::atomic::AtomicI64::new(0),
            labels: RwLock::new(HashMap::new()),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set value
    pub fn set(&self, v: i64) {
        self.value.store(v, std::sync::atomic::Ordering::Relaxed);
    }

    /// Increment
    pub fn inc(&self) {
        self.add(1);
    }

    /// Decrement
    pub fn dec(&self) {
        self.sub(1);
    }

    /// Add value
    pub fn add(&self, n: i64) {
        self.value.fetch_add(n, std::sync::atomic::Ordering::Relaxed);
    }

    /// Subtract value
    pub fn sub(&self, n: i64) {
        self.value.fetch_sub(n, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get current value
    pub fn get(&self) -> i64 {
        self.value.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Histogram metric (distribution of values)
pub struct Histogram {
    name: String,
    values: RwLock<Vec<f64>>,
    sum: std::sync::atomic::AtomicU64,
    count: std::sync::atomic::AtomicU64,
    buckets: Vec<f64>,
}

impl Histogram {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            values: RwLock::new(Vec::new()),
            sum: std::sync::atomic::AtomicU64::new(0),
            count: std::sync::atomic::AtomicU64::new(0),
            buckets: vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Record a value
    pub fn observe(&self, v: f64) {
        self.values.write().push(v);
        self.sum.fetch_add(v.to_bits(), std::sync::atomic::Ordering::Relaxed);
        self.count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record duration in seconds
    pub fn observe_duration(&self, start: std::time::Instant) {
        let duration = start.elapsed().as_secs_f64();
        self.observe(duration);
    }

    /// Get snapshot
    pub fn snapshot(&self) -> HistogramSnapshot {
        let values = self.values.read();
        let count = values.len();
        
        if count == 0 {
            return HistogramSnapshot {
                count: 0,
                sum: 0.0,
                min: 0.0,
                max: 0.0,
                mean: 0.0,
                p50: 0.0,
                p95: 0.0,
                p99: 0.0,
            };
        }

        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let sum: f64 = sorted.iter().sum();

        HistogramSnapshot {
            count: count as u64,
            sum,
            min: sorted[0],
            max: sorted[count - 1],
            mean: sum / count as f64,
            p50: sorted[count / 2],
            p95: sorted[(count as f64 * 0.95) as usize],
            p99: sorted[(count as f64 * 0.99) as usize],
        }
    }
}

/// Histogram snapshot
#[derive(Debug, Clone)]
pub struct HistogramSnapshot {
    pub count: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

/// Metrics snapshot
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub data: HashMap<String, MetricValue>,
}

/// Metric value
#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(u64),
    Gauge(i64),
    Histogram(HistogramSnapshot),
}

/// Timer helper for histograms
pub struct Timer<'a> {
    histogram: &'a Histogram,
    start: std::time::Instant,
}

impl<'a> Timer<'a> {
    pub fn new(histogram: &'a Histogram) -> Self {
        Self {
            histogram,
            start: std::time::Instant::now(),
        }
    }
}

impl<'a> Drop for Timer<'a> {
    fn drop(&mut self) {
        self.histogram.observe_duration(self.start);
    }
}
