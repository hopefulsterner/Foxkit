//! # Foxkit Signature Help
//!
//! Function/method signature hints while typing.

use std::path::PathBuf;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Signature help service
pub struct SignatureHelpService {
    /// Current signature help
    current: RwLock<Option<SignatureHelpState>>,
    /// Events
    events: broadcast::Sender<SignatureHelpEvent>,
    /// Configuration
    config: RwLock<SignatureHelpConfig>,
}

impl SignatureHelpService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            current: RwLock::new(None),
            events,
            config: RwLock::new(SignatureHelpConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<SignatureHelpEvent> {
        self.events.subscribe()
    }

    /// Configure signature help
    pub fn configure(&self, config: SignatureHelpConfig) {
        *self.config.write() = config;
    }

    /// Request signature help
    pub async fn get_signature_help(
        &self,
        file: &PathBuf,
        line: u32,
        column: u32,
        trigger: SignatureHelpTrigger,
    ) -> Option<SignatureHelp> {
        let config = self.config.read();
        
        if !config.enabled {
            return None;
        }

        // Would call LSP signatureHelp
        None
    }

    /// Set current signature help
    pub fn set_current(&self, help: SignatureHelp, position: (u32, u32)) {
        *self.current.write() = Some(SignatureHelpState {
            help: help.clone(),
            position,
        });

        let _ = self.events.send(SignatureHelpEvent::Shown { help });
    }

    /// Get current signature help
    pub fn current(&self) -> Option<SignatureHelpState> {
        self.current.read().clone()
    }

    /// Hide signature help
    pub fn hide(&self) {
        *self.current.write() = None;
        let _ = self.events.send(SignatureHelpEvent::Hidden);
    }

    /// Navigate to next signature
    pub fn next_signature(&self) {
        if let Some(mut state) = self.current.write().as_mut() {
            if state.help.signatures.len() > 1 {
                let next = (state.help.active_signature + 1) % state.help.signatures.len();
                state.help.active_signature = next;
            }
        }
    }

    /// Navigate to previous signature
    pub fn previous_signature(&self) {
        if let Some(mut state) = self.current.write().as_mut() {
            if state.help.signatures.len() > 1 {
                let prev = if state.help.active_signature == 0 {
                    state.help.signatures.len() - 1
                } else {
                    state.help.active_signature - 1
                };
                state.help.active_signature = prev;
            }
        }
    }
}

impl Default for SignatureHelpService {
    fn default() -> Self {
        Self::new()
    }
}

/// Signature help state
#[derive(Debug, Clone)]
pub struct SignatureHelpState {
    /// Signature help data
    pub help: SignatureHelp,
    /// Display position
    pub position: (u32, u32),
}

/// Signature help
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureHelp {
    /// Available signatures
    pub signatures: Vec<SignatureInformation>,
    /// Active signature index
    pub active_signature: usize,
    /// Active parameter index
    pub active_parameter: Option<usize>,
}

impl SignatureHelp {
    pub fn new(signatures: Vec<SignatureInformation>) -> Self {
        Self {
            signatures,
            active_signature: 0,
            active_parameter: None,
        }
    }

    pub fn active(&self) -> Option<&SignatureInformation> {
        self.signatures.get(self.active_signature)
    }

    pub fn active_param(&self) -> Option<&ParameterInformation> {
        let sig = self.active()?;
        let param_idx = self.active_parameter?;
        sig.parameters.get(param_idx)
    }

    pub fn has_multiple(&self) -> bool {
        self.signatures.len() > 1
    }
}

/// Signature information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureInformation {
    /// Signature label
    pub label: String,
    /// Documentation
    pub documentation: Option<Documentation>,
    /// Parameters
    pub parameters: Vec<ParameterInformation>,
    /// Active parameter (override)
    pub active_parameter: Option<usize>,
}

