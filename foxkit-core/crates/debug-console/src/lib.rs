//! # Foxkit Debug Console
//!
//! Debug REPL console and expression evaluation.

use std::collections::VecDeque;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};

/// Debug console service
pub struct DebugConsoleService {
    /// Console entries
    entries: RwLock<VecDeque<ConsoleEntry>>,
    /// Max entries
    max_entries: usize,
    /// Command history
    history: RwLock<VecDeque<String>>,
    /// History position
    history_pos: RwLock<Option<usize>>,
    /// Current input
    current_input: RwLock<String>,
    /// Evaluation sender
    eval_tx: mpsc::Sender<EvalRequest>,
    /// Evaluation receiver (stored for handing off)
    eval_rx: RwLock<Option<mpsc::Receiver<EvalRequest>>>,
    /// Event sender
    event_tx: broadcast::Sender<ConsoleEvent>,
    /// Filter
    filter: RwLock<ConsoleFilter>,
}

impl DebugConsoleService {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let (eval_tx, eval_rx) = mpsc::channel(64);

        Self {
            entries: RwLock::new(VecDeque::new()),
            max_entries: 10000,
            history: RwLock::new(VecDeque::new()),
            history_pos: RwLock::new(None),
            current_input: RwLock::new(String::new()),
            eval_tx,
            eval_rx: RwLock::new(Some(eval_rx)),
            event_tx,
            filter: RwLock::new(ConsoleFilter::default()),
        }
    }

    /// Take eval receiver (for debugger integration)
    pub fn take_eval_receiver(&self) -> Option<mpsc::Receiver<EvalRequest>> {
        self.eval_rx.write().take()
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<ConsoleEvent> {
        self.event_tx.subscribe()
    }

    /// Add entry
    pub fn add_entry(&self, entry: ConsoleEntry) {
        let mut entries = self.entries.write();
        
        // Trim if needed
        while entries.len() >= self.max_entries {
            entries.pop_front();
        }

        entries.push_back(entry.clone());
        let _ = self.event_tx.send(ConsoleEvent::EntryAdded(entry));
    }

    /// Log output
    pub fn log(&self, category: impl Into<String>, message: impl Into<String>) {
        self.add_entry(ConsoleEntry {
            timestamp: Utc::now(),
            kind: EntryKind::Output,
            category: Some(category.into()),
            content: message.into(),
            variables: None,
            source: None,
        });
    }

    /// Log info
    pub fn info(&self, message: impl Into<String>) {
        self.add_entry(ConsoleEntry {
            timestamp: Utc::now(),
            kind: EntryKind::Info,
            category: None,
            content: message.into(),
            variables: None,
            source: None,
        });
    }

    /// Log warning
    pub fn warn(&self, message: impl Into<String>) {
        self.add_entry(ConsoleEntry {
            timestamp: Utc::now(),
            kind: EntryKind::Warning,
            category: None,
            content: message.into(),
            variables: None,
            source: None,
        });
    }

    /// Log error
    pub fn error(&self, message: impl Into<String>) {
        self.add_entry(ConsoleEntry {
            timestamp: Utc::now(),
            kind: EntryKind::Error,
            category: None,
            content: message.into(),
            variables: None,
            source: None,
        });
    }

    /// Evaluate expression
    pub async fn evaluate(&self, expression: impl Into<String>) -> Result<(), mpsc::error::SendError<EvalRequest>> {
        let expr = expression.into();
        
        // Add to history
        let mut history = self.history.write();
        if history.back() != Some(&expr) {
            history.push_back(expr.clone());
            if history.len() > 100 {
                history.pop_front();
            }
        }
        *self.history_pos.write() = None;

        // Add input entry
        self.add_entry(ConsoleEntry {
            timestamp: Utc::now(),
            kind: EntryKind::Input,
            category: None,
            content: expr.clone(),
            variables: None,
            source: None,
        });

        // Send for evaluation
        self.eval_tx.send(EvalRequest {
            expression: expr,
            context: EvalContext::default(),
        }).await
    }

    /// Handle evaluation result
    pub fn handle_eval_result(&self, result: EvalResult) {
        let entry = match result {
            EvalResult::Success { value, type_name, variables } => {
                ConsoleEntry {
                    timestamp: Utc::now(),
                    kind: EntryKind::Result,
                    category: type_name,
                    content: value,
                    variables,
                    source: None,
                }
            }
            EvalResult::Error { message } => {
                ConsoleEntry {
                    timestamp: Utc::now(),
                    kind: EntryKind::Error,
                    category: None,
                    content: message,
                    variables: None,
                    source: None,
                }
            }
        };

        self.add_entry(entry);
    }

    /// Navigate history up
    pub fn history_up(&self) -> Option<String> {
        let history = self.history.read();
        let mut pos = self.history_pos.write();

        if history.is_empty() {
            return None;
        }

        let new_pos = match *pos {
            None => history.len() - 1,
            Some(0) => 0,
            Some(p) => p - 1,
        };

        *pos = Some(new_pos);
        history.get(new_pos).cloned()
    }

    /// Navigate history down
    pub fn history_down(&self) -> Option<String> {
        let history = self.history.read();
        let mut pos = self.history_pos.write();

        match *pos {
            None => None,
            Some(p) if p >= history.len() - 1 => {
                *pos = None;
                Some(self.current_input.read().clone())
            }
            Some(p) => {
                *pos = Some(p + 1);
                history.get(p + 1).cloned()
            }
        }
    }

    /// Set current input
    pub fn set_input(&self, input: impl Into<String>) {
        *self.current_input.write() = input.into();
    }

    /// Get entries
    pub fn entries(&self) -> Vec<ConsoleEntry> {
        self.entries.read().iter().cloned().collect()
    }

    /// Get filtered entries
    pub fn filtered_entries(&self) -> Vec<ConsoleEntry> {
        let filter = self.filter.read();
        let entries = self.entries.read();

        entries.iter()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect()
    }

    /// Clear console
    pub fn clear(&self) {
        self.entries.write().clear();
        let _ = self.event_tx.send(ConsoleEvent::Cleared);
    }

    /// Set filter
    pub fn set_filter(&self, filter: ConsoleFilter) {
        *self.filter.write() = filter;
        let _ = self.event_tx.send(ConsoleEvent::FilterChanged);
    }

    /// Get filter
    pub fn filter(&self) -> ConsoleFilter {
        self.filter.read().clone()
    }

    /// Get history
    pub fn history(&self) -> Vec<String> {
        self.history.read().iter().cloned().collect()
    }

    /// Entry count
    pub fn entry_count(&self) -> usize {
        self.entries.read().len()
    }
}

