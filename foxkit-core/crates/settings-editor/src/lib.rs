//! # Foxkit Settings Editor
//!
//! Visual settings editor with search and categorization.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;

/// Settings editor service
pub struct SettingsEditorService {
    /// Setting definitions
    definitions: RwLock<Vec<SettingDefinition>>,
    /// Current values
    values: RwLock<HashMap<String, Value>>,
    /// Events
    events: broadcast::Sender<SettingsEditorEvent>,
}

impl SettingsEditorService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            definitions: RwLock::new(Vec::new()),
            values: RwLock::new(HashMap::new()),
            events,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<SettingsEditorEvent> {
        self.events.subscribe()
    }

    /// Register setting definition
    pub fn register(&self, definition: SettingDefinition) {
        self.definitions.write().push(definition);
    }

    /// Register multiple definitions
    pub fn register_many(&self, definitions: Vec<SettingDefinition>) {
        self.definitions.write().extend(definitions);
    }

    /// Set value
    pub fn set_value(&self, key: &str, value: Value) {
        self.values.write().insert(key.to_string(), value.clone());

        let _ = self.events.send(SettingsEditorEvent::ValueChanged {
            key: key.to_string(),
            value,
        });
    }

    /// Get value
    pub fn get_value(&self, key: &str) -> Option<Value> {
        self.values.read().get(key).cloned()
    }

    /// Get effective value (with default)
    pub fn get_effective(&self, key: &str) -> Option<Value> {
        if let Some(value) = self.get_value(key) {
            return Some(value);
        }

        // Return default
        self.definitions.read()
            .iter()
            .find(|d| d.key == key)
            .map(|d| d.default.clone())
    }

    /// Reset to default
    pub fn reset(&self, key: &str) {
        self.values.write().remove(key);

        let _ = self.events.send(SettingsEditorEvent::Reset {
            key: key.to_string(),
        });
    }

    /// Get all definitions
    pub fn all_definitions(&self) -> Vec<SettingDefinition> {
        self.definitions.read().clone()
    }

    /// Search settings
    pub fn search(&self, query: &str) -> Vec<SettingDefinition> {
        let query_lower = query.to_lowercase();

        self.definitions.read()
            .iter()
            .filter(|d| {
                d.key.to_lowercase().contains(&query_lower) ||
                d.title.to_lowercase().contains(&query_lower) ||
                d.description.as_ref()
                    .map(|desc| desc.to_lowercase().contains(&query_lower))
                    .unwrap_or(false) ||
                d.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    /// Get settings by category
    pub fn by_category(&self) -> HashMap<String, Vec<SettingDefinition>> {
        let mut by_category: HashMap<String, Vec<SettingDefinition>> = HashMap::new();

        for def in self.definitions.read().iter() {
            let category = def.category.clone().unwrap_or_else(|| "Other".to_string());
            by_category.entry(category).or_default().push(def.clone());
        }

        by_category
    }

    /// Get setting entry (definition + current value)
    pub fn get_entry(&self, key: &str) -> Option<SettingEntry> {
        let definitions = self.definitions.read();
        let values = self.values.read();

        let definition = definitions.iter().find(|d| d.key == key)?.clone();
        let value = values.get(key).cloned();
        let is_modified = value.is_some();

        Some(SettingEntry {
            definition,
            value,
            is_modified,
        })
    }

    /// Get modified settings count
    pub fn modified_count(&self) -> usize {
        self.values.read().len()
    }

    /// Reset all to defaults
    pub fn reset_all(&self) {
        self.values.write().clear();
        let _ = self.events.send(SettingsEditorEvent::AllReset);
    }
}

impl Default for SettingsEditorService {
    fn default() -> Self {
        Self::new()
    }
}

/// Setting definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingDefinition {
    /// Setting key (e.g., "editor.fontSize")
    pub key: String,
    /// Display title
    pub title: String,
    /// Description
    pub description: Option<String>,
    /// Category
    pub category: Option<String>,
    /// Setting type
    #[serde(rename = "type")]
    pub setting_type: SettingType,
    /// Default value
    pub default: Value,
    /// Scope
    pub scope: SettingScope,
    /// Tags for search
    pub tags: Vec<String>,
    /// Deprecation message
    pub deprecated: Option<String>,
}

impl SettingDefinition {
    pub fn string(key: impl Into<String>, title: impl Into<String>, default: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            description: None,
            category: None,
            setting_type: SettingType::String { pattern: None },
            default: Value::String(default.into()),
            scope: SettingScope::Window,
            tags: Vec::new(),
            deprecated: None,
        }
    }

    pub fn number(key: impl Into<String>, title: impl Into<String>, default: f64) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            description: None,
            category: None,
            setting_type: SettingType::Number { minimum: None, maximum: None },
            default: Value::Number(serde_json::Number::from_f64(default).unwrap()),
            scope: SettingScope::Window,
            tags: Vec::new(),
            deprecated: None,
        }
    }

    pub fn boolean(key: impl Into<String>, title: impl Into<String>, default: bool) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            description: None,
            category: None,
            setting_type: SettingType::Boolean,
            default: Value::Bool(default),
            scope: SettingScope::Window,
            tags: Vec::new(),
            deprecated: None,
        }
    }

    pub fn enum_type(key: impl Into<String>, title: impl Into<String>, options: Vec<String>, default: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            description: None,
            category: None,
            setting_type: SettingType::Enum { options },
            default: Value::String(default.into()),
            scope: SettingScope::Window,
            tags: Vec::new(),
            deprecated: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    pub fn with_scope(mut self, scope: SettingScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn is_deprecated(&self) -> bool {
        self.deprecated.is_some()
    }
}

/// Setting type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SettingType {
    String { pattern: Option<String> },
    Number { minimum: Option<f64>, maximum: Option<f64> },
    Integer { minimum: Option<i64>, maximum: Option<i64> },
    Boolean,
    Enum { options: Vec<String> },
    Array { item_type: Box<SettingType> },
    Object,
}