impl SignatureInformation {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            documentation: None,
            parameters: Vec::new(),
            active_parameter: None,
        }
    }

    pub fn with_documentation(mut self, doc: Documentation) -> Self {
        self.documentation = Some(doc);
        self
    }

    pub fn with_parameters(mut self, params: Vec<ParameterInformation>) -> Self {
        self.parameters = params;
        self
    }

    /// Get formatted signature with highlighted parameter
    pub fn formatted(&self, active_param: Option<usize>) -> FormattedSignature {
        let mut parts = Vec::new();
        let label = &self.label;

        if let Some(idx) = active_param {
            if let Some(param) = self.parameters.get(idx) {
                match &param.label {
                    ParameterLabel::Simple(name) => {
                        if let Some(start) = label.find(name.as_str()) {
                            let end = start + name.len();
                            
                            if start > 0 {
                                parts.push(SignaturePart::text(&label[..start]));
                            }
                            parts.push(SignaturePart::active(&label[start..end]));
                            if end < label.len() {
                                parts.push(SignaturePart::text(&label[end..]));
                            }
                        } else {
                            parts.push(SignaturePart::text(label));
                        }
                    }
                    ParameterLabel::Offsets(start, end) => {
                        let start = *start as usize;
                        let end = *end as usize;
                        
                        if start > 0 {
                            parts.push(SignaturePart::text(&label[..start]));
                        }
                        if end <= label.len() {
                            parts.push(SignaturePart::active(&label[start..end]));
                        }
                        if end < label.len() {
                            parts.push(SignaturePart::text(&label[end..]));
                        }
                    }
                }
            } else {
                parts.push(SignaturePart::text(label));
            }
        } else {
            parts.push(SignaturePart::text(label));
        }

        FormattedSignature { parts }
    }
}

/// Parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInformation {
    /// Parameter label
    pub label: ParameterLabel,
    /// Documentation
    pub documentation: Option<Documentation>,
}

impl ParameterInformation {
    pub fn simple(name: impl Into<String>) -> Self {
        Self {
            label: ParameterLabel::Simple(name.into()),
            documentation: None,
        }
    }

    pub fn offsets(start: u32, end: u32) -> Self {
        Self {
            label: ParameterLabel::Offsets(start, end),
            documentation: None,
        }
    }

    pub fn with_documentation(mut self, doc: Documentation) -> Self {
        self.documentation = Some(doc);
        self
    }
}

/// Parameter label
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterLabel {
    /// Simple string label
    Simple(String),
    /// Start/end offsets into signature label
    Offsets(u32, u32),
}

/// Documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Documentation {
    String(String),
    Markdown(String),
}

impl Documentation {
    pub fn plain(text: impl Into<String>) -> Self {
        Self::String(text.into())
    }

    pub fn markdown(text: impl Into<String>) -> Self {
        Self::Markdown(text.into())
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::String(s) => s,
            Self::Markdown(s) => s,
        }
    }

    pub fn is_markdown(&self) -> bool {
        matches!(self, Self::Markdown(_))
    }
}

/// Formatted signature
#[derive(Debug, Clone)]
pub struct FormattedSignature {
    pub parts: Vec<SignaturePart>,
}

/// Signature part
#[derive(Debug, Clone)]
pub struct SignaturePart {
    pub text: String,
    pub is_active: bool,
}

impl SignaturePart {
    pub fn text(s: &str) -> Self {
        Self { text: s.to_string(), is_active: false }
    }

    pub fn active(s: &str) -> Self {
        Self { text: s.to_string(), is_active: true }
    }
}

/// Signature help trigger
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SignatureHelpTrigger {
    /// Invoked manually
    Invoked,
    /// Trigger character typed
    TriggerCharacter(char),
    /// Content changed while active
    ContentChange,
}

/// Signature help configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureHelpConfig {
    /// Enable signature help
    pub enabled: bool,
    /// Trigger characters
    pub trigger_characters: Vec<char>,
    /// Retrigger characters
    pub retrigger_characters: Vec<char>,
    /// Auto-show on trigger
    pub auto_trigger: bool,
}

impl Default for SignatureHelpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            trigger_characters: vec!['(', ','],
            retrigger_characters: vec![',', ')'],
            auto_trigger: true,
        }
    }
}

/// Signature help event
#[derive(Debug, Clone)]
pub enum SignatureHelpEvent {
    Shown { help: SignatureHelp },
    Hidden,
    SignatureChanged { index: usize },
    ParameterChanged { index: usize },
}