impl Default for DebugConsoleService {
    fn default() -> Self {
        Self::new()
    }
}

/// Console entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleEntry {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Entry kind
    pub kind: EntryKind,
    /// Category/type
    pub category: Option<String>,
    /// Content
    pub content: String,
    /// Expandable variables
    pub variables: Option<Vec<Variable>>,
    /// Source location
    pub source: Option<SourceLocation>,
}

impl ConsoleEntry {
    pub fn format_timestamp(&self) -> String {
        self.timestamp.format("%H:%M:%S%.3f").to_string()
    }

    pub fn icon(&self) -> &'static str {
        match self.kind {
            EntryKind::Input => "chevron-right",
            EntryKind::Output => "output",
            EntryKind::Result => "symbol-variable",
            EntryKind::Info => "info",
            EntryKind::Warning => "warning",
            EntryKind::Error => "error",
            EntryKind::StartGroup => "chevron-down",
            EntryKind::EndGroup => "chevron-up",
        }
    }
}

/// Entry kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    /// User input
    Input,
    /// Program output
    Output,
    /// Evaluation result
    Result,
    /// Info message
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Start group
    StartGroup,
    /// End group
    EndGroup,
}

/// Variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// Name
    pub name: String,
    /// Value
    pub value: String,
    /// Type
    pub type_name: Option<String>,
    /// Can expand
    pub has_children: bool,
    /// Children (lazy loaded)
    pub children: Option<Vec<Variable>>,
    /// Variables reference (for DAP)
    pub variables_reference: u64,
}

/// Source location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub path: String,
    pub line: u32,
    pub column: Option<u32>,
}

/// Evaluation request
#[derive(Debug, Clone)]
pub struct EvalRequest {
    pub expression: String,
    pub context: EvalContext,
}

/// Evaluation context
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    /// Frame ID
    pub frame_id: Option<u64>,
    /// Context type
    pub context: EvalContextType,
}

/// Context type
#[derive(Debug, Clone, Copy, Default)]
pub enum EvalContextType {
    #[default]
    Repl,
    Watch,
    Hover,
    Clipboard,
}

/// Evaluation result
#[derive(Debug, Clone)]
pub enum EvalResult {
    Success {
        value: String,
        type_name: Option<String>,
        variables: Option<Vec<Variable>>,
    },
    Error {
        message: String,
    },
}

/// Console filter
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsoleFilter {
    /// Text filter
    pub text: Option<String>,
    /// Show input
    pub show_input: bool,
    /// Show output
    pub show_output: bool,
    /// Show info
    pub show_info: bool,
    /// Show warnings
    pub show_warnings: bool,
    /// Show errors
    pub show_errors: bool,
}

impl ConsoleFilter {
    pub fn all() -> Self {
        Self {
            text: None,
            show_input: true,
            show_output: true,
            show_info: true,
            show_warnings: true,
            show_errors: true,
        }
    }

    pub fn matches(&self, entry: &ConsoleEntry) -> bool {
        // Check kind
        let kind_match = match entry.kind {
            EntryKind::Input => self.show_input,
            EntryKind::Output | EntryKind::Result => self.show_output,
            EntryKind::Info | EntryKind::StartGroup | EntryKind::EndGroup => self.show_info,
            EntryKind::Warning => self.show_warnings,
            EntryKind::Error => self.show_errors,
        };

        if !kind_match {
            return false;
        }

        // Check text filter
        if let Some(ref text) = self.text {
            if !entry.content.to_lowercase().contains(&text.to_lowercase()) {
                return false;
            }
        }

        true
    }
}

/// Console event
#[derive(Debug, Clone)]
pub enum ConsoleEvent {
    EntryAdded(ConsoleEntry),
    Cleared,
    FilterChanged,
}
