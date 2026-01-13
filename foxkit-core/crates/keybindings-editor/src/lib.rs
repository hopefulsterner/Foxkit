//! # Foxkit Keybindings Editor
//!
//! Visual editor for keyboard shortcuts.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Keybindings editor service
pub struct KeybindingsEditorService {
    /// All keybindings
    bindings: RwLock<Vec<KeybindingEntry>>,
    /// User overrides
    overrides: RwLock<HashMap<String, KeybindingOverride>>,
    /// Events
    events: broadcast::Sender<KeybindingsEditorEvent>,
    /// Keybindings file path
    file_path: RwLock<Option<PathBuf>>,
}

impl KeybindingsEditorService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            bindings: RwLock::new(Vec::new()),
            overrides: RwLock::new(HashMap::new()),
            events,
            file_path: RwLock::new(None),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<KeybindingsEditorEvent> {
        self.events.subscribe()
    }

    /// Load default keybindings
    pub fn load_defaults(&self) {
        let defaults = default_keybindings();
        *self.bindings.write() = defaults;
    }

    /// Load user keybindings file
    pub fn load_user_keybindings(&self, path: PathBuf) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(&path)?;
        let overrides: Vec<KeybindingOverride> = serde_json::from_str(&content)?;

        let mut map = HashMap::new();
        for override_ in overrides {
            map.insert(override_.command.clone(), override_);
        }

        *self.overrides.write() = map;
        *self.file_path.write() = Some(path);

        Ok(())
    }

    /// Save user keybindings
    pub fn save(&self) -> anyhow::Result<()> {
        let path = self.file_path.read().clone()
            .ok_or_else(|| anyhow::anyhow!("No keybindings file path set"))?;

        let overrides: Vec<&KeybindingOverride> = self.overrides.read()
            .values()
            .collect();

        let json = serde_json::to_string_pretty(&overrides)?;
        std::fs::write(path, json)?;

        let _ = self.events.send(KeybindingsEditorEvent::Saved);

        Ok(())
    }

    /// Get all keybindings
    pub fn all_bindings(&self) -> Vec<KeybindingEntry> {
        let bindings = self.bindings.read();
        let overrides = self.overrides.read();

        bindings.iter().map(|b| {
            if let Some(override_) = overrides.get(&b.command) {
                KeybindingEntry {
                    command: b.command.clone(),
                    key: override_.key.clone().unwrap_or_else(|| b.key.clone()),
                    when: override_.when.clone().or_else(|| b.when.clone()),
                    source: KeybindingSource::User,
                    description: b.description.clone(),
                    category: b.category.clone(),
                    is_disabled: override_.disabled,
                }
            } else {
                b.clone()
            }
        }).collect()
    }

    /// Search keybindings
    pub fn search(&self, query: &str) -> Vec<KeybindingEntry> {
        let query_lower = query.to_lowercase();
        
        self.all_bindings().into_iter()
            .filter(|b| {
                b.command.to_lowercase().contains(&query_lower) ||
                b.key.to_lowercase().contains(&query_lower) ||
                b.description.as_ref()
                    .map(|d| d.to_lowercase().contains(&query_lower))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Set keybinding for command
    pub fn set_keybinding(&self, command: &str, key: &str) {
        let mut overrides = self.overrides.write();
        
        if let Some(override_) = overrides.get_mut(command) {
            override_.key = Some(key.to_string());
        } else {
            overrides.insert(command.to_string(), KeybindingOverride {
                command: command.to_string(),
                key: Some(key.to_string()),
                when: None,
                disabled: false,
            });
        }

        let _ = self.events.send(KeybindingsEditorEvent::KeybindingChanged {
            command: command.to_string(),
            key: key.to_string(),
        });
    }

    /// Remove keybinding override
    pub fn reset_keybinding(&self, command: &str) {
        self.overrides.write().remove(command);

        let _ = self.events.send(KeybindingsEditorEvent::KeybindingReset {
            command: command.to_string(),
        });
    }

    /// Disable keybinding
    pub fn disable_keybinding(&self, command: &str) {
        let mut overrides = self.overrides.write();

        if let Some(override_) = overrides.get_mut(command) {
            override_.disabled = true;
        } else {
            overrides.insert(command.to_string(), KeybindingOverride {
                command: command.to_string(),
                key: None,
                when: None,
                disabled: true,
            });
        }
    }

    /// Enable keybinding
    pub fn enable_keybinding(&self, command: &str) {
        if let Some(override_) = self.overrides.write().get_mut(command) {
            override_.disabled = false;
        }
    }

    /// Find conflicts
    pub fn find_conflicts(&self, key: &str) -> Vec<KeybindingEntry> {
        let key_lower = key.to_lowercase();
        
        self.all_bindings().into_iter()
            .filter(|b| b.key.to_lowercase() == key_lower && !b.is_disabled)
            .collect()
    }

    /// Get by category
    pub fn by_category(&self) -> HashMap<String, Vec<KeybindingEntry>> {
        let mut by_category: HashMap<String, Vec<KeybindingEntry>> = HashMap::new();

        for binding in self.all_bindings() {
            let category = binding.category.clone().unwrap_or_else(|| "Other".to_string());
            by_category.entry(category).or_default().push(binding);
        }

        by_category
    }

    /// Record key sequence (for capture)
    pub fn start_recording(&self) -> KeyRecorder {
        KeyRecorder::new()
    }
}

impl Default for KeybindingsEditorService {
    fn default() -> Self {
        Self::new()
    }
}

/// Keybinding entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingEntry {
    /// Command ID
    pub command: String,
    /// Key sequence (e.g., "Ctrl+Shift+P")
    pub key: String,
    /// When condition
    pub when: Option<String>,
    /// Source (default or user)
    pub source: KeybindingSource,
    /// Description
    pub description: Option<String>,
    /// Category
    pub category: Option<String>,
    /// Is disabled
    pub is_disabled: bool,
}

impl KeybindingEntry {
    pub fn new(command: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            key: key.into(),
            when: None,
            source: KeybindingSource::Default,
            description: None,
            category: None,
            is_disabled: false,
        }
    }

    pub fn with_when(mut self, when: impl Into<String>) -> Self {
        self.when = Some(when.into());
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }
}

