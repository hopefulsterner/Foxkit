//! Toast notifications

use serde::{Deserialize, Serialize};
use crate::new_id;

/// Toast type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToastType {
    Info,
    Warning,
    Error,
    Success,
}

impl ToastType {
    pub fn icon(&self) -> &'static str {
        match self {
            ToastType::Info => "info",
            ToastType::Warning => "warning",
            ToastType::Error => "error",
            ToastType::Success => "check",
        }
    }
}

/// Toast notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toast {
    /// Unique ID
    pub id: String,
    /// Toast type
    pub toast_type: ToastType,
    /// Message
    pub message: String,
    /// Title (optional)
    pub title: Option<String>,
    /// Source (optional)
    pub source: Option<String>,
    /// Actions
    #[serde(default)]
    pub actions: Vec<ToastAction>,
    /// Auto-dismiss timeout in ms (0 = no auto-dismiss)
    pub timeout: u64,
    /// Is dismissible
    pub dismissible: bool,
    /// Show close button
    pub show_close: bool,
}

impl Toast {
    pub fn new(toast_type: ToastType, message: &str) -> Self {
        Self {
            id: new_id(),
            toast_type,
            message: message.to_string(),
            title: None,
            source: None,
            actions: Vec::new(),
            timeout: Self::default_timeout(toast_type),
            dismissible: true,
            show_close: true,
        }
    }

    pub fn info(message: &str) -> Self {
        Self::new(ToastType::Info, message)
    }

    pub fn warning(message: &str) -> Self {
        Self::new(ToastType::Warning, message)
    }

    pub fn error(message: &str) -> Self {
        Self::new(ToastType::Error, message)
    }

    pub fn success(message: &str) -> Self {
        Self::new(ToastType::Success, message)
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_source(mut self, source: &str) -> Self {
        self.source = Some(source.to_string());
        self
    }

    pub fn with_action(mut self, label: &str, id: &str) -> Self {
        self.actions.push(ToastAction {
            label: label.to_string(),
            id: id.to_string(),
            is_close: false,
        });
        self
    }

    pub fn with_close_action(mut self, label: &str) -> Self {
        self.actions.push(ToastAction {
            label: label.to_string(),
            id: "close".to_string(),
            is_close: true,
        });
        self
    }

    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn sticky(mut self) -> Self {
        self.timeout = 0;
        self
    }

    pub fn not_dismissible(mut self) -> Self {
        self.dismissible = false;
        self.show_close = false;
        self
    }

    fn default_timeout(toast_type: ToastType) -> u64 {
        match toast_type {
            ToastType::Info => 5000,
            ToastType::Success => 3000,
            ToastType::Warning => 8000,
            ToastType::Error => 0, // Errors don't auto-dismiss
        }
    }
}

/// Toast action button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToastAction {
    /// Button label
    pub label: String,
    /// Action ID
    pub id: String,
    /// Is close action
    pub is_close: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_creation() {
        let toast = Toast::info("Hello!")
            .with_title("Greeting")
            .with_action("OK", "ok");

        assert_eq!(toast.toast_type, ToastType::Info);
        assert_eq!(toast.message, "Hello!");
        assert_eq!(toast.title, Some("Greeting".to_string()));
        assert_eq!(toast.actions.len(), 1);
    }
}
