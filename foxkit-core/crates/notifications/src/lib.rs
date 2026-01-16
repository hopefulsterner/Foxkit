//! # Foxkit Notifications
//!
//! Toast notifications and progress indicators.

pub mod toast;
pub mod progress;

use std::sync::Arc;
use parking_lot::RwLock;
use crossbeam_channel::{Sender, Receiver, unbounded};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

pub use toast::{Toast, ToastType};
pub use progress::{Progress, ProgressLocation};

/// Notification ID
pub type NotificationId = String;

/// Generate unique notification ID
pub fn new_id() -> NotificationId {
    Uuid::new_v4().to_string()
}

/// Notification event
#[derive(Debug, Clone)]
pub enum NotificationEvent {
    /// Toast shown
    ToastShown(NotificationId),
    /// Toast closed
    ToastClosed(NotificationId),
    /// Toast action clicked
    ToastAction(NotificationId, String),
    /// Progress started
    ProgressStarted(NotificationId),
    /// Progress updated
    ProgressUpdated(NotificationId, f32),
    /// Progress completed
    ProgressCompleted(NotificationId),
    /// Progress cancelled
    ProgressCancelled(NotificationId),
}

/// Notification service
pub struct NotificationService {
    /// Active toasts
    toasts: RwLock<Vec<Toast>>,
    /// Active progress indicators
    progress: RwLock<Vec<Progress>>,
    /// Event sender
    event_tx: Sender<NotificationEvent>,
    /// Event receiver
    event_rx: Receiver<NotificationEvent>,
    /// Maximum visible toasts
    max_toasts: usize,
}

impl NotificationService {
    pub fn new() -> Self {
        let (event_tx, event_rx) = unbounded();
        Self {
            toasts: RwLock::new(Vec::new()),
            progress: RwLock::new(Vec::new()),
            event_tx,
            event_rx,
            max_toasts: 5,
        }
    }

    /// Show info toast
    pub fn info(&self, message: &str) -> NotificationId {
        self.show_toast(Toast::info(message))
    }

    /// Show warning toast
    pub fn warn(&self, message: &str) -> NotificationId {
        self.show_toast(Toast::warning(message))
    }

    /// Show error toast
    pub fn error(&self, message: &str) -> NotificationId {
        self.show_toast(Toast::error(message))
    }

    /// Show success toast
    pub fn success(&self, message: &str) -> NotificationId {
        self.show_toast(Toast::success(message))
    }

    /// Show custom toast
    pub fn show_toast(&self, toast: Toast) -> NotificationId {
        let id = toast.id.clone();
        let mut toasts = self.toasts.write();
        
        // Remove old toasts if over limit
        while toasts.len() >= self.max_toasts {
            if let Some(old) = toasts.pop() {
                let _ = self.event_tx.send(NotificationEvent::ToastClosed(old.id));
            }
        }
        
        toasts.insert(0, toast);
        let _ = self.event_tx.send(NotificationEvent::ToastShown(id.clone()));
        id
    }

    /// Close toast
    pub fn close_toast(&self, id: &str) {
        let mut toasts = self.toasts.write();
        if let Some(pos) = toasts.iter().position(|t| t.id == id) {
            toasts.remove(pos);
            let _ = self.event_tx.send(NotificationEvent::ToastClosed(id.to_string()));
        }
    }

    /// Handle toast action
    pub fn handle_action(&self, id: &str, action: &str) {
        let _ = self.event_tx.send(NotificationEvent::ToastAction(
            id.to_string(),
            action.to_string(),
        ));
        // Auto-close after action
        self.close_toast(id);
    }

    /// Get active toasts
    pub fn toasts(&self) -> Vec<Toast> {
        self.toasts.read().clone()
    }

