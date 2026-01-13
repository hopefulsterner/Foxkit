//! Test results

use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::TestId;

/// Test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Test ID
    pub test_id: TestId,
    /// Outcome
    pub outcome: TestOutcome,
    /// Duration
    pub duration: Duration,
    /// Messages (errors, output, etc.)
    pub messages: Vec<TestMessage>,
}

impl TestResult {
    /// Create a passed result
    pub fn passed(test_id: TestId, duration: Duration) -> Self {
        Self {
            test_id,
            outcome: TestOutcome::Passed,
            duration,
            messages: Vec::new(),
        }
    }

    /// Create a failed result
    pub fn failed(test_id: TestId, duration: Duration, message: impl Into<String>) -> Self {
        Self {
            test_id,
            outcome: TestOutcome::Failed,
            duration,
            messages: vec![TestMessage::error(message)],
        }
    }

    /// Create a skipped result
    pub fn skipped(test_id: TestId) -> Self {
        Self {
            test_id,
            outcome: TestOutcome::Skipped,
            duration: Duration::ZERO,
            messages: Vec::new(),
        }
    }

    /// Add a message
    pub fn with_message(mut self, message: TestMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Is this result a failure?
    pub fn is_failure(&self) -> bool {
        matches!(self.outcome, TestOutcome::Failed | TestOutcome::Errored)
    }
}

/// Test outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestOutcome {
    /// Test passed
    Passed,
    /// Test failed (assertion)
    Failed,
    /// Test was skipped
    Skipped,
    /// Test errored (exception/panic)
    Errored,
}

impl TestOutcome {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Passed => "✓",
            Self::Failed => "✗",
            Self::Skipped => "○",
            Self::Errored => "⚠",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::Passed => "green",
            Self::Failed => "red",
            Self::Skipped => "yellow",
            Self::Errored => "red",
        }
    }
}

/// Test message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMessage {
    /// Message kind
    pub kind: TestMessageKind,
    /// Message text
    pub text: String,
    /// Location (if applicable)
    pub location: Option<TestLocation>,
    /// Expected value (for comparison failures)
    pub expected: Option<String>,
    /// Actual value (for comparison failures)
    pub actual: Option<String>,
}

impl TestMessage {
    /// Create an error message
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            kind: TestMessageKind::Error,
            text: text.into(),
            location: None,
            expected: None,
            actual: None,
        }
    }

    /// Create an output message
    pub fn output(text: impl Into<String>) -> Self {
        Self {
            kind: TestMessageKind::Output,
            text: text.into(),
            location: None,
            expected: None,
            actual: None,
        }
    }

    /// Create a comparison failure
    pub fn comparison(
        text: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self {
            kind: TestMessageKind::Comparison,
            text: text.into(),
            location: None,
            expected: Some(expected.into()),
            actual: Some(actual.into()),
        }
    }

    /// Add location
    pub fn at(mut self, file: impl Into<String>, line: u32, column: Option<u32>) -> Self {
        self.location = Some(TestLocation {
            file: file.into(),
            line,
            column,
        });
        self
    }
}

/// Test message kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TestMessageKind {
    /// Error/failure message
    Error,
    /// Standard output
    Output,
    /// Comparison failure (expected vs actual)
    Comparison,
    /// Warning
    Warning,
    /// Info
    Info,
}

/// Test location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestLocation {
    pub file: String,
    pub line: u32,
    pub column: Option<u32>,
}

/// Test diff for comparison failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDiff {
    pub lines: Vec<DiffLine>,
}

impl TestDiff {
    pub fn compute(expected: &str, actual: &str) -> Self {
        let mut lines = Vec::new();
        
        let expected_lines: Vec<_> = expected.lines().collect();
        let actual_lines: Vec<_> = actual.lines().collect();
        
        // Simple line-by-line diff
        let max_len = expected_lines.len().max(actual_lines.len());
        
        for i in 0..max_len {
            let exp = expected_lines.get(i);
            let act = actual_lines.get(i);
            
            match (exp, act) {
                (Some(e), Some(a)) if e == a => {
                    lines.push(DiffLine::Same(e.to_string()));
                }
                (Some(e), Some(a)) => {
                    lines.push(DiffLine::Removed(e.to_string()));
                    lines.push(DiffLine::Added(a.to_string()));
                }
                (Some(e), None) => {
                    lines.push(DiffLine::Removed(e.to_string()));
                }
                (None, Some(a)) => {
                    lines.push(DiffLine::Added(a.to_string()));
                }
                (None, None) => {}
            }
        }
        
        Self { lines }
    }
}

/// Diff line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffLine {
    Same(String),
    Added(String),
    Removed(String),
}

impl DiffLine {
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Same(_) => " ",
            Self::Added(_) => "+",
            Self::Removed(_) => "-",
        }
    }

    pub fn text(&self) -> &str {
        match self {
            Self::Same(t) | Self::Added(t) | Self::Removed(t) => t,
        }
    }
}
