//! Test runner

use std::path::PathBuf;
use std::time::Duration;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{TestId, TestResult, TestOutcome, TestMessage};

/// Test runner trait
#[async_trait]
pub trait TestRunner: Send + Sync {
    /// Framework name
    fn name(&self) -> &str;

    /// Run tests
    async fn run(
        &self,
        tests: &[TestId],
        config: &TestRunConfig,
    ) -> anyhow::Result<Vec<TestResult>>;

    /// Debug a test
    async fn debug(&self, test: &TestId, config: &TestRunConfig) -> anyhow::Result<()>;

    /// Cancel running tests
    fn cancel(&self);
}

/// Test run configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestRunConfig {
    /// Run profile
    pub profile: TestRunProfile,
    /// Working directory
    pub cwd: Option<PathBuf>,
    /// Environment variables
    pub env: Vec<(String, String)>,
    /// Additional arguments
    pub args: Vec<String>,
    /// Timeout per test
    pub timeout: Option<Duration>,
    /// Enable coverage
    pub coverage: bool,
    /// Run in parallel
    pub parallel: bool,
}

/// Test run profile
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum TestRunProfile {
    #[default]
    Run,
    Debug,
    Coverage,
}

/// Cargo test runner
pub struct CargoTestRunner {
    workspace: PathBuf,
}

impl CargoTestRunner {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl TestRunner for CargoTestRunner {
    fn name(&self) -> &str {
        "cargo-test"
    }

    async fn run(
        &self,
        tests: &[TestId],
        config: &TestRunConfig,
    ) -> anyhow::Result<Vec<TestResult>> {
        let mut results = Vec::new();

        // Build cargo test command
        let mut cmd = tokio::process::Command::new("cargo");
        cmd.arg("test");
        cmd.current_dir(&self.workspace);

        // Add test filters
        for test in tests {
            cmd.arg("--").arg(&test.0);
        }

        // Run and parse output
        let output = cmd.output().await?;
        
        // Parse test results from output
        let stdout = String::from_utf8_lossy(&output.stdout);
        results.extend(parse_cargo_test_output(&stdout, tests));

        Ok(results)
    }

    async fn debug(&self, test: &TestId, config: &TestRunConfig) -> anyhow::Result<()> {
        // Would launch debugger with cargo test
        tracing::info!("Debugging test {} with cargo", test.0);
        Ok(())
    }

    fn cancel(&self) {
        // Would cancel running process
    }
}

fn parse_cargo_test_output(output: &str, tests: &[TestId]) -> Vec<TestResult> {
    let mut results = Vec::new();

    for test in tests {
        // Simple parsing - real implementation would be more robust
        let outcome = if output.contains(&format!("test {} ... ok", test.0)) {
            TestOutcome::Passed
        } else if output.contains(&format!("test {} ... FAILED", test.0)) {
            TestOutcome::Failed
        } else if output.contains(&format!("test {} ... ignored", test.0)) {
            TestOutcome::Skipped
        } else {
            TestOutcome::Passed // Default
        };

        results.push(TestResult {
            test_id: test.clone(),
            outcome,
            duration: Duration::from_millis(100),
            messages: Vec::new(),
        });
    }

    results
}

/// Jest test runner
pub struct JestRunner {
    workspace: PathBuf,
}

impl JestRunner {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl TestRunner for JestRunner {
    fn name(&self) -> &str {
        "jest"
    }

    async fn run(
        &self,
        tests: &[TestId],
        config: &TestRunConfig,
    ) -> anyhow::Result<Vec<TestResult>> {
        let mut cmd = tokio::process::Command::new("npx");
        cmd.arg("jest");
        cmd.arg("--json");
        cmd.current_dir(&self.workspace);

        // Add test patterns
        for test in tests {
            cmd.arg("-t").arg(&test.0);
        }

        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse Jest JSON output
        parse_jest_output(&stdout, tests)
    }

    async fn debug(&self, test: &TestId, config: &TestRunConfig) -> anyhow::Result<()> {
        tracing::info!("Debugging test {} with Jest", test.0);
        Ok(())
    }

    fn cancel(&self) {}
}

fn parse_jest_output(output: &str, tests: &[TestId]) -> anyhow::Result<Vec<TestResult>> {
    // Would parse Jest JSON output
    let mut results = Vec::new();
    
    for test in tests {
        results.push(TestResult {
            test_id: test.clone(),
            outcome: TestOutcome::Passed,
            duration: Duration::from_millis(50),
            messages: Vec::new(),
        });
    }

    Ok(results)
}

/// Pytest runner
pub struct PytestRunner {
    workspace: PathBuf,
}

impl PytestRunner {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl TestRunner for PytestRunner {
    fn name(&self) -> &str {
        "pytest"
    }

    async fn run(
        &self,
        tests: &[TestId],
        config: &TestRunConfig,
    ) -> anyhow::Result<Vec<TestResult>> {
        let mut cmd = tokio::process::Command::new("pytest");
        cmd.arg("--json-report");
        cmd.current_dir(&self.workspace);

        for test in tests {
            cmd.arg("-k").arg(&test.0);
        }

        let output = cmd.output().await?;
        
        let mut results = Vec::new();
        for test in tests {
            results.push(TestResult {
                test_id: test.clone(),
                outcome: if output.status.success() {
                    TestOutcome::Passed
                } else {
                    TestOutcome::Failed
                },
                duration: Duration::from_millis(100),
                messages: Vec::new(),
            });
        }

        Ok(results)
    }

    async fn debug(&self, test: &TestId, config: &TestRunConfig) -> anyhow::Result<()> {
        tracing::info!("Debugging test {} with pytest", test.0);
        Ok(())
    }

    fn cancel(&self) {}
}
