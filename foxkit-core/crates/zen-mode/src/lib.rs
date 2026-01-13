//! # Foxkit Zen Mode
//!
//! Distraction-free editing mode.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Zen mode service
pub struct ZenModeService {
    /// Is active
    active: RwLock<bool>,
    /// Stored UI state
    stored_state: RwLock<Option<StoredUiState>>,
    /// Configuration
    config: RwLock<ZenModeConfig>,
    /// Event sender
    event_tx: broadcast::Sender<ZenModeEvent>,
}

impl ZenModeService {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(16);

        Self {
            active: RwLock::new(false),
            stored_state: RwLock::new(None),
            config: RwLock::new(ZenModeConfig::default()),
            event_tx,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<ZenModeEvent> {
        self.event_tx.subscribe()
    }

    /// Enter zen mode
    pub fn enter(&self, current_state: StoredUiState) {
        if *self.active.read() {
            return;
        }

        *self.stored_state.write() = Some(current_state);
        *self.active.write() = true;

        let config = self.config.read().clone();
        let _ = self.event_tx.send(ZenModeEvent::Entered(config));
    }

    /// Exit zen mode
    pub fn exit(&self) -> Option<StoredUiState> {
        if !*self.active.read() {
            return None;
        }

        *self.active.write() = false;
        let state = self.stored_state.write().take();

        let _ = self.event_tx.send(ZenModeEvent::Exited);
        state
    }

    /// Toggle zen mode
    pub fn toggle(&self, current_state: StoredUiState) -> Option<StoredUiState> {
        if *self.active.read() {
            self.exit()
        } else {
            self.enter(current_state);
            None
        }
    }

    /// Is zen mode active
    pub fn is_active(&self) -> bool {
        *self.active.read()
    }

    /// Configure zen mode
    pub fn configure(&self, config: ZenModeConfig) {
        *self.config.write() = config;
    }

    /// Get configuration
    pub fn config(&self) -> ZenModeConfig {
        self.config.read().clone()
    }

    /// Get stored state
    pub fn stored_state(&self) -> Option<StoredUiState> {
        self.stored_state.read().clone()
    }

    /// Build zen mode UI settings
    pub fn build_settings(&self) -> ZenModeSettings {
        let config = self.config.read();

        ZenModeSettings {
            hide_activity_bar: config.hide_activity_bar,
            hide_status_bar: config.hide_status_bar,
            hide_tabs: config.hide_tabs,
            hide_line_numbers: config.hide_line_numbers,
            hide_sidebar: config.hide_sidebar,
            hide_panel: config.hide_panel,
            hide_minimap: config.hide_minimap,
            fullscreen: config.fullscreen,
            center_layout: config.center_layout,
            center_width: config.center_width,
            silence_notifications: config.silence_notifications,
            font_size_increase: config.font_size_increase,
            line_height_increase: config.line_height_increase,
        }
    }
}

impl Default for ZenModeService {
    fn default() -> Self {
        Self::new()
    }
}

/// Stored UI state for restoration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoredUiState {
    /// Activity bar visible
    pub activity_bar_visible: bool,
    /// Status bar visible
    pub status_bar_visible: bool,
    /// Tabs visible
    pub tabs_visible: bool,
    /// Line numbers visible
    pub line_numbers_visible: bool,
    /// Sidebar visible
    pub sidebar_visible: bool,
    /// Sidebar width
    pub sidebar_width: u32,
    /// Panel visible
    pub panel_visible: bool,
    /// Panel height
    pub panel_height: u32,
    /// Minimap visible
    pub minimap_visible: bool,
    /// Font size
    pub font_size: f32,
    /// Line height
    pub line_height: f32,
    /// Is fullscreen
    pub fullscreen: bool,
}

