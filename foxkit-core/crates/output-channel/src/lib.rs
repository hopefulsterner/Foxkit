//! # Foxkit Output Channel
//!
//! Log output panels for extensions and system messages.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use chrono::{DateTime, Local};

/// Output channel service
pub struct OutputChannelService {
    /// Channels by name
    channels: RwLock<HashMap<String, OutputChannel>>,
    /// Active channel
    active: RwLock<Option<String>>,
    /// Events
    events: broadcast::Sender<OutputChannelEvent>,
    /// Configuration
    config: RwLock<OutputConfig>,
}

impl OutputChannelService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(256);

        Self {
            channels: RwLock::new(HashMap::new()),
            active: RwLock::new(None),
            events,
            config: RwLock::new(OutputConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<OutputChannelEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: OutputConfig) {
        *self.config.write() = config;
    }

    /// Create or get channel
    pub fn get_channel(&self, name: &str) -> OutputChannelHandle {
        let mut channels = self.channels.write();

        if !channels.contains_key(name) {
            let channel = OutputChannel::new(name.to_string());
            channels.insert(name.to_string(), channel);

            let _ = self.events.send(OutputChannelEvent::ChannelCreated {
                name: name.to_string(),
            });
        }

        OutputChannelHandle {
            name: name.to_string(),
            service: Arc::new(self.clone()),
        }
    }

    /// Get all channel names
    pub fn channel_names(&self) -> Vec<String> {
        self.channels.read().keys().cloned().collect()
    }

    /// Set active channel
    pub fn set_active(&self, name: &str) {
        *self.active.write() = Some(name.to_string());

        let _ = self.events.send(OutputChannelEvent::ActiveChanged {
            name: name.to_string(),
        });
    }

    /// Get active channel name
    pub fn active(&self) -> Option<String> {
        self.active.read().clone()
    }

    /// Append to channel
    pub fn append(&self, name: &str, text: &str) {
        let config = self.config.read();
        let mut channels = self.channels.write();

        if let Some(channel) = channels.get_mut(name) {
            // Add timestamp if configured
            let line = if config.show_timestamps {
                let now: DateTime<Local> = Local::now();
                format!("[{}] {}", now.format("%H:%M:%S"), text)
            } else {
                text.to_string()
            };

            channel.lines.push(OutputLine {
                text: line.clone(),
                timestamp: Local::now(),
                level: OutputLevel::Info,
            });

            // Trim if too many lines
            let max = config.max_lines;
            if channel.lines.len() > max {
                channel.lines.drain(0..(channel.lines.len() - max));
            }

            let _ = self.events.send(OutputChannelEvent::LineAppended {
                channel: name.to_string(),
                line,
            });
        }
    }

    /// Append line with newline
    pub fn append_line(&self, name: &str, text: &str) {
        self.append(name, &format!("{}\n", text));
    }

    /// Clear channel
    pub fn clear(&self, name: &str) {
        let mut channels = self.channels.write();

        if let Some(channel) = channels.get_mut(name) {
            channel.lines.clear();

            let _ = self.events.send(OutputChannelEvent::ChannelCleared {
                name: name.to_string(),
            });
        }
    }

    /// Get channel content
    pub fn get_content(&self, name: &str) -> Option<String> {
        let channels = self.channels.read();

        channels.get(name).map(|c| {
            c.lines.iter()
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>()
                .join("")
        })
    }

    /// Get channel lines
    pub fn get_lines(&self, name: &str) -> Vec<OutputLine> {
        let channels = self.channels.read();
        channels.get(name)
            .map(|c| c.lines.clone())
            .unwrap_or_default()
    }

    /// Show channel (make visible and active)
    pub fn show(&self, name: &str, preserve_focus: bool) {
        self.set_active(name);

        let _ = self.events.send(OutputChannelEvent::Shown {
            name: name.to_string(),
            preserve_focus,
        });
    }

    /// Hide channel
    pub fn hide(&self, name: &str) {
        let _ = self.events.send(OutputChannelEvent::Hidden {
            name: name.to_string(),
        });
    }

    /// Dispose channel
    pub fn dispose(&self, name: &str) {
        self.channels.write().remove(name);

        let _ = self.events.send(OutputChannelEvent::ChannelDisposed {
            name: name.to_string(),
        });
    }
}

impl Clone for OutputChannelService {
    fn clone(&self) -> Self {
        Self {
            channels: RwLock::new(self.channels.read().clone()),
            active: RwLock::new(self.active.read().clone()),
            events: self.events.clone(),
            config: RwLock::new(self.config.read().clone()),
        }
    }
}

impl Default for OutputChannelService {
    fn default() -> Self {
        Self::new()
    }
}

/// Output channel
#[derive(Debug, Clone)]
pub struct OutputChannel {
    /// Channel name
    pub name: String,
    /// Output lines
    pub lines: Vec<OutputLine>,
    /// Is hidden
    pub hidden: bool,
}