impl SettingType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::String { .. } => "string",
            Self::Number { .. } => "number",
            Self::Integer { .. } => "integer",
            Self::Boolean => "boolean",
            Self::Enum { .. } => "enum",
            Self::Array { .. } => "array",
            Self::Object => "object",
        }
    }

    pub fn validate(&self, value: &Value) -> bool {
        match (self, value) {
            (Self::String { pattern }, Value::String(s)) => {
                if let Some(ref p) = pattern {
                    regex::Regex::new(p)
                        .map(|r| r.is_match(s))
                        .unwrap_or(true)
                } else {
                    true
                }
            }
            (Self::Number { minimum, maximum }, Value::Number(n)) => {
                if let Some(f) = n.as_f64() {
                    minimum.map(|m| f >= m).unwrap_or(true) &&
                        maximum.map(|m| f <= m).unwrap_or(true)
                } else {
                    false
                }
            }
            (Self::Integer { minimum, maximum }, Value::Number(n)) => {
                if let Some(i) = n.as_i64() {
                    minimum.map(|m| i >= m).unwrap_or(true) &&
                        maximum.map(|m| i <= m).unwrap_or(true)
                } else {
                    false
                }
            }
            (Self::Boolean, Value::Bool(_)) => true,
            (Self::Enum { options }, Value::String(s)) => options.contains(s),
            (Self::Array { .. }, Value::Array(_)) => true,
            (Self::Object, Value::Object(_)) => true,
            _ => false,
        }
    }
}

/// Setting scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettingScope {
    /// Application-wide
    Application,
    /// Per-window
    Window,
    /// Per-workspace
    Resource,
    /// Per-language
    LanguageOverridable,
}

impl SettingScope {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Application => "Application",
            Self::Window => "Window",
            Self::Resource => "Resource",
            Self::LanguageOverridable => "Language",
        }
    }
}

/// Setting entry (definition + value)
#[derive(Debug, Clone)]
pub struct SettingEntry {
    pub definition: SettingDefinition,
    pub value: Option<Value>,
    pub is_modified: bool,
}