/// Zen mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZenModeConfig {
    /// Hide activity bar
    pub hide_activity_bar: bool,
    /// Hide status bar
    pub hide_status_bar: bool,
    /// Hide tabs
    pub hide_tabs: bool,
    /// Hide line numbers
    pub hide_line_numbers: bool,
    /// Hide sidebar
    pub hide_sidebar: bool,
    /// Hide panel
    pub hide_panel: bool,
    /// Hide minimap
    pub hide_minimap: bool,
    /// Enter fullscreen
    pub fullscreen: bool,
    /// Center layout
    pub center_layout: bool,
    /// Center width ratio
    pub center_width: f32,
    /// Silence notifications
    pub silence_notifications: bool,
    /// Font size increase
    pub font_size_increase: f32,
    /// Line height increase
    pub line_height_increase: f32,
    /// Restore on exit
    pub restore_on_exit: bool,
}

impl Default for ZenModeConfig {
    fn default() -> Self {
        Self {
            hide_activity_bar: true,
            hide_status_bar: true,
            hide_tabs: true,
            hide_line_numbers: false,
            hide_sidebar: true,
            hide_panel: true,
            hide_minimap: true,
            fullscreen: true,
            center_layout: true,
            center_width: 0.6,
            silence_notifications: true,
            font_size_increase: 0.0,
            line_height_increase: 0.0,
            restore_on_exit: true,
        }
    }
}

/// Applied zen mode settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZenModeSettings {
    pub hide_activity_bar: bool,
    pub hide_status_bar: bool,
    pub hide_tabs: bool,
    pub hide_line_numbers: bool,
    pub hide_sidebar: bool,
    pub hide_panel: bool,
    pub hide_minimap: bool,
    pub fullscreen: bool,
    pub center_layout: bool,
    pub center_width: f32,
    pub silence_notifications: bool,
    pub font_size_increase: f32,
    pub line_height_increase: f32,
}

/// Zen mode event
#[derive(Debug, Clone)]
pub enum ZenModeEvent {
    /// Entered zen mode
    Entered(ZenModeConfig),
    /// Exited zen mode
    Exited,
}

/// Focus mode (lighter version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusModeConfig {
    /// Dim unfocused editors
    pub dim_unfocused: bool,
    /// Dim opacity
    pub dim_opacity: f32,
    /// Highlight current line
    pub highlight_current_line: bool,
    /// Typewriter scrolling
    pub typewriter_scroll: bool,
    /// Typewriter position (fraction of viewport)
    pub typewriter_position: f32,
}

impl Default for FocusModeConfig {
    fn default() -> Self {
        Self {
            dim_unfocused: true,
            dim_opacity: 0.3,
            highlight_current_line: true,
            typewriter_scroll: false,
            typewriter_position: 0.5,
        }
    }
}

/// Focus mode service
pub struct FocusModeService {
    active: RwLock<bool>,
    config: RwLock<FocusModeConfig>,
    event_tx: broadcast::Sender<FocusModeEvent>,
}

impl FocusModeService {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(16);

        Self {
            active: RwLock::new(false),
            config: RwLock::new(FocusModeConfig::default()),
            event_tx,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<FocusModeEvent> {
        self.event_tx.subscribe()
    }

    /// Toggle focus mode
    pub fn toggle(&self) {
        let mut active = self.active.write();
        *active = !*active;

        if *active {
            let _ = self.event_tx.send(FocusModeEvent::Enabled);
        } else {
            let _ = self.event_tx.send(FocusModeEvent::Disabled);
        }
    }

    /// Enable focus mode
    pub fn enable(&self) {
        *self.active.write() = true;
        let _ = self.event_tx.send(FocusModeEvent::Enabled);
    }

    /// Disable focus mode
    pub fn disable(&self) {
        *self.active.write() = false;
        let _ = self.event_tx.send(FocusModeEvent::Disabled);
    }

    /// Is active
    pub fn is_active(&self) -> bool {
        *self.active.read()
    }

    /// Configure
    pub fn configure(&self, config: FocusModeConfig) {
        *self.config.write() = config;
    }

    /// Get config
    pub fn config(&self) -> FocusModeConfig {
        self.config.read().clone()
    }
}

impl Default for FocusModeService {
    fn default() -> Self {
        Self::new()
    }
}

/// Focus mode event
#[derive(Debug, Clone)]
pub enum FocusModeEvent {
    Enabled,
    Disabled,
}