/// Keybinding source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeybindingSource {
    Default,
    User,
    Extension,
}

impl KeybindingSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::User => "User",
            Self::Extension => "Extension",
        }
    }
}

/// Keybinding override
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingOverride {
    pub command: String,
    pub key: Option<String>,
    pub when: Option<String>,
    #[serde(default)]
    pub disabled: bool,
}

/// Keybindings editor event
#[derive(Debug, Clone)]
pub enum KeybindingsEditorEvent {
    KeybindingChanged { command: String, key: String },
    KeybindingReset { command: String },
    Saved,
    Loaded,
}

/// Key recorder for capturing key sequences
pub struct KeyRecorder {
    keys: Vec<String>,
    recording: bool,
}

impl KeyRecorder {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            recording: true,
        }
    }

    pub fn record_key(&mut self, key: KeyEvent) {
        if !self.recording {
            return;
        }

        let mut parts = Vec::new();

        if key.ctrl {
            parts.push("Ctrl");
        }
        if key.shift {
            parts.push("Shift");
        }
        if key.alt {
            parts.push("Alt");
        }
        if key.meta {
            parts.push("Meta");
        }

        parts.push(&key.key);

        self.keys.push(parts.join("+"));
    }

    pub fn finish(mut self) -> String {
        self.recording = false;
        self.keys.join(" ")
    }

    pub fn current(&self) -> String {
        self.keys.join(" ")
    }

    pub fn clear(&mut self) {
        self.keys.clear();
    }
}

impl Default for KeyRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Key event
#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

impl KeyEvent {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        }
    }

    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }
}

