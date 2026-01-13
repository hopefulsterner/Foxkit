//! Internationalization (i18n) Support for Foxkit
//!
//! Multi-language support using Fluent message format with
//! locale detection, fallback chains, and dynamic loading.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Language identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LanguageId(pub String);

impl LanguageId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn en_us() -> Self {
        Self("en-US".to_string())
    }

    pub fn language_code(&self) -> &str {
        self.0.split('-').next().unwrap_or(&self.0)
    }

    pub fn region_code(&self) -> Option<&str> {
        self.0.split('-').nth(1)
    }
}

impl Default for LanguageId {
    fn default() -> Self {
        Self::en_us()
    }
}

/// Message key for localization lookup
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageKey(pub String);

impl MessageKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
}

impl From<&str> for MessageKey {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Localized message bundle
#[derive(Debug, Clone, Default)]
pub struct MessageBundle {
    pub language: LanguageId,
    pub messages: HashMap<String, String>,
    pub plural_rules: Option<PluralRules>,
}

/// Plural rule forms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluralRules {
    pub zero: Option<String>,
    pub one: Option<String>,
    pub two: Option<String>,
    pub few: Option<String>,
    pub many: Option<String>,
    pub other: String,
}

impl PluralRules {
    pub fn select(&self, count: i64) -> &str {
        match count {
            0 if self.zero.is_some() => self.zero.as_ref().unwrap(),
            1 if self.one.is_some() => self.one.as_ref().unwrap(),
            2 if self.two.is_some() => self.two.as_ref().unwrap(),
            3..=10 if self.few.is_some() => self.few.as_ref().unwrap(),
            11..=99 if self.many.is_some() => self.many.as_ref().unwrap(),
            _ => &self.other,
        }
    }
}

/// Message arguments for interpolation
#[derive(Debug, Clone, Default)]
pub struct MessageArgs {
    args: HashMap<String, MessageValue>,
}

/// Value types for message arguments
#[derive(Debug, Clone)]
pub enum MessageValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
}

impl MessageArgs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(mut self, key: impl Into<String>, value: impl Into<MessageValue>) -> Self {
        self.args.insert(key.into(), value.into());
        self
    }

    pub fn get(&self, key: &str) -> Option<&MessageValue> {
        self.args.get(key)
    }
}

impl From<String> for MessageValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for MessageValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for MessageValue {
    fn from(n: i64) -> Self {
        Self::Integer(n)
    }
}

impl From<f64> for MessageValue {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<bool> for MessageValue {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

/// Format result with potential errors
#[derive(Debug)]
pub struct FormatResult {
    pub text: String,
    pub errors: Vec<FormatError>,
}

/// Format error
#[derive(Debug, Clone)]
pub struct FormatError {
    pub key: String,
    pub kind: FormatErrorKind,
}

/// Kind of format error
#[derive(Debug, Clone)]
pub enum FormatErrorKind {
    MissingMessage,
    MissingArgument(String),
    InvalidArgument(String),
    SyntaxError(String),
}

/// Locale information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleInfo {
    pub id: LanguageId,
    pub display_name: String,
    pub native_name: String,
    pub direction: TextDirection,
    pub date_format: String,
    pub number_format: NumberFormat,
}

/// Text direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

impl Default for TextDirection {
    fn default() -> Self {
        Self::LeftToRight
    }
}

/// Number format configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumberFormat {
    pub decimal_separator: char,
    pub thousands_separator: char,
    pub grouping: Vec<u8>,
}

impl Default for NumberFormat {
    fn default() -> Self {
        Self {
            decimal_separator: '.',
            thousands_separator: ',',
            grouping: vec![3],
        }
    }
}

/// Localization service
pub struct LocalizationService {
    current_locale: RwLock<LanguageId>,
    fallback_chain: RwLock<Vec<LanguageId>>,
    bundles: RwLock<HashMap<LanguageId, MessageBundle>>,
    locale_info: RwLock<HashMap<LanguageId, LocaleInfo>>,
}

impl LocalizationService {
    pub fn new() -> Self {
        Self {
            current_locale: RwLock::new(LanguageId::en_us()),
            fallback_chain: RwLock::new(vec![LanguageId::en_us()]),
            bundles: RwLock::new(HashMap::new()),
            locale_info: RwLock::new(HashMap::new()),
        }
    }

    pub fn current_locale(&self) -> LanguageId {
        self.current_locale.read().clone()
    }

