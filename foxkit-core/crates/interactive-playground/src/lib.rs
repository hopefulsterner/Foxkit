//! # Foxkit Interactive Playground
//!
//! Code playground and scratch files for experimentation.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

static PLAYGROUND_ID: AtomicU64 = AtomicU64::new(1);

/// Interactive playground service
pub struct InteractivePlaygroundService {
    /// Active playgrounds
    playgrounds: RwLock<HashMap<PlaygroundId, Playground>>,
    /// Active playground
    active: RwLock<Option<PlaygroundId>>,
    /// Templates
    templates: RwLock<HashMap<String, PlaygroundTemplate>>,
    /// Event sender
    event_tx: broadcast::Sender<PlaygroundEvent>,
}

impl InteractivePlaygroundService {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(64);

        let mut service = Self {
            playgrounds: RwLock::new(HashMap::new()),
            active: RwLock::new(None),
            templates: RwLock::new(HashMap::new()),
            event_tx,
        };

        // Register default templates
        service.register_default_templates();
        service
    }

    fn register_default_templates(&mut self) {
        let mut templates = self.templates.write();
        
        templates.insert("rust".to_string(), PlaygroundTemplate {
            name: "Rust".to_string(),
            language: "rust".to_string(),
            content: r#"fn main() {
    println!("Hello, playground!");
}
"#.to_string(),
            description: "Basic Rust playground".to_string(),
        });

        templates.insert("typescript".to_string(), PlaygroundTemplate {
            name: "TypeScript".to_string(),
            language: "typescript".to_string(),
            content: r#"const greeting: string = "Hello, playground!";
console.log(greeting);
"#.to_string(),
            description: "Basic TypeScript playground".to_string(),
        });

        templates.insert("python".to_string(), PlaygroundTemplate {
            name: "Python".to_string(),
            language: "python".to_string(),
            content: r#"print("Hello, playground!")
"#.to_string(),
            description: "Basic Python playground".to_string(),
        });

        templates.insert("javascript".to_string(), PlaygroundTemplate {
            name: "JavaScript".to_string(),
            language: "javascript".to_string(),
            content: r#"const greeting = "Hello, playground!";
console.log(greeting);
"#.to_string(),
            description: "Basic JavaScript playground".to_string(),
        });
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<PlaygroundEvent> {
        self.event_tx.subscribe()
    }

    /// Create new playground
    pub fn create(&self, language: &str) -> PlaygroundId {
        let template = self.templates.read()
            .get(language)
            .cloned()
            .unwrap_or_else(|| PlaygroundTemplate {
                name: language.to_string(),
                language: language.to_string(),
                content: String::new(),
                description: String::new(),
            });

        let playground = Playground::new(template.language.clone(), template.content.clone());
        let id = playground.id.clone();

        self.playgrounds.write().insert(id.clone(), playground.clone());
        *self.active.write() = Some(id.clone());

        let _ = self.event_tx.send(PlaygroundEvent::Created(playground));
        id
    }

    /// Create from template
    pub fn create_from_template(&self, template_id: &str) -> Option<PlaygroundId> {
        let template = self.templates.read().get(template_id)?.clone();
        
        let playground = Playground::new(template.language, template.content);
        let id = playground.id.clone();

        self.playgrounds.write().insert(id.clone(), playground.clone());
        *self.active.write() = Some(id.clone());

        let _ = self.event_tx.send(PlaygroundEvent::Created(playground));
        Some(id)
    }

    /// Get playground
    pub fn get(&self, id: &PlaygroundId) -> Option<Playground> {
        self.playgrounds.read().get(id).cloned()
    }

    /// Update content
    pub fn update_content(&self, id: &PlaygroundId, content: String) {
        if let Some(pg) = self.playgrounds.write().get_mut(id) {
            pg.content = content;
            pg.modified = Utc::now();
            let _ = self.event_tx.send(PlaygroundEvent::Updated(id.clone()));
        }
    }

    /// Run playground
    pub async fn run(&self, id: &PlaygroundId) -> Option<ExecutionResult> {
        let pg = self.playgrounds.read().get(id)?.clone();
        
        let _ = self.event_tx.send(PlaygroundEvent::ExecutionStarted(id.clone()));

        // Simulate execution (real impl would use actual runtime)
        let result = self.execute(&pg).await;

        // Store result
        if let Some(playground) = self.playgrounds.write().get_mut(id) {
            playground.last_result = Some(result.clone());
            playground.execution_count += 1;
        }

        let _ = self.event_tx.send(PlaygroundEvent::ExecutionCompleted {
            id: id.clone(),
            result: result.clone(),
        });

        Some(result)
    }

    async fn execute(&self, _playground: &Playground) -> ExecutionResult {
        // Placeholder - real impl would execute code
        ExecutionResult {
            success: true,
            output: "Execution not implemented in playground".to_string(),
            error: None,
            duration_ms: 0,
            memory_bytes: None,
        }
    }

    /// Stop execution
    pub fn stop(&self, id: &PlaygroundId) {
        let _ = self.event_tx.send(PlaygroundEvent::ExecutionStopped(id.clone()));
    }

    /// Close playground
    pub fn close(&self, id: &PlaygroundId) {
        if let Some(pg) = self.playgrounds.write().remove(id) {
            let mut active = self.active.write();
            if active.as_ref() == Some(id) {
                *active = self.playgrounds.read().keys().next().cloned();
            }
            let _ = self.event_tx.send(PlaygroundEvent::Closed(pg.id));
        }
    }

    /// Get active playground
    pub fn active(&self) -> Option<Playground> {
        self.active.read()
            .as_ref()
            .and_then(|id| self.playgrounds.read().get(id).cloned())
    }

    /// Set active
    pub fn set_active(&self, id: &PlaygroundId) {
        if self.playgrounds.read().contains_key(id) {
            *self.active.write() = Some(id.clone());
            let _ = self.event_tx.send(PlaygroundEvent::Activated(id.clone()));
        }
    }

    /// List playgrounds
    pub fn list(&self) -> Vec<Playground> {
        self.playgrounds.read().values().cloned().collect()
    }

    /// List templates
    pub fn templates(&self) -> Vec<PlaygroundTemplate> {
        self.templates.read().values().cloned().collect()
    }

    /// Register template
    pub fn register_template(&self, id: impl Into<String>, template: PlaygroundTemplate) {
        self.templates.write().insert(id.into(), template);
    }

    /// Share playground (get shareable state)
    pub fn share(&self, id: &PlaygroundId) -> Option<SharedPlayground> {
        let pg = self.playgrounds.read().get(id)?.clone();
        
        Some(SharedPlayground {
            language: pg.language,
            content: pg.content,
            created: pg.created,
        })
    }

    /// Import shared playground
    pub fn import(&self, shared: SharedPlayground) -> PlaygroundId {
        let playground = Playground::new(shared.language, shared.content);
        let id = playground.id.clone();

        self.playgrounds.write().insert(id.clone(), playground.clone());
        *self.active.write() = Some(id.clone());

        let _ = self.event_tx.send(PlaygroundEvent::Created(playground));
        id
    }
}