/// Default keybindings
fn default_keybindings() -> Vec<KeybindingEntry> {
    vec![
        // File operations
        KeybindingEntry::new("file.new", "Ctrl+N")
            .with_description("New File")
            .with_category("File"),
        KeybindingEntry::new("file.open", "Ctrl+O")
            .with_description("Open File")
            .with_category("File"),
        KeybindingEntry::new("file.save", "Ctrl+S")
            .with_description("Save File")
            .with_category("File"),
        KeybindingEntry::new("file.saveAs", "Ctrl+Shift+S")
            .with_description("Save File As")
            .with_category("File"),
        KeybindingEntry::new("file.close", "Ctrl+W")
            .with_description("Close File")
            .with_category("File"),

        // Edit operations
        KeybindingEntry::new("edit.undo", "Ctrl+Z")
            .with_description("Undo")
            .with_category("Edit"),
        KeybindingEntry::new("edit.redo", "Ctrl+Shift+Z")
            .with_description("Redo")
            .with_category("Edit"),
        KeybindingEntry::new("edit.cut", "Ctrl+X")
            .with_description("Cut")
            .with_category("Edit"),
        KeybindingEntry::new("edit.copy", "Ctrl+C")
            .with_description("Copy")
            .with_category("Edit"),
        KeybindingEntry::new("edit.paste", "Ctrl+V")
            .with_description("Paste")
            .with_category("Edit"),
        KeybindingEntry::new("edit.selectAll", "Ctrl+A")
            .with_description("Select All")
            .with_category("Edit"),
        KeybindingEntry::new("edit.find", "Ctrl+F")
            .with_description("Find")
            .with_category("Edit"),
        KeybindingEntry::new("edit.replace", "Ctrl+H")
            .with_description("Replace")
            .with_category("Edit"),

        // Navigation
        KeybindingEntry::new("navigation.goToLine", "Ctrl+G")
            .with_description("Go to Line")
            .with_category("Navigation"),
        KeybindingEntry::new("navigation.goToFile", "Ctrl+P")
            .with_description("Go to File")
            .with_category("Navigation"),
        KeybindingEntry::new("navigation.goToSymbol", "Ctrl+Shift+O")
            .with_description("Go to Symbol")
            .with_category("Navigation"),
        KeybindingEntry::new("navigation.goToDefinition", "F12")
            .with_description("Go to Definition")
            .with_category("Navigation"),
        KeybindingEntry::new("navigation.peekDefinition", "Alt+F12")
            .with_description("Peek Definition")
            .with_category("Navigation"),

        // View
        KeybindingEntry::new("view.commandPalette", "Ctrl+Shift+P")
            .with_description("Command Palette")
            .with_category("View"),
        KeybindingEntry::new("view.sidebar", "Ctrl+B")
            .with_description("Toggle Sidebar")
            .with_category("View"),
        KeybindingEntry::new("view.terminal", "Ctrl+`")
            .with_description("Toggle Terminal")
            .with_category("View"),
        KeybindingEntry::new("view.zoomIn", "Ctrl+=")
            .with_description("Zoom In")
            .with_category("View"),
        KeybindingEntry::new("view.zoomOut", "Ctrl+-")
            .with_description("Zoom Out")
            .with_category("View"),
    ]
}

/// Keybindings editor view model
pub struct KeybindingsEditorViewModel {
    service: Arc<KeybindingsEditorService>,
    /// Search query
    query: RwLock<String>,
    /// Selected index
    selected: RwLock<usize>,
    /// Recording mode
    recording: RwLock<Option<String>>,
}

impl KeybindingsEditorViewModel {
    pub fn new(service: Arc<KeybindingsEditorService>) -> Self {
        Self {
            service,
            query: RwLock::new(String::new()),
            selected: RwLock::new(0),
            recording: RwLock::new(None),
        }
    }

    pub fn search(&self, query: &str) -> Vec<KeybindingEntry> {
        *self.query.write() = query.to_string();
        self.service.search(query)
    }

    pub fn current_query(&self) -> String {
        self.query.read().clone()
    }

    pub fn select(&self, index: usize) {
        *self.selected.write() = index;
    }

    pub fn start_recording(&self, command: &str) {
        *self.recording.write() = Some(command.to_string());
    }

    pub fn stop_recording(&self) {
        *self.recording.write() = None;
    }

    pub fn is_recording(&self) -> bool {
        self.recording.read().is_some()
    }

    pub fn recording_command(&self) -> Option<String> {
        self.recording.read().clone()
    }
}