impl OutputChannel {
    pub fn new(name: String) -> Self {
        Self {
            name,
            lines: Vec::new(),
            hidden: false,
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

/// Output line
#[derive(Debug, Clone)]
pub struct OutputLine {
    /// Line text
    pub text: String,
    /// Timestamp
    pub timestamp: DateTime<Local>,
    /// Log level
    pub level: OutputLevel,
}

/// Output level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

impl OutputLevel {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Trace => "",
            Self::Debug => "",
            Self::Info => "ℹ",
            Self::Warning => "⚠",
            Self::Error => "✗",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::Trace => "dimmed",
            Self::Debug => "default",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Maximum lines per channel
    pub max_lines: usize,
    /// Show timestamps
    pub show_timestamps: bool,
    /// Word wrap
    pub word_wrap: bool,
    /// Smart scroll (scroll to bottom on new output)
    pub smart_scroll: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            max_lines: 10000,
            show_timestamps: false,
            word_wrap: true,
            smart_scroll: true,
        }
    }
}

/// Output channel event
#[derive(Debug, Clone)]
pub enum OutputChannelEvent {
    ChannelCreated { name: String },
    ChannelDisposed { name: String },
    ChannelCleared { name: String },
    LineAppended { channel: String, line: String },
    ActiveChanged { name: String },
    Shown { name: String, preserve_focus: bool },
    Hidden { name: String },
}

/// Handle for interacting with a specific channel
pub struct OutputChannelHandle {
    name: String,
    service: Arc<OutputChannelService>,
}

impl OutputChannelHandle {
    /// Get channel name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Append text
    pub fn append(&self, text: &str) {
        self.service.append(&self.name, text);
    }

    /// Append line
    pub fn append_line(&self, text: &str) {
        self.service.append_line(&self.name, text);
    }

    /// Clear
    pub fn clear(&self) {
        self.service.clear(&self.name);
    }

    /// Show
    pub fn show(&self, preserve_focus: bool) {
        self.service.show(&self.name, preserve_focus);
    }

    /// Hide
    pub fn hide(&self) {
        self.service.hide(&self.name);
    }

    /// Dispose
    pub fn dispose(&self) {
        self.service.dispose(&self.name);
    }
}

/// Output panel view model
pub struct OutputPanelViewModel {
    service: Arc<OutputChannelService>,
    /// Scroll position
    scroll_position: RwLock<usize>,
    /// Search filter
    filter: RwLock<Option<String>>,
}

impl OutputPanelViewModel {
    pub fn new(service: Arc<OutputChannelService>) -> Self {
        Self {
            service,
            scroll_position: RwLock::new(0),
            filter: RwLock::new(None),
        }
    }

    pub fn channels(&self) -> Vec<String> {
        self.service.channel_names()
    }

    pub fn active_channel(&self) -> Option<String> {
        self.service.active()
    }

    pub fn select_channel(&self, name: &str) {
        self.service.set_active(name);
    }

    pub fn lines(&self) -> Vec<OutputLine> {
        if let Some(name) = self.service.active() {
            let mut lines = self.service.get_lines(&name);

            // Apply filter
            if let Some(ref filter) = *self.filter.read() {
                let filter_lower = filter.to_lowercase();
                lines.retain(|l| l.text.to_lowercase().contains(&filter_lower));
            }

            lines
        } else {
            Vec::new()
        }
    }

    pub fn set_filter(&self, filter: Option<String>) {
        *self.filter.write() = filter;
    }

    pub fn clear_active(&self) {
        if let Some(name) = self.service.active() {
            self.service.clear(&name);
        }
    }

    pub fn scroll_to_bottom(&self) {
        if let Some(name) = self.service.active() {
            let line_count = self.service.get_lines(&name).len();
            *self.scroll_position.write() = line_count.saturating_sub(1);
        }
    }
}

/// Log output channel (for extensions)
pub struct LogOutputChannel {
    handle: OutputChannelHandle,
    log_level: RwLock<OutputLevel>,
}

impl LogOutputChannel {
    pub fn new(handle: OutputChannelHandle) -> Self {
        Self {
            handle,
            log_level: RwLock::new(OutputLevel::Info),
        }
    }

    pub fn set_log_level(&self, level: OutputLevel) {
        *self.log_level.write() = level;
    }

    pub fn trace(&self, message: &str) {
        self.log(OutputLevel::Trace, message);
    }

    pub fn debug(&self, message: &str) {
        self.log(OutputLevel::Debug, message);
    }

    pub fn info(&self, message: &str) {
        self.log(OutputLevel::Info, message);
    }

    pub fn warn(&self, message: &str) {
        self.log(OutputLevel::Warning, message);
    }

    pub fn error(&self, message: &str) {
        self.log(OutputLevel::Error, message);
    }

    fn log(&self, level: OutputLevel, message: &str) {
        let current_level = *self.log_level.read();

        if (level as u8) >= (current_level as u8) {
            let prefix = match level {
                OutputLevel::Trace => "[trace]",
                OutputLevel::Debug => "[debug]",
                OutputLevel::Info => "[info]",
                OutputLevel::Warning => "[warn]",
                OutputLevel::Error => "[error]",
            };
            
            self.handle.append_line(&format!("{} {}", prefix, message));
        }
    }
}
