//! # Foxkit Keybindings
//!
//! Keyboard shortcut management system.
//! Compatible with VS Code keybindings format.

pub mod chord;
pub mod context;
pub mod keymap;

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub use chord::{Chord, Key, Modifiers};
pub use context::{Context, ContextKey};
pub use keymap::Keymap;

/// Keybinding registry
pub struct KeybindingRegistry {
    /// Keymaps by name
    keymaps: HashMap<String, Keymap>,
    /// Active keymap
    active: String,
    /// Pending chord (for multi-key bindings)
    pending: Option<Chord>,
    /// User overrides
    overrides: Vec<Keybinding>,
}

impl KeybindingRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            keymaps: HashMap::new(),
            active: "default".to_string(),
            pending: None,
            overrides: Vec::new(),
        };
        
        // Load default keymap
        registry.keymaps.insert("default".to_string(), Keymap::default());
        
        registry
    }

    /// Register a keymap
    pub fn register_keymap(&mut self, name: &str, keymap: Keymap) {
        self.keymaps.insert(name.to_string(), keymap);
    }

    /// Set active keymap
    pub fn set_keymap(&mut self, name: &str) -> bool {
        if self.keymaps.contains_key(name) {
            self.active = name.to_string();
            true
        } else {
            false
        }
    }

    /// Get active keymap
    pub fn keymap(&self) -> Option<&Keymap> {
        self.keymaps.get(&self.active)
    }

    /// Add a keybinding override
    pub fn add_override(&mut self, binding: Keybinding) {
        // Remove any existing binding for same key
        self.overrides.retain(|b| b.key != binding.key);
        self.overrides.push(binding);
    }

    /// Load user keybindings from JSON
    pub fn load_user_keybindings(&mut self, json: &str) -> anyhow::Result<()> {
        let bindings: Vec<Keybinding> = serde_json::from_str(json)?;
        for binding in bindings {
            self.add_override(binding);
        }
        Ok(())
    }

    /// Resolve key press to command
    pub fn resolve(&mut self, chord: Chord, context: &Context) -> Option<ResolvedBinding> {
        // Check for chord continuation
        if let Some(pending) = &self.pending {
            // Look for two-chord bindings
            let full_key = format!("{} {}", pending, chord);
            
            // Check overrides first
            for binding in &self.overrides {
                if binding.key == full_key && binding.matches_context(context) {
                    self.pending = None;
                    return Some(ResolvedBinding {
                        command: binding.command.clone(),
                        args: binding.args.clone(),
                    });
                }
            }
            
            // Check keymap
            if let Some(keymap) = self.keymaps.get(&self.active) {
                if let Some(binding) = keymap.get(&full_key, context) {
                    self.pending = None;
                    return Some(binding);
                }
            }
            
            self.pending = None;
        }

        let key_str = chord.to_string();

        // Check overrides first
        for binding in &self.overrides {
            if binding.key == key_str && binding.matches_context(context) {
                // Check for chord prefix
                if self.is_chord_prefix(&key_str) {
                    self.pending = Some(chord);
                    return None;
                }
                return Some(ResolvedBinding {
                    command: binding.command.clone(),
                    args: binding.args.clone(),
                });
            }
        }

        // Check keymap
        if let Some(keymap) = self.keymaps.get(&self.active) {
            // Check for chord prefix
            if keymap.has_prefix(&key_str) {
                self.pending = Some(chord);
                return None;
            }
            
            return keymap.get(&key_str, context);
        }

        None
    }

    fn is_chord_prefix(&self, key: &str) -> bool {
        for binding in &self.overrides {
            if binding.key.starts_with(key) && binding.key.len() > key.len() {
                return true;
            }
        }
        false
    }

    /// Clear pending chord
    pub fn clear_pending(&mut self) {
        self.pending = None;
    }

    /// Has pending chord?
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Get all bindings for a command
    pub fn bindings_for(&self, command: &str) -> Vec<String> {
        let mut keys = Vec::new();
        
        // From overrides
        for binding in &self.overrides {
            if binding.command == command {
                keys.push(binding.key.clone());
            }
        }
        
        // From active keymap
        if let Some(keymap) = self.keymaps.get(&self.active) {
            keys.extend(keymap.bindings_for(command));
        }
        
        keys
    }
}

impl Default for KeybindingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A keybinding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybinding {
    /// Key combination (e.g., "ctrl+shift+p")
    pub key: String,
    /// Command to execute
    pub command: String,
    /// Command arguments
    #[serde(default)]
    pub args: Option<serde_json::Value>,
    /// When clause (context condition)
    #[serde(rename = "when")]
    pub when_clause: Option<String>,
}

impl Keybinding {
    pub fn new(key: &str, command: &str) -> Self {
        Self {
            key: key.to_string(),
            command: command.to_string(),
            args: None,
            when_clause: None,
        }
    }

    pub fn with_when(mut self, when: &str) -> Self {
        self.when_clause = Some(when.to_string());
        self
    }

    pub fn with_args(mut self, args: serde_json::Value) -> Self {
        self.args = Some(args);
        self
    }

    /// Check if binding matches context
    pub fn matches_context(&self, context: &Context) -> bool {
        match &self.when_clause {
            Some(clause) => context.evaluate(clause),
            None => true,
        }
    }
}

/// Resolved binding result
#[derive(Debug, Clone)]
pub struct ResolvedBinding {
    pub command: String,
    pub args: Option<serde_json::Value>,
}

/// Global registry
pub static KEYBINDINGS: once_cell::sync::Lazy<RwLock<KeybindingRegistry>> =
    once_cell::sync::Lazy::new(|| RwLock::new(KeybindingRegistry::new()));
