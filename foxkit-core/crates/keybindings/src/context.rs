//! Keybinding context

use std::collections::HashMap;

/// Context for when-clause evaluation
#[derive(Debug, Clone, Default)]
pub struct Context {
    values: HashMap<String, ContextValue>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a context key
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<ContextValue>) {
        self.values.insert(key.into(), value.into());
    }

    /// Get a context value
    pub fn get(&self, key: &str) -> Option<&ContextValue> {
        self.values.get(key)
    }

    /// Check if key exists
    pub fn has(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Remove a key
    pub fn remove(&mut self, key: &str) {
        self.values.remove(key);
    }

    /// Evaluate a when clause
    pub fn evaluate(&self, clause: &str) -> bool {
        // Simple expression parser for when clauses
        // Supports: key, !key, key == value, key != value, &&, ||
        
        let clause = clause.trim();
        
        // Handle OR
        if let Some(idx) = clause.find(" || ") {
            let (left, right) = clause.split_at(idx);
            return self.evaluate(left) || self.evaluate(&right[4..]);
        }
        
        // Handle AND
        if let Some(idx) = clause.find(" && ") {
            let (left, right) = clause.split_at(idx);
            return self.evaluate(left) && self.evaluate(&right[4..]);
        }
        
        // Handle NOT
        if let Some(rest) = clause.strip_prefix('!') {
            return !self.evaluate(rest);
        }
        
        // Handle equality
        if let Some((key, value)) = clause.split_once(" == ") {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            return self.get(key).map(|v| v.as_str() == Some(value)).unwrap_or(false);
        }
        
        // Handle inequality
        if let Some((key, value)) = clause.split_once(" != ") {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            return self.get(key).map(|v| v.as_str() != Some(value)).unwrap_or(true);
        }
        
        // Handle regex match
        if let Some((key, _pattern)) = clause.split_once(" =~ ") {
            let key = key.trim();
            // Simplified - just check existence
            return self.has(key);
        }
        
        // Simple boolean check
        self.get(clause).map(|v| v.is_truthy()).unwrap_or(false)
    }
}

/// A context value
#[derive(Debug, Clone)]
pub enum ContextValue {
    Bool(bool),
    String(String),
    Number(f64),
}

impl ContextValue {
    pub fn is_truthy(&self) -> bool {
        match self {
            ContextValue::Bool(b) => *b,
            ContextValue::String(s) => !s.is_empty(),
            ContextValue::Number(n) => *n != 0.0,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            ContextValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ContextValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

impl From<bool> for ContextValue {
    fn from(b: bool) -> Self {
        ContextValue::Bool(b)
    }
}

impl From<String> for ContextValue {
    fn from(s: String) -> Self {
        ContextValue::String(s)
    }
}

impl From<&str> for ContextValue {
    fn from(s: &str) -> Self {
        ContextValue::String(s.to_string())
    }
}

impl From<f64> for ContextValue {
    fn from(n: f64) -> Self {
        ContextValue::Number(n)
    }
}

impl From<i32> for ContextValue {
    fn from(n: i32) -> Self {
        ContextValue::Number(n as f64)
    }
}

/// Common context keys
pub struct ContextKey;

impl ContextKey {
    // Editor context
    pub const EDITOR_FOCUS: &'static str = "editorFocus";
    pub const EDITOR_TEXT_FOCUS: &'static str = "editorTextFocus";
    pub const EDITOR_HAS_SELECTION: &'static str = "editorHasSelection";
    pub const EDITOR_HAS_MULTI_SELECTIONS: &'static str = "editorHasMultipleSelections";
    pub const EDITOR_READONLY: &'static str = "editorReadonly";
    pub const EDITOR_LANG_ID: &'static str = "editorLangId";
    
    // Input context
    pub const INPUT_FOCUS: &'static str = "inputFocus";
    pub const TEXT_INPUT_FOCUS: &'static str = "textInputFocus";
    
    // View context
    pub const VIEW_ITEM: &'static str = "viewItem";
    pub const VIEW: &'static str = "view";
    
    // Panel context
    pub const PANEL_FOCUS: &'static str = "panelFocus";
    pub const TERMINAL_FOCUS: &'static str = "terminalFocus";
    
    // Sidebar
    pub const SIDEBAR_VISIBLE: &'static str = "sideBarVisible";
    pub const SIDEBAR_FOCUS: &'static str = "sideBarFocus";
    
    // Explorer
    pub const EXPLORER_VIEW_FOCUS: &'static str = "explorerViewletFocus";
    pub const FILES_EXPLORER_FOCUS: &'static str = "filesExplorerFocus";
    
    // Search
    pub const SEARCH_VIEW_FOCUS: &'static str = "searchViewletFocus";
    pub const IN_SEARCH_EDITOR: &'static str = "inSearchEditor";
    
    // Debug
    pub const IN_DEBUG_MODE: &'static str = "inDebugMode";
    pub const DEBUG_STATE: &'static str = "debugState";
    
    // Git
    pub const IN_SCM_INPUT: &'static str = "inSCMInput";
    
    // Suggestions/Autocomplete
    pub const SUGGEST_WIDGET_VISIBLE: &'static str = "suggestWidgetVisible";
    pub const SUGGEST_WIDGET_DETAILS_VISIBLE: &'static str = "suggestWidgetDetailsVisible";
    
    // Quick open
    pub const IN_QUICK_OPEN: &'static str = "inQuickOpen";
    
    // Misc
    pub const IS_MAC: &'static str = "isMac";
    pub const IS_LINUX: &'static str = "isLinux";
    pub const IS_WINDOWS: &'static str = "isWindows";
    pub const IS_WEB: &'static str = "isWeb";
}
