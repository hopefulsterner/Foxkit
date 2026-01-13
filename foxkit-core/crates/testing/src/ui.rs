//! Test UI components

use serde::{Deserialize, Serialize};
use crate::{TestId, TestItem, TestResult, TestOutcome, TestRunResult};

/// Test explorer view model
#[derive(Debug, Clone, Default)]
pub struct TestExplorerViewModel {
    /// Root test items
    pub items: Vec<TestTreeItem>,
    /// Currently selected item
    pub selected: Option<TestId>,
    /// Filter text
    pub filter: String,
    /// Show only failed tests
    pub show_failed_only: bool,
    /// Expanded items
    pub expanded: Vec<TestId>,
}

impl TestExplorerViewModel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update from discovered tests
    pub fn update_tests(&mut self, items: Vec<TestItem>) {
        self.items = items.into_iter().map(TestTreeItem::from).collect();
    }

    /// Update result for a test
    pub fn update_result(&mut self, result: &TestResult) {
        fn update_item(items: &mut [TestTreeItem], result: &TestResult) {
            for item in items {
                if item.id == result.test_id {
                    item.result = Some(result.outcome);
                    item.duration = Some(result.duration);
                    return;
                }
                update_item(&mut item.children, result);
            }
        }
        
        update_item(&mut self.items, result);
    }

    /// Toggle item expansion
    pub fn toggle_expanded(&mut self, id: &TestId) {
        if let Some(pos) = self.expanded.iter().position(|i| i == id) {
            self.expanded.remove(pos);
        } else {
            self.expanded.push(id.clone());
        }
    }

    /// Is item expanded?
    pub fn is_expanded(&self, id: &TestId) -> bool {
        self.expanded.contains(id)
    }

    /// Get filtered items
    pub fn filtered_items(&self) -> Vec<&TestTreeItem> {
        self.items.iter()
            .filter(|item| self.matches_filter(item))
            .collect()
    }

    fn matches_filter(&self, item: &TestTreeItem) -> bool {
        if self.filter.is_empty() && !self.show_failed_only {
            return true;
        }

        let name_matches = self.filter.is_empty() 
            || item.label.to_lowercase().contains(&self.filter.to_lowercase());
        
        let outcome_matches = !self.show_failed_only 
            || matches!(item.result, Some(TestOutcome::Failed | TestOutcome::Errored));

        name_matches && outcome_matches
    }
}

/// Test tree item
#[derive(Debug, Clone)]
pub struct TestTreeItem {
    pub id: TestId,
    pub label: String,
    pub kind: TestTreeItemKind,
    pub children: Vec<TestTreeItem>,
    pub result: Option<TestOutcome>,
    pub duration: Option<std::time::Duration>,
}

impl From<TestItem> for TestTreeItem {
    fn from(item: TestItem) -> Self {
        Self {
            id: item.id,
            label: item.name,
            kind: match item.kind {
                crate::discovery::TestItemKind::Test => TestTreeItemKind::Test,
                crate::discovery::TestItemKind::Suite => TestTreeItemKind::Suite,
                crate::discovery::TestItemKind::Benchmark => TestTreeItemKind::Benchmark,
                crate::discovery::TestItemKind::DocTest => TestTreeItemKind::DocTest,
            },
            children: item.children.into_iter().map(TestTreeItem::from).collect(),
            result: None,
            duration: None,
        }
    }
}

/// Test tree item kind
#[derive(Debug, Clone, Copy)]
pub enum TestTreeItemKind {
    Suite,
    Test,
    Benchmark,
    DocTest,
}

impl TestTreeItemKind {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Suite => "📁",
            Self::Test => "🧪",
            Self::Benchmark => "⏱",
            Self::DocTest => "📝",
        }
    }
}

/// Test run progress
#[derive(Debug, Clone, Default)]
pub struct TestRunProgress {
    pub total: usize,
    pub completed: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub running: Option<TestId>,
}

impl TestRunProgress {
    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.completed as f64 / self.total as f64 * 100.0
        }
    }

    pub fn status_text(&self) -> String {
        if let Some(ref running) = self.running {
            format!("Running: {}", running.0)
        } else if self.completed == self.total {
            format!(
                "Completed: {} passed, {} failed, {} skipped",
                self.passed, self.failed, self.skipped
            )
        } else {
            format!("{}/{} tests", self.completed, self.total)
        }
    }
}

/// Test output panel
#[derive(Debug, Clone, Default)]
pub struct TestOutputPanel {
    pub entries: Vec<TestOutputEntry>,
    pub selected_test: Option<TestId>,
}

impl TestOutputPanel {
    pub fn add_output(&mut self, test_id: TestId, output: String) {
        self.entries.push(TestOutputEntry {
            test_id,
            kind: OutputKind::Stdout,
            text: output,
            timestamp: std::time::Instant::now(),
        });
    }

    pub fn add_error(&mut self, test_id: TestId, error: String) {
        self.entries.push(TestOutputEntry {
            test_id,
            kind: OutputKind::Stderr,
            text: error,
            timestamp: std::time::Instant::now(),
        });
    }

    pub fn get_for_test(&self, test_id: &TestId) -> Vec<&TestOutputEntry> {
        self.entries.iter()
            .filter(|e| &e.test_id == test_id)
            .collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Test output entry
#[derive(Debug, Clone)]
pub struct TestOutputEntry {
    pub test_id: TestId,
    pub kind: OutputKind,
    pub text: String,
    pub timestamp: std::time::Instant,
}

/// Output kind
#[derive(Debug, Clone, Copy)]
pub enum OutputKind {
    Stdout,
    Stderr,
}

/// Test result details panel
#[derive(Debug, Clone)]
pub struct TestResultDetails {
    pub test_id: TestId,
    pub result: TestResult,
    pub diff: Option<crate::results::TestDiff>,
}

impl TestResultDetails {
    pub fn from_result(result: TestResult) -> Self {
        let diff = result.messages.iter()
            .find_map(|m| {
                if let (Some(expected), Some(actual)) = (&m.expected, &m.actual) {
                    Some(crate::results::TestDiff::compute(expected, actual))
                } else {
                    None
                }
            });

        Self {
            test_id: result.test_id.clone(),
            result,
            diff,
        }
    }
}