impl SettingEntry {
    pub fn effective_value(&self) -> &Value {
        self.value.as_ref().unwrap_or(&self.definition.default)
    }
}

/// Settings editor event
#[derive(Debug, Clone)]
pub enum SettingsEditorEvent {
    ValueChanged { key: String, value: Value },
    Reset { key: String },
    AllReset,
}

/// Settings editor view model
pub struct SettingsEditorViewModel {
    service: Arc<SettingsEditorService>,
    /// Current search query
    query: RwLock<String>,
    /// Selected category
    category: RwLock<Option<String>>,
    /// Show modified only
    modified_only: RwLock<bool>,
}

impl SettingsEditorViewModel {
    pub fn new(service: Arc<SettingsEditorService>) -> Self {
        Self {
            service,
            query: RwLock::new(String::new()),
            category: RwLock::new(None),
            modified_only: RwLock::new(false),
        }
    }

    pub fn search(&self, query: &str) -> Vec<SettingEntry> {
        *self.query.write() = query.to_string();

        let results = if query.is_empty() {
            self.service.all_definitions()
        } else {
            self.service.search(query)
        };

        results.into_iter()
            .map(|d| {
                let key = d.key.clone();
                SettingEntry {
                    definition: d,
                    value: self.service.get_value(&key),
                    is_modified: self.service.get_value(&key).is_some(),
                }
            })
            .filter(|e| {
                let modified_only = *self.modified_only.read();
                !modified_only || e.is_modified
            })
            .filter(|e| {
                let category = self.category.read();
                category.is_none() || e.definition.category == *category
            })
            .collect()
    }

    pub fn set_category(&self, category: Option<String>) {
        *self.category.write() = category;
    }

    pub fn toggle_modified_only(&self) {
        let mut modified_only = self.modified_only.write();
        *modified_only = !*modified_only;
    }

    pub fn categories(&self) -> Vec<String> {
        let mut categories: Vec<String> = self.service.by_category()
            .keys()
            .cloned()
            .collect();
        categories.sort();
        categories
    }

    pub fn modified_count(&self) -> usize {
        self.service.modified_count()
    }
}

/// Register common editor settings
pub fn register_editor_settings(service: &SettingsEditorService) {
    let settings = vec![
        SettingDefinition::number("editor.fontSize", "Font Size", 14.0)
            .with_description("Controls the font size in pixels")
            .with_category("Editor"),
        
        SettingDefinition::string("editor.fontFamily", "Font Family", "monospace")
            .with_description("Controls the font family")
            .with_category("Editor"),
        
        SettingDefinition::number("editor.tabSize", "Tab Size", 4.0)
            .with_description("The number of spaces a tab is equal to")
            .with_category("Editor"),
        
        SettingDefinition::boolean("editor.insertSpaces", "Insert Spaces", true)
            .with_description("Insert spaces when pressing Tab")
            .with_category("Editor"),
        
        SettingDefinition::boolean("editor.wordWrap", "Word Wrap", false)
            .with_description("Controls how lines should wrap")
            .with_category("Editor"),
        
        SettingDefinition::boolean("editor.minimap.enabled", "Show Minimap", true)
            .with_description("Controls whether the minimap is shown")
            .with_category("Editor"),
        
        SettingDefinition::boolean("editor.lineNumbers", "Line Numbers", true)
            .with_description("Controls the display of line numbers")
            .with_category("Editor"),
        
        SettingDefinition::enum_type(
            "editor.cursorStyle",
            "Cursor Style",
            vec!["line".into(), "block".into(), "underline".into()],
            "line",
        )
            .with_description("Controls the cursor style")
            .with_category("Editor"),
        
        SettingDefinition::boolean("files.autoSave", "Auto Save", false)
            .with_description("Controls auto save of editors")
            .with_category("Files"),
        
        SettingDefinition::boolean("files.trimTrailingWhitespace", "Trim Trailing Whitespace", false)
            .with_description("Trim trailing whitespace when saving")
            .with_category("Files"),
    ];

    service.register_many(settings);
}