impl Default for InteractivePlaygroundService {
    fn default() -> Self {
        Self::new()
    }
}

/// Playground ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlaygroundId(u64);

impl PlaygroundId {
    fn new() -> Self {
        Self(PLAYGROUND_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Playground
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playground {
    /// Unique ID
    pub id: PlaygroundId,
    /// Language
    pub language: String,
    /// Content
    pub content: String,
    /// Created timestamp
    pub created: DateTime<Utc>,
    /// Modified timestamp
    pub modified: DateTime<Utc>,
    /// Last execution result
    pub last_result: Option<ExecutionResult>,
    /// Execution count
    pub execution_count: u32,
    /// Title
    pub title: Option<String>,
}

impl Playground {
    pub fn new(language: String, content: String) -> Self {
        let now = Utc::now();
        Self {
            id: PlaygroundId::new(),
            language,
            content,
            created: now,
            modified: now,
            last_result: None,
            execution_count: 0,
            title: None,
        }
    }

    pub fn display_title(&self) -> String {
        self.title.clone().unwrap_or_else(|| {
            format!("Playground #{}", self.id.0)
        })
    }
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Success
    pub success: bool,
    /// Output
    pub output: String,
    /// Error message
    pub error: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Memory usage
    pub memory_bytes: Option<u64>,
}

/// Playground template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundTemplate {
    /// Name
    pub name: String,
    /// Language ID
    pub language: String,
    /// Initial content
    pub content: String,
    /// Description
    pub description: String,
}

/// Shared playground (for import/export)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedPlayground {
    /// Language
    pub language: String,
    /// Content
    pub content: String,
    /// Created timestamp
    pub created: DateTime<Utc>,
}

/// Playground event
#[derive(Debug, Clone)]
pub enum PlaygroundEvent {
    Created(Playground),
    Updated(PlaygroundId),
    Closed(PlaygroundId),
    Activated(PlaygroundId),
    ExecutionStarted(PlaygroundId),
    ExecutionCompleted {
        id: PlaygroundId,
        result: ExecutionResult,
    },
    ExecutionStopped(PlaygroundId),
}

/// Multi-cell playground (notebook-like)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookPlayground {
    pub id: PlaygroundId,
    pub cells: Vec<PlaygroundCell>,
    pub metadata: PlaygroundMetadata,
}

/// Playground cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundCell {
    pub id: u64,
    pub kind: CellKind,
    pub content: String,
    pub output: Option<CellOutput>,
    pub execution_count: Option<u32>,
}

/// Cell kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CellKind {
    Code,
    Markdown,
}

/// Cell output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellOutput {
    pub output_type: OutputType,
    pub content: String,
    pub mime_type: Option<String>,
}

/// Output type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OutputType {
    Text,
    Html,
    Image,
    Error,
}

/// Playground metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaygroundMetadata {
    pub title: Option<String>,
    pub language: Option<String>,
    pub author: Option<String>,
    pub tags: Vec<String>,
}
