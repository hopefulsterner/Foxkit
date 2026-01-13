//! Debug console

use std::collections::VecDeque;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{DebugView, DebugViewId};

/// Debug console
pub struct DebugConsole {
    /// Console output
    output: RwLock<VecDeque<ConsoleMessage>>,
    /// Maximum messages to keep
    max_messages: usize,
    /// Input history
    history: RwLock<Vec<String>>,
    /// Visibility
    visible: bool,
}

impl DebugConsole {
    pub fn new() -> Self {
        Self {
            output: RwLock::new(VecDeque::new()),
            max_messages: 10000,
            history: RwLock::new(Vec::new()),
            visible: true,
        }
    }

    /// Append message
    pub fn append(&self, message: ConsoleMessage) {
        let mut output = self.output.write();
        
        if output.len() >= self.max_messages {
            output.pop_front();
        }
        
        output.push_back(message);
    }

    /// Append text output
    pub fn append_text(&self, text: &str, category: OutputCategory) {
        self.append(ConsoleMessage {
            content: text.to_string(),
            category,
            source: None,
            line: None,
            timestamp: std::time::SystemTime::now(),
        });
    }

    /// Append evaluation result
    pub fn append_evaluation(&self, expression: &str, result: &str) {
        self.append_text(&format!("> {}", expression), OutputCategory::Input);
        self.append_text(result, OutputCategory::Output);
    }

    /// Get all messages
    pub fn messages(&self) -> Vec<ConsoleMessage> {
        self.output.read().iter().cloned().collect()
    }

    /// Clear console
    pub fn clear_output(&self) {
        self.output.write().clear();
    }

    /// Add to input history
    pub fn add_history(&self, input: String) {
        let mut history = self.history.write();
        // Don't add duplicates
        if history.last() != Some(&input) {
            history.push(input);
        }
    }

    /// Get input history
    pub fn history(&self) -> Vec<String> {
        self.history.read().clone()
    }
}

impl Default for DebugConsole {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugView for DebugConsole {
    fn id(&self) -> DebugViewId {
        DebugViewId::Console
    }

    fn title(&self) -> &str {
        "Debug Console"
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
        // No refresh needed
    }

    fn clear(&mut self) {
        self.clear_output();
    }
}

/// Console message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleMessage {
    /// Message content
    pub content: String,
    /// Message category
    pub category: OutputCategory,
    /// Source file
    pub source: Option<String>,
    /// Line number
    pub line: Option<u32>,
    /// Timestamp
    #[serde(with = "timestamp_serde")]
    pub timestamp: std::time::SystemTime,
}

mod timestamp_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u128::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_millis(millis as u64))
    }
}

/// Output category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputCategory {
    Console,
    Stdout,
    Stderr,
    Output,
    Input,
    Important,
}

impl OutputCategory {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Console => "console",
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
            Self::Output => "output",
            Self::Input => "input",
            Self::Important => "important",
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Stderr)
    }
}

/// REPL state
pub struct ReplState {
    /// Current input
    pub input: String,
    /// Cursor position
    pub cursor: usize,
    /// History index
    pub history_index: Option<usize>,
    /// Completion suggestions
    pub suggestions: Vec<String>,
}

impl ReplState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            history_index: None,
            suggestions: Vec::new(),
        }
    }

    /// Insert character at cursor
    pub fn insert(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += 1;
    }

    /// Delete character before cursor
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.input.remove(self.cursor);
        }
    }

    /// Clear input
    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor = 0;
    }

    /// Submit input and return it
    pub fn submit(&mut self) -> String {
        let input = std::mem::take(&mut self.input);
        self.cursor = 0;
        self.history_index = None;
        input
    }
}

impl Default for ReplState {
    fn default() -> Self {
        Self::new()
    }
}
