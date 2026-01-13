//! Progress indicators

use serde::{Deserialize, Serialize};
use crate::new_id;

/// Progress location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgressLocation {
    /// Notification area
    Notification,
    /// Status bar
    StatusBar,
    /// Window title
    Window,
    /// Source control
    SourceControl,
    /// Explorer
    Explorer,
}

impl Default for ProgressLocation {
    fn default() -> Self {
        ProgressLocation::Notification
    }
}

/// Progress indicator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    /// Unique ID
    pub id: String,
    /// Title
    pub title: String,
    /// Current message
    pub message: Option<String>,
    /// Progress value (0.0 - 1.0, or None for indeterminate)
    pub value: f32,
    /// Is indeterminate
    pub indeterminate: bool,
    /// Is cancellable
    pub cancellable: bool,
    /// Location
    pub location: ProgressLocation,
    /// Total steps (for step-based progress)
    pub total: Option<u32>,
    /// Current step
    pub current: Option<u32>,
}

impl Progress {
    pub fn new(title: &str) -> Self {
        Self {
            id: new_id(),
            title: title.to_string(),
            message: None,
            value: 0.0,
            indeterminate: false,
            cancellable: false,
            location: ProgressLocation::Notification,
            total: None,
            current: None,
        }
    }

    pub fn indeterminate(title: &str) -> Self {
        Self {
            id: new_id(),
            title: title.to_string(),
            message: None,
            value: 0.0,
            indeterminate: true,
            cancellable: false,
            location: ProgressLocation::Notification,
            total: None,
            current: None,
        }
    }

    pub fn with_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_string());
        self
    }

    pub fn with_location(mut self, location: ProgressLocation) -> Self {
        self.location = location;
        self
    }

    pub fn cancellable(mut self) -> Self {
        self.cancellable = true;
        self
    }

    pub fn with_total(mut self, total: u32) -> Self {
        self.total = Some(total);
        self.current = Some(0);
        self
    }

    /// Get percentage (0-100)
    pub fn percentage(&self) -> u32 {
        (self.value * 100.0) as u32
    }

    /// Get step progress string (e.g., "3/10")
    pub fn step_progress(&self) -> Option<String> {
        match (self.current, self.total) {
            (Some(current), Some(total)) => Some(format!("{}/{}", current, total)),
            _ => None,
        }
    }

    /// Is complete?
    pub fn is_complete(&self) -> bool {
        self.value >= 1.0
    }
}

/// Progress builder for running tasks
pub struct ProgressTask<T> {
    progress: Progress,
    task: Box<dyn FnOnce(&mut ProgressReporter) -> T + Send>,
}

impl<T> ProgressTask<T> {
    pub fn new<F>(title: &str, task: F) -> Self
    where
        F: FnOnce(&mut ProgressReporter) -> T + Send + 'static,
    {
        Self {
            progress: Progress::new(title),
            task: Box::new(task),
        }
    }

    pub fn with_location(mut self, location: ProgressLocation) -> Self {
        self.progress.location = location;
        self
    }

    pub fn cancellable(mut self) -> Self {
        self.progress.cancellable = true;
        self
    }
}

/// Progress reporter for tasks
pub struct ProgressReporter {
    value: f32,
    message: Option<String>,
    cancelled: bool,
}

impl ProgressReporter {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            message: None,
            cancelled: false,
        }
    }

    /// Report progress
    pub fn report(&mut self, value: f32) {
        self.value = value.clamp(0.0, 1.0);
    }

    /// Report progress with message
    pub fn report_message(&mut self, value: f32, message: &str) {
        self.value = value.clamp(0.0, 1.0);
        self.message = Some(message.to_string());
    }

    /// Set message
    pub fn message(&mut self, message: &str) {
        self.message = Some(message.to_string());
    }

    /// Increment
    pub fn increment(&mut self, amount: f32) {
        self.value = (self.value + amount).clamp(0.0, 1.0);
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Get current value
    pub fn value(&self) -> f32 {
        self.value
    }
}

impl Default for ProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress() {
        let mut progress = Progress::new("Loading")
            .with_total(10);
        
        assert_eq!(progress.step_progress(), Some("0/10".to_string()));
        
        progress.current = Some(5);
        progress.value = 0.5;
        
        assert_eq!(progress.percentage(), 50);
        assert_eq!(progress.step_progress(), Some("5/10".to_string()));
    }
}
