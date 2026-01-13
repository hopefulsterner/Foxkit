//! Embedded Web Views for Foxkit
//!
//! Sandboxed webview panels for extensions, previews, and custom UI.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Unique webview identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WebviewId(pub Uuid);

impl WebviewId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for WebviewId {
    fn default() -> Self { Self::new() }
}

/// Webview panel options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebviewOptions {
    pub enable_scripts: bool,
    pub enable_forms: bool,
    pub local_resource_roots: Vec<String>,
    pub port_mapping: Vec<PortMapping>,
    pub retain_context_when_hidden: bool,
}

impl Default for WebviewOptions {
    fn default() -> Self {
        Self {
            enable_scripts: true,
            enable_forms: true,
            local_resource_roots: Vec::new(),
            port_mapping: Vec::new(),
            retain_context_when_hidden: false,
        }
    }
}

/// Port mapping for localhost
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub webview_port: u16,
    pub extension_host_port: u16,
}

/// Webview panel state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebviewState {
    Active,
    Hidden,
    Disposed,
}

/// A webview panel
#[derive(Debug, Clone)]
pub struct WebviewPanel {
    pub id: WebviewId,
    pub view_type: String,
    pub title: String,
    pub options: WebviewOptions,
    pub state: WebviewState,
    pub html: String,
    pub icon_path: Option<String>,
}

impl WebviewPanel {
    pub fn new(view_type: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: WebviewId::new(),
            view_type: view_type.into(),
            title: title.into(),
            options: WebviewOptions::default(),
            state: WebviewState::Active,
            html: String::new(),
            icon_path: None,
        }
    }

    pub fn set_html(&mut self, html: impl Into<String>) {
        self.html = html.into();
    }
}

/// Message from webview to extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebviewMessage {
    pub webview_id: WebviewId,
    pub data: serde_json::Value,
}

/// Content Security Policy builder
#[derive(Debug, Clone, Default)]
pub struct CspBuilder {
    directives: HashMap<String, Vec<String>>,
}

impl CspBuilder {
    pub fn new() -> Self { Self::default() }

    pub fn default_src(mut self, sources: Vec<&str>) -> Self {
        self.directives.insert("default-src".into(), sources.iter().map(|s| s.to_string()).collect());
        self
    }

    pub fn script_src(mut self, sources: Vec<&str>) -> Self {
        self.directives.insert("script-src".into(), sources.iter().map(|s| s.to_string()).collect());
        self
    }

    pub fn style_src(mut self, sources: Vec<&str>) -> Self {
        self.directives.insert("style-src".into(), sources.iter().map(|s| s.to_string()).collect());
        self
    }

    pub fn build(&self) -> String {
        self.directives.iter()
            .map(|(k, v)| format!("{} {}", k, v.join(" ")))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Webview service
pub struct WebviewService {
    panels: RwLock<HashMap<WebviewId, Arc<RwLock<WebviewPanel>>>>,
    message_handlers: RwLock<HashMap<WebviewId, Vec<Box<dyn Fn(WebviewMessage) + Send + Sync>>>>,
}

impl WebviewService {
    pub fn new() -> Self {
        Self {
            panels: RwLock::new(HashMap::new()),
            message_handlers: RwLock::new(HashMap::new()),
        }
    }

    pub fn create_panel(&self, view_type: &str, title: &str, options: WebviewOptions) -> WebviewId {
        let mut panel = WebviewPanel::new(view_type, title);
        panel.options = options;
        let id = panel.id;
        self.panels.write().insert(id, Arc::new(RwLock::new(panel)));
        id
    }

    pub fn get_panel(&self, id: WebviewId) -> Option<WebviewPanel> {
        self.panels.read().get(&id).map(|p| p.read().clone())
    }

    pub fn set_html(&self, id: WebviewId, html: &str) {
        if let Some(panel) = self.panels.read().get(&id) {
            panel.write().set_html(html);
        }
    }

    pub fn dispose(&self, id: WebviewId) {
        if let Some(panel) = self.panels.read().get(&id) {
            panel.write().state = WebviewState::Disposed;
        }
        self.panels.write().remove(&id);
        self.message_handlers.write().remove(&id);
    }

    pub fn post_message(&self, id: WebviewId, data: serde_json::Value) {
        let msg = WebviewMessage { webview_id: id, data };
        if let Some(handlers) = self.message_handlers.read().get(&id) {
            for handler in handlers { handler(msg.clone()); }
        }
    }

    pub fn list_panels(&self) -> Vec<WebviewId> {
        self.panels.read().keys().copied().collect()
    }
}

impl Default for WebviewService {
    fn default() -> Self { Self::new() }
}
