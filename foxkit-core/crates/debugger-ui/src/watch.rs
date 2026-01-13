//! Watch expressions view

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{DebugView, DebugViewId};
use crate::variables::{Variable, VariableReference};

/// Watch view
pub struct WatchView {
    /// Watch expressions
    expressions: RwLock<Vec<WatchExpression>>,
    /// Selected expression
    selected: RwLock<Option<usize>>,
    /// Visibility
    visible: bool,
}

impl WatchView {
    pub fn new() -> Self {
        Self {
            expressions: RwLock::new(Vec::new()),
            selected: RwLock::new(None),
            visible: true,
        }
    }

    /// Add watch expression
    pub fn add(&self, expression: String) {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        
        self.expressions.write().push(WatchExpression {
            id: COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            expression,
            result: None,
            error: None,
        });
    }

    /// Remove watch expression
    pub fn remove(&self, id: u64) {
        self.expressions.write().retain(|e| e.id != id);
    }

    /// Edit watch expression
    pub fn edit(&self, id: u64, new_expression: String) {
        let mut exprs = self.expressions.write();
        if let Some(expr) = exprs.iter_mut().find(|e| e.id == id) {
            expr.expression = new_expression;
            expr.result = None;
            expr.error = None;
        }
    }

    /// Get all expressions
    pub fn expressions(&self) -> Vec<WatchExpression> {
        self.expressions.read().clone()
    }

    /// Update expression result
    pub fn set_result(&self, id: u64, result: EvaluateResult) {
        let mut exprs = self.expressions.write();
        if let Some(expr) = exprs.iter_mut().find(|e| e.id == id) {
            match result {
                EvaluateResult::Value(v) => {
                    expr.result = Some(v);
                    expr.error = None;
                }
                EvaluateResult::Error(e) => {
                    expr.result = None;
                    expr.error = Some(e);
                }
            }
        }
    }

    /// Select expression
    pub fn select(&self, index: usize) {
        *self.selected.write() = Some(index);
    }

    /// Clear all expressions
    pub fn clear_all(&self) {
        self.expressions.write().clear();
    }
}

impl Default for WatchView {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugView for WatchView {
    fn id(&self) -> DebugViewId {
        DebugViewId::Watch
    }

    fn title(&self) -> &str {
        "Watch"
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn refresh(&mut self) {
        // Re-evaluate all expressions
    }

    fn clear(&mut self) {
        // Clear results but keep expressions
        for expr in self.expressions.write().iter_mut() {
            expr.result = None;
            expr.error = None;
        }
    }
}

/// Watch expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchExpression {
    /// Unique ID
    pub id: u64,
    /// Expression text
    pub expression: String,
    /// Evaluated result
    pub result: Option<WatchResult>,
    /// Error message
    pub error: Option<String>,
}

/// Watch result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchResult {
    /// Result value
    pub value: String,
    /// Result type
    pub result_type: Option<String>,
    /// Variables reference (for expandable)
    pub variables_reference: VariableReference,
    /// Named variables
    pub named_variables: Option<i64>,
    /// Indexed variables
    pub indexed_variables: Option<i64>,
    /// Memory reference
    pub memory_reference: Option<String>,
}

/// Evaluate result
pub enum EvaluateResult {
    Value(WatchResult),
    Error(String),
}

/// Inline value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineValue {
    /// Variable name
    pub name: String,
    /// Value
    pub value: String,
    /// Line number
    pub line: u32,
    /// Column
    pub column: u32,
    /// End column
    pub end_column: u32,
}

/// Hover evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverResult {
    /// Expression evaluated
    pub expression: String,
    /// Result
    pub result: WatchResult,
}
