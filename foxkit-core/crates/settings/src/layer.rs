//! Settings layers

use std::collections::HashMap;
use std::path::Path;
use serde_json::Value;

/// Layer priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayerPriority {
    /// Default (built-in) settings
    Default = 0,
    /// User global settings
    User = 10,
    /// Workspace settings
    Workspace = 20,
    /// Folder-specific settings
    Folder = 30,
    /// Policy/admin settings (highest priority)
    Policy = 100,
}

/// A settings layer
#[derive(Debug, Clone)]
pub struct SettingsLayer {
    pub priority: LayerPriority,
    values: HashMap<String, Value>,
}

impl SettingsLayer {
    pub fn new(priority: LayerPriority) -> Self {
        Self {
            priority,
            values: HashMap::new(),
        }
    }

    /// Load from file
    pub fn from_file(path: &Path, priority: LayerPriority) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json(&content, priority)
    }

    /// Load from JSON string
    pub fn from_json(json: &str, priority: LayerPriority) -> anyhow::Result<Self> {
        // Handle JSON with comments (JSONC)
        let clean_json = strip_json_comments(json);
        let value: Value = serde_json::from_str(&clean_json)?;
        
        let mut layer = Self::new(priority);
        
        if let Some(obj) = value.as_object() {
            for (key, val) in obj {
                layer.set(key, val.clone());
            }
        }
        
        Ok(layer)
    }

    /// Get a value
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    /// Set a value
    pub fn set(&mut self, key: &str, value: Value) {
        self.values.insert(key.to_string(), value);
    }

    /// Remove a value
    pub fn remove(&mut self, key: &str) {
        self.values.remove(key);
    }

    /// Get all values
    pub fn values(&self) -> &HashMap<String, Value> {
        &self.values
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Count of settings
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Save to file
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let json = serde_json::to_string_pretty(&self.values)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// To JSON value
    pub fn to_json(&self) -> Value {
        Value::Object(
            self.values
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        )
    }
}

/// Strip comments from JSONC
fn strip_json_comments(json: &str) -> String {
    let mut result = String::with_capacity(json.len());
    let mut chars = json.chars().peekable();
    let mut in_string = false;
    let mut escape = false;

    while let Some(c) = chars.next() {
        if escape {
            result.push(c);
            escape = false;
            continue;
        }

        if c == '\\' && in_string {
            result.push(c);
            escape = true;
            continue;
        }

        if c == '"' {
            in_string = !in_string;
            result.push(c);
            continue;
        }

        if !in_string {
            if c == '/' {
                if let Some(&next) = chars.peek() {
                    if next == '/' {
                        // Line comment - skip until newline
                        chars.next();
                        while let Some(&ch) = chars.peek() {
                            if ch == '\n' {
                                break;
                            }
                            chars.next();
                        }
                        continue;
                    } else if next == '*' {
                        // Block comment - skip until */
                        chars.next();
                        let mut prev = ' ';
                        while let Some(ch) = chars.next() {
                            if prev == '*' && ch == '/' {
                                break;
                            }
                            prev = ch;
                        }
                        continue;
                    }
                }
            }
        }

        result.push(c);
    }

    result
}

/// Merge multiple layers into one value
pub fn merge_layers(layers: &[SettingsLayer]) -> HashMap<String, Value> {
    let mut result = HashMap::new();
    
    for layer in layers {
        for (key, value) in &layer.values {
            result.insert(key.clone(), value.clone());
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_comments() {
        let jsonc = r#"{
            // Comment
            "key": "value", /* block */
            "num": 42
        }"#;
        
        let clean = strip_json_comments(jsonc);
        let value: Value = serde_json::from_str(&clean).unwrap();
        assert_eq!(value["key"], "value");
        assert_eq!(value["num"], 42);
    }
}
