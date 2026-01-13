//! # Foxkit Document Links
//!
//! Clickable URLs, file paths, and custom links.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Document links service
pub struct DocumentLinksService {
    /// Registered providers
    providers: RwLock<Vec<Arc<dyn LinkProvider>>>,
    /// Cached links
    cache: RwLock<HashMap<PathBuf, Vec<DocumentLink>>>,
    /// Events
    events: broadcast::Sender<DocumentLinksEvent>,
    /// Configuration
    config: RwLock<DocumentLinksConfig>,
}

impl DocumentLinksService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            providers: RwLock::new(Vec::new()),
            cache: RwLock::new(HashMap::new()),
            events,
            config: RwLock::new(DocumentLinksConfig::default()),
        }
    }

    /// Register a link provider
    pub fn register_provider(&self, provider: Arc<dyn LinkProvider>) {
        self.providers.write().push(provider);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<DocumentLinksEvent> {
        self.events.subscribe()
    }

    /// Configure links
    pub fn configure(&self, config: DocumentLinksConfig) {
        *self.config.write() = config;
    }

    /// Get links for document
    pub async fn get_links(&self, file: &PathBuf, content: &str) -> Vec<DocumentLink> {
        let providers = self.providers.read().clone();
        let config = self.config.read().clone();
        let mut links = Vec::new();

        if !config.enabled {
            return links;
        }

        for provider in providers {
            match provider.provide_links(file, content).await {
                Ok(provider_links) => links.extend(provider_links),
                Err(e) => {
                    tracing::warn!("Link provider {} failed: {}", provider.id(), e);
                }
            }
        }

        // Cache and return
        self.cache.write().insert(file.clone(), links.clone());

        links
    }

    /// Resolve link (for lazy resolution)
    pub async fn resolve_link(&self, link: &DocumentLink) -> Option<String> {
        if link.target.is_some() {
            return link.target.clone();
        }

        let providers = self.providers.read().clone();

        for provider in providers {
            if let Ok(Some(target)) = provider.resolve_link(link).await {
                return Some(target);
            }
        }

        None
    }

    /// Invalidate cache
    pub fn invalidate(&self, file: &PathBuf) {
        self.cache.write().remove(file);
    }
}

impl Default for DocumentLinksService {
    fn default() -> Self {
        Self::new()
    }
}

/// Link provider trait
#[async_trait::async_trait]
pub trait LinkProvider: Send + Sync {
    /// Provider ID
    fn id(&self) -> &str;

    /// Provide links
    async fn provide_links(&self, file: &PathBuf, content: &str) -> anyhow::Result<Vec<DocumentLink>>;

    /// Resolve link target
    async fn resolve_link(&self, link: &DocumentLink) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
}

/// Document link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentLink {
    /// Range
    pub range: LinkRange,
    /// Target URL/path
    pub target: Option<String>,
    /// Tooltip
    pub tooltip: Option<String>,
    /// Link kind
    pub kind: LinkKind,
    /// Data (for resolution)
    #[serde(skip)]
    pub data: Option<serde_json::Value>,
}

impl DocumentLink {
    pub fn new(range: LinkRange, target: impl Into<String>) -> Self {
        Self {
            range,
            target: Some(target.into()),
            tooltip: None,
            kind: LinkKind::Url,
            data: None,
        }
    }

    pub fn unresolved(range: LinkRange) -> Self {
        Self {
            range,
            target: None,
            tooltip: None,
            kind: LinkKind::Url,
            data: None,
        }
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn with_kind(mut self, kind: LinkKind) -> Self {
        self.kind = kind;
        self
    }
}

/// Link range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl LinkRange {
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self { start_line, start_col, end_line, end_col }
    }

    pub fn single_line(line: u32, start_col: u32, end_col: u32) -> Self {
        Self { start_line: line, start_col, end_line: line, end_col }
    }
}

/// Link kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkKind {
    /// HTTP/HTTPS URL
    Url,
    /// File path
    File,
    /// Definition link
    Definition,
    /// Custom link
    Custom,
}

/// Document links configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentLinksConfig {
    /// Enable document links
    pub enabled: bool,
    /// Enable URL detection
    pub urls: bool,
    /// Enable file path detection
    pub file_paths: bool,
    /// Modifiers to show links (ctrl, alt, etc)
    pub modifier: LinkModifier,
}

