//! # Foxkit Testing
//!
//! Test discovery, running, and UI integration.

pub mod discovery;
pub mod runner;
pub mod results;
pub mod adapters;
pub mod coverage;
pub mod ui;

use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

pub use discovery::{TestDiscovery, TestItem, TestItemKind};
pub use runner::{TestRunner, TestRunConfig, TestRunProfile};
pub use results::{TestResult, TestOutcome, TestMessage};

/// Test service
pub struct TestService {
    /// Test discovery
    discovery: Arc<TestDiscovery>,
    /// Test runners by framework
    runners: RwLock<HashMap<String, Arc<dyn TestRunner>>>,
    /// Test results
    results: RwLock<HashMap<TestId, TestResult>>,
    /// Event broadcast
    events: broadcast::Sender<TestEvent>,
    /// Configuration
    config: RwLock<TestConfig>,
}

impl TestService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(256);
        
        Self {
            discovery: Arc::new(TestDiscovery::new()),
            runners: RwLock::new(HashMap::new()),
            results: RwLock::new(HashMap::new()),
            events,
            config: RwLock::new(TestConfig::default()),
        }
    }

    /// Configure testing
    pub fn configure(&self, config: TestConfig) {
        *self.config.write() = config;
    }

    /// Register a test runner
    pub fn register_runner(&self, framework: &str, runner: Arc<dyn TestRunner>) {
        self.runners.write().insert(framework.to_string(), runner);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<TestEvent> {
        self.events.subscribe()
    }

    /// Discover tests in workspace
    pub async fn discover(&self, workspace: &PathBuf) -> anyhow::Result<Vec<TestItem>> {
        let _ = self.events.send(TestEvent::DiscoveryStarted);
        
        let items = self.discovery.discover(workspace).await?;
        
        let _ = self.events.send(TestEvent::DiscoveryCompleted {
            test_count: items.len(),
        });

        Ok(items)
    }

    /// Run tests
    pub async fn run(
        &self,
        tests: &[TestId],
        profile: TestRunProfile,
    ) -> anyhow::Result<TestRunResult> {
        let run_id = TestRunId::new();
        
        let _ = self.events.send(TestEvent::RunStarted {
            run_id: run_id.clone(),
            test_count: tests.len(),
        });

        let config = TestRunConfig {
            profile,
            ..Default::default()
        };

        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;

        for test_id in tests {
            let _ = self.events.send(TestEvent::TestStarted {
                run_id: run_id.clone(),
                test_id: test_id.clone(),
            });

            // Find appropriate runner
            // For now, simulate running
            let result = TestResult {
                test_id: test_id.clone(),
                outcome: TestOutcome::Passed,
                duration: std::time::Duration::from_millis(100),
                messages: Vec::new(),
            };

            match result.outcome {
                TestOutcome::Passed => passed += 1,
                TestOutcome::Failed => failed += 1,
                TestOutcome::Skipped => skipped += 1,
                TestOutcome::Errored => failed += 1,
            }

            self.results.write().insert(test_id.clone(), result.clone());

            let _ = self.events.send(TestEvent::TestCompleted {
                run_id: run_id.clone(),
                result: result.clone(),
            });
        }

        let run_result = TestRunResult {
            run_id: run_id.clone(),
            passed,
            failed,
            skipped,
            duration: std::time::Duration::from_secs(1),
        };

        let _ = self.events.send(TestEvent::RunCompleted {
            result: run_result.clone(),
        });

        Ok(run_result)
    }

    /// Run all tests
    pub async fn run_all(&self, workspace: &PathBuf, profile: TestRunProfile) -> anyhow::Result<TestRunResult> {
        let tests = self.discover(workspace).await?;
        let ids: Vec<_> = tests.iter().map(|t| t.id.clone()).collect();
        self.run(&ids, profile).await
    }

    /// Debug a test
    pub async fn debug(&self, test_id: &TestId) -> anyhow::Result<()> {
        let _ = self.events.send(TestEvent::DebugStarted {
            test_id: test_id.clone(),
        });
        
        // Would launch debugger
        tracing::info!("Debugging test: {:?}", test_id);
        
        Ok(())
    }

    /// Get test result
    pub fn get_result(&self, test_id: &TestId) -> Option<TestResult> {
        self.results.read().get(test_id).cloned()
    }

    /// Clear results
    pub fn clear_results(&self) {
        self.results.write().clear();
    }

    /// Cancel running tests
    pub fn cancel(&self) {
        let _ = self.events.send(TestEvent::RunCancelled);
    }
}

impl Default for TestService {
    fn default() -> Self {
        Self::new()
    }
}

/// Test ID
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestId(pub String);

impl TestId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Test run ID
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRunId(pub String);

impl TestRunId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for TestRunId {
    fn default() -> Self {
        Self::new()
    }
}

/// Test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Auto-discover tests on file changes
    pub auto_discover: bool,
    /// Run tests on save
    pub run_on_save: bool,
    /// Show inline test results
    pub show_inline_results: bool,
    /// Enable coverage collection
    pub enable_coverage: bool,
    /// Test timeout in seconds
    pub timeout_secs: u64,
    /// Parallel test execution
    pub parallel: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            auto_discover: true,
            run_on_save: false,
            show_inline_results: true,
            enable_coverage: false,
            timeout_secs: 30,
            parallel: true,
        }
    }
}

/// Test event
#[derive(Debug, Clone)]
pub enum TestEvent {
    DiscoveryStarted,
    DiscoveryCompleted { test_count: usize },
    RunStarted { run_id: TestRunId, test_count: usize },
    TestStarted { run_id: TestRunId, test_id: TestId },
    TestCompleted { run_id: TestRunId, result: TestResult },
    RunCompleted { result: TestRunResult },
    RunCancelled,
    DebugStarted { test_id: TestId },
}

/// Test run result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunResult {
    pub run_id: TestRunId,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration: std::time::Duration,
}
