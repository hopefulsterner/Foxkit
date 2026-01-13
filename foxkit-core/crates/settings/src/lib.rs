//! # Foxkit Settings
//!
//! User and workspace settings management with layered configuration.

pub mod schema;
pub mod layer;
pub mod watcher;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use schema::{SettingSchema, SettingType, SettingScope};
pub use layer::{SettingsLayer, LayerPriority};

/// Settings manager
pub struct Settings {
    /// Schema registry
    schemas: HashMap<String, SettingSchema>,
    /// Settings layers (in priority order)
    layers: Vec<SettingsLayer>,
    /// Cached resolved values
    cache: HashMap<String, Value>,
    /// Change listeners
    listeners: Vec<Box<dyn Fn(&str, &Value) + Send + Sync>>,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            layers: Vec::new(),
            cache: HashMap::new(),
            listeners: Vec::new(),
        }
    }

    /// Load default settings
    pub fn load_defaults() -> Self {
        let mut settings = Self::new();
        
        // Add default layer
        settings.add_layer(SettingsLayer::new(LayerPriority::Default));
        
        // Register core schemas
        settings.register_core_schemas();
        
        // Load user settings
        if let Some(path) = user_settings_path() {
            if path.exists() {
                if let Ok(layer) = SettingsLayer::from_file(&path, LayerPriority::User) {
                    settings.add_layer(layer);
                }
            }
        }
        
        settings
    }

    /// Register a setting schema
    pub fn register_schema(&mut self, key: &str, schema: SettingSchema) {
        self.schemas.insert(key.to_string(), schema);
    }

    /// Add a settings layer
    pub fn add_layer(&mut self, layer: SettingsLayer) {
        self.layers.push(layer);
        self.layers.sort_by_key(|l| l.priority);
        self.invalidate_cache();
    }

    /// Remove layer by priority
    pub fn remove_layer(&mut self, priority: LayerPriority) {
        self.layers.retain(|l| l.priority != priority);
        self.invalidate_cache();
    }

    /// Get a setting value
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.get_value(key)
            .and_then(|v| serde_json::from_value(v).ok())
    }

    /// Get raw value
    pub fn get_value(&self, key: &str) -> Option<Value> {
        // Check cache
        if let Some(value) = self.cache.get(key) {
            return Some(value.clone());
        }

        // Search layers in reverse priority order
        for layer in self.layers.iter().rev() {
            if let Some(value) = layer.get(key) {
                return Some(value.clone());
            }
        }

        // Return default from schema
        self.schemas.get(key).and_then(|s| s.default.clone())
    }

    /// Set a setting in specific layer
    pub fn set(&mut self, key: &str, value: Value, priority: LayerPriority) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.priority == priority) {
            layer.set(key, value.clone());
            self.cache.remove(key);
            
            // Notify listeners
            for listener in &self.listeners {
                listener(key, &value);
            }
        }
    }

    /// Set user setting
    pub fn set_user(&mut self, key: &str, value: Value) {
        self.set(key, value, LayerPriority::User);
    }

    /// Set workspace setting
    pub fn set_workspace(&mut self, key: &str, value: Value) {
        self.set(key, value, LayerPriority::Workspace);
    }

    /// Update settings from JSON
    pub fn update(&mut self, json: &Value, priority: LayerPriority) {
        if let Some(obj) = json.as_object() {
            for (key, value) in obj {
                self.set(key, value.clone(), priority);
            }
        }
    }

    /// Get all settings (merged)
    pub fn all(&self) -> HashMap<String, Value> {
        let mut result = HashMap::new();
        
        // Apply layers in order
        for layer in &self.layers {
            for (key, value) in layer.values() {
                result.insert(key.clone(), value.clone());
            }
        }
        
        result
    }

    /// Save user settings
    pub fn save_user(&self) -> anyhow::Result<()> {
        if let Some(layer) = self.layers.iter().find(|l| l.priority == LayerPriority::User) {
            if let Some(path) = user_settings_path() {
                layer.save(&path)?;
            }
        }
        Ok(())
    }

    /// Add change listener
    pub fn on_change(&mut self, listener: impl Fn(&str, &Value) + Send + Sync + 'static) {
        self.listeners.push(Box::new(listener));
    }

    /// Check if setting exists
    pub fn has(&self, key: &str) -> bool {
        self.get_value(key).is_some()
    }

    /// Get schema for setting
    pub fn schema(&self, key: &str) -> Option<&SettingSchema> {
        self.schemas.get(key)
    }

    fn invalidate_cache(&mut self) {
        self.cache.clear();
    }

    fn register_core_schemas(&mut self) {
        // Editor settings
        self.register_schema("editor.fontSize", SettingSchema {
            setting_type: SettingType::Number,
            default: Some(Value::Number(14.into())),
            description: "Controls the font size in pixels".to_string(),
            scope: SettingScope::Resource,
            ..Default::default()
        });

        self.register_schema("editor.fontFamily", SettingSchema {
            setting_type: SettingType::String,
            default: Some(Value::String("Consolas, 'Courier New', monospace".to_string())),
            description: "Controls the font family".to_string(),
            scope: SettingScope::Resource,
            ..Default::default()
        });

        self.register_schema("editor.tabSize", SettingSchema {
            setting_type: SettingType::Number,
            default: Some(Value::Number(4.into())),
            description: "The number of spaces a tab is equal to".to_string(),
            scope: SettingScope::Resource,
            ..Default::default()
        });

        self.register_schema("editor.insertSpaces", SettingSchema {
            setting_type: SettingType::Boolean,
            default: Some(Value::Bool(true)),
            description: "Insert spaces when pressing Tab".to_string(),
            scope: SettingScope::Resource,
            ..Default::default()
        });

        self.register_schema("editor.wordWrap", SettingSchema {
            setting_type: SettingType::String,
            default: Some(Value::String("off".to_string())),
            description: "Controls how lines should wrap".to_string(),
            scope: SettingScope::Resource,
            enum_values: Some(vec!["off".to_string(), "on".to_string(), "wordWrapColumn".to_string(), "bounded".to_string()]),
            ..Default::default()
        });

        // Workbench settings
        self.register_schema("workbench.colorTheme", SettingSchema {
            setting_type: SettingType::String,
            default: Some(Value::String("Default Dark+".to_string())),
            description: "Specifies the color theme".to_string(),
            scope: SettingScope::Application,
            ..Default::default()
        });

        self.register_schema("workbench.iconTheme", SettingSchema {
            setting_type: SettingType::String,
            default: Some(Value::String("vs-seti".to_string())),
            description: "Specifies the icon theme".to_string(),
            scope: SettingScope::Application,
            ..Default::default()
        });

        // Files settings
        self.register_schema("files.autoSave", SettingSchema {
            setting_type: SettingType::String,
            default: Some(Value::String("off".to_string())),
            description: "Controls auto save of editors".to_string(),
            scope: SettingScope::Application,
            enum_values: Some(vec!["off".to_string(), "afterDelay".to_string(), "onFocusChange".to_string(), "onWindowChange".to_string()]),
            ..Default::default()
        });

        self.register_schema("files.encoding", SettingSchema {
            setting_type: SettingType::String,
            default: Some(Value::String("utf8".to_string())),
            description: "The default character set encoding to use".to_string(),
            scope: SettingScope::Resource,
            ..Default::default()
        });

        // Terminal settings
        self.register_schema("terminal.integrated.fontSize", SettingSchema {
            setting_type: SettingType::Number,
            default: Some(Value::Number(14.into())),
            description: "Controls the font size of the terminal".to_string(),
            scope: SettingScope::Application,
            ..Default::default()
        });

        self.register_schema("terminal.integrated.shell.linux", SettingSchema {
            setting_type: SettingType::String,
            default: Some(Value::Null),
            description: "The path of the shell on Linux".to_string(),
            scope: SettingScope::Application,
            ..Default::default()
        });
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::load_defaults()
    }
}

/// Get user settings path
pub fn user_settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("foxkit").join("settings.json"))
}

/// Get user keybindings path
pub fn user_keybindings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("foxkit").join("keybindings.json"))
}

/// Get user snippets directory
pub fn user_snippets_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("foxkit").join("snippets"))
}

/// Global settings instance
pub static SETTINGS: once_cell::sync::Lazy<RwLock<Settings>> =
    once_cell::sync::Lazy::new(|| RwLock::new(Settings::load_defaults()));
