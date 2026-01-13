//! Setting schema definitions

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Setting schema
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettingSchema {
    /// Setting type
    #[serde(rename = "type")]
    pub setting_type: SettingType,
    /// Default value
    pub default: Option<Value>,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Markdown description
    #[serde(rename = "markdownDescription")]
    pub markdown_description: Option<String>,
    /// Scope
    #[serde(default)]
    pub scope: SettingScope,
    /// Enum values (for string type)
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<String>>,
    /// Enum descriptions
    #[serde(rename = "enumDescriptions")]
    pub enum_descriptions: Option<Vec<String>>,
    /// Minimum value (for number type)
    pub minimum: Option<f64>,
    /// Maximum value (for number type)
    pub maximum: Option<f64>,
    /// Deprecation message
    #[serde(rename = "deprecationMessage")]
    pub deprecation_message: Option<String>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

impl SettingSchema {
    pub fn new(setting_type: SettingType) -> Self {
        Self {
            setting_type,
            ..Default::default()
        }
    }

    pub fn string() -> Self {
        Self::new(SettingType::String)
    }

    pub fn number() -> Self {
        Self::new(SettingType::Number)
    }

    pub fn boolean() -> Self {
        Self::new(SettingType::Boolean)
    }

    pub fn with_default(mut self, default: Value) -> Self {
        self.default = Some(default);
        self
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    pub fn with_scope(mut self, scope: SettingScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_enum(mut self, values: Vec<String>) -> Self {
        self.enum_values = Some(values);
        self
    }

    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.minimum = Some(min);
        self.maximum = Some(max);
        self
    }

    pub fn deprecated(mut self, message: &str) -> Self {
        self.deprecation_message = Some(message.to_string());
        self
    }

    /// Validate a value against this schema
    pub fn validate(&self, value: &Value) -> ValidationResult {
        match self.setting_type {
            SettingType::String => {
                if !value.is_string() && !value.is_null() {
                    return ValidationResult::Error("Expected string".to_string());
                }
                
                if let Some(ref enum_values) = self.enum_values {
                    if let Some(s) = value.as_str() {
                        if !enum_values.contains(&s.to_string()) {
                            return ValidationResult::Error(format!(
                                "Value must be one of: {}",
                                enum_values.join(", ")
                            ));
                        }
                    }
                }
            }
            SettingType::Number | SettingType::Integer => {
                if !value.is_number() && !value.is_null() {
                    return ValidationResult::Error("Expected number".to_string());
                }
                
                if let Some(n) = value.as_f64() {
                    if let Some(min) = self.minimum {
                        if n < min {
                            return ValidationResult::Error(format!("Value must be >= {}", min));
                        }
                    }
                    if let Some(max) = self.maximum {
                        if n > max {
                            return ValidationResult::Error(format!("Value must be <= {}", max));
                        }
                    }
                }
            }
            SettingType::Boolean => {
                if !value.is_boolean() && !value.is_null() {
                    return ValidationResult::Error("Expected boolean".to_string());
                }
            }
            SettingType::Array => {
                if !value.is_array() && !value.is_null() {
                    return ValidationResult::Error("Expected array".to_string());
                }
            }
            SettingType::Object => {
                if !value.is_object() && !value.is_null() {
                    return ValidationResult::Error("Expected object".to_string());
                }
            }
            SettingType::Null => {}
        }
        
        // Check for deprecation
        if self.deprecation_message.is_some() {
            return ValidationResult::Warning("This setting is deprecated".to_string());
        }
        
        ValidationResult::Ok
    }
}

/// Setting type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingType {
    #[default]
    String,
    Number,
    Integer,
    Boolean,
    Array,
    Object,
    Null,
}

/// Setting scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingScope {
    /// Application-wide setting
    Application,
    /// Machine-specific setting
    Machine,
    /// Applies to all windows
    Window,
    /// Language-specific setting
    LanguageOverridable,
    /// Per-resource setting
    #[default]
    Resource,
}

impl SettingScope {
    pub fn label(&self) -> &'static str {
        match self {
            SettingScope::Application => "Application",
            SettingScope::Machine => "Machine",
            SettingScope::Window => "Window",
            SettingScope::LanguageOverridable => "Language",
            SettingScope::Resource => "Resource",
        }
    }
}

/// Validation result
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Ok,
    Warning(String),
    Error(String),
}

impl ValidationResult {
    pub fn is_ok(&self) -> bool {
        matches!(self, ValidationResult::Ok)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, ValidationResult::Error(_))
    }
}
