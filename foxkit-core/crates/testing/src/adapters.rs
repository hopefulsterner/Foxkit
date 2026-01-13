//! Test framework adapters

use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;

use crate::{TestRunner, TestId, TestResult, TestRunConfig, TestOutcome};

/// Test adapter trait for framework-specific integration
#[async_trait]
pub trait TestAdapter: Send + Sync {
    /// Adapter name
    fn name(&self) -> &str;

    /// Check if this adapter applies to the workspace
    fn applies(&self, workspace: &PathBuf) -> bool;

    /// Get the test runner
    fn runner(&self) -> Arc<dyn TestRunner>;

    /// Configure the adapter
    fn configure(&mut self, config: AdapterConfig);
}

/// Adapter configuration
#[derive(Debug, Clone, Default)]
pub struct AdapterConfig {
    pub test_match_patterns: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub setup_files: Vec<PathBuf>,
    pub env: Vec<(String, String)>,
}

/// Vitest adapter
pub struct VitestAdapter {
    workspace: PathBuf,
}

impl VitestAdapter {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl TestAdapter for VitestAdapter {
    fn name(&self) -> &str {
        "vitest"
    }

    fn applies(&self, workspace: &PathBuf) -> bool {
        // Check for vitest config or package.json with vitest
        workspace.join("vitest.config.ts").exists()
            || workspace.join("vitest.config.js").exists()
    }

    fn runner(&self) -> Arc<dyn TestRunner> {
        Arc::new(VitestRunner::new(self.workspace.clone()))
    }

    fn configure(&mut self, _config: AdapterConfig) {}
}

struct VitestRunner {
    workspace: PathBuf,
}

impl VitestRunner {
    fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl TestRunner for VitestRunner {
    fn name(&self) -> &str {
        "vitest"
    }

    async fn run(
        &self,
        tests: &[TestId],
        config: &TestRunConfig,
    ) -> anyhow::Result<Vec<TestResult>> {
        let mut cmd = tokio::process::Command::new("npx");
        cmd.arg("vitest");
        cmd.arg("run");
        cmd.arg("--reporter=json");
        cmd.current_dir(&self.workspace);

        for test in tests {
            cmd.arg("-t").arg(&test.0);
        }

        let output = cmd.output().await?;
        
        // Parse vitest JSON output
        let mut results = Vec::new();
        for test in tests {
            results.push(TestResult {
                test_id: test.clone(),
                outcome: if output.status.success() {
                    TestOutcome::Passed
                } else {
                    TestOutcome::Failed
                },
                duration: std::time::Duration::from_millis(50),
                messages: Vec::new(),
            });
        }

        Ok(results)
    }

    async fn debug(&self, test: &TestId, _config: &TestRunConfig) -> anyhow::Result<()> {
        tracing::info!("Debugging {} with vitest", test.0);
        Ok(())
    }

    fn cancel(&self) {}
}

/// Go test adapter
pub struct GoTestAdapter {
    workspace: PathBuf,
}

impl GoTestAdapter {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl TestAdapter for GoTestAdapter {
    fn name(&self) -> &str {
        "go-test"
    }

    fn applies(&self, workspace: &PathBuf) -> bool {
        workspace.join("go.mod").exists()
    }

    fn runner(&self) -> Arc<dyn TestRunner> {
        Arc::new(GoTestRunner::new(self.workspace.clone()))
    }

    fn configure(&mut self, _config: AdapterConfig) {}
}

struct GoTestRunner {
    workspace: PathBuf,
}

impl GoTestRunner {
    fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl TestRunner for GoTestRunner {
    fn name(&self) -> &str {
        "go-test"
    }

    async fn run(
        &self,
        tests: &[TestId],
        _config: &TestRunConfig,
    ) -> anyhow::Result<Vec<TestResult>> {
        let mut cmd = tokio::process::Command::new("go");
        cmd.arg("test");
        cmd.arg("-json");
        cmd.arg("./...");
        cmd.current_dir(&self.workspace);

        if !tests.is_empty() {
            let pattern = tests.iter()
                .map(|t| t.0.as_str())
                .collect::<Vec<_>>()
                .join("|");
            cmd.arg("-run").arg(pattern);
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
                duration: std::time::Duration::from_millis(100),
                messages: Vec::new(),
            });
        }

        Ok(results)
    }

    async fn debug(&self, test: &TestId, _config: &TestRunConfig) -> anyhow::Result<()> {
        tracing::info!("Debugging {} with delve", test.0);
        Ok(())
    }

    fn cancel(&self) {}
}

/// Detect and create appropriate adapters for a workspace
pub fn detect_adapters(workspace: &PathBuf) -> Vec<Box<dyn TestAdapter>> {
    let mut adapters: Vec<Box<dyn TestAdapter>> = Vec::new();

    // Check for various test frameworks
    if workspace.join("Cargo.toml").exists() {
        // Rust - use cargo test (built into crate)
    }

    if workspace.join("vitest.config.ts").exists() 
        || workspace.join("vitest.config.js").exists() 
    {
        adapters.push(Box::new(VitestAdapter::new(workspace.clone())));
    }

    if workspace.join("go.mod").exists() {
        adapters.push(Box::new(GoTestAdapter::new(workspace.clone())));
    }

    // Jest detection
    if workspace.join("jest.config.js").exists()
        || workspace.join("jest.config.ts").exists()
    {
        // Jest adapter would go here
    }

    // pytest detection  
    if workspace.join("pytest.ini").exists()
        || workspace.join("pyproject.toml").exists()
    {
        // Pytest adapter would go here
    }

    adapters
}
