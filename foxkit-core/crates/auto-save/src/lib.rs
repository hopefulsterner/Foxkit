//! # Foxkit Auto Save
//!
//! Automatic file saving with configurable triggers.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Auto save service
pub struct AutoSaveService {
    /// Pending saves
    pending: RwLock<HashMap<PathBuf, PendingSave>>,
    /// Events
    events: broadcast::Sender<AutoSaveEvent>,
    /// Configuration
    config: RwLock<AutoSaveConfig>,
    /// Is enabled
    enabled: RwLock<bool>,
}

impl AutoSaveService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            pending: RwLock::new(HashMap::new()),
            events,
            config: RwLock::new(AutoSaveConfig::default()),
            enabled: RwLock::new(false),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<AutoSaveEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: AutoSaveConfig) {
        *self.enabled.write() = config.enabled;
        *self.config.write() = config;
    }

    /// Enable auto save
    pub fn enable(&self) {
        *self.enabled.write() = true;
    }

    /// Disable auto save
    pub fn disable(&self) {
        *self.enabled.write() = false;
    }

    /// Is auto save enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    /// Handle file change
    pub fn on_file_changed(&self, file: PathBuf) {
        if !*self.enabled.read() {
            return;
        }

        let config = self.config.read();
        
        match config.mode {
            AutoSaveMode::Off => return,
            AutoSaveMode::AfterDelay => {
                self.schedule_save(file, config.delay);
            }
            AutoSaveMode::OnFocusChange | AutoSaveMode::OnWindowChange => {
                // Handled by on_focus_change
                self.mark_dirty(file);
            }
        }
    }

    /// Handle focus change
    pub fn on_focus_change(&self, from_file: Option<PathBuf>) {
        if !*self.enabled.read() {
            return;
        }

        let config = self.config.read();
        
        if config.mode == AutoSaveMode::OnFocusChange {
            if let Some(file) = from_file {
                self.trigger_save(file);
            }
        }
    }

    /// Handle window focus change
    pub fn on_window_change(&self) {
        if !*self.enabled.read() {
            return;
        }

        let config = self.config.read();
        
        if config.mode == AutoSaveMode::OnWindowChange {
            self.save_all_dirty();
        }
    }

    /// Schedule a save
    fn schedule_save(&self, file: PathBuf, delay: Duration) {
        let save_at = Instant::now() + delay;
        
        self.pending.write().insert(file.clone(), PendingSave {
            scheduled_at: save_at,
            dirty: true,
        });

        let _ = self.events.send(AutoSaveEvent::Scheduled {
            file,
            delay,
        });
    }

    /// Mark file as dirty
    fn mark_dirty(&self, file: PathBuf) {
        self.pending.write()
            .entry(file)
            .and_modify(|p| p.dirty = true)
            .or_insert(PendingSave {
                scheduled_at: Instant::now(),
                dirty: true,
            });
    }

    /// Trigger save for file
    fn trigger_save(&self, file: PathBuf) {
        if let Some(pending) = self.pending.write().remove(&file) {
            if pending.dirty {
                let _ = self.events.send(AutoSaveEvent::Saving { file: file.clone() });
                // Would actually save the file here
                let _ = self.events.send(AutoSaveEvent::Saved { file });
            }
        }
    }

    /// Save all dirty files
    fn save_all_dirty(&self) {
        let files: Vec<PathBuf> = self.pending
            .read()
            .iter()
            .filter(|(_, p)| p.dirty)
            .map(|(f, _)| f.clone())
            .collect();

        for file in files {
            self.trigger_save(file);
        }
    }

    /// Process pending saves (called on timer)
    pub fn process_pending(&self) {
        if !*self.enabled.read() {
            return;
        }

        let now = Instant::now();
        let files_to_save: Vec<PathBuf> = self.pending
            .read()
            .iter()
            .filter(|(_, p)| p.dirty && p.scheduled_at <= now)
            .map(|(f, _)| f.clone())
            .collect();

        for file in files_to_save {
            self.trigger_save(file);
        }
    }

    /// Cancel pending save
    pub fn cancel(&self, file: &PathBuf) {
        self.pending.write().remove(file);
    }

    /// Get pending files
    pub fn get_pending(&self) -> Vec<PathBuf> {
        self.pending
            .read()
            .iter()
            .filter(|(_, p)| p.dirty)
            .map(|(f, _)| f.clone())
            .collect()
    }
}

impl Default for AutoSaveService {
    fn default() -> Self {
        Self::new()
    }
}

/// Pending save info
struct PendingSave {
    scheduled_at: Instant,
    dirty: bool,
}

/// Auto save configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSaveConfig {
    /// Enable auto save
    pub enabled: bool,
    /// Auto save mode
    pub mode: AutoSaveMode,
    /// Delay before saving
    #[serde(with = "duration_millis")]
    pub delay: Duration,
    /// Files to exclude
    pub exclude: Vec<String>,
}

impl Default for AutoSaveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: AutoSaveMode::AfterDelay,
            delay: Duration::from_millis(1000),
            exclude: Vec::new(),
        }
    }
}

/// Auto save mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoSaveMode {
    /// Auto save is off
    Off,
    /// Save after a delay
    AfterDelay,
    /// Save when editor loses focus
    OnFocusChange,
    /// Save when window loses focus
    OnWindowChange,
}

impl AutoSaveMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::AfterDelay => "afterDelay",
            Self::OnFocusChange => "onFocusChange",
            Self::OnWindowChange => "onWindowChange",
        }
    }
}

/// Auto save event
#[derive(Debug, Clone)]
pub enum AutoSaveEvent {
    Scheduled { file: PathBuf, delay: Duration },
    Saving { file: PathBuf },
    Saved { file: PathBuf },
    Failed { file: PathBuf, error: String },
    Cancelled { file: PathBuf },
}

mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}