    pub fn set_locale(&self, locale: LanguageId) {
        *self.current_locale.write() = locale;
    }

    pub fn set_fallback_chain(&self, chain: Vec<LanguageId>) {
        *self.fallback_chain.write() = chain;
    }

    pub fn register_bundle(&self, bundle: MessageBundle) {
        let lang = bundle.language.clone();
        self.bundles.write().insert(lang, bundle);
    }

    pub fn register_locale_info(&self, info: LocaleInfo) {
        let id = info.id.clone();
        self.locale_info.write().insert(id, info);
    }

    pub fn get_message(&self, key: &str) -> Option<String> {
        self.get_message_with_args(key, &MessageArgs::new())
    }

    pub fn get_message_with_args(&self, key: &str, args: &MessageArgs) -> Option<String> {
        let current = self.current_locale.read().clone();
        let chain = self.fallback_chain.read().clone();

        // Try current locale first
        if let Some(msg) = self.lookup_and_format(&current, key, args) {
            return Some(msg);
        }

        // Try fallback chain
        for fallback in &chain {
            if let Some(msg) = self.lookup_and_format(fallback, key, args) {
                return Some(msg);
            }
        }

        None
    }

    fn lookup_and_format(&self, lang: &LanguageId, key: &str, args: &MessageArgs) -> Option<String> {
        let bundles = self.bundles.read();
        let bundle = bundles.get(lang)?;
        let template = bundle.messages.get(key)?;
        Some(self.interpolate(template, args))
    }

    fn interpolate(&self, template: &str, args: &MessageArgs) -> String {
        let mut result = template.to_string();
        for (key, value) in &args.args {
            let placeholder = format!("{{{}}}", key);
            let replacement = match value {
                MessageValue::String(s) => s.clone(),
                MessageValue::Number(n) => n.to_string(),
                MessageValue::Integer(n) => n.to_string(),
                MessageValue::Boolean(b) => b.to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }
        result
    }

    pub fn format(&self, key: &str, args: MessageArgs) -> FormatResult {
        match self.get_message_with_args(key, &args) {
            Some(text) => FormatResult {
                text,
                errors: Vec::new(),
            },
            None => FormatResult {
                text: format!("[{}]", key),
                errors: vec![FormatError {
                    key: key.to_string(),
                    kind: FormatErrorKind::MissingMessage,
                }],
            },
        }
    }

    pub fn available_locales(&self) -> Vec<LanguageId> {
        self.bundles.read().keys().cloned().collect()
    }

    pub fn get_locale_info(&self, id: &LanguageId) -> Option<LocaleInfo> {
        self.locale_info.read().get(id).cloned()
    }

    pub fn text_direction(&self) -> TextDirection {
        let current = self.current_locale.read();
        self.locale_info
            .read()
            .get(&current)
            .map(|i| i.direction)
            .unwrap_or_default()
    }
}

impl Default for LocalizationService {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience macro for localized strings
#[macro_export]
macro_rules! t {
    ($service:expr, $key:expr) => {
        $service.get_message($key).unwrap_or_else(|| format!("[{}]", $key))
    };
    ($service:expr, $key:expr, $($arg:tt)*) => {{
        let args = $crate::MessageArgs::new()$($arg)*;
        $service.get_message_with_args($key, &args).unwrap_or_else(|| format!("[{}]", $key))
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_id() {
        let lang = LanguageId::new("en-US");
        assert_eq!(lang.language_code(), "en");
        assert_eq!(lang.region_code(), Some("US"));
    }

    #[test]
    fn test_message_interpolation() {
        let service = LocalizationService::new();
        let mut bundle = MessageBundle::default();
        bundle.language = LanguageId::en_us();
        bundle.messages.insert("hello".to_string(), "Hello, {name}!".to_string());
        service.register_bundle(bundle);

        let args = MessageArgs::new().set("name", "World");
        let msg = service.get_message_with_args("hello", &args);
        assert_eq!(msg, Some("Hello, World!".to_string()));
    }

    #[test]
    fn test_fallback() {
        let service = LocalizationService::new();
        let mut en_bundle = MessageBundle::default();
        en_bundle.language = LanguageId::en_us();
        en_bundle.messages.insert("test".to_string(), "Test".to_string());
        service.register_bundle(en_bundle);

        service.set_locale(LanguageId::new("de-DE"));
        service.set_fallback_chain(vec![LanguageId::en_us()]);

        assert_eq!(service.get_message("test"), Some("Test".to_string()));
    }
}