impl Default for DocumentLinksConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            urls: true,
            file_paths: true,
            modifier: LinkModifier::Ctrl,
        }
    }
}

/// Modifier key for showing links
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkModifier {
    None,
    Ctrl,
    Alt,
    Shift,
    Meta,
}

/// Document links event
#[derive(Debug, Clone)]
pub enum DocumentLinksEvent {
    LinksUpdated { file: PathBuf },
}

/// URL link provider
pub struct UrlLinkProvider {
    url_regex: Regex,
}

impl UrlLinkProvider {
    pub fn new() -> Self {
        // Match common URLs
        let url_regex = Regex::new(
            r"https?://[^\s<>\[\](){}|\\^`\x00-\x1f\x7f]+"
        ).expect("Invalid URL regex");

        Self { url_regex }
    }
}

impl Default for UrlLinkProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LinkProvider for UrlLinkProvider {
    fn id(&self) -> &str {
        "url"
    }

    async fn provide_links(&self, _file: &PathBuf, content: &str) -> anyhow::Result<Vec<DocumentLink>> {
        let mut links = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            for m in self.url_regex.find_iter(line) {
                let link = DocumentLink::new(
                    LinkRange::single_line(
                        line_num as u32,
                        m.start() as u32,
                        m.end() as u32,
                    ),
                    m.as_str(),
                )
                .with_kind(LinkKind::Url)
                .with_tooltip(format!("Open: {}", m.as_str()));

                links.push(link);
            }
        }

        Ok(links)
    }
}

/// File path link provider
pub struct FilePathLinkProvider {
    path_regex: Regex,
    base_path: PathBuf,
}

impl FilePathLinkProvider {
    pub fn new(base_path: PathBuf) -> Self {
        // Match file paths like ./file.rs, ../folder/file.ts, etc.
        let path_regex = Regex::new(
            r#"(?:\.\.?/)?(?:[\w.-]+/)*[\w.-]+\.\w+"#
        ).expect("Invalid path regex");

        Self { path_regex, base_path }
    }
}

#[async_trait::async_trait]
impl LinkProvider for FilePathLinkProvider {
    fn id(&self) -> &str {
        "file-path"
    }

    async fn provide_links(&self, file: &PathBuf, content: &str) -> anyhow::Result<Vec<DocumentLink>> {
        let mut links = Vec::new();
        let dir = file.parent().unwrap_or(&self.base_path);

        for (line_num, line) in content.lines().enumerate() {
            for m in self.path_regex.find_iter(line) {
                let path_str = m.as_str();
                let resolved = dir.join(path_str);

                // Only add if file exists
                if resolved.exists() {
                    let link = DocumentLink::new(
                        LinkRange::single_line(
                            line_num as u32,
                            m.start() as u32,
                            m.end() as u32,
                        ),
                        format!("file://{}", resolved.display()),
                    )
                    .with_kind(LinkKind::File)
                    .with_tooltip(format!("Open: {}", resolved.display()));

                    links.push(link);
                }
            }
        }

        Ok(links)
    }
}

/// Package.json dependency link provider
pub struct DependencyLinkProvider;

impl DependencyLinkProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DependencyLinkProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LinkProvider for DependencyLinkProvider {
    fn id(&self) -> &str {
        "dependency"
    }

    async fn provide_links(&self, file: &PathBuf, content: &str) -> anyhow::Result<Vec<DocumentLink>> {
        let mut links = Vec::new();

        // Only process package.json
        if !file.ends_with("package.json") {
            return Ok(links);
        }

        // Simple parsing for dependencies
        let dep_regex = Regex::new(r#""([@\w/-]+)":\s*""#)?;

        for (line_num, line) in content.lines().enumerate() {
            for cap in dep_regex.captures_iter(line) {
                if let Some(name) = cap.get(1) {
                    let pkg_name = name.as_str();
                    let url = format!("https://www.npmjs.com/package/{}", pkg_name);

                    let link = DocumentLink::new(
                        LinkRange::single_line(
                            line_num as u32,
                            name.start() as u32,
                            name.end() as u32,
                        ),
                        url,
                    )
                    .with_kind(LinkKind::Url)
                    .with_tooltip(format!("npm: {}", pkg_name));

                    links.push(link);
                }
            }
        }

        Ok(links)
    }
}