    /// Start progress indicator
    pub fn progress(&self, title: &str) -> ProgressHandle {
        let progress = Progress::new(title);
        let id = progress.id.clone();
        self.progress.write().push(progress);
        let _ = self.event_tx.send(NotificationEvent::ProgressStarted(id.clone()));
        
        ProgressHandle {
            id,
            service: self,
        }
    }

    /// Start progress with location
    pub fn progress_in(&self, title: &str, location: ProgressLocation) -> ProgressHandle {
        let progress = Progress::new(title).with_location(location);
        let id = progress.id.clone();
        self.progress.write().push(progress);
        let _ = self.event_tx.send(NotificationEvent::ProgressStarted(id.clone()));
        
        ProgressHandle {
            id,
            service: self,
        }
    }

    /// Update progress
    fn update_progress(&self, id: &str, value: f32, message: Option<&str>) {
        let mut progress = self.progress.write();
        if let Some(p) = progress.iter_mut().find(|p| p.id == id) {
            p.value = value;
            if let Some(msg) = message {
                p.message = Some(msg.to_string());
            }
            let _ = self.event_tx.send(NotificationEvent::ProgressUpdated(id.to_string(), value));
        }
    }

    /// Complete progress
    fn complete_progress(&self, id: &str) {
        let mut progress = self.progress.write();
        if let Some(pos) = progress.iter().position(|p| p.id == id) {
            progress.remove(pos);
            let _ = self.event_tx.send(NotificationEvent::ProgressCompleted(id.to_string()));
        }
    }

    /// Cancel progress
    fn cancel_progress(&self, id: &str) {
        let mut progress = self.progress.write();
        if let Some(pos) = progress.iter().position(|p| p.id == id) {
            progress.remove(pos);
            let _ = self.event_tx.send(NotificationEvent::ProgressCancelled(id.to_string()));
        }
    }

    /// Get active progress indicators
    pub fn progress_items(&self) -> Vec<Progress> {
        self.progress.read().clone()
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> Receiver<NotificationEvent> {
        self.event_rx.clone()
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to control progress
pub struct ProgressHandle<'a> {
    id: NotificationId,
    service: &'a NotificationService,
}

impl<'a> ProgressHandle<'a> {
    /// Get ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Update progress value (0.0 - 1.0)
    pub fn report(&self, value: f32) {
        self.service.update_progress(&self.id, value.clamp(0.0, 1.0), None);
    }

    /// Update progress with message
    pub fn report_with_message(&self, value: f32, message: &str) {
        self.service.update_progress(&self.id, value.clamp(0.0, 1.0), Some(message));
    }

    /// Set message without changing value
    pub fn message(&self, message: &str) {
        let value = {
            let progress = self.service.progress.read();
            progress.iter().find(|p| p.id == self.id).map(|p| p.value)
        };
        if let Some(v) = value {
            self.service.update_progress(&self.id, v, Some(message));
        }
    }

    /// Increment by amount
    pub fn increment(&self, amount: f32) {
        let value = {
            let progress = self.service.progress.read();
            progress.iter().find(|p| p.id == self.id).map(|p| p.value)
        };
        if let Some(v) = value {
            let new_value = (v + amount).clamp(0.0, 1.0);
            self.service.update_progress(&self.id, new_value, None);
        }
    }

    /// Complete
    pub fn complete(self) {
        self.service.complete_progress(&self.id);
    }

    /// Cancel
    pub fn cancel(self) {
        self.service.cancel_progress(&self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast() {
        let service = NotificationService::new();
        let id = service.info("Test message");
        assert_eq!(service.toasts().len(), 1);
        
        service.close_toast(&id);
        assert_eq!(service.toasts().len(), 0);
    }

    #[test]
    fn test_progress() {
        let service = NotificationService::new();
        let handle = service.progress("Loading...");
        assert_eq!(service.progress_items().len(), 1);
        
        handle.report(0.5);
        assert_eq!(service.progress_items()[0].value, 0.5);
        
        handle.complete();
        assert_eq!(service.progress_items().len(), 0);
    }
}
